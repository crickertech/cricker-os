//! **The display terminal** (milestone 29, the display ladder's text).
//!
//! The component that makes the framebuffer readable: it serves the terminal contract's IPC half
//! (notes/terminal-contract.md) against a **grid and a font** instead of a serial line, and puts the
//! result on a screen it does not own.
//!
//! ```text
//!   application ──OP_WRITE──►┌──────────┐──glyphs──► its surface ──FLUSH/COMMIT──► a screen
//!   keystrokes ──OP_BYTES───►│  display_terminal   │
//!                            └──────────┘
//! ```
//!
//! # It is a client, twice over, and that is the whole point
//!
//! The same binary runs in two wirings, chosen by `arg0`, and neither of them holds a device:
//!
//! - [`MODE_DISPLAY`]: it holds rung one's **display endpoint** and the
//!   scanout frames, exactly the authority `painter` had, and shows text on the whole screen. This
//!   is the wiring that answers "did the framebuffer contract need to change for text?" The answer
//!   is no: it draws pixels and calls `FLUSH` with a damage rectangle, which is what the contract
//!   already said a client does.
//! - [`MODE_WINDOW`]: it holds rung two's **doorbell** and one window's
//!   control page and surface, exactly the authority `window` had, and shows text in a window
//!   among mutually distrusting neighbours.
//!
//! In both, its whole world is: a report endpoint, one endpoint to present through, **one endpoint
//! it serves** (slot 2), a page an application writes text into, and its own pixels. No device, no
//! interrupt, no DMA authority, no physical address, and no way to name another client's anything.
//!
//! # One endpoint, because this process has one wait point
//!
//! A terminal has two classes of sender: an application printing, and an input source typing.
//! DECISIONS §33 recorded that a process here has exactly **one blocking wait point** (one `RECV`,
//! no wait-any, and two threads cannot share an address space), so distinguishing them by endpoint
//! is not available. They arrive on **one** endpoint and are distinguished by opcode, which is what
//! `line_editor` already does for the serial terminal, and the security consequence is stated rather than
//! hidden: an application holding this endpoint could send `OP_BYTES` and forge a keystroke into its
//! own terminal. It gains nothing by it (the keystrokes come back to the same grid it is already
//! printing on), and the boundary that matters, one client's input not reaching another's, is the
//! compositor's and is a capability there. See notes/glyphs.md.
//!
//! # Why input does not ring the doorbell, and the deadlock that taught us
//!
//! In [`MODE_WINDOW`] the compositor delivers a keystroke by **`CALL`ing this process** and blocking
//! until it replies (DECISIONS §33). So a terminal that answered a keystroke by ringing the doorbell
//! would deadlock the moment two keystrokes arrived in one drain: the compositor would be blocked in
//! its `CALL` to us while we were blocked in our `CALL` to it.
//!
//! It does not need to. The compositor rescans **every** client's control page on every `COMMIT`
//! from anyone, and the input source rings `COMMIT` itself after it fills the ring. So the frame
//! that delivers the keystroke is the frame that will show it: this process paints, records its
//! damage, bumps its sequence, and replies. Application output is different (nobody else is going to
//! ring for it), so `OP_WRITE` does ring, and that is safe because the caller blocked in `CALL` is
//! the *application*, not the compositor.
//!
//! Name: ratified 2026-08-01 (calef, milestone 63), replacing `vterm`. Refused `vterm` (an
//! abbreviation), `text_console` (`console` is already a program) and `video_terminal` for the
//! program (the crate is named for the protocol it implements and this program for its role, next
//! to `gpu_driver`, the virtio-gpu driver it is a client of, so the display ladder reads straight from
//! the filenames).

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

use compositor::proto::ctl;
use graphics_protocol as gfx;
use line_editor::proto;
use user_mode_runtime::mapped_window::MappedWindow;
use user_mode_runtime::{call, recv_cap, reply, send};
use video_terminal::status::{MODE_DISPLAY, MODE_WINDOW};

/// Capability slots, by convention with `kernel/src/user/display_service.rs` and
/// `compositor_service`.
const REPORT: u64 = 0;
/// The endpoint this terminal presents through: the display driver in [`MODE_DISPLAY`], the
/// compositor's doorbell in [`MODE_WINDOW`].
const PRESENT: u64 = 1;
/// The endpoint it serves. **One**, for both classes of sender; see the module note.
const TERM: u64 = 2;
/// The untyped it spends on the page tables its own mappings need. [`MODE_DISPLAY`] only; see the
/// note on [`SURFACE_FRAME`].
const BUDGET: u64 = 3;
/// The whole scanout, one `PageFrame` capability naming the `gfx::SURFACE_PAGE_FRAMES`-page run
/// (DECISIONS §102), then one more slot for [`OUT_VA`]'s page. **[`MODE_DISPLAY`] only**
/// (milestone 108).
///
/// The two wirings do not agree about this yet, and the asymmetry is deliberate rather than
/// overlooked: milestone 108 migrated the disk and display paths, and rung two's compositor is not
/// one of them. In [`MODE_WINDOW`] the surface, the output page and the control page still arrive as
/// spawn-time mappings from `compositor_service`, which is why the `MAP` calls below are inside the
/// `MODE_DISPLAY` arm.
const SURFACE_FRAME: u64 = 4;
const OUT_PAGE_FRAME: u64 = SURFACE_FRAME + 1;

/// Where the scanout goes. In [`MODE_DISPLAY`] this process holds the frames and picks the address;
/// in [`MODE_WINDOW`] the compositor's wiring maps it here.
///
/// **`OUT_VA`/`CTL_VA` moved past it** (milestone 142, DECISIONS §102): the scanout grew from 8
/// page frames to 900 (up to 4 MiB from `SURFACE_VA`), so the old `OUT_VA` (`0x68_0000`, inside
/// that span's old 512 KiB neighbourhood) would now be inside the middle of
/// [`MODE_DISPLAY`]'s own surface mapping, and its `PageFrame::MAP` would refuse it as
/// already-mapped. Both moved well clear; `SURFACE_VA`'s own 2 MiB alignment is unchanged and is
/// what keeps a run this large inside as few page-table windows as possible
/// (`display_service::MAP_BUDGET_PAGES`'s own comment has the arithmetic).
const SURFACE_VA: u64 = address_space_map::pair_page(0x0000_0000_0060_0000);
/// The page an application writes the bytes of an `OP_WRITE` into. The terminal contract's
/// "control by message, bulk by shared page" split (DECISIONS §10), the same one `filesystem_protocol` makes.
const OUT_VA: u64 = address_space_map::pair_page(0x0000_0000_0a00_0000);
/// The compositor's per-client control page. [`MODE_WINDOW`] only.
const CTL_VA: u64 = address_space_map::pair_page(0x0000_0000_0a01_0000);

/// Failure codes, in a `0xDEAD_...` word so a failure names its step rather than hanging.
const E_INFO: u64 = 0x01;
const E_GEOMETRY: u64 = 0x02;
const E_PRESENT: u64 = 0x03;
const E_HELLO: u64 = 0x04;
const E_MAGIC: u64 = 0x05;
const E_MODE: u64 = 0x06;
/// A frame this process holds would not map. [`MODE_DISPLAY`] only.
const E_SURFACE: u64 = 0x07;

/// **The grid lives in `.bss`, not on the stack.** A user process here gets one 4 KiB page of stack
/// (`kernel/src/user.rs`, `USER_STACK_VA`), and a `Vt` is a kilobyte before the temporary a move
/// would make. `video_terminal::Vt::new` is `const` precisely so this can be a `static`.
static mut TERMINAL: video_terminal::Vt = video_terminal::Vt::new(1, 1);

/// The one terminal this process owns.
///
/// # Safety
/// This process has exactly one thread (a `ThreadControlBlock` here owns its address space, DECISIONS §33), so
/// there is no second reference and no aliasing question.
fn term() -> &'static mut video_terminal::Vt {
    // A raw pointer first, then one dereference: taking `&mut TERMINAL` directly is what
    // `static_mut_refs` exists to refuse, and it refuses it for a real reason (a second reference
    // would be undefined behaviour, not merely untidy).
    let p = &raw mut TERMINAL;
    // SAFETY: see above; single-threaded by construction, so `p` is the only route to it.
    unsafe { &mut *p }
}

fn rd32(va: u64) -> u32 {
    // SAFETY: inside a page the kernel mapped into this process at spawn.
    unsafe { core::ptr::read_volatile(va as *const u32) }
}

fn wr32(va: u64, v: u32) {
    // SAFETY: as above, and the page is writable.
    unsafe { core::ptr::write_volatile(va as *mut u32, v) }
}

fn out_byte(i: usize) -> u8 {
    // SAFETY: `i` is below FRAME_SIZE, inside the application's output page.
    unsafe { core::ptr::read_volatile((OUT_VA + i as u64) as *const u8) }
}

fn die(code: u64) -> ! {
    send(REPORT, 0xDEAD_0000_0000_0000 | code, 0, 0);
    user_mode_runtime::exit();
}

/// **Paint a rectangle of the surface** from the engine's picture.
///
/// `stride` is the surface's bytes per row, which differs between the two wirings (the scanout's in
/// [`MODE_DISPLAY`], the window's in [`MODE_WINDOW`]) and is therefore *read* rather than assumed.
/// Ordinarily only the damaged rectangle is passed: everything outside it is already correct, and
/// repainting it would make the damage rectangle a decoration rather than a saving. The exception
/// is the first frame, which paints the whole surface; see [`Wiring::present`].
fn paint(
    surface: &MappedWindow,
    x0: u32,
    y0: u32,
    w: u32,
    h: u32,
    stride: u32,
) -> (u32, u32, u32, u32) {
    let t = term();
    for y in y0..y0 + h {
        for x in x0..x0 + w {
            let off = (y * stride) as u64 + (x * 4) as u64;
            // The rectangle is either the engine's damage or the whole surface this process asked
            // the geometry of, and `Vt::pixel` is defined outside its own grid (it is the default
            // background), which is what makes the second case safe as well as correct.
            surface.w32(off, t.pixel(x, y));
        }
    }
    (x0, y0, w, h)
}

/// A whole terminal's worth of state that the two wirings differ on. Small, and gathered here so the
/// serving loop below reads the same in both.
struct Wiring {
    mode: u64,
    stride: u32,
    /// [`MODE_WINDOW`]: damage this process has published but not seen composited yet, unioned so a
    /// second update before the compositor's next scan does not lose the first.
    pending: Option<video_terminal::CellRect>,
    seq: u32,
    /// The surface's size in pixels, which is **not** always a whole number of character cells.
    /// 128 is not a multiple of 7, so a full-scanout terminal owns 18 columns and leaves two
    /// pixels on the right that no cell covers. See [`Wiring::present`].
    surface: (u32, u32),
    /// The bounds-checked window onto the surface frames, sized to this wiring's own geometry
    /// (milestone 139 round 4; see the `SAFETY` comment where `_start` constructs it).
    window: MappedWindow,
    /// Has the whole surface been painted once? Until it has, the strip outside the grid holds
    /// whatever the frames held at boot.
    painted_all: bool,
}

impl Wiring {
    /// **Put whatever changed on the screen.**
    ///
    /// The two wirings are two sentences at the same seam. `MODE_DISPLAY` calls rung one's
    /// `FLUSH(rect)` on the display endpoint, which is the request `painter` made and the driver has
    /// honoured since rung one; `MODE_WINDOW` writes the rectangle into its own control page, bumps
    /// its sequence, and rings `COMMIT`, which is the request `window` made. Neither contract needed
    /// a change to carry text, because both carry pixels.
    ///
    /// `ring` is false for a keystroke: see the module note on the deadlock.
    fn present(&mut self, ring: bool) {
        let Some(damage) = term().take_damage() else {
            return;
        };
        // **The first frame paints the whole surface, not just the grid.**
        //
        // A 7-pixel cell does not divide a 128-pixel scanout, so a full-width terminal is 18
        // columns of 7 and two pixels wide of nothing. `Vt::pixel` answers for those two (a cell
        // outside the grid is a blank on the default background, which is what makes the picture a
        // total function), but no cell ever *damages* them, so without this they would keep
        // whatever the frame held at boot: a strip of noise beside the text that looks like a
        // rendering bug for a day. The grid can never write there afterwards, so once is enough.
        let (x, y, w, h) = if self.painted_all {
            let (x, y, w, h) = damage.to_pixels();
            paint(&self.window, x, y, w, h, self.stride)
        } else {
            self.painted_all = true;
            paint(
                &self.window,
                0,
                0,
                self.surface.0,
                self.surface.1,
                self.stride,
            )
        };

        // The pixels must be visible to whoever reads them next: another address space, and through
        // it a device. A release fence is the portable way to say so (`dmb ish` on aarch64, `fence`
        // on RISC-V), and it belongs in this userspace program rather than in arch code (rule 1).
        //
        // PAIR: whichever of the two paths below runs. `MODE_DISPLAY` pairs with `barrier()` in
        // components/src/gpu_driver.rs, the same as the compositor's `flush`; `MODE_WINDOW` pairs with
        // `serve_frame` in components/src/compositor.rs. Both are also ordered by the `call` each path
        // makes, when it makes one; the second fence below is the case where it does not.
        core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);

        if self.mode == MODE_DISPLAY {
            let (r0, _) = call(
                PRESENT,
                gfx::req(gfx::display::FLUSH, gfx::rect(x, y, w, h)),
                0,
            );
            if r0 as i64 != 0 {
                // The driver refuses a rectangle outside the surface rather than clamping it, so
                // this is our arithmetic being wrong. Fail loudly.
                die(E_PRESENT);
            }
            return;
        }

        // MODE_WINDOW. The compositor publishes the sequence it has composited, so damage that has
        // not been acknowledged is carried forward rather than overwritten.
        if rd32(CTL_VA + ctl::ACKED) == self.seq {
            self.pending = None;
        }
        let d = match self.pending {
            Some(p) => p.union(damage),
            None => damage,
        };
        self.pending = Some(d);
        // The rectangle published is the one painted, which on the first frame is the whole
        // surface rather than the grid, for the reason above.
        let (x, y, w, h) = if self.seq == 0 {
            (x, y, w, h)
        } else {
            d.to_pixels()
        };
        wr32(CTL_VA + ctl::DAMAGE_X, x);
        wr32(CTL_VA + ctl::DAMAGE_Y, y);
        wr32(CTL_VA + ctl::DAMAGE_W, w);
        wr32(CTL_VA + ctl::DAMAGE_H, h);
        self.seq += 1;
        // The sequence must become visible after the pixels and the rectangle that describes them,
        // or the compositor could composite a frame we have not finished writing.
        //
        // PAIR: `serve_frame` in components/src/compositor.rs, which milestone 43's audit found had no
        // fence (finding 7). **Read `ring` before assuming a rendezvous covers this one.** On a
        // keystroke `ring` is false and no `call` follows, so the only edge is the reply this
        // process is about to send to the compositor that `CALL`ed it. That reply does order it. The
        // case the reader's fence is genuinely load-bearing for is a *second* window committing
        // while `serve_frame` rescans every client's page; see the note in fixtures/src/window.rs and
        // notes/memory-ordering.md.
        core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
        wr32(CTL_VA + ctl::SEQ, self.seq);
        if ring {
            let (r0, _) = call(
                PRESENT,
                compositor::proto::req(compositor::proto::COMMIT, 0),
                0,
            );
            if r0 as i64 != 0 {
                die(E_PRESENT);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn _start(mode: u64, _arg1: u64, _arg2: u64) -> ! {
    // **The geometry is asked for, never assumed**, in both wirings. Rung one added `INFO` for this
    // and rung two publishes a control page for it, and the reason is the same in both: the process
    // that owns the screen decides how big a surface is, and a client that guessed would paint the
    // wrong shape the first time anything changed.
    let (w, h, stride) = match mode {
        MODE_DISPLAY => {
            // The scanout and the application's output page are `PageFrame`s this process holds, and it
            // maps them itself out of its own budget (milestone 108). Before the `INFO` call,
            // because a terminal with nowhere to paint has no use for the geometry. One `MAP` call
            // for the whole scanout run (DECISIONS §102), not one per page.
            if !user_mode_runtime::map_page_frame(SURFACE_FRAME, SURFACE_VA, true, BUDGET) {
                die(E_SURFACE);
            }
            if !user_mode_runtime::map_page_frame(OUT_PAGE_FRAME, OUT_VA, true, BUDGET) {
                die(E_SURFACE);
            }
            let (r0, geometry) = call(PRESENT, gfx::req(gfx::display::INFO, 0), 0);
            if r0 as i64 != 0 {
                die(E_INFO);
            }
            let (w, h) = ((geometry & 0xffff_ffff) as u32, (geometry >> 32) as u32);
            (w, h, gfx::STRIDE)
        }
        MODE_WINDOW => {
            // `HELLO` first: its reply cannot arrive until the compositor is serving the doorbell,
            // and the compositor publishes every control page before it starts serving. The
            // rendezvous is the synchronization, so nothing polls for a valid page.
            let (r0, _) = call(
                PRESENT,
                compositor::proto::req(compositor::proto::HELLO, 0),
                0,
            );
            if r0 as i64 != 0 {
                die(E_HELLO);
            }
            if rd32(CTL_VA + ctl::MAGIC) != ctl::MAGIC_VALUE {
                die(E_MAGIC);
            }
            (
                rd32(CTL_VA + ctl::WIDTH),
                rd32(CTL_VA + ctl::HEIGHT),
                rd32(CTL_VA + ctl::STRIDE),
            )
        }
        _ => die(E_MODE),
    };

    // A surface too small for one character has nothing to show and is refused. It does **not**
    // have to be a whole number of cells: the font is 7 wide and the scanout is 128, so the normal
    // case now has a two-pixel strip on the right that no cell covers. `present` paints it once
    // with the background the engine says is there, which is why a partial cell is a defined
    // picture rather than a strip of whatever the frames held at boot.
    if w < bitmap_font::GLYPH_W || h < bitmap_font::GLYPH_H {
        die(E_GEOMETRY);
    }
    let (cols, rows) = (w / bitmap_font::GLYPH_W, h / bitmap_font::GLYPH_H);
    if cols as usize > video_terminal::MAX_COLS || rows as usize > video_terminal::MAX_ROWS {
        die(E_GEOMETRY);
    }
    // `reset_to`, not `*term() = Vt::new(cols, rows)` (milestone 142): `cols`/`rows` are runtime
    // values negotiated with the driver or the compositor, so `Vt::new` here would run as an
    // ordinary function call rather than being evaluated at compile time, and its return value (a
    // `Vt` is hundreds of KiB now, see `Vt`'s own doc) would need to exist somewhere at runtime.
    // This process gets one 4 KiB page of stack; `reset_to` mutates the existing `static` in place
    // instead, so no `Vt`-sized value is ever a stack local or a return value here.
    term().reset_to(cols, rows);

    // SAFETY: MODE_DISPLAY mapped `gfx::SURFACE_FRAMES` frames (`gfx::SURFACE_BYTES` bytes) at
    // SURFACE_VA itself, in the `MAP` loop above; MODE_WINDOW's frames are mapped by the
    // compositor's `spawn_client_term` before `HELLO`'s reply arrived, sized to the same geometry
    // (`stride`, `h`) this process just read off the control page it validated above (milestone 139
    // round 4). `stride * h` stays inside what was mapped in both wirings: it is exactly the bound
    // in the first case, and it is the compositor's own published geometry, which is what it sized
    // the mapping to, in the second.
    let window = unsafe {
        MappedWindow::new(
            SURFACE_VA,
            if mode == MODE_DISPLAY {
                gfx::SURFACE_BYTES as u64
            } else {
                stride as u64 * h as u64
            },
        )
    };

    let mut wiring = Wiring {
        mode,
        stride,
        pending: None,
        seq: 0,
        surface: (w, h),
        window,
        painted_all: false,
    };
    // The blank grid is a *defined* picture (spaces on the default background), so presenting it
    // before anyone has written a byte means the screen a spawner sees is black rather than whatever
    // was in those frames. A fresh `Vt` reports its whole grid as damage for exactly this reason.
    wiring.present(true);
    send(
        REPORT,
        video_terminal::status::TERM_UP,
        cols as u64 | ((rows as u64) << 32),
        mode,
    );

    loop {
        // The one wait point. An application's `OP_WRITE` and an input source's `OP_BYTES` both
        // arrive here and are told apart by opcode, because there is no wait-any to tell them apart
        // by endpoint (DECISIONS §33).
        let (w0, reply_slot, w1) = recv_cap(TERM);
        let mut r0: u64 = 0;
        match proto::op(w0) {
            // The application half: print `len` bytes from the shared output page. The terminal
            // performs no newline translation, deliberately: `line_editor::expand_output` already put
            // `\r\n` on the wire for a Unix `\n`, and a second translation here would move the
            // carriage twice. The engine treats a bare `LF` as a line feed, which is what a VT does.
            proto::OP_WRITE => {
                let n = proto::len(w0).min(4096);
                for i in 0..n {
                    let b = [out_byte(i)];
                    term().feed(&b);
                }
                r0 = n as u64;
                // Ring: nobody else is going to ask for this frame.
                wiring.present(true);
            }
            // The driver half: one to eight raw wire bytes, packed little-endian in the second word.
            // Byte for byte the framing `input.rs` sends and the compositor forwards, so this
            // terminal is a focusable compositor client without either contract changing.
            proto::OP_BYTES => {
                let n = proto::len(w0).min(8);
                for k in 0..n {
                    let b = [((w1 >> (8 * k)) & 0xff) as u8];
                    term().feed(&b);
                }
                // Do NOT ring in MODE_WINDOW: the compositor is blocked in its CALL to us. See the
                // module note. In MODE_DISPLAY there is no such caller, so a flush is safe and
                // necessary.
                wiring.present(wiring.mode == MODE_DISPLAY);
            }
            // `OP_READLINE` and `OP_INTRCOUNT` are the line discipline's, and this component is not
            // one: it renders a stream and echoes keystrokes. A client that wants edited lines puts
            // `line_editor` in front of this and prints its echo through `OP_WRITE`, which needs no new
            // protocol because `line_editor`'s echo is exactly a byte stream this engine parses (the
            // `video_terminal` crate's interoperability test proves that on the host). Recorded as a limit in
            // notes/glyphs.md rather than half-implemented.
            _ => r0 = proto::BAD_REQUEST,
        }
        reply(reply_slot, r0, 0);
    }
}

user_mode_runtime::panic_handler!();
