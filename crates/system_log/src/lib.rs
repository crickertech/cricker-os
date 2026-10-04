//! **The system log, as logic**: milestone 613 (a system log service: the in-memory half), the
//! service §242 (a system log) ruled.
//!
//! One service that programs append to and readers read from. This crate is all of it except the
//! syscalls, which `components/src/system_log.rs` wraps around [`Log`]:
//!
//! 1. **Stamping.** A message arrives with the badge the kernel read off the writer's capability.
//!    The service looks the badge up in what the spawner registered ([`registry`]) and fills `seq`,
//!    `time`, `program`, `user` and `severity` itself. The writer supplies text and, optionally, a
//!    level; it never supplies its own name, user or time.
//! 2. **Lines.** A byte-sink writer sends sixteen bytes at a time. [`assembler`] joins one writer's
//!    chunks into lines, at most [`record::TEXT_MAX`] bytes, so two writers interleaving chunk by
//!    chunk still produce whole lines each.
//! 3. **F3 to JSONL.** A finished line becomes an F3 [`record::Header`] plus text, and [`json`]
//!    renders that as one JSON object per line, which is what is stored and what a reader gets
//!    (§242 Question 3 as ruled: F3 is the transport, JSONL the stored and read form).
//! 4. **The ring** ([`ring`]): 64 KiB, oldest out, read by sequence-number cursor. A reader whose
//!    cursor fell out of the ring gets one dropped-count line for the gap. A per-user read filters
//!    the same ring by the stamped user; there is no per-user store.
//!
//! # EXAMPLES
//!
//! Two writers, interleaved chunk by chunk, and a system reader:
//!
//! ```
//! use system_log::{Log, Handled};
//! use system_log_protocol::{control, read};
//! use byte_sink_protocol::pack;
//!
//! let mut log = Log::new();
//! for (badge, program) in [(1u32, &b"alpha"[..]), (2, &b"beta"[..])] {
//!     for m in control::name_messages(badge, control::PROGRAM, program).into_iter().flatten() {
//!         log.handle(0, m.0, m.1, m.2, 0);
//!     }
//! }
//! log.handle(0, control::word(control::OP_READER, control::SCOPE_SYSTEM, 0, 9), 0, 0, 0);
//!
//! let (a, b) = (b"first half, then", b"one\n");
//! let (w0, w1, w2, _) = pack(a);
//! log.handle(1, w0, w1, w2, 10);
//! let (w0, w1, w2, _) = pack(b"hello from beta\n");
//! log.handle(2, w0, w1, w2, 20);
//! let (w0, w1, w2, _) = pack(b);
//! log.handle(1, w0, w1, w2, 30);
//!
//! let mut window = [0u8; read::WINDOW_BYTES];
//! let asked = log.handle(9, read::request(), 0, 0, 40);
//! assert_eq!(asked, Handled::Read { window: 0, cursor: 0 });
//! log.fill(9, 0, &mut window);
//! let (next, len, status) = read::header(&window);
//! let text = core::str::from_utf8(&window[read::DATA..read::DATA + len]).unwrap();
//! assert_eq!(status, read::status::OK);
//! assert_eq!(next, 2);
//! let mut lines = text.lines();
//! let beta = lines.next().unwrap();
//! let alpha = lines.next().unwrap();
//! assert!(beta.starts_with(r#"{"seq":0,"time":20,"source":2,"program":"beta","user":null,"#));
//! assert!(alpha.ends_with(r#""severity":"info","msg":"first half, thenone"}"#));
//! ```
//!
//! # BUGS
//!
//! - **A dropped-count line counts every user's records.** A per-user reader whose cursor fell out
//!   of the ring is told how many records were lost, and that number includes other users'
//!   records it would never have been shown. It leaks how busy the machine was, not what anybody
//!   wrote. Fixing it means keeping a per-user count of what was evicted, which is per-user state
//!   §242 ruled the ring should not need.
//! - **A line is stamped when it completes, not when its first byte arrived.** `time` is the
//!   moment the newline (or the cut, or the end of stream) reached the service. For a writer that
//!   prints a line in one go the difference is the sixteen-byte chunking and nothing more.
//! - **Eight writers can be mid-line at once.** A ninth writer's first chunk cuts the line of the
//!   writer that wrote least recently, which then appears as two records (the first flagged
//!   `"cut":true`). Lines are not lost; long-quiet partial lines are split.
//! - **No writer has a quota, and a writer picks its own severity.** The ring (`ring::BYTES`,
//!   64 KiB) evicts oldest-first across every writer, so one badge writing fast enough pushes every
//!   other writer's records out before a reader drains them, and readers see only the dropped
//!   count. Severity is the writer's framing bits, so any writer can mark a line `emerg`. Both are
//!   what `syslog(3)` allows a client too; what the badge guarantees is `program` and `user`, which
//!   a writer cannot choose (the registry is written from badge 0 only, and the kernel refuses to
//!   re-badge). Recorded by the 2026-10-03 security audit; a per-writer share of the ring is the
//!   per-user state §242 ruled the ring should not need, and no second domain writes here yet.
//! - **The intake endpoint must receive nothing the kernel writes.** [`Log::handle`] reads badge
//!   `0` as the spawner and any other badge as a writer, and the kernel's own deliveries carry
//!   whatever the mailbox's fourth word holds: `0` for an interrupt signal and a bound
//!   notification, so either reads as a control word from the spawner, and the fault address for
//!   a death message under §26 (the fault endpoint: thread death becomes a message a supervisor
//!   holds), which reads as bytes from a writer whose badge is that address. So the
//!   intake is never a thread's fault endpoint, never bound to a notification and never an
//!   interrupt's rendezvous. The spawner today (the kernel test) does none of these; milestone 342
//!   (the kernel and the `console` server drive one UART from two address spaces) is the first
//!   to wire the service in and inherits the rule (the 2026-10-03 security audit's follow-up).
//!
//! Name: provisional (milestone 613's lane, 2026-10-02 UTC), the noun §242 uses for the service.

#![no_std]

pub mod assembler;
pub mod json;
pub mod registry;
pub mod ring;

use assembler::{Assembler, Line};
use byte_sink_protocol::{INLINE_MAX, Msg, unpack};
use registry::{Name, Registry};
use ring::Ring;
pub use system_log_protocol::record;
use system_log_protocol::record::Header;
use system_log_protocol::{control, read, severity};

/// What [`Log::handle`] asks its caller to do next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Handled {
    /// Nothing: the message was stored, registered, or refused and counted.
    Nothing,
    /// A registered reader asked for records from `cursor`. The caller fills the reader's
    /// `window` with [`Log::fill`] and then signals that window's notification.
    Read {
        /// Which reader window.
        window: u8,
        /// The cursor the reader passed.
        cursor: u64,
    },
    /// A read from a badge that is not a registered reader. Nothing to fill and nobody to signal.
    Refused,
}

/// The whole service's state. `const`-constructible so a `no_std` program holds it in a `static`.
pub struct Log {
    registry: Registry,
    assembler: Assembler,
    store: Store,
    /// Messages refused: malformed byte-sink words, a body kind other than text, or control from a
    /// badge that is not 0. Counted rather than stored, and exposed for the service's tests.
    pub refused: u64,
}

impl Default for Log {
    fn default() -> Self {
        Self::new()
    }
}

impl Log {
    /// An empty log.
    pub const fn new() -> Self {
        Log {
            registry: Registry::new(),
            assembler: Assembler::new(),
            store: Store {
                ring: Ring::new(),
                next_seq: 0,
                scratch: [0; json::LINE_MAX],
            },
            refused: 0,
        }
    }

    /// **Handle one received message**: the three words a `SEND` carried, the badge the kernel
    /// delivered beside them, and the time it arrived.
    ///
    /// Badge 0 is the spawner, which holds the one unbadged capability: its messages are
    /// [`control`] words. Any other badge is a writer's bytes or a reader's request.
    pub fn handle(&mut self, badge: u64, w0: u64, w1: u64, w2: u64, now: u64) -> Handled {
        if badge == 0 {
            self.control(w0, w1, w2);
            return Handled::Nothing;
        }
        let op = w0 >> byte_sink_protocol::OP_SHIFT;
        if op == read::OP_READ {
            return match self.registry.get(badge).and_then(|e| e.reader) {
                Some(r) => Handled::Read {
                    window: r.window,
                    cursor: w1,
                },
                None => Handled::Refused,
            };
        }
        if byte_sink_protocol::kind(w0) != byte_sink_protocol::KIND_TEXT {
            self.refused += 1;
            return Handled::Nothing;
        }
        let mut bytes = [0u8; INLINE_MAX];
        match unpack(w0, w1, w2, &mut bytes) {
            Msg::Bytes(n) => {
                let level = byte_sink_protocol::severity(w0);
                // Split the borrow: the assembler emits while the store takes the line.
                let Log {
                    assembler,
                    registry,
                    store,
                    ..
                } = self;
                assembler.push(badge, &bytes[..n], level, |line| {
                    store.line(registry, line, now);
                });
            }
            Msg::Eof => {
                if let Some(line) = self.assembler.finish(badge) {
                    self.store.line(&self.registry, line, now);
                }
            }
            Msg::Malformed => self.refused += 1,
        }
        Handled::Nothing
    }

    /// One spawner word. A malformed one is counted, never half-applied.
    fn control(&mut self, w0: u64, w1: u64, w2: u64) {
        let (op, field, arg, badge) = control::fields(w0);
        let ok = match op {
            control::OP_NAME if badge != 0 && arg < 2 && field <= control::USER => {
                let mut chunk = [0u8; 16];
                chunk[..8].copy_from_slice(&w1.to_le_bytes());
                chunk[8..].copy_from_slice(&w2.to_le_bytes());
                self.registry
                    .name(badge as u64, field, arg as usize, &chunk)
            }
            control::OP_STREAM if badge != 0 && arg <= severity::DEBUG as u64 => self
                .registry
                .entry(badge as u64)
                .map(|e| e.default_severity = arg as u8)
                .is_some(),
            control::OP_READER if badge != 0 && field <= control::SCOPE_SYSTEM => self
                .registry
                .entry(badge as u64)
                .map(|e| {
                    e.reader = Some(registry::Reader {
                        window: arg as u8,
                        system: field == control::SCOPE_SYSTEM,
                    });
                })
                .is_some(),
            control::OP_FORGET if badge != 0 => {
                // A partial line dies with its writer's registration rather than being stamped
                // later under a name nobody holds.
                let _ = self.assembler.finish(badge as u64);
                self.registry.forget(badge as u64);
                true
            }
            _ => false,
        };
        if !ok {
            self.refused += 1;
        }
    }

    /// **Store one F3 record**, already stamped: the path the kernel's records will take once
    /// milestone 342 (the kernel and the `console` server drive one UART from two address spaces)
    /// builds its ring, with [`record::flags::KERNEL`] set and `program` given as `kernel`. The
    /// header's `seq` is replaced by the service's own, so the ring's numbering stays dense whoever
    /// wrote the record.
    pub fn ingest(
        &mut self,
        header: &Header,
        text: &[u8],
        program: Option<Name>,
        user: Option<Name>,
    ) {
        self.store.record(header, text, program, user);
    }

    /// **Answer a read**: write whole JSONL lines from `cursor` into `window` (a
    /// [`read::WINDOW_BYTES`] page) and its header. `badge` is the reader's, and decides the scope:
    /// a system reader gets every record, a per-user reader only those stamped with its own user.
    pub fn fill(&self, badge: u64, cursor: u64, window: &mut [u8]) {
        let Some(entry) = self.registry.get(badge) else {
            read::put_header(window, cursor, 0, read::status::REFUSED);
            return;
        };
        let Some(reader) = entry.reader else {
            read::put_header(window, cursor, 0, read::status::REFUSED);
            return;
        };
        let scope = if reader.system {
            None
        } else {
            Some(entry.user.as_bytes())
        };
        let (next, n) =
            self.store
                .ring
                .read(cursor, scope, &mut window[read::DATA..read::WINDOW_BYTES]);
        read::put_header(window, next, n as u32, read::status::OK);
    }

    /// How many records the ring holds now, for tests and gauges.
    pub fn held(&self) -> usize {
        self.store.ring.held()
    }
}

/// The ring and what feeds it: the sequence counter, and the buffer a line is rendered into.
struct Store {
    ring: Ring,
    next_seq: u64,
    /// Where a line is rendered before it is copied into the ring. Here rather than on the stack
    /// because a program the kernel spawns has one 4 KiB stack page, and half of it is too much to
    /// spend on one buffer.
    scratch: [u8; json::LINE_MAX],
}

impl Store {
    /// Stamp a finished line from its writer's badge, render it, store it.
    fn line(&mut self, registry: &Registry, line: &Line, now: u64) {
        let entry = registry.get(line.badge);
        let header = Header {
            seq: 0,
            time: now,
            source: line.badge,
            severity: line
                .severity
                .unwrap_or(entry.map_or(severity::INFO, |e| e.default_severity)),
            flags: if line.cut { record::flags::CUT } else { 0 },
            len: line.len as u16,
            kind: byte_sink_protocol::KIND_TEXT,
        };
        let program = entry.map(|e| e.program);
        let user = entry.map(|e| e.user);
        self.record(&header, &line.text[..line.len], program, user);
    }

    /// F3 to JSONL to the ring, numbering the record.
    fn record(&mut self, header: &Header, text: &[u8], program: Option<Name>, user: Option<Name>) {
        let mut h = *header;
        h.seq = self.next_seq;
        self.next_seq += 1;
        let program = program
            .as_ref()
            .map(Name::as_bytes)
            .filter(|p| !p.is_empty());
        let user = user.as_ref().map(Name::as_bytes).filter(|u| !u.is_empty());
        let n = json::render(&h, program, user, text, &mut self.scratch);
        self.ring
            .append(h.seq, user.unwrap_or(&[]), &self.scratch[..n]);
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use std::string::String;
    use std::vec::Vec;

    use byte_sink_protocol::{eof, pack, with_severity};

    use super::*;

    fn register(log: &mut Log, badge: u32, program: &[u8], user: &[u8]) {
        for (f, n) in [(control::PROGRAM, program), (control::USER, user)] {
            for m in control::name_messages(badge, f, n).into_iter().flatten() {
                log.handle(0, m.0, m.1, m.2, 0);
            }
        }
    }

    fn reader(log: &mut Log, badge: u32, user: &[u8], system: bool, window: u8) {
        register(log, badge, b"reader", user);
        let scope = if system {
            control::SCOPE_SYSTEM
        } else {
            control::SCOPE_USER
        };
        log.handle(
            0,
            control::word(control::OP_READER, scope, window as u64, badge),
            0,
            0,
            0,
        );
    }

    fn write(log: &mut Log, badge: u64, bytes: &[u8], now: u64) {
        let mut rest = bytes;
        while !rest.is_empty() {
            let (w0, w1, w2, n) = pack(rest);
            log.handle(badge, w0, w1, w2, now);
            rest = &rest[n..];
        }
    }

    fn read_all(log: &mut Log, badge: u64, cursor: u64) -> (u64, Vec<String>) {
        let mut window = [0u8; read::WINDOW_BYTES];
        assert!(matches!(
            log.handle(badge, read::request(), cursor, 0, 0),
            Handled::Read { .. }
        ));
        log.fill(badge, cursor, &mut window);
        let (next, len, status) = read::header(&window);
        assert_eq!(status, read::status::OK);
        let text = core::str::from_utf8(&window[read::DATA..read::DATA + len]).unwrap();
        (next, text.lines().map(String::from).collect())
    }

    /// **Stamping is the service's, not the writer's.** A writer that puts a forged attribution in
    /// its text gets it quoted as text; `program`, `user`, `seq`, `time` and `source` come from the
    /// badge and the service's own clock and counter.
    #[test]
    fn the_service_stamps_and_the_writer_only_supplies_text() {
        let mut log = Log::new();
        register(&mut log, 3, b"honest", b"alice");
        reader(&mut log, 9, b"", true, 0);
        write(
            &mut log,
            3,
            b"{\"program\":\"login\",\"user\":\"root\"}\n",
            777,
        );
        let (_, lines) = read_all(&mut log, 9, 0);
        assert_eq!(
            lines,
            [concat!(
                r#"{"seq":0,"time":777,"source":3,"program":"honest","user":"alice","#,
                r#""severity":"info","msg":"{\"program\":\"login\",\"user\":\"root\"}"}"#
            )]
        );
    }

    /// Severity: the writer's framing bits when it set them, the registered stream's inference
    /// when it did not, and `info` for a badge nobody registered a stream for.
    #[test]
    fn severity_is_the_writers_bits_or_the_streams_inference() {
        let mut log = Log::new();
        register(&mut log, 1, b"out", b"");
        register(&mut log, 2, b"diag", b"");
        log.handle(
            0,
            control::word(control::OP_STREAM, 0, severity::WARNING as u64, 2),
            0,
            0,
            0,
        );
        reader(&mut log, 9, b"", true, 0);
        write(&mut log, 1, b"plain\n", 1);
        write(&mut log, 2, b"inferred\n", 2);
        let (w0, w1, w2, _) = pack(b"explicit\n");
        log.handle(2, with_severity(w0, severity::ERROR), w1, w2, 3);
        let (_, lines) = read_all(&mut log, 9, 0);
        assert!(lines[0].contains(r#""severity":"info","msg":"plain""#));
        assert!(lines[1].contains(r#""severity":"warning","msg":"inferred""#));
        assert!(lines[2].contains(r#""severity":"err","msg":"explicit""#));
    }

    /// **Two writers interleaved chunk by chunk still make whole lines each**, which is the
    /// property milestone 342 needs this service for: the console sees lines, never a splice.
    #[test]
    fn interleaved_writers_produce_whole_lines() {
        let mut log = Log::new();
        register(&mut log, 1, b"a", b"");
        register(&mut log, 2, b"b", b"");
        reader(&mut log, 9, b"", true, 0);
        let one = b"the first writer's line is longer than one chunk\n";
        let two = b"and so is the second writer's, by a margin\n";
        let (mut x, mut y) = (&one[..], &two[..]);
        while !x.is_empty() || !y.is_empty() {
            for (badge, rest) in [(1u64, &mut x), (2, &mut y)] {
                if !rest.is_empty() {
                    let (w0, w1, w2, n) = pack(rest);
                    log.handle(badge, w0, w1, w2, 5);
                    *rest = &rest[n..];
                }
            }
        }
        let (_, lines) = read_all(&mut log, 9, 0);
        assert_eq!(lines.len(), 2);
        assert!(
            lines.iter().any(
                |l| l.ends_with(r#""msg":"the first writer's line is longer than one chunk"}"#)
            )
        );
        assert!(
            lines
                .iter()
                .any(|l| l.ends_with(r#""msg":"and so is the second writer's, by a margin"}"#))
        );
    }

    /// The per-user read is a filter over the one ring: a per-user reader sees its own user's
    /// records and nobody else's, and the system reader sees all of them.
    #[test]
    fn a_per_user_read_filters_the_one_ring() {
        let mut log = Log::new();
        register(&mut log, 1, b"w", b"alice");
        register(&mut log, 2, b"w", b"bob");
        reader(&mut log, 8, b"alice", false, 1);
        reader(&mut log, 9, b"", true, 0);
        for i in 0..6u64 {
            write(&mut log, 1 + i % 2, b"line\n", i);
        }
        let (_, alice) = read_all(&mut log, 8, 0);
        let (next, all) = read_all(&mut log, 9, 0);
        assert_eq!(alice.len(), 3);
        assert!(alice.iter().all(|l| l.contains(r#""user":"alice""#)));
        assert_eq!(all.len(), 6);
        assert_eq!(next, 6);
        assert_eq!(
            read_all(&mut log, 9, next).1.len(),
            0,
            "a caught-up reader gets nothing"
        );
    }

    /// Reading is a registered authority: a writer's badge asking for records is refused, and so
    /// is a badge nobody registered.
    #[test]
    fn only_a_registered_reader_reads() {
        let mut log = Log::new();
        register(&mut log, 1, b"w", b"");
        assert_eq!(log.handle(1, read::request(), 0, 0, 0), Handled::Refused);
        assert_eq!(log.handle(77, read::request(), 0, 0, 0), Handled::Refused);
        let mut window = [0u8; read::WINDOW_BYTES];
        log.fill(1, 0, &mut window);
        assert_eq!(read::header(&window).2, read::status::REFUSED);
    }

    /// Control is badge 0's alone: the same words from a writer's badge are refused and change
    /// nothing, which is what keeps a writer from renaming itself.
    #[test]
    fn a_writer_cannot_register_itself() {
        let mut log = Log::new();
        register(&mut log, 1, b"real", b"");
        reader(&mut log, 9, b"", true, 0);
        for m in control::name_messages(1, control::PROGRAM, b"forged")
            .into_iter()
            .flatten()
        {
            log.handle(1, m.0, m.1, m.2, 0);
        }
        write(&mut log, 1, b"x\n", 0);
        let (_, lines) = read_all(&mut log, 9, 0);
        assert!(lines[0].contains(r#""program":"real""#));
        assert!(log.refused > 0);
    }

    /// End of stream flushes a writer's partial line, flagged as cut, and a line longer than the
    /// text bound becomes two records rather than one silently truncated.
    #[test]
    fn eof_flushes_and_a_long_line_is_split_not_truncated() {
        let mut log = Log::new();
        register(&mut log, 1, b"w", b"");
        reader(&mut log, 9, b"", true, 0);
        write(&mut log, 1, b"no newline", 0);
        log.handle(1, eof(), 0, 0, 1);
        let long = [b'x'; record::TEXT_MAX + 10];
        write(&mut log, 1, &long, 2);
        write(&mut log, 1, b"\n", 3);
        let (_, lines) = read_all(&mut log, 9, 0);
        assert_eq!(lines.len(), 3);
        assert!(lines[0].ends_with(r#""msg":"no newline","cut":true}"#));
        assert!(lines[1].contains(r#""cut":true"#));
        assert!(lines[2].ends_with(r#""msg":"xxxxxxxxxx"}"#));
    }

    /// **The ring is 64 KiB, oldest out, and a reader who fell behind is told what it lost**: one
    /// dropped-count line for the gap, then the records still held, in order.
    #[test]
    fn overflow_evicts_the_oldest_and_a_late_reader_gets_one_dropped_line() {
        let mut log = Log::new();
        register(&mut log, 1, b"w", b"");
        reader(&mut log, 9, b"", true, 0);
        let line = [b'y'; 150];
        let mut total = 0u64;
        while log.store.ring.first_seq() == 0 {
            write(&mut log, 1, &line, total);
            write(&mut log, 1, b"\n", total);
            total += 1;
        }
        let first = log.store.ring.first_seq();
        let (next, lines) = read_all(&mut log, 9, 0);
        assert_eq!(
            lines[0],
            std::format!(r#"{{"dropped":{first},"from":0,"to":{}}}"#, first - 1)
        );
        assert!(lines[1].starts_with(&std::format!(r#"{{"seq":{first},"#)));
        assert!(next > first);
        const { assert!(ring::RING_BYTES == 64 * 1024) };
    }

    /// A forgotten badge's later lines are from nobody: no program, no user, still stamped with
    /// the badge itself, so a reader can tell an unknown writer from a kernel line.
    #[test]
    fn a_forgotten_badge_is_nobody() {
        let mut log = Log::new();
        register(&mut log, 1, b"gone", b"alice");
        reader(&mut log, 9, b"", true, 0);
        log.handle(0, control::word(control::OP_FORGET, 0, 0, 1), 0, 0, 0);
        write(&mut log, 1, b"late\n", 0);
        let (_, lines) = read_all(&mut log, 9, 0);
        assert!(
            lines[0].starts_with(r#"{"seq":0,"time":0,"source":1,"program":null,"user":null,"#)
        );
    }

    // ---- survivor triage of the 2026-10-03 census ---------------------------------------------

    /// A message that is not text is counted and dropped, each time.
    #[test]
    fn a_message_of_another_kind_is_counted_every_time() {
        let mut log = Log::new();
        let other_kind = 1u64 << byte_sink_protocol::KIND_SHIFT;
        log.handle(7, other_kind, 0, 0, 0);
        assert_eq!(log.refused, 1);
        log.handle(7, other_kind, 0, 0, 0);
        assert_eq!(log.refused, 2);
    }

    /// Each control word is refused for the one thing wrong with it, and a good one is not counted.
    #[test]
    fn a_control_word_is_refused_for_each_thing_that_can_be_wrong_with_it() {
        let mut log = Log::new();
        register(&mut log, 5, b"p", b"u");
        assert_eq!(log.refused, 0, "registering is not a refusal");
        let mut refused = 0;
        let mut try_word = |log: &mut Log, w: u64, bad: bool| {
            log.handle(0, w, 0, 0, 0);
            refused += u64::from(bad);
            assert_eq!(log.refused, refused, "{w:#x}");
        };
        // Badge 0 is the spawner itself, and none of the four ops may name it.
        for op in [
            control::OP_NAME,
            control::OP_STREAM,
            control::OP_READER,
            control::OP_FORGET,
        ] {
            try_word(&mut log, control::word(op, 0, 0, 0), true);
        }
        // Past the last argument or field each op takes.
        try_word(&mut log, control::word(control::OP_NAME, 0, 2, 5), true);
        try_word(
            &mut log,
            control::word(control::OP_NAME, control::USER + 1, 0, 5),
            true,
        );
        try_word(
            &mut log,
            control::word(control::OP_STREAM, 0, severity::DEBUG as u64 + 1, 5),
            true,
        );
        try_word(
            &mut log,
            control::word(control::OP_READER, control::SCOPE_SYSTEM + 1, 0, 5),
            true,
        );
        try_word(&mut log, control::word(0x7f, 0, 0, 5), true);
        // And the edges that are allowed.
        try_word(
            &mut log,
            control::word(control::OP_NAME, control::USER, 0, 5),
            false,
        );
        try_word(
            &mut log,
            control::word(control::OP_STREAM, 0, severity::DEBUG as u64, 5),
            false,
        );
        try_word(
            &mut log,
            control::word(control::OP_READER, control::SCOPE_SYSTEM, 0, 5),
            false,
        );
        try_word(&mut log, control::word(control::OP_FORGET, 0, 0, 5), false);
    }

    /// A record handed straight to the store is held and read back, and counted.
    #[test]
    fn an_ingested_record_is_held_and_readable() {
        let mut log = Log::new();
        reader(&mut log, 9, b"", true, 0);
        assert_eq!(log.held(), 0);
        let h = Header {
            seq: 99,
            time: 5,
            source: 0,
            severity: severity::ERROR,
            flags: record::flags::KERNEL,
            len: 4,
            kind: 0,
        };
        log.ingest(&h, b"boot", Some(Name::new(b"kernel")), None);
        assert_eq!(log.held(), 1);
        log.ingest(&h, b"more", Some(Name::new(b"kernel")), None);
        assert_eq!(log.held(), 2);
        let (_, lines) = read_all(&mut log, 9, 0);
        assert!(lines[0].starts_with(r#"{"seq":0,"time":5,"source":0,"program":"kernel""#));
        assert!(
            lines[1].starts_with(r#"{"seq":1,"#),
            "the service numbers it, not the header"
        );
    }
}
