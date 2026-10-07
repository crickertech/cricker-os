//! **`uptime`**: print how long the machine has been counting (milestone 126,
//! design/roadmap/0126-who-else-is-running.md, notes/process-view.md).
//!
//! The whole program is: read the ambient monotonic counter, hand the nanoseconds to
//! [`uptime::format`], send the bytes. It holds one capability (the output sink) and cannot
//! change anything.
//!
//! # It needed no new capability, and that is the finding worth stating
//!
//! Milestone 126's own BUGS section named `free`, `uptime` and `vmstat` together as "machine
//! statistics rather than process enumeration," on the assumption all three want kernel-side
//! accounting nothing exposes today. `uptime` turned out not to: [`user_mode_runtime::monotonic_nanos`] is
//! the same counter `date` already reads, granted to **every** process unconditionally
//! (`kernel/src/arch/*/timer.rs`'s documented, deliberate exception to DECISIONS §10's
//! no-ambient-authority rule). So this program's manifest is `least_authority_demo`'s, not `date`'s: no memory,
//! no file, no clock capability, no domain, nothing but the report channel every spawn carries.
//! `free` and `vmstat` are a different body of work; see the roadmap doc's fork write-up for why.
//!
//! # Capability contract
//!
//! | slot | what | why |
//! |---|---|---|
//! | 0 | the output sink, `WRITE` | where the one line goes |
//!
//! # EXAMPLES
//!
//! ```text
//! $ uptime
//! up 00:12:34
//! $ uptime > when.txt
//! $ caps uptime
//!   uptime would grant the new process, and nothing else:
//!     cap 0  endpoint  result   report its answer back
//! ```
//!
//! # BUGS
//!
//! See `crates/uptime`'s module docs: no load average, no logged-in-user count, one-second
//! resolution, and the counter's own zero predates this kernel's init by an unmeasured amount.
//!
//! Name: ratified 2026-09-13 (calef, working the unratified worklist), with `crates/uptime` in
//! one ruling, which is what the crate-and-program pair means. See that crate's module docs
//! for the argument and the refusals.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

use user_mode_runtime::{exit, monotonic_nanos, send};

/// Slot 0: where the line goes. An endpoint with `WRITE`, the sink contract's framing.
const REPORT: u64 = 0;

#[unsafe(no_mangle)]
pub extern "C" fn _start(_a0: u64, _a1: u64, _a2: u64) -> ! {
    let line = uptime::format(monotonic_nanos());
    write_bytes(line.as_bytes());
    send(REPORT, byte_sink_protocol::eof(), 0, 0);
    exit();
}

/// Write `bytes` to [`REPORT`] in the sink contract's framing, then return without announcing
/// end-of-stream: the caller does that once, after every producer (here, one line) is done.
///
/// A failed send is not recoverable and not reportable, the same posture `wc`'s `report` takes:
/// `Gone` means the reader stopped caring, which is exactly when there is nothing left to say.
fn write_bytes(bytes: &[u8]) {
    let mut off = 0usize;
    while off < bytes.len() {
        let (w0, w1, w2, took) = byte_sink_protocol::pack(&bytes[off..]);
        if !matches!(
            byte_sink_protocol::classify(send(REPORT, w0, w1, w2)),
            byte_sink_protocol::Sent::Ok
        ) {
            return;
        }
        off += took;
    }
}

user_mode_runtime::panic_handler!();
