//! **A program that does not know what it is writing to** (milestone 50, split out of the `sink`
//! binary by milestone 292; notes/sink-protocol.md).
//!
//! It writes [`byte_sink_protocol::fixture::TRANSCRIPT`] to whatever is in slot 0 and reports how the
//! last `SEND` classified. Its whole job is to make the classification assertable **by value**,
//! because "gone" and "never had one" are two numbers and a test that could not tell them apart
//! would not be testing milestone 50's one behaviour change.
//!
//! **The negative is the point.** This process holds one writable endpoint and a report endpoint.
//! Nothing it holds names a file, a directory, a terminal or the FS server, so it cannot discover
//! what is on the other side of slot 0 even if it wanted to. Three different things are put there
//! by `kernel::user::sink_tests` (an endpoint the kernel drains, `file_sink` with a real RedoxFS
//! file behind it, and `terminal_sink_caretaker` in front of a real terminal) and the same bytes
//! come out of all three.
//!
//! # The wiring
//!
//! | slot 0 | slot 1 | `a0` |
//! |---|---|---|
//! | the byte sink, `WRITE` (or **empty**) | the report, `WRITE` | how many times to write the transcript |
//!
//! `a0 = 0` means "until the sink stops taking it", which is what the destroyed-sink test uses: the
//! program cannot know when its reader will die, so it writes until the sink tells it to stop,
//! which is exactly the position a real producer in `yes | head` is in.
//!
//! **An empty slot 0 is a case, not a mistake.** It is how this kernel spells "you were never given
//! one", and the program must keep running and classify [`byte_sink_protocol::Sent::NoSink`], because
//! every operating system lets a process whose stdout is closed run to completion.
//!
//! # EXAMPLES
//!
//! The one the tests assert, in the kernel's vocabulary (`system_tests/src/user/sink_tests.rs`):
//!
//! ```text
//! let report = spawn_writer(image, Some(endpoint), 1);
//! let [class, total, ..] = ipc_receive(report);
//! assert_eq!(class, fixture::code(Sent::Ok));
//! assert_eq!(total as usize, fixture::TRANSCRIPT.len());
//! ```
//!
//! # BUGS
//!
//! The report is a single `send` of three words, so a caller that never drains it parks this
//! process forever at `exit`'s doorstep rather than losing the result. That is the rendezvous
//! working as designed, and it is a foot gun for a test that spawns the writer and then returns
//! early: drain the report or do not spawn it.
//!
//! Name: recorded (design/roadmap/292-the-sink-contract-ends-are-three-programs.md and
//! notes/sink-protocol.md, which together carry the argument). **Provisional: calef has not
//! ratified it.** It was `ROLE_WRITER` inside the `sink` binary until milestone 292 split that
//! binary into the three programs it had always been. `sink` is kept as the contract word because
//! the 2026-09-13 structural-versus-current test (design/naming/vocabulary-rulings.md) keeps it:
//! `byte_sink_protocol` is a
//! wire contract named for what it carries and makes no disposal claim, and this program is named
//! for the contract it speaks rather than for an end of a stream. `transcript` names the thing it
//! writes, which is a pinned constant rather than anything it computes, and a reader who sees
//! `sink_transcript_writer` in a process listing is not going to expect it to be useful. Refused
//! `writer` (a generic word that could name almost anything in an operating system), and
//! `indifferent_writer` (the header's own phrase for it, which names the property being *proved*
//! rather than the program; a fixture whose name asserts the conclusion is the borrowed-recognition
//! mistake this tree already caught once), and `sink_writer` (it reads as "the thing that writes a
//! sink" rather than "the thing that writes to one", and says nothing about what it writes).

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

use byte_sink_protocol::fixture;
use user_mode_runtime::{exit, send};

/// The byte sink, `WRITE`, and the whole of what this program holds towards its output: no page,
/// no acknowledgement channel, nothing else. Empty is a legal state and a tested one.
const SINK: u64 = 0;
/// Where the classification and the byte total go.
const REPORT: u64 = 1;

/// Write the transcript `repeat` times, or forever if `repeat` is 0, and report how the last
/// `SEND` classified plus how many bytes got through.
///
/// A clean finish announces end of stream; a failed one does not, because there is nothing left to
/// announce it to.
#[unsafe(no_mangle)]
pub extern "C" fn _start(repeat: u64, _a1: u64, _a2: u64) -> ! {
    let mut total = 0u64;
    let mut rounds = 0u64;
    let mut last = byte_sink_protocol::Sent::Ok;

    'outer: loop {
        let mut off = 0usize;
        while off < fixture::TRANSCRIPT.len() {
            let (w0, w1, w2, n) = byte_sink_protocol::pack(&fixture::TRANSCRIPT[off..]);
            last = byte_sink_protocol::classify(send(SINK, w0, w1, w2));
            if !matches!(last, byte_sink_protocol::Sent::Ok) {
                break 'outer;
            }
            off += n;
            total += n as u64;
        }
        rounds += 1;
        if repeat != 0 && rounds >= repeat {
            break;
        }
    }

    if matches!(last, byte_sink_protocol::Sent::Ok) {
        let _ = send(SINK, byte_sink_protocol::eof(), 0, 0);
    }
    send(REPORT, fixture::code(last), total, rounds);
    exit();
}

user_mode_runtime::panic_handler!();
