//! **A file in front of a byte source** (milestone 50, split out of the `sink` binary by milestone
//! 292; notes/sink-protocol.md).
//!
//! The read-back. It opens the name `file_sink` wrote and streams the contents out **over the sink
//! contract**, which is a small piece of evidence in itself: the same sixteen-byte framing carries a
//! file's contents as carried a program's `println!`.
//!
//! That is what makes `<` the same shape as `|` in this system. A source is the sink contract
//! *received* rather than sent, so the program on the reading end (`wc`, in the test that matters)
//! holds one endpoint and cannot tell whether a pipe or a filesystem is behind it. This process is
//! the part that makes a file look like a pipe, and it is a separate process with its own FS session
//! precisely so that the reader's side of the claim is not co-located with the writer's.
//!
//! # The wiring
//!
//! | slot 0 | slot 1 | slot 2 | [`PAGE_VA`] |
//! |---|---|---|---|
//! | the byte sink, `WRITE` | the FS service, `WRITE` | the report, `WRITE` | shared with the FS server |
//!
//! Slot 0 is `WRITE` here and `READ` in `file_sink`, which is the convention the whole milestone is
//! about: the same slot, the same contract, and the direction is a right rather than a different
//! protocol.
//!
//! **It is spawned only after `file_sink` has reported that it closed the file**, because the two
//! share the FS server's one file page. One page is sound between parties that are never using it at
//! once, and sequencing is what makes that true.
//!
//! # EXAMPLES
//!
//! The pairing the test asserts, in the kernel's vocabulary
//! (`kernel/src/user/sink_tests.rs::one_reader_two_sources_and_the_same_answer`):
//!
//! ```text
//! let (source, report) = fs_service::start_file_source(blk, fs_server, image).unwrap();
//! let out = spawn_wc(source);              // wc holds one endpoint and nothing else
//! assert_eq!(wc_counts(out), piped);       // the same answer it gave for a pipe
//! assert_eq!(ipc_recv(report)[0], fixture::DONE);
//! ```
//!
//! # BUGS
//!
//! It reports [`fixture::DONE`] and the size it *found*, not the size it managed to send. A sink
//! that stopped taking bytes part way through would be reported by the `SEND` return code this
//! program ignores, so a caller that wants delivery proven must compare what it received rather
//! than trusting the report. Every caller does, which is why this has not been fixed rather than a
//! reason it does not matter.
//!
//! It opens one name, [`fixture::SINK_NAME`], in the FS service's root. It is the read half of a
//! two-program fixture rather than a general `cat`: nothing here takes a name from anywhere.
//!
//! [`PAGE_VA`] is hard-coded here and in every other program that maps the FS server's shared
//! frame, each with its own copy of the same address and its own byte accessors. That duplication is
//! older than this file and outlives it, and the lane splitting `components/src/ntp.rs`
//! concurrently with this one has proposed the crate that would end it.
//!
//! Name: recorded (design/roadmap/292-the-sink-contract-ends-are-three-programs.md and
//! notes/sink-protocol.md, which together carry the argument). **Provisional: calef has not
//! ratified it.** It was `ROLE_VERIFY` inside the `sink` binary until milestone 292 split that
//! binary into the three programs it had always been. `source` is this tree's own word for the
//! reading end of the sink contract: `grant_plan::spawnproto::Wiring` already carries `sink` and
//! `source` as the two directions, and `kernel::user::sink_tests` names the endpoint `source` at the
//! call site. So `file_sink` and `file_source` are one pair under the vocabulary the shell wiring
//! already uses. Refused `sink_verifier` (the old role's word, which names the *test* this happens
//! to serve rather than the program: it verifies nothing, it reads a file and sends bytes, and every
//! assertion is in the kernel) and `file_reader` (a generic word for something whose whole identity
//! is the contract it speaks on the far side) and `cat` (the standard term a reader knows from
//! outside, and exactly why it cannot be used: a reader who typed it would expect a name argument
//! and a general program, and this is neither).

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

use byte_sink_protocol::fixture;
use filesystem_protocol::fs;
use user_mode_runtime::mapped_window::MappedWindow;
use user_mode_runtime::{call, exit, send};

/// The byte sink this process writes the file into, `WRITE`.
const SINK: u64 = 0;
/// The FS-service endpoint: a directory capability, and the authority the program on the far side
/// of [`SINK`] deliberately does not hold.
const FS: u64 = 1;
/// Where the size it found, or any failure, goes.
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

/// Read one byte out of the shared page.
fn get(i: usize) -> u8 {
    WINDOW.r8(i as u64)
}

/// One request to the FS server. Negative replies are the caller's to interpret.
fn fs_call(w0: u64, w1: u64) -> i64 {
    call(FS, w0, w1).0 as i64
}

/// Open what `file_sink` wrote and stream it out over the sink contract, so the bytes that reach the
/// test have been through a real filesystem and back.
#[unsafe(no_mangle)]
pub extern "C" fn _start(_a0: u64, _a1: u64, _a2: u64) -> ! {
    let name = fixture::SINK_NAME.as_bytes();
    put(name);
    let handle = fs_call(fs::req(fs::OPEN, fs::ROOT, name.len() as u64), 0);
    if handle < 0 {
        send(REPORT, fixture::BAD, handle as u64, 0);
        exit();
    }
    let handle = handle as u64;

    let size = fs_call(fs::req(fs::FSTAT, handle, 0), 0);
    if size < 0 {
        send(REPORT, fixture::BAD, size as u64, 1);
        exit();
    }
    let size = size as u64;

    let mut off = 0u64;
    while off < size {
        let want = (size - off).min(PAGE as u64);
        let got = fs_call(fs::req(fs::READ, handle, want), off);
        if got <= 0 {
            send(REPORT, fixture::BAD, got as u64, off);
            exit();
        }
        // Out of the page and onto the sink, sixteen bytes at a time. The page is the FS contract's
        // transfer unit and the message is the sink contract's; this loop is the seam between them,
        // and it is the whole of what a sink adapter is.
        let mut i = 0usize;
        while i < got as usize {
            let mut chunk = [0u8; byte_sink_protocol::INLINE_MAX];
            let n = (got as usize - i).min(byte_sink_protocol::INLINE_MAX);
            for (k, b) in chunk[..n].iter_mut().enumerate() {
                *b = get(i + k);
            }
            let (w0, w1, w2, _) = byte_sink_protocol::pack(&chunk[..n]);
            send(SINK, w0, w1, w2);
            i += n;
        }
        off += got as u64;
    }

    send(SINK, byte_sink_protocol::eof(), 0, 0);
    let _ = fs_call(fs::req(fs::CLOSE, handle, 0), 0);
    send(REPORT, fixture::DONE, size, 0);
    exit();
}

user_mode_runtime::panic_handler!();
