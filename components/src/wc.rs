//! **`wc`: the consumer** (milestone 50, notes/pipes.md).
//!
//! It counts lines, words and bytes on its input and writes the three numbers to its output. Both
//! ends are the sink contract (`crates/byte_sink_protocol`), which is the entire reason this program is
//! worth having: it is the first thing in the tree that **reads** a stream, and it reads it with
//! the same sixteen-byte framing that `date` and every `println!` write.
//!
//! # It cannot tell what is feeding it, and that is the demonstration
//!
//! Slot 1 holds an endpoint with `READ`. Sink messages arrive on it until [`byte_sink_protocol::OP_EOF`].
//! Behind that endpoint there is either another program (`date | wc`), a file the shell opened and
//! streamed (`wc < report.txt`, and `wc report.txt`, which is the same thing with the operator left
//! out), or the shell itself typing bytes into it (`echo hello | wc`). There is no message this
//! program can send to ask which, no field in a message that says, and no capability in its capability table
//! that names anything but the two endpoints. **The same binary, three sources, one behaviour.**
//!
//! `wc report.txt` is worth a second look for exactly that reason. It reads like Unix's `wc`, where
//! naming a file means the program `open()`s it with your whole authority; here the name is a
//! designation the *shell* resolves, and what arrives is bytes. A `wc` that opened the file it
//! counts would be a `wc` that could open any file.
//!
//! The output side is the same claim, already made by `std_exerciser` in the protocol lane: slot 0 holds
//! an endpoint with `WRITE`, and whether the shell prints the answer or a file records it is not
//! something this program participates in.
//!
//! # Capability contract
//!
//! | slot | what | why |
//! |---|---|---|
//! | 0 | the output sink, `WRITE` | where the three numbers go |
//! | 1 | the input source, `READ` | where the bytes come from |
//!
//! Two capabilities, no memory grant, no file, no directory, no shared page. A `wc` that could open
//! the file it counts would be a `wc` that could open any file; this one is handed the bytes.
//!
//! # EXAMPLES
//!
//! ```text
//! $ echo the quick brown fox | wc
//!   1 4 20
//! $ date | wc
//!   1 5 29
//! $ wc < report.txt
//!   12 84 511
//! $ wc report.txt
//!   12 84 511
//! $ wc
//!   wc: reads an input stream: name a file, redirect with '<', or pipe into it
//! ```
//!
//! # BUGS
//!
//! - **No `-l`, `-w` or `-c`.** Unix's `wc` prints a subset chosen by options; this one prints all
//!   three, because selecting among them is formatting and the pipeline is where formatting
//!   belongs. When there is something to select *with*, the selection is a program downstream.
//! - **A word is a run of non-whitespace**, which is Unix's definition and is wrong for text that
//!   is not ASCII. There is no locale here to be wrong about, so the definition is at least honest.
//! - **It counts what it is given.** A line with no trailing newline still counts as a line, which
//!   differs from Unix's `wc -l` (that counts newlines). The number this prints is the number of
//!   lines a person would count.
//! - **One name, and it cannot count two files or print a total.** `wc a.txt b.txt` is refused at
//!   the prompt as an operand with nowhere to go, because a manifest here declares one input and a
//!   second would need positional arity the shell does not have yet. Unix prints a per-file table
//!   and a total; that is formatting over a set, and this program counts one stream.
//!
//! Name: ratified 2026-09-14 (calef, working the unratified worklist), on AGENTS.md's naming
//! section, which reasons about this exact string twice and so needed no new argument to sign. The
//! Unix command's own name, in the protected class the tenet calls the best names available: a
//! word a reader already knows from outside this project costs a newcomer nothing to learn.
//! AGENTS.md does not merely cover it by rule, it argues it by name, first as the case that
//! killed the two-tier draft (it was internal plumbing and became a prompt-typed pipeline stage
//! inside a day, so "a short name for a typed command" is not a stable property to key a
//! convention on) and then as the rule's closing line, that nobody needs a rule to know `wc`
//! beats `word_count`. Introduced 2026-07-31. What is missing is an architect's signature, not a
//! reason.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

use user_mode_runtime::{exit, recv, send};

/// The output sink: where the three numbers go, in the sink contract's framing.
const SINK: u64 = 0;
/// The input source: an endpoint we hold `READ` on, carrying the same contract inbound.
const SOURCE: u64 = 1;

#[unsafe(no_mangle)]
pub extern "C" fn _start(_a0: u64, _a1: u64, _a2: u64) -> ! {
    let (lines, words, bytes) = count();
    report(lines, words, bytes);
    exit();
}

/// Drain the input to end of stream, counting as it goes.
///
/// A malformed message ends the count rather than being skipped. The alternative is to keep going
/// and print a number that is missing bytes nobody was told about, which is the silent degradation
/// DECISIONS §42 exists to refuse; the numbers this program prints are its whole output, so a wrong
/// one is indistinguishable from a right one.
fn count() -> (u64, u64, u64) {
    let mut lines = 0u64;
    let mut words = 0u64;
    let mut bytes = 0u64;
    // Whether the byte before this one was part of a word. Carried across messages, because a word
    // that straddles a sixteen-byte boundary is one word and not two, and a decoder that only ever
    // saw whole messages would never find that out.
    let mut in_word = false;
    // Whether anything at all arrived since the last newline, so a final line with no newline on
    // the end is still counted as a line.
    let mut pending = false;

    loop {
        let (w0, w1, w2) = recv(SOURCE);
        let mut buf = [0u8; byte_sink_protocol::INLINE_MAX];
        let n = match byte_sink_protocol::unpack(w0, w1, w2, &mut buf) {
            byte_sink_protocol::Msg::Bytes(n) => n,
            byte_sink_protocol::Msg::Eof => break,
            byte_sink_protocol::Msg::Malformed => break,
        };
        for &b in &buf[..n] {
            bytes += 1;
            if b == b'\n' {
                lines += 1;
                pending = false;
            } else {
                pending = true;
            }
            let ws = b.is_ascii_whitespace();
            if !ws && !in_word {
                words += 1;
            }
            in_word = !ws;
        }
    }
    if pending {
        lines += 1;
    }
    (lines, words, bytes)
}

/// Write the answer as one line of text, then announce the end of the stream.
///
/// The end-of-stream message is not politeness. A reader downstream (the shell draining the answer,
/// or another `wc`) blocks until something tells it the producer is finished, and "the producer
/// exited" is a fact about process supervision that a reader of a stream should not have to know
/// how to find out. See notes/sink-protocol.md.
fn report(lines: u64, words: u64, bytes: u64) {
    let mut out = [0u8; 64];
    let mut n = 0usize;
    for (i, v) in [lines, words, bytes].into_iter().enumerate() {
        if i > 0 {
            out[n] = b' ';
            n += 1;
        }
        n += render(v, &mut out[n..]);
    }
    out[n] = b'\n';
    n += 1;

    let mut off = 0usize;
    while off < n {
        let (w0, w1, w2, took) = byte_sink_protocol::pack(&out[off..n]);
        // A failed send is not recoverable and not reportable: the one channel this program would
        // report on is the one that failed. `Gone` means the reader stopped caring, which is
        // exactly when there is nothing left to say.
        if !matches!(
            byte_sink_protocol::classify(send(SINK, w0, w1, w2)),
            byte_sink_protocol::Sent::Ok
        ) {
            return;
        }
        off += took;
    }
    send(SINK, byte_sink_protocol::eof(), 0, 0);
}

/// Render a `u64` in base 10 into `out`, returning how many bytes it took.
fn render(mut v: u64, out: &mut [u8]) -> usize {
    let mut digits = [0u8; 20];
    let mut i = digits.len();
    loop {
        i -= 1;
        digits[i] = b'0' + (v % 10) as u8;
        v /= 10;
        if v == 0 {
            break;
        }
    }
    let n = (digits.len() - i).min(out.len());
    out[..n].copy_from_slice(&digits[i..i + n]);
    n
}

user_mode_runtime::panic_handler!();
