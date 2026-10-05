//! **Reading a screen the way [`super::watch`] reads a wire** (milestone 243).
//!
//! Milestone 216 gave a gate a way to read a board it cannot see: open the serial port, log the
//! bytes, judge how far the boot got. **Six of the machines in calef's house have no serial port**,
//! and on those nife's console is a framebuffer (`screen_console`). This module is the other half
//! of the same tool: it turns a *picture* of that framebuffer back into the text that was written
//! into it, so that everything in [`super::progress`] applies unchanged.
//!
//! # Why this is possible at all, and is not optical character recognition
//!
//! [`bitmap_font`] is a constant table of monochrome 7x8 glyphs, so the picture a character
//! produces is a pure function of the character. Reading it back is therefore an exact table
//! lookup rather than a guess: **a cell either matches a glyph bit for bit or it matches nothing**,
//! and there is no threshold to tune and no confidence to report. That property is the same one
//! `bitmap_font`'s own documentation claims for the writing direction, used backwards.
//!
//! It also means this is worthless on any screen nife did not draw. A firmware splash decodes to
//! nothing, which is the correct answer.
//!
//! # What it is for
//!
//! Two things, and the second is the one that could not be done before.
//!
//! - Under QEMU, `cargo xtask uefi-boot` asks the emulator for a screendump and reads the tour off
//!   it. That gates the whole framebuffer path on every run: the loader's `LocateProtocol`, the
//!   byte order, the stride, the mapping surviving `mmu::init`, and the glyphs.
//! - On a machine nobody can attach a cable to, a **photograph of the screen** is not this: it is
//!   not pixel-aligned and it is not a screendump. Turning one into text is a different problem
//!   this module does not solve, and `notes/serial-less-output.md` says so where a reader meets it.
//!
//! # Examples
//!
//! ```
//! use board_console::screen;
//!
//! // A two-by-one grid of 7x8 cells, in the PPM QEMU's `screendump` writes.
//! let mut ppm = b"P6\n14 8\n255\n".to_vec();
//! for y in 0..8 {
//!     for x in 0..14 {
//!         let ink = bitmap_font::is_ink(if x < 7 { 'h' } else { 'i' }, x % 7, y);
//!         ppm.extend_from_slice(if ink { &[0xc8, 0xc8, 0xc8] } else { &[0, 0, 0] });
//!     }
//! }
//!
//! assert_eq!(screen::read(&ppm).unwrap(), "hi\n");
//! ```
//!
//! # BUGS
//!
//! - **Trailing blanks are trimmed and leading ones are not.** A framebuffer console pads every row
//!   to the full width with background, so keeping them would make every line 182 characters long
//!   and every substring search in [`super::progress`] still work but every printed transcript
//!   unreadable. Leading blanks are the indentation the boot tour actually uses.
//! - **A cell that matches no glyph reads as `?`**, not as an error. A screen is a picture and part
//!   of it may be something else entirely (a firmware logo the console did not clear over, half a
//!   character at the right edge); refusing the whole dump for one cell would throw away the
//!   ninety-nine percent that decoded.
//! - **Several characters share a glyph in a 7x8 font.** The lookup returns the first match in
//!   ASCII order, so a font with a duplicate would silently prefer the earlier character. Nothing
//!   currently checks `bitmap_font` for duplicate glyphs.
//! - **Only [`INK`] on [`PAPER`] is recognised.** Any other colour scheme decodes to blanks. The
//!   kernel console has exactly one and this reads it.

/// What `screen_console` paints ink with, as one 24-bit RGB pixel.
///
/// Duplicated from that crate rather than depended on, deliberately: this is a *host* tool reading
/// a picture, and a dependency edge from the gate to the kernel's console crate would make the two
/// unable to disagree, which is the one thing a gate is for. The value is asserted against the
/// crate's own constant in `xtask`, where both are already in scope.
pub const INK: [u8; 3] = [0xc8, 0xc8, 0xc8];

/// What it paints background with.
pub const PAPER: [u8; 3] = [0x00, 0x00, 0x00];

/// Anything at least this bright in all three channels is ink. A midpoint rather than an exact
/// match, so that a screendump that went through a colour conversion still reads.
const INK_THRESHOLD: u8 = 0x60;

/// What went wrong reading a dump. Small on purpose: there are only two ways a PPM can be
/// unreadable that are worth telling apart.
#[derive(Debug, PartialEq, Eq)]
pub enum ReadError {
    /// The bytes are not a binary (`P6`) portable pixmap, or its header is malformed.
    NotAPixmap,
    /// The header describes more pixels than the file holds.
    Truncated {
        /// What the header promised, in bytes of pixel data.
        wanted: usize,
        /// What was there.
        got: usize,
    },
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotAPixmap => write!(f, "not a binary P6 portable pixmap"),
            Self::Truncated { wanted, got } => {
                write!(
                    f,
                    "the pixmap promised {wanted} bytes of pixels and has {got}"
                )
            }
        }
    }
}

impl std::error::Error for ReadError {}

/// **Turn a screendump back into the text that was drawn into it.**
///
/// One line per row of character cells, newline-terminated, trailing blanks trimmed. Rows that are
/// entirely blank are kept, because a blank line in a boot tour is a blank line.
///
/// # Errors
///
/// [`ReadError`] when the bytes are not a readable binary PPM.
pub fn read(ppm: &[u8]) -> Result<String, ReadError> {
    let (width, height, pixels) = parse_pixmap(ppm)?;
    // The console draws a wide screen at an integer scale (`ScreenConsole::scale_for`, 2026-10-04),
    // and asking the painter's own rule is what keeps this reader from disagreeing with it.
    let k =
        screen_console::ScreenConsole::scale_for(u32::try_from(width).unwrap_or(u32::MAX)) as usize;
    let cols = width / (bitmap_font::GLYPH_W as usize * k);
    let rows = height / (bitmap_font::GLYPH_H as usize * k);
    let mut out = String::new();
    for row in 0..rows {
        let mut line = String::new();
        for col in 0..cols {
            line.push(cell(pixels, width, k, col, row));
        }
        out.push_str(line.trim_end());
        out.push('\n');
    }
    Ok(out)
}

/// The character in one cell, or `?` when the cell matches no glyph.
fn cell(pixels: &[u8], width: usize, k: usize, col: usize, row: usize) -> char {
    let left = col * bitmap_font::GLYPH_W as usize * k;
    let top = row * bitmap_font::GLYPH_H as usize * k;
    let mut seen = [[false; bitmap_font::GLYPH_W as usize]; bitmap_font::GLYPH_H as usize];
    for (y, line) in seen.iter_mut().enumerate() {
        for (x, ink) in line.iter_mut().enumerate() {
            // One sample per k-by-k block: its top-left screen pixel.
            let at = ((top + y * k) * width + left + x * k) * 3;
            *ink = pixels[at..at + 3].iter().all(|c| *c >= INK_THRESHOLD);
        }
    }
    // ASCII order, so `' '` wins any tie with another blank glyph.
    for candidate in 0x20u8..0x7f {
        let ch = candidate as char;
        let matches = seen.iter().enumerate().all(|(y, line)| {
            line.iter()
                .enumerate()
                .all(|(x, ink)| *ink == bitmap_font::is_ink(ch, x as u32, y as u32))
        });
        if matches {
            return ch;
        }
    }
    '?'
}

/// Header and pixel data of a binary portable pixmap: `(width, height, pixels)`.
///
/// PPM's header is whitespace-separated tokens with `#` comments, and exactly **one** whitespace
/// byte after the maximum value belongs to the header rather than to the pixels. Getting that one
/// byte wrong shifts the whole image by a third of a pixel, which looks like a font bug.
fn parse_pixmap(ppm: &[u8]) -> Result<(usize, usize, &[u8]), ReadError> {
    let token = |at: &mut usize| -> Option<usize> {
        loop {
            while ppm.get(*at).is_some_and(|b| b.is_ascii_whitespace()) {
                *at += 1;
            }
            if ppm.get(*at) == Some(&b'#') {
                while ppm.get(*at).is_some_and(|b| *b != b'\n') {
                    *at += 1;
                }
                continue;
            }
            break;
        }
        let start = *at;
        while ppm
            .get(*at)
            .is_some_and(|b| !b.is_ascii_whitespace() && *b != b'#')
        {
            *at += 1;
        }
        (start != *at).then_some(start)?;
        std::str::from_utf8(&ppm[start..*at]).ok()?.parse().ok()
    };

    if ppm.get(..2) != Some(b"P6") {
        return Err(ReadError::NotAPixmap);
    }
    // Past the magic; the three header numbers follow, whitespace-separated.
    let mut at = 2;
    let width = token(&mut at).ok_or(ReadError::NotAPixmap)?;
    let height = token(&mut at).ok_or(ReadError::NotAPixmap)?;
    let max = token(&mut at).ok_or(ReadError::NotAPixmap)?;
    if max != 255 || width == 0 || height == 0 {
        return Err(ReadError::NotAPixmap);
    }
    // Exactly one whitespace byte separates the header from the pixels.
    at += 1;
    let wanted = width * height * 3;
    let pixels = ppm.get(at..).unwrap_or(&[]);
    if pixels.len() < wanted {
        return Err(ReadError::Truncated {
            wanted,
            got: pixels.len(),
        });
    }
    Ok((width, height, &pixels[..wanted]))
}

#[cfg(test)]
mod tests {
    use machine_discovery::framebuffer::{Framebuffer, PixelOrder};
    use screen_console::{Cells, PixelSink, ScreenConsole};

    use super::{PAPER, ReadError, read};

    /// Paint `text` with the **kernel's own console** and hand back the screendump QEMU would have
    /// produced of it.
    ///
    /// This is what makes the test below worth having rather than a test of this file against
    /// itself: the writer is the crate the kernel links, so a change to either side that the other
    /// does not follow fails here, on the host, in milliseconds. It is the same rule the
    /// `uefi_loader` handoff tests are held to.
    fn painted(cols: u32, rows: u32, text: &str) -> Vec<u8> {
        painted_px(
            cols * bitmap_font::GLYPH_W,
            rows * bitmap_font::GLYPH_H,
            text,
        )
    }

    /// [`painted`] at a pixel size, for screens wide enough that the console scales its font.
    fn painted_px(width: u32, height: u32, text: &str) -> Vec<u8> {
        let screen = Framebuffer {
            base: 0,
            width,
            height,
            // A padded stride, because a decoder that ignores the difference between the two would
            // pass with the picture sheared and this test is the only thing that would notice.
            stride: width * 4 + 16,
            order: PixelOrder::Bgrx,
        };
        let mut pixels = vec![0u8; screen.span().expect("a valid geometry")];
        let mut console = ScreenConsole::new(screen).expect("a screen of whole cells");
        let mut cells = Box::new(Cells::new());
        console.clear(&mut cells, &mut PixelSink::new(&mut pixels));
        console.write(&mut cells, &mut PixelSink::new(&mut pixels), text);

        // QEMU's screendump is 24-bit RGB with no padding, so this is the conversion the emulator
        // does: drop the stride, drop the unused byte, and reorder to R, G, B.
        let mut ppm = format!("P6\n{} {}\n255\n", screen.width, screen.height).into_bytes();
        for y in 0..screen.height {
            for x in 0..screen.width {
                let at = (y * screen.stride + x * 4) as usize;
                let bgrx = &pixels[at..at + 4];
                ppm.extend_from_slice(&[bgrx[2], bgrx[1], bgrx[0]]);
            }
        }
        ppm
    }

    /// **The whole claim of this module, end to end**: what the kernel's console draws is what this
    /// reads back, exactly, with no screen and no emulator involved.
    #[test]
    fn what_the_kernel_console_draws_is_what_this_reads_back() {
        let tour = "nife on x86_64 (long mode, ring 0, 4-level paging)\n\
                      cpu 0 booted: high-half kernel, .bss, and the 16550 console are up.\n\
                    \n\
                    nife x86_64: boot complete, halting.\n";
        let decoded = read(&painted(80, 6, tour)).expect("a readable dump");
        for line in tour.lines() {
            assert!(
                decoded.contains(line),
                "{line:?} was drawn and did not come back\n--- decoded ---\n{decoded}"
            );
        }
    }

    /// xenon's 1920x1080 monitor, where the console draws every font pixel as a 2x2 block
    /// (`ScreenConsole::scale_for`, 2026-10-04). The reader must decode at the painter's scale, and
    /// a line longer than the 137 columns must come back wrapped, not cut.
    #[test]
    fn a_scaled_screen_reads_back_at_the_painters_scale() {
        assert_eq!(ScreenConsole::scale_for(1920), 2);
        let long = "disk-throughput: ".repeat(9);
        let text = format!("diag vt-d frcd0 sid 01:00.0 reason 0x01 read addr 0x9e1000\n{long}\n");
        let decoded = read(&painted_px(1920, 64, &text)).expect("a readable dump");
        let lines: Vec<&str> = decoded.lines().collect();
        assert_eq!(
            lines[0],
            "diag vt-d frcd0 sid 01:00.0 reason 0x01 read addr 0x9e1000"
        );
        assert_eq!(
            lines[1].len(),
            137,
            "the long line fills one row of 137 columns"
        );
        assert_eq!(format!("{}{}", lines[1], lines[2]), long.trim_end());
    }

    /// The point of decoding at all: [`crate::progress`] judges a screen exactly as it judges a
    /// wire, with nothing between them that knows which it is.
    #[test]
    fn a_decoded_screen_is_judged_by_the_same_recogniser_as_a_serial_log() {
        let banner = "nife on x86_64 (long mode, ring 0, 4-level paging)";
        let decoded = read(&painted(80, 4, &format!("{banner}\n"))).expect("a readable dump");

        let mut from_the_screen = crate::progress::BootProgress::default();
        for line in decoded.lines() {
            from_the_screen.observe_line(line);
        }
        let mut from_a_wire = crate::progress::BootProgress::default();
        from_a_wire.observe_line(banner);

        assert_eq!(
            from_the_screen.reached(),
            from_a_wire.reached(),
            "the same text reaches the same stage however it was carried"
        );
        assert_ne!(
            from_a_wire.reached(),
            crate::progress::Stage::Cold,
            "and the stage it reached is not the do-nothing one, or this proves nothing"
        );
    }

    /// Text that wraps and text that scrolls both come back, because the console's own wrapping is
    /// what put it there and this reads the result rather than replaying the input.
    #[test]
    fn a_wrapped_line_reads_back_as_the_two_lines_it_became() {
        let decoded = read(&painted(4, 2, "abcdefgh")).expect("a readable dump");
        assert_eq!(decoded, "abcd\nefgh\n");
    }

    /// A blank screen is blank lines, not an error and not `?`s.
    #[test]
    fn a_cleared_screen_decodes_to_nothing() {
        let decoded = read(&painted(8, 3, "")).expect("a readable dump");
        assert_eq!(decoded, "\n\n\n");
    }

    /// A cell holding something that is not one of our glyphs is one `?`, and the rest of the
    /// screen still decodes. See this module's BUGS.
    #[test]
    fn a_cell_that_is_not_a_glyph_reads_as_a_question_mark() {
        let mut ppm = painted(2, 1, "ab");
        let header = ppm.len() - 14 * 8 * 3;
        // Scribble a diagonal through the first cell that no 7x8 glyph could be.
        for y in 0..8 {
            let at = header + (y * 14 + y.min(6)) * 3;
            ppm[at..at + 3].copy_from_slice(&[0xff, 0xff, 0xff]);
        }
        assert_eq!(read(&ppm).expect("still readable"), "?b\n");
    }

    /// The two ways a dump can be unusable are told apart, because "you pointed me at a PNG" and
    /// "the emulator was still writing" want different reactions from whoever reads the failure.
    #[test]
    fn an_unreadable_dump_says_which_kind_it_is() {
        assert_eq!(read(b"\x89PNG\r\n").unwrap_err(), ReadError::NotAPixmap);
        assert_eq!(read(b"P6\n2 2\n254\n").unwrap_err(), ReadError::NotAPixmap);
        assert!(matches!(
            read(b"P6\n8 8\n255\n\x00\x00\x00").unwrap_err(),
            ReadError::Truncated {
                wanted: 192,
                got: 3
            }
        ));
    }

    /// A comment in the header is legal PPM and nothing in this tree writes one, so it is the sort
    /// of thing a parser gets wrong the first time somebody else's tool produces a dump.
    #[test]
    fn a_header_comment_is_skipped() {
        let mut ppm = b"P6\n# written by something else\n7 8\n255\n".to_vec();
        ppm.extend(std::iter::repeat_n(PAPER, 7 * 8).flatten());
        assert_eq!(read(&ppm).expect("readable"), "\n");
    }

    /// [`ReadError`]'s `Display`, which nothing else in the suite reads: the other error test
    /// checks the *value* with `PartialEq`, never the words a person at a bench would see.
    #[test]
    fn a_read_error_displays_what_went_wrong() {
        assert_eq!(
            ReadError::NotAPixmap.to_string(),
            "not a binary P6 portable pixmap"
        );
        assert_eq!(
            ReadError::Truncated { wanted: 10, got: 3 }.to_string(),
            "the pixmap promised 10 bytes of pixels and has 3"
        );
    }

    /// **Width and height of zero are each rejected alone**, not only together. The header check
    /// is three conditions joined by `||`; a test that only ever sets more than one of them at
    /// once cannot tell that operator apart from `&&`, since either grouping still fires when
    /// every condition is true. `an_unreadable_dump_says_which_kind_it_is` covers the third
    /// (`max != 255`) alone already; this covers the other two.
    #[test]
    fn width_or_height_of_zero_is_rejected_even_when_the_other_is_fine() {
        assert_eq!(read(b"P6\n0 5\n255\n").unwrap_err(), ReadError::NotAPixmap);
        assert_eq!(read(b"P6\n5 0\n255\n").unwrap_err(), ReadError::NotAPixmap);
    }
}
