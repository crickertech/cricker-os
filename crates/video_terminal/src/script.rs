//! **What the milestone-29 tests put on the screen**, in one place so every party agrees.
//!
//! Text on a screen can only be *proved* if the picture is a value more than one independent
//! witness can compute. These constants are that value's input. The kernel-side test sends these
//! bytes to the terminal and then predicts the framebuffer by running the engine itself; the
//! host-side scanout check runs the same engine over the same bytes and grades what QEMU is
//! actually displaying. Neither reads the other's answer, and the terminal component that draws
//! holds none of this: it renders whatever arrives.
//!
//! This is the same move `graphics_protocol::pixel` makes for rung one's test pattern and `compositor::SCENE`
//! for rung two's scene: the test's content lives in the contract crate, not in the test.

/// The columns of a terminal that owns the whole scanout: `graphics_protocol::WIDTH / bitmap_font::GLYPH_W`.
///
/// Spelled as a literal here rather than computed, because this crate deliberately does not depend
/// on the framebuffer contract (a VT engine that knew about a scanout would be the wrong shape).
/// `components/src/display_terminal.rs` and the kernel wiring both assert the two agree at **compile time**, so a
/// screen that changed size is a build error rather than a terminal quietly missing its last column.
///
/// **132, no remainder**: 924 / 7 is exactly 132, unlike 1280 / 7's six leftover pixels (and 128 /
/// 7's two).
///
/// **Grown from 18 to 182 at milestone 142's increment 1, then retargeted to 132 on 2026-08-27**
/// (`graphics_protocol::WIDTH`'s doc comment has the full story: 182 was arithmetic against a *future*
/// 14-pixel cell that never shipped in this increment, applied by mistake to the 7x8 cell that did,
/// producing a grid nearly double any terminal anyone runs). 132 columns is the classic VT100/VT220
/// "wide mode" size at today's 7x8 bitmap font (`bitmap_font::GLYPH_W`/`GLYPH_H`, unchanged by this
/// retarget), and still clears the 80-column floor with real room. When the atlas lands and the
/// cell widens, this constant shrinks with it.
pub const COLS: u32 = 132;
/// The rows of a terminal that owns the whole scanout. See [`COLS`]: 344 / 8 is 43 exactly, no
/// remainder, unlike 64 / 8 which was also exact. Grown from 8 to 90, then retargeted to 43 (the
/// VT100/VT220 "wide mode" row count).
pub const ROWS: u32 = 43;

/// **What the application prints.** Delivered as `OP_WRITE`, the terminal contract's application
/// half (notes/terminal-contract.md), exactly as a program printing to a serial console would.
///
/// Chosen so the picture is hard to produce by accident:
///
/// - **four rows of text**, so a stride error or a one-row shift is visible (one row would not be);
/// - **five renditions**: a 24-bit foreground, default, a green foreground, an underline, and a
///   reversed block. A terminal that ignored SGR would draw every glyph correctly and still fail,
///   and since milestone 142 (a text display good enough that people use it instead of a GUI)'s
///   truecolour pass (2026-09-26) so would one that dropped `38;2` or drew underlined text without
///   its line. The 24-bit value is Solarized's yellow (`#b58900`), from §104 (the palette is
///   Solarized), and it is in neither the sixteen-colour palette nor the 256-colour table, so a
///   terminal that rounded it to the nearest indexed colour fails too;
/// - **a `\r\n` pair**, which is what `line_editor::expand_output` puts on the wire for a Unix `\n`, so
///   this is the byte stream a real program's output actually becomes;
/// - **descenders and an underscore** (`y`, `_`), which are the glyph rows a font table truncated to
///   seven rows would lose.
pub const GREETING: &[u8] =
    b"\x1b[38;2;181;137;0mnife\x1b[0m\r\n\x1b[32mglyphs_ok\x1b[0m\r\nby a \x1b[4mvt\x1b[24m\r\n\x1b[7mFOCUS\x1b[0m";

/// **What the user types.** Delivered as `OP_BYTES`, the terminal contract's driver half, which is
/// the same framing the compositor uses to route a keystroke to the focused client (DECISIONS §33).
/// Echoed into the grid, so a keystroke that never arrived is a missing letter on the screen rather
/// than a silent nothing.
pub const TYPED: &[u8] = b"\r\n> hi";

/// **A wrong picture, for the negative control.** The greeting with one letter changed.
///
/// The host-side scanout check has to reject this, and that is the assertion that makes the whole
/// text proof mean something: a checker that only asked "is the screen not blank?" would pass every
/// run, including the ones where the terminal drew the wrong thing. One letter, because a wholly
/// different screen would be rejected by a much weaker check.
pub const GREETING_TYPO: &[u8] =
    b"\x1b[38;2;181;137;0mnife\x1b[0m\r\n\x1b[32mglyphs_0k\x1b[0m\r\nby a \x1b[4mvt\x1b[24m\r\n\x1b[7mFOCUS\x1b[0m";

/// **The banner each terminal in the compositor test prints**, indexed by window.
///
/// Different text per window on purpose: it is what turns "the screen has text on it" into "each
/// window has *its own* text on it", so an `OP_WRITE` delivered to the wrong terminal is a wrong
/// picture rather than a duplicate one.
pub const WINDOW_BANNER: [&[u8]; 2] = [b"term0", b"term1"];

/// **What is typed at each terminal in the compositor test** after focus moves to it.
///
/// The letters differ so that focus routing is checkable from the *picture*: an `A` appearing in
/// window 1, or a `B` in both, is a compositor that sent a keystroke to a client that should not
/// have received it. Since a client can only be sent input because it *holds* an input endpoint
/// (DECISIONS §33), this is the capability claim made visible in pixels.
pub const WINDOW_TYPED: [&[u8]; 2] = [b"\r\nA", b"\r\nB"];

/// **The key the host presses on the real keyboard device**, as QEMU's monitor `sendkey` names it.
///
/// The keyboard test's one genuinely host-driven input: nothing in the guest can press a key, so
/// `cargo xtask` sends this on the monitor beside the suite, the same connection the scanout check
/// already uses. Shared here so the side that presses and the side that asserts cannot disagree
/// about which key it was.
pub const HOST_KEY: &str = "a";

/// The byte [`HOST_KEY`] must arrive as, once the driver's evdev event has been through
/// [`crate::keymap`]. Unshifted, so this is the plain letter.
pub const HOST_KEY_BYTE: u8 = b'a';

/// **Feed the greeting, then the typing, into `vt`**: what a full-scanout wiring shows after the
/// whole script. The caller constructs `vt` (typically `Vt::new(COLS, ROWS)`, compile-time
/// constants, so a `static` costs nothing at runtime) rather than this function returning one.
///
/// **Takes `&mut Vt`, not `-> Vt`, since milestone 142's grid growth.** A `Vt` is now hundreds of
/// KiB (`Vt`'s own doc comment), so a function that built one and returned it by value would need
/// that whole value to exist somewhere at the call site; a kernel test calling this on a 24 KiB
/// thread stack is exactly the caller this signature protects (`script/stack-frame-check` is the
/// gate that would have caught the old shape once the grid grew, and this signature is why it does
/// not have to).
pub fn full_screen(vt: &mut crate::Vt) {
    vt.feed(GREETING);
    vt.feed(TYPED);
}

/// **Feed window `i`'s banner and its typed text into `vt`**: what its terminal shows in the
/// compositor test. `vt` must already be at the right geometry (`Vt::reset_to(cols, rows)`, since a
/// window's size is not known until runtime, unlike [`full_screen`]'s fixed one); this only feeds
/// the script, for [`full_screen`]'s own reason (a `Vt`-by-value return is no longer a cheap thing
/// to hand back).
pub fn window(vt: &mut crate::Vt, i: usize) {
    vt.feed(WINDOW_BANNER[i]);
    vt.feed(WINDOW_TYPED[i]);
}
