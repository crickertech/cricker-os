//! **`free`: the machine's memory, and this prompt's share of it** (milestone 126 (the `procps` package), DECISIONS §225
//! (`free` sees the machine and your share), `crates/free`).
//!
//! Two reads and two sinks. The machine line reads the machine statistics page the progenitor
//! mapped at `machine_statistics_protocol::PAGE_VA`; the "yours" line asks `MemoryRegion::USAGE`
//! twice of this prompt's job budget. Everything that decides what to print is `crates/free`.
//!
//! # Capability contract
//!
//! | slot | what | why |
//! |---|---|---|
//! | 0 | the output sink, `WRITE` | where the table goes |
//! | 8 | the diagnostics sink, `WRITE` | where a withheld page or a refusal is said |
//! | 11 | the machine statistics page, `READ`, also mapped read-only | the `Mem:` line |
//! | 12 | this prompt's job budget, `ENUMERATE` | the `Yours:` line; it cannot spend, split or destroy |
//!
//! Neither view can change anything. The page is written by the kernel alone, and `ENUMERATE` on a
//! region answers what it spent and nothing else.
//!
//! # EXAMPLES
//!
//! ```text
//! $ free
//!               total        used        free
//! Mem:         131072       20480      110592
//! Yours:         2048         320        1728
//! ```
//!
//! # BUGS
//!
//! `crates/free`'s, all of them: no cache columns because the kernel keeps none, no `Swap:` line
//! because nife refuses paging out (pull request #1356), and `Yours:` counts `free` itself.
//!
//! Name: provisional, milestone 126's `free` lane, 2026-09-26.

#![no_std]
// Program entry points, not the crates/ library surface the ratchet of milestone 68 (code-quality
// gates: one lint policy) tracks (DECISIONS §107 (`missing_docs` moves to `workspace.lints.rust`)).
#![allow(missing_docs)]
#![no_main]

use core::sync::atomic::{AtomicBool, Ordering};

use free::{MachineRefusal, Share, ShareRefusal};
use machine_statistics_protocol::{PAGE_VA, Snapshot};
use user_mode_runtime::{exit, invoke, is_granted, send};

const REPORT: u64 = 0;
const DIAG_SLOT: u64 = grant_plan::DIAGNOSTICS_SLOT;
const MACHINE_SLOT: u64 = grant_plan::MACHINE_SLOT;
const SHARE_SLOT: u64 = grant_plan::SHARE_SLOT;

static HAS_DIAG: AtomicBool = AtomicBool::new(false);

#[unsafe(no_mangle)]
pub extern "C" fn _start(_a0: u64, _a1: u64, _a2: u64) -> ! {
    HAS_DIAG.store(is_granted(DIAG_SLOT), Ordering::Relaxed);

    let page = if is_granted(MACHINE_SLOT) {
        // SAFETY: the capability at `MACHINE_SLOT` is granted only alongside a read-only mapping of
        // the same frame at `PAGE_VA` (`system_initializer`'s spawn service does both from one
        // declaration), and that mapping lives as long as this process.
        unsafe { Snapshot::read(PAGE_VA) }.ok_or(MachineRefusal::Unrecognized)
    } else {
        Err(MachineRefusal::Withheld)
    };
    let machine = page.as_ref().map_err(|e| *e);
    let share = if !is_granted(SHARE_SLOT) {
        Err(ShareRefusal::NotHeld)
    } else {
        usage(abi::usage::SIZE).and_then(|pages| {
            usage(abi::usage::COMMITTED).map(|committed| Share { pages, committed })
        })
    };

    free::write_diagnostics(machine, share, &mut |b| write_on(diag_slot(), b));
    if HAS_DIAG.load(Ordering::Relaxed) {
        send(DIAG_SLOT, byte_sink_protocol::eof(), 0, 0);
    }
    free::write_report(machine, share, &mut |b| write_on(REPORT, b));
    send(REPORT, byte_sink_protocol::eof(), 0, 0);
    exit();
}

/// One `MemoryRegion::USAGE` question about the job budget.
fn usage(record: u64) -> Result<u64, ShareRefusal> {
    // SAFETY: `invoke` traps to the kernel, which validates the capability, the right and the
    // record before answering (user_mode_runtime's contract).
    let r = unsafe { invoke(SHARE_SLOT, abi::memory_region::USAGE, record, 0, 0) };
    if r < 0 {
        Err(ShareRefusal::Refused(r))
    } else {
        Ok(r as u64)
    }
}

fn diag_slot() -> u64 {
    if HAS_DIAG.load(Ordering::Relaxed) {
        DIAG_SLOT
    } else {
        REPORT
    }
}

/// Write bytes to an endpoint under the sink contract, sixteen at a time.
fn write_on(slot: u64, bytes: &[u8]) {
    let mut rest = bytes;
    while !rest.is_empty() {
        let (w0, w1, w2, n) = byte_sink_protocol::pack(rest);
        send(slot, w0, w1, w2);
        rest = &rest[n..];
    }
}

user_mode_runtime::panic_handler!();
