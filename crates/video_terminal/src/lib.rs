//! **The VT state engine: the grid a display terminal keeps** (milestone 29, the display ladder's
//! text).
//!
//! Bytes in, a character grid out, plus the rectangle that changed. Sans-IO, exactly as the
//! `line_editor` crate is: this crate holds no endpoint, makes no syscall, and has never heard of a
//! framebuffer. `components/src/display_terminal.rs` feeds it and paints what it says.
//!
//! # Why this shape
//!
//! DECISIONS §7: pure logic belongs in a crate that compiles for the host, so most tests run in
//! milliseconds. A VT engine is almost entirely pure logic (a parser and a two-dimensional array),
//! and the parts that are not (a shared surface, an IPC endpoint) are the parts a QEMU boot has to
//! prove anyway.
//!
//! It buys something specific here beyond fast tests. Because the engine is a value with no IO, the
//! **expected picture is computable by anyone holding the same script**: the terminal component runs
//! it to draw, the kernel-side test runs it to predict what should be in the framebuffer, and
//! `cargo xtask` runs it on the host to grade what QEMU is actually scanning out. Three independent
//! witnesses, one definition, no possibility of the three agreeing on a wrong answer.
//!
//! # Examples
//!
//! Bytes in, a grid out, and the rectangle that changed. Because the engine is a value with no IO,
//! the expected picture is computable by anyone holding the same bytes, which is what lets three
//! independent witnesses agree about what should be on the screen.
//!
//! ```
//! use video_terminal::Vt;
//!
//! let mut vt = Vt::new(8, 4);
//! vt.feed(b"hello\r\nworld");
//!
//! let mut row = [0u8; 8];
//! let n = vt.row_bytes(0, &mut row);
//! assert_eq!(&row[..n], b"hello   "); // the rest of the row is blanks, not garbage
//! vt.row_bytes(1, &mut row);
//! assert_eq!(&row[..5], b"world");
//! assert_eq!(vt.cursor(), (5, 1));
//!
//! // Damage is in cells and is taken, not read: the caller repaints exactly what changed and the
//! // record clears. That is the whole reason the engine reports a rectangle instead of "redraw".
//! let dirty = vt.take_damage().expect("something changed");
//! assert!(dirty.rows >= 2, "two rows were written to");
//! assert_eq!(vt.take_damage(), None); // taken once
//! ```
//!
//! **Deferred wrap** is the one subtlety in printing, and it is the reason a line that exactly fills
//! the width does not scroll the screen before anything asked it to:
//!
//! ```
//! use video_terminal::Vt;
//!
//! let mut vt = Vt::new(4, 4);
//! vt.feed(b"abcd"); // exactly fills row 0
//!
//! // The cursor stays on the last cell it wrote, with a wrap pending. It has NOT moved to row 1.
//! assert_eq!(vt.cursor(), (3, 0));
//!
//! // A CR arriving now finds the cursor on row 0, which is where a line discipline expects it. If
//! // the wrap had already happened it would be a row too low.
//! vt.feed(b"\r");
//! assert_eq!(vt.cursor(), (0, 0));
//!
//! // And when the next printable byte does arrive without an intervening CR, it wraps first.
//! let mut vt = Vt::new(4, 4);
//! vt.feed(b"abcde");
//! assert_eq!(vt.cursor(), (1, 1));
//! let mut row = [0u8; 4];
//! vt.row_bytes(1, &mut row);
//! assert_eq!(row[0], b'e');
//! ```
//!
//! The escape sequences are the ones a line discipline actually emits, plus the screen verbs any
//! program expects:
//!
//! ```
//! use video_terminal::Vt;
//!
//! let mut vt = Vt::new(8, 4);
//! vt.feed(b"one\r\ntwo\r\n");
//!
//! // CSI H homes the cursor; CSI 2J clears the screen.
//! vt.feed(b"\x1b[2J\x1b[H");
//! assert_eq!(vt.cursor(), (0, 0));
//! let mut row = [0u8; 8];
//! let n = vt.row_bytes(0, &mut row);
//! assert_eq!(&row[..n], b"        ");
//! ```
//!
//! # What it implements, and why exactly this set
//!
//! The escape sequences here are **the ones the line discipline already emits** (DECISIONS §21,
//! notes/terminal-contract.md), plus the screen verbs any program expects. That is not a guess: the
//! interoperability test in this crate runs the real `line_editor` and feeds its echo stream to this
//! parser, so the two components are checked against each other rather than against a list somebody
//! wrote down.
//!
//! - Printable bytes, with **deferred wrap** at the right margin (see [`Vt::feed`]) and **UTF-8
//!   decoding** (milestone 142 increment 2): a multi-byte sequence occupies one cell, drawn as
//!   [`bitmap_font::glyph`]'s missing-glyph box for anything past basic latin, which is this font's
//!   whole repertoire (see `bitmap_font::glyph`'s own doc).
//! - `CR`, `LF` (with scrolling into [`SCROLLBACK_ROWS`] of history, milestone 142 increment 2), `BS`,
//!   `TAB`, `BEL` (ignored: there is no bell here).
//! - `CSI A/B/C/D` cursor motion, `CSI H` / `CSI f` absolute positioning.
//! - `CSI J` erase in display, `CSI K` erase in line, both with all three modes.
//! - `CSI m` (SGR): reset; bold, dim, underline, reverse, concealed and crossed-out, each with its
//!   off switch; the eight ANSI colours and their bright forms, foreground and background; and the
//!   256-colour (`38;5;n`) and 24-bit (`38;2;r;g;b`) forms of both (milestone 142 (a text display
//!   good enough that people use it instead of a GUI), the 2026-09-26 pass). Italic and blink are
//!   parsed and dropped, since italic needs a face this font does not have and blink needs a clock
//!   this engine does not read.
//! - `ESC c` (RIS), a full reset.
//!
//! Anything else is **swallowed whole**, introducer and all, rather than printed as garbage.
//!
//! # What deliberately is NOT here
//!
//! No alternate screen, no origin mode, no scrolling regions, no tab stops other than every eight
//! columns, no mouse, and no reporting sequences at all: this engine never writes to its input,
//! which is what "sans-IO" means here and what keeps it a *value*. No reflow: `MAX_COLS`/`MAX_ROWS`
//! are fixed, and nothing here resizes a live grid. The honest limits are listed in
//! notes/glyphs.md.
//!
//! Name: ratified 2026-08-01 (calef, milestone 63), replacing `vt`. Refused `vt` (two letters that
//! read as *vector table* in a kernel), `virtual_terminal` (wrong twice: that is not what VT stood
//! for, and "virtual terminal" already names Linux's virtual consoles) and `screen_grid` (the crate
//! carries 63 escape-sequence references, so the grid is the output and interpreting the protocol
//! is the work). Deliberately not the program's name: this crate is named for the protocol it
//! implements (DEC's Video Terminals) and `display_terminal` for its role.

#![no_std]

pub mod keymap;
pub mod script;

// ================================================================================================
// Geometry.
// ================================================================================================

/// The widest grid this engine can hold, in cells.
///
/// Fixed rather than allocated, because the terminal component has no allocator and this crate is
/// the same code the kernel and the host run. **Grown from 32 to 182 at milestone 142's increment
/// 1, then retargeted to 132 on 2026-08-27** alongside the scanout (128x64 -> 1280x720 -> 924x344,
/// `graphics_protocol::WIDTH`'s doc comment has the full story). 132 is exactly
/// `graphics_protocol::WIDTH / bitmap_font::GLYPH_W` (924 / 7), the classic VT100/VT220 "wide mode"
/// column count and roughly what a real terminal actually runs, unlike 182's near-double of any
/// terminal anyone uses. A screen bigger than that gets a bigger constant, and the terminal
/// component asserts its own geometry fits at compile time so the failure is a build error rather
/// than a truncated screen.
pub const MAX_COLS: usize = 132;
/// The tallest grid this engine can hold, in cells. See [`MAX_COLS`]. Grown from 16 to 90, then
/// retargeted to 43 (924x344's `HEIGHT` / 8, the font's row height), the VT100/VT220 "wide mode"
/// row count.
pub const MAX_ROWS: usize = 43;
/// Cells in the largest possible **live** grid (the on-screen viewport; see [`SCROLLBACK_ROWS`] for
/// the off-screen history alongside it). A real cost, paid once per terminal instance and
/// auto-provisioned from that program's own region (`kernel/src/user.rs`'s `load` sizes a process's
/// address-space region from its ELF segments, `.bss` included), not from any shared budget.
pub const MAX_CELLS: usize = MAX_COLS * MAX_ROWS;

/// **Off-screen history**, in whole rows, kept alongside the live grid (milestone 142 increment 2).
///
/// A fixed ring rather than a `Vec`, for the same reason the live grid is a fixed array: this crate
/// reaches no allocator, so the capacity is a constant three parties (the terminal, the kernel test,
/// the host-side check) already agree on the same way they agree on [`MAX_COLS`]/[`MAX_ROWS`].
///
/// **300, chosen as a working depth rather than derived from anything.** At [`MAX_COLS`] (132) and
/// sixteen bytes a [`Cell`] that is 633,600 bytes of `.bss` (it was half that before the truecolour
/// pass widened the cell, 2026-09-26), and a whole `Vt` is 724,416 bytes, up from 362,208. That is
/// 177 page frames per terminal instance, a small fraction of the free page-frame pool a terminal's
/// own region draws from (see notes/frames.md's measurement that hundreds of page frames are "under
/// one percent of the free pool"). The kernel's test image holds eight `Vt` statics as witnesses
/// (`kernel/src/user/display_tests.rs` and `compositor_tests.rs`), so the widening cost it about
/// 2.9 MB of `.bss` against QEMU's 256 MiB. There is no principled reason it could not be larger or smaller; it is a constant a
/// future lane can change without touching the shape of the ring around it.
pub const SCROLLBACK_ROWS: usize = 300;
/// Cells in the scrollback ring. See [`SCROLLBACK_ROWS`].
pub const SCROLLBACK_CELLS: usize = MAX_COLS * SCROLLBACK_ROWS;

// ================================================================================================
// Colour.
// ================================================================================================

/// **The sixteen ANSI colours**, as `0x00RRGGBB` words in the surface's own pixel format.
///
/// Indices 0..8 are the normal colours in the usual ANSI order (black, red, green, yellow, blue,
/// magenta, cyan, white) and 8..16 their bright forms. The values are the widely used xterm set
/// rather than pure primaries, because pure primaries on a 128x64 surface make a *pretty* screen and
/// a bad test: two channels at 0 and one at 255 means half the failure modes (a swapped channel, a
/// dropped shift) produce another legal colour. These have all three channels distinct in most
/// entries.
///
/// # BUGS
///
/// **That last sentence is false, measured 2026-08-19, and the argument above does not hold as
/// written.** *No* entry has three distinct channel values: every one of the sixteen is built from
/// at most two levels (`0xcd0000` is `cd,00,00`; `0xe5e5e5` is one level three times). And **eight
/// pairs are related by a channel permutation**, so a swapped channel turns one legal palette colour
/// into another legal palette colour, which is exactly the failure this palette claims to catch.
/// `(1, 2)` is red and green: swap red and green and red becomes green, undetected.
///
/// The *shape* of the argument is right and the palette does not implement it. What it does buy is
/// real but smaller: values at `0xcd` and `0xe5` rather than `0xff` mean a dropped shift or a
/// saturating write lands off-palette. **Nothing gates any of this**, which is why a false claim sat
/// in a comment; a check that every entry has three distinct channels, and that no two entries are
/// permutations of each other, is a few lines and is milestone 141's first piece.
///
/// This matters beyond tidiness because the claim is what makes the palette ugly on purpose. See
/// design/roadmap/141-a-palette-worth-looking-at.md.
pub const PALETTE: [u32; 16] = [
    0x0000_0000, // 0 black
    0x00cd_0000, // 1 red
    0x0000_cd00, // 2 green
    0x00cd_cd00, // 3 yellow
    0x0000_00ee, // 4 blue
    0x00cd_00cd, // 5 magenta
    0x0000_cdcd, // 6 cyan
    0x00e5_e5e5, // 7 white
    0x007f_7f7f, // 8 bright black (grey)
    0x00ff_5c5c, // 9 bright red
    0x0000_ff00, // 10 bright green
    0x00ff_ff00, // 11 bright yellow
    0x005c_5cff, // 12 bright blue
    0x00ff_00ff, // 13 bright magenta
    0x0000_ffff, // 14 bright cyan
    0x00ff_ffff, // 15 bright white
];

/// The foreground a reset terminal writes with.
pub const DEFAULT_FG: u8 = 7;
/// The background a reset terminal clears to. Black, so an unwritten cell is the darkest thing on
/// the screen and a blank terminal is a defined picture rather than whatever the frames held.
pub const DEFAULT_BG: u8 = 0;

// ================================================================================================
// Cells.
// ================================================================================================

/// **One colour a cell can name**: an entry in the 256-colour table, or a 24-bit value.
///
/// The two are the two ways a program asks for colour (milestone 142, the 2026-09-26 pass): the
/// indexed form is what SGR 30..37, 90..97 and `38;5;n` carry, and the 24-bit form is what `38;2;r;g;b`
/// carries, which is what every syntax highlighter emits and was this terminal's single largest gap.
/// Four bytes, alignment one, so the rendition that holds two of them stays small.
///
/// Name: provisional (milestone 142's lane, 2026-09-26). British spelling to match the rest of this
/// crate ([`Attr::colours`], [`PALETTE`]'s doc).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Colour {
    /// An index into the 256-colour table: 0..16 are [`PALETTE`], 16..232 the 6x6x6 cube, 232..256
    /// the grey ramp. See [`Colour::resolve`].
    Indexed(u8),
    /// A 24-bit colour, red, green and blue.
    Rgb(u8, u8, u8),
}

/// One channel of the 256-colour cube at step `v` (0..6): `0, 0x5f, 0x87, 0xaf, 0xd7, 0xff`.
const fn cube_level(v: u32) -> u32 {
    if v == 0 { 0 } else { 0x37 + 40 * v }
}

impl Colour {
    /// **The `0x00RRGGBB` word this colour paints**, in the surface's own pixel format.
    ///
    /// The indexed range above sixteen is xterm's, computed rather than tabulated because it is
    /// arithmetic: the cube's six levels per channel are `0, 0x5f, 0x87, 0xaf, 0xd7, 0xff` (a step of
    /// 40 after an uneven first one), and the ramp is `0x08 + 10 * k` for 24 greys that stop short of
    /// both black and white, which the cube already has. Every emulator that implements `38;5;n`
    /// agrees on these numbers, which is why a program's 256-colour theme looks the same here.
    ///
    /// Name: provisional (milestone 142's lane, 2026-09-26).
    pub const fn resolve(self) -> u32 {
        match self {
            Colour::Rgb(r, g, b) => (r as u32) << 16 | (g as u32) << 8 | b as u32,
            Colour::Indexed(i) if i < 16 => PALETTE[i as usize],
            Colour::Indexed(i) if i < 232 => {
                let n = i as u32 - 16;
                cube_level(n / 36) << 16 | cube_level(n / 6 % 6) << 8 | cube_level(n % 6)
            }
            Colour::Indexed(i) => {
                let v = 0x08 + 10 * (i as u32 - 232);
                v << 16 | v << 8 | v
            }
        }
    }
}

/// **How a cell is painted**: two colours, and the renditions that change how they are used.
///
/// Nine bytes (two [`Colour`]s and a byte of flags), where it was one byte of palette indices
/// before milestone 142's 2026-09-26 pass: truecolour does not fit in less, and a [`Cell`] is
/// sixteen bytes either way once a `char` is beside it. The cost is measured where it lands, in
/// [`SCROLLBACK_ROWS`]'s doc.
///
/// **Bold is bright, and that is a decision rather than a shortcut.** A bold weight needs a second
/// font, and in a five-column cell a bold face is a smudge; every terminal from the DEC VT onward
/// has answered SGR 1 by brightening instead, which is why the palette has eight bright entries.
/// Bold is now a **flag** rather than a rewrite of the foreground index, so it is resolved at paint
/// time: it brightens the eight normal colours and nothing else (a 256-colour or 24-bit foreground
/// is drawn as asked, which is xterm's rule), and SGR 22 restores exactly the colour that was set,
/// including an explicitly bright one. Keeping the flag is also what lets the atlas (increment 3)
/// draw a real bold face later without the parser changing. Recorded in notes/glyphs.md.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Attr {
    fg: Colour,
    bg: Colour,
    flags: u8,
}

/// SGR 1.
const BOLD: u8 = 1 << 0;
/// SGR 2, faint.
const DIM: u8 = 1 << 1;
/// SGR 4 (and 21, drawn the same: one underline row is all an eight-row cell has room for).
const UNDERLINE: u8 = 1 << 2;
/// SGR 7.
const REVERSE: u8 = 1 << 3;
/// SGR 8, concealed.
const INVISIBLE: u8 = 1 << 4;
/// SGR 9, crossed out.
const STRIKETHROUGH: u8 = 1 << 5;

/// The glyph row an underline is drawn on: the last, which carries only descenders and the
/// underscore (`bitmap_font::GLYPH_H`'s doc), so an underline runs under the letters rather than
/// through them and joins across cells.
const UNDERLINE_ROW: u32 = bitmap_font::GLYPH_H - 1;
/// The glyph row a strikethrough is drawn on: the hyphen's row, which is also the crossbar of `e`,
/// so a struck-out word is crossed at the middle of its lower-case letters.
const STRIKETHROUGH_ROW: u32 = 4;

impl Attr {
    /// The rendition a reset terminal writes with.
    pub const DEFAULT: Attr = Attr::new(Colour::Indexed(DEFAULT_FG), Colour::Indexed(DEFAULT_BG));

    /// A plain rendition: these two colours, and no flag set.
    pub const fn new(fg: Colour, bg: Colour) -> Attr {
        Attr { fg, bg, flags: 0 }
    }

    /// The same rendition with `flag` turned on or off.
    const fn with(self, flag: u8, on: bool) -> Attr {
        Attr {
            flags: if on {
                self.flags | flag
            } else {
                self.flags & !flag
            },
            ..self
        }
    }

    const fn has(self, flag: u8) -> bool {
        self.flags & flag != 0
    }

    /// The foreground as it was set, before bold, dim or reverse are applied.
    pub const fn fg(self) -> Colour {
        self.fg
    }

    /// The background as it was set, before reverse is applied.
    pub const fn bg(self) -> Colour {
        self.bg
    }

    /// Whether the reverse-video bit is set. See [`colours`](Self::colours) for what it does to
    /// the actual paint colours.
    ///
    /// Name: provisional, flagged 2026-09-24 by the boolean-predicate pass
    /// (design/naming/boolean-predicates-worklist.md). It does not yet follow the Rust predicate
    /// rule calef ratified 2026-09-24; recommended `is_reverse_video`, because `is_reverse` reads
    /// as a direction.
    pub const fn reverse(self) -> bool {
        self.has(REVERSE)
    }

    /// Whether SGR 1 is in force. Name: provisional (milestone 142's lane, 2026-09-26).
    pub const fn is_bold(self) -> bool {
        self.has(BOLD)
    }

    /// Whether SGR 2 is in force. Name: provisional (milestone 142's lane, 2026-09-26).
    pub const fn is_dim(self) -> bool {
        self.has(DIM)
    }

    /// Whether SGR 4 is in force. Name: provisional (milestone 142's lane, 2026-09-26).
    pub const fn is_underlined(self) -> bool {
        self.has(UNDERLINE)
    }

    /// Whether SGR 8 is in force. Name: provisional (milestone 142's lane, 2026-09-26).
    pub const fn is_invisible(self) -> bool {
        self.has(INVISIBLE)
    }

    /// Whether SGR 9 is in force. Name: provisional (milestone 142's lane, 2026-09-26).
    pub const fn is_struck_through(self) -> bool {
        self.has(STRIKETHROUGH)
    }

    /// The `(foreground, background)` colours this rendition actually paints with, as
    /// `0x00RRGGBB` words. One place, so the cursor and the SGR path cannot disagree about what any
    /// flag means. In order:
    ///
    /// 1. **Bold** brightens a foreground among the eight normal colours (see [`Attr`]).
    /// 2. **Dim** moves the foreground halfway to the background, channel by channel, in sRGB. The
    ///    midpoint is xterm's choice of amount. sRGB rather than linear light on purpose: dim asks
    ///    for half the *apparent* brightness, and the linear-light midpoint of white and black is
    ///    sRGB 188, which reads as barely dimmed. Linear light is right for anti-aliased coverage
    ///    (increment 4), where the question is how much physical light a partly covered pixel emits.
    /// 3. **Reverse** swaps the two.
    /// 4. **Invisible** paints the foreground in the (post-reverse) background, so the text is
    ///    there to copy and not there to see. Last, so the block cursor (which is reverse toggled on
    ///    a cell) still shows on a concealed cell rather than vanishing with its text.
    pub const fn colours(self) -> (u32, u32) {
        let fg = match self.fg {
            Colour::Indexed(i) if i < 8 && self.has(BOLD) => Colour::Indexed(i + 8),
            other => other,
        };
        let (mut f, mut b) = (fg.resolve(), self.bg.resolve());
        if self.has(DIM) {
            f = midpoint(f, b);
        }
        if self.has(REVERSE) {
            (f, b) = (b, f);
        }
        if self.has(INVISIBLE) {
            f = b;
        }
        (f, b)
    }
}

/// The per-channel midpoint of two `0x00RRGGBB` words, rounding down.
const fn midpoint(a: u32, b: u32) -> u32 {
    channel_midpoint(a, b, 16) | channel_midpoint(a, b, 8) | channel_midpoint(a, b, 0)
}

const fn channel_midpoint(a: u32, b: u32, shift: u32) -> u32 {
    ((((a >> shift) & 0xff) + ((b >> shift) & 0xff)) / 2) << shift
}

impl Default for Attr {
    fn default() -> Attr {
        Attr::DEFAULT
    }
}

/// One cell of the grid: a character and how to paint it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    /// The character, decoded from UTF-8 by the engine (milestone 142 increment 2). One `char`, not
    /// one byte: a multi-byte UTF-8 sequence occupies exactly one cell, the same as it occupies one
    /// column, rather than one cell per encoded byte. `bitmap_font` still only has pictures for basic
    /// latin (`bitmap_font::glyph`'s own doc), so a non-ASCII `char` here draws the missing-glyph
    /// box; what changed is that it draws *one* box per character instead of a run of wrong pictures,
    /// one per UTF-8 continuation byte.
    pub ch: char,
    /// How to paint it.
    pub attr: Attr,
}

// **Sixteen bytes a cell, as a gate rather than a hope.** A `char` (four) beside an [`Attr`] (nine,
// alignment one) pads to sixteen; before the truecolour pass it was eight. Every `Vt` holds
// `MAX_CELLS + SCROLLBACK_CELLS` of these, so this number is what the memory cost in
// [`SCROLLBACK_ROWS`]'s doc is computed from, and a change that grew the cell again (a wider flag
// word, a third colour for SGR 58's underline colour) must fail here and be priced there.
const _: () = assert!(core::mem::size_of::<Cell>() == 16);

impl Cell {
    /// A blank cell in `attr`. Erasing writes **spaces in the current rendition**, not zeroes, which
    /// is what makes `CSI K` on a coloured background leave the background rather than a black gap.
    pub const fn blank(attr: Attr) -> Cell {
        Cell { ch: ' ', attr }
    }
}

impl Default for Cell {
    fn default() -> Cell {
        Cell::blank(Attr::DEFAULT)
    }
}

// ================================================================================================
// Damage.
// ================================================================================================

/// **The rectangle of cells that changed**, half-open in both axes.
///
/// A bounding box rather than a region list, the same trade `compositor::Rect` makes for the
/// compositor and for the same reason: two small changes far apart cost the box that contains both,
/// which is a few extra glyph blits here and the wrong call at desktop resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellRect {
    /// Leftmost changed column.
    pub col: u32,
    /// Topmost changed row.
    pub row: u32,
    /// Width in cells.
    pub cols: u32,
    /// Height in cells.
    pub rows: u32,
}

impl CellRect {
    const fn cell(col: u32, row: u32) -> CellRect {
        CellRect {
            col,
            row,
            cols: 1,
            rows: 1,
        }
    }

    /// The bounding box of two rectangles. Public because a client that must carry damage forward
    /// across a frame the compositor has not acknowledged yet does the same accumulation
    /// (`components/src/display_terminal.rs`), and two spellings of a bounding box is one too many.
    pub const fn union(self, o: CellRect) -> CellRect {
        let col = if self.col < o.col { self.col } else { o.col };
        let row = if self.row < o.row { self.row } else { o.row };
        let (sr, or) = (self.col + self.cols, o.col + o.cols);
        let right = if sr > or { sr } else { or };
        let (sb, ob) = (self.row + self.rows, o.row + o.rows);
        let bottom = if sb > ob { sb } else { ob };
        CellRect {
            col,
            row,
            cols: right - col,
            rows: bottom - row,
        }
    }

    /// This rectangle in **pixels**, as `(x, y, w, h)`. What a `FLUSH` or a compositor damage
    /// rectangle wants.
    pub const fn to_pixels(self) -> (u32, u32, u32, u32) {
        (
            self.col * bitmap_font::GLYPH_W,
            self.row * bitmap_font::GLYPH_H,
            self.cols * bitmap_font::GLYPH_W,
            self.rows * bitmap_font::GLYPH_H,
        )
    }
}

// ================================================================================================
// The engine.
// ================================================================================================

/// The parser's state. Deliberately few: a VT's full state machine has a dozen states, and most of
/// the extra ones exist to distinguish sequences this engine swallows anyway.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    /// Printing.
    Ground,
    /// `ESC` has been seen.
    Esc,
    /// Inside `ESC [`, accumulating parameters.
    Csi,
    /// Inside a **string** sequence (`ESC ]`, the operating-system command, and its relatives).
    ///
    /// This state earns its place: a string sequence carries arbitrary text, so a parser without it
    /// prints the window title onto the screen. That is not hypothetical, it is what this engine did
    /// before the test caught it, and it is the reason the test feeds a title-setting sequence.
    Str,
    /// Inside a string sequence, having just seen `ESC`: a `\` ends the string (ST), anything else
    /// is part of it.
    StrEsc,
}

/// How many numeric parameters a CSI sequence may carry. **Sixteen since milestone 142's truecolour
/// pass**, up from four: one 24-bit colour is five parameters (`38;2;r;g;b`), a foreground and a
/// background together are ten, and a highlighter commonly adds a reset and a rendition or two in
/// the same sequence. A seventeenth is swallowed rather than growing the array, because a parameter
/// list that long belongs to a sequence this engine does not implement.
const MAX_PARAMS: usize = 16;

/// **A terminal's live grid, plus its off-screen history.**
///
/// Construct with [`Vt::new`], push bytes with [`Vt::feed`], read pixels with [`Vt::pixel`], and ask
/// what changed with [`Vt::take_damage`]. Scroll into history with [`Vt::scroll_up`]/
/// [`Vt::scroll_down`] (milestone 142 increment 2).
///
/// **Large enough now that a `Vt` must never be a runtime-constructed local or return value**,
/// which is worth stating as a rule next to the type rather than only in the notes. At [`MAX_COLS`]
/// x [`MAX_ROWS`] plus [`SCROLLBACK_CELLS`] this is several hundred KiB, comfortably past a kernel
/// thread stack (24 KiB) and past a user process's own stack page (4 KiB). `Vt::new` stays `const
/// fn` for exactly this reason: called with compile-time-constant `cols`/`rows` (as every
/// `static mut ... = Vt::new(...)` in this tree does), the value is built by the compiler and placed
/// directly in `.bss`, never on a stack. A caller with a **runtime** geometry (`display_terminal`
/// negotiating with its driver, a kernel test reading dimensions off a reply) must construct once,
/// at a fixed or upper-bound size, and then call [`Vt::reset_to`] to retarget it in place; it must
/// never write `let vt = Vt::new(runtime_cols, runtime_rows);` or `*existing = Vt::new(...);`; both
/// are calls to a `const fn` outside a const context, which run as ordinary functions and would
/// require an ordinary, Vt-sized return value to exist somewhere at runtime. `script/stack-frame-
/// check` is the gate that would catch a violation, and this doc comment is why a reader should not
/// need it to.
pub struct Vt {
    cells: [Cell; MAX_CELLS],
    /// Off-screen history: a ring of whole rows, oldest overwritten first. See
    /// [`Vt::scroll_up`]/[`Vt::scroll_down`] for the read side and [`Vt::line_feed`] for the write
    /// side (a row scrolled off the live grid's top is pushed here before being discarded).
    scrollback: [Cell; SCROLLBACK_CELLS],
    /// The ring index (in rows) the **next** pushed row will occupy. Rows already stored occupy the
    /// `sb_len` slots immediately before this one, wrapping.
    sb_tail: u32,
    /// How many scrollback rows are populated, capped at [`SCROLLBACK_ROWS`].
    sb_len: u32,
    /// How many rows scrolled back from the live view, `0..=sb_len`. `0` is the ordinary live view;
    /// nonzero means [`Vt::cell`] (and therefore [`Vt::pixel`]/[`Vt::row_bytes`]) reads from
    /// [`scrollback`](Self::scrollback) for the topmost `view_offset` display rows.
    view_offset: u32,
    cols: u32,
    rows: u32,
    col: u32,
    row: u32,
    attr: Attr,
    /// The right margin has been reached and the *next* printable wraps. See [`Vt::feed`].
    wrap_pending: bool,
    cursor_visible: bool,
    state: State,
    params: [u16; MAX_PARAMS],
    nparams: usize,
    /// A parameter list this engine will not act on (private `?` sequences, an intermediate byte, or
    /// more parameters than fit). The sequence is still swallowed whole; only its effect is dropped.
    ignore: bool,
    /// How many UTF-8 continuation bytes are still expected before [`utf8_code`](Self::utf8_code) is
    /// a complete code point. `0` means the next byte starts a fresh character (or is plain ASCII).
    utf8_need: u8,
    /// The code point accumulated so far, valid only while [`utf8_need`](Self::utf8_need) is
    /// nonzero.
    utf8_code: u32,
    dirty: Option<CellRect>,
}

impl Vt {
    /// A blank terminal of `cols` by `rows` cells, clamped to [`MAX_COLS`] and [`MAX_ROWS`].
    ///
    /// Clamped rather than refused because the alternative in a `no_std` component is a panic in a
    /// process that has no way to report one, and a terminal that is smaller than asked for is
    /// visibly wrong. The component that wires it asserts the real geometry at compile time.
    ///
    /// **`const`, so a terminal can live in `.bss`.** See this struct's own doc for why that
    /// matters more now than it used to: a `Vt` is hundreds of KiB, and `const` is what lets a
    /// compile-time-constant geometry cost nothing at runtime. The clamps are spelled out rather
    /// than using `Ord::clamp`, which is not `const`.
    ///
    /// A **runtime** geometry must not call this directly and bind the result (see the struct doc);
    /// construct once with any geometry (typically `(1, 1)`, as `components/src/display_terminal.rs`'s
    /// `static` does) and call [`Vt::reset_to`] instead.
    pub const fn new(cols: u32, rows: u32) -> Vt {
        let cols = Self::clamp_cols(cols);
        let rows = Self::clamp_rows(rows);
        Vt {
            cells: [Cell::blank(Attr::DEFAULT); MAX_CELLS],
            scrollback: [Cell::blank(Attr::DEFAULT); SCROLLBACK_CELLS],
            sb_tail: 0,
            sb_len: 0,
            view_offset: 0,
            cols,
            rows,
            col: 0,
            row: 0,
            attr: Attr::DEFAULT,
            wrap_pending: false,
            cursor_visible: true,
            state: State::Ground,
            params: [0; MAX_PARAMS],
            nparams: 0,
            ignore: false,
            utf8_need: 0,
            utf8_code: 0,
            // A fresh terminal is entirely damage: nothing has painted its surface yet, so the
            // first present must cover the whole grid rather than the empty rectangle "nothing
            // changed" would give.
            dirty: Some(CellRect {
                col: 0,
                row: 0,
                cols,
                rows,
            }),
        }
    }

    const fn clamp_cols(cols: u32) -> u32 {
        if cols == 0 {
            1
        } else if cols > MAX_COLS as u32 {
            MAX_COLS as u32
        } else {
            cols
        }
    }

    const fn clamp_rows(rows: u32) -> u32 {
        if rows == 0 {
            1
        } else if rows > MAX_ROWS as u32 {
            MAX_ROWS as u32
        } else {
            rows
        }
    }

    /// **Re-target this terminal to `cols` by `rows`, in place, clearing the grid and the
    /// scrollback.** What [`Vt::new`] does, applied to an existing `Vt` by mutating it field by
    /// field, so a caller with a **runtime** geometry never needs a `Vt`-sized return value or
    /// local (see this struct's own doc for why that is now a real hazard rather than a style
    /// preference). Typical use: a `static mut` constructed once at `(1, 1)`, retargeted here the
    /// moment the real geometry is known (`components/src/display_terminal.rs`'s own bring-up, a kernel
    /// test that read a window's size off a control page).
    pub fn reset_to(&mut self, cols: u32, rows: u32) {
        let cols = Self::clamp_cols(cols);
        let rows = Self::clamp_rows(rows);
        self.cells = [Cell::blank(Attr::DEFAULT); MAX_CELLS];
        self.scrollback = [Cell::blank(Attr::DEFAULT); SCROLLBACK_CELLS];
        self.sb_tail = 0;
        self.sb_len = 0;
        self.view_offset = 0;
        self.cols = cols;
        self.rows = rows;
        self.col = 0;
        self.row = 0;
        self.attr = Attr::DEFAULT;
        self.wrap_pending = false;
        self.cursor_visible = true;
        self.state = State::Ground;
        self.params = [0; MAX_PARAMS];
        self.nparams = 0;
        self.ignore = false;
        self.utf8_need = 0;
        self.utf8_code = 0;
        self.damage_all();
    }

    /// The grid's width in cells.
    pub const fn cols(&self) -> u32 {
        self.cols
    }

    /// The grid's height in cells.
    pub const fn rows(&self) -> u32 {
        self.rows
    }

    /// The grid's width in pixels.
    pub const fn width(&self) -> u32 {
        self.cols * bitmap_font::GLYPH_W
    }

    /// The grid's height in pixels.
    pub const fn height(&self) -> u32 {
        self.rows * bitmap_font::GLYPH_H
    }

    /// Where the cursor is, as `(col, row)`.
    pub const fn cursor(&self) -> (u32, u32) {
        (self.col, self.row)
    }

    /// Show or hide the block cursor. A hidden cursor is what a terminal that is only *printing*
    /// wants, and it is what makes a picture independent of where the last write left off.
    pub fn set_cursor_visible(&mut self, visible: bool) {
        if self.cursor_visible != visible {
            self.cursor_visible = visible;
            self.damage_cell(self.col, self.row);
        }
    }

    /// **The cell at `(col, row)` of what is currently displayed** (the live grid, or scrollback if
    /// [`Vt::view_offset`] is nonzero). Out of range gives a default blank rather than a panic,
    /// because the callers are pixel loops.
    ///
    /// `row` is a **display** row: `0` is always the top of whatever is currently shown, whether
    /// that is the live grid (view offset `0`) or a scrolled-back history page. This is the one
    /// function every reader (`pixel`, `row_bytes`) goes through, so the scrollback view is uniform
    /// rather than a special case each caller has to know about.
    pub fn cell(&self, col: u32, row: u32) -> Cell {
        if col >= self.cols || row >= self.rows {
            return Cell::default();
        }
        if row < self.view_offset {
            // The topmost `view_offset` display rows come from history: display row 0 is the
            // oldest of the rows being shown from scrollback (`age = view_offset - 1`), and each
            // row closer to the live grid is one age newer, down to `age = 0` immediately above it.
            let age = self.view_offset - 1 - row;
            return self.scrollback_cell(col, age);
        }
        let live_row = row - self.view_offset;
        self.cells[(live_row * self.cols + col) as usize]
    }

    /// The scrollback cell `age` rows above the live grid's top (`age = 0` is the most recently
    /// scrolled-off row), at column `col`. `age >= sb_len` (asked for history that was never kept,
    /// or never existed) gives a default blank, the same total-function discipline [`Vt::cell`]
    /// uses for a coordinate outside the grid.
    fn scrollback_cell(&self, col: u32, age: u32) -> Cell {
        if age >= self.sb_len {
            return Cell::default();
        }
        // `sb_tail` is the ring slot the *next* push will use, so the most recent row (age 0) is
        // one slot behind it, and each older age is one slot further behind, wrapping.
        let capacity = SCROLLBACK_ROWS as u32;
        let ring_row = (self.sb_tail + capacity - 1 - age) % capacity;
        self.scrollback[(ring_row * self.cols + col) as usize]
    }

    /// **The pixel at `(x, y)` of the terminal's surface.** The whole of rendering, as a pure
    /// function of the grid (and, since milestone 142 increment 2, of the scroll position).
    ///
    /// The cursor is a **block**, drawn by swapping the cell's own two colours. Drawing it here
    /// rather than as a separate overlay is what keeps the picture a function of the state: a test
    /// that predicts the screen predicts the cursor too, and a cursor left in the wrong place is a
    /// failure rather than a cosmetic difference nobody notices.
    ///
    /// **The cursor does not draw while scrolled back.** It names a position in the live grid, which
    /// is not what is on screen when `view_offset` is nonzero; drawing it there would put the block
    /// on whatever history cell happens to share its `(col, row)`, which is not the cursor's
    /// position and would be actively misleading.
    ///
    /// **`col < self.cols` is checked after `col == self.col`, and by then it is redundant.**
    /// `self.col` is never `>= self.cols` (the same cursor-in-bounds invariant `Vt::erase_display`'s
    /// doc comment names), so `col == self.col` already implies `col < self.cols`; relaxing the
    /// comparison to `<=` cannot change which branch is taken. It stays written out because a reader
    /// should not have to chase that invariant to see the guard is safe.
    pub fn pixel(&self, x: u32, y: u32) -> u32 {
        let (col, row) = (x / bitmap_font::GLYPH_W, y / bitmap_font::GLYPH_H);
        let cell = self.cell(col, row);
        let mut attr = cell.attr;
        if self.view_offset == 0
            && self.cursor_visible
            && col == self.col
            && row == self.row
            && col < self.cols
        {
            attr = attr.with(REVERSE, !attr.reverse());
        }
        let (fg, bg) = attr.colours();
        let (gx, gy) = (x % bitmap_font::GLYPH_W, y % bitmap_font::GLYPH_H);
        // The two line renditions are ink across the **whole** cell width, gutters included, so an
        // underlined or struck-out word is one unbroken line rather than a dash per letter (unlike
        // the underscore glyph, whose ink stops at the gutters; see notes/glyphs.md).
        let line = (attr.is_underlined() && gy == UNDERLINE_ROW)
            || (attr.is_struck_through() && gy == STRIKETHROUGH_ROW);
        if line || bitmap_font::is_ink(cell.ch, gx, gy) {
            fg
        } else {
            bg
        }
    }

    /// What has changed since the last [`Vt::take_damage`], in cells.
    pub const fn damage(&self) -> Option<CellRect> {
        self.dirty
    }

    /// What has changed since the last call, in cells, clearing the record.
    pub fn take_damage(&mut self) -> Option<CellRect> {
        self.dirty.take()
    }

    /// Copy row `row`'s characters into `out` as bytes, returning how many were written. For tests
    /// and for a caller that wants the text rather than the pixels; there is no `String` in
    /// `no_std`. ASCII characters pass through unchanged; anything else (a non-ASCII `char`, since
    /// milestone 142's UTF-8 increment) becomes `?`, the conventional lossy placeholder, since the
    /// return type has no room for anything wider than a byte.
    pub fn row_bytes(&self, row: u32, out: &mut [u8]) -> usize {
        let n = (self.cols as usize).min(out.len());
        for (i, o) in out.iter_mut().enumerate().take(n) {
            let ch = self.cell(i as u32, row).ch;
            *o = if ch.is_ascii() { ch as u8 } else { b'?' };
        }
        n
    }

    /// How many rows scrolled back from the live view. `0` is the ordinary live view.
    pub const fn view_offset(&self) -> u32 {
        self.view_offset
    }

    /// How many rows of history are available to scroll into.
    pub const fn scrollback_len(&self) -> u32 {
        self.sb_len
    }

    /// **Scroll `n` rows further into history**, clamped at [`Vt::scrollback_len`]. Marks the whole
    /// grid dirty, because scrolling changes every displayed row at once (the same reason the
    /// ordinary scroll on `LF` at the bottom row does).
    pub fn scroll_up(&mut self, n: u32) {
        let new_offset = self.view_offset.saturating_add(n).min(self.sb_len);
        if new_offset != self.view_offset {
            self.view_offset = new_offset;
            self.damage_all();
        }
    }

    /// **Scroll `n` rows back toward the live view**, clamped at `0`. See [`Vt::scroll_up`].
    pub fn scroll_down(&mut self, n: u32) {
        let new_offset = self.view_offset.saturating_sub(n);
        if new_offset != self.view_offset {
            self.view_offset = new_offset;
            self.damage_all();
        }
    }

    /// **Push bytes through the parser.**
    ///
    /// # Deferred wrap, which is the one subtlety in printing
    ///
    /// Writing into the last column does **not** move the cursor to the next row. It leaves the
    /// cursor on that last cell and arms a pending wrap, and the *next* printable byte does the
    /// wrap first. Every real terminal does this, and the reason is visible in a line discipline's
    /// echo: a line that exactly fills the width would otherwise scroll the screen before anything
    /// asked it to, and a `CR` arriving right after the last character would find the cursor a row
    /// too low. Any cursor motion, `CR`, or `LF` cancels the pending wrap.
    pub fn feed(&mut self, bytes: &[u8]) {
        // New output snaps the view back to live, the same convention every real terminal follows:
        // typing (or a program printing) while scrolled into history is disorienting otherwise, and
        // it is what makes `view_offset` a read-only concern for a caller that never scrolls. A
        // caller mid-history who then feeds nothing (a scroll key alone, which goes through
        // `scroll_up`/`scroll_down` and never reaches this function) is unaffected.
        if self.view_offset != 0 {
            self.view_offset = 0;
            self.damage_all();
        }
        let before = (self.col, self.row);
        for &b in bytes {
            self.byte(b);
        }
        if (self.col, self.row) != before && self.cursor_visible {
            // The cursor is painted, so moving it dirties both the cell it left and the one it
            // arrived at. Doing it once per `feed` rather than per byte keeps a long print from
            // dirtying a trail of cells it also overwrote.
            self.damage_cell(before.0, before.1);
            self.damage_cell(self.col, self.row);
        }
    }

    fn byte(&mut self, b: u8) {
        match self.state {
            State::Ground => self.ground(b),
            State::Esc => self.esc(b),
            State::Csi => self.csi(b),
            State::Str => match b {
                0x07 => self.state = State::Ground, // BEL, the xterm terminator
                0x1b => self.state = State::StrEsc,
                _ => {}
            },
            State::StrEsc => {
                // `ESC \` is the standard string terminator; an ESC followed by anything else was
                // part of the string after all.
                self.state = if b == b'\\' {
                    State::Ground
                } else {
                    State::Str
                };
            }
        }
    }

    /// The Unicode replacement character, drawn for an invalid or incomplete UTF-8 sequence. The
    /// conventional choice (the same one `String::from_utf8_lossy` makes), so this engine's failure
    /// picture is the one a reader has likely seen from every other tool that decodes UTF-8.
    const REPLACEMENT: char = '\u{fffd}';

    fn ground(&mut self, b: u8) {
        // **UTF-8 decoding, ahead of the control-code match.** Every control code and every escape
        // introducer this engine understands is plain ASCII (`< 0x80`), so a byte with the high bit
        // set can only be part of a multi-byte character; checking `utf8_need` first is what lets
        // the two decoders (this one, and the ANSI/CSI state machine below) coexist without either
        // having to know about the other's bytes. State persists across `feed` calls, so a sequence
        // split at a buffer boundary still decodes correctly.
        if self.utf8_need > 0 {
            if b & 0xc0 == 0x80 {
                // A well-formed continuation byte.
                //
                // **The `|` cannot be distinguished from `^` here.** `self.utf8_code << 6` always has
                // its low six bits zero (a left shift by six fills them with zero), and `b & 0x3f` is
                // exactly six bits, so the two operands never share a set bit; OR and XOR agree on
                // every input for that reason alone, not because of anything this loop's callers do.
                self.utf8_code = (self.utf8_code << 6) | (b & 0x3f) as u32;
                self.utf8_need -= 1;
                if self.utf8_need == 0 {
                    let ch = char::from_u32(self.utf8_code).unwrap_or(Self::REPLACEMENT);
                    self.print(ch);
                }
                return;
            }
            // Not a continuation byte: the sequence was truncated. Draw the replacement for what
            // was collected so far and reprocess `b` as a fresh byte, rather than eating it, so a
            // truncated sequence loses exactly the bytes it claimed and nothing after them.
            self.utf8_need = 0;
            self.print(Self::REPLACEMENT);
        }
        match b {
            0x1b => {
                self.state = State::Esc;
            }
            b'\r' => {
                self.col = 0;
                self.wrap_pending = false;
            }
            b'\n' => {
                self.wrap_pending = false;
                self.line_feed();
            }
            0x08 => {
                // Backspace does not wrap back to the previous row. A line discipline never asks it
                // to (it only backs up within the line it echoed), and a terminal that did would
                // have to remember whether the previous row ended in a wrap.
                self.wrap_pending = false;
                self.col = self.col.saturating_sub(1);
            }
            b'\t' => {
                self.wrap_pending = false;
                // Every eight columns, and never past the last one: a tab at the right margin stops
                // there rather than wrapping, which is what the fixed-tab-stop terminals do.
                self.col = ((self.col / 8 + 1) * 8).min(self.cols - 1);
            }
            0x07 => {} // BEL: there is no bell on a framebuffer, and a visual bell is policy
            0x00..=0x1f | 0x7f => {} // every other control code: consumed, never drawn
            0x20..=0x7e => self.print(b as char), // plain printable ASCII, the common case
            // A UTF-8 lead byte: 0xc2..0xdf is two bytes total, 0xe0..0xef three, 0xf0..0xf4 four
            // (RFC 3629's range, past which no code point is assigned). 0x80..0xc1 and 0xf5..0xff
            // can never start a sequence (0x80..0xbf are continuation-only; 0xc0/0xc1 could only
            // encode a code point already representable in one byte, which RFC 3629 forbids as an
            // overlong form), so those draw the replacement immediately rather than waiting for
            // bytes that would never complete a valid character.
            0xc2..=0xdf => {
                self.utf8_need = 1;
                self.utf8_code = (b & 0x1f) as u32;
            }
            0xe0..=0xef => {
                self.utf8_need = 2;
                self.utf8_code = (b & 0x0f) as u32;
            }
            0xf0..=0xf4 => {
                self.utf8_need = 3;
                self.utf8_code = (b & 0x07) as u32;
            }
            0x80..=0xc1 | 0xf5..=0xff => self.print(Self::REPLACEMENT),
        }
    }

    fn print(&mut self, ch: char) {
        if self.wrap_pending {
            self.wrap_pending = false;
            self.col = 0;
            self.line_feed();
        }
        let (col, row) = (self.col, self.row);
        self.put(
            col,
            row,
            Cell {
                ch,
                attr: self.attr,
            },
        );
        if self.col + 1 >= self.cols {
            self.wrap_pending = true;
        } else {
            self.col += 1;
        }
    }

    /// Down one row, scrolling the grid up if that would fall off the bottom.
    fn line_feed(&mut self) {
        if self.row + 1 < self.rows {
            self.row += 1;
            return;
        }
        // Scroll: the row about to fall off the top goes to scrollback first (milestone 142
        // increment 2), before the shift below overwrites it. Every row moves, so the whole grid is
        // damage. Honest rather than clever; a real terminal with a scrolling accelerator still
        // repaints the screen it exposes.
        let cols = self.cols as usize;
        self.push_scrollback_row(0);
        let used = cols * self.rows as usize;
        self.cells.copy_within(cols..used, 0);
        for c in &mut self.cells[used - cols..used] {
            *c = Cell::blank(self.attr);
        }
        self.damage_all();
    }

    /// Copy live row `row` into the scrollback ring's next slot, as the newest entry.
    ///
    /// Only ever called from [`Vt::line_feed`], itself only reachable through [`Vt::feed`]'s byte
    /// loop, and [`Vt::feed`] resets `view_offset` to `0` before that loop runs (see its own doc).
    /// So a push never happens while the caller is looking at history, and this does not need to
    /// (and does not) adjust `view_offset` to compensate for the shift a push would otherwise cause.
    ///
    /// **`row` is always `0`.** This is a private method with one call site, which passes the
    /// literal `0` (see [`Vt::line_feed`]): the row about to scroll off the top is always the grid's
    /// first row, because scrolling is what makes room at the bottom, not a copy from elsewhere. So
    /// the source index's `row * cols` is always `0 * cols`, and a mutant reading it as `row / cols`
    /// computes the same `0` on the only input this function is ever given.
    fn push_scrollback_row(&mut self, row: u32) {
        let cols = self.cols;
        let capacity = SCROLLBACK_ROWS as u32;
        let ring_row = self.sb_tail;
        for c in 0..cols {
            self.scrollback[(ring_row * cols + c) as usize] = self.cells[(row * cols + c) as usize];
        }
        self.sb_tail = (self.sb_tail + 1) % capacity;
        if self.sb_len < capacity {
            self.sb_len += 1;
        }
    }

    fn esc(&mut self, b: u8) {
        self.state = State::Ground;
        match b {
            b'[' => {
                self.state = State::Csi;
                self.params = [0; MAX_PARAMS];
                self.nparams = 0;
                self.ignore = false;
            }
            b'c' => self.reset(),
            // The string introducers: OSC (`]`), DCS (`P`), SOS/PM/APC (`X`, `^`, `_`). Their
            // payload is arbitrary text, so it has to be *consumed*, not returned to Ground.
            b']' | b'P' | b'X' | b'^' | b'_' => self.state = State::Str,
            // Anything else: the introducer and this byte are both consumed. A two-byte escape this
            // engine does not speak must not leave its final byte to be printed as a letter, which is
            // the classic "stray letter on the screen" bug.
            _ => {}
        }
    }

    fn csi(&mut self, b: u8) {
        match b {
            b'0'..=b'9' => {
                if self.nparams == 0 {
                    self.nparams = 1;
                }
                if self.nparams <= MAX_PARAMS {
                    let p = &mut self.params[self.nparams - 1];
                    *p = p.saturating_mul(10).saturating_add((b - b'0') as u16);
                }
            }
            b';' => {
                self.nparams += 1;
                if self.nparams > MAX_PARAMS {
                    self.ignore = true;
                    self.nparams = MAX_PARAMS;
                }
            }
            // A private-use introducer (`?`, `<`, `=`, `>`) or an intermediate byte: this engine
            // implements none of those sequences, so it swallows the whole thing rather than acting
            // on a parameter list that means something else.
            //
            // **Deleting this arm changes nothing**: `0x20..=0x2f` and `0x3c..=0x3f` are not matched
            // by `b'0'..=b'9'`, `b';'`, or `0x40..=0x7e`, so removing the arm routes those bytes to
            // the catch-all below, which sets `self.ignore = true` too. Kept written out rather than
            // folded into `_` because a reader needs to see the two families of swallowed byte (the
            // sequences this engine chose not to implement, versus a stray control code) named
            // separately, even though the code they run is identical.
            0x20..=0x2f | 0x3c..=0x3f => self.ignore = true,
            0x40..=0x7e => {
                let ignore = self.ignore;
                self.state = State::Ground;
                if !ignore {
                    self.csi_final(b);
                }
            }
            _ => {
                // A control code inside a sequence: real terminals execute it. Nothing that reaches
                // this engine does that, so it is dropped along with the sequence, which fails in the
                // direction of drawing nothing rather than drawing garbage.
                self.ignore = true;
            }
        }
    }

    /// The first parameter, defaulting to `d` when absent or zero (the ANSI convention: `CSI 0 A`
    /// and `CSI A` both mean one row).
    fn param(&self, i: usize, d: u32) -> u32 {
        match self.params.get(i) {
            Some(&0) | None => d,
            Some(&p) => p as u32,
        }
    }

    fn csi_final(&mut self, b: u8) {
        match b {
            b'A' => {
                self.wrap_pending = false;
                self.row = self.row.saturating_sub(self.param(0, 1));
            }
            b'B' => {
                self.wrap_pending = false;
                self.row = (self.row + self.param(0, 1)).min(self.rows - 1);
            }
            b'C' => {
                self.wrap_pending = false;
                self.col = (self.col + self.param(0, 1)).min(self.cols - 1);
            }
            b'D' => {
                self.wrap_pending = false;
                self.col = self.col.saturating_sub(self.param(0, 1));
            }
            // CUP. Parameters are **one-based** on the wire and zero-based here, which is the
            // off-by-one every terminal implementation has had at least once.
            b'H' | b'f' => {
                self.wrap_pending = false;
                self.row = (self.param(0, 1) - 1).min(self.rows - 1);
                self.col = (self.param(1, 1) - 1).min(self.cols - 1);
            }
            b'J' => self.erase_display(self.param(0, 0)),
            b'K' => self.erase_line(self.param(0, 0)),
            b'm' => self.sgr(),
            // Every other final byte: the sequence is consumed and nothing happens. That includes
            // the device-report sequences, deliberately: this engine has no way to answer one and a
            // half-answered query is worse than an unanswered one.
            _ => {}
        }
    }

    /// `CSI n J`: 0 erases from the cursor to the end of the screen, 1 to its start, 2 all of it.
    /// Note that `CSI 2 J` does **not** move the cursor; that is why a line discipline's ^L sends
    /// `CSI 2J` followed by `CSI H`.
    ///
    /// **`to > from` always holds here**, given two invariants this type maintains everywhere else:
    /// `cols >= 1` and `rows >= 1` ([`Vt::clamp_cols`]/[`Vt::clamp_rows`], the only places `cols`/
    /// `rows` are ever set), and `col < cols`, `row < rows` (every cursor-moving path clamps with
    /// `.min(self.cols - 1)`/`.min(self.rows - 1)`, so the cursor is never parked on or past the
    /// margin). Mode 0's `end - here = (rows - row) * cols - col >= cols - col >= 1`; mode 1's
    /// `here + 1 >= 1`; the default's `end = rows * cols >= 1`. So the guard below can never see
    /// `to == from`, which is why a mutant relaxing `>` to `>=` survives: there is no reachable
    /// input on which the two disagree.
    fn erase_display(&mut self, mode: u32) {
        let here = self.row * self.cols + self.col;
        let end = self.rows * self.cols;
        let (from, to) = match mode {
            0 => (here, end),
            1 => (0, here + 1),
            _ => (0, end),
        };
        for i in from..to.min(end) {
            self.cells[i as usize] = Cell::blank(self.attr);
        }
        if to > from {
            self.damage_all();
        }
    }

    /// `CSI n K`: 0 erases from the cursor to the end of the line, 1 to its start, 2 the whole line.
    fn erase_line(&mut self, mode: u32) {
        let (from, to) = match mode {
            0 => (self.col, self.cols),
            1 => (0, self.col + 1),
            _ => (0, self.cols),
        };
        for c in from..to.min(self.cols) {
            let row = self.row;
            self.put(c, row, Cell::blank(self.attr));
        }
    }

    /// `CSI ... m`: the rendition. An empty parameter list is `CSI 0 m`, a reset, which is the one
    /// place the "absent means zero" default differs from the cursor sequences' "absent means one".
    ///
    /// The extended colours (`38`/`48` followed by `5;n` or `2;r;g;b`) consume the parameters they
    /// name, so the loop walks an index rather than iterating. A malformed one (a missing or
    /// out-of-range component, or a colour space other than 2 or 5) ends the sequence's effect
    /// there, which is xterm's behaviour: the parameters after it cannot be told apart from the
    /// broken colour's own.
    fn sgr(&mut self) {
        let n = self.nparams.max(1);
        let mut i = 0;
        while i < n {
            let p = self.params[i];
            i += 1;
            let a = self.attr;
            self.attr = match p {
                0 => Attr::DEFAULT,
                1 => a.with(BOLD, true),
                2 => a.with(DIM, true),
                4 | 21 => a.with(UNDERLINE, true),
                7 => a.with(REVERSE, true),
                8 => a.with(INVISIBLE, true),
                9 => a.with(STRIKETHROUGH, true),
                22 => a.with(BOLD, false).with(DIM, false),
                24 => a.with(UNDERLINE, false),
                27 => a.with(REVERSE, false),
                28 => a.with(INVISIBLE, false),
                29 => a.with(STRIKETHROUGH, false),
                30..=37 => Attr {
                    fg: Colour::Indexed(p as u8 - 30),
                    ..a
                },
                39 => Attr {
                    fg: Colour::Indexed(DEFAULT_FG),
                    ..a
                },
                40..=47 => Attr {
                    bg: Colour::Indexed(p as u8 - 40),
                    ..a
                },
                49 => Attr {
                    bg: Colour::Indexed(DEFAULT_BG),
                    ..a
                },
                90..=97 => Attr {
                    fg: Colour::Indexed(p as u8 - 90 + 8),
                    ..a
                },
                100..=107 => Attr {
                    bg: Colour::Indexed(p as u8 - 100 + 8),
                    ..a
                },
                38 | 48 => {
                    let Some((colour, used)) = self.extended_colour(i, n) else {
                        return;
                    };
                    i += used;
                    if p == 38 {
                        Attr { fg: colour, ..a }
                    } else {
                        Attr { bg: colour, ..a }
                    }
                }
                // Everything else (italic, blink, overline, the underline colour) is dropped. Italic
                // needs a face this font does not have (increment 3), and blink needs a clock this
                // engine deliberately does not read; drawing the *text* in the wrong style is better
                // than not drawing it.
                _ => a,
            };
        }
    }

    /// The colour named by the parameters from `i` onward, after a `38` or `48`, and how many
    /// parameters it used. `None` for a malformed one; see [`Vt::sgr`].
    fn extended_colour(&self, i: usize, n: usize) -> Option<(Colour, usize)> {
        let component = |k: usize| -> Option<u8> {
            if i + k < n {
                u8::try_from(self.params[i + k]).ok()
            } else {
                None
            }
        };
        match self.params.get(i).filter(|_| i < n)? {
            5 => Some((Colour::Indexed(component(1)?), 2)),
            2 => Some((Colour::Rgb(component(1)?, component(2)?, component(3)?), 4)),
            _ => None,
        }
    }

    /// `ESC c`: back to power-on. Clears the grid in the *default* rendition rather than the current
    /// one, which is the difference between a reset and an erase.
    fn reset(&mut self) {
        self.attr = Attr::DEFAULT;
        self.cells = [Cell::blank(Attr::DEFAULT); MAX_CELLS];
        self.col = 0;
        self.row = 0;
        self.wrap_pending = false;
        self.damage_all();
    }

    fn put(&mut self, col: u32, row: u32, cell: Cell) {
        if col >= self.cols || row >= self.rows {
            return;
        }
        let at = (row * self.cols + col) as usize;
        if self.cells[at] != cell {
            self.cells[at] = cell;
            self.damage_cell(col, row);
        }
    }

    fn damage_cell(&mut self, col: u32, row: u32) {
        if col >= self.cols || row >= self.rows {
            return;
        }
        let r = CellRect::cell(col, row);
        self.dirty = Some(match self.dirty {
            Some(d) => d.union(r),
            None => r,
        });
    }

    fn damage_all(&mut self) {
        self.dirty = Some(CellRect {
            col: 0,
            row: 0,
            cols: self.cols,
            rows: self.rows,
        });
    }
}

/// What the display terminal reports to whoever spawned it.
///
/// **Status, not contract**: no client can ask for any of this, and it is here rather than in the
/// terminal contract (`line_editor::proto`) because it is about the *component*, not about the terminal
/// a program talks to. The engine above is sans-IO and this module is three constants; nothing in
/// the engine reads them.
pub mod status {
    /// The terminal is up: `send(REPORT, TERM_UP, cols | rows << 32, mode)`. Sent once, after the
    /// blank grid has been presented, so a spawner that sees it knows the geometry was negotiated
    /// and the first picture reached the screen.
    ///
    /// **One report, ever**, for the reason `compositor::status::COMP_UP` gives: a status `SEND` is a
    /// rendezvous, so a component that narrated every frame would block until its spawner listened,
    /// and a spawner that stopped listening would wedge everything behind it. What is on the screen
    /// is observable where it belongs: in the frames, and at the display endpoint.
    pub const TERM_UP: u64 = 0x7E7_0001;

    /// The terminal drives a display endpoint directly (`gfx FLUSH`), owning the whole scanout.
    pub const MODE_DISPLAY: u64 = 0;
    /// The terminal is a compositor client, owning one window (`compose COMMIT`).
    pub const MODE_WINDOW: u64 = 1;

    /// The keyboard driver is up: `send(REPORT, KEYBOARD_UP, buffers posted, 0)`. The device is
    /// enumerated, the event queue is programmed through the confined transport, and every
    /// device-writable buffer is posted, so a spawner that sees this knows a key pressed from here
    /// on has somewhere to land. Also one report, ever, and for the same reason [`TERM_UP`] is.
    ///
    /// Renamed from `KBD_UP` (calef, 2026-08-27), the same pass that renamed `user/src/kbd.rs` to
    /// `keyboard_driver.rs`: the constant embedded the old short name and would have gone stale.
    pub const KEYBOARD_UP: u64 = 0x7E7_0002;
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate std;
    use std::string::String;
    use std::vec::Vec;

    /// A terminal's rows as strings, for assertions that read like the screen.
    fn rows(vt: &Vt) -> Vec<String> {
        (0..vt.rows())
            .map(|r| {
                let mut buf = [0u8; MAX_COLS];
                let n = vt.row_bytes(r, &mut buf);
                String::from_utf8_lossy(&buf[..n]).into_owned()
            })
            .collect()
    }

    /// A plain rendition in two indexed colours.
    fn attr(fg: u8, bg: u8) -> Attr {
        Attr::new(Colour::Indexed(fg), Colour::Indexed(bg))
    }

    /// The foreground a cell actually paints with, bold and dim applied: what a person sees, which
    /// is the thing an SGR test should be about now that bold is a flag rather than a rewritten
    /// index.
    fn ink(t: &Vt, col: u32, row: u32) -> u32 {
        t.cell(col, row).attr.colours().0
    }

    fn vt(cols: u32, rows: u32) -> Vt {
        let mut vt = Vt::new(cols, rows);
        vt.take_damage();
        vt
    }

    /// **A rendition names two palette entries, and reverse swaps which is ink.** Every other test
    /// compares one `Attr` against another, which cannot see a packer that lost a field or a
    /// `colours` that returns a constant: both sides move together. These are the numbers.
    #[test]
    fn a_rendition_resolves_to_the_palette_entries_it_names() {
        assert_eq!(Attr::DEFAULT.fg(), Colour::Indexed(DEFAULT_FG));
        assert_eq!(Attr::DEFAULT.bg(), Colour::Indexed(DEFAULT_BG));
        assert!(!Attr::DEFAULT.reverse());
        assert_eq!(Attr::DEFAULT.colours(), (PALETTE[7], PALETTE[0]));
        // A background that is not 7, so a mask that widened to "always 7" is visible.
        assert_eq!(attr(3, 4).colours(), (PALETTE[3], PALETTE[4]));
        assert_eq!(
            attr(3, 4).with(REVERSE, true).colours(),
            (PALETTE[4], PALETTE[3]),
            "reverse swaps ink and paper, it does not pick different colours"
        );
    }

    /// **The damage box's arithmetic, at the edges where a bounding box is decided.** A rectangle
    /// that already contains the other must be its own union either way round, which is the case
    /// that reads both operands' far edges; the cell-by-cell unions elsewhere only ever grow the
    /// second operand's, so half of `union` was never consulted.
    #[test]
    fn a_damage_box_contains_a_rectangle_it_already_covered() {
        let big = CellRect {
            col: 1,
            row: 2,
            cols: 4,
            rows: 3,
        };
        let inside = CellRect {
            col: 2,
            row: 3,
            cols: 1,
            rows: 1,
        };
        assert_eq!(big.union(inside), big);
        assert_eq!(inside.union(big), big);
        // Cells are 7 by 8, so a rect at (1, 2) starts at (7, 16) and is 28 by 24. The two axes
        // differ on purpose: a single constant used for both would pass whatever it was.
        assert_eq!(big.to_pixels(), (7, 16, 28, 24));
    }

    /// The grid's size in cells and in pixels, as numbers rather than as the expression that
    /// computes them. 6 by 3 because 6*7, 6+7 and 6/7 are three different answers.
    #[test]
    fn the_grid_reports_its_size_in_cells_and_in_pixels() {
        let t = Vt::new(6, 3);
        assert_eq!((t.cols(), t.rows()), (6, 3));
        assert_eq!(t.width(), 42);
        assert_eq!(t.height(), 24);
    }

    /// **The pixel is a pure function of the grid**, so it can be pinned exactly. `L` at cell
    /// (1, 1) in green, which starts at pixel (7, 8) because the cell is 7 by 8: its stem is the
    /// second column of the cell (the first is the font's gutter), its top row holds nothing else,
    /// and its foot reaches the last ink column on the baseline. Those four pixels separate every
    /// way the cell-versus-glyph coordinate split can go wrong (a mixed-up divide and remainder
    /// agree at cell (0, 0), which is where every other pixel assertion sits), and the two axes
    /// use different divisors so a single constant cannot satisfy both.
    #[test]
    fn a_pixel_names_its_cell_and_its_place_inside_the_glyph() {
        let mut t = vt(4, 3);
        t.set_cursor_visible(false);
        t.feed(b"\x1b[2;2H\x1b[32mL");
        let (green, black) = (PALETTE[2], PALETTE[0]);
        assert_eq!(
            t.pixel(7, 8),
            black,
            "the cell's first column is the font's gutter"
        );
        assert_eq!(t.pixel(8, 8), green, "the stem is the column after it");
        assert_eq!(
            t.pixel(12, 8),
            black,
            "and the L's top row is stem and nothing else"
        );
        assert_eq!(
            t.pixel(12, 14),
            green,
            "its foot reaches the last ink column on the baseline"
        );
    }

    /// A column past the last one is off the grid, not the next row's first cell. The rows of the
    /// grid are contiguous, so an out-of-range column that is not rejected reads a real cell.
    #[test]
    fn a_column_past_the_margin_is_not_the_next_rows_first_cell() {
        let mut t = vt(4, 2);
        t.feed(b"\x1b[2;1Hab");
        assert_eq!(t.cell(4, 0), Cell::default());
    }

    /// **Tabs stop every eight columns and never past the last one.** Nothing else feeds a tab, and
    /// a line discipline does not emit one, but a program's output does.
    #[test]
    fn a_tab_advances_to_the_next_stop_and_stops_at_the_margin() {
        let mut t = vt(20, 2);
        t.feed(b"\t");
        assert_eq!(t.cursor(), (8, 0));
        t.feed(b"\t");
        assert_eq!(t.cursor(), (16, 0), "the stop is measured from where it is");
        t.feed(b"\t");
        assert_eq!(
            t.cursor(),
            (19, 0),
            "a tab at the right margin stops there rather than wrapping"
        );
    }

    /// Sixteen parameters is the limit and the limit is **legal**: a sixteen-parameter `CSI m`
    /// acts, and only a seventeenth is dropped. Every other test here uses fewer, so the limit
    /// itself was never judged from the inside.
    #[test]
    fn the_sixteenth_parameter_is_legal_and_the_seventeenth_is_not() {
        let mut t = vt(8, 1);
        // Fourteen harmless parameters, then bold and green: the last two are the ones that count.
        t.feed(b"\x1b[0;0;0;0;0;0;0;0;0;0;0;0;0;0;1;32ma");
        let want = attr(2, DEFAULT_BG).with(BOLD, true);
        assert_eq!(t.cell(0, 0).attr, want);
        t.feed(b"\x1b[0;0;0;0;0;0;0;0;0;0;0;0;0;0;0;0;31mb");
        assert_eq!(
            t.cell(1, 0).attr,
            want,
            "a sequence with too many parameters is swallowed, not half-applied"
        );
    }

    /// `CSI n J` from a cursor that is **not on the first row**, in the mode that erases backwards.
    /// Both directions read `row * cols + col`, and at row 0 that arithmetic cannot be wrong.
    #[test]
    fn erase_in_display_starts_from_where_the_cursor_actually_is() {
        let mut t = vt(4, 3);
        t.feed(b"abcd\r\nefgh\r\nijkl\x1b[2;3H\x1b[J");
        assert_eq!(rows(&t), ["abcd", "ef  ", "    "]);

        let mut t = vt(4, 3);
        t.set_cursor_visible(false);
        t.feed(b"abcd\r\nefgh\r\nijkl");
        t.take_damage();
        t.feed(b"\x1b[2;3H\x1b[1J");
        assert_eq!(
            rows(&t),
            ["    ", "   h", "ijkl"],
            "mode 1 erases through the cursor's own cell and no further"
        );
        assert!(t.damage().is_some(), "erasing the screen is damage");
    }

    /// The rest of SGR: the bright foregrounds as their own sequences, and the three switches that
    /// turn something *off*. A terminal that only ever set attributes would pass every other test
    /// here and leave a line reversed forever.
    #[test]
    fn sgr_has_bright_colours_and_switches_that_turn_things_off() {
        let mut t = vt(8, 1);
        t.feed(b"\x1b[92ma");
        assert_eq!(t.cell(0, 0).attr, attr(10, DEFAULT_BG));
        t.feed(b"\x1b[1mb");
        assert_eq!(
            ink(&t, 1, 0),
            PALETTE[10],
            "bold on an already-bright colour is idempotent, not a toggle"
        );
        t.feed(b"\x1b[22mc");
        assert_eq!(
            ink(&t, 2, 0),
            PALETTE[10],
            "SGR 22 undoes bold, not a colour that was asked for bright"
        );
        t.feed(b"\x1b[7;41md\x1b[27me\x1b[39mf\x1b[49mg");
        assert_eq!(t.cell(3, 0).attr, attr(10, 1).with(REVERSE, true));
        assert_eq!(t.cell(4, 0).attr, attr(10, 1), "SGR 27");
        assert_eq!(t.cell(5, 0).attr, attr(DEFAULT_FG, 1), "SGR 39");
        assert_eq!(t.cell(6, 0).attr, Attr::DEFAULT, "SGR 49");
    }

    /// Text lands in the grid, `CR` returns to column 0, and `LF` goes down without returning.
    /// The `\r\n` pair is what `line_editor::expand_output` produces from a Unix `\n`, so a terminal
    /// that treated `LF` as `CRLF` would look right on that stream and wrong on every other.
    #[test]
    fn printing_moves_the_cursor_the_way_a_terminal_does() {
        let mut t = vt(8, 3);
        t.feed(b"hi");
        assert_eq!(t.cursor(), (2, 0));
        t.feed(b"\n");
        assert_eq!(t.cursor(), (2, 1), "LF alone must not return the carriage");
        t.feed(b"\r");
        assert_eq!(t.cursor(), (0, 1));
        t.feed(b"there");
        assert_eq!(rows(&t), ["hi      ", "there   ", "        "]);
        // Backspace steps back within the row and stops at the margin.
        t.feed(b"\x08\x08");
        assert_eq!(t.cursor(), (3, 1));
        t.feed(b"\r\x08");
        assert_eq!(
            t.cursor(),
            (0, 1),
            "backspace must not wrap to the row above"
        );
    }

    /// **Deferred wrap.** Filling the last column leaves the cursor on it; the next printable wraps.
    /// A terminal that wrapped eagerly would scroll a full-width line before anything asked it to.
    #[test]
    fn the_right_margin_wraps_late_not_early() {
        let mut t = vt(4, 3);
        t.feed(b"abcd");
        assert_eq!(t.cursor(), (3, 0), "the cursor stays on the last column");
        assert_eq!(rows(&t)[1], "    ", "nothing has moved to the next row yet");
        t.feed(b"e");
        assert_eq!(t.cursor(), (1, 1));
        assert_eq!(rows(&t), ["abcd", "e   ", "    "]);

        // And a CR arriving right after a full line finds the cursor on the same row.
        let mut t = vt(4, 3);
        t.feed(b"abcd\rX");
        assert_eq!(rows(&t)[0], "Xbcd");
        assert_eq!(rows(&t)[1], "    ");
    }

    /// A line feed on the bottom row scrolls, and scrolling exposes a blank row rather than the
    /// row that used to be there.
    #[test]
    fn the_bottom_row_scrolls() {
        let mut t = vt(4, 3);
        t.feed(b"one\r\ntwo\r\nsix\r\n");
        assert_eq!(rows(&t), ["two ", "six ", "    "]);
        assert_eq!(t.cursor(), (0, 2), "the cursor stays on the bottom row");
        t.feed(b"new\r\nend");
        assert_eq!(rows(&t), ["six ", "new ", "end "]);
        // Scrolling is whole-screen damage, honestly reported.
        assert_eq!(
            t.damage(),
            Some(CellRect {
                col: 0,
                row: 0,
                cols: 4,
                rows: 3
            }),
        );

        // The same claim with nothing else in the frame. Above, the printing and the cursor
        // between them already dirty the whole grid, so a scroll that reported no damage at all
        // would still add up to the right rectangle. A bare LF on the bottom row moves every row
        // and writes no cell, which is the only shape that can tell the two apart.
        let mut t = vt(4, 3);
        t.set_cursor_visible(false);
        t.feed(b"ab\r\ncd\r\nef");
        t.take_damage();
        t.feed(b"\r\n");
        assert_eq!(rows(&t), ["cd  ", "ef  ", "    "]);
        assert_eq!(
            t.take_damage(),
            Some(CellRect {
                col: 0,
                row: 0,
                cols: 4,
                rows: 3
            }),
            "a scroll moves every row, so the whole grid is damage",
        );
    }

    /// Cursor sequences, with the parameter defaults ANSI specifies: an absent or zero parameter is
    /// one, and `CUP`'s parameters are one-based on the wire.
    #[test]
    fn the_cursor_sequences_use_ansi_defaults() {
        let mut t = vt(8, 4);
        t.feed(b"\x1b[3;5H");
        assert_eq!(t.cursor(), (4, 2), "CSI H is row;col and one-based");
        t.feed(b"\x1b[A");
        assert_eq!(t.cursor(), (4, 1), "an absent parameter means one");
        t.feed(b"\x1b[0B");
        assert_eq!(t.cursor(), (4, 2), "a zero parameter means one too");
        t.feed(b"\x1b[2D");
        assert_eq!(t.cursor(), (2, 2));
        t.feed(b"\x1b[99C");
        assert_eq!(t.cursor(), (7, 2), "motion clamps to the grid");
        t.feed(b"\x1b[99A");
        assert_eq!(t.cursor(), (7, 0));
        t.feed(b"\x1b[H");
        assert_eq!(t.cursor(), (0, 0), "CSI H with no parameters is home");
        // The far edges. A clamp that is one too generous parks the cursor off the grid, where
        // every subsequent write is silently discarded by `put`.
        t.feed(b"\x1b[99B");
        assert_eq!(t.cursor(), (0, 3), "downward motion clamps to the last row");
        t.feed(b"\x1b[99;99H");
        assert_eq!(t.cursor(), (7, 3), "CUP clamps on both axes");
    }

    /// Erasing, in all three modes of both verbs, and the property that makes `CSI K` useful to a
    /// line discipline: it erases **to the end of the line** and leaves the cursor alone.
    #[test]
    fn erasing_clears_what_it_says_and_no_more() {
        let mut t = vt(6, 2);
        t.feed(b"abcdef\r\nghijkl");
        t.feed(b"\x1b[2;3H\x1b[K");
        assert_eq!(rows(&t), ["abcdef", "gh    "]);
        assert_eq!(t.cursor(), (2, 1), "erase in line must not move the cursor");
        t.feed(b"\x1b[1;4H\x1b[1K");
        assert_eq!(
            rows(&t),
            ["    ef", "gh    "],
            "mode 1 erases through the cursor"
        );
        t.feed(b"\x1b[2K");
        assert_eq!(rows(&t), ["      ", "gh    "]);

        let mut t = vt(6, 2);
        t.feed(b"abcdef\r\nghijkl\x1b[1;4H\x1b[J");
        assert_eq!(
            rows(&t),
            ["abc   ", "      "],
            "erase to the end of the screen"
        );
        let mut t = vt(6, 2);
        t.feed(b"abcdef\r\nghijkl\x1b[2J");
        assert_eq!(rows(&t), ["      ", "      "]);
        assert_eq!(t.cursor(), (5, 1), "CSI 2J does not home the cursor");
    }

    /// **Erasing writes spaces in the current rendition**, so clearing on a coloured background
    /// leaves the background. A terminal that erased to black would leave a gap in a coloured line,
    /// and that is exactly the redraw a line discipline does on every backspace.
    #[test]
    fn erasing_keeps_the_current_background() {
        let mut t = vt(4, 1);
        t.feed(b"\x1b[44mxy\x1b[K");
        for c in 0..4 {
            assert_eq!(
                t.cell(c, 0).attr.bg(),
                Colour::Indexed(4),
                "column {c} lost its background"
            );
        }
        assert_eq!(t.cell(3, 0).ch, ' ');
    }

    /// SGR: the colours, the reverse flag, and the two rules a terminal actually needs. Bold is a
    /// bright foreground, and setting a colour afterwards keeps the brightness.
    #[test]
    fn sgr_sets_colour_brightness_and_reverse() {
        let mut t = vt(8, 1);
        t.feed(b"\x1b[31ma");
        assert_eq!(t.cell(0, 0).attr, attr(1, DEFAULT_BG));
        t.feed(b"\x1b[1mb");
        assert_eq!(ink(&t, 1, 0), PALETTE[9], "bold must brighten the colour");
        t.feed(b"\x1b[32mc");
        assert_eq!(
            ink(&t, 2, 0),
            PALETTE[10],
            "a new colour keeps the bold bit"
        );
        t.feed(b"\x1b[22md");
        assert_eq!(ink(&t, 3, 0), PALETTE[2]);
        t.feed(b"\x1b[7;44me");
        assert_eq!(t.cell(4, 0).attr, attr(2, 4).with(REVERSE, true));
        t.feed(b"\x1b[0mf");
        assert_eq!(t.cell(5, 0).attr, Attr::DEFAULT, "SGR 0 resets everything");
        t.feed(b"\x1b[mg");
        assert_eq!(t.cell(6, 0).attr, Attr::DEFAULT, "an empty SGR is a reset");
    }

    /// **The three ways to name a colour resolve to the numbers every emulator agrees on.** The
    /// 256-colour table is arithmetic, so it is checked at its seams: the last of the sixteen, the
    /// cube's first, a mid-cube entry whose three channels all differ (so a swapped axis fails),
    /// the cube's last, and both ends of the grey ramp. Values are xterm's, written out.
    #[test]
    fn indexed_and_24_bit_colours_resolve_to_xterms_numbers() {
        assert_eq!(Colour::Indexed(15).resolve(), PALETTE[15]);
        assert_eq!(Colour::Indexed(16).resolve(), 0x00_0000);
        // 16 + 36*1 + 6*2 + 3 = 67: red step 1, green step 2, blue step 3.
        assert_eq!(Colour::Indexed(67).resolve(), 0x5f_87af);
        assert_eq!(Colour::Indexed(231).resolve(), 0xff_ffff);
        assert_eq!(Colour::Indexed(232).resolve(), 0x08_0808);
        assert_eq!(Colour::Indexed(255).resolve(), 0xee_eeee);
        assert_eq!(Colour::Rgb(0xb5, 0x89, 0x00).resolve(), 0xb5_8900);
    }

    /// **Truecolour and the 256-colour form, as a program sends them**, including both in one
    /// sequence (ten parameters, past the old limit of four) and a rendition after them, which is
    /// the case that proves the extended forms consume exactly the parameters they name.
    #[test]
    fn sgr_takes_24_bit_and_256_colour_foregrounds_and_backgrounds() {
        let mut t = vt(8, 1);
        t.feed(b"\x1b[38;2;181;137;0ma");
        assert_eq!(t.cell(0, 0).attr.fg(), Colour::Rgb(181, 137, 0));
        t.feed(b"\x1b[48;5;67mb");
        assert_eq!(t.cell(1, 0).attr.bg(), Colour::Indexed(67));
        assert_eq!(
            t.cell(1, 0).attr.fg(),
            Colour::Rgb(181, 137, 0),
            "the foreground stays"
        );
        t.feed(b"\x1b[0;38;5;196;48;2;0;43;54;4mc");
        let c = t.cell(2, 0).attr;
        assert_eq!(
            (c.fg(), c.bg()),
            (Colour::Indexed(196), Colour::Rgb(0, 43, 54))
        );
        assert!(
            c.is_underlined(),
            "the rendition after two extended colours was lost"
        );
        // Bold does not brighten a colour that was not one of the eight: xterm's rule.
        t.feed(b"\x1b[1md");
        assert_eq!(ink(&t, 3, 0), Colour::Indexed(196).resolve());
        // The bright backgrounds, which the one-byte rendition had no room for.
        t.feed(b"\x1b[0;104me");
        assert_eq!(t.cell(4, 0).attr.bg(), Colour::Indexed(12));
    }

    /// **A malformed extended colour ends the sequence's effect** rather than reading its missing
    /// components out of whatever follows: a `38;2` with one component, an out-of-range component,
    /// and an unknown colour space. What came *before* it in the same sequence still applies.
    #[test]
    fn a_malformed_extended_colour_is_not_half_applied() {
        let mut t = vt(8, 1);
        t.feed(b"\x1b[31;38;2;10ma");
        assert_eq!(t.cell(0, 0).attr, attr(1, DEFAULT_BG), "missing components");
        t.feed(b"\x1b[0;38;2;10;300;10;4mb");
        assert_eq!(t.cell(1, 0).attr, Attr::DEFAULT, "a component past 255");
        t.feed(b"\x1b[38;3;1;2;3;4mc");
        assert_eq!(
            t.cell(2, 0).attr,
            Attr::DEFAULT,
            "colour space 3 is not one"
        );
        t.feed(b"\x1b[38;5md");
        assert_eq!(
            t.cell(3, 0).attr,
            Attr::DEFAULT,
            "an index that never arrived"
        );
    }

    /// **Underline and strikethrough are ink across the whole cell, on their own rows**, and each
    /// has an off switch. Read from the pixels, because a flag that is set and never drawn is the
    /// failure a person would see and an attribute comparison would not. A space is used so every
    /// ink pixel in the cell is the line's.
    #[test]
    fn underline_and_strikethrough_draw_a_full_width_line_on_their_rows() {
        let mut t = vt(4, 1);
        t.set_cursor_visible(false);
        t.feed(b"\x1b[4m \x1b[9m \x1b[24m \x1b[29m ");
        let (fg, bg) = Attr::DEFAULT.colours();
        let row_of = |col: u32, gy: u32| -> std::vec::Vec<u32> {
            (0..bitmap_font::GLYPH_W)
                .map(|gx| t.pixel(col * bitmap_font::GLYPH_W + gx, gy))
                .collect()
        };
        let full = std::vec![fg; bitmap_font::GLYPH_W as usize];
        let none = std::vec![bg; bitmap_font::GLYPH_W as usize];
        for gy in 0..bitmap_font::GLYPH_H {
            let under = gy == UNDERLINE_ROW;
            let strike = gy == STRIKETHROUGH_ROW;
            assert_eq!(
                &row_of(0, gy),
                if under { &full } else { &none },
                "underline, row {gy}"
            );
            assert_eq!(
                &row_of(1, gy),
                if under || strike { &full } else { &none },
                "both, row {gy}"
            );
            assert_eq!(
                &row_of(2, gy),
                if strike { &full } else { &none },
                "SGR 24, row {gy}"
            );
            assert_eq!(row_of(3, gy), none, "SGR 29, row {gy}");
        }
        assert_ne!(UNDERLINE_ROW, STRIKETHROUGH_ROW);
    }

    /// **Dim, invisible and the cursor, in the order [`Attr::colours`] documents.** Dim is the
    /// channel midpoint toward the background; invisible paints the text in the background; and the
    /// block cursor on a concealed cell still shows, which is what applying invisible last buys.
    #[test]
    fn dim_and_invisible_resolve_in_the_documented_order() {
        let red_on_blue = Attr::new(Colour::Rgb(200, 0, 0), Colour::Rgb(0, 0, 100));
        assert_eq!(
            red_on_blue.with(DIM, true).colours(),
            (0x64_0032, 0x00_0064)
        );
        assert_eq!(
            red_on_blue.with(INVISIBLE, true).colours(),
            (0x00_0064, 0x00_0064)
        );
        assert_eq!(
            red_on_blue
                .with(INVISIBLE, true)
                .with(REVERSE, true)
                .colours(),
            (0xc8_0000, 0xc8_0000),
            "reversed and concealed is a solid block of the old foreground"
        );

        let mut t = vt(4, 1);
        t.feed(b"\x1b[8mA\x1b[28mB\x1b[8m");
        let (fg, bg) = Attr::DEFAULT.colours();
        let ink_in = |col: u32| {
            (0..bitmap_font::GLYPH_W * bitmap_font::GLYPH_H).any(|i| {
                let (gx, gy) = (i % bitmap_font::GLYPH_W, i / bitmap_font::GLYPH_W);
                t.pixel(col * bitmap_font::GLYPH_W + gx, gy) == fg
            })
        };
        assert!(!ink_in(0), "concealed text drew ink");
        assert!(ink_in(1), "SGR 28 reveals");
        // The cursor sits on column 2, which is blank; conceal it and it is still a block.
        t.feed(b"\x1b[D\x1b[C");
        assert_eq!(t.cursor(), (2, 0));
        t.feed(b" \x1b[D");
        assert_eq!(
            t.pixel(2 * bitmap_font::GLYPH_W, 0),
            fg,
            "the cursor vanished on a concealed cell"
        );
        assert_ne!(fg, bg);
        assert!(t.cell(2, 0).attr.is_invisible());
    }

    /// **A sequence this engine does not implement is swallowed whole.** The failure mode this
    /// prevents is the one everybody has seen: an unimplemented escape whose final byte lands on the
    /// screen as a stray letter, corrupting a line that was otherwise fine.
    #[test]
    fn an_unknown_sequence_leaves_nothing_on_the_screen() {
        let mut t = vt(8, 1);
        t.feed(b"a\x1b[?25lb\x1b[38;5;120mc\x1b]0;title\x07d\x1bZe");
        assert_eq!(
            rows(&t)[0],
            "abcde   ",
            "an unimplemented sequence left bytes on the screen",
        );
        // A sequence split across two feeds is still one sequence: the parser is a state machine
        // across calls, which is what a byte-at-a-time IPC path needs.
        let mut t = vt(8, 1);
        t.feed(b"x\x1b");
        t.feed(b"[2");
        t.feed(b"Cy");
        assert_eq!(rows(&t)[0], "x  y    ");

        // A string sequence ends at ST (`ESC \`) as well as at BEL, and an ESC inside it that is
        // not ST is part of the string. Only the BEL terminator is exercised above, so the whole
        // ESC-in-a-string state was reachable but never distinguished from Ground.
        let mut t = vt(8, 1);
        t.feed(b"a\x1b]0;title\x1b\\b");
        assert_eq!(rows(&t)[0], "ab      ");
        let mut t = vt(8, 1);
        t.feed(b"a\x1b]x\x1bZy\x1b\\b");
        assert_eq!(
            rows(&t)[0],
            "ab      ",
            "an ESC that is not ST stays in the string"
        );

        // Control codes with no meaning here are consumed. Drawn instead, they are the font's
        // blanks or its missing-glyph box, in the middle of a line that was otherwise fine.
        let mut t = vt(8, 1);
        t.feed(b"a\x01\x02\x1f\x7fb");
        assert_eq!(rows(&t)[0], "ab      ");
    }

    /// The cursor is part of the picture, so moving it is damage; and a hidden cursor is not.
    #[test]
    fn the_cursor_is_painted_and_therefore_damages() {
        let mut t = vt(4, 2);
        let (fg, bg) = Attr::DEFAULT.colours();
        // Under the cursor, a blank cell shows the *background* colour as its ink field.
        assert_eq!(t.pixel(0, 0), fg, "the block cursor should invert its cell");
        assert_eq!(
            t.pixel(bitmap_font::GLYPH_W, 0),
            bg,
            "and only its own cell"
        );

        t.take_damage();
        t.feed(b"\x1b[1;3H");
        let d = t.damage().expect("moving the cursor must be damage");
        assert!(
            d.col == 0 && d.cols >= 3,
            "both cells must be in the damage"
        );

        t.set_cursor_visible(false);
        assert_eq!(
            t.pixel(2 * bitmap_font::GLYPH_W, 0),
            bg,
            "hidden means not drawn"
        );
    }

    /// Damage is the bounding box of what changed, and writing a cell the value it already holds is
    /// not a change. Without that, a terminal that repainted the same line would flush the screen
    /// every frame and the damage rectangle would be decoration.
    #[test]
    fn damage_is_a_bounding_box_of_real_changes() {
        let mut t = vt(8, 4);
        t.set_cursor_visible(false);
        t.take_damage();
        assert_eq!(t.damage(), None, "a terminal at rest reports no damage");

        t.feed(b"\x1b[2;3Hab");
        assert_eq!(
            t.take_damage(),
            Some(CellRect {
                col: 2,
                row: 1,
                cols: 2,
                rows: 1
            }),
        );
        assert_eq!(t.damage(), None, "taking the damage clears it");

        t.feed(b"\x1b[2;3Hab");
        assert_eq!(t.damage(), None, "rewriting identical cells is not damage");

        t.feed(b"\x1b[1;1HZ\x1b[4;8HY");
        let d = t.take_damage().unwrap();
        assert_eq!(
            (d.col, d.row, d.cols, d.rows),
            (0, 0, 8, 4),
            "two far-apart changes cost the box that contains both",
        );
        assert_eq!(d.to_pixels(), (0, 0, 56, 32));
    }

    /// `ESC c` is a reset and not an erase: it clears in the *default* rendition and homes the
    /// cursor, where `CSI 2J` does neither.
    #[test]
    fn ris_resets_the_rendition_too() {
        let mut t = vt(4, 2);
        t.feed(b"\x1b[41;33mab\x1bc");
        assert_eq!(t.cursor(), (0, 0));
        assert_eq!(t.cell(0, 0).attr, Attr::DEFAULT);
        assert_eq!(rows(&t), ["    ", "    "]);
    }

    /// **The engine parses what the line discipline emits**, checked against the real component
    /// rather than against a list of sequences somebody wrote down.
    ///
    /// This is the interoperability claim milestone 28's contract makes and milestone 29 relies on:
    /// the display terminal's VT engine is fed the same echo stream the serial console gets, so a
    /// sequence `line_editor` emits and this engine does not understand would be a hole between two
    /// components that are otherwise separately correct. Feeding the actual editor closes it, and it
    /// keeps closing it if `line_editor` changes its redraw strategy.
    #[test]
    fn it_understands_the_line_disciplines_echo() {
        struct Echo(Vec<u8>);
        impl line_editor::Sink for Echo {
            fn put(&mut self, bytes: &[u8]) {
                self.0.extend_from_slice(bytes);
            }
        }

        // Type "hello", back up two (^B), insert "XY", delete forward (^D), kill to end (^K), then
        // Enter. That is the editing that produces CSI D, CSI C, CSI K and a mid-line redraw, which
        // is the whole set a display terminal has to understand to show an edited line correctly.
        let mut ld = line_editor::LineDisc::new();
        let mut echo = Echo(Vec::new());
        let mut event = line_editor::Event::None;
        for &b in b"hello\x02\x02XY\x04\x0b\r" {
            event = ld.feed(b, &mut echo);
        }
        assert_eq!(event, line_editor::Event::Line);
        assert!(
            echo.0.windows(3).any(|w| w == b"\x1b[K"),
            "the discipline stopped emitting CSI K: this test is no longer interoperability",
        );
        assert!(
            echo.0.windows(3).any(|w| w == b"\x1b[D"),
            "the discipline stopped emitting cursor motion: likewise",
        );

        let mut t = vt(16, 3);
        t.set_cursor_visible(false);
        t.feed(&echo.0);
        // Whatever redraw strategy the discipline chose, the screen must show the line it completed.
        let line = String::from_utf8_lossy(ld.line()).into_owned();
        assert_eq!(line, "helXY");
        assert_eq!(
            rows(&t)[0].trim_end(),
            line,
            "the grid disagrees with the line the discipline assembled",
        );

        // And the ^L repaint (CSI 2J, CSI H, then the prompt and the line) drives this engine
        // correctly: the screen is cleared and the line is back at the top.
        let mut echo = Echo(Vec::new());
        for &b in b"redraw\x0c" {
            ld.feed(b, &mut echo);
        }
        assert!(echo.0.windows(4).any(|w| w == b"\x1b[2J"));
        t.feed(&echo.0);
        assert_eq!(
            rows(&t)[0].trim_end(),
            "redraw",
            "the ^L repaint did not land at the top of a cleared screen",
        );
        assert_eq!(rows(&t)[1].trim_end(), "", "the old screen survived a ^L");
    }

    /// Each compositor window shows its own banner and its own typed text, so a test that mixed
    /// two windows up, or drew one twice, cannot pass. `script::window` is what the guest builds;
    /// covering it here means a change to the shared script fails on the host in milliseconds
    /// rather than in QEMU minutes later.
    #[test]
    fn each_window_shows_its_own_banner_and_its_own_typing() {
        let mut a = vt(script::COLS, script::ROWS);
        let mut b = vt(script::COLS, script::ROWS);
        script::window(&mut a, 0);
        script::window(&mut b, 1);
        let (ra, rb) = (rows(&a), rows(&b));
        assert_eq!(ra[0].trim_end(), "term0");
        assert_eq!(rb[0].trim_end(), "term1");
        assert_ne!(
            ra, rb,
            "two windows rendering identically would pass a mixed-up test"
        );
        assert!(
            ra.iter().any(|r| r.contains('A')) && rb.iter().any(|r| r.contains('B')),
            "each window must show what was typed at it, not at its neighbour",
        );
    }

    /// The script the milestone-29 tests drive is a fair one: it uses more than one row, more than
    /// one rendition, and leaves a picture that a blank screen, a shifted screen, or a screen
    /// missing its last write cannot match. Asserted here so the QEMU test is not the first thing to
    /// find out otherwise.
    #[test]
    fn the_demo_script_produces_a_picture_worth_checking() {
        // Drive `script::full_screen` rather than rebuilding it here. The guest test renders
        // exactly that, so reconstructing the setup by hand let the two witnesses diverge: this
        // test fed GREETING and stopped, while the screen being graded in QEMU also had TYPED on
        // it. `script.rs` exists so every party agrees, which only works if every party calls it.
        let mut t = vt(script::COLS, script::ROWS);
        script::full_screen(&mut t);
        let seen = rows(&t);
        assert_eq!(seen[0].trim_end(), "nife");
        assert!(
            seen.iter().filter(|r| !r.trim().is_empty()).count() >= 3,
            "the script fills one row: a stride bug would be invisible",
        );
        let attrs: Vec<Attr> = (0..t.rows())
            .flat_map(|r| (0..t.cols()).map(move |c| (c, r)))
            .map(|(c, r)| t.cell(c, r).attr)
            .collect();
        assert!(
            attrs.iter().any(|a| *a != Attr::DEFAULT),
            "the script never changes rendition: the colour path is untested on the machine",
        );
        // The truecolour pass's own two claims reach the machine only if the script carries them:
        // a 24-bit colour, and a line rendition, each on a cell with ink.
        assert!(
            attrs.iter().any(|a| matches!(a.fg(), Colour::Rgb(..))),
            "the script has no 24-bit colour: truecolour is untested on the machine",
        );
        assert!(
            attrs.iter().any(|a| a.is_underlined()),
            "the script has no underline: the line renditions are untested on the machine",
        );
        assert!(
            (0..t.rows()).any(|r| (0..t.cols()).any(|c| t.cell(c, r).ch != ' ')),
            "the script drew nothing",
        );
    }

    /// **The size clamp is exact at the boundary.** `MAX_COLS`/`MAX_ROWS` themselves are legal, one
    /// past either is clamped down, and zero clamps up to one rather than producing an empty grid no
    /// cursor could ever occupy. Milestone 326 (turn a mutation score upward)'s mutation triage.
    #[test]
    fn geometry_clamps_exactly_at_the_boundary_not_one_off_it() {
        let t = Vt::new(MAX_COLS as u32, MAX_ROWS as u32);
        assert_eq!(
            (t.cols(), t.rows()),
            (MAX_COLS as u32, MAX_ROWS as u32),
            "the boundary value itself must not be clamped"
        );

        let t = Vt::new(MAX_COLS as u32 + 1, MAX_ROWS as u32 + 1);
        assert_eq!(
            (t.cols(), t.rows()),
            (MAX_COLS as u32, MAX_ROWS as u32),
            "one past the boundary must be clamped"
        );

        let t = Vt::new(0, 0);
        assert_eq!(
            (t.cols(), t.rows()),
            (1, 1),
            "a zero-sized grid has nowhere for the cursor to be"
        );
    }

    /// **`reset_to` clears the grid, the scrollback and the geometry, in place.** Nothing before this
    /// test called it at all: every other test either builds a `Vt` at its final size with `Vt::new`
    /// or never resizes, so the whole function was dead as far as the suite could tell. Milestone
    /// 326's mutation triage.
    #[test]
    fn reset_to_clears_everything_and_retargets_the_geometry() {
        let mut t = Vt::new(4, 2);
        t.feed(b"one\r\ntwo\r\nthree\r\n"); // scrolls at least once, building real scrollback
        assert!(
            t.scrollback_len() > 0,
            "test setup: something must have scrolled into history"
        );
        t.take_damage();

        t.reset_to(3, 5);
        assert_eq!(
            (t.cols(), t.rows()),
            (3, 5),
            "the geometry actually changed"
        );
        assert_eq!(t.cursor(), (0, 0), "the cursor went home");
        for r in 0..t.rows() {
            let mut buf = [0u8; 3];
            let n = t.row_bytes(r, &mut buf);
            assert_eq!(&buf[..n], b"   ", "row {r} carried old content forward");
        }
        assert_eq!(
            t.take_damage(),
            Some(CellRect {
                col: 0,
                row: 0,
                cols: 3,
                rows: 5
            }),
            "a retarget is damage across the whole new grid",
        );
        assert_eq!(
            t.scrollback_len(),
            0,
            "reset_to must clear the old grid's history, not just the live view"
        );
    }

    /// **Multi-byte UTF-8 decodes to the right code point**, exercising the shift-and-accumulate
    /// arithmetic and the lead-byte masks for all three lengths this engine understands, plus the two
    /// ways a sequence can go wrong: truncation (a non-continuation byte arrives early) and a byte
    /// that can never start a sequence. Nothing before this test fed a non-ASCII byte at all.
    /// Milestone 326's mutation triage.
    #[test]
    fn utf8_decodes_every_sequence_length_and_recovers_from_a_bad_one() {
        // An accented Latin letter: two bytes, one continuation.
        let mut t = vt(8, 1);
        t.feed("é".as_bytes());
        assert_eq!(t.cell(0, 0).ch, 'é');

        // A CJK character: three bytes, two continuations.
        let mut t = vt(8, 1);
        t.feed("日".as_bytes());
        assert_eq!(t.cell(0, 0).ch, '日');

        // An astral character outside the BMP: four bytes, three continuations.
        let mut t = vt(8, 1);
        t.feed("🎉".as_bytes());
        assert_eq!(t.cell(0, 0).ch, '🎉');

        // A truncated sequence: the lead byte of a two-byte character, then a byte that is not a
        // continuation byte. The truncated character draws the replacement, and the byte that
        // interrupted it is reprocessed as its own character rather than eaten.
        let mut t = vt(8, 1);
        t.feed(b"\xc3Z");
        assert_eq!(t.cell(0, 0).ch, '\u{fffd}', "truncated sequence");
        assert_eq!(
            t.cell(1, 0).ch,
            'Z',
            "the interrupting byte must still be typed"
        );

        // A byte that can never start a sequence (a bare continuation byte): the replacement,
        // immediately, with nothing held waiting for continuation bytes that were never coming.
        let mut t = vt(8, 1);
        t.feed(b"\x80A");
        assert_eq!(t.cell(0, 0).ch, '\u{fffd}', "a lone continuation byte");
        assert_eq!(t.cell(1, 0).ch, 'A');
    }

    /// **Scrollback survives the ring's wrap, and `Vt::cell` reads it back in the right order.**
    /// Feeding 350 lines through a two-row grid scrolls 349 times, comfortably past
    /// [`SCROLLBACK_ROWS`] (300): the ring overwrites its oldest 49 pushes, and this is the test that
    /// the retained 300 are the right ones, at the right ages, at both edges of the ring rather than
    /// only near where it happens to start. It doubles as the test for [`SCROLLBACK_CELLS`] itself:
    /// that constant sizes the `scrollback` array, nothing else in this suite pushes past
    /// `SCROLLBACK_CELLS / cols` rows, and a wrong constant here is an index past a too-small array,
    /// which panics rather than silently misdrawing a pixel. Milestone 326's mutation triage: before
    /// this test, nothing in the suite called `scroll_up`, `scroll_down`, `view_offset`, or
    /// `scrollback_len`, and nothing scrolled far enough to read scrollback at all.
    #[test]
    fn scrollback_survives_the_rings_wrap_and_reads_back_in_order() {
        const LINES: u32 = 350;
        let mut t = vt(4, 2);
        for i in 0..LINES {
            t.feed(std::format!("L{i:03}\r\n").as_bytes());
        }

        // 349 scrolls happened (every line after the first fills the grid and pushes one row), and
        // the ring caps at SCROLLBACK_ROWS.
        assert_eq!(t.scrollback_len(), SCROLLBACK_ROWS as u32);

        // The live grid holds the last line printed; nothing was fed after it, so the bottom row is
        // still blank.
        assert_eq!(rows(&t), ["L349", "    "]);

        // Scrolling clamps at the ring's true depth, not at whatever was asked for.
        t.scroll_up(u32::MAX);
        assert_eq!(t.view_offset(), SCROLLBACK_ROWS as u32);

        // Six checkpoints spanning the ring, including both edges: age 0 (the row pushed right
        // before the wrap stopped, where `sb_tail` last wrote) and age SCROLLBACK_ROWS - 1 (the
        // oldest survivor, on the far side of the ring from where it is currently writing). A wrong
        // `+`/`*`/`%` in the ring's index arithmetic is very unlikely to agree with the right one at
        // all six. Both display rows are checked, not just the top one: at row 0, `Vt::cell`'s
        // `age = view_offset - 1 - row` has `row == 0`, so a `-row` mutated to `+row` is invisible
        // there and needs row 1 (and, at `offset == 1`, row 1 comes from the *live* half of
        // `Vt::cell` instead, which is what exercises `live_row = row - view_offset`).
        for offset in [1u32, 2, 50, 150, 299, 300] {
            t.scroll_down(u32::MAX);
            t.scroll_up(offset);
            assert_eq!(
                t.view_offset(),
                offset,
                "view_offset did not reach what was asked for"
            );
            let seen = rows(&t);
            assert_eq!(
                seen[0],
                std::format!("L{:03}", LINES - 1 - offset),
                "view_offset {offset}: wrong row surfaced from the ring at row 0"
            );
            assert_eq!(
                seen[1],
                std::format!("L{:03}", LINES - offset),
                "view_offset {offset}: wrong row surfaced from the ring at row 1"
            );
        }

        // Scrolling back down reaches live ground exactly at zero, not before or after it.
        t.scroll_down(u32::MAX);
        assert_eq!(t.view_offset(), 0);
        assert_eq!(rows(&t), ["L349", "    "]);

        // Typing snaps the view back to live even from deep in history.
        t.scroll_up(200);
        assert_ne!(t.view_offset(), 0);
        t.feed(b"!");
        assert_eq!(
            t.view_offset(),
            0,
            "new output must snap the view back to live"
        );
    }

    /// **`put` and `damage_cell` refuse a coordinate where only one axis is out of range.** Nothing
    /// in this crate ever calls either with such a coordinate (every caller clamps first: `print`
    /// always writes at the cursor, which is always in range, and `erase_line` bounds its column with
    /// `.min(self.cols)`), so this reaches them directly, the way this module already can
    /// (`use super::*` at the top of `mod tests`, which is the same crate as the private methods
    /// under test). Milestone 326's mutation triage.
    #[test]
    fn put_and_damage_cell_reject_a_partly_out_of_range_coordinate() {
        let mut t = vt(4, 3);
        let mark = Cell {
            ch: 'X',
            attr: Attr::DEFAULT,
        };

        // Column out of range, row in range.
        t.put(t.cols(), 1, mark);
        assert_eq!(
            t.cells,
            [Cell::default(); MAX_CELLS],
            "a partly out-of-range put must be a no-op"
        );
        assert_eq!(t.damage(), None);

        // Row out of range, column in range.
        t.put(1, t.rows(), mark);
        assert_eq!(t.cells, [Cell::default(); MAX_CELLS]);
        assert_eq!(t.damage(), None);

        t.damage_cell(t.cols(), 1);
        assert_eq!(
            t.damage(),
            None,
            "a partly out-of-range damage_cell must be a no-op"
        );
        t.damage_cell(1, t.rows());
        assert_eq!(t.damage(), None);
    }
}
