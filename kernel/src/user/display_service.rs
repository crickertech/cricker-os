//! **The display seam's wiring**: a confined virtio-gpu driver, the client that paints through it,
//! and the terminal that took the client's place (milestones 29, 33; notes/framebuffer-contract.md).
//!
//! # The pixels are capabilities now (milestone 108, notes/frames.md)
//!
//! Everything here used to arrive as `Spawn::maps`: the kernel wrote the DMA region into the
//! driver's address space and the surface into the client's, at addresses the kernel picked, before
//! either program's first instruction. Nothing in either capability table said the memory was there, and
//! nothing could narrow it, hand it on, or take it back.
//!
//! Each program now holds its pages as `PageFrame` capabilities and maps them itself, out of an untyped
//! it also holds.
//!
//! # One capability per run, not one per page (DECISIONS §102, milestone 142)
//!
//! `PageFrame` used to name exactly one page, so the driver's DMA region (rings and control
//! buffers, then the surface) was one capability per page: at the 128x64 scanout this milestone
//! grew from, that was 9 capabilities in 9 consecutive slots of a sixteen-slot capability table,
//! with one slot spare and no room for anything larger. §102 gave `PageFrame` a page count, so a
//! whole run is now **one capability and one `MAP` call**, however many pages it holds. That is
//! what makes the grown scanout ([`graphics_protocol::WIDTH`] x [`graphics_protocol::HEIGHT`],
//! [`graphics_protocol::SURFACE_PAGE_FRAMES`] page frames) fit at all: that many one-page capabilities
//! never would have, whether the surface is the 900-frame 1280x720 this milestone first grew to or
//! the 311-frame 924x344 it was retargeted to on 2026-08-27. See notes/frames.md.
//!
//! What stays wired at spawn is nothing at all here: these programs have no extra stack pages, so
//! the only page the kernel still places is the one `load` gives every process.

use core::num::NonZeroU64;

use super::*;
use crate::cap::{
    Rights, irq_cap, memory_region_cap, page_frame_run_cap, page_frame_run_len, rendezvous_cap,
    virtio_cap,
};
use crate::sched::RendezvousId;

/// The DMA region, in frames: one for the rings and control buffers, then the surface.
const DMA_PAGE_FRAMES: u64 = 1 + graphics_protocol::SURFACE_PAGE_FRAMES as u64;

/// The three run lengths this file grants, as run lengths.
///
/// They are `const` rather than computed at the call site so that
/// [`crate::cap::page_frame_run_len`]'s refusal of a zero-page run happens **while this file is
/// compiled**: a geometry change that took `SURFACE_PAGE_FRAMES` to zero would fail the build here
/// rather than panic a driver at boot. (Provisional names; a lane does not settle a name.)
const SURFACE_RUN: NonZeroU64 = page_frame_run_len(graphics_protocol::SURFACE_PAGE_FRAMES as u64);
const DMA_RUN: NonZeroU64 = page_frame_run_len(DMA_PAGE_FRAMES);
const ONE_PAGE_RUN: NonZeroU64 = NonZeroU64::MIN;

/// The driver binary's escape-attempt role; must match `components/src/gpu_driver.rs` `ROLE_BACKING_ESCAPE`.
const ROLE_BACKING_ESCAPE: u64 = 1;

/// **The budget every program on this path draws its page tables from** (milestone 108, widened for
/// milestone 142's larger scanout, then not shrunk back when the scanout was retargeted). At 128x64
/// the driver's whole DMA region and the terminal's surface each fit one 2 MiB window (one L3
/// table); at 1280x720 the 900-page surface alone spanned two windows, and the terminal's separate
/// output page a third, so more than one L3 was briefly the ordinary case rather than the edge case
/// notes/frames.md recorded at 800x608. **At 924x344 (retargeted 2026-08-27) the surface is back
/// under 512 pages** (311, [`graphics_protocol::SURFACE_PAGE_FRAMES`]), one 2 MiB window again, the
/// same shape 128x64 had. Twenty-four pages is more headroom than that now strictly needs, kept
/// rather than re-tuned down: still under one percent of the free pool, and a budget this path has
/// already exercised at the larger size is the safer one to keep than a smaller number nobody has
/// booted against.
const MAP_BUDGET_PAGES: u64 = 24;

// The driver's capability table. Must match components/src/gpu_driver.rs.
const DRIVER_SLOT_REPORT: u64 = 0;
const DRIVER_SLOT_IRQ: u64 = 1;
const DRIVER_SLOT_VIRTIO: u64 = 2;
const DRIVER_SLOT_DISPLAY: u64 = 3;
const DRIVER_SLOT_BUDGET: u64 = 4;
/// **The whole DMA region, one capability** (§102). Before the widening this was
/// [`DMA_PAGE_FRAMES`] separate slots, one per page, and a `const` assertion guarded the last one
/// against the fault slot; a scanout large enough to need this milestone would have failed that
/// assertion outright. One slot names the whole run now, so the *pressure* that assertion guarded
/// against is retired. See notes/frames.md's BUGS.
const DRIVER_SLOT_DMA: u64 = 5;

// The painting client's capability table. Must match fixtures/src/painter.rs.
const CLIENT_SLOT_REPORT: u64 = 0;
const CLIENT_SLOT_DISPLAY: u64 = 1;
const CLIENT_SLOT_BUDGET: u64 = 2;
/// The whole scanout, one capability (§102).
const CLIENT_SLOT_SURFACE: u64 = 3;

// The display terminal's capability table. Must match components/src/display_terminal.rs.
const TERM_SLOT_REPORT: u64 = 0;
const TERM_SLOT_DISPLAY: u64 = 1;
const TERM_SLOT_TERM: u64 = 2;
const TERM_SLOT_BUDGET: u64 = 3;
/// The whole scanout, one capability (§102).
const TERM_SLOT_SURFACE: u64 = 4;
/// The page an application writes text into. Still its own single-page capability: it is not part
/// of the scanout's contiguous run.
const TERM_SLOT_OUT: u64 = TERM_SLOT_SURFACE + 1;

// **No grant list on this path may reach the fault slot** (reinstated by milestone 142's review,
// MAJOR 6). `abi::fault::FAULT_EP_SLOT` is `CAPABILITY_TABLE_SLOTS - 1` and the supervisor writes
// the process's fault endpoint there; a grant that landed on it would overwrite that endpoint with
// a frame, and the process would simply stop being supervised. There is no runtime complaint for
// that, because `grant_at` on an *empty* slot succeeds: it surfaces later as a faulting process
// that hangs instead of being reaped, three subsystems away from the cause.
//
// The retired assertion said `DRIVER_SLOT_DMA + DMA_PAGE_FRAMES <= FAULT_EP_SLOT`, which was the
// same claim expressed in the one-slot-per-page representation §102 abolished. This is that claim
// in run terms: the highest slot each of the three grant lists actually uses, against the slot none
// of them may reach. It is the *statement* that was worth keeping, not the arithmetic.
const _: () = {
    assert!(DRIVER_SLOT_DMA < abi::fault::FAULT_EP_SLOT);
    assert!(CLIENT_SLOT_SURFACE < abi::fault::FAULT_EP_SLOT);
    assert!(TERM_SLOT_OUT < abi::fault::FAULT_EP_SLOT);
};

/// Grant one capability naming the `count`-frame run at `base`, read/write, at `slot`. The
/// counterpart of the single `MAP` call each of these programs now makes at startup for the run
/// (§102): before this widening a run was `count` separate slots and `count` separate `MAP` calls.
fn grant_run(slot: u64, base: u64, count: NonZeroU64, what: &str) {
    crate::sched::grant_at(
        slot,
        page_frame_run_cap(base, count, Rights::READ.union(Rights::WRITE)),
    )
    .unwrap_or_else(|_| panic!("{what}: slot {slot} was occupied"));
}

/// **Wire and spawn the GPU driver and the painting client.** Returns
/// `(driver report, client report)`, or `None` if no virtio-gpu function is on the bus.
///
/// One spawn site for two processes on purpose: they are only meaningful together (a driver with
/// no client serves nobody, a client with no driver blocks on its first CALL), and the endpoint
/// and the shared frames that join them are created here, in the one place that is allowed to
/// know both halves.
pub fn start(
    driver_image: &'static [u8],
    client_image: &'static [u8],
) -> Option<(RendezvousId, RendezvousId)> {
    let (driver_report, display_ep, surface) = wire_driver(driver_image, 0, 0)?;

    // --- the client: an endpoint and the pixels. Nothing else, which is the point. ---
    let client_report = crate::sched::create_rendezvous();
    let budget =
        crate::memory_region::create(MAP_BUDGET_PAGES).expect("no map budget for the client");
    crate::sched::spawn(move || {
        crate::sched::grant_at(
            CLIENT_SLOT_REPORT,
            rendezvous_cap(client_report, Rights::WRITE),
        )
        .expect("client slot 0 was occupied");
        crate::sched::grant_at(
            CLIENT_SLOT_DISPLAY,
            rendezvous_cap(display_ep, Rights::WRITE),
        )
        .expect("client slot 1 was occupied");
        crate::sched::grant_at(CLIENT_SLOT_BUDGET, memory_region_cap(budget))
            .expect("client slot 2 was occupied");
        grant_run(
            CLIENT_SLOT_SURFACE,
            surface,
            SURFACE_RUN,
            "the painting client",
        );
        run(
            client_image,
            Spawn {
                arg0: 0,
                arg1: 0, // no physical address: a client has no business knowing one
                arg2: 0,
                grants: &[], // every one of them is placed above, at its own slot
                maps: &[],
            },
        )
    })
    .expect("could not spawn the painting client");

    Some((driver_report, client_report))
}

/// **Spawn a driver that attacks its own confinement** (`components/src/gpu_driver.rs` `run_backing_escape`):
/// it asks the device to read pixels out of a frame outside its grant. Returns
/// `(report endpoint, the victim frame's physical address)`, or `None` if no GPU is on the bus.
///
/// It gets exactly the honest driver's world, no more: the same confined transport, the same
/// region, the same interrupt. That is what makes it a fair test of the barrier rather than of a
/// missing capability. No client, because it never serves one.
///
/// The **kernel** picks the victim frame and hands it over in `arg2`, the same way milestone 16b's
/// confinement test picks its own escape frame: the caller has to know the exact address to look
/// for in the IOMMU's fault queue, and a driver guessing at "the frame past my region" guesses
/// wrong (the shadow page is allocated right after it, and that frame IS in the domain). The frame
/// is deliberately never freed: it is an escape target, and handing it back to the allocator while
/// a device has been told to read it is the use-after-free-by-hardware notes/dma.md warns about.
pub fn start_backing_escape(driver_image: &'static [u8]) -> Option<(RendezvousId, u64)> {
    let victim = crate::memory::alloc()
        .expect("no victim frame for the backing-escape test")
        .addr();
    let (report, _, _) = wire_driver(driver_image, ROLE_BACKING_ESCAPE, victim)?;
    Some((report, victim))
}

/// **A virtio-gpu, wired and confined but driven by nobody yet**: what [`wire_device`] hands back.
///
/// Everything the kernel has to do for a GPU that is not driving it: the transport registered and
/// confined to [`dma`](GpuDevice::dma)'s run, the interrupt routed and enabled, and the region's own
/// physical base written into its first page at `abi::virtio::DMA_PHYS_OFFSET`. What happens next is
/// somebody's spawn: the kernel's own test wiring ([`wire_driver`]), or since milestone 600
/// (provisional) the progenitor, which is granted these as capabilities by
/// `kernel::user::boot_progenitor` and builds `gpu_driver` and `display_terminal` itself.
/// Name: provisional.
pub struct GpuDevice {
    /// The `Virtio` capability's id (`crate::virtio::register`'s return value).
    pub vid: usize,
    /// The completion interrupt, routed and enabled.
    pub intid: u32,
    /// The DMA region's physical base: [`DMA_PAGE_FRAMES`] contiguous frames, the rings and
    /// control buffers in the first and the surface after it.
    pub dma: u64,
    /// The surface's physical base, `dma` plus one frame: [`SURFACE_PAGE_FRAMES`] frames, the part
    /// of the region a client also maps.
    ///
    /// [`SURFACE_PAGE_FRAMES`]: graphics_protocol::SURFACE_PAGE_FRAMES
    pub surface: u64,
}

/// The run lengths a holder of a [`GpuDevice`] grants, so `kernel::user::boot_progenitor` mints the
/// same two runs this file does rather than recomputing them.
pub const GPU_DMA_RUN: NonZeroU64 = DMA_RUN;
/// See [`GPU_DMA_RUN`].
pub const GPU_SURFACE_RUN: NonZeroU64 = SURFACE_RUN;

/// **Find the GPU, build its DMA region, route its interrupt and register its confined transport**,
/// and spawn nothing. `None` if no virtio-gpu function is on the bus.
///
/// **The region is one contiguous run, and has been since milestone 142 (a text display good
/// enough that people use it instead of a GUI)**, by DECISIONS §102 (a Frame names a run of pages):
/// one `alloc_contiguous_zeroed` of [`DMA_PAGE_FRAMES`] frames, registered with `crate::virtio` as
/// one region, and granted as one `PageFrame` capability. That is the fact milestone 600
/// (provisional) was promoted to check, because milestone 177 (wire the graphical terminal stack
/// into the real interactive boot) had built the graphical stack kernel-side on the premise that
/// the driver needed a capability per DMA page.
pub fn wire_device() -> Option<GpuDevice> {
    let d = crate::pci::find_gpu_device()?;

    // The DMA region: contiguous, because the surface must be one run of physical frames for the
    // device's backing to be a single memory entry and for the IOMMU domain to cover it as one
    // range. Zeroed, so neither a stale descriptor nor a stale pixel is ever visible to the
    // device or to the client.
    let dma = crate::memory::alloc_contiguous_zeroed(DMA_PAGE_FRAMES as usize)
        .expect("no contiguous DMA region for the GPU driver")
        .addr();
    // The region says where it is, in the tail of its first page, which `gpu_driver`'s rings and
    // control buffers end well short of (`abi::virtio::DMA_PHYS_OFFSET`). A driver the progenitor
    // builds has no other way to learn it: the progenitor holds the run as a capability and knows
    // no physical address either.
    super::write_dma_phys(dma);
    let surface = dma + FRAME_SIZE; // page 1 onward: the frames the client also maps

    // The device's interrupt, routed to an endpoint so the driver's WAIT receives it as a
    // message (milestone 9a).
    let irq_ep = crate::sched::create_rendezvous();
    crate::sched::bind_irq(d.intid, irq_ep);
    crate::arch::irq::enable(d.intid);

    // Register the transport: the kernel keeps the registers and the two DMA-critical powers,
    // and confines the device in hardware to exactly this region plus the shadow page.
    let vid = crate::virtio::register(
        crate::virtio::Transport::pci(&d),
        dma,
        DMA_PAGE_FRAMES * FRAME_SIZE,
        Some(d.rid), // the PCIe requester id the IOMMU keys its tables on
    );
    Some(GpuDevice {
        vid,
        intid: d.intid,
        dma,
        surface,
    })
}

/// The shared half of both spawns: [`wire_device`], then spawn `driver_image` at `role` with
/// `arg2`. Returns `(report endpoint, display endpoint, the surface's physical base)`.
fn wire_driver(
    driver_image: &'static [u8],
    role: u64,
    arg2: u64,
) -> Option<(RendezvousId, RendezvousId, u64)> {
    let GpuDevice {
        vid,
        intid,
        dma,
        surface,
    } = wire_device()?;

    let display_ep = crate::sched::create_rendezvous(); // client WRITE (CALL) -> driver READ
    let driver_report = crate::sched::create_rendezvous();

    // --- the driver: the confined transport, the interrupt, the whole DMA region, and the
    // display endpoint's serving half. The region is one `PageFrame` capability naming the whole
    // DMA_PAGE_FRAMES-page run (§102; see [`DRIVER_SLOT_DMA`]). ---
    let budget =
        crate::memory_region::create(MAP_BUDGET_PAGES).expect("no map budget for the driver");
    crate::sched::spawn(move || {
        crate::sched::grant_at(
            DRIVER_SLOT_REPORT,
            rendezvous_cap(driver_report, Rights::WRITE),
        )
        .expect("driver slot 0 was occupied");
        // The completion IRQ.
        crate::sched::grant_at(DRIVER_SLOT_IRQ, irq_cap(intid))
            .expect("driver slot 1 was occupied");
        // The confined transport.
        crate::sched::grant_at(DRIVER_SLOT_VIRTIO, virtio_cap(vid))
            .expect("driver slot 2 was occupied");
        // Serve clients.
        crate::sched::grant_at(
            DRIVER_SLOT_DISPLAY,
            rendezvous_cap(display_ep, Rights::READ),
        )
        .expect("driver slot 3 was occupied");
        crate::sched::grant_at(DRIVER_SLOT_BUDGET, memory_region_cap(budget))
            .expect("driver slot 4 was occupied");
        grant_run(DRIVER_SLOT_DMA, dma, DMA_RUN, "the GPU driver");
        run(
            driver_image,
            Spawn {
                arg0: role, // 0 = the GPU driver; 1 = the escape attempt
                // No physical address: the region's first page carries it
                // (`abi::virtio::DMA_PHYS_OFFSET`), the same place the progenitor-built driver
                // reads it from, so the harness and the boot hand the driver one world.
                arg1: 0,
                arg2,        // the escape role's victim frame; unused (0) by the GPU driver
                grants: &[], // every one of them is placed above, at its own slot
                maps: &[],
            },
        )
    })
    .expect("could not spawn the GPU driver");

    Some((driver_report, display_ep, surface))
}

/// **Wire and spawn the GPU driver alone**, with no client: `(report endpoint, display
/// endpoint, the surface's physical base)`, or `None` if no virtio-gpu is on the bus.
///
/// For rung two (milestone 33). The compositor takes `painter`'s place at this seam exactly as the
/// contract promised it would, so what it needs from rung one is a display endpoint to CALL and the
/// frames the device scans out. Nothing about the driver changes, which is the claim
/// notes/framebuffer-contract.md made when it said routing was by endpoint.
///
/// The caller must receive `FLUSHED` on the report endpoint after the client's first flush, or
/// the driver never serves a second one; see [`start_terminal`].
pub fn start_driver(driver_image: &'static [u8]) -> Option<(RendezvousId, RendezvousId, u64)> {
    wire_driver(driver_image, 0, 0)
}

/// What the kernel keeps after wiring a display terminal onto the scanout.
pub struct TerminalWiring {
    /// The GPU driver's status endpoint.
    pub driver_report: RendezvousId,
    /// The terminal's status endpoint.
    pub term_report: RendezvousId,
    /// The endpoint the terminal serves. The kernel holds WRITE, so it can play **both** classes
    /// of sender: an application (`OP_WRITE`) and an input source (`OP_BYTES`).
    pub term: RendezvousId,
    /// The application's output page, so the kernel can put the bytes of an `OP_WRITE` there.
    pub out: u64,
    /// The scanout frames, so the kernel can read the picture back through the direct map and
    /// grade it against a value it computed itself.
    pub surface: u64,
    /// **What a caller hands back to end it**: the threads, the region its endpoints live in, and
    /// the map budgets. Filled by [`start_screen_terminal`], whose endpoints come out of a region
    /// for exactly this reason (a server parked in `RECV` on the kernel's own endpoint chunks
    /// cannot be woken to die; `user::holding`'s BUGS). **Empty on the virtio path**, which
    /// predates it and which no caller tears down. The surface and the output page are not in it:
    /// they are frames rather than regions, and a caller that releases this frees them after.
    pub held: super::holding::Holding,
}

/// **Wire and spawn the GPU driver with a terminal on the whole scanout** (milestone 29's
/// remaining increment). `None` if no virtio-gpu function is on the bus.
///
/// The terminal takes `painter`'s place at the display seam with **exactly `painter`'s
/// authority**: a report endpoint, the display endpoint, and the surface frames. It holds no
/// device, no interrupt, and no physical address, and `gpu_driver` cannot tell it from the client that
/// drew a test pattern. That is the answer to "did the framebuffer contract need changing to
/// carry text?", and it is an answer made of a spawn literal rather than an argument.
///
/// What it adds over `painter`'s wiring is two things, and both are the terminal contract's, not
/// the framebuffer's: an endpoint it **serves** (the terminal contract's IPC half), and a page an
/// application writes bytes into (DECISIONS §10's control-by-message, bulk-by-shared-page split).
///
/// **The caller must receive three reports, not two, or the screen freezes after one frame**
/// (milestone 177). `driver_report` carries `UP` and then, once the terminal's first flush (its
/// blank grid) is served, `FLUSHED`; `term_report` carries `TERM_UP`. All three are blocking
/// `SEND`s, and the driver sends `FLUSHED` from inside its serving loop, so a caller that stops
/// after `UP` and `TERM_UP` leaves the driver parked there and every later flush unanswered. That is
/// the bug the real boot shipped with. **This is a foot gun kept on purpose, at rung three**: the
/// tests that call this read the digest `FLUSHED` carries as the driver-side witness, so the
/// function cannot swallow it for them. The boot no longer calls this (milestone 600
/// (provisional)): the progenitor builds the same two processes and takes the same three reports
/// in `system_initializer`'s `build_graphical_stack`. [`start_driver`] has the same obligation.
pub fn start_terminal(
    driver_image: &'static [u8],
    term_image: &'static [u8],
) -> Option<TerminalWiring> {
    // A scanout with no room for a character has nothing to show. It does **not** have to be a
    // whole number of them: 128 is not a multiple of the font's 7-pixel cell, so the ordinary case
    // leaves a two-pixel strip on the right that the terminal paints as background on its first
    // frame (see `components/src/display_terminal.rs`) and that `Vt::pixel` answers for, which is what
    // keeps the picture a total function of the state.
    const _: () = assert!(
        graphics_protocol::WIDTH >= bitmap_font::GLYPH_W
            && graphics_protocol::HEIGHT >= bitmap_font::GLYPH_H,
        "the scanout is too small for one character cell",
    );
    // And the script's geometry is the scanout's, checked here rather than trusted, because the
    // script is what three independent parties predict the picture from.
    const _: () = assert!(
        graphics_protocol::WIDTH / bitmap_font::GLYPH_W == video_terminal::script::COLS
            && graphics_protocol::HEIGHT / bitmap_font::GLYPH_H == video_terminal::script::ROWS,
        "video_terminal::script's geometry and the scanout's have drifted apart",
    );

    let (driver_report, display_ep, surface) = wire_driver(driver_image, 0, 0)?;
    let t = spawn_terminal(
        term_image,
        display_ep,
        surface,
        &crate::sched::create_rendezvous,
    );
    Some(TerminalWiring {
        driver_report,
        term_report: t.term_report,
        term: t.term,
        out: t.out,
        surface,
        held: super::holding::Holding::new(),
    })
}

/// **`display_terminal` on a display endpoint and a surface somebody else already serves**: the
/// half of [`start_terminal`] that does not care which driver is on the other side, shared with
/// [`start_screen_terminal`] so the terminal's authority is written down once. Returns
/// `(its report endpoint, the endpoint it serves, its output page)`.
///
/// `endpoint` makes its two endpoints: the kernel's own chunks on the virtio path, a region the
/// caller can reclaim on the firmware-screen path (see [`start_screen_terminal`]). The terminal's
/// thread and its map budget come back beside them, for a caller that tears it down.
fn spawn_terminal(
    term_image: &'static [u8],
    display_ep: RendezvousId,
    surface: u64,
    endpoint: &dyn Fn() -> RendezvousId,
) -> SpawnedTerminal {
    let out = crate::memory::alloc_zeroed()
        .expect("no output-page frame for the display terminal")
        .addr();

    let term_report = endpoint();
    let term = endpoint();
    let budget =
        crate::memory_region::create(MAP_BUDGET_PAGES).expect("no map budget for the terminal");

    let tid = crate::sched::spawn(move || {
        crate::sched::grant_at(TERM_SLOT_REPORT, rendezvous_cap(term_report, Rights::WRITE))
            .expect("terminal slot 0 was occupied");
        // CALL the driver.
        crate::sched::grant_at(TERM_SLOT_DISPLAY, rendezvous_cap(display_ep, Rights::WRITE))
            .expect("terminal slot 1 was occupied");
        // Serve the terminal.
        crate::sched::grant_at(TERM_SLOT_TERM, rendezvous_cap(term, Rights::READ))
            .expect("terminal slot 2 was occupied");
        crate::sched::grant_at(TERM_SLOT_BUDGET, memory_region_cap(budget))
            .expect("terminal slot 3 was occupied");
        grant_run(
            TERM_SLOT_SURFACE,
            surface,
            SURFACE_RUN,
            "the display terminal",
        );
        // The page an application writes text into.
        grant_run(TERM_SLOT_OUT, out, ONE_PAGE_RUN, "the display terminal");
        run(
            term_image,
            Spawn {
                arg0: video_terminal::status::MODE_DISPLAY,
                arg1: 0, // no physical address: a terminal has no business knowing one
                arg2: 0,
                grants: &[], // every one of them is placed above, at its own slot
                maps: &[],
            },
        )
    })
    .expect("could not spawn the display terminal");

    SpawnedTerminal {
        term_report,
        term,
        out,
        tid,
        budget,
    }
}

/// What [`spawn_terminal`] started, including what a teardown needs.
struct SpawnedTerminal {
    term_report: RendezvousId,
    term: RendezvousId,
    out: u64,
    tid: crate::thread::ThreadId,
    budget: u64,
}

/// Where `framebuffer_driver` finds the covered part of the aperture. **Must match
/// `components/src/framebuffer_driver.rs`'s `APERTURE_VA`.**
const SCREEN_APERTURE_VA: u64 = 0x0000_0000_4000_0000;

/// **The most aperture pages this wiring maps**: 8 MiB, four 2 MiB page-table windows. The tables
/// come out of the driver's own address-space budget (`AS_OVERHEAD`, sixteen pages shared with its
/// image and stack), so this is a bound rather than a preference. It is far more than the surface
/// needs: 344 rows at a 3840-pixel pitch is 1,290 pages. A screen whose covered rows would need
/// more is refused rather than mapped short.
const MAX_APERTURE_PAGES: u64 = 2048;

// The firmware screen driver's capability table. Must match components/src/framebuffer_driver.rs.
const SCREEN_SLOT_REPORT: u64 = 0;
const SCREEN_SLOT_DISPLAY: u64 = 1;
const SCREEN_SLOT_BUDGET: u64 = 2;
const SCREEN_SLOT_SURFACE: u64 = 3;
const _: () = assert!(SCREEN_SLOT_SURFACE < abi::fault::FAULT_EP_SLOT);

/// **The terminal on a screen the firmware already set up** (the shell on the firmware screen,
/// milestone 198's rung 1b): `framebuffer_driver` serving the framebuffer contract over `screen`,
/// and `display_terminal` on it exactly as [`start_terminal`] wires it over virtio-gpu. `None`, with
/// nothing spawned, when the screen cannot show a surface or its covered rows would need more than
/// [`MAX_APERTURE_PAGES`].
///
/// **`screen` must already be off the kernel's console** (`console::yield_screen`): the driver this
/// spawns paints the aperture, and two painters on one screen is the defect that function exists to
/// make unrepresentable.
///
/// The driver's world is its `Spawn` below, and the security argument is what it lacks: no
/// interrupt, no DMA, no transport, no physical address, and only the rows of the screen its
/// surface can reach (`screen_console::Aperture::span`), mapped at spawn so it holds no name for
/// them. The terminal's world is byte for byte the virtio path's.
pub fn start_screen_terminal(
    driver_image: &'static [u8],
    term_image: &'static [u8],
    screen: machine_discovery::framebuffer::Framebuffer,
) -> Option<TerminalWiring> {
    let aperture = screen_console::Aperture::new(
        &screen,
        graphics_protocol::WIDTH,
        graphics_protocol::HEIGHT,
    )?;
    // The aperture need not start on a page: the run is mapped from the page that holds pixel
    // (0, 0), and the driver is told how far into it that pixel is.
    let offset = screen.base % FRAME_SIZE;
    let pages = (offset + aperture.span() as u64).div_ceil(FRAME_SIZE);
    if pages > MAX_APERTURE_PAGES {
        crate::println!(
            "  screen    : not handed on: {pages} pages of aperture exceed this wiring's \
             {MAX_APERTURE_PAGES}"
        );
        return None;
    }

    // The surface: RAM, the contract's run of frames, shared by the driver and the terminal. The
    // same allocation `wire_driver` makes, minus the ring page, because nothing here is a device's
    // DMA: the driver copies it with the CPU.
    let surface =
        crate::memory::alloc_contiguous_zeroed(graphics_protocol::SURFACE_PAGE_FRAMES as usize)
            .expect("no contiguous surface for the framebuffer driver")
            .addr();

    // **The four endpoints come out of a region of their own**, `virtio_service::wire_net_server`'s
    // shape and for its reason: both programs park in `RECV` for good, and reclaiming the region
    // their endpoints live in is the only thing that wakes them to die. The boot never tears this
    // down; the suite does, because every service a test leaves standing is frames and region slots
    // a later test cannot have (`user::holding`). One page per endpoint, and one spare.
    let ep_region =
        crate::memory_region::create(5).expect("no endpoint region for the screen terminal");
    let endpoint = || {
        crate::sched::create_rendezvous_from(ep_region)
            .expect("no endpoint for the screen terminal")
    };
    let display_ep = endpoint(); // terminal WRITE (CALL) -> driver READ
    let driver_report = endpoint();
    let budget =
        crate::memory_region::create(MAP_BUDGET_PAGES).expect("no map budget for the driver");
    let (size, layout) = aperture.to_words();
    let device = DeviceRun {
        va: SCREEN_APERTURE_VA,
        phys: screen.base - offset,
        pages,
    };
    let driver_tid = crate::sched::spawn(move || {
        crate::sched::grant_at(
            SCREEN_SLOT_REPORT,
            rendezvous_cap(driver_report, Rights::WRITE),
        )
        .expect("screen driver slot 0 was occupied");
        crate::sched::grant_at(
            SCREEN_SLOT_DISPLAY,
            rendezvous_cap(display_ep, Rights::READ),
        )
        .expect("screen driver slot 1 was occupied");
        crate::sched::grant_at(SCREEN_SLOT_BUDGET, memory_region_cap(budget))
            .expect("screen driver slot 2 was occupied");
        grant_run(
            SCREEN_SLOT_SURFACE,
            surface,
            SURFACE_RUN,
            "the framebuffer driver",
        );
        run_with_device_run(
            driver_image,
            Spawn {
                arg0: size,
                arg1: layout,
                arg2: offset, // where pixel (0, 0) is in the first mapped page
                grants: &[],  // every one of them is placed above, at its own slot
                maps: &[],
            },
            device,
        )
    })
    .expect("could not spawn the framebuffer driver");

    let t = spawn_terminal(term_image, display_ep, surface, &endpoint);
    let mut held = super::holding::Holding::new();
    held.add_thread(driver_tid);
    held.add_thread(t.tid);
    held.add_region(ep_region);
    held.add_region_after_death(budget);
    held.add_region_after_death(t.budget);
    Some(TerminalWiring {
        driver_report,
        term_report: t.term_report,
        term: t.term,
        out: t.out,
        surface,
        held,
    })
}

impl TerminalWiring {
    /// **Play the application**: put `text` in the output page and `OP_WRITE` it.
    ///
    /// Returns when the terminal has drawn it and the GPU driver has put it on the scanout,
    /// because that is what the terminal contract says an `OP_WRITE` reply means (the bytes are
    /// on the console's side). So a test needs no polling and no sleep between writes.
    pub fn print(&self, text: &[u8]) {
        super::term_print(self.out, self.term, text);
    }

    /// **Play the input driver**: `OP_BYTES` these keystrokes, eight to a message.
    ///
    /// Byte for byte the framing `components/src/input.rs` sends and the compositor forwards
    /// (DECISIONS §33), which is the point: the display terminal is fed by the same driver half
    /// as the serial one, so neither contract had to grow anything to carry a keystroke to a
    /// screen.
    pub fn type_bytes(&self, bytes: &[u8]) {
        for chunk in bytes.chunks(8) {
            let mut w1 = 0u64;
            for (k, &b) in chunk.iter().enumerate() {
                w1 |= (b as u64) << (8 * k);
            }
            let w0 = line_editor::proto::req(line_editor::proto::OP_BYTES, chunk.len() as u64);
            assert_eq!(
                crate::sched::ipc_call(self.term, [w0, w1])[0],
                0,
                "the terminal refused a keystroke",
            );
        }
    }

    /// A scanout pixel, read by the **kernel** through the direct map: a witness that belongs to
    /// no process in userspace.
    pub fn screen_pixel(&self, x: u32, y: u32) -> u32 {
        let at = mmu::phys_to_virt(self.surface) + (y * graphics_protocol::WIDTH + x) as u64 * 4;
        // SAFETY: inside the scanout frames this kernel allocated.
        unsafe { core::ptr::read_volatile(at as *const u32) }
    }

    /// **The scanout holds exactly the picture `expect` describes.** Compared pixel for pixel
    /// rather than by digest, so a failure names a coordinate.
    pub fn assert_screen_is(&self, expect: &video_terminal::Vt, what: &str) {
        for y in 0..graphics_protocol::HEIGHT {
            for x in 0..graphics_protocol::WIDTH {
                let (got, want) = (self.screen_pixel(x, y), expect.pixel(x, y));
                assert_eq!(
                    got,
                    want,
                    "{what}: the framebuffer is wrong at ({x},{y}) [cell ({},{})]: {got:#010x}, \
                     expected {want:#010x}",
                    x / bitmap_font::GLYPH_W,
                    y / bitmap_font::GLYPH_H,
                );
            }
        }
    }
}
