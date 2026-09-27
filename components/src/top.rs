//! **`top`: the domain `ps` lists, ordered by what it is costing** (milestone 282 (a thread's CPU time, and the `top` it makes possible),
//! DECISIONS §150 (how does a thread's CPU time reach userspace?), notes/process-view.md).
//!
//! The whole program is: walk one supervision domain twice, once for each thread's run state and
//! once for its CPU time, rank the rows by the second, and say what it found on one stream and what
//! went wrong on the other. The table and the walk are `crates/ps`; the summary line is
//! `crates/top`; what lives here is the syscalls and the two sinks.
//!
//! # It is `ps`'s authority, asking a different question
//!
//! Three capabilities, and they are `ps`'s three, from the same named constants. Nothing here can
//! name a process the prompt did not already put in its reach, and the CPU figures come from a
//! second walk of the **same** endpoint under the **same** right, so a column that on Unix comes
//! from reading an ambient `/proc` comes here from a capability somebody handed this program.
//!
//! What differs from `ps` is the question, not the endowment. `ps` answers *what exists*, in the
//! kernel's own slot order. This answers *what is consuming*, most first, with a summary line
//! saying how large the thing being ranked is. Milestone 281 (`watch` holds exactly what `ps` holds) deleted `watch` for holding `ps`'s
//! authority while being `ps`'s own loop, and whether this program clears that bar or belongs in
//! `ps` as a flag is an architect's; `crates/top`'s module docs carry the argument both ways.
//!
//! # Capability contract
//!
//! | slot | what | why |
//! |---|---|---|
//! | 0 | the output sink, `WRITE` | where the summary and the table go |
//! | 7 | the process domain, `ENUMERATE` | the supervision endpoint whose members it may **name** |
//! | 8 | the diagnostics sink, `WRITE` | where a refusal goes, so `>` cannot swallow it |
//!
//! No clock, and that is worth stating because a `top` looks like it needs one: the uptime in the
//! summary is `user_mode_runtime::monotonic_nanos`, the ambient counter every process holds
//! unconditionally, which is the same finding `uptime` reported when it turned out to need no new
//! capability at all.
//!
//! # What a reader learns, and it is more than `ps` gave them
//!
//! A run state is one of five words about an instant. CPU time is **continuous and monotonic**: two
//! runs of this program measure how much work another thread did in between, and a holder of
//! `ENUMERATE` over a domain can watch the shape of a workload it cannot otherwise name. DECISIONS
//! §150 (how does a thread's CPU time reach userspace?) weighed that and accepted it, because `ENUMERATE` is already the right to learn what
//! exists rather than to act on it, and because the leak is bounded by the domain: this program
//! cannot see outside the subtree it was endowed, and there is nothing it can hold that would widen
//! that. It is written here as well as in the decision because this is where a reader meets the
//! method.
//!
//! # EXAMPLES
//!
//! ```text
//! $ top
//! up 00:01:12, 3 threads: 1 running, 0 ready, 1 blocked, 1 dead
//!          TID  STATE     TIME(ms)
//!            9  running        310
//! 4294967302  dead             150
//!            5  blocked         20
//!
//! $ top > busiest.txt     the summary and the table land in the file
//! $ top | wc              and no /proc was read to make either
//! ```
//!
//! And the case with no Unix equivalent, because on Unix there is no domain to be outside of:
//!
//! ```text
//! $ top        top: this process holds no process-domain capability
//! ```
//!
//! # BUGS
//!
//! - **It does not refresh**, which is the first thing a reader will look for. This kernel has no
//!   timed wait, so an interval would be a yield-spin, and a spin here is not merely wasteful the
//!   way it was in the deleted `watch`: it would be charged to this program's own thread by the
//!   counter the table ranks on, and `top` would truthfully report itself as the busiest thing on
//!   the machine. See `crates/top`'s `BUGS` for the rest, including why there is no `%CPU` column
//!   and no way to ask for the top *N*.
//! - **`top` lists itself**, as `running`, because it is a member of the domain it was spawned
//!   into, and unlike the rows below it, it is *accumulating* while it prints. Unix's `top` has the
//!   same property and the same excuse. Its own figure is small (a few ticks at most) and is
//!   truthful.
//! - **Everything `ps` records applies here**, since the walk is `ps`'s: no `CMD` column because a
//!   process has no name, a generational tid that is a large ugly number after slot reuse, and a
//!   table that is a sequence of snapshots rather than one.
//!
//! Name: provisional. `top` is the standard term for a view ranked by resource consumption, which
//! the naming tenet calls the best name available, and both DECISIONS §150 (how does a thread's CPU time reach userspace?) and milestone 281 (`watch` holds exactly what `ps` holds)
//! named this program in advance. Provisional because calef has not ruled, and because the prior
//! question is whether this is a program at all or a flag on `ps`.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68 (code-quality gates: one lint policy)'s ratchet
// tracks (DECISIONS §107 (`missing_docs` moves to `workspace.lints.rust`)): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

use core::sync::atomic::{AtomicBool, Ordering};

use user_mode_runtime::{exit, is_granted, monotonic_nanos, send, survey, survey_record};

/// The output sink: where the summary and the table go. Slot 0 is where every spawned program's
/// output lands.
const REPORT: u64 = 0;

/// **The process domain**: a supervision endpoint with `ENUMERATE`. Everything this program can see
/// comes through here, and both walks use it.
const DOMAIN_SLOT: u64 = grant_plan::DOMAIN_SLOT;

/// The declared second stream (DECISIONS §67): complaints about the run, never about the domain.
const DIAG_SLOT: u64 = grant_plan::DIAGNOSTICS_SLOT;

/// Whether this process was granted a second stream, decided once in [`_start`].
static HAS_DIAG: AtomicBool = AtomicBool::new(false);

#[unsafe(no_mangle)]
pub extern "C" fn _start(_a0: u64, _a1: u64, _a2: u64) -> ! {
    HAS_DIAG.store(is_granted(DIAG_SLOT), Ordering::Relaxed);

    // **Walk first, complain second, print third** (DECISIONS §67 (a program's second stream is a declaration, not a number)), for the reason `ps` gives: a
    // survey cannot know its complaints up front, and the reader of the second stream drains it to
    // end-of-stream before it reads a byte of the first.
    let mut rows = [ps::Row::default(); ps::MAX_ROWS];
    let mut found = ps::collect(&mut rows, &mut |cursor| survey(DOMAIN_SLOT, cursor));
    found.join_cpu_time(&mut |cursor| {
        survey_record(DOMAIN_SLOT, cursor, abi::survey::record::CPU_TIME)
    });
    // The ranking is the program. Everything above it is `ps`.
    found.rank_by_cpu_time();

    found.write_diagnostics(&mut |bytes| write_on(diag_slot(), bytes));
    diag_end();

    // The summary goes out only when the table does. A summary above a refusal would be a count of
    // rows this program is about to decline to print, which is the "plausible listing of nothing"
    // `ps::Survey::write_report` refuses for the same reason.
    if found.is_complete() && !found.rows().is_empty() {
        top::write_summary(found.rows(), monotonic_nanos(), &mut |bytes| {
            write_on(REPORT, bytes);
        });
    }
    found.write_report(&mut |bytes| write_on(REPORT, bytes));
    send(REPORT, byte_sink_protocol::eof(), 0, 0);
    exit();
}

/// Which endpoint a complaint goes to: the declared second stream when there is one, and the output
/// otherwise. The fallback is honest rather than a silent drop, and it is what the kernel's own
/// tests spawn this program with, since they wire no second sink.
fn diag_slot() -> u64 {
    if HAS_DIAG.load(Ordering::Relaxed) {
        DIAG_SLOT
    } else {
        REPORT
    }
}

/// **Close the second stream**, which is not tidiness: its reader waits for the end before it reads
/// the first, so a `top` that exited without this would leave the prompt blocked.
fn diag_end() {
    if HAS_DIAG.load(Ordering::Relaxed) {
        send(DIAG_SLOT, byte_sink_protocol::eof(), 0, 0);
    }
}

/// Write bytes to an endpoint under the sink contract, sixteen at a time. No newline is added:
/// where a line ends is the table's business and not the transport's.
fn write_on(slot: u64, bytes: &[u8]) {
    let mut rest = bytes;
    while !rest.is_empty() {
        let (w0, w1, w2, n) = byte_sink_protocol::pack(rest);
        send(slot, w0, w1, w2);
        rest = &rest[n..];
    }
}

user_mode_runtime::panic_handler!();
