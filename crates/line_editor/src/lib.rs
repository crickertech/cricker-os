//! **The line discipline engine: a sans-IO editor for a serial terminal** (milestone 28).
//!
//! This crate is the pure logic of the layer Unix calls the tty line discipline: byte in,
//! echo bytes out, completed lines out. It performs no IO and knows nothing about IPC, UARTs,
//! or endpoints; the `line_editor` userspace component feeds it one byte at a time and forwards
//! whatever it emits. That split is DECISIONS §7 applied: the editing rules are host-tested in
//! milliseconds (with a small terminal model, see the tests), and only the wiring needs QEMU.
//!
//! # Examples
//!
//! Typing a line, editing it, and getting it back. The [`Sink`] is where echo goes; here it is a
//! `Vec`, and in the running system it is an endpoint.
//!
//! ```
//! use line_editor::{Event, LineDisc, Sink};
//!
//! struct Echo(Vec<u8>);
//! impl Sink for Echo {
//!     fn put(&mut self, bytes: &[u8]) {
//!         self.0.extend_from_slice(bytes);
//!     }
//! }
//!
//! let mut out = Echo(Vec::new());
//! let mut ld = LineDisc::new();
//! ld.start_line(b"$ ", &mut out);
//!
//! // Type "13", then fix it: ^B moves the cursor left one, onto the 3, and typing 2 **inserts**
//! // before it rather than overwriting it.
//! for b in b"13" {
//!     assert_eq!(ld.feed(*b, &mut out), Event::None);
//! }
//! ld.feed(0x02, &mut out); // ^B
//! ld.feed(b'2', &mut out);
//!
//! // Carriage return ends the line. The caller reads it before feeding the next byte.
//! assert_eq!(ld.feed(b'\r', &mut out), Event::Line);
//! assert_eq!(ld.line(), b"123");
//!
//! // The prompt was echoed, and so was everything typed.
//! assert!(out.0.starts_with(b"$ "));
//! ```
//!
//! Three control characters mean things a caller has to act on rather than echo, and `^D`'s double
//! duty is the one worth demonstrating: on an empty line it ends input, and on a non-empty line it is
//! an ordinary delete. That is Unix's behaviour, and it is a decision rather than an accident.
//!
//! ```
//! # use line_editor::{Event, LineDisc, Sink};
//! # struct Echo(Vec<u8>);
//! # impl Sink for Echo {
//! #     fn put(&mut self, bytes: &[u8]) { self.0.extend_from_slice(bytes); }
//! # }
//! let mut out = Echo(Vec::new());
//! let mut ld = LineDisc::new();
//! ld.start_line(b"$ ", &mut out);
//!
//! // ^D on an empty line: end of input.
//! assert_eq!(ld.feed(0x04, &mut out), Event::Eof);
//!
//! // ^C discards whatever was in progress.
//! ld.start_line(b"$ ", &mut out);
//! for b in b"rm -rf" {
//!     ld.feed(*b, &mut out);
//! }
//! assert_eq!(ld.feed(0x03, &mut out), Event::Interrupt);
//!
//! // On a non-empty line, ^D deletes at the cursor instead. ^A goes to the start first.
//! ld.start_line(b"$ ", &mut out);
//! for b in b"xls" {
//!     ld.feed(*b, &mut out);
//! }
//! ld.feed(0x01, &mut out); // ^A: home
//! assert_eq!(ld.feed(0x04, &mut out), Event::None); // deletes the 'x', does NOT end input
//! assert_eq!(ld.feed(b'\r', &mut out), Event::Line);
//! assert_eq!(ld.line(), b"ls");
//! ```
//!
//! An LF straight after a CR is swallowed, which is what makes one engine serve both an interactive
//! terminal (CR) and piped CRLF text without the caller knowing which it has:
//!
//! ```
//! # use line_editor::{Event, LineDisc, Sink};
//! # struct Echo(Vec<u8>);
//! # impl Sink for Echo {
//! #     fn put(&mut self, bytes: &[u8]) { self.0.extend_from_slice(bytes); }
//! # }
//! let mut out = Echo(Vec::new());
//! let mut ld = LineDisc::new();
//! ld.start_line(b"", &mut out);
//!
//! ld.feed(b'a', &mut out);
//! assert_eq!(ld.feed(b'\r', &mut out), Event::Line);
//! assert_eq!(ld.line(), b"a");
//!
//! // The LF of the CRLF pair is not a second, empty line.
//! assert_eq!(ld.feed(b'\n', &mut out), Event::None);
//! ```
//!
//! # What it implements
//!
//! - Insertion and deletion at a cursor, with minimal echo for the common case (typing at the
//!   end of the line) and a full input-region redraw for everything else.
//! - Cursor movement: left/right/home/end, as control characters (^B ^F ^A ^E) and as ANSI
//!   sequences (CSI D/C/H/F, SS3 variants, and the vt220 `~` forms).
//! - Kill and yank: ^K (to end), ^U (to start), ^W (word back), ^Y (insert the kill buffer).
//! - History: an in-ring of the last [`HIST`] non-empty lines, browsed with up/down arrows;
//!   the in-progress line is stashed and restored when you browse back past the newest entry.
//! - Line endings: CR or LF ends a line; an LF immediately after a CR is swallowed, so both
//!   interactive terminals (CR) and piped text (LF, CRLF) work.
//! - ^C discards the line in progress ([`Event::Interrupt`]); ^D on an empty line is
//!   end-of-input ([`Event::Eof`]), on a non-empty line it deletes at the cursor, exactly the
//!   Unix double duty.
//! - ^L clears the screen and repaints the prompt and the line.
//!
//! # What it deliberately does not do
//!
//! - **No terminal size tracking.** The redraw math assumes the line fits one terminal row.
//!   `noline` probes the terminal with a cursor-position query at startup; a line discipline is
//!   always on and cannot block on a terminal that may not answer (a piped boot script never
//!   would). A wrapped long line therefore redraws incorrectly past the margin. Recorded in
//!   notes/terminal-contract.md as an honest limit; [`LINE_MAX`] keeps it rare.
//! - **No tab completion here.** Completion needs the command namespace, which is the
//!   application's knowledge, not the terminal's. The engine reports Tab as [`Event::Tab`] and
//!   leaves the line alone; the terminal component ignores it. A program that runs this engine
//!   in its own process over raw mode (the shell, DECISIONS §227 (how Tab reaches the shell)
//!   option D) answers it with [`LineDisc::pending`] and [`LineDisc::insert_text`].
//! - **No output-side state.** Application output passes through the component untouched except
//!   for newline translation, which is [`expand_output`], a free function on purpose: output
//!   processing shares no state with input editing (Unix tangles these; see notes/tcb.md's
//!   mistake-catalog reading of the tty layer).
//!
//! The echo this engine emits and the sequences it understands are the terminal contract's
//! wire half; the IPC framing is `proto`. Both documented in notes/terminal-contract.md.
//!
//! Name: ratified 2026-07-30 (calef, DECISIONS §39, landed by milestone 46) for the word and again
//! 2026-08-01 (milestone 63) for the spelling, replacing `linedisc` and then `lineedit`. Refused
//! `linedisc` (the correct Unix term of art, which the person who built this system did not
//! recognise, and which imports vocabulary from exactly the model this OS rejects),
//! `line_discipline` (overclaims: that term covers echo, canonical mode, signals and flow control,
//! and this crate is narrower) and `line_edit` (a verb phrase where its sibling `video_terminal` is
//! a noun).

#![no_std]

/// The IPC framing of the terminal contract (notes/terminal-contract.md): opcodes, flags, and
/// limits shared by the discipline server, its clients, and the kernel-side tests. This is a
/// **userspace protocol**, not kernel ABI: the kernel routes these words without reading them.
pub mod proto {
    /// Request opcode, in bits 63:56 of the first `CALL` word. The low 32 bits carry a length
    /// or count; bits 55:32 are reserved and must be zero.
    pub const OP_SHIFT: u32 = 56;
    /// Application → terminal: print `len` bytes from the client's output page. Reply when the
    /// bytes are on the wire: r0 = bytes consumed.
    pub const OP_WRITE: u64 = 1;
    /// Application → terminal: read one line. The low bits carry a prompt length (0 for none);
    /// the prompt bytes are in the client's output page. The reply comes when a line is ready:
    /// r0 = line length (the bytes are in the client's input page), r1 = [`FLAG_EOF`] /
    /// [`FLAG_INTERRUPTED`] or 0.
    pub const OP_READLINE: u64 = 2;
    /// Input driver → terminal: `count` (1..=8) raw wire bytes, packed little-endian in the
    /// second word. Replied immediately (r0 = 0); the rendezvous is the flow control.
    pub const OP_BYTES: u64 = 3;
    /// Application → terminal: how many `^C` has the terminal seen since boot? Replied immediately
    /// (r0 = the running count). This is the shell's `^C` sensor while a foreground job runs and no
    /// read is parked to fail: the shell polls this and drives its two-tier interrupt escalation
    /// from the count's advance (DECISIONS §24). Deliberately a poll, not a delivered
    /// signal: there is no non-blocking receive, so a busy-poll with `yield` is how the shell watches
    /// two things (the job and `^C`) at once until the blocking notification primitive arrives.
    pub const OP_INTRCOUNT: u64 = 4;
    /// Adapter → terminal: print `len` (1..=8) bytes carried **in registers**, packed
    /// little-endian in the second word. Replied when the bytes are on the wire: r0 = bytes
    /// consumed, exactly as [`OP_WRITE`] answers.
    ///
    /// Eight, not sixteen, and that is the request shape rather than a choice: a served request
    /// here arrives through `receive_cap`, which hands the server the reply capability and **two** data
    /// words. [`OP_BYTES`] carries eight for the same reason, from the other direction. So a
    /// sixteen-byte `byte_sink_protocol` message is two of these, which is the honest cost of a second
    /// writer that needs no page.
    ///
    /// # Why this exists when `OP_WRITE` already prints
    ///
    /// [`OP_WRITE`] reads from **the client's output page**, and there is one of those: init maps a
    /// single frame into the terminal read-only and into the shell read/write. A second printing
    /// client would need a second frame, and the terminal would have to be told which one a request
    /// meant, which is a protocol change with a page-index in it. `filesystem_protocol` has the same shape and
    /// the same problem (DECISIONS §55: two client processes staging into one page do not compose,
    /// and that is why the file behind a `>` is the shell itself).
    ///
    /// Register-only sidesteps it entirely, and the sixteen bytes are not a compromise: they are the
    /// three-word fastpath and the exact payload of a `byte_sink_protocol` message, so an adapter that turns
    /// the sink contract into terminal output is one unpack and one `CALL` with **no page at all**.
    /// [`OP_BYTES`] already proved the shape in the other direction, for the input driver.
    ///
    /// So the terminal gets a second *writer* without a second page and without a second client
    /// convention, which is what a sink adapter needed (notes/sink-protocol.md).
    pub const OP_PRINT: u64 = 5;

    /// Application → terminal: switch the terminal between line-discipline and raw mode
    /// (milestone 169). `len` is 1 to enter raw mode, 0 to leave it. Replied immediately, r0 = 0.
    ///
    /// **Raw mode is the primitive a screen editor needs that the line discipline does not give
    /// it** (design/roadmap/169-kilo-editor.md): DECISIONS §21 says a program "never sees a
    /// keystroke, an escape sequence, or an echo", which is exactly wrong for `kilo`, which needs
    /// all three. While raw mode is on, [`OP_BYTES`] bypasses [`super::LineDisc`] entirely: no echo, no
    /// editing, no line assembly. A keystroke reaches the application exactly as it arrived, one
    /// [`OP_READRAW`] reply per burst the driver delivered.
    ///
    /// `OP_READLINE` is refused (`BAD_REQUEST`) while raw mode is on, and `OP_READRAW` is refused
    /// while it is off: the two input models do not mix on one terminal at once, and a client that
    /// tries gets a fast, loud refusal rather than a read that silently never completes.
    ///
    /// Switching in either direction **abandons whatever line was in progress**: the line
    /// discipline's edit buffer going in, raw mode's queued-but-unread bytes coming out. That is
    /// the same discard `^C` already does to the edit buffer, and it exists for the same reason:
    /// a session must never resume half a line typed under the mode it just left. History and the
    /// kill buffer are untouched, because neither belongs to the in-progress line.
    pub const OP_RAWMODE: u64 = 6;

    /// Application → terminal: read raw bytes, no line discipline (milestone 169). Valid only
    /// while raw mode ([`OP_RAWMODE`]) is on; refused with `BAD_REQUEST` otherwise. Replied when
    /// at least one byte is available, never held back to fill more (raw mode's whole point is
    /// keystroke-at-a-time delivery, not batching): r0 = byte count (1..=8), r1 = the bytes packed
    /// little-endian, the same register-only shape [`OP_BYTES`] and [`OP_PRINT`] already use, so
    /// this needs no page either. **At most one read may be outstanding**, exactly like
    /// [`OP_READLINE`]; a second one while one is parked is refused with `BAD_REQUEST`.
    pub const OP_READRAW: u64 = 7;

    /// Supervisor → terminal: **stop serving so a replacement can take over** (milestone 23 (a capability-routed component OS with live replacement), the
    /// `line_editor` swap; ruled by calef 2026-09-26, "1a"). It rides the served endpoint, so its
    /// FIFO does the draining: every request queued ahead of it is served by this instance, and
    /// every one behind it by whoever receives next.
    ///
    /// Before replying, the terminal answers any parked [`OP_READLINE`] or [`OP_READRAW`] with
    /// [`FLAG_RETRY`], because a reply capability cannot leave this process and a reader stranded
    /// on it could never be woken. Then it replies r0 = [`QUIESCED`] and stops receiving until its
    /// supervisor says [`CTL_RESUME`] or [`CTL_QUIT`] on its control endpoint.
    ///
    /// **Refused with [`BAD_REQUEST`] by a terminal started with no control endpoint**, which is
    /// every terminal a boot builds today. Without one there is nobody to tell it to resume, so an
    /// honoured quiesce would be a dead terminal. See this crate's `BUGS` for who may send it.
    ///
    /// Name: provisional (the lane for milestone 23, 2026-09-26).
    pub const OP_QUIESCE: u64 = 8;

    /// The reply word to [`OP_QUIESCE`]: "QUIT", the value `swap_protocol::QUIESCED` also uses.
    pub const QUIESCED: u64 = 0x5155_4954;

    /// Supervisor → quiesced terminal, on its control endpoint: go back to serving. The swap did not
    /// commit, and this instance never lost its state.
    pub const CTL_RESUME: u64 = 1;
    /// Supervisor → quiesced terminal, on its control endpoint: exit. The replacement has taken
    /// over.
    pub const CTL_QUIT: u64 = 2;

    /// `line_editor`'s third start argument, a set of bits a supervisor passes (milestone 23 (a
    /// capability-routed component OS with live replacement)). `START_HANDOFF`: a handoff page is
    /// mapped at [`HANDOFF_VA`](super::component::HANDOFF_VA), so a quiesce writes the blob there.
    /// `START_ABSORB`: this instance replaces another, so it reads that blob before serving and
    /// tells its supervisor [`NOTE_ABSORBED`] or [`NOTE_REFUSED`] on the slot after its control
    /// endpoint.
    ///
    /// Name: provisional (the lane for milestone 23, 2026-09-27).
    pub const START_HANDOFF: u64 = 1 << 0;
    /// See [`START_HANDOFF`].
    pub const START_ABSORB: u64 = 1 << 1;
    /// Terminal → supervisor: the blob was absorbed; this instance is serving.
    pub const NOTE_ABSORBED: u64 = 1;
    /// Terminal → supervisor: the blob was refused; this instance is leaving without serving.
    /// The second word is the refusal: 0 no blob, 1 malformed, otherwise the layout found.
    pub const NOTE_REFUSED: u64 = 2;

    /// Pack a request's first word from an opcode and a length/count.
    pub const fn req(op: u64, len: u64) -> u64 {
        (op << OP_SHIFT) | (len & 0xffff_ffff)
    }
    /// The opcode of a request word.
    pub const fn op(w0: u64) -> u64 {
        w0 >> OP_SHIFT
    }
    /// The length/count of a request word.
    pub const fn len(w0: u64) -> usize {
        (w0 & 0xffff_ffff) as usize
    }

    /// READLINE reply flag: end of input (^D on an empty line). The line length is 0.
    pub const FLAG_EOF: u64 = 1 << 0;
    /// READLINE reply flag: the read was interrupted (^C). The line length is 0. This is the
    /// contract's hook for interrupt routing; see design/interrupt-routing.md.
    pub const FLAG_INTERRUPTED: u64 = 1 << 1;
    /// Read reply flag: **ask again** (calef, 2026-09-26, "2b"). The terminal is about to be
    /// replaced and could not hold this read across the swap; nothing was typed away. Re-issue the
    /// same request, unchanged, and the terminal that answers it resumes the line where it was.
    ///
    /// An [`OP_READLINE`] reply carries it in r1 with r0 = 0. An [`OP_READRAW`] reply cannot,
    /// because r1 is the data there, so it is r0 = 0 (never a byte count, which is 1..=8) with this
    /// flag in r1. [`is_retry`] reads both.
    ///
    /// Name: provisional (the lane for milestone 23, 2026-09-26).
    pub const FLAG_RETRY: u64 = 1 << 2;

    /// **Whether a read reply means "ask again".** One test for both read shapes, so a reader
    /// cannot check the flag in the wrong register.
    pub const fn is_retry(r0: u64, r1: u64) -> bool {
        r0 == 0 && r1 == FLAG_RETRY
    }

    /// The reply to a request whose opcode the terminal does not implement: r0 = this, r1 = 0.
    /// A sentinel rather than silence, so a confused client fails fast instead of hanging.
    pub const BAD_REQUEST: u64 = u64::MAX;
}

/// Where echo bytes go. The server implements this over its console channel; the tests
/// implement it over a terminal model.
pub trait Sink {
    /// Write echo bytes out. Called synchronously from the byte-feeding methods below, so a
    /// `Sink` that blocks blocks the whole discipline.
    fn put(&mut self, bytes: &[u8]);
}

/// The longest line the discipline will assemble. Longer input is refused with a bell.
pub const LINE_MAX: usize = 256;
/// How many history entries the ring holds.
pub const HIST: usize = 8;
/// The longest prompt the discipline will repaint. Longer prompts are truncated.
pub const PROMPT_MAX: usize = 64;

const BS: u8 = 0x08;
const BEL: u8 = 0x07;
const ESC: u8 = 0x1b;

/// What one input byte amounted to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// Nothing the caller needs to act on (echo may still have been emitted).
    None,
    /// A line is complete; read it with [`LineDisc::line`] before feeding the next byte.
    Line,
    /// ^D on an empty line: end of input.
    Eof,
    /// ^C: the line in progress was discarded.
    Interrupt,
    /// Tab: nothing was echoed and the line is unchanged. Whoever runs the engine decides what a
    /// Tab means, because completion needs a namespace the engine does not have (§227).
    Tab,
}

/// The ANSI parser's state. `Esc` has seen 0x1b; `Csi` is inside `ESC [`, accumulating numeric
/// parameters; `Ss3` is inside `ESC O` (application-mode cursor keys send SS3, not CSI).
enum EscState {
    Idle,
    Esc,
    Csi { params: [u16; 2], n: usize },
    Ss3,
}

/// The editing engine. One per terminal; a few KiB of fixed buffers, no allocation.
pub struct LineDisc {
    buf: [u8; LINE_MAX],
    len: usize,
    cur: usize,
    /// The completed line, snapshotted when [`Event::Line`] is returned.
    done: [u8; LINE_MAX],
    done_len: usize,
    kill: [u8; LINE_MAX],
    kill_len: usize,
    hist: [([u8; LINE_MAX], usize); HIST],
    hist_count: usize,
    hist_next: usize,
    /// `Some(k)`: showing the k-th most recent history entry (k in `1..=hist_count`).
    browse: Option<usize>,
    stash: ([u8; LINE_MAX], usize, usize),
    prompt: [u8; PROMPT_MAX],
    prompt_len: usize,
    esc: EscState,
    last_was_cr: bool,
}

impl Default for LineDisc {
    fn default() -> Self {
        Self::new()
    }
}

impl LineDisc {
    /// An empty discipline: no line in progress, no history, prompt unset. `const` on purpose: a
    /// user process here gets one 4 KiB page of stack (`kernel/src/user.rs`'s `USER_STACK_VA`),
    /// and this struct is a few KiB before the temporary a stack-local move would make, so
    /// `components/src/line_editor.rs` holds it in a `static` rather than in `_start`'s own frame
    /// (`display_terminal.rs`'s `TERMINAL` is the same fix for the same reason, one program over).
    pub const fn new() -> Self {
        LineDisc {
            buf: [0; LINE_MAX],
            len: 0,
            cur: 0,
            done: [0; LINE_MAX],
            done_len: 0,
            kill: [0; LINE_MAX],
            kill_len: 0,
            hist: [([0; LINE_MAX], 0); HIST],
            hist_count: 0,
            hist_next: 0,
            browse: None,
            stash: ([0; LINE_MAX], 0, 0),
            prompt: [0; PROMPT_MAX],
            prompt_len: 0,
            esc: EscState::Idle,
            last_was_cr: false,
        }
    }

    /// The completed line, valid after [`Event::Line`] until the next one.
    pub fn line(&self) -> &[u8] {
        &self.done[..self.done_len]
    }

    /// Discard whatever line is in progress: the edit buffer, the cursor, and any history
    /// browsing. History and the kill buffer are untouched, because neither belongs to the line
    /// that was in progress. No echo; the caller prints whatever tells the human why, or nothing
    /// (`^C`'s own `"^C\r\n"` is the caller side of that choice, and [`proto::OP_RAWMODE`]'s
    /// mode switch prints nothing at all).
    pub fn abandon(&mut self) {
        self.len = 0;
        self.cur = 0;
        self.browse = None;
    }

    /// **Resume a read the reader re-issued after [`proto::FLAG_RETRY`]**: remember `prompt` as
    /// [`start_line`](LineDisc::start_line) does, but paint nothing, because the screen already
    /// shows the prompt and the half-typed line. Painting again would print them twice.
    pub fn resume_line(&mut self, prompt: &[u8]) {
        let n = prompt.len().min(PROMPT_MAX);
        self.prompt[..n].copy_from_slice(&prompt[..n]);
        self.prompt_len = n;
    }

    /// The line being edited and the cursor's position in it, for a caller answering
    /// [`Event::Tab`]. Unlike [`LineDisc::line`], this is the line in progress, not a finished one.
    pub fn pending(&self) -> (&[u8], usize) {
        (&self.buf[..self.len], self.cur)
    }

    /// Insert `text` at the cursor as if it had been typed, echoing it. Printable bytes only; any
    /// other byte is skipped rather than inserted, so a completion cannot smuggle a control
    /// sequence into the line. What does not fit in [`LINE_MAX`] is refused with a bell.
    pub fn insert_text(&mut self, text: &[u8], out: &mut impl Sink) {
        for &b in text {
            if (0x20..=0x7e).contains(&b) {
                self.insert(b, out);
            }
        }
    }

    /// Paint the prompt and the line again on a fresh row, cursor where it was. For a caller that
    /// printed something between two keystrokes, as a completion listing does.
    pub fn repaint(&mut self, out: &mut impl Sink) {
        out.put(&self.prompt[..self.prompt_len]);
        out.put(&self.buf[..self.len]);
        csi_left(out, self.len - self.cur);
    }

    /// Begin a read: remember `prompt` (for repaints) and paint it, followed by whatever the
    /// user has already typed ahead. Called by the server when a READLINE request arrives.
    pub fn start_line(&mut self, prompt: &[u8], out: &mut impl Sink) {
        let n = prompt.len().min(PROMPT_MAX);
        self.prompt[..n].copy_from_slice(&prompt[..n]);
        self.prompt_len = n;
        out.put(&self.prompt[..n]);
        out.put(&self.buf[..self.len]);
        csi_left(out, self.len - self.cur);
    }

    /// Feed one wire byte; echo goes to `out`. See [`Event`] for what came of it.
    pub fn feed(&mut self, b: u8, out: &mut impl Sink) -> Event {
        // A bare LF right after a CR is the second half of a CRLF, not a second Enter.
        let was_cr = core::mem::replace(&mut self.last_was_cr, false);
        if b == b'\n' && was_cr {
            return Event::None;
        }

        match core::mem::replace(&mut self.esc, EscState::Idle) {
            EscState::Idle => {}
            EscState::Esc => {
                match b {
                    b'[' => {
                        self.esc = EscState::Csi {
                            params: [0; 2],
                            n: 0,
                        }
                    }
                    b'O' => self.esc = EscState::Ss3,
                    _ => {} // an escape we do not speak; drop it and the introducer
                }
                return Event::None;
            }
            EscState::Csi { mut params, mut n } => {
                match b {
                    b'0'..=b'9' => {
                        let i = n.min(1);
                        params[i] = params[i].saturating_mul(10) + (b - b'0') as u16;
                        self.esc = EscState::Csi { params, n };
                    }
                    b';' => {
                        n += 1;
                        self.esc = EscState::Csi { params, n };
                    }
                    // A final byte: dispatch. Unknown finals are dropped whole, which is what
                    // makes unrecognized keys (F-keys, mouse noise) harmless.
                    0x40..=0x7e => return self.csi_final(b, params[0], out),
                    _ => {} // malformed; abandon
                }
                return Event::None;
            }
            EscState::Ss3 => return self.csi_final(b, 0, out),
        }

        match b {
            ESC => {
                self.esc = EscState::Esc;
                Event::None
            }
            b'\r' | b'\n' => {
                self.last_was_cr = b == b'\r';
                self.finish_line(out)
            }
            0x03 => {
                // ^C: discard the line in progress. The caller routes the interrupt (or, today,
                // merely fails a pending read); see design/interrupt-routing.md.
                out.put(b"^C\r\n");
                self.abandon();
                Event::Interrupt
            }
            b'\t' => Event::Tab,
            0x04 => {
                if self.len == 0 {
                    Event::Eof
                } else {
                    self.delete_at(out);
                    Event::None
                }
            }
            0x01 => {
                self.home(out);
                Event::None
            }
            0x05 => {
                self.end(out);
                Event::None
            }
            0x02 => {
                self.left(out);
                Event::None
            }
            0x06 => {
                self.right(out);
                Event::None
            }
            0x0b => {
                // ^K: kill to end of line.
                self.kill[..self.len - self.cur].copy_from_slice(&self.buf[self.cur..self.len]);
                self.kill_len = self.len - self.cur;
                self.len = self.cur;
                out.put(b"\x1b[K");
                Event::None
            }
            0x15 => {
                // ^U: kill to start of line.
                self.kill[..self.cur].copy_from_slice(&self.buf[..self.cur]);
                self.kill_len = self.cur;
                self.delete_range(0, self.cur, out);
                Event::None
            }
            0x17 => {
                // ^W: kill the word before the cursor (blanks, then non-blanks, like bash).
                let mut a = self.cur;
                while a > 0 && self.buf[a - 1] == b' ' {
                    a -= 1;
                }
                while a > 0 && self.buf[a - 1] != b' ' {
                    a -= 1;
                }
                self.kill[..self.cur - a].copy_from_slice(&self.buf[a..self.cur]);
                self.kill_len = self.cur - a;
                self.delete_range(a, self.cur, out);
                Event::None
            }
            0x19 => {
                self.yank(out);
                Event::None
            }
            0x0c => {
                // ^L: clear the screen, repaint prompt and line. The one op that needs the
                // prompt, which is why READLINE carries it.
                out.put(b"\x1b[2J\x1b[H");
                out.put(&self.prompt[..self.prompt_len]);
                out.put(&self.buf[..self.len]);
                csi_left(out, self.len - self.cur);
                Event::None
            }
            BS | 0x7f => {
                self.backspace(out);
                Event::None
            }
            0x20..=0x7e => {
                self.insert(b, out);
                Event::None
            }
            _ => Event::None, // other control bytes: ignored, deliberately unecho'd
        }
    }

    /// A CSI (or SS3) final byte, with its first numeric parameter.
    fn csi_final(&mut self, b: u8, p: u16, out: &mut impl Sink) -> Event {
        match (b, p) {
            (b'D', _) => self.left(out),
            (b'C', _) => self.right(out),
            (b'A', _) => self.hist_prev(out),
            (b'B', _) => self.hist_next_entry(out),
            (b'H', _) | (b'~', 1) | (b'~', 7) => self.home(out),
            (b'F', _) | (b'~', 4) | (b'~', 8) => self.end(out),
            (b'~', 3) => self.delete_at(out),
            _ => {} // unknown key; ignore
        }
        Event::None
    }

    fn finish_line(&mut self, out: &mut impl Sink) -> Event {
        out.put(b"\r\n");
        self.done[..self.len].copy_from_slice(&self.buf[..self.len]);
        self.done_len = self.len;
        // Record non-empty lines, skipping an exact repeat of the newest entry.
        if self.len > 0 && !self.repeats_newest() {
            self.hist[self.hist_next].0[..self.len].copy_from_slice(&self.buf[..self.len]);
            self.hist[self.hist_next].1 = self.len;
            self.hist_next = (self.hist_next + 1) % HIST;
            self.hist_count = (self.hist_count + 1).min(HIST);
        }
        self.len = 0;
        self.cur = 0;
        self.browse = None;
        // The prompt belonged to the read that just completed; the next read brings its own.
        self.prompt_len = 0;
        Event::Line
    }

    fn repeats_newest(&self) -> bool {
        if self.hist_count == 0 {
            return false;
        }
        let (entry, elen) = &self.hist[(self.hist_next + HIST - 1) % HIST];
        *elen == self.len && entry[..*elen] == self.buf[..self.len]
    }

    fn insert(&mut self, b: u8, out: &mut impl Sink) {
        if self.len >= LINE_MAX {
            out.put(&[BEL]);
            return;
        }
        self.buf.copy_within(self.cur..self.len, self.cur + 1);
        self.buf[self.cur] = b;
        self.len += 1;
        self.cur += 1;
        if self.cur == self.len {
            // The common case, typing at the end: echo the byte and nothing else.
            out.put(&[b]);
        } else {
            // Mid-line: echo the byte and the shifted tail, then walk back over the tail.
            out.put(&self.buf[self.cur - 1..self.len]);
            csi_left(out, self.len - self.cur);
        }
    }

    fn backspace(&mut self, out: &mut impl Sink) {
        if self.cur == 0 {
            return;
        }
        self.delete_range(self.cur - 1, self.cur, out);
    }

    fn delete_at(&mut self, out: &mut impl Sink) {
        if self.cur == self.len {
            return;
        }
        self.delete_range(self.cur, self.cur + 1, out);
    }

    /// Remove `buf[a..b]` and repaint what changed: step back to column `a`, print the shifted
    /// tail, erase the leftover columns, step back to the cursor. CSI K does the erasing, so no
    /// space-padding arithmetic. Works for both directions: a backward kill has the cursor at
    /// `b` (it moves left with the deletion), a forward delete has it at `a` (it stays put).
    fn delete_range(&mut self, a: usize, b: usize, out: &mut impl Sink) {
        debug_assert!(a <= b && b <= self.len && a <= self.cur);
        let removed = b - a;
        if removed == 0 {
            return;
        }
        let old_cur = self.cur;
        self.buf.copy_within(b..self.len, a);
        self.len -= removed;
        self.cur = if old_cur >= b {
            old_cur - removed
        } else {
            old_cur.min(a)
        };
        csi_left(out, old_cur - a);
        out.put(&self.buf[a..self.len]);
        out.put(b"\x1b[K");
        csi_left(out, self.len - self.cur);
    }

    fn yank(&mut self, out: &mut impl Sink) {
        let n = self.kill_len.min(LINE_MAX - self.len);
        if n == 0 {
            if self.kill_len > 0 {
                out.put(&[BEL]); // there was something to yank and no room for it
            }
            return;
        }
        self.buf.copy_within(self.cur..self.len, self.cur + n);
        // A yank of the kill buffer into the line it was cut from cannot overlap: kill is a
        // separate buffer, copied out at kill time.
        let kill = self.kill;
        self.buf[self.cur..self.cur + n].copy_from_slice(&kill[..n]);
        self.len += n;
        self.cur += n;
        out.put(&self.buf[self.cur - n..self.len]);
        csi_left(out, self.len - self.cur);
    }

    fn left(&mut self, out: &mut impl Sink) {
        if self.cur > 0 {
            self.cur -= 1;
            out.put(&[BS]);
        }
    }

    fn right(&mut self, out: &mut impl Sink) {
        if self.cur < self.len {
            self.cur += 1;
            out.put(b"\x1b[C");
        }
    }

    fn home(&mut self, out: &mut impl Sink) {
        csi_left(out, self.cur);
        self.cur = 0;
    }

    fn end(&mut self, out: &mut impl Sink) {
        csi_right(out, self.len - self.cur);
        self.cur = self.len;
    }

    /// Up arrow: show the previous (older) history entry. The first press stashes the line in
    /// progress so browsing is non-destructive.
    fn hist_prev(&mut self, out: &mut impl Sink) {
        let k = match self.browse {
            None => {
                if self.hist_count == 0 {
                    out.put(&[BEL]);
                    return;
                }
                self.stash.0[..self.len].copy_from_slice(&self.buf[..self.len]);
                self.stash.1 = self.len;
                self.stash.2 = self.cur;
                1
            }
            Some(k) if k < self.hist_count => k + 1,
            Some(_) => {
                out.put(&[BEL]); // already at the oldest
                return;
            }
        };
        self.browse = Some(k);
        let (entry, elen) = self.hist[(self.hist_next + HIST - k) % HIST];
        self.replace_line(&entry[..elen], out);
    }

    /// Down arrow: toward newer entries; past the newest, restore the stashed line.
    fn hist_next_entry(&mut self, out: &mut impl Sink) {
        match self.browse {
            None => out.put(&[BEL]),
            Some(1) => {
                self.browse = None;
                let (stash, slen, scur) = self.stash;
                self.replace_line(&stash[..slen], out);
                // replace_line leaves the cursor at the end; restore the stashed column.
                csi_left(out, slen - scur);
                self.cur = scur;
            }
            Some(k) => {
                self.browse = Some(k - 1);
                let (entry, elen) = self.hist[(self.hist_next + HIST - (k - 1)) % HIST];
                self.replace_line(&entry[..elen], out);
            }
        }
    }

    /// Replace the whole edit line with `text` and repaint the input region, cursor at the end.
    fn replace_line(&mut self, text: &[u8], out: &mut impl Sink) {
        csi_left(out, self.cur);
        let n = text.len().min(LINE_MAX);
        self.buf[..n].copy_from_slice(&text[..n]);
        self.len = n;
        self.cur = n;
        out.put(&self.buf[..n]);
        out.put(b"\x1b[K");
    }
}

/// How many raw bytes [`RawQueue`] holds for a parked `OP_READRAW`, when bytes arrive faster than
/// the raw reader drains them. Sized like [`LINE_MAX`]'s bell-on-overflow precedent: a burst past
/// this is dropped, audibly, rather than grown without bound. 64 is generous for a keystroke-at-a
/// time reader (`kilo`'s own event loop reads one key, redraws, reads the next) and still covers a
/// terminal-paste dumped in one go.
pub const RAW_QUEUE_MAX: usize = 64;

/// **Raw-mode byte delivery** (milestone 169, `OP_READRAW`): a plain FIFO of wire bytes, with no
/// interpretation of any of them. Kept separate from [`LineDisc`] rather than a mode bit on it,
/// because raw mode's whole point is that nothing here ever looks at what a byte means; a
/// `LineDisc` exists to do exactly that.
pub struct RawQueue {
    buf: [u8; RAW_QUEUE_MAX],
    head: usize,
    len: usize,
}

impl Default for RawQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl RawQueue {
    /// An empty queue. `const` for the same reason [`LineDisc::new`] is: so a holder can be a
    /// `static` instead of a stack local.
    pub const fn new() -> Self {
        RawQueue {
            buf: [0; RAW_QUEUE_MAX],
            head: 0,
            len: 0,
        }
    }

    /// True once nothing is queued.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Push bytes onto the queue. Returns the number actually pushed; once the queue is full the
    /// rest are dropped, and the caller (which holds the [`Sink`] this crate deliberately does
    /// not) is the one that rings the bell, the same split [`LineDisc::feed`] keeps between
    /// interpretation and echo.
    pub fn push(&mut self, bytes: &[u8]) -> usize {
        let room = RAW_QUEUE_MAX - self.len;
        let n = bytes.len().min(room);
        for &b in &bytes[..n] {
            self.buf[(self.head + self.len) % RAW_QUEUE_MAX] = b;
            self.len += 1;
        }
        n
    }

    /// Pop up to 8 bytes (the register-only reply's own limit), packed little-endian. `None` if
    /// the queue is empty.
    pub fn pop8(&mut self) -> Option<(usize, u64)> {
        if self.len == 0 {
            return None;
        }
        let n = self.len.min(8);
        let mut packed = 0u64;
        for i in 0..n {
            packed |= (self.buf[(self.head + i) % RAW_QUEUE_MAX] as u64) << (8 * i);
        }
        self.head = (self.head + n) % RAW_QUEUE_MAX;
        self.len -= n;
        Some((n, packed))
    }
}

/// Output-side processing: translate `\n` to `\r\n` so applications can speak Unix newlines to
/// a serial terminal that wants a carriage return. Everything else, ANSI included, passes
/// through untouched: the wire belongs to the application when it is printing.
pub fn expand_output(src: &[u8], out: &mut impl Sink) {
    let mut from = 0;
    for (i, &b) in src.iter().enumerate() {
        if b == b'\n' {
            out.put(&src[from..i]);
            out.put(b"\r\n");
            from = i + 1;
        }
    }
    out.put(&src[from..]);
}

/// Emit `CSI <n> D` (cursor left `n`), eliding the count when it is 1 and the whole sequence
/// when it is 0.
fn csi_left(out: &mut impl Sink, n: usize) {
    csi_move(out, n, b'D');
}

/// Emit `CSI <n> C` (cursor right `n`), same elisions.
fn csi_right(out: &mut impl Sink, n: usize) {
    csi_move(out, n, b'C');
}

fn csi_move(out: &mut impl Sink, n: usize, dir: u8) {
    match n {
        0 => {}
        1 => out.put(&[ESC, b'[', dir]),
        _ => {
            // One contiguous put per sequence: a sink must never see half an escape.
            let mut seq = [0u8; 11];
            seq[0] = ESC;
            seq[1] = b'[';
            let mut i = 10;
            let mut v = n;
            while v > 0 {
                i -= 1;
                seq[i] = b'0' + (v % 10) as u8;
                v /= 10;
            }
            seq.copy_within(i..10, 2);
            let digits = 10 - i;
            seq[2 + digits] = dir;
            out.put(&seq[..3 + digits]);
        }
    }
}

/// **What a supervisor routes to a `line_editor` it can replace** (milestone 23 (a
/// capability-routed component OS with live replacement)): the capability half of the terminal
/// contract, beside the wire half in [`proto`]. Two declarations, because the output sink is one of
/// two shapes: the console's two-endpoint protocol, older than §12 (call/reply IPC: a
/// one-shot reply capability), which needs a reply endpoint, or
/// `display_terminal`'s one `CALL`, which does not. Everything else is shared.
///
/// **Slot order is the one `line_editor` already reads**: the terminal in 0, the sink in 1, the
/// console's reply in 2 when there is one, then the control endpoint and the supervisor's
/// notification endpoint. `line_editor` learns the control slot from its second start argument and
/// finds the notification endpoint in the slot after it, so one binary serves both shapes; the
/// `const` assertions below hold the declarations to that.
///
/// Name: provisional (the lane for milestone 23, 2026-09-27), for the module, both constants,
/// `HANDOFF_VA` and the role and contract strings.
pub mod component {
    use component_plan::Direction::{Serve, Use};
    use component_plan::{CapNeed, Handoff, MapNeed, PageKind, Requirements};

    /// Where the handoff page sits. Clear of the three pages `line_editor` already maps.
    pub const HANDOFF_VA: u64 = 0x00a0_0000;
    /// One page. Measured, not assumed: a full history ring and a full queue are under 4 KiB.
    pub const HANDOFF_PAGES: u64 = 1;
    /// Pages one instance is built out of: its image, its `.bss` (the discipline, the queues), a
    /// twelve-page stack, its page tables and revocation log: twelve image pages measured on riscv64
    /// (2026-09-27), about thirty in all, so roughly one and a half times what it uses.
    pub const INSTANCE_PAGES: u64 = 48;
    /// The stack a supervisor builds an instance with: `system_initializer`'s `CHILD_STACK_PAGES`,
    /// which is what `line_editor` has always run on.
    pub const STACK_PAGES: u64 = 12;

    const HANDOFF: Handoff = Handoff {
        va: HANDOFF_VA,
        pages: HANDOFF_PAGES,
    };
    const MAPS: &[MapNeed] = &[
        MapNeed {
            role: "sink_page",
            va: 0x0060_0000,
            kind: PageKind::Shared,
        },
        MapNeed {
            role: "client_out",
            va: 0x0080_0000,
            kind: PageKind::ReadOnly,
        },
        MapNeed {
            role: "client_in",
            va: 0x0090_0000,
            kind: PageKind::Shared,
        },
    ];

    /// A `line_editor` printing through the console server.
    pub const CONSOLE: Requirements = Requirements {
        contract: "terminal",
        caps: &[
            CapNeed {
                role: "terminal",
                direction: Serve,
            },
            CapNeed {
                role: "sink",
                direction: Use,
            },
            CapNeed {
                role: "sink_reply",
                direction: Serve,
            },
            CapNeed {
                role: "control",
                direction: Serve,
            },
            CapNeed {
                role: "supervisor",
                direction: Use,
            },
        ],
        maps: MAPS,
        pages: INSTANCE_PAGES,
        // It forwards synchronously to its sink while serving the shell, which is the one shape
        // `depends_on` exists to name.
        depends_on: &["console"],
        handoff: Some(HANDOFF),
    };

    /// A `line_editor` printing through `display_terminal`.
    pub const DISPLAY: Requirements = Requirements {
        contract: "terminal",
        caps: &[
            CapNeed {
                role: "terminal",
                direction: Serve,
            },
            CapNeed {
                role: "sink",
                direction: Use,
            },
            CapNeed {
                role: "control",
                direction: Serve,
            },
            CapNeed {
                role: "supervisor",
                direction: Use,
            },
        ],
        maps: MAPS,
        pages: INSTANCE_PAGES,
        depends_on: &["display"],
        handoff: Some(HANDOFF),
    };

    /// **How `terminal_supervisor` is endowed**, by `system_initializer` on a real boot and by the
    /// kernel's own test: one list both builders and the supervisor read (AGENTS.md rule 7). The
    /// supervisor holds every object a `line_editor` is built from, with `GRANT`, so it can build a
    /// second one; and the terminal endpoint with `WRITE` too, so it can send `OP_QUIESCE`.
    ///
    /// Start arguments: the output sink's mode (`line_editor`'s own `MODE_CONSOLE` = 0 or
    /// `MODE_DISPLAY` = 1), the length of the `line_editor` image copied to [`ELF_VA`](supervisor::ELF_VA), and the slot
    /// of an endpoint to receive swap requests on, `0` for none (a boot today: the trigger is
    /// milestone 198 (a package manager, and the trivial install)'s installer and is not built).
    ///
    /// Name: provisional (the lane for milestone 23, 2026-09-27), for the module and every constant
    /// in it. `terminal_supervisor` itself is calef's to name.
    pub mod supervisor {
        /// The construction budget.
        pub const BUDGET: u64 = 0;
        /// The terminal endpoint: `READ | WRITE | GRANT`.
        pub const TERMINAL: u64 = 1;
        /// The output sink's request endpoint (console) or served endpoint (display).
        pub const SINK: u64 = 2;
        /// The page `line_editor` fills and the sink prints.
        pub const SINK_PAGE: u64 = 3;
        /// The client's outgoing page, which `line_editor` only reads.
        pub const CLIENT_OUT: u64 = 4;
        /// The client's incoming page, which `line_editor` writes completed lines into.
        pub const CLIENT_IN: u64 = 5;
        /// The console's reply endpoint, in console mode only.
        pub const SINK_REPLY: u64 = 6;
        /// Where the `line_editor` image is copied into the supervisor. Below `supervision_protocol`'s
        /// scratch window, which starts at `0x1000_0000` and grows upward.
        pub const ELF_VA: u64 = 0x0800_0000;
        /// How many pages of budget the supervisor needs: two instances at once (the incumbent
        /// and its replacement) and its own handful of objects and page tables.
        pub const BUDGET_PAGES: u64 = 2 * super::INSTANCE_PAGES + 16;

        /// Requester → supervisor, on the swap endpoint: replace the running `line_editor`.
        pub const SWAP: u64 = 1;
        /// Requester → supervisor: retire the running `line_editor` (quiesce it, tell it to quit,
        /// collect it), reply [`STOPPED`], and exit. So a test's terminal can end with every
        /// region back in its budget; nothing on a boot sends it.
        pub const STOP: u64 = 2;
        /// Supervisor → requester: stopped, and exiting.
        pub const STOPPED: u64 = 0x5354_4f50;
        /// Supervisor → requester: the replacement absorbed the state and serves; the old one is
        /// gone. Second word: how many instances this supervisor has started.
        pub const SWAPPED: u64 = 0x5357_4150;
        /// Supervisor → requester: the replacement refused, and the incumbent serves on. Second
        /// word: the refusal `NOTE_REFUSED` carried.
        pub const ROLLED_BACK: u64 = 0x524f_4c4c;
    }

    const _: () = assert!(CONSOLE.problem().is_none());
    const _: () = assert!(DISPLAY.problem().is_none());
    // The slots `line_editor` reads without being told.
    const _: () = assert!(component_plan::slot_of(&CONSOLE, "terminal") == 0);
    const _: () = assert!(component_plan::slot_of(&CONSOLE, "sink") == 1);
    const _: () = assert!(component_plan::slot_of(&CONSOLE, "sink_reply") == 2);
    const _: () = assert!(component_plan::slot_of(&DISPLAY, "terminal") == 0);
    const _: () = assert!(component_plan::slot_of(&DISPLAY, "sink") == 1);
    // And the one it is told: the notification endpoint follows the control endpoint in both.
    const _: () = assert!(
        component_plan::slot_of(&CONSOLE, "supervisor")
            == component_plan::slot_of(&CONSOLE, "control") + 1
    );
    const _: () = assert!(
        component_plan::slot_of(&DISPLAY, "supervisor")
            == component_plan::slot_of(&DISPLAY, "control") + 1
    );
}

/// **The line editor's state as a handoff blob** (milestone 23 (a capability-routed component OS
/// with live replacement); DECISIONS §209 (state handoff is an opaque blob over a granted frame, and
/// it is optional)). What a replacement `line_editor` needs to carry on as if nothing happened: the
/// half-typed line and its cursor, the kill buffer, the history ring and the prompt.
///
/// **What is deliberately not carried**: a half-received escape sequence (the replacement starts
/// between keys), history browsing (the line being browsed is the edit line, which is carried),
/// and the snapshot of the last completed line (it was already delivered or queued). Each is a
/// state that lasts one keystroke.
///
/// The layout is this crate's, versioned by [`handoff::LAYOUT`]; a supervisor never reads it.
///
/// Name: provisional (the lane for milestone 23, 2026-09-27).
pub mod handoff {
    use super::{HIST, LINE_MAX, LineDisc, PROMPT_MAX, RAW_QUEUE_MAX, RawQueue};

    /// "LEDT", then the layout, in the blob's first eight bytes.
    pub const MAGIC: u32 = 0x5444_454c;
    /// The one layout this build writes and reads.
    pub const LAYOUT: u32 = 1;

    /// A cursor writing into a blob. Refuses to run past its end rather than truncating, because a
    /// truncated blob that parsed would be state silently lost.
    pub struct Writer<'a> {
        buf: &'a mut [u8],
        at: usize,
        full: bool,
    }

    impl<'a> Writer<'a> {
        /// Start a blob: the magic and layout go in first.
        pub fn new(buf: &'a mut [u8]) -> Self {
            let mut w = Writer {
                buf,
                at: 0,
                full: false,
            };
            w.u32(MAGIC);
            w.u32(LAYOUT);
            w
        }
        /// Append bytes.
        pub fn bytes(&mut self, b: &[u8]) {
            if self.at + b.len() > self.buf.len() {
                self.full = true;
                return;
            }
            self.buf[self.at..self.at + b.len()].copy_from_slice(b);
            self.at += b.len();
        }
        /// Append a little-endian word.
        pub fn u32(&mut self, v: u32) {
            self.bytes(&v.to_le_bytes());
        }
        /// Append a length, then that many bytes.
        pub fn field(&mut self, b: &[u8]) {
            self.u32(b.len() as u32);
            self.bytes(b);
        }
        /// How many bytes were written, or `None` if the blob did not fit.
        pub fn finish(self) -> Option<usize> {
            if self.full { None } else { Some(self.at) }
        }
    }

    /// A cursor reading a blob.
    pub struct Reader<'a> {
        buf: &'a [u8],
        at: usize,
    }

    /// Why a blob was not absorbed.
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub enum Refusal {
        /// The blob does not start with [`MAGIC`]: there is no blob at all.
        NoBlob,
        /// The blob is in a layout this build does not understand; carries that layout.
        Layout(u32),
        /// The blob ends early or a field is longer than its buffer.
        Malformed,
    }

    impl<'a> Reader<'a> {
        /// Open a blob, checking the magic and the layout.
        pub fn new(buf: &'a [u8]) -> Result<Self, Refusal> {
            let mut r = Reader { buf, at: 0 };
            if r.u32().ok_or(Refusal::NoBlob)? != MAGIC {
                return Err(Refusal::NoBlob);
            }
            let layout = r.u32().ok_or(Refusal::Malformed)?;
            if layout != LAYOUT {
                return Err(Refusal::Layout(layout));
            }
            Ok(r)
        }
        /// Read `n` bytes.
        pub fn bytes(&mut self, n: usize) -> Option<&'a [u8]> {
            let out = self.buf.get(self.at..self.at.checked_add(n)?)?;
            self.at += n;
            Some(out)
        }
        /// Read a little-endian word.
        pub fn u32(&mut self) -> Option<u32> {
            let b = self.bytes(4)?;
            Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        }
        /// Read a length-prefixed field into `out`, returning its length. `None` when it would
        /// not fit, which is a malformed blob rather than a truncation.
        pub fn field(&mut self, out: &mut [u8]) -> Option<usize> {
            let n = self.u32()? as usize;
            if n > out.len() {
                return None;
            }
            out[..n].copy_from_slice(self.bytes(n)?);
            Some(n)
        }
    }

    impl LineDisc {
        /// Write this discipline's carried state (see the module docs) into `w`.
        pub fn save(&self, w: &mut Writer<'_>) {
            w.field(&self.buf[..self.len]);
            w.u32(self.cur as u32);
            w.field(&self.kill[..self.kill_len]);
            w.field(&self.prompt[..self.prompt_len]);
            w.u32(self.hist_count as u32);
            w.u32(self.hist_next as u32);
            for (line, n) in &self.hist {
                w.field(&line[..*n]);
            }
        }

        /// Replace this discipline's carried state with what `r` holds. On `None` the discipline
        /// may be half written, and the caller refuses the whole blob rather than serve from it.
        pub fn restore(&mut self, r: &mut Reader<'_>) -> Option<()> {
            self.len = r.field(&mut self.buf)?;
            let cur = r.u32()? as usize;
            if cur > self.len {
                return None;
            }
            self.cur = cur;
            self.kill_len = r.field(&mut self.kill)?;
            self.prompt_len = r.field(&mut self.prompt[..PROMPT_MAX])?;
            self.hist_count = (r.u32()? as usize).min(HIST);
            self.hist_next = (r.u32()? as usize) % HIST;
            for entry in &mut self.hist {
                let mut line = [0u8; LINE_MAX];
                let n = r.field(&mut line)?;
                *entry = (line, n);
            }
            self.browse = None;
            Some(())
        }
    }

    impl RawQueue {
        /// Write the unread raw bytes into `w`, oldest first.
        pub fn save(&self, w: &mut Writer<'_>) {
            let mut out = [0u8; RAW_QUEUE_MAX];
            for (i, b) in out.iter_mut().take(self.len).enumerate() {
                *b = self.buf[(self.head + i) % RAW_QUEUE_MAX];
            }
            w.field(&out[..self.len]);
        }

        /// Replace the queue with what `r` holds.
        pub fn restore(&mut self, r: &mut Reader<'_>) -> Option<()> {
            let mut buf = [0u8; RAW_QUEUE_MAX];
            let n = r.field(&mut buf)?;
            *self = RawQueue::new();
            self.push(&buf[..n]);
            Some(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate std;
    use std::string::String;
    use std::vec::Vec;

    /// A one-row terminal model: interprets the echo stream the way a VT does, so the tests
    /// assert **what the user sees**, not which escape sequences we happened to emit. This is
    /// what lets the engine change its redraw strategy without rewriting every test.
    struct Screen {
        row: Vec<u8>,
        col: usize,
        bells: usize,
        cleared: bool,
    }

    impl Screen {
        fn new() -> Self {
            Screen {
                row: Vec::new(),
                col: 0,
                bells: 0,
                cleared: false,
            }
        }

        /// The visible row as a string (trailing blanks trimmed).
        fn text(&self) -> String {
            let end = self
                .row
                .iter()
                .rposition(|&b| b != b' ')
                .map_or(0, |p| p + 1);
            String::from_utf8_lossy(&self.row[..end]).into_owned()
        }

        fn interpret(&mut self, bytes: &[u8]) {
            let mut i = 0;
            while i < bytes.len() {
                let b = bytes[i];
                match b {
                    0x07 => self.bells += 1,
                    0x08 => self.col = self.col.saturating_sub(1),
                    b'\r' => self.col = 0,
                    b'\n' => { /* row model: a fresh line, content untouched */ }
                    0x1b => {
                        // CSI only; that is all the engine emits.
                        assert_eq!(bytes.get(i + 1), Some(&b'['), "non-CSI escape in echo");
                        let mut j = i + 2;
                        let mut n: usize = 0;
                        let mut has_n = false;
                        while j < bytes.len() && bytes[j].is_ascii_digit() {
                            n = n * 10 + (bytes[j] - b'0') as usize;
                            has_n = true;
                            j += 1;
                        }
                        let count = if has_n { n } else { 1 };
                        match bytes[j] {
                            b'D' => self.col = self.col.saturating_sub(count),
                            b'C' => self.col += count,
                            b'K' => self.row.truncate(self.col),
                            b'J' => {
                                self.row.clear();
                                self.cleared = true;
                            }
                            b'H' => self.col = 0,
                            other => panic!("echo used unmodeled CSI final {other:?}"),
                        }
                        i = j;
                    }
                    _ => {
                        if self.col == self.row.len() {
                            self.row.push(b);
                        } else {
                            self.row[self.col] = b;
                        }
                        self.col += 1;
                    }
                }
                i += 1;
            }
        }
    }

    impl Sink for Screen {
        fn put(&mut self, bytes: &[u8]) {
            self.interpret(bytes);
        }
    }

    fn feed_all(d: &mut LineDisc, s: &mut Screen, bytes: &[u8]) -> Vec<Event> {
        bytes
            .iter()
            .map(|&b| d.feed(b, s))
            .filter(|e| *e != Event::None)
            .collect()
    }

    /// Typing echoes, Enter completes, and the completed line is exactly what was typed. The
    /// baseline everything else builds on.
    #[test]
    fn plain_typing_makes_a_line() {
        let (mut d, mut s) = (LineDisc::new(), Screen::new());
        let events = feed_all(&mut d, &mut s, b"echo hi\r");
        assert_eq!(events, [Event::Line]);
        assert_eq!(d.line(), b"echo hi");
        assert_eq!(s.text(), "echo hi");
    }

    /// **Tab is reported and changes nothing** (§227 option D): no echo, no edit, and the line in
    /// progress and its cursor are what [`LineDisc::pending`] says. A terminal that ignores the
    /// event therefore behaves exactly as it did when Tab fell into the catch-all.
    #[test]
    fn tab_is_an_event_that_leaves_the_line_alone() {
        let (mut d, mut s) = (LineDisc::new(), Screen::new());
        feed_all(&mut d, &mut s, b"ls do\x1b[D");
        let before = s.text();
        assert_eq!(feed_all(&mut d, &mut s, b"\t"), [Event::Tab]);
        assert_eq!(s.text(), before);
        assert_eq!(d.pending(), (b"ls do".as_slice(), 4));
    }

    /// **A completion inserts at the cursor as typing would**, mid-line included, and cannot
    /// smuggle a control byte into the line: an escape in the text is skipped, not inserted.
    #[test]
    fn inserted_text_lands_at_the_cursor_and_control_bytes_are_refused() {
        let (mut d, mut s) = (LineDisc::new(), Screen::new());
        feed_all(&mut d, &mut s, b"cat do x\x1b[D\x1b[D");
        d.insert_text(b"cs/\x1b[2J", &mut s);
        assert_eq!(s.text(), "cat docs/[2J x");
        assert_eq!(d.pending().1, 12);
        assert_eq!(feed_all(&mut d, &mut s, b"\r"), [Event::Line]);
        assert_eq!(d.line(), b"cat docs/[2J x");
    }

    /// A repaint after a listing redraws the prompt and the line with the cursor where it was.
    #[test]
    fn repaint_restores_prompt_line_and_cursor() {
        let (mut d, mut s) = (LineDisc::new(), Screen::new());
        d.start_line(b"$ ", &mut s);
        feed_all(&mut d, &mut s, b"ls a\x1b[D");
        let mut fresh = Screen::new();
        d.repaint(&mut fresh);
        assert_eq!(fresh.text(), "$ ls a");
        assert_eq!(fresh.col, 5);
    }

    /// Backspace erases from the screen as well as the buffer, including mid-line, where the
    /// tail must shift left visibly.
    #[test]
    fn backspace_erases_on_screen() {
        let (mut d, mut s) = (LineDisc::new(), Screen::new());
        feed_all(&mut d, &mut s, b"ekko");
        feed_all(&mut d, &mut s, &[0x7f, 0x7f, 0x7f]);
        assert_eq!(s.text(), "e");
        feed_all(&mut d, &mut s, b"cho\r");
        assert_eq!(d.line(), b"echo");
    }

    /// Cursor keys move the insertion point: arrow-left three times into "hllo", insert 'e',
    /// get "hello", and the screen agrees at every step.
    #[test]
    fn arrow_keys_edit_mid_line() {
        let (mut d, mut s) = (LineDisc::new(), Screen::new());
        feed_all(&mut d, &mut s, b"hllo");
        feed_all(&mut d, &mut s, b"\x1b[D\x1b[D\x1b[D");
        assert_eq!(feed_all(&mut d, &mut s, b"e"), []);
        assert_eq!(s.text(), "hello");
        assert_eq!(feed_all(&mut d, &mut s, b"\r"), [Event::Line]);
        assert_eq!(d.line(), b"hello");
    }

    /// SS3 arrows (application cursor mode, `ESC O D`) work exactly like CSI arrows.
    #[test]
    fn ss3_arrows_are_understood() {
        let (mut d, mut s) = (LineDisc::new(), Screen::new());
        feed_all(&mut d, &mut s, b"ab\x1bODc\r");
        assert_eq!(d.line(), b"acb");
    }

    /// Home/End in all three encodings land the cursor where they claim: type at Home and the
    /// character appears at the front.
    #[test]
    fn home_and_end_place_the_cursor() {
        for home in [&b"\x1b[H"[..], b"\x1b[1~", b"\x01"] {
            let (mut d, mut s) = (LineDisc::new(), Screen::new());
            feed_all(&mut d, &mut s, b"bc");
            feed_all(&mut d, &mut s, home);
            feed_all(&mut d, &mut s, b"a");
            feed_all(&mut d, &mut s, b"\x1b[F"); // End
            feed_all(&mut d, &mut s, b"d\r");
            assert_eq!(d.line(), b"abcd", "home encoding {home:x?}");
            assert_eq!(s.text(), "abcd");
        }
    }

    /// Delete (CSI 3~) removes under the cursor; ^D does the same on a non-empty line.
    #[test]
    fn delete_forward_both_ways() {
        let (mut d, mut s) = (LineDisc::new(), Screen::new());
        feed_all(&mut d, &mut s, b"axbc");
        feed_all(&mut d, &mut s, b"\x1b[D\x1b[D\x1b[D"); // cursor on 'x'
        feed_all(&mut d, &mut s, b"\x1b[3~");
        assert_eq!(s.text(), "abc");
        feed_all(&mut d, &mut s, b"\x04"); // ^D on 'b'
        assert_eq!(s.text(), "ac");
        feed_all(&mut d, &mut s, b"\r");
        assert_eq!(d.line(), b"ac");
    }

    /// ^K kills to end, ^Y yanks it back: cut "ef", move home, yank, and the killed text lands
    /// at the cursor, on screen and in the line.
    #[test]
    fn kill_to_end_and_yank() {
        let (mut d, mut s) = (LineDisc::new(), Screen::new());
        feed_all(&mut d, &mut s, b"abcdef");
        feed_all(&mut d, &mut s, b"\x02\x02"); // ^B x2, cursor before "ef"
        feed_all(&mut d, &mut s, b"\x0b"); // ^K
        assert_eq!(s.text(), "abcd");
        feed_all(&mut d, &mut s, b"\x01\x19"); // ^A then ^Y
        assert_eq!(s.text(), "efabcd");
        feed_all(&mut d, &mut s, b"\r");
        assert_eq!(d.line(), b"efabcd");
    }

    /// ^U kills to start; ^W kills the word before the cursor, blanks first.
    #[test]
    fn kill_to_start_and_kill_word() {
        let (mut d, mut s) = (LineDisc::new(), Screen::new());
        feed_all(&mut d, &mut s, b"rm -rf /\x15"); // ^U
        assert_eq!(s.text(), "");
        feed_all(&mut d, &mut s, b"echo one two");
        feed_all(&mut d, &mut s, b"\x17"); // ^W
        assert_eq!(s.text(), "echo one");
        feed_all(&mut d, &mut s, b"\x17\x17\r");
        assert_eq!(d.line(), b"");
    }

    /// Up arrow recalls the previous line; a second up goes older; down comes back; and
    /// browsing past the newest entry restores the line that was in progress.
    #[test]
    fn history_browsing_is_non_destructive() {
        let (mut d, mut s) = (LineDisc::new(), Screen::new());
        feed_all(&mut d, &mut s, b"first\r");
        feed_all(&mut d, &mut s, b"second\r");
        s = Screen::new();
        feed_all(&mut d, &mut s, b"in prog"); // a line in progress
        feed_all(&mut d, &mut s, b"\x1b[A");
        assert_eq!(s.text(), "second");
        feed_all(&mut d, &mut s, b"\x1b[A");
        assert_eq!(s.text(), "first");
        feed_all(&mut d, &mut s, b"\x1b[B\x1b[B");
        assert_eq!(s.text(), "in prog");
        feed_all(&mut d, &mut s, b"\r");
        assert_eq!(d.line(), b"in prog");
    }

    /// A recalled history entry can be edited and submitted, and an exact repeat of the newest
    /// entry is not recorded twice (up, up must reach the older command).
    #[test]
    fn history_recall_edits_and_dedups() {
        let (mut d, mut s) = (LineDisc::new(), Screen::new());
        feed_all(&mut d, &mut s, b"run 9\r");
        feed_all(&mut d, &mut s, b"run 9\r"); // repeat: must not duplicate
        feed_all(&mut d, &mut s, b"help\r");
        s = Screen::new();
        feed_all(&mut d, &mut s, b"\x1b[A\x1b[A"); // help, then run 9 (not help twice)
        assert_eq!(s.text(), "run 9");
        feed_all(&mut d, &mut s, &[0x7f]);
        feed_all(&mut d, &mut s, b"7\r");
        assert_eq!(d.line(), b"run 7");
    }

    /// The history ring holds [`HIST`] entries and evicts the oldest.
    #[test]
    fn history_ring_evicts_the_oldest() {
        let (mut d, mut s) = (LineDisc::new(), Screen::new());
        for i in 0..=HIST {
            feed_all(&mut d, &mut s, std::format!("cmd{i}\r").as_bytes());
        }
        s = Screen::new();
        for _ in 0..HIST + 5 {
            feed_all(&mut d, &mut s, b"\x1b[A"); // walk to the oldest, then bell
        }
        assert_eq!(s.text(), "cmd1", "cmd0 should have been evicted");
        assert!(s.bells > 0, "walking past the oldest should ring the bell");
    }

    /// ^C discards the line in progress and reports Interrupt; the next line starts clean.
    #[test]
    fn ctrl_c_discards_the_line() {
        let (mut d, mut s) = (LineDisc::new(), Screen::new());
        let events = feed_all(&mut d, &mut s, b"rm -rf\x03");
        assert_eq!(events, [Event::Interrupt]);
        let events = feed_all(&mut d, &mut s, b"ok\r");
        assert_eq!(events, [Event::Line]);
        assert_eq!(d.line(), b"ok");
    }

    /// ^D is EOF only on an empty line.
    #[test]
    fn ctrl_d_is_eof_only_when_empty() {
        let (mut d, mut s) = (LineDisc::new(), Screen::new());
        assert_eq!(feed_all(&mut d, &mut s, b"\x04"), [Event::Eof]);
        assert_eq!(feed_all(&mut d, &mut s, b"x\x04"), []);
    }

    /// CR, LF, and CRLF each end exactly one line: piped scripts (LF), terminals (CR), and
    /// CRLF senders all behave.
    #[test]
    fn line_endings_cr_lf_crlf() {
        let (mut d, mut s) = (LineDisc::new(), Screen::new());
        let events = feed_all(&mut d, &mut s, b"a\rb\nc\r\nd\r");
        assert_eq!(events.len(), 4, "CRLF must not count as two Enters");
        assert_eq!(d.line(), b"d");
    }

    /// An overlong line refuses further input with a bell instead of truncating silently or
    /// walking off the buffer.
    #[test]
    fn overflow_rings_the_bell() {
        let (mut d, mut s) = (LineDisc::new(), Screen::new());
        for _ in 0..LINE_MAX + 3 {
            d.feed(b'a', &mut s);
        }
        assert_eq!(s.bells, 3);
        feed_all(&mut d, &mut s, b"\r");
        assert_eq!(d.line().len(), LINE_MAX);
    }

    /// ^L repaints the prompt and the line after clearing, with the cursor where it was:
    /// typing continues mid-line.
    #[test]
    fn ctrl_l_repaints_prompt_and_line() {
        let (mut d, mut s) = (LineDisc::new(), Screen::new());
        d.start_line(b"$ ", &mut s);
        feed_all(&mut d, &mut s, b"ab\x1b[D"); // cursor between a and b
        feed_all(&mut d, &mut s, b"\x0c");
        assert!(s.cleared);
        assert_eq!(s.text(), "$ ab");
        feed_all(&mut d, &mut s, b"x");
        assert_eq!(
            s.text(),
            "$ axb",
            "typing after ^L must land mid-line on screen"
        );
        feed_all(&mut d, &mut s, b"\r");
        assert_eq!(d.line(), b"axb");
    }

    /// `start_line` paints the prompt and any type-ahead, and the completed line includes the
    /// type-ahead: bytes that arrived before the read are not lost.
    #[test]
    fn start_line_shows_type_ahead() {
        let (mut d, mut s) = (LineDisc::new(), Screen::new());
        feed_all(&mut d, &mut s, b"early");
        let mut s2 = Screen::new();
        d.start_line(b"$ ", &mut s2);
        assert_eq!(s2.text(), "$ early");
        feed_all(&mut d, &mut s2, b"\r");
        assert_eq!(d.line(), b"early");
    }

    /// **A read re-issued after `FLAG_RETRY` paints nothing and loses nothing.** The half-typed
    /// line survives, the prompt is remembered for later repaints, and finishing the line returns
    /// all of it: the promise `FLAG_RETRY` makes to a reader.
    #[test]
    fn a_resumed_read_keeps_the_half_typed_line_and_paints_nothing() {
        let (mut d, mut s) = (LineDisc::new(), Screen::new());
        d.start_line(b"$ ", &mut s);
        feed_all(&mut d, &mut s, b"ech");
        let mut quiet = Screen::new();
        d.resume_line(b"$ ");
        assert_eq!(
            quiet.text(),
            "",
            "a resumed read must not repaint the prompt"
        );
        feed_all(&mut d, &mut quiet, b"o\r");
        assert_eq!(d.line(), b"echo");
    }

    /// `FLAG_RETRY` is distinguishable from every other read reply, in both shapes: a line reply
    /// of length zero with no flags is an empty line, and a raw reply always carries 1..=8 bytes.
    #[test]
    fn a_retry_is_never_mistaken_for_data() {
        assert!(proto::is_retry(0, proto::FLAG_RETRY));
        assert!(!proto::is_retry(0, 0), "an empty line is not a retry");
        assert!(!proto::is_retry(0, proto::FLAG_INTERRUPTED));
        assert!(
            !proto::is_retry(1, proto::FLAG_RETRY),
            "a one-byte raw read of 0x04 is data"
        );
        assert_eq!(
            proto::FLAG_RETRY & (proto::FLAG_EOF | proto::FLAG_INTERRUPTED),
            0
        );
        assert_eq!(proto::OP_QUIESCE, 8);
    }

    /// **A handoff round trip carries the half-typed line, the cursor, the kill buffer and the
    /// history**, so the replacement's next keystrokes land exactly where the old one's would have:
    /// finishing the line returns all of it, and Up recalls a line typed before the swap.
    #[test]
    fn a_handed_off_discipline_carries_on_where_the_old_one_stopped() {
        let (mut old, mut s) = (LineDisc::new(), Screen::new());
        // A full ring of long lines, so the blob is the size a real terminal's is.
        // Distinct lines, because the ring skips an exact repeat of its newest entry.
        for i in 0..HIST - 1 {
            let long = [b'a' + i as u8; LINE_MAX - 8];
            old.start_line(b"$ ", &mut s);
            feed_all(&mut old, &mut s, &long);
            feed_all(&mut old, &mut s, b"\r");
        }
        old.start_line(b"$ ", &mut s);
        feed_all(&mut old, &mut s, b"first\r");
        old.start_line(b"$ ", &mut s);
        feed_all(&mut old, &mut s, b"ec");

        let mut page = [0u8; 8192];
        let mut w = handoff::Writer::new(&mut page);
        old.save(&mut w);
        let n = w.finish().expect("a whole history fits");
        // Measured, correcting an estimate this lane recorded on 2026-09-26: a full ring of
        // near-maximal lines is under 2 KiB, so the whole terminal (with its four queued lines) fits
        // in one page. The estimate had counted buffers the blob does not carry.
        assert!(n < 2048, "the discipline's blob grew to {n} bytes");

        let mut new = LineDisc::new();
        let mut r = handoff::Reader::new(&page[..n]).unwrap();
        new.restore(&mut r).unwrap();
        new.resume_line(b"$ ");
        let mut s2 = Screen::new();
        feed_all(&mut new, &mut s2, b"ho\r");
        assert_eq!(
            new.line(),
            b"echo",
            "the half-typed line did not survive the swap"
        );
        new.start_line(b"$ ", &mut s2);
        feed_all(&mut new, &mut s2, b"\x1b[A\x1b[A\r");
        assert_eq!(
            new.line(),
            b"first",
            "history typed before the swap is not recallable after it"
        );
    }

    /// **A blob this build cannot read is refused whole, with the reason**, which is what lets a
    /// supervisor not commit a swap to it.
    #[test]
    fn a_blob_in_another_layout_or_none_at_all_is_refused() {
        assert_eq!(
            handoff::Reader::new(&[0u8; 16]).err(),
            Some(handoff::Refusal::NoBlob)
        );
        let mut page = [0u8; 16];
        page[..4].copy_from_slice(&handoff::MAGIC.to_le_bytes());
        page[4..8].copy_from_slice(&2u32.to_le_bytes());
        assert_eq!(
            handoff::Reader::new(&page).err(),
            Some(handoff::Refusal::Layout(2))
        );
        let mut w = handoff::Writer::new(&mut page);
        w.field(&[1; 64]);
        assert_eq!(
            w.finish(),
            None,
            "a blob that does not fit must say so, not truncate"
        );
    }

    /// Unknown CSI sequences (F-keys and friends) are swallowed whole: no stray bytes appear
    /// in the line or on the screen.
    #[test]
    fn unknown_sequences_are_swallowed() {
        let (mut d, mut s) = (LineDisc::new(), Screen::new());
        feed_all(&mut d, &mut s, b"a\x1b[15~b\x1b[Zc\r"); // F5, shift-tab
        assert_eq!(d.line(), b"abc");
        assert_eq!(s.text(), "abc");
    }

    /// Output expansion: every bare `\n` becomes `\r\n`, existing `\r\n` is untouched (the
    /// engine never emits `\r\r\n`), and ANSI passes through.
    #[test]
    fn output_expansion_translates_newlines() {
        struct Buf(Vec<u8>);
        impl Sink for Buf {
            fn put(&mut self, b: &[u8]) {
                self.0.extend_from_slice(b);
            }
        }
        let mut out = Buf(Vec::new());
        expand_output(b"a\nb\x1b[1mc\n", &mut out);
        assert_eq!(out.0, b"a\r\nb\x1b[1mc\r\n");
    }

    /// The proto word packing round-trips, and the reserved bits stay zero.
    #[test]
    fn proto_words_round_trip() {
        let w = proto::req(proto::OP_READLINE, 42);
        assert_eq!(proto::op(w), proto::OP_READLINE);
        assert_eq!(proto::len(w), 42);
        assert_eq!(w & 0x00ff_ffff_0000_0000, 0);
        // The reply flags are wire ABI shared with every client: pin the values, because a
        // drifted FLAG_INTERRUPTED makes an interrupted read look like a normal empty line.
        assert_eq!(proto::FLAG_EOF, 1);
        assert_eq!(proto::FLAG_INTERRUPTED, 2);
        // Pinned for the same reason: OP_RAWMODE and OP_READRAW are wire ABI too, and a drifted
        // opcode number would silently reassign OP_PRINT's or a future opcode's traffic.
        assert_eq!(proto::OP_RAWMODE, 6);
        assert_eq!(proto::OP_READRAW, 7);
    }

    /// `abandon` clears the buffer, cursor, and history browsing, but leaves history and the kill
    /// buffer alone: a mode switch mid-line must not erase what earlier lines put there.
    #[test]
    fn abandon_clears_the_line_not_the_history() {
        let (mut d, mut s) = (LineDisc::new(), Screen::new());
        feed_all(&mut d, &mut s, b"first\r"); // one real history entry
        feed_all(&mut d, &mut s, b"x\x02\x0b"); // type 'x', ^B onto it, ^K kills it to end
        feed_all(&mut d, &mut s, b"\x1b[A"); // browse into history
        d.abandon();
        assert_eq!(
            d.line(),
            b"first",
            "abandon must not touch the last completed line"
        );
        // The kill buffer survives: yanking it after abandon still works.
        s = Screen::new();
        feed_all(&mut d, &mut s, b"\x19");
        assert_eq!(s.text(), "x", "abandon must not clear the kill buffer");
        // History survives: up-arrow still reaches the recorded line.
        s = Screen::new();
        feed_all(&mut d, &mut s, b"\x15"); // ^U: clear the yanked "x"
        feed_all(&mut d, &mut s, b"\x1b[A");
        assert_eq!(s.text(), "first", "abandon must not clear history");
    }

    /// [`RawQueue`] delivers bytes in order, packs up to 8 per pop, and drains to empty: the
    /// primitive `OP_READRAW` is built on, proven with no IPC involved.
    #[test]
    fn raw_queue_pops_in_order_up_to_eight() {
        let mut q = RawQueue::new();
        assert!(q.is_empty());
        assert_eq!(q.push(b"hello, kilo!"), 12); // 12 bytes, under the cap
        assert!(!q.is_empty());

        let (n, packed) = q.pop8().unwrap();
        assert_eq!(n, 8);
        let first8: [u8; 8] = packed.to_le_bytes();
        assert_eq!(&first8, b"hello, k");

        let (n, packed) = q.pop8().unwrap();
        assert_eq!(n, 4);
        assert_eq!(&packed.to_le_bytes()[..4], b"ilo!");
        assert!(q.is_empty());
        assert!(q.pop8().is_none());
    }

    /// A burst past [`RAW_QUEUE_MAX`] is truncated: `push` reports how many actually landed, so
    /// the caller (which owns the [`Sink`]) knows to ring the bell for the rest, exactly the
    /// `LineDisc` overflow contract one level over.
    #[test]
    fn raw_queue_overflow_is_reported_not_grown() {
        let mut q = RawQueue::new();
        assert_eq!(q.push(&[b'a'; RAW_QUEUE_MAX]), RAW_QUEUE_MAX);
        assert_eq!(q.push(b"more"), 0, "a full queue accepts nothing further");
        // Draining still returns exactly what was pushed, byte for byte.
        let mut drained = 0;
        while let Some((n, _)) = q.pop8() {
            drained += n;
        }
        assert_eq!(drained, RAW_QUEUE_MAX);
    }

    /// The queue wraps its ring correctly: push past the wrap point, pop some, push more, and the
    /// bytes must still come out in the order they went in.
    #[test]
    fn raw_queue_wraps_the_ring_in_order() {
        let mut q = RawQueue::new();
        q.push(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        let (n, packed) = q.pop8().unwrap(); // drain the first 8, head now at index 8
        assert_eq!(n, 8);
        assert_eq!(packed.to_le_bytes(), [1, 2, 3, 4, 5, 6, 7, 8]);
        q.push(&[11, 12, 13]); // wraps past RAW_QUEUE_MAX
        let mut out = std::vec::Vec::new();
        while let Some((n, packed)) = q.pop8() {
            out.extend_from_slice(&packed.to_le_bytes()[..n]);
        }
        assert_eq!(out, [9, 10, 11, 12, 13]);
    }

    /// The control-character encodings of movement (^B ^A ^E ^F), which the CSI tests do not
    /// cover, including ^B at column 0 where there is nowhere to go.
    #[test]
    fn control_key_movement() {
        let (mut d, mut s) = (LineDisc::new(), Screen::new());
        feed_all(&mut d, &mut s, b"\x02"); // ^B on an empty line: nothing to do
        feed_all(&mut d, &mut s, b"ab");
        feed_all(&mut d, &mut s, b"\x01\x06x"); // ^A, ^F right one, insert mid-line
        assert_eq!(s.text(), "axb");
        feed_all(&mut d, &mut s, b"\x05c\r"); // ^E, append
        assert_eq!(d.line(), b"axbc");
        assert_eq!(s.text(), "axbc");
    }

    /// CSI C moves right mid-line and is a no-op at the end of the line; typing after each
    /// proves where the cursor actually is (the echo alone would not).
    #[test]
    fn right_arrow_moves_and_stops_at_end() {
        let (mut d, mut s) = (LineDisc::new(), Screen::new());
        feed_all(&mut d, &mut s, b"ab\x1b[D\x1b[D"); // cursor to column 0
        feed_all(&mut d, &mut s, b"\x1b[Cx"); // right one, insert between a and b
        assert_eq!(s.text(), "axb");
        feed_all(&mut d, &mut s, b"\x1b[C\x1b[C"); // to the end, then once more (no-op)
        feed_all(&mut d, &mut s, b"c\r");
        assert_eq!(d.line(), b"axbc");
    }

    /// A CSI with a ';' (shift-Delete, `ESC [ 3 ; 2 ~`) still dispatches on the first
    /// parameter: the ';' advances to the second slot instead of aborting the sequence.
    #[test]
    fn csi_second_parameter_is_parsed() {
        let (mut d, mut s) = (LineDisc::new(), Screen::new());
        feed_all(&mut d, &mut s, b"abc\x01"); // ^A, cursor on 'a'
        feed_all(&mut d, &mut s, b"\x1b[3;2~"); // delete under cursor, modifier ignored
        assert_eq!(s.text(), "bc");
        feed_all(&mut d, &mut s, b"\r");
        assert_eq!(d.line(), b"bc");
    }

    /// Type-ahead with the cursor moved off the end: `start_line` repaints and puts the
    /// cursor back where it was, so the next insertion lands mid-line on screen too.
    #[test]
    fn start_line_restores_mid_line_cursor() {
        let (mut d, mut s) = (LineDisc::new(), Screen::new());
        feed_all(&mut d, &mut s, b"ab\x02"); // type ahead, cursor between a and b
        let mut s2 = Screen::new();
        d.start_line(b"$ ", &mut s2);
        feed_all(&mut d, &mut s2, b"X");
        assert_eq!(s2.text(), "$ aXb");
        feed_all(&mut d, &mut s2, b"\r");
        assert_eq!(d.line(), b"aXb");
    }

    /// Enter on an empty line completes an empty read but records nothing: up must recall
    /// the last real command, not a blank.
    #[test]
    fn empty_lines_stay_out_of_history() {
        let (mut d, mut s) = (LineDisc::new(), Screen::new());
        feed_all(&mut d, &mut s, b"one\r\r");
        s = Screen::new();
        feed_all(&mut d, &mut s, b"\x1b[A");
        assert_eq!(s.text(), "one");
    }

    /// An immediate repeat is stored once: with "a" entered twice, the second up-arrow has
    /// nowhere older to go and rings the bell. The screen alone cannot tell one stored copy
    /// from two, which is why the bell count is the assertion here.
    #[test]
    fn duplicate_entry_is_stored_once() {
        let (mut d, mut s) = (LineDisc::new(), Screen::new());
        feed_all(&mut d, &mut s, b"a\ra\r");
        s = Screen::new();
        feed_all(&mut d, &mut s, b"\x1b[A\x1b[A");
        assert_eq!(s.text(), "a");
        assert_eq!(s.bells, 1, "one stored entry: the second up must bell");
    }

    /// Walk a wrapped ring end to end in both directions, asserting every entry: ten
    /// commands in a ring of eight, plus a deduplicated repeat across the wrap. The
    /// existing browsing tests never cross the wrap point, which is where the ring index
    /// arithmetic can be wrong without them noticing.
    #[test]
    fn wrapped_ring_walks_correctly_both_ways() {
        let (mut d, mut s) = (LineDisc::new(), Screen::new());
        for i in 0..10 {
            feed_all(&mut d, &mut s, std::format!("cmd{i}\r").as_bytes());
        }
        feed_all(&mut d, &mut s, b"cmd9\r"); // repeat of the newest, post-wrap: not stored
        s = Screen::new();
        for i in (2..=9).rev() {
            feed_all(&mut d, &mut s, b"\x1b[A");
            assert_eq!(s.text(), std::format!("cmd{i}"), "up should reach cmd{i}");
        }
        feed_all(&mut d, &mut s, b"\x1b[A"); // past the oldest
        assert_eq!(s.bells, 1);
        assert_eq!(s.text(), "cmd2");
        for i in 3..=9 {
            feed_all(&mut d, &mut s, b"\x1b[B");
            assert_eq!(s.text(), std::format!("cmd{i}"), "down should reach cmd{i}");
        }
        feed_all(&mut d, &mut s, b"\x1b[B"); // past the newest: the (empty) line in progress
        assert_eq!(s.text(), "");
    }

    /// Browsing away and back restores not just the in-progress text but the cursor column:
    /// typing after the round trip lands where the cursor was left. The buffer restore is
    /// checked elsewhere; this pins the on-screen column, which only the echo carries.
    #[test]
    fn stash_restores_cursor_column() {
        let (mut d, mut s) = (LineDisc::new(), Screen::new());
        feed_all(&mut d, &mut s, b"old\r");
        s = Screen::new();
        feed_all(&mut d, &mut s, b"hello\x02\x02"); // cursor after "hel"
        feed_all(&mut d, &mut s, b"\x1b[A\x1b[B"); // up to "old", back down
        feed_all(&mut d, &mut s, b"X");
        assert_eq!(s.text(), "helXlo");
        feed_all(&mut d, &mut s, b"\r");
        assert_eq!(d.line(), b"helXlo");
    }

    /// A yank that does not fit is truncated to the room left: the line ends exactly full,
    /// silently (the bell is reserved for a yank with no room at all).
    #[test]
    fn yank_truncates_to_the_room_left() {
        let (mut d, mut s) = (LineDisc::new(), Screen::new());
        for _ in 0..250 {
            d.feed(b'a', &mut s);
        }
        feed_all(&mut d, &mut s, b"\x15"); // ^U: kill all 250
        for _ in 0..10 {
            d.feed(b'b', &mut s);
        }
        feed_all(&mut d, &mut s, b"\x19\r"); // ^Y: only 246 of the 250 fit
        assert_eq!(s.bells, 0);
        assert_eq!(d.line().len(), LINE_MAX);
        assert_eq!(&d.line()[..10], b"bbbbbbbbbb");
        assert!(d.line()[10..].iter().all(|&b| b == b'a'));
    }

    /// ^Y with nothing ever killed is silent; ^Y with a kill buffer but a full line rings
    /// the bell (there was something to yank and nowhere to put it). Both sides matter: the
    /// distinction is the condition, not just the bell.
    #[test]
    fn yank_bell_needs_a_kill_and_a_full_line() {
        let (mut d, mut s) = (LineDisc::new(), Screen::new());
        feed_all(&mut d, &mut s, b"\x19"); // empty kill buffer: nothing, no bell
        assert_eq!(s.bells, 0);
        feed_all(&mut d, &mut s, b"x\x15"); // kill buffer now holds "x"
        for _ in 0..LINE_MAX {
            d.feed(b'a', &mut s);
        }
        feed_all(&mut d, &mut s, b"\x19"); // no room at all
        assert_eq!(s.bells, 1);
    }

    /// Yank into the middle of a line, then keep typing: the tail stays put on screen
    /// through both the yank repaint and the next insertion.
    #[test]
    fn yank_mid_line_keeps_the_tail_in_place() {
        let (mut d, mut s) = (LineDisc::new(), Screen::new());
        feed_all(&mut d, &mut s, b"abcdef\x02\x02\x0b"); // ^K kills "ef"
        feed_all(&mut d, &mut s, b"\x02\x02\x19"); // yank it between "ab" and "cd"
        assert_eq!(s.text(), "abefcd");
        feed_all(&mut d, &mut s, b"X");
        assert_eq!(s.text(), "abefXcd");
        feed_all(&mut d, &mut s, b"\r");
        assert_eq!(d.line(), b"abefXcd");
    }

    /// ^W with only blanks before the cursor stops cleanly at column 0 (the blank-skipping
    /// loop must not walk past the front of the buffer).
    #[test]
    fn word_kill_over_leading_blanks() {
        let (mut d, mut s) = (LineDisc::new(), Screen::new());
        feed_all(&mut d, &mut s, b"  \x17\r");
        assert_eq!(d.line(), b"");
        assert_eq!(s.text(), "");
    }

    /// ^W then ^Y round-trips exactly the killed word: the kill length is the word's, not
    /// the cursor position's.
    #[test]
    fn word_kill_yank_round_trip() {
        let (mut d, mut s) = (LineDisc::new(), Screen::new());
        feed_all(&mut d, &mut s, b"ab cd\x17\x19\r");
        assert_eq!(d.line(), b"ab cd");
    }

    /// Home from column 12 needs a two-digit CSI count; typing at the front proves the
    /// cursor really travelled all twelve columns. Every other test moves fewer than ten,
    /// so the multi-digit encoding path was unexercised.
    #[test]
    fn multi_digit_cursor_move() {
        let (mut d, mut s) = (LineDisc::new(), Screen::new());
        feed_all(&mut d, &mut s, b"abcdefghijkl\x01X");
        assert_eq!(s.text(), "Xabcdefghijkl");
        feed_all(&mut d, &mut s, b"\r");
        assert_eq!(d.line(), b"Xabcdefghijkl");
    }

    // ---- the survivors of the 2026-10-03 mutation census ----

    /// A sink that keeps the exact bytes, which the `Screen` model deliberately does not: it reads
    /// `ESC [ D` and `ESC [ 1 D` alike.
    struct Raw(Vec<u8>);
    impl Sink for Raw {
        fn put(&mut self, b: &[u8]) {
            self.0.extend_from_slice(b);
        }
    }

    /// A move of one column elides its count, and a move of ten writes both digits.
    #[test]
    fn a_cursor_move_of_one_elides_its_count_and_ten_spells_it() {
        let mut o = Raw(Vec::new());
        csi_left(&mut o, 1);
        csi_right(&mut o, 1);
        csi_left(&mut o, 0);
        csi_right(&mut o, 10);
        assert_eq!(o.0, b"\x1b[D\x1b[C\x1b[10C");
    }

    /// The two start bits are the two lowest bits, one each.
    #[test]
    fn the_two_start_bits_are_distinct_low_bits() {
        assert_eq!(proto::START_HANDOFF, 1);
        assert_eq!(proto::START_ABSORB, 2);
    }

    /// **Resuming a read paints nothing** and still remembers the prompt, so a later repaint (after
    /// a completion listing) prints it. A resume that forgot the prompt would repaint a bare line.
    #[test]
    fn resuming_a_read_remembers_the_prompt_and_paints_nothing() {
        let mut d = LineDisc::new();
        let mut o = Raw(Vec::new());
        d.resume_line(b"$ ");
        assert!(o.0.is_empty());
        d.repaint(&mut o);
        assert_eq!(o.0, b"$ ");
    }

    /// A repaint puts the cursor back where it was: the distance left of the end is `len - cur`,
    /// nothing at the end of the line and the whole line at column 0.
    #[test]
    fn a_repaint_leaves_the_cursor_where_it_was() {
        let (mut d, mut s) = (LineDisc::new(), Screen::new());
        feed_all(&mut d, &mut s, b"abcde\x1b[D\x1b[D");
        let mut o = Raw(Vec::new());
        d.repaint(&mut o);
        assert_eq!(o.0, b"abcde\x1b[2D");
        feed_all(&mut d, &mut s, b"\x05"); // ^E: to the end
        let mut o = Raw(Vec::new());
        d.repaint(&mut o);
        assert_eq!(o.0, b"abcde");
        feed_all(&mut d, &mut s, b"\x01"); // ^A: to the start
        let mut o = Raw(Vec::new());
        d.repaint(&mut o);
        assert_eq!(o.0, b"abcde\x1b[5D");
    }

    /// A blob that exactly fills its buffer is a blob, and one byte short is refused: the header
    /// alone is eight bytes.
    #[test]
    fn a_blob_may_fill_its_buffer_exactly_and_no_more() {
        let mut exact = [0u8; 8];
        assert_eq!(handoff::Writer::new(&mut exact).finish(), Some(8));
        let mut short = [0u8; 7];
        assert_eq!(handoff::Writer::new(&mut short).finish(), None);
        let mut page = [0u8; 12];
        let mut w = handoff::Writer::new(&mut page);
        w.field(&[]);
        assert_eq!(w.finish(), Some(12), "the length word fills the last four");
    }

    /// A field is refused when longer than the buffer it lands in and accepted when exactly as long.
    #[test]
    fn a_field_exactly_as_long_as_its_buffer_fits_and_one_longer_is_malformed() {
        let mut page = [0u8; 64];
        let mut w = handoff::Writer::new(&mut page);
        w.field(&[7; 5]);
        w.field(&[8; 5]);
        let n = w.finish().unwrap();
        let mut r = handoff::Reader::new(&page[..n]).unwrap();
        let mut five = [0u8; 5];
        assert_eq!(r.field(&mut five), Some(5));
        assert_eq!(five, [7; 5]);
        let mut four = [0u8; 4];
        assert_eq!(r.field(&mut four), None);
    }

    /// **The cursor a blob carries cannot lie past the line**: a cursor on the last byte or past
    /// it is accepted only up to the line's length, and the history's write position survives a
    /// swap, so Up recalls the newest line and not whichever sits at index 0.
    #[test]
    fn a_restored_cursor_stays_inside_its_line_and_history_keeps_its_place() {
        let (mut old, mut s) = (LineDisc::new(), Screen::new());
        for line in [&b"one\r"[..], b"two\r", b"three\r"] {
            old.start_line(b"$ ", &mut s);
            feed_all(&mut old, &mut s, line);
        }
        old.start_line(b"$ ", &mut s);
        feed_all(&mut old, &mut s, b"abc\x1b[D");
        let mut page = [0u8; 4096];
        let mut w = handoff::Writer::new(&mut page);
        old.save(&mut w);
        let n = w.finish().unwrap();

        let mut new = LineDisc::new();
        let mut r = handoff::Reader::new(&page[..n]).unwrap();
        assert_eq!(new.restore(&mut r), Some(()));
        assert_eq!(new.pending(), (b"abc".as_slice(), 2));
        feed_all(&mut new, &mut s, b"\x15\x1b[A"); // ^U, then Up
        assert_eq!(new.pending().0, b"three");

        // The same blob with its cursor word moved past the line: refused whole. The header is 8
        // bytes, the line field 4 + 3, so the cursor word sits at 15.
        let mut bad = page;
        bad[15..19].copy_from_slice(&4u32.to_le_bytes());
        let mut r = handoff::Reader::new(&bad[..n]).unwrap();
        assert_eq!(LineDisc::new().restore(&mut r), None);
        // And a cursor on the line's own end is not past it.
        bad[15..19].copy_from_slice(&3u32.to_le_bytes());
        let mut r = handoff::Reader::new(&bad[..n]).unwrap();
        assert_eq!(LineDisc::new().restore(&mut r), Some(()));
    }

    /// **Unread raw bytes cross a swap in order**, including when the ring has wrapped, and an
    /// empty queue stays empty.
    #[test]
    fn the_raw_queue_crosses_a_swap_in_order_across_the_wrap() {
        let mut q = RawQueue::new();
        let first: Vec<u8> = (0..60).collect();
        q.push(&first);
        while q.len > 4 {
            q.pop8();
        }
        let more: Vec<u8> = (100..110).collect();
        q.push(&more); // the ring wraps: head is near the end
        assert!(q.head + q.len > RAW_QUEUE_MAX);

        let mut page = [0u8; 256];
        let mut w = handoff::Writer::new(&mut page);
        q.save(&mut w);
        let n = w.finish().unwrap();
        let mut copy = RawQueue::new();
        let mut r = handoff::Reader::new(&page[..n]).unwrap();
        assert_eq!(copy.restore(&mut r), Some(()));

        let drain = |q: &mut RawQueue| {
            let mut out = Vec::new();
            while let Some((n, packed)) = q.pop8() {
                out.extend_from_slice(&packed.to_le_bytes()[..n]);
            }
            out
        };
        let mut expect: Vec<u8> = (56..60).collect();
        expect.extend(100..110);
        assert_eq!(drain(&mut q), expect);
        assert_eq!(drain(&mut copy), expect);

        let mut w = handoff::Writer::new(&mut page);
        RawQueue::new().save(&mut w);
        let n = w.finish().unwrap();
        let mut empty = RawQueue::new();
        empty.push(b"x");
        let mut r = handoff::Reader::new(&page[..n]).unwrap();
        assert_eq!(empty.restore(&mut r), Some(()));
        assert!(empty.is_empty());
    }
}
