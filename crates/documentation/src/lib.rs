//! **The documentation crate**: markdown in, styled terminal bytes out, and the index that says what exists.
//!
//! Milestone 40. This crate is the pure half of the documentation service. It holds three things
//! and no others: a streaming markdown renderer ([`Renderer`]), the byte layout of a search index
//! ([`index`]), and the query that reads one. There is no IO here, no syscall, no endpoint and no
//! allocator on the path a confined program takes. `components/src/mdr.rs` is the program; this is
//! what
//! it computes.
//!
//! Name: ratified 2026-09-13 (calef, in conversation while working the unratified worklist).
//! Refused `manual` (names what a reader is looking for rather than what the code is, and it was
//! the name that hid the real reason the pair split), `doc` (ungreppable against rustdoc's own
//! vocabulary, and it collides with `cargo doc` in conversation), `documentation_system` (this
//! crate is the pure half: no IO, no syscall, no endpoint, so `system` claims the running service
//! rather than the computation), `markdown` (names a third of the crate; the index layout and its
//! query are the other two thirds and `apropos` is their consumer).
//!
//! **Why the crate and the program do not share a name**, which is the tree's one deliberate break
//! of the pairing AGENTS.md describes: this crate is a superset rather than a collision workaround.
//! It holds the renderer, the byte layout of the search index and the query that reads one; the
//! program is only the renderer's two ends. The block that minted `manual` gave the `cargo doc`
//! collision as the reason, and that was never the real one.
//!
//! # The renderer is a stream, not a parser
//!
//! [`Renderer::feed`] takes whatever bytes arrived and writes whatever output they completed. It
//! never holds a document, never builds a tree, and never allocates. That shape is not an
//! optimisation, it is what the program's capabilities already are: `doc` receives its input as
//! [`byte_sink_protocol`](../byte_sink_protocol/index.html) messages of sixteen bytes each and writes its output
//! the same way, so a renderer that needed the whole document would need somewhere to put it, and
//! somewhere to put it is a memory grant the program can otherwise do without.
//!
//! The cost is stated rather than hidden: a source line longer than [`LINE_MAX`] is truncated, and
//! it is the only place this renderer still loses characters. A table *longer* than [`TABLE_ROWS`]
//! does not lose rows; it spills into a second aligned chunk. A table *wider* than [`TABLE_COLS`]
//! does not lose cells either; they fold into the last column. Losing text is the one failure mode
//! a documentation service cannot have. See `BUGS`.
//!
//! # What it renders
//!
//! ATX headings, paragraphs with word wrapping, fenced code blocks, block quotes, bullet and
//! ordered lists with nesting by indent, tables with computed column widths and the alignment their
//! delimiter row asks for, thematic breaks, and
//! the inline set: `**strong**`, `*emphasis*`, `` `code` ``, `~~strike~~`, `[text](dest)` and
//! `![alt](dest)`. That set was chosen by counting: it is every construct that appears in this
//! repository's own 328 markdown files, which is the corpus the viewer exists to serve.
//!
//! # EXAMPLES
//!
//! Render a document to plain text at eighty columns:
//!
//! ```
//! use documentation::{Renderer, Sink, Style};
//!
//! struct Buf(String);
//! impl Sink for Buf {
//!     fn put(&mut self, bytes: &[u8]) {
//!         self.0.push_str(core::str::from_utf8(bytes).unwrap());
//!     }
//! }
//!
//! let mut out = Buf(String::new());
//! let mut r = Renderer::new(Style { width: 80, color: false });
//! r.feed(b"# Title\n\nSome **strong** text.\n", &mut out);
//! r.finish(&mut out);
//! assert_eq!(out.0, "TITLE\n\n  Some strong text.\n");
//! ```
//!
//! The same document with colour on, which is what `doc` emits when it is writing to a terminal:
//!
//! ```
//! # use documentation::{Renderer, Sink, Style};
//! # struct Buf(String);
//! # impl Sink for Buf {
//! #     fn put(&mut self, bytes: &[u8]) { self.0.push_str(core::str::from_utf8(bytes).unwrap()); }
//! # }
//! let mut out = Buf(String::new());
//! let mut r = Renderer::new(Style { width: 80, color: true });
//! r.feed(b"a **b** c\n", &mut out);
//! r.finish(&mut out);
//! assert!(out.0.contains("\x1b[1mb"));
//! ```
//!
//! # BUGS
//!
//! - **A source line longer than [`LINE_MAX`] is truncated**, and the truncation is silent in the
//!   output. The longest line in this repository is 1925 bytes <!--count:longest-markdown-line-->,
//!   which is why the limit is what it is; a document from somewhere else may lose text. The
//!   renderer records it, so a caller that wants to know can ask [`Renderer::is_truncated`].
//! - **A fence inside a block quote closes now, and did not until 2026-08-18.** The closing test
//!   was matched against the raw line, so a quoted closing fence never matched its own opener and the
//!   renderer stayed in code mode to the end of the document: one quoted transcript misrendered
//!   every line after it. The corpus test could not find it, because verbatim output loses no
//!   characters; what it found was the *opening* fence's info string missing from a page that then
//!   rendered wrong for three hundred lines, and the filter that hid the symptom was left in place
//!   on purpose until somebody answered whether the renderer kept quote state across a nested
//!   fence. It did not. `a_fence_inside_a_block_quote_closes` is what holds it now, and it has to
//!   be a unit test: the corpus check cannot see this class of failure at all, because verbatim
//!   output loses no characters. [`Renderer::has_unclosed_fence`] catches the case where the stuck
//!   fence is the last one in the page, which is a cheap invariant rather than the guard.
//! - **A lazy continuation inside a quoted fence keeps its quote markers.** A line inside
//!   a quoted fence that drops its own marker is taken verbatim, marker and all, because the
//!   alternative is guessing which of the two readings the author meant. `CommonMark` says the fence ends; this
//!   renderer says the line is code. Nothing in this repository writes one.
//! - **Setext headings (`Title` over `=====`) are not recognised.** Nothing in this repository uses
//!   one, and `---` on its own line is far more often a thematic break here, so treating it as a
//!   heading would misrender 64 real lines to catch zero.
//! - **Reference links (`[text][label]`) render as their literal text.** There is exactly one in
//!   the corpus.
//! - **No HTML.** An HTML tag is passed through as text, which is what it looks like in a terminal
//!   anyway. There are 53 in the corpus and every one of them is either inside a code span or a
//!   placeholder like `<name>` that a reader wants to see literally.
//! - **A table wider than [`TABLE_COLS`] folds its remaining cells into the last column**, so the
//!   layout degrades and the text does not. Until 2026-09-23 it dropped them instead, in silence,
//!   and `notes/rented-metal.md` landed with twelve columns against a bound of 8 and turned `main`
//!   red an hour later. That page still renders as eight columns with the last five cells of each
//!   row inside the eighth, which is ugly and loses nothing.
//!
//!   **The bound stayed at 8 on measured evidence, having been raised and put back.** Sixteen
//!   would have held that table as twelve real columns and costs 1536 bytes in [`Renderer`], which
//!   `components/src/mdr.rs` holds as a `static mut` in a process whose whole memory is a grant the
//!   progenitor pays for out of a bounded untyped. CI's `x86_64` `shell-check` answered at once:
//!   `mdr gate.txt` began reporting "could not spawn (the progenitor is out of memory)" while
//!   aarch64 and riscv64 stayed green. Raising it is a decision about `mdr`'s grant, not about
//!   rendering, and it wants somebody who can price the untyped.
//!
//!   The fold is deliberately not loud: a renderer that refused input it could not lay out
//!   prettily would be worse than one that lays it out badly, and [`Renderer::is_truncated`] stays
//!   what it says it is, a report that characters were lost.
//!
//!   **The entry this replaces is why the bug survived**, and it is worth saying so here rather
//!   than in a commit message. It read "wider tables lose their right-hand columns", which is an
//!   honest sentence describing a permanent property, so every reader who met it treated a silent
//!   text loss as a design and nobody asked what the bound should be. A `BUGS` entry is a fact,
//!   and the convention holds; but an entry recording that this renderer *loses characters* is a
//!   defect report with a deadline on it, not a property, and the next one written in this crate
//!   should be read that way.
//! - **A table that spills past [`TABLE_ROWS`] loses its header emphasis and its column alignment
//!   in the second chunk.** Both are read off the delimiter row, which arrived in the first chunk
//!   and is not carried across the flush that makes room. The rows themselves are never lost, which
//!   is the failure mode that matters, and this repository's largest table (the decision index in
//!   `design/decisions/README.md`, over 200 rows) hits it. Carrying them means keeping two more fields across a
//!   flush that exists to reset state, and nobody has asked for it.
//! - **Column alignment reached nothing at all until 2026-09-13**, which is worth recording rather
//!   than quietly fixing: [`Renderer`] parsed `:---`, `---:` and `:---:` off the delimiter row from
//!   the first day and then padded every cell on the right. No test could have found it, because
//!   the value had no consumer; the mutation sweep did, by leaving sixteen mutants alive in a
//!   function whose result nothing read. See notes/mutation-testing.md.
//! - **This crate's mutation score is not 100% and the residue is listed rather than implied.**
//!   `notes/mutation-testing.md` carries the per-function ledger: what was killed, what is proved
//!   equivalent, and what is an honest deferral. A reader who finds a number for `documentation` in
//!   a published report can go there for what it is made of.
//! - **Width is counted in characters, not columns.** A UTF-8 continuation byte counts as zero, so
//!   ASCII and Latin text wrap correctly and a wide CJK character is counted as one column when it
//!   occupies two. There is no CJK in the corpus and no font that could draw it.

#![no_std]

#[cfg(feature = "builder")]
extern crate alloc;

pub mod index;
mod render;

pub use render::{Attr, LINE_MAX, MAX_DEPTH, Renderer, Style, TABLE_COLS, TABLE_ROWS, TABLE_TEXT};

/// Somewhere rendered bytes go.
///
/// The same shape as [`line_editor::Sink`](../line_editor/trait.Sink.html), and deliberately not
/// that trait: this crate has no dependencies, and a renderer that needed the line discipline crate
/// to describe "bytes leave here" would have taken one for a four-line definition. A caller that
/// holds both can implement this over that in one line.
///
/// **An implementation may be called with an empty slice** and must tolerate it.
pub trait Sink {
    /// Take these bytes. There is no partial acceptance and no error: a sink that cannot take them
    /// drops them, which is the same choice `byte_sink_protocol` makes for a destination that has gone.
    fn put(&mut self, bytes: &[u8]);
}
