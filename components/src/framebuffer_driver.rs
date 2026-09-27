//! **The framebuffer driver: a screen the firmware already set up, served behind the framebuffer
//! contract** (the shell on the firmware screen; milestone 198's rung 1b).
//!
//! `gpu_driver` serves [`graphics_protocol`] by programming a virtio-gpu device. A PC has no
//! virtio-gpu, but its firmware leaves a linear framebuffer running when it hands over (milestone
//! 243: UEFI's `EFI_GRAPHICS_OUTPUT_PROTOCOL`, whose aperture outlives `ExitBootServices`). This
//! driver serves the **same contract** over that screen, so `display_terminal`, which cannot tell
//! the two apart, puts the shell on a PC's monitor with no device of our own to negotiate with.
//! Milestone 157 describes exactly this driver for the boards ("a new backend behind that same
//! contract, read a pre-set buffer instead of negotiating virtio queues"), fed there by U-Boot's
//! `simple-framebuffer` rather than by UEFI.
//!
//! # Its whole authority
//!
//! - slot 0, a **report** endpoint (`WRITE`): one `UP` message once it is serving;
//! - slot 1, the **display** endpoint (`READ`): where its one client `CALL`s `INFO` and `FLUSH`;
//! - slot 2, an **untyped**: the budget its surface mapping's page tables come out of;
//! - slot 3, the **surface**: one `PageFrame` capability naming the contract's run of RAM frames,
//!   shared with the client, which it maps itself (DECISIONS §102, milestone 108's shape);
//! - mapped before `_start`: **the covered part of the aperture**, device-typed (uncacheable on
//!   x86), at [`APERTURE_VA`]. Only the rows the surface can reach, not the whole screen
//!   (`screen_console::Aperture::span`), and as a spawn-time mapping rather than a capability, so
//!   it holds no name for the screen and cannot map it again, delegate it, or hand it on. That is
//!   the choice milestone 159's TRNG driver and milestone 261's NVMe server made for the same
//!   reason; a capability would need a `DeviceFrame` naming more than one page, which the
//!   capability surface does not have.
//!
//! No interrupt, no DMA, no transport, no physical address. The kernel tells it the geometry in its
//! three argument registers (`screen_console::Aperture::to_words`, then the aperture's offset into
//! its first page), because nothing it holds could describe it.
//!
//! # What a flush is, here
//!
//! The client draws into the surface (RAM) and says which rectangle changed. For virtio-gpu that
//! is two device commands. For a screen that is already scanning out, it is **a copy**: each pixel
//! of the rectangle is read from the surface and stored into the aperture in the screen's byte
//! order, at the screen's stride. The arithmetic is `screen_console::Aperture::copy`, host-tested
//! there. The surface lands at the screen's top-left corner, clipped to the screen when the screen
//! is the smaller of the two.
//!
//! # BUGS
//!
//! - **The terminal is the contract's size, not the screen's.** The surface is fixed at
//!   [`graphics_protocol::WIDTH`] by [`graphics_protocol::HEIGHT`] (924x344, a 132x43 grid), so on
//!   a 1920x1080 monitor the shell occupies the top-left corner and the rest stays black. Growing it
//!   means a surface sized at spawn rather than at compile time, which is a change to the contract
//!   and not to this driver.
//! - **Every pixel crosses an uncacheable mapping one word at a time.** A full-surface flush (every
//!   scroll) is 1.2 MB of UC stores: free under QEMU, and not measured on silicon. x86 has no
//!   write-combining here because the kernel does not program the PAT (milestone 243's `BUGS`).
//! - **One client, no arbitration.** Whoever holds the display endpoint draws; that is the
//!   contract's rung-one shape and the compositor is what multiplexes it.
//!
//! Name: provisional. Introduced 2026-09-19 by the shell on the firmware screen's lane, in the
//! `<device>_driver` shape `gpu_driver`, `block_driver` and `keyboard_driver` take, the device
//! being a linear framebuffer.
//! Considered `screen_driver`, which names a thing every display driver drives, and
//! `firmware_screen_driver`, which names who set it up rather than what it is and would be wrong on
//! the boards, where U-Boot sets it up.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

use graphics_protocol as gfx;
use screen_console::Aperture;
use user_mode_runtime::mapped_window::MappedWindow;
use user_mode_runtime::{exit, recv_cap, reply, send};

/// Capability slots, by convention with `kernel/src/user/display_service.rs`.
const REPORT: u64 = 0;
const DISPLAY: u64 = 1;
const BUDGET: u64 = 2;
/// The surface, one `PageFrame` capability naming the whole run (DECISIONS §102).
const SURFACE_FRAME: u64 = 3;

/// Where this driver maps the surface. Its choice: it holds the frames. 2 MiB-aligned for the
/// reason `gpu_driver`'s `DMA_VA` gives, so the run spans as few page-table windows as it can.
const SURFACE_VA: u64 = address_space_map::pair_page(0x0000_0000_0100_0000);

/// Where the kernel maps the covered part of the aperture before `_start`. **Must match
/// `kernel/src/user/display_service.rs`'s `SCREEN_APERTURE_VA`.** A service window on the
/// address-space map (milestone 206 (a program image has under 896 KiB)), 8 MiB of it at most, and 2 MiB-aligned for the same reason as
/// above. It was `0x4000_0000` until 2026-09-26, the first address of the heap band, which this
/// driver could use only because it has no heap.
const APERTURE_VA: u64 = address_space_map::service_window(0x5800_0000);

/// Failure codes, sent on the report endpoint in a `0xDEAD_...` word so a failure names its step.
const E_GEOMETRY: u64 = 0x01;
const E_SURFACE: u64 = 0x02;

fn die(code: u64) -> ! {
    send(REPORT, 0xDEAD_0000_0000_0000 | code, 0, 0);
    exit();
}

#[unsafe(no_mangle)]
pub extern "C" fn _start(size: u64, layout: u64, offset: u64) -> ! {
    // The geometry arrived in registers from the kernel; it is checked here rather than believed,
    // because a geometry that does not close would have this process storing past its own mapping.
    let Some(aperture) = Aperture::from_words(size, layout) else {
        die(E_GEOMETRY)
    };
    let (width, height) = aperture.size();
    // The aperture can never be larger than the surface it shows: the kernel clipped it to the
    // contract's size, and a larger one would let a flush read past the surface.
    if width > gfx::WIDTH || height > gfx::HEIGHT || offset >= 4096 {
        die(E_GEOMETRY);
    }

    if !user_mode_runtime::map_page_frame(SURFACE_FRAME, SURFACE_VA, true, BUDGET) {
        die(E_SURFACE);
    }
    // SAFETY: the surface run is `SURFACE_PAGE_FRAMES` frames, mapped read/write at SURFACE_VA by
    // the call just above, and `SURFACE_BYTES` is inside it (`graphics_protocol`'s own arithmetic).
    let surface = unsafe { MappedWindow::new(SURFACE_VA, gfx::SURFACE_BYTES as u64) };
    // SAFETY: the kernel mapped every page from APERTURE_VA through `offset + span` device-typed
    // and writable before this program's first instruction
    // (`display_service::start_screen_terminal`), and the geometry that span was computed from is
    // the one `from_words` just validated.
    let screen = unsafe { MappedWindow::new(APERTURE_VA + offset, aperture.span() as u64) };

    send(
        REPORT,
        gfx::status::UP,
        width as u64 | ((height as u64) << 32),
        1,
    );

    loop {
        let (w0, reply_slot, _) = recv_cap(DISPLAY);
        let (r0, r1): (i64, u64) = match gfx::op(w0) {
            // The runtime half of the geometry contract: the part of the screen the surface
            // covers, which is what a client should lay its grid out over. It is never larger
            // than the compile-time surface, so a client that maps `SURFACE_BYTES` and paints at
            // `STRIDE` is right either way.
            gfx::display::INFO => (0, width as u64 | ((height as u64) << 32)),
            gfx::display::FLUSH => {
                let (x, y, w, h) = gfx::unrect(gfx::operand(w0));
                let copied = aperture.copy(
                    x,
                    y,
                    w,
                    h,
                    |px, py| surface.r32(gfx::offset_of(px, py) as u64),
                    |at, word| screen.w32(at as u64, word),
                );
                (if copied { 0 } else { gfx::EINVAL }, 0)
            }
            _ => (gfx::EINVAL, 0),
        };
        reply(reply_slot, r0 as u64, r1);
    }
}

user_mode_runtime::panic_handler!();
