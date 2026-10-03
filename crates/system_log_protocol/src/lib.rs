//! **The system log's wire contracts**: milestone 613 (a system log service: the in-memory half),
//! for §242 (a system log).
//!
//! Three things two programs agree on, so by AGENTS.md rule 7 one crate rather than copies:
//!
//! - [`record`]: **F3**, the binary record. A 32-byte header the service (or, from milestone 342
//!   (the kernel and the `console` server drive one UART from two address spaces), the kernel)
//!   writes, then up to [`record::TEXT_MAX`] bytes of text. It is the transport, never
//!   the stored form: the service converts it to one JSON object per line before storing it
//!   (§242, Question 3 as ruled).
//! - [`control`]: what the spawner that minted a writer's badge tells the service about it (which
//!   program, which user, which stream), sent through the **unbadged** intake capability, which is
//!   what makes the spawner the only one who can say it.
//! - [`read`]: how a reader asks for records and where they land: a page shared with the service,
//!   one per reader, filled with whole JSONL lines.
//!
//! What a *writer* sends is not here. A writer speaks the byte sink (`crates/byte_sink_protocol`),
//! whose first word's spare bits carry a severity and a body kind since §242 amended it; that is
//! the whole point of the shape, because a program that already prints can be given the log
//! instead of a terminal and change nothing.
//!
//! # Attribution, and why the writer cannot lie about it
//!
//! Every field of a record but the text is filled by the service from the badge on the capability
//! the writer invoked. The kernel stamps that badge (`abi::rendezvous::BADGE`, §230 (badged
//! endpoint capabilities)), and the spawner that minted it registers what it means. A writer holds
//! a badged copy, so it can neither send as badge 0 (the spawner's control channel) nor as anybody
//! else.
//!
//! # BUGS
//!
//! - **Badges are 32 bits here, and the kernel's are 64.** The control words pack a badge into the
//!   low half of one word beside an opcode and a field. A spawner minting past `u32::MAX` would
//!   have its registration refused rather than truncated ([`control::badge_fits`]). No spawner in
//!   the tree comes near it; recorded because the narrowing is a choice, not a fact of the kernel.
//! - **A name is at most [`control::NAME_MAX`] bytes.** Longer program names are cut at the
//!   registration, so a JSONL reader sees the prefix. Every program in the archive fits today.
//!
//! Name: provisional (milestone 613's lane, 2026-10-02 UTC), on the `_protocol` suffix ruled at
//! milestone 265 (`_proto` is a truncation) for a crate that is a wire contract.

#![no_std]

/// Syslog's eight severity levels (RFC 5424, section 6.2.1), numbered as it numbers them: 0 is the
/// most severe. §242 Question 3 ruled these rather than a scheme of the tree's own.
pub mod severity {
    /// System is unusable.
    pub const EMERGENCY: u8 = 0;
    /// Action must be taken immediately.
    pub const ALERT: u8 = 1;
    /// Critical conditions.
    pub const CRITICAL: u8 = 2;
    /// Error conditions.
    pub const ERROR: u8 = 3;
    /// Warning conditions. What a declared diagnostics stream infers (§242, Question 4).
    pub const WARNING: u8 = 4;
    /// Normal but significant condition.
    pub const NOTICE: u8 = 5;
    /// Informational. What ordinary output infers (§242, Question 4), and what a badge nobody
    /// registered a stream for gets.
    pub const INFO: u8 = 6;
    /// Debug-level messages.
    pub const DEBUG: u8 = 7;

    /// The keyword a JSONL record spells a level with: syslog's own (`logger -p user.err`), so a
    /// reader who knows syslog reads these without a table. Past 7 is `debug`, the clamp
    /// `byte_sink_protocol::with_severity` makes.
    pub const fn name(level: u8) -> &'static str {
        match level {
            EMERGENCY => "emerg",
            ALERT => "alert",
            CRITICAL => "crit",
            ERROR => "err",
            WARNING => "warning",
            NOTICE => "notice",
            INFO => "info",
            _ => "debug",
        }
    }
}

/// **F3, the binary record** (§242, Question 4): a fixed header, then text.
///
/// | bytes | field | written by |
/// |---|---|---|
/// | 0..8 | `seq`, little-endian | the service: one counter for every record it has accepted |
/// | 8..16 | `time`, boot-relative nanoseconds | the service, at the moment the line completed |
/// | 16..24 | `source`, the writer's badge (0 for the kernel) | the kernel, through the badge |
/// | 24 | `severity`, syslog's level | the writer's framing bits, or inferred |
/// | 25 | `flags` ([`record::flags`]) | the service |
/// | 26..28 | `len`, the text's length | the service |
/// | 28 | `kind`, the body kind (`byte_sink_protocol::KIND_TEXT`) | the writer's framing bits |
/// | 29..32 | zero | reserved, and refused when not zero |
///
/// The header plus [`record::TEXT_MAX`] is 256 bytes, Zircon's debuglog record's total.
pub mod record {
    /// The header's length. F1's 32 bytes with one spare byte spent on `kind`.
    pub const HEADER_LEN: usize = 32;

    /// The most text one record carries. §242 measured it against a whole xenon boot (longest
    /// kernel line 123 bytes) and an eight-hour radon soak (99% of lines fit).
    pub const TEXT_MAX: usize = 224;

    /// Header and text together, at most.
    pub const RECORD_MAX: usize = HEADER_LEN + TEXT_MAX;

    /// The `flags` byte's bits.
    pub mod flags {
        /// The kernel wrote this record (milestone 342's ring). Its `source` is 0 and it belongs
        /// to no user.
        pub const KERNEL: u8 = 1 << 0;
        /// Records were lost immediately before this one (a kernel ring that overflowed before it
        /// was drained).
        pub const DROPPED_BEFORE: u8 = 1 << 1;
        /// The line was longer than [`super::TEXT_MAX`] and this record is its first part; or the
        /// writer ended (or was crowded out) mid-line.
        pub const CUT: u8 = 1 << 2;
    }

    /// One record's header, decoded.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Header {
        /// The service's sequence number for this record.
        pub seq: u64,
        /// Boot-relative nanoseconds.
        pub time: u64,
        /// The writer's badge, or 0 for the kernel.
        pub source: u64,
        /// Syslog's level, 0 to 7 ([`crate::severity`]).
        pub severity: u8,
        /// [`flags`].
        pub flags: u8,
        /// How many text bytes follow, at most [`TEXT_MAX`].
        pub len: u16,
        /// The body kind; 0 is text.
        pub kind: u8,
    }

    impl Header {
        /// The 32 bytes on the wire.
        pub fn encode(&self) -> [u8; HEADER_LEN] {
            let mut b = [0u8; HEADER_LEN];
            b[0..8].copy_from_slice(&self.seq.to_le_bytes());
            b[8..16].copy_from_slice(&self.time.to_le_bytes());
            b[16..24].copy_from_slice(&self.source.to_le_bytes());
            b[24] = self.severity;
            b[25] = self.flags;
            b[26..28].copy_from_slice(&self.len.to_le_bytes());
            b[28] = self.kind;
            b
        }

        /// Decode a header, or `None` when it cannot be one: a length past [`TEXT_MAX`] (obeying it
        /// would be a read past the record), a level past 7, or a reserved byte that is not zero.
        /// Refused rather than repaired, for `byte_sink_protocol::Msg::Malformed`'s reason.
        pub fn decode(b: &[u8; HEADER_LEN]) -> Option<Header> {
            let word = |i: usize| {
                let mut w = [0u8; 8];
                w.copy_from_slice(&b[i..i + 8]);
                u64::from_le_bytes(w)
            };
            let len = u16::from_le_bytes([b[26], b[27]]);
            if len as usize > TEXT_MAX || b[24] > 7 || b[29..32] != [0, 0, 0] {
                return None;
            }
            Some(Header {
                seq: word(0),
                time: word(8),
                source: word(16),
                severity: b[24],
                flags: b[25],
                len,
                kind: b[28],
            })
        }
    }
}

/// **What a spawner tells the service about a badge it minted.** Plain `SEND`s on the intake
/// endpoint through the unbadged capability, which only the minter holds: the service reads a
/// badge-0 message as control and nothing else can produce one.
///
/// Every control word is `op << 56 | field << 40 | arg << 32 | badge`, a badge in the low 32 bits.
/// The opcodes start at 0x10 so none is a byte-sink opcode, which keeps "a writer's bytes" and "the
/// spawner's word about a writer" distinct on one endpoint even before the badge is looked at.
pub mod control {
    /// **Name a badge's program or user.** `arg` is the chunk (0 or 1), `field` is [`PROGRAM`] or
    /// [`USER`], and `w1`/`w2` carry sixteen bytes of the name, NUL-padded. Chunk 0 resets the
    /// field, so a re-registration cannot leave a stale tail. [`name_messages`] builds them.
    pub const OP_NAME: u64 = 0x10;
    /// **Set a writer badge's inferred severity**: `arg` is the syslog level a line gets when its
    /// writer set none. A spawner sends [`crate::severity::WARNING`] for a declared diagnostics
    /// stream and nothing (the default, [`crate::severity::INFO`]) for ordinary output.
    pub const OP_STREAM: u64 = 0x11;
    /// **Make a badge a reader**: `arg` is its window (0 to the service's window count minus 1),
    /// and `field` is [`SCOPE_SYSTEM`] (every record) or [`SCOPE_USER`] (only records stamped with
    /// this badge's own registered user). The spawner decides which, because the per-user read is
    /// a lesser authority than the system read (§242, Question 2).
    pub const OP_READER: u64 = 0x12;
    /// **Forget a badge**: its process is gone. Records already stamped keep its names; a later
    /// message on the badge is from nobody the service knows.
    pub const OP_FORGET: u64 = 0x13;

    /// [`OP_NAME`]'s field: the program.
    pub const PROGRAM: u64 = 0;
    /// [`OP_NAME`]'s field: the user.
    pub const USER: u64 = 1;
    /// [`OP_READER`]'s field: the system read.
    pub const SCOPE_SYSTEM: u64 = 1;
    /// [`OP_READER`]'s field: the per-user read.
    pub const SCOPE_USER: u64 = 0;

    /// The longest name a registration carries: two chunks of sixteen.
    pub const NAME_MAX: usize = 32;

    /// A control word.
    pub const fn word(op: u64, field: u64, arg: u64, badge: u32) -> u64 {
        (op << 56) | ((field & 0xff) << 40) | ((arg & 0xff) << 32) | badge as u64
    }

    /// A control word's `(op, field, arg, badge)`.
    pub const fn fields(w0: u64) -> (u64, u64, u64, u32) {
        (
            w0 >> 56,
            (w0 >> 40) & 0xff,
            (w0 >> 32) & 0xff,
            (w0 & 0xffff_ffff) as u32,
        )
    }

    /// Whether `badge` can be registered at all (see the crate's BUGS): nonzero, and 32 bits.
    pub const fn badge_fits(badge: u64) -> bool {
        badge != 0 && badge <= u32::MAX as u64
    }

    /// The [`OP_NAME`] messages naming `badge`'s `field` as `name` (cut at [`NAME_MAX`]): one when
    /// it fits in sixteen bytes, two otherwise. Unused entries are `None`.
    pub fn name_messages(badge: u32, field: u64, name: &[u8]) -> [Option<(u64, u64, u64)>; 2] {
        let name = &name[..name.len().min(NAME_MAX)];
        let chunk = |i: usize| {
            let mut b = [0u8; 16];
            let part = name.get(i * 16..).unwrap_or(&[]);
            let n = part.len().min(16);
            b[..n].copy_from_slice(&part[..n]);
            let mut lo = [0u8; 8];
            let mut hi = [0u8; 8];
            lo.copy_from_slice(&b[..8]);
            hi.copy_from_slice(&b[8..]);
            (
                word(OP_NAME, field, i as u64, badge),
                u64::from_le_bytes(lo),
                u64::from_le_bytes(hi),
            )
        };
        [
            Some(chunk(0)),
            if name.len() > 16 {
                Some(chunk(1))
            } else {
                None
            },
        ]
    }
}

/// **How a reader reads.** The reader `SEND`s [`read::OP_READ`] with a cursor on the intake
/// endpoint through its own badged copy; the service fills that badge's window with whole JSONL lines and
/// signals the reader's notification. The reader waits on the notification, never the service on
/// the reader: a reader that stops reading cannot hold up a writer (§242's "dropping rather than
/// letting a writer wait").
///
/// The window, one page:
///
/// | bytes | field |
/// |---|---|
/// | 0..8 | the next cursor: pass it to the next read |
/// | 8..12 | how many bytes of JSONL follow |
/// | 12..16 | [`read::status`] |
/// | [`read::DATA`].. | the lines, each ending in `\n` |
pub mod read {
    /// A read request's opcode, in the first word's top byte; the cursor rides in the second word.
    /// A cursor is a sequence number: the first record wanted. 0 asks for everything still held.
    pub const OP_READ: u64 = 0x20;

    /// The window's size: one page.
    pub const WINDOW_BYTES: usize = 4096;
    /// Where the service maps window 0; window *n* is [`WINDOW_BYTES`] times *n* above it. A
    /// pair-page address (`address_space_map::pair_page`), agreed between the service and the
    /// spawner that maps the same frames into its readers.
    pub const WINDOW_VA: u64 = 0x0060_0000;
    /// How many reader windows the service maps, and so how many readers it can answer.
    pub const WINDOWS: usize = 4;
    /// Where the lines start in a window.
    pub const DATA: usize = 16;
    /// The most JSONL one read returns.
    pub const DATA_MAX: usize = WINDOW_BYTES - DATA;

    /// A read's outcome, at window bytes 12..16.
    pub mod status {
        /// The lines are there (possibly none: the reader is caught up).
        pub const OK: u32 = 0;
        /// The badge is not a registered reader, or its window is not one the service maps.
        /// Nothing was written except this header, and nobody was signalled: the service cannot
        /// know whom to signal. Kept for a window a later registration fixes.
        pub const REFUSED: u32 = 1;
    }

    /// The first word of a read request.
    pub const fn request() -> u64 {
        OP_READ << 56
    }

    /// Write a window's header.
    pub fn put_header(window: &mut [u8], next: u64, len: u32, status: u32) {
        window[0..8].copy_from_slice(&next.to_le_bytes());
        window[8..12].copy_from_slice(&len.to_le_bytes());
        window[12..16].copy_from_slice(&status.to_le_bytes());
    }

    /// Read a window's header: `(next, len, status)`. `len` is clamped to [`DATA_MAX`], because
    /// the page is shared and its bytes are only as honest as whoever wrote them last.
    pub fn header(window: &[u8]) -> (u64, usize, u32) {
        let mut next = [0u8; 8];
        next.copy_from_slice(&window[0..8]);
        let mut len = [0u8; 4];
        len.copy_from_slice(&window[8..12]);
        let mut status = [0u8; 4];
        status.copy_from_slice(&window[12..16]);
        (
            u64::from_le_bytes(next),
            (u32::from_le_bytes(len) as usize).min(DATA_MAX),
            u32::from_le_bytes(status),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A header survives the wire, and the three things that cannot be a header are refused: a
    /// length past the text bound, a level syslog does not have, and a reserved byte in use.
    #[test]
    fn a_header_round_trips_and_impossible_ones_are_refused() {
        let h = record::Header {
            seq: 0x0102_0304_0506_0708,
            time: 42,
            source: 7,
            severity: severity::WARNING,
            flags: record::flags::KERNEL | record::flags::CUT,
            len: record::TEXT_MAX as u16,
            kind: 0,
        };
        let b = h.encode();
        assert_eq!(b.len(), 32);
        assert_eq!(record::Header::decode(&b), Some(h));

        let mut long = b;
        long[26..28].copy_from_slice(&(record::TEXT_MAX as u16 + 1).to_le_bytes());
        assert_eq!(record::Header::decode(&long), None);
        let mut loud = b;
        loud[24] = 8;
        assert_eq!(record::Header::decode(&loud), None);
        let mut spare = b;
        spare[31] = 1;
        assert_eq!(record::Header::decode(&spare), None);
    }

    /// The control opcodes cannot be byte-sink opcodes, so a writer's message and the spawner's
    /// word about a writer are distinct on the wire before anyone looks at the badge.
    #[test]
    fn control_opcodes_are_not_byte_sink_opcodes() {
        for op in [
            control::OP_NAME,
            control::OP_STREAM,
            control::OP_READER,
            control::OP_FORGET,
            read::OP_READ,
        ] {
            assert!(op > 1, "opcode {op:#x} collides with OP_BYTES or OP_EOF");
        }
        let w = control::word(control::OP_READER, control::SCOPE_SYSTEM, 3, 0xdead_beef);
        assert_eq!(
            control::fields(w),
            (control::OP_READER, control::SCOPE_SYSTEM, 3, 0xdead_beef)
        );
        assert!(!control::badge_fits(0));
        assert!(!control::badge_fits(1 << 32));
        assert!(control::badge_fits(u32::MAX as u64));
    }

    /// A name longer than one chunk takes two messages, one that fits takes one, and one past the
    /// bound is cut rather than spilling into a third.
    #[test]
    fn a_name_is_one_or_two_chunks() {
        let short = control::name_messages(5, control::PROGRAM, b"wc");
        assert!(short[0].is_some() && short[1].is_none());
        let (w0, w1, _) = short[0].unwrap();
        assert_eq!(
            control::fields(w0),
            (control::OP_NAME, control::PROGRAM, 0, 5)
        );
        assert_eq!(&w1.to_le_bytes()[..3], b"wc\0");

        let long = control::name_messages(5, control::USER, b"sink_transcript_writer");
        let (w0, w1, w2) = long[1].unwrap();
        assert_eq!(control::fields(w0), (control::OP_NAME, control::USER, 1, 5));
        assert_eq!(&w1.to_le_bytes()[..6], b"writer");
        assert_eq!(w2, 0);
    }

    /// A shared window's length is clamped on the way in, because the page is the reader's too.
    #[test]
    fn a_window_header_round_trips_and_a_lying_length_is_clamped() {
        let mut w = [0u8; read::WINDOW_BYTES];
        read::put_header(&mut w, 9, 100, read::status::OK);
        assert_eq!(read::header(&w), (9, 100, read::status::OK));
        read::put_header(&mut w, 9, u32::MAX, read::status::OK);
        assert_eq!(read::header(&w).1, read::DATA_MAX);
    }

    /// The names are syslog's keywords, one per level, and none repeats.
    #[test]
    fn every_level_has_its_own_syslog_keyword() {
        let names: [&str; 8] = core::array::from_fn(|l| severity::name(l as u8));
        assert_eq!(
            names,
            [
                "emerg", "alert", "crit", "err", "warning", "notice", "info", "debug"
            ]
        );
    }
}
