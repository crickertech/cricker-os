//! **A file behind a byte sink** (milestone 50, split out of the `sink` binary by milestone 292;
//! notes/sink-protocol.md).
//!
//! The `fs_file_caretaker` shape, one contract further out. `fs_file_caretaker` is a caretaker
//! because it "serves the same `filesystem_protocol` protocol its own client speaks"; this one serves a
//! *different* and much smaller protocol than it speaks, and that asymmetry is the point. It holds
//! an FS-service endpoint, which is a directory capability: it can open names, read them, write at
//! arbitrary offsets, truncate, and stat. Its client holds an endpoint to this process, over which
//! the only expressible request is *here are up to sixteen bytes, append them*.
//!
//! So `> report.txt` grants strictly less than Unix's fd 1 does, and it is structural rather than
//! policy: the confined program cannot seek, cannot truncate, cannot re-read and cannot stat,
//! because there is no message that says any of those things and nothing in its capability table
//! that names the FS server. That is milestone 50's claim, and this file is where it is either true
//! or not.
//!
//! # The wiring
//!
//! | slot 0 | slot 1 | slot 2 | [`PAGE_VA`] |
//! |---|---|---|---|
//! | the sink it serves, `READ` | the FS service, `WRITE` | the report, `WRITE` | shared with the FS server |
//!
//! The report carries [`fixture::READY`] once the file is open, then [`fixture::DONE`] and the byte
//! total at end of stream, or [`fixture::BAD`] and the failing request. Readiness is separate from
//! completion because a caller has to know the file exists before it hands the sink to a writer; a
//! writer that arrives first would block on a rendezvous nobody is serving yet.
//!
//! # What this is not
//!
//! It is a fixture, not the `>` a person types. The shell's redirection goes through
//! `components/src/fs_file_caretaker.rs`, which narrows the *same* protocol rather than translating
//! between two. This program exists so that the sink contract's receiving end can be exercised
//! against a real RedoxFS image, and `kernel::user::fs_service::start_file_sink` is its only
//! caller.
//!
//! # EXAMPLES
//!
//! The whole of what a writer can say to it, which is the claim:
//!
//! ```
//! use byte_sink_protocol::{eof, pack};
//!
//! let (w0, w1, w2, n) = pack(b"hello\n"); // SEND these three words: append n bytes
//! assert_eq!(n, 6);
//! let _end = eof();                       // SEND this: close the file
//! # let _ = (w0, w1, w2);
//! ```
//!
//! There is no third message, and that absence is the confinement.
//!
//! # BUGS
//!
//! Each message is written straight through rather than buffered. That costs an FS round trip per
//! sixteen bytes and it is the honest starting point: buffering is a component that speaks the sink
//! contract on both sides, inserted into the chain, not a hidden behaviour of one sink. See
//! notes/sink-protocol.md.
//!
//! It appends to one name, [`fixture::SINK_NAME`], in the FS service's root. Two of these running
//! at once on one image would interleave into one file, and nothing stops that; the wiring runs
//! exactly one.
//!
//! [`PAGE_VA`] is hard-coded here and in every other program that maps the FS server's shared
//! frame, each with its own copy of the same address and its own byte accessors. That duplication
//! is older than this file and outlives it, and the lane splitting `components/src/ntp.rs`
//! concurrently with this one has proposed the crate that would end it.
//!
//! Name: recorded (design/roadmap/292-the-sink-contract-ends-are-three-programs.md and
//! notes/sink-protocol.md, which together carry the argument). **Provisional: calef has not
//! ratified it.** It was `ROLE_FILE` inside the `sink` binary until milestone 292 split that binary
//! into the three programs it had always been, and `file_sink` is the name the kernel side has
//! spelled all along (`fs_service::FileSink`, `fs_service::start_file_sink`). `sink` survives the
//! 2026-09-13 structural-versus-current test (design/naming.md) here for the strongest of the three
//! reasons it survives anywhere: this terminus is structural, because the client holds a capability
//! over which no message but "append" is expressible, and no grant anybody could make would change
//! that. Refused `file_sink_caretaker` (it would match the shape the tree already spells
//! `terminal_sink_caretaker`, the same adapter for a different backend, and that analogy nearly won;
//! it loses on collision with the existing and different `fs_file_caretaker`, and two names a reader
//! must hold apart cost more than the consistency buys) and `sink_file_writer` (it reads as a
//! sibling of the transcript writer and is not one: that program writes *into* a sink, this one *is*
//! the sink).
//!
//! That the tree spells one translating adapter `terminal_sink_caretaker` and this one not is a real
//! inconsistency in what `caretaker` means. It is calef's to settle rather than this file's, and it
//! has a home: milestone 413, design/roadmap/413-what-a-caretaker-is-when-it-translates.md.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

use byte_sink_protocol::fixture;
use filesystem_protocol::fs;
use user_mode_runtime::mapped_window::MappedWindow;
use user_mode_runtime::{call, exit, recv, send};

/// The byte sink this process serves, `READ`. Its clients hold `WRITE` on the same endpoint and
/// nothing else.
const SINK: u64 = 0;
/// The FS-service endpoint: a directory capability, and the authority this process holds that its
/// client deliberately does not.
const FS: u64 = 1;
/// Where readiness, the byte total, and any failure go.
const REPORT: u64 = 2;

/// The page shared with the FS server. Matches `fs_service`'s `FILE_VA_CLIENT`.
const PAGE_VA: u64 = address_space_map::pair_page(0x0000_0000_0060_0000);
/// Its size, the FS contract's transfer unit.
const PAGE: usize = filesystem_protocol::PAGE;

// SAFETY: the wiring maps one page read/write at PAGE_VA before this program runs (milestone 139
// round 2; see `user_mode_runtime::mapped_window`, which is what collapsed the hand-rolled read_volatile/
// write_volatile this used to carry).
const WINDOW: MappedWindow = unsafe { MappedWindow::new(PAGE_VA, PAGE as u64) };

/// Copy `bytes` into the shared page.
fn put(bytes: &[u8]) {
    for (i, &b) in bytes.iter().take(PAGE).enumerate() {
        WINDOW.w8(i as u64, b);
    }
}

/// One request to the FS server. Negative replies are the caller's to interpret.
fn fs_call(w0: u64, w1: u64) -> i64 {
    call(FS, w0, w1).0 as i64
}

/// **Open the sink's file, creating it if it is not there and emptying it if it is.**
///
/// `CREATE` is create, not create-or-open (DECISIONS §27), so the fallback is explicit: on any
/// refusal, open the existing name and truncate it to nothing. Stated rather than inferred, because
/// a sink that appended to whatever a previous run left behind would pass its own test on a stale
/// file. Both ISA legs write the same image name on a fresh image per run, so in practice the
/// create succeeds; the fallback is what keeps a repeat boot honest instead of confusing.
fn open_for_writing() -> u64 {
    let name = fixture::SINK_NAME.as_bytes();
    put(name);
    let created = fs_call(fs::req(fs::CREATE, fs::ROOT, name.len() as u64), 0);
    if created >= 0 {
        return created as u64;
    }
    put(name);
    let opened = fs_call(fs::req(fs::OPEN, fs::ROOT, name.len() as u64), 0);
    if opened < 0 {
        send(REPORT, fixture::BAD, opened as u64, 0);
        exit();
    }
    if fs_call(fs::req(fs::TRUNCATE, opened as u64, 0), 0) < 0 {
        send(REPORT, fixture::BAD, opened as u64, 1);
        exit();
    }
    opened as u64
}

/// Receive sink messages, append each one's bytes at the running offset, and close on end of
/// stream.
#[unsafe(no_mangle)]
pub extern "C" fn _start(_a0: u64, _a1: u64, _a2: u64) -> ! {
    let handle = open_for_writing();
    send(REPORT, fixture::READY, 0, 0);

    let mut off = 0u64;
    loop {
        let (w0, w1, w2) = recv(SINK);
        let mut buf = [0u8; byte_sink_protocol::INLINE_MAX];
        match byte_sink_protocol::unpack(w0, w1, w2, &mut buf) {
            byte_sink_protocol::Msg::Bytes(0) => {}
            byte_sink_protocol::Msg::Bytes(n) => {
                put(&buf[..n]);
                let wrote = fs_call(fs::req(fs::WRITE, handle, n as u64), off);
                if wrote != n as i64 {
                    // A short write is a lost byte, and a sink that reported DONE after one would
                    // be certifying a file it did not write. Say which request failed and stop.
                    send(REPORT, fixture::BAD, wrote as u64, off);
                    exit();
                }
                off += n as u64;
            }
            byte_sink_protocol::Msg::Eof => break,
            byte_sink_protocol::Msg::Malformed => {
                send(REPORT, fixture::BAD, w0, off);
                exit();
            }
        }
    }

    let _ = fs_call(fs::req(fs::CLOSE, handle, 0), 0);
    send(REPORT, fixture::DONE, off, 0);
    exit();
}

user_mode_runtime::panic_handler!();
