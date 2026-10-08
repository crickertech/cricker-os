//! **The second copy of the package** (milestone 614 (two installed versions of one program, each
//! runnable, and a caller granted the one it needs)): a program whose bytes
//! differ from `greeting`'s, packaged as `greeting` at 0.2.0 by `packages/greeting-0.2.0.recipe.toml`
//! (the `as` key renames the member; the program keeps its own name here so the two ELFs cannot be
//! confused on the host).
//!
//! It exists because rows key on the digest (ruling 2): a second version of `greeting` carrying the
//! same member bytes would be the same row relabelled, and the gate could not prove that two
//! versions are live at once. This one's line is different, so `script/swish-check` can tell which
//! copy ran, and its digest differs, so the table holds two rows for `greeting`.
//!
//! ```text
//! $ package install downloads/0.2.0/greeting.nifepkg
//!   installed; generation 3 is live
//! $ packages/greeting/0.2.0/greeting
//! hello from the second copy of the package
//! ```
//!
//! # What this program holds
//!
//! Slot 0, its output: the sink contract (`crates/byte_sink_protocol`). No manifest note, so a
//! vouched copy is endowed as `grant_plan::NO_NOTE_MANIFEST`, its output alone.
//!
//! # BUGS
//!
//! It proves the second row and nothing about the program.
//!
//! Name: ratified 2026-10-07 (calef, §258 (names for two installed versions of one program)).
//! Refused `greeting_two` (says "second", not what differs), `greeting_0_2_0` (noisy, and stale if
//! the test version changes). It is `greeting` at 0.2.0 as far as any machine that installs the
//! package is concerned; the name says what it is on the host. Minted as `greeting_two` on
//! 2026-09-29.

#![no_std]
#![allow(missing_docs)]
#![no_main]

use user_mode_runtime::{exit, send};

/// Slot 0: where the line goes.
const OUT: u64 = 0;

/// The line. Different from `greeting`'s, so a transcript can tell which copy ran; the gate
/// asserts it.
const LINE: &[u8] = b"hello from the second copy of the package\n";

#[unsafe(no_mangle)]
pub extern "C" fn _start(_a0: u64, _a1: u64, _a2: u64) -> ! {
    let mut rest = LINE;
    while !rest.is_empty() {
        let (w0, w1, w2, n) = byte_sink_protocol::pack(rest);
        send(OUT, w0, w1, w2);
        rest = &rest[n..];
    }
    send(OUT, byte_sink_protocol::eof(), 0, 0);
    exit();
}

user_mode_runtime::panic_handler!();
