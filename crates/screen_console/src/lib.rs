//! **Text on a screen, for a machine with no serial port** (milestone 243).
//!
//! Every word nife had ever said, it said down a UART: the boot tour on all three machines, the
//! console server, the kernel's fault reports, and every automated gate that reads any of them.
//! **A commodity machine does not have one.** The six machines in calef's house that milestone 87's
//! USB stick could already boot (two desktops, a laptop, an Intel `MacBook`, cordoba) have between
//! them no serial port at all, and until this crate existed a nife stick booted on one of them
//! would have printed the loader's four lines through the firmware's own console and then gone
//! permanently silent at `ExitBootServices`.
//!
//! This is the smallest thing that fixes that: given a linear framebuffer the previous boot stage
//! found ([`machine_discovery::framebuffer`]) and the tree's existing 7x8 font ([`bitmap_font`]),
//! put bytes on the screen.
//!
//! # Why this is not `video_terminal`
//!
//! `video_terminal::Vt` is the real terminal: a cell grid, a full escape-sequence parser, 300
//! rows of scrollback, damage rectangles. It is the right engine for the interactive terminal
//! (milestone 177 wires it) and it is deliberately **not** used here, for three reasons that all
//! point the same way.
//!
//! - **It is a value of several hundred kilobytes.** Its own documentation says so and warns the
//!   reader off putting one on a stack. In a kernel it would be a static of that size, in `.bss`,
//!   present on every architecture, for a diagnostic path.
//! - **It would put an escape-sequence parser in the TCB.** A state machine over untrusted-ish
//!   bytes is exactly the kind of thing this project keeps *out* of the kernel, and the kernel's
//!   own `println!` emits no escape sequences to parse.
//! - **The thing being reported is often the reason the machine is broken.** That is the block's
//!   own constraint on this milestone, and it argues for the console with the least state that
//!   could work. This one holds a cursor, a geometry and a scale: six `u32`s and no buffer.
//!
//! What is deliberately shared is the **font**, so the letters on an early boot screen and the
//! letters in the graphical terminal are the same letters.
//!
//! # What it does not do, and that is a design rather than a gap
//!
//! No colour changes, no escape sequences, no cursor, no scrollback, no reflow. A newline moves
//! down, a carriage return moves to column zero, everything else is a glyph, and running off the
//! bottom scrolls the picture up by one row of cells. **A console that cannot be put into a
//! surprising state is worth more here than a capable one**, because its whole job is to be
//! working at the moment something else is not.
//!
//! # Scale
//!
//! A wide screen draws each font pixel as a square block, at the largest integer scale that still
//! leaves [`MIN_COLUMNS`] columns: 120, calef's choice on 2026-10-04 after 80 proved too big on
//! xenon's monitor. A 1920x1080 monitor gets two (137 columns by 67 rows); anything under 1680
//! wide, QEMU's 1280x800 OVMF screen among them, stays at one.
//!
//! # Examples
//!
//! A whole console, painted into ordinary memory, which is exactly what the host tests below do and
//! what makes this crate provable without a screen:
//!
//! ```
//! use machine_discovery::framebuffer::{Framebuffer, PixelOrder};
//! use screen_console::ScreenConsole;
//!
//! // Two cells wide, one tall: 14x8 pixels at four bytes each.
//! let screen = Framebuffer { base: 0, width: 14, height: 8, stride: 56, order: PixelOrder::Bgrx };
//! let mut pixels = [0u8; 56 * 8];
//! let mut console = ScreenConsole::new(screen).expect("a screen big enough for one cell");
//!
//! assert_eq!(console.size(), (2, 1));
//! console.write(&mut pixels, "F");
//!
//! // The top-left pixel of an `F` is background and the one beside it is ink, which is the
//! // property a mirrored or transposed painter gets wrong. See `bitmap_font`'s own example.
//! let pixel = |x: usize, y: usize| u32::from_le_bytes(
//!     pixels[y * 56 + x * 4..y * 56 + x * 4 + 4].try_into().unwrap(),
//! );
//! assert_eq!(pixel(0, 0), ScreenConsole::BACKGROUND);
//! assert_eq!(pixel(1, 0), ScreenConsole::FOREGROUND);
//! ```
//!
//! # BUGS
//!
//! - **Scrolling reads the framebuffer back**, one screenful of bytes per scrolled row, and a
//!   framebuffer aperture is mapped uncacheable by every caller this crate has. That is cheap under
//!   QEMU and is the slowest thing here on real silicon; a write-combining mapping or a shadow copy
//!   in RAM would both fix it and both cost more than this milestone is buying. The boot tour is
//!   shorter than a 1280x800 screen is tall, so nothing scrolls during the boot this was built for.
//! - **Only 32-bit pixels.** [`machine_discovery::framebuffer::PixelOrder`] expresses the two byte
//!   orders UEFI reports and nothing else, so a 24-bit packed or 16-bit mode has no console. Every
//!   machine in the fleet reports one of the two.
//! - **On the one real machine that has run this, the screen shows a regular grid instead of
//!   text**, and it is the painting path rather than scrolling or the halt path. Observed on xenon
//!   on 2026-09-04 and 2026-09-17. The discriminating run was the **tour** image on 2026-09-17
//!   (`bench/xenon-2026-09-17/tour-display-225100.log`, video `IMG_4145`): about 40 lines onto a
//!   135-row screen, so **nothing scrolled**, and it ends at a live shell rather than a panic, so
//!   the machine was **not halted**. The grid appeared anyway, which eliminates both of the
//!   explanations the first sighting allowed for.
//!
//!   **The kernel's own geometry is not wrong**: that boot reported `8294400` bytes of
//!   framebuffer, exactly 1920 x 1080 x 4, so [`Framebuffer::span`] and a 7680-byte stride agree
//!   with each other and with the panel. What the screen shows is single pixels at regular
//!   vertical intervals, in columns at regular horizontal intervals, with the columns **leaning**.
//!   A constant lean down the screen is the signature of a fixed per-row drift between the address
//!   this crate computes for row *n* and the address the display scans out for row *n*, which is
//!   what a pitch disagreement produces. That is a hypothesis read off a photograph and **not a
//!   measurement**; counting columns in a JPEG is not evidence this tree accepts.
//!
//!   **The experiment that turns it into a number needs no bench time to write**: paint one
//!   horizontal run of known length at a known row before printing anything. A correct pitch shows
//!   one line; a mismatched one shows a line that steps or wraps, and the step is the delta.
//!
//!   Recorded here rather than fixed because an earlier session nearly wrote up a framebuffer
//!   defect from a photograph of a halted machine that had been displaying text correctly, and the
//!   lesson taken was to name what a photograph can and cannot settle.
//! - **Nothing here is proved on real silicon.** It is proved on the host and under OVMF. A
//!   framebuffer that works under QEMU's emulated adapter is not a framebuffer that works on
//!   Graeme's laptop, and `notes/serial-less-output.md` carries the bench procedure that would
//!   settle it.
//! - **The right-hand and bottom edges are never painted** when the screen is not a whole number of
//!   cells. 1280 is not a multiple of 7, so a 1280-pixel-wide screen owns 182 columns and leaves
//!   six pixels of whatever the firmware last drew. The same is true of `display_terminal`'s
//!   scanout wiring and for the same reason.
//!
//! Name: ratified 2026-09-13 (calef, working the unratified worklist), and ruled as half of a pair:
//! `serial_console` and `screen_console` are two consoles over two wires, named for where the text
//! comes out. Coined by milestone 243's lane. The other half is `crates/board_console`, whose
//! rename to `serial_console` is recorded there and not yet performed.
//!
//! A noun, `snake_case`, naming what the thing is rather than how it works: a console on a screen.
//!
//! Refused `framebuffer_console`, because "framebuffer" already names the thing this writes *into*
//! (`machine_discovery::framebuffer`) and a reader meeting both would have to hold two senses of
//! one word. Refused `pixel_console`, which names the unit rather than the surface. Refused
//! `early_console`, which says when it runs rather than what it is, and is wrong about that too:
//! nothing here is early-only.
//!
//! **Refused `video_terminal`, and this is the explanation behind a standing gate NOTE.** That name
//! is taken by the crate this one deliberately is not: `video_terminal` is the terminal *emulator*,
//! holding escape sequences, a cursor and scrollback, where this is a place to put text. Until
//! 2026-09-13 the distinction sat here as a parenthetical rather than a refusal, so
//! `script/names` reported `video_terminal` as recorded-refused-and-also-live with nothing anywhere
//! saying why both are correct.
//!
//! **The NOTE stays, and should.** It flags a word that is refused in one block and live in
//! another, which is exactly true here and is the co-existence a reader should be told about; it
//! is not a complaint that the reason is missing. Checked rather than assumed: the NOTE is still
//! reported after this paragraph was written.

#![no_std]

use machine_discovery::framebuffer::{Framebuffer, PixelOrder};

/// **The fewest columns a console is scaled down to**: 120. A screen is drawn at the largest
/// integer scale that still leaves this many columns (see [`ScreenConsole::scale_for`]). A choice,
/// not a derivation, and this is the one place it lives: calef's starting choice on 2026-10-04
/// (UTC), "80 is a good place to start", then 120 the same evening ("80 is too big") after seeing
/// 80 on xenon's monitor at 3x. 80 was the Kaypro II's width, whose character cell §100 (the
/// terminal font) took the font from. Introduced that day on the xenon bench of milestone 261 (the
/// NVMe driver leaves the kernel), when the 1:1 font on a 1920-wide monitor could not be read or
/// photographed.
pub const MIN_COLUMNS: u32 = 120;

/// **Where a surface lands on a firmware screen**: the arithmetic a driver needs to copy a
/// rectangle of pixels it was handed onto a screen the firmware set up (the shell on the firmware
/// screen, rung 1b of milestone 198; `components/src/framebuffer_driver.rs` is its one caller).
///
/// The other half of this crate paints *text* into a framebuffer. This paints *pixels* someone
/// else already drew, and it lives here rather than in the driver because every framebuffer bug in
/// history is a stride bug and a stride bug is only catchable on the host: the driver is a
/// ring-3 program with no test harness, and this is a pure function of five numbers.
///
/// What it holds is the part of the screen a surface covers: the surface's size clipped to the
/// screen's, placed at the screen's top-left corner, plus the screen's stride and byte order. A
/// surface larger than the screen loses its right and bottom edges; a surface smaller than the
/// screen leaves the rest of it alone.
///
/// Name: provisional (the shell on the firmware screen's lane). "Aperture" is the word the
/// tree already uses for the firmware's framebuffer as a device window
/// (`kernel/src/arch/x86_64/machine.rs`, milestone 243).
///
/// # Examples
///
/// ```
/// use machine_discovery::framebuffer::{Framebuffer, PixelOrder};
/// use screen_console::Aperture;
///
/// // A 4x2 screen with four bytes of padding per row, and a 3x3 surface: the surface is clipped
/// // to three columns and two rows.
/// let screen = Framebuffer { base: 0, width: 4, height: 2, stride: 20, order: PixelOrder::Rgbx };
/// let aperture = Aperture::new(&screen, 3, 3).expect("a screen and a surface both bigger than 0");
/// assert_eq!(aperture.size(), (3, 2));
///
/// // Copy the whole visible part of a surface whose every pixel is pure red, 0x00ff0000.
/// let mut pixels = [0u8; 40];
/// assert!(aperture.copy(0, 0, 3, 2, |_, _| 0x00ff_0000, |at, word| {
///     pixels[at..at + 4].copy_from_slice(&word.to_le_bytes());
/// }));
/// // An rgbx screen stores red in its first byte, and the fourth column was never touched.
/// assert_eq!(&pixels[0..4], &[0xff, 0, 0, 0]);
/// assert_eq!(&pixels[12..16], &[0, 0, 0, 0]);
///
/// // A rectangle that leaves the visible part is refused rather than clipped.
/// assert!(!aperture.copy(0, 0, 4, 2, |_, _| 0, |_, _| {}));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Aperture {
    width: u32,
    height: u32,
    stride: u32,
    order: PixelOrder,
}

impl Aperture {
    /// The part of `screen` a `surface_width` by `surface_height` surface covers, or `None` when the
    /// screen's own geometry does not close ([`Framebuffer::span`]) or either size is zero.
    #[must_use]
    pub fn new(screen: &Framebuffer, surface_width: u32, surface_height: u32) -> Option<Self> {
        screen.span()?;
        Self::checked(
            screen.width.min(surface_width),
            screen.height.min(surface_height),
            screen.stride,
            screen.order,
        )
    }

    /// The one validation both constructors share: a non-empty rectangle whose every row fits in
    /// the stride, and whose last byte is addressable.
    fn checked(width: u32, height: u32, stride: u32, order: PixelOrder) -> Option<Self> {
        if width == 0 || height == 0 || (stride as u64) < width as u64 * 4 {
            return None;
        }
        let aperture = Self {
            width,
            height,
            stride,
            order,
        };
        aperture.checked_span()?;
        Some(aperture)
    }

    /// The covered part, in pixels: `(width, height)`. This is what the driver answers a client's
    /// `INFO` with, so the client lays its grid out over what can actually be seen.
    #[must_use]
    pub const fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// How many bytes from pixel (0, 0) a copy can reach: the last covered row's last pixel, plus
    /// one. **This, and not the screen's whole span, is what the driver needs mapped**, so a driver
    /// is never handed the rows of the screen it will never paint.
    #[must_use]
    pub fn span(&self) -> usize {
        self.checked_span().unwrap_or(0)
    }

    fn checked_span(&self) -> Option<usize> {
        let bytes = (self.stride as u64)
            .checked_mul(self.height as u64 - 1)?
            .checked_add(self.width as u64 * 4)?;
        usize::try_from(bytes).ok()
    }

    /// **Two words that carry this across a spawn**, for a driver that is told its geometry in
    /// registers because it holds no capability that could describe it. [`Self::from_words`] is
    /// the other half, and the pair lives here so the kernel that packs and the driver that
    /// unpacks are one definition rather than two that could drift (AGENTS.md rule 7).
    #[must_use]
    pub const fn to_words(&self) -> (u64, u64) {
        let order = match self.order {
            PixelOrder::Bgrx => 0,
            PixelOrder::Rgbx => 1,
        };
        (
            self.width as u64 | (self.height as u64) << 32,
            self.stride as u64 | order << 32,
        )
    }

    /// The inverse of [`Self::to_words`], validated the same way [`Self::new`] is, because the
    /// words crossed a process boundary and the driver should not paint on the strength of a
    /// geometry it cannot check.
    #[must_use]
    pub fn from_words(size: u64, layout: u64) -> Option<Self> {
        let order = match layout >> 32 {
            0 => PixelOrder::Bgrx,
            1 => PixelOrder::Rgbx,
            _ => return None,
        };
        Self::checked(size as u32, (size >> 32) as u32, layout as u32, order)
    }

    /// **Copy one rectangle of a surface onto the screen.** `read(x, y)` is the surface's pixel in
    /// the tree's usual `0x00RRGGBB` spelling; `write(offset, word)` stores `word` at `offset`
    /// bytes from the screen's pixel (0, 0), already in the screen's byte order.
    ///
    /// Returns `false`, having written nothing, when the rectangle is empty or leaves the covered
    /// part: **refused rather than clamped**, the same stance the framebuffer contract takes
    /// (`graphics_protocol::rect_in_surface`), because a clamp would absorb a client's coordinate
    /// bug silently.
    pub fn copy(
        &self,
        x: u32,
        y: u32,
        w: u32,
        h: u32,
        read: impl Fn(u32, u32) -> u32,
        mut write: impl FnMut(usize, u32),
    ) -> bool {
        let fits = |start: u32, len: u32, limit: u32| {
            len > 0 && start.checked_add(len).is_some_and(|end| end <= limit)
        };
        if !fits(x, w, self.width) || !fits(y, h, self.height) {
            return false;
        }
        for row in y..y + h {
            let line = row as usize * self.stride as usize;
            for col in x..x + w {
                write(line + col as usize * 4, self.order.store(read(col, row)));
            }
        }
        true
    }

    /// **[`Self::copy`] with qword stores where the path permits them** (the paint lane,
    /// 2026-09-30). `write64(offset, pair)` receives two already-staged pixels in one `u64` and is
    /// called only at offsets where `(misalign + offset) % 8 == 0`; `write32` receives the head and
    /// tail pixels a row's alignment leaves over, exactly as [`Self::copy`] would. Every other
    /// word, including the byte order and the refusal stance, is [`Self::copy`]'s own.
    ///
    /// **Why the store width is the point.** The aperture a driver copies into is mapped
    /// uncacheable (x86: no PAT programming yet), and under QEMU's TCG store-heavy guest code runs
    /// ~3.8x slower per instruction than arithmetic
    /// (`notes/benchmarks/icount-tick-scales.md`), which made the one-word-per-pixel copy most of
    /// the `x86_64` swish leg's 321 s. Pairing the pixels halves the store count and nothing else
    /// changes. A `u64` is also the widest store this tree's `x86_64` target can legalise: the
    /// kernel and initrd build for `x86_64-unknown-none`, whose spec is `-mmx,-sse,+soft-float`,
    /// so there is no vector store to reach for (`xtask/src/archive.rs` records the target).
    ///
    /// `misalign` is the aperture's own base misalignment in bytes: the offset of pixel (0, 0)
    /// from an 8-aligned address, which only the driver that holds the mapping can know. With it,
    /// every geometry still takes its widest path: a padded or odd stride merely earns head and
    /// tail words on some rows.
    ///
    /// **The pair is byte-identical to two word stores on a little-endian machine**, which all
    /// three of this tree's targets are, and the host test that proves `copy_wide` equals `copy`
    /// pixel for pixel runs on one. `lo` is the earlier pixel.
    #[allow(clippy::too_many_arguments)] // the argument list is the copy contract: a rectangle,
    // the base's misalignment, a read, and the two store widths the caller's hardware offers
    pub fn copy_wide(
        &self,
        x: u32,
        y: u32,
        w: u32,
        h: u32,
        misalign: u32,
        read: impl Fn(u32, u32) -> u32,
        mut write64: impl FnMut(usize, u64),
        mut write32: impl FnMut(usize, u32),
    ) -> bool {
        let fits = |start: u32, len: u32, limit: u32| {
            len > 0 && start.checked_add(len).is_some_and(|end| end <= limit)
        };
        if !fits(x, w, self.width) || !fits(y, h, self.height) {
            return false;
        }
        for row in y..y + h {
            let line = row as usize * self.stride as usize;
            // The run's byte extent. A qword has two homes to satisfy at once: it must cover
            // exactly two pixels (`at` a multiple of 4 from `line`), and the store must be
            // 8-aligned where the pixels live (`(misalign + at) % 8 == 0`). Both are homes for
            // `at = line + 4k` exactly when `(misalign + line) % 4 == 0`, and then k's parity is
            // forced; a stride that is not itself 4-aligned (no framebuffer this tree has booted
            // reports one, but `checked` does not refuse it) has no such `at`, and that row takes
            // word stores alone.
            let run = line + x as usize * 4..line + (x + w) as usize * 4;
            let mis = misalign as usize;
            let qw = if (mis + line).is_multiple_of(4) && run.len() >= 8 {
                run.start
                    + if (mis + run.start).is_multiple_of(8) {
                        0
                    } else {
                        4
                    }
            } else {
                run.end
            };
            // Head words up to the first aligned pair (at most one: `qw` is 0 or 4 past the
            // start, or the whole run when the row has no aligned pair).
            for at in (run.start..qw.min(run.end)).step_by(4) {
                let col = (at - line) as u32 / 4;
                write32(at, self.order.store(read(col, row)));
            }
            // Staged pairs: two reads from cacheable RAM, one qword store into the aperture.
            let pairs = run.end.saturating_sub(qw) / 8;
            for p in 0..pairs {
                let at = qw + p * 8;
                let col = (at - line) as u32 / 4;
                let lo = self.order.store(read(col, row)) as u64;
                let hi = self.order.store(read(col + 1, row)) as u64;
                write64(at, lo | hi << 32);
            }
            // Tail words after the last whole pair.
            for at in (qw + pairs * 8..run.end).step_by(4) {
                let col = (at - line) as u32 / 4;
                write32(at, self.order.store(read(col, row)));
            }
        }
        true
    }
}

/// A cursor on a screen, and the arithmetic that puts a byte under it.
///
/// It holds no picture. The framebuffer is the only storage, and it is passed in on every call
/// rather than held, so that this type is a plain value a caller can put in a `static` without a
/// pointer to device memory living inside it.
#[derive(Clone, Copy, Debug)]
pub struct ScreenConsole {
    screen: Framebuffer,
    cols: u32,
    rows: u32,
    col: u32,
    row: u32,
    /// How many screen pixels each font pixel becomes, in each direction. See [`Self::scale_for`].
    scale: u32,
}

impl ScreenConsole {
    /// Ink. A light grey rather than white: white on black at a 1:1 pixel scale is glare on a real
    /// monitor, and every other console in this tree is a terminal's default rather than maximum
    /// contrast.
    pub const FOREGROUND: u32 = 0x00c8_c8c8;

    /// Paper. Black, because the alternative is repainting a firmware logo one cell at a time and
    /// getting a boot tour written over a splash screen.
    pub const BACKGROUND: u32 = 0x0000_0000;

    /// A console over `screen`, or `None` if it cannot hold a single character cell.
    ///
    /// The geometry is validated by [`Framebuffer::span`], which is the same check the loader
    /// applied before writing the description down. Doing it twice is deliberate: this crate is
    /// handed a description that came across a boot handoff, and the cost of believing a bad one is
    /// a kernel writing over memory it does not own.
    #[must_use]
    pub fn new(screen: Framebuffer) -> Option<Self> {
        screen.span()?;
        let scale = Self::scale_for(screen.width);
        let cols = screen.width / (bitmap_font::GLYPH_W * scale);
        let rows = screen.height / (bitmap_font::GLYPH_H * scale);
        if cols == 0 || rows == 0 {
            return None;
        }
        Some(Self {
            screen,
            cols,
            rows,
            col: 0,
            row: 0,
            scale,
        })
    }

    /// **The integer scale a screen `width` pixels wide is drawn at**: the largest that still
    /// leaves [`MIN_COLUMNS`] columns, never less than one. Each font pixel becomes a
    /// scale-by-scale block.
    ///
    /// The font's cell is 7 pixels wide, so a 1920x1080 monitor drew 274 columns of 8-pixel-tall
    /// glyphs at one, and on xenon's bench on 2026-10-04 calef could neither read nor photograph
    /// them. At two it draws 14x16 cells, 137 columns by 67 rows. Anything under 1680 wide
    /// (QEMU's 1280x800 OVMF screen among them) stays at one, as before.
    #[must_use]
    pub const fn scale_for(width: u32) -> u32 {
        let s = width / (bitmap_font::GLYPH_W * MIN_COLUMNS);
        if s == 0 { 1 } else { s }
    }

    /// The grid, in character cells: `(columns, rows)`.
    #[must_use]
    pub const fn size(&self) -> (u32, u32) {
        (self.cols, self.rows)
    }

    /// The screen this console paints, as the boot handoff described it.
    ///
    /// For the one caller that gives the screen away (the kernel's console, when a userspace
    /// terminal takes it over): what it hands on is the description it was itself given, so the
    /// driver that paints next and the console that painted before cannot disagree about the
    /// geometry. **Name provisional** (the shell on the firmware screen).
    #[must_use]
    pub const fn screen(&self) -> Framebuffer {
        self.screen
    }

    /// How many bytes of framebuffer this console addresses, which is what a caller has to map and
    /// how long the slice it passes to [`Self::write`] must be.
    #[must_use]
    pub fn span(&self) -> usize {
        self.screen.span().unwrap_or(0)
    }

    /// Paint the whole surface in [`Self::BACKGROUND`] and put the cursor at the top left.
    ///
    /// Called once when a console is armed, because whatever the firmware left on the screen is not
    /// this kernel's and text drawn over a logo is text nobody can read.
    pub fn clear(&mut self, pixels: &mut [u8]) {
        let paper = self.screen.order.store(Self::BACKGROUND).to_le_bytes();
        for pixel in pixels.as_chunks_mut::<4>().0 {
            pixel.copy_from_slice(&paper);
        }
        self.col = 0;
        self.row = 0;
    }

    /// Write `text` at the cursor, advancing it.
    ///
    /// `pixels` is the framebuffer, and a slice shorter than [`Self::span`] simply truncates the
    /// picture rather than panicking: this is the code that runs when something else has already
    /// gone wrong, and a bounds panic inside the console would take the message with it.
    pub fn write(&mut self, pixels: &mut [u8], text: &str) {
        for byte in text.bytes() {
            self.put(pixels, byte);
        }
    }

    /// One byte.
    fn put(&mut self, pixels: &mut [u8], byte: u8) {
        match byte {
            b'\n' => {
                self.col = 0;
                self.newline(pixels);
                return;
            }
            b'\r' => {
                self.col = 0;
                return;
            }
            _ => {}
        }
        if self.col >= self.cols {
            self.col = 0;
            self.newline(pixels);
        }
        // A byte with no glyph is drawn as a space rather than dropped, so that a run of them still
        // occupies the columns it occupies on the UART and the two transcripts line up.
        let glyph = if (0x20..0x7f).contains(&byte) {
            byte as char
        } else {
            ' '
        };
        self.draw(pixels, glyph);
        self.col += 1;
    }

    /// Move to the next row, scrolling when there is not one.
    fn newline(&mut self, pixels: &mut [u8]) {
        if self.row + 1 < self.rows {
            self.row += 1;
        } else {
            self.scroll(pixels);
        }
    }

    /// Move the picture up by one row of cells and blank the row that opens at the bottom.
    ///
    /// See this crate's `BUGS`: the read half of this is the expensive half on real silicon.
    fn scroll(&mut self, pixels: &mut [u8]) {
        let stride = self.screen.stride as usize;
        let band = stride * (bitmap_font::GLYPH_H * self.scale) as usize;
        let live = stride * (self.rows * bitmap_font::GLYPH_H * self.scale) as usize;
        if live > pixels.len() || band >= live {
            return;
        }
        pixels.copy_within(band..live, 0);
        let paper = self.screen.order.store(Self::BACKGROUND).to_le_bytes();
        for pixel in pixels[live - band..live].as_chunks_mut::<4>().0 {
            pixel.copy_from_slice(&paper);
        }
    }

    /// Paint one glyph at the cursor.
    fn draw(&self, pixels: &mut [u8], glyph: char) {
        let stride = self.screen.stride as usize;
        let k = self.scale as usize;
        let left = (self.col * bitmap_font::GLYPH_W) as usize * k * 4;
        let top = (self.row * bitmap_font::GLYPH_H) as usize * k;
        // Each font pixel becomes a k-by-k block: `y`/`x` walk the screen, `/ k` finds the font
        // pixel under them.
        for y in 0..bitmap_font::GLYPH_H as usize * k {
            let row = (top + y) * stride + left;
            for x in 0..bitmap_font::GLYPH_W as usize * k {
                let at = row + x * 4;
                let Some(pixel) = pixels.get_mut(at..at + 4) else {
                    return;
                };
                let colour = bitmap_font::cell_pixel(
                    glyph,
                    (x / k) as u32,
                    (y / k) as u32,
                    Self::FOREGROUND,
                    Self::BACKGROUND,
                );
                pixel.copy_from_slice(&self.screen.order.store(colour).to_le_bytes());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use machine_discovery::framebuffer::{Framebuffer, PixelOrder};

    use super::ScreenConsole;

    /// A screen with a **padded stride**, which is the geometry that catches the classic
    /// framebuffer bug: a painter that multiplies by the width instead of the stride paints a
    /// picture that shears progressively down the screen, and it looks perfectly correct on the
    /// first row.
    fn screen(cols: u32, rows: u32, pad: u32, order: PixelOrder) -> (Framebuffer, [u8; 4096 * 16]) {
        let width = cols * bitmap_font::GLYPH_W;
        let height = rows * bitmap_font::GLYPH_H;
        let found = Framebuffer {
            base: 0,
            width,
            height,
            stride: width * 4 + pad,
            order,
        };
        assert!(found.span().expect("a valid geometry") <= 4096 * 16);
        (found, [0u8; 4096 * 16])
    }

    /// Read one pixel back as a `u32`, the way a screendump would.
    fn pixel(found: &Framebuffer, pixels: &[u8], x: u32, y: u32) -> u32 {
        let at = (y * found.stride + x * 4) as usize;
        u32::from_le_bytes(pixels[at..at + 4].try_into().expect("four bytes"))
    }

    /// The whole crate is a pure function from text to pixels, so the expected picture is
    /// computable and can be *printed*. This is the same property `bitmap_font` claims for itself
    /// and it is the reason a console nobody can see is nonetheless provable.
    #[test]
    fn a_letter_is_drawn_the_right_way_up_and_the_right_way_round() {
        let (found, mut pixels) = screen(4, 2, 12, PixelOrder::Bgrx);
        let mut console = ScreenConsole::new(found).expect("four by two cells");
        console.write(&mut pixels, "F");

        let art: [[char; 7]; 8] = core::array::from_fn(|y| {
            core::array::from_fn(|x| {
                if pixel(&found, &pixels, x as u32, y as u32) == ScreenConsole::FOREGROUND {
                    '#'
                } else {
                    '.'
                }
            })
        });
        assert_eq!(
            art,
            [
                ['.', '#', '#', '#', '#', '#', '.'], // the top bar
                ['.', '#', '.', '.', '.', '.', '.'], // the stem is on the LEFT, which is what a
                ['.', '#', '.', '.', '.', '.', '.'], // mirrored font gets wrong
                ['.', '#', '#', '#', '#', '.', '.'], // one column short: that is what makes it an F
                ['.', '#', '.', '.', '.', '.', '.'],
                ['.', '#', '.', '.', '.', '.', '.'],
                ['.', '#', '.', '.', '.', '.', '.'],
                ['.', '.', '.', '.', '.', '.', '.'], // row 7 is the descender row, and F has none
            ]
        );
    }

    /// A 1920-wide screen (xenon's) draws at two: 137 columns, and every font pixel a 2x2 block.
    /// Narrow screens, which every other test here uses, stay at one.
    #[test]
    fn a_wide_screen_draws_each_font_pixel_as_a_block() {
        assert_eq!(ScreenConsole::scale_for(1280), 1);
        assert_eq!(ScreenConsole::scale_for(1679), 1);
        assert_eq!(ScreenConsole::scale_for(1680), 2);
        assert_eq!(ScreenConsole::scale_for(1920), 2);
        assert_eq!(ScreenConsole::scale_for(2520), 3);
        let found = Framebuffer {
            base: 0,
            width: 1920,
            height: 32,
            stride: 1920 * 4,
            order: PixelOrder::Bgrx,
        };
        let mut pixels = [0u8; 1920 * 4 * 32];
        assert_eq!(found.span(), Some(pixels.len()));
        let mut console = ScreenConsole::new(found).expect("a wide screen");
        assert_eq!(console.size(), (137, 2));
        console.write(&mut pixels, &"F".repeat(138));
        for (col, row) in [(0, 0), (0, 1)] {
            for y in 0..16 {
                for x in 0..14 {
                    let want = bitmap_font::cell_pixel(
                        'F',
                        x / 2,
                        y / 2,
                        ScreenConsole::FOREGROUND,
                        ScreenConsole::BACKGROUND,
                    );
                    assert_eq!(
                        pixel(&found, &pixels, col * 14 + x, row * 16 + y),
                        want,
                        "cell ({col},{row}) pixel ({x},{y}); the 138th F must wrap to row 1"
                    );
                }
            }
        }
    }

    /// The stride is padding, and a painter that ignores it writes the second row seven pixels to
    /// the left of where it belongs. The pad bytes must be untouched.
    #[test]
    fn the_padding_between_rows_is_never_written() {
        const PAD: u32 = 12;
        let (found, mut pixels) = screen(4, 2, PAD, PixelOrder::Bgrx);
        let mut console = ScreenConsole::new(found).expect("four by two cells");
        console.clear(&mut pixels);
        // Poison the padding, then paint over the whole grid and check it survived.
        let visible = (found.width * 4) as usize;
        for y in 0..found.height as usize {
            let at = y * found.stride as usize + visible;
            pixels[at..at + PAD as usize].fill(0xa5);
        }
        console.write(&mut pixels, "MMMM\nMMMM");
        for y in 0..found.height as usize {
            let at = y * found.stride as usize + visible;
            assert_eq!(
                &pixels[at..at + PAD as usize],
                &[0xa5; PAD as usize],
                "row {y}'s padding was painted over"
            );
        }
    }

    /// Running past the last column wraps rather than painting outside the screen, and running past
    /// the last row scrolls. Both are checked by where a known glyph ends up.
    #[test]
    fn text_wraps_at_the_right_edge_and_scrolls_at_the_bottom() {
        let (found, mut pixels) = screen(2, 2, 0, PixelOrder::Bgrx);
        let mut console = ScreenConsole::new(found).expect("two by two cells");
        assert_eq!(console.size(), (2, 2));
        console.clear(&mut pixels);

        // Five characters into a 2x2 grid: "ab" wraps to "cd", then "e" scrolls and lands at (0,1).
        console.write(&mut pixels, "abcde");
        let cell = |pixels: &[u8], col: u32, row: u32, ch: char| {
            (0..bitmap_font::GLYPH_H).all(|y| {
                (0..bitmap_font::GLYPH_W).all(|x| {
                    let want = bitmap_font::cell_pixel(
                        ch,
                        x,
                        y,
                        ScreenConsole::FOREGROUND,
                        ScreenConsole::BACKGROUND,
                    );
                    pixel(
                        &found,
                        pixels,
                        col * bitmap_font::GLYPH_W + x,
                        row * bitmap_font::GLYPH_H + y,
                    ) == want
                })
            })
        };
        assert!(cell(&pixels, 0, 0, 'c'), "the second line scrolled up");
        assert!(cell(&pixels, 1, 0, 'd'));
        assert!(cell(&pixels, 0, 1, 'e'), "the fifth character opened a row");
        assert!(cell(&pixels, 1, 1, ' '), "the rest of the new row is blank");
    }

    /// `ScreenConsole::span` had no caller at all: every test either uses `Framebuffer::span`
    /// directly (`screen`'s own assertion) or `Aperture::span` (a different method entirely).
    #[test]
    fn console_span_is_the_underlying_screens_span() {
        let (found, _pixels) = screen(4, 2, 12, PixelOrder::Bgrx);
        let console = ScreenConsole::new(found).expect("four by two cells");
        assert_eq!(console.span(), found.span().unwrap());
    }

    /// Every other test's `clear()` call is immediately followed by writing text that covers the
    /// whole grid, so nothing ever looked at the screen right after a clear and before a write,
    /// which is the one moment a stubbed-out `clear` and the real one look different.
    #[test]
    fn clear_paints_the_whole_surface_in_the_background_colour() {
        let (found, mut pixels) = screen(2, 2, 0, PixelOrder::Bgrx);
        pixels.fill(0xa5); // poison: a no-op clear would leave this untouched
        let mut console = ScreenConsole::new(found).expect("two by two cells");
        console.clear(&mut pixels);
        for y in 0..found.height {
            for x in 0..found.width {
                assert_eq!(
                    pixel(&found, &pixels, x, y),
                    ScreenConsole::BACKGROUND,
                    "({x},{y})"
                );
            }
        }
    }

    /// A carriage return resets the column without drawing anything, which is the one control
    /// character `text_wraps_at_the_right_edge_and_scrolls_at_the_bottom` never sends.
    #[test]
    fn a_carriage_return_resets_the_column_without_drawing_a_glyph() {
        let (found, mut pixels) = screen(3, 1, 0, PixelOrder::Bgrx);
        let mut console = ScreenConsole::new(found).expect("three by one cells");
        console.clear(&mut pixels);
        console.write(&mut pixels, "a\rb");
        let cell = |pixels: &[u8], col: u32, ch: char| {
            (0..bitmap_font::GLYPH_H).all(|y| {
                (0..bitmap_font::GLYPH_W).all(|x| {
                    let want = bitmap_font::cell_pixel(
                        ch,
                        x,
                        y,
                        ScreenConsole::FOREGROUND,
                        ScreenConsole::BACKGROUND,
                    );
                    pixel(&found, pixels, col * bitmap_font::GLYPH_W + x, y) == want
                })
            })
        };
        assert!(cell(&pixels, 0, 'b'), "'b' overwrote 'a' at column 0");
        assert!(
            cell(&pixels, 1, ' '),
            "the carriage return must not have advanced the column or drawn anything"
        );
    }

    /// A one-row console has nowhere to scroll to: `scroll`'s `band >= live` guard exists for
    /// exactly this case, and no existing test has a console with only one row (every `screen()`
    /// fixture that reaches `scroll` uses at least two).
    #[test]
    fn scrolling_a_one_row_console_leaves_it_alone_rather_than_blanking_it() {
        let (found, mut pixels) = screen(3, 1, 0, PixelOrder::Bgrx);
        let mut console = ScreenConsole::new(found).expect("three by one cells");
        console.clear(&mut pixels);
        console.write(&mut pixels, "abcd"); // the fourth character wraps and tries to scroll
        let cell = |pixels: &[u8], col: u32, ch: char| {
            (0..bitmap_font::GLYPH_H).all(|y| {
                (0..bitmap_font::GLYPH_W).all(|x| {
                    let want = bitmap_font::cell_pixel(
                        ch,
                        x,
                        y,
                        ScreenConsole::FOREGROUND,
                        ScreenConsole::BACKGROUND,
                    );
                    pixel(&found, pixels, col * bitmap_font::GLYPH_W + x, y) == want
                })
            })
        };
        assert!(
            cell(&pixels, 0, 'd'),
            "'d' wrapped back to column 0, overwriting 'a'"
        );
        assert!(
            cell(&pixels, 1, 'b'),
            "a one-row console has nowhere to scroll; 'b' must survive"
        );
        assert!(cell(&pixels, 2, 'c'), "same for 'c'");
    }

    /// `scroll`'s `live > pixels.len()` guard, at the one point a `>` and an `==`/`>=` disagree: a
    /// buffer exactly as long as the live region must still be scrolled, not refused. Every
    /// `screen()` fixture's buffer (4096 * 16 bytes) is far larger than any `live` this crate's
    /// tests compute, so this boundary has never been reached before.
    #[test]
    fn scroll_accepts_a_buffer_that_is_exactly_as_long_as_the_live_region() {
        // A two-by-two `screen(2, 2, 0, ..)` geometry, with its `live` region as a compile-time
        // constant so the buffer below can be sized to match it exactly (this crate is
        // unconditionally `no_std`, so no `Vec` to size at runtime).
        const STRIDE: usize = (2 * bitmap_font::GLYPH_W * 4) as usize;
        const LIVE: usize = STRIDE * (2 * bitmap_font::GLYPH_H) as usize;
        let (found, _big) = screen(2, 2, 0, PixelOrder::Bgrx);
        assert_eq!(
            found.stride as usize, STRIDE,
            "the constant must match the fixture"
        );
        let mut console = ScreenConsole::new(found).expect("two by two cells");
        let mut exact = [0xa5u8; LIVE];
        console.scroll(&mut exact);
        assert_ne!(
            exact, [0xa5u8; LIVE],
            "a buffer exactly as long as the live region must still be scrolled"
        );
    }

    /// The byte order is the one thing that cannot be seen by looking at the screen in a test, and
    /// is the one thing that makes the picture wrong on half the machines that could run this.
    #[test]
    fn the_two_pixel_orders_store_different_bytes_for_the_same_ink() {
        let (bgrx, mut a) = screen(1, 1, 0, PixelOrder::Bgrx);
        let (rgbx, mut b) = screen(1, 1, 0, PixelOrder::Rgbx);
        ScreenConsole::new(bgrx)
            .expect("one cell")
            .write(&mut a, "#");
        ScreenConsole::new(rgbx)
            .expect("one cell")
            .write(&mut b, "#");
        // FOREGROUND is grey, so its red and blue bytes are equal and the two orders agree. That is
        // exactly why the check below uses a colour whose channels differ.
        assert_eq!(
            PixelOrder::Bgrx.store(0x0011_2233),
            0x0011_2233,
            "bgrx stores a colour as written"
        );
        assert_eq!(
            PixelOrder::Rgbx.store(0x0011_2233),
            0x0033_2211,
            "rgbx exchanges red and blue"
        );
        assert_eq!(a, b, "grey ink is the same bytes in both orders");
    }

    /// A screen too small for one character cell has no console, rather than a console that writes
    /// outside it.
    #[test]
    fn a_screen_smaller_than_one_cell_has_no_console() {
        for (width, height) in [(6, 8), (7, 7), (0, 0)] {
            let found = Framebuffer {
                base: 0,
                width,
                height,
                stride: width * 4,
                order: PixelOrder::Bgrx,
            };
            assert!(ScreenConsole::new(found).is_none(), "{width}x{height}");
        }
    }

    /// **The aperture copies through the stride, not the width**, and leaves the padding alone.
    /// A copy that multiplied by the width would put row 1 twelve bytes early, which is the shear
    /// `the_padding_between_rows_is_never_written` guards against in the text painter.
    #[test]
    fn an_aperture_copies_through_the_stride_and_never_touches_the_padding() {
        use super::Aperture;
        const PAD: u32 = 12;
        let (found, mut pixels) = screen(2, 1, PAD, PixelOrder::Bgrx);
        pixels.fill(0xa5);
        let aperture = Aperture::new(&found, 1000, 1000).expect("a real screen");
        assert_eq!(
            aperture.size(),
            (found.width, found.height),
            "clipped to the screen"
        );
        let (w, h) = aperture.size();
        // Every surface pixel a function of its coordinate, so a misplaced pixel is a wrong value.
        let surface = |x: u32, y: u32| (y << 8) | x;
        assert!(aperture.copy(0, 0, w, h, surface, |at, word| {
            pixels[at..at + 4].copy_from_slice(&word.to_le_bytes());
        }));
        for y in 0..h {
            for x in 0..w {
                assert_eq!(pixel(&found, &pixels, x, y), surface(x, y), "({x},{y})");
            }
            let pad = (y * found.stride + w * 4) as usize;
            assert_eq!(
                &pixels[pad..pad + PAD as usize],
                &[0xa5; PAD as usize],
                "row {y}"
            );
        }
        assert_eq!(aperture.span(), ((h - 1) * found.stride + w * 4) as usize);
    }

    /// Clipping takes the smaller of the two sizes on each axis independently, and a rectangle
    /// that leaves the clipped part, or is empty, or overflows `u32`, is refused with nothing
    /// written.
    #[test]
    fn an_aperture_is_the_smaller_of_screen_and_surface_and_refuses_what_leaves_it() {
        use super::Aperture;
        let found = Framebuffer {
            base: 0,
            width: 1280,
            height: 800,
            stride: 1280 * 4,
            order: PixelOrder::Bgrx,
        };
        let aperture = Aperture::new(&found, 924, 344).expect("a real screen");
        assert_eq!(
            aperture.size(),
            (924, 344),
            "the surface is smaller: it wins"
        );
        let narrow = Framebuffer {
            width: 800,
            stride: 800 * 4,
            ..found
        };
        assert_eq!(
            Aperture::new(&narrow, 924, 344)
                .expect("a real screen")
                .size(),
            (800, 344),
            "a narrow screen clips the surface's width and not its height"
        );
        for (x, y, w, h) in [
            (0, 0, 925, 1),
            (0, 0, 1, 345),
            (923, 0, 2, 1),
            (0, 0, 0, 1),
            (0, 0, 1, 0),
            (u32::MAX, 0, 2, 1),
        ] {
            assert!(
                !aperture.copy(
                    x,
                    y,
                    w,
                    h,
                    |_, _| 0,
                    |_, _| panic!("wrote for ({x},{y},{w},{h})")
                ),
                "({x},{y},{w},{h}) should have been refused"
            );
        }
        assert!(Aperture::new(&found, 0, 344).is_none(), "an empty surface");
    }

    /// The words that carry an aperture across a spawn come back as the same aperture, both byte
    /// orders survive, and words describing an impossible geometry are refused rather than
    /// believed.
    #[test]
    fn an_aperture_survives_the_trip_through_two_words() {
        use super::Aperture;
        for order in [PixelOrder::Bgrx, PixelOrder::Rgbx] {
            let found = Framebuffer {
                base: 0,
                width: 1920,
                height: 1080,
                stride: 7680,
                order,
            };
            let aperture = Aperture::new(&found, 924, 344).expect("a real screen");
            let (size, layout) = aperture.to_words();
            assert_eq!(Aperture::from_words(size, layout), Some(aperture));
        }
        // A stride narrower than the row, and an order this tree has no name for.
        assert_eq!(Aperture::from_words(10 | 10 << 32, 39), None);
        assert_eq!(Aperture::from_words(10 | 10 << 32, 40 | 2 << 32), None);
        assert_eq!(Aperture::from_words(0, 40), None);
    }

    /// `copy_wide` lands the same bytes as `copy` for every base misalignment, on a stride with
    /// padding that is not itself 8-aligned. This is the whole correctness claim of the wide path
    /// (a pair is two word stores in one, on a little-endian machine), checked byte for byte
    /// including the padding, which neither path may touch.
    #[test]
    fn the_wide_copy_lands_the_same_bytes_for_every_rectangle_and_base_alignment() {
        use super::Aperture;
        // 10 pixels wide (40 bytes), rows of 44: padding of 4, and a stride that is 4 off 8, so
        // some rows' qwords start at head words and stop at tails.
        let found = Framebuffer {
            base: 0,
            width: 10,
            height: 3,
            stride: 44,
            order: PixelOrder::Bgrx,
        };
        let aperture = Aperture::new(&found, 10, 3).expect("a real screen");
        let (w, h) = aperture.size();
        // The surface refuses a read outside it, as `MappedWindow` does in the driver: a column
        // counted from the wrong origin is a panic here, not a quietly wrong pixel.
        let surface = |x: u32, y: u32| {
            assert!(x < w && y < h, "read ({x},{y}) outside the {w}x{h} surface");
            (y << 12) | (x << 8) | 0x0f
        };
        const SPAN: usize = 44 * 2 + 40;
        // **Every rectangle, not only the whole one.** A run that starts at `x > 0` is what a
        // cell's damage flushes; until 2026-10-02 every case here started at column 0, which is
        // the one origin where counting `x` twice (the run's offset already holds it) is
        // harmless. The driver read past the surface on the x86_64 swish leg instead.
        for (x, y) in (0..w).flat_map(|x| (0..h).map(move |y| (x, y))) {
            for (rw, rh) in (1..=w - x).flat_map(|rw| (1..=h - y).map(move |rh| (rw, rh))) {
                let mut narrow = [0xa5u8; SPAN];
                assert!(aperture.copy(x, y, rw, rh, surface, |at, word| {
                    narrow[at..at + 4].copy_from_slice(&word.to_le_bytes());
                }));
                for misalign in 0..8u32 {
                    // RefCell because the two writer closures must share one buffer; a plain
                    // `&mut` split across them is what the borrow checker rightly refuses.
                    let wide = core::cell::RefCell::new([0xa5u8; SPAN]);
                    assert!(aperture.copy_wide(
                        x,
                        y,
                        rw,
                        rh,
                        misalign,
                        surface,
                        |at, pair| {
                            wide.borrow_mut()[at..at + 8].copy_from_slice(&pair.to_le_bytes());
                        },
                        |at, word| {
                            wide.borrow_mut()[at..at + 4].copy_from_slice(&word.to_le_bytes());
                        }
                    ));
                    assert_eq!(
                        narrow,
                        wide.into_inner(),
                        "({x},{y}) {rw}x{rh}, misalign {misalign}"
                    );
                }
            }
        }
    }

    /// **What the wide path permits, as a store count.** The scanout's own geometry (924x344,
    /// stride 3696, an even width on an 8-aligned stride) is the best case and the one the swish
    /// leg pays: one qword store per pixel pair, no head or tail words, 158,928 stores for a
    /// full-surface flush against 317,856 one-word stores. A run that starts misaligned earns at
    /// most one head word and one tail word, never a narrower pair.
    #[test]
    fn the_wide_copy_uses_one_qword_per_pair_and_never_more_than_two_edge_words() {
        use graphics_protocol as gfx;

        use super::Aperture;
        let found = Framebuffer {
            base: 0,
            width: gfx::WIDTH,
            height: gfx::HEIGHT,
            stride: gfx::STRIDE,
            order: PixelOrder::Bgrx,
        };
        let aperture = Aperture::new(&found, gfx::WIDTH, gfx::HEIGHT).expect("the scanout");
        let (w, h) = aperture.size();
        let mut qwords = 0u32;
        let mut words = 0u32;
        assert!(aperture.copy_wide(
            0,
            0,
            w,
            h,
            0,
            |_, _| 0,
            |_, _| qwords += 1,
            |_, _| words += 1,
        ));
        assert_eq!(words, 0, "an even width on an aligned stride has no edges");
        assert_eq!(
            qwords,
            (gfx::WIDTH / 2) * gfx::HEIGHT,
            "one qword per pixel pair, whole surface"
        );

        // Five pixels on a 4-off stride, base misaligned by 4: one head word, two pairs, no tail.
        let odd = Framebuffer {
            base: 0,
            width: 5,
            height: 1,
            stride: 24,
            order: PixelOrder::Bgrx,
        };
        let aperture = Aperture::new(&odd, 5, 1).expect("a real screen");
        let mut qwords = 0u32;
        let mut words = 0u32;
        assert!(aperture.copy_wide(
            0,
            0,
            5,
            1,
            4,
            |_, _| 0,
            |_, _| qwords += 1,
            |_, _| words += 1,
        ));
        assert_eq!((qwords, words), (2, 1), "head word, then whole pairs");
    }

    /// The two pixel orders survive the pairing: a qword's low half is the earlier pixel in the
    /// screen's byte order, which is the half a big-endian packing or a swapped pair would get
    /// wrong and a grey test pattern cannot see.
    #[test]
    fn a_staged_pair_keeps_the_earlier_pixel_low_in_the_screens_byte_order() {
        use super::Aperture;
        let found = Framebuffer {
            base: 0,
            width: 2,
            height: 1,
            stride: 8,
            order: PixelOrder::Rgbx,
        };
        let aperture = Aperture::new(&found, 2, 1).expect("a real screen");
        let mut seen = [0u8; 8];
        assert!(aperture.copy_wide(
            0,
            0,
            2,
            1,
            0,
            |x, _| 0x0011_0000 | x,
            |at, pair| {
                seen.copy_from_slice(&pair.to_le_bytes());
                assert_eq!(at, 0, "the pair sits at the run's start");
            },
            |_, _| panic!("two aligned pixels need no word store"),
        ));
        // Rgbx exchanges red and blue on the way in: pixel 0 (red set) stores as 0x0000_0011 and
        // pixel 1 (red and blue set) as 0x0001_0011, little-endian bytes below. The low half of
        // the qword is the earlier pixel.
        assert_eq!(
            seen,
            [0x11, 0x00, 0x00, 0x00, 0x11, 0x00, 0x01, 0x00],
            "low half is the earlier pixel, byte order applied per word"
        );
    }

    /// Both `|`s in `to_words` survive being mutated to `^`, and this is why: each word packs a
    /// `u32` in the low half and a value shifted left by 32 in the high half, and a left shift by
    /// 32 zeroes exactly the low 32 bits it would otherwise collide with. Checked at the widest
    /// each half can be; if the extremes do not collide, no narrower value can either.
    #[test]
    fn to_words_two_halves_share_no_bit_so_or_and_xor_agree() {
        assert_eq!(u32::MAX as u64 & (u32::MAX as u64) << 32, 0);
    }

    /// An rgbx screen gets its red and blue exchanged on the way in, which is the byte-order half
    /// of the copy and the half a grey test pattern cannot see.
    #[test]
    fn an_aperture_stores_in_the_screens_byte_order() {
        use super::Aperture;
        let (found, mut pixels) = screen(1, 1, 0, PixelOrder::Rgbx);
        let aperture = Aperture::new(&found, 1, 1).expect("one pixel");
        assert!(aperture.copy(
            0,
            0,
            1,
            1,
            |_, _| 0x0011_2233,
            |at, word| {
                pixels[at..at + 4].copy_from_slice(&word.to_le_bytes());
            }
        ));
        assert_eq!(
            &pixels[0..4],
            &[0x11, 0x22, 0x33, 0x00],
            "bytes R, G, B, unused"
        );
    }

    /// A framebuffer slice shorter than the geometry claims must truncate the picture, never panic.
    /// The console runs on the path where something else has already gone wrong.
    #[test]
    fn a_short_framebuffer_truncates_rather_than_panicking() {
        let (found, _) = screen(8, 8, 0, PixelOrder::Bgrx);
        let mut console = ScreenConsole::new(found).expect("eight by eight cells");
        let mut pixels = [0u8; 64];
        console.clear(&mut pixels);
        console.write(
            &mut pixels,
            "this is far more text than sixty-four bytes can hold\n",
        );
    }
}
