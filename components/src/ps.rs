//! **`ps`: who else is running, and who is allowed to ask** (milestone 126, notes/process-view.md).
//!
//! The whole program is: walk one supervision domain with `abi::rendezvous::SURVEY`, then say what it
//! found on one stream and what went wrong on the other. The listing itself is `crates/ps`, which
//! runs on the host in milliseconds; what lives here is the syscall and the two sinks.
//!
//! Name: ratified 2026-09-14 (calef, working the unratified worklist), on milestone 126 and
//! design/naming.md. Its crate was ratified 2026-08-23 in a kernel-dependency naming review, so the
//! pair is now signed at both ends. `ps` is the name every reader already knows
//! from outside this project, which the naming tenet calls the best name available and not one to
//! spend a rename on: renaming a standard term costs a reader the recognition the whole tenet exists
//! to buy. The crate beside it shares the name deliberately, which is the crate-and-program pair the
//! same tenet describes, and `coremark`, `line_editor` and `compositor` already are. That rule plus
//! the standard term produces this name; calef ruled on the rules, and never on this program.
//!
//! # It cannot enumerate the machine, and that is the demonstration
//!
//! A Unix `ps` reads `/proc`, which is **ambient**. It needs no grant from anyone, so `ps aux`
//! prints every process on the box, including the command lines with secrets in `argv`. Nobody
//! defends that design; `hidepid` exists because enough people stopped wanting to live with it.
//!
//! This one holds a **supervision endpoint** and can see exactly the threads whose deaths arrive on
//! it. There is no `/proc` to open, no pid space to scan, and no syscall in this binary's reach that
//! takes a process id it was not handed. Run it and it lists the domain it was spawned into; it
//! cannot discover that any other domain exists, let alone read one.
//!
//! The wide grant is not forbidden, it is **nameable**. An operator's monitor over the whole machine
//! is a `ps` handed the endpoint that supervises the whole machine, and `caps ps` prints which one
//! this is. On Linux there is no such distinction to print, and that is the entire difference.
//!
//! # Capability contract
//!
//! | slot | what | why |
//! |---|---|---|
//! | 0 | the output sink, `WRITE` | where the table goes |
//! | 7 | the process domain, `ENUMERATE` | the supervision endpoint whose members it may **name** |
//! | 8 | the diagnostics sink, `WRITE` | where a refusal goes, so `>` cannot swallow it |
//!
//! Three capabilities, and **none of them names a process**. No memory grant, no file, no directory,
//! no clock. That stayed true when the `TIME` column arrived (milestone 282 (a thread's CPU time, and the `top` it makes possible)): the CPU
//! figure is a second walk of the *same* endpoint under the *same* right, so a column that on Unix
//! comes from reading `/proc` comes here from a capability this program was already holding.
//!
//! The domain sits in a slot the shell's grant plan **names** ([`DOMAIN_SLOT`]) rather than the next
//! free one, for the reason DECISIONS §67 gave the diagnostics slot the same treatment: how many low
//! slots a child gets depends on what else the command line granted it, and a program that probes a
//! fixed number needs that number not to move.
//!
//! # EXAMPLES
//!
//! ```text
//! $ ps
//!          TID  STATE     TIME(ms)
//!            5  running        310
//!            9  blocked         20
//!
//! $ ps > running.txt      the table lands in the file
//! $ ps | wc               the table is counted, and no /proc was read to make it
//! ```
//!
//! And the case that has no Unix equivalent, because on Unix there is no domain to be outside of:
//!
//! ```text
//! $ ps        ps: this process holds no process-domain capability
//! ```
//!
//! # BUGS
//!
//! - **Holding the domain with `READ` was more authority than looking needs. Fixed 2026-08-17**, and
//!   the entry is kept because the shape recurs. `READ` on a supervision endpoint is also what `RECEIVE`
//!   and `abi::rendezvous::REAP` take, so a `ps` endowed a view could have taken a death message out
//!   from under the real supervisor or collected a corpse, and this binary's own source was the whole
//!   argument that it did not. `abi::rights::ENUMERATE` is the right it holds now, and the lane that
//!   deferred the fix was waiting on a decision the signalling stratum never needed: calef's ruling
//!   that a domain names its members and does not act on them abolished `pkill` rather than
//!   answering it. notes/process-view.md carries the argument.
//! - **The listing has no `CMD` column**, because a process here has no name (see `crates/ps`).
//! - **`pgrep` is this program filtered**, and it is granted the same three capabilities and not one
//!   right more (`crates/pgrep`, `components/src/pgrep.rs`). There is no `pkill` beside it and there cannot
//!   be, which is the whole point of the pair.
//! - **`ps` lists itself**, as `running`, because it is a member of the domain it was spawned into.
//!   That is truthful rather than a bug in the walk, and Unix's `ps` does the same; it is recorded
//!   because the first thing a reader counts is one more than they expected.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

use core::sync::atomic::{AtomicBool, Ordering};

use user_mode_runtime::{exit, is_granted, send, survey, survey_record};

/// The output sink: where the table goes. Slot 0 is where every spawned program's output lands.
const REPORT: u64 = 0;

/// **The process domain**: a supervision endpoint, `READ`. Everything this program can see comes
/// through here and nothing else in the capability table names a process.
const DOMAIN_SLOT: u64 = grant_plan::DOMAIN_SLOT;

/// The declared second stream (DECISIONS §67): complaints about the run, never about the domain.
const DIAG_SLOT: u64 = grant_plan::DIAGNOSTICS_SLOT;

/// Whether this process was granted a second stream, decided once in [`_start`].
static HAS_DIAG: AtomicBool = AtomicBool::new(false);

#[unsafe(no_mangle)]
pub extern "C" fn _start(_a0: u64, _a1: u64, _a2: u64) -> ! {
    HAS_DIAG.store(is_granted(DIAG_SLOT), Ordering::Relaxed);

    // **Walk first, complain second, print third.** A survey cannot know its complaints up front:
    // the domain may be refused on the first call or vanish on the tenth, and §67's reader drains
    // diagnostics to end-of-stream *before* it reads a byte of output. So the whole domain is taken
    // into a buffer, then the second stream is said and closed, then the table goes out.
    // The buffer is this program's, and it is sized so that truncation is unreachable: `MAX_ROWS`
    // is the kernel's whole thread table, so even a `ps` handed the widest grant this system can
    // express has room for every row. Two kilobytes on a twelve-page stack.
    let mut rows = [ps::Row::default(); ps::MAX_ROWS];
    let mut found = ps::collect(&mut rows, &mut |cursor| survey(DOMAIN_SLOT, cursor));
    // **The `TIME` column is a second walk** (milestone 282 (a thread's CPU time, and the `top` it makes possible)). `SURVEY` answers one record
    // per call, so both facts about one domain are two walks joined on the tid, which is the shape
    // the selector was chosen for: a caller that wants one fact pays nothing for the existence of
    // the others. Both walks are over the same endpoint and the same right, so this asks for
    // nothing the listing above did not already hold.
    found.join_cpu_time(&mut |cursor| {
        survey_record(DOMAIN_SLOT, cursor, abi::survey::record::CPU_TIME)
    });

    found.write_diagnostics(&mut |bytes| write_on(diag_slot(), bytes));
    diag_end();

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
/// the first, so a `ps` that exited without this would leave the prompt blocked.
fn diag_end() {
    if HAS_DIAG.load(Ordering::Relaxed) {
        send(DIAG_SLOT, byte_sink_protocol::eof(), 0, 0);
    }
}

/// Write bytes to an endpoint under the sink contract, sixteen at a time. No newline is added: the
/// listing already carries its own, because where a line ends is the table's business and not the
/// transport's.
fn write_on(slot: u64, bytes: &[u8]) {
    let mut rest = bytes;
    while !rest.is_empty() {
        let (w0, w1, w2, n) = byte_sink_protocol::pack(rest);
        send(slot, w0, w1, w2);
        rest = &rest[n..];
    }
}

user_mode_runtime::panic_handler!();
