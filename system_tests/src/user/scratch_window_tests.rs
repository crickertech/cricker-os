//! **A userspace builder keeps building past its scratch window** (milestone 604 (provisional), the
//! builder's scratch cursor is bounded).
//!
//! `supervision_protocol`'s loader maps every page it fills for a child into its own address space,
//! and nothing in the ABI unmaps it (DECISIONS §162 (whether a holder can give up a mapping) is
//! open). Until 2026-09-26 its cursor only climbed, so a long-lived builder ran out: in the
//! progenitor, first of page tables for the cursor and then of address space below the initrd
//! window, after about seventy-five `ripgrep`-sized spawns. Every build after that failed.
//!
//! The loader now wraps inside a fixed window and asks the kernel which pages are free, and this
//! proves the kernel half of that bargain as well as the loader's: that destroying a child's region
//! takes the builder's scratch mappings of its frames back (DECISIONS §13 (frame revocation)), and
//! that `PageFrame::MAP` refuses a taken page whole, so a probe is safe. The builder is
//! `fixtures/src/scratch_window_exerciser.rs`, whose header has the numbers and the negative control.
//!
//! Cross-ISA: the loader is portable userspace code and the revocation is portable kernel code, so
//! the parity gate (DECISIONS §19 (architectural parity is a tenet)) is met by this one test running
//! on each architecture. `x86_64` and `riscv64` build one more scratch page per child (the timebase
//! page), which the fixture counts.
//!
//! # BUGS
//!
//! - **CI does not run it.** The whole suite skips it; it runs only when named
//!   (`script/test --test scratch_window`). It was run that way on all three architectures on
//!   2026-09-26 (milestone 604's block has the times). A gate that only runs by hand is weaker than
//!   one CI runs, and this is an exception recorded as one: the cost below is the kernel's, and
//!   making the default suite pay it on every leg is what it would take otherwise.
//! - **It is slow in the suite and fast alone, and the kernel's reap is why.** Five seconds on
//!   aarch64 run by itself; 104 s in CI's whole aarch64 suite (run 36279440107), and on `x86_64` it
//!   ran past the leg's own timeout.
//!   `crate::revoke::revoke_region` finds each page to unmap by scanning every live address space's
//!   mapping log from the start, once per page, so destroying a region costs its mapped pages times
//!   every record on the machine. Forty 668-page reaps behind a suite's worth of live spaces pay
//!   that forty times. The progenitor reaping a job pays it too. Proposed as its own milestone:
//!   `design/roadmap/658-a-region-reap-scans-every-mapping-on-the-machine-per-page.md`.
//! - **The test keeps 32 frames, unexplained.** Measured by the suite's frame ledger on aarch64,
//!   run alone, 2026-09-26: both regions reclaim without error and the exerciser exits, yet 32
//!   frames do not come back. 32 is also the scratch window's last-level table count
//!   (`supervision_protocol::SCRATCH_TABLE_PAGES`), which points at the tables the builder's own
//!   region paid for, but that is a guess from one number, not a trace.
//!
//! - **It proves the loader, not the progenitor.** The progenitor's own numbers (its 128-page table
//!   budget, its job pool against the window) are compile-time assertions in
//!   `crates/system_initializer`, not a boot that spawns `rg` a hundred times; `rg` is not in any
//!   CI archive, for the reasons of milestone 121 (`ripgrep` on nife).

use super::*;
use crate::cap::{Rights, memory_region_cap, rendezvous_cap};
use crate::sched;

/// The exerciser's building budget: 7 extra stack pages and 720 for one child at a time, with 33
/// over. Must cover `EXTRA_STACK_PAGES + CHILD_PAGES` in the fixture.
const BUDGET_PAGES: u64 = 760;

/// **The exerciser's own table budget, and the negative control.** 48 pages covers the scratch
/// window's 32 last-level tables and the one or two above them; the old cursor would have needed
/// 53 for this run.
const OWN_TABLE_PAGES: u64 = 48;

/// The builds the exerciser is written to make.
const BUILDS: u64 = 40;

/// How long to wait for the report, run by name: 5 s measured on aarch64 alone, so twelve times
/// that. In the whole suite it took 104 s (see this module's BUGS), which is why it does not run
/// there.
const WAIT_SECS: u64 = 60;

/// Why the whole suite skips this test.
const OPT_IN: &str = "opt-in: 104 s in CI's whole aarch64 suite, and past x86_64's leg timeout, \
                      because every region reap scans every live mapping log per page; run it \
                      with `script/test --test scratch_window` (milestone 604)";

/// **A builder with a 48-page table budget builds 40 `ripgrep`-sized children in a row**, three
/// more than a climbing cursor manages on that budget, and 1.6 laps of its scratch window.
#[test_case]
fn a_builder_reuses_scratch_its_reaped_children_gave_back() {
    if !crate::testing::run_was_filtered() {
        crate::testing::skip!(OPT_IN);
    }
    use core::sync::atomic::Ordering;

    use crate::arch::exceptions::USER_FAULTS;

    let image = program("scratch_window_exerciser")
        .expect("no scratch_window_exerciser program in the archive");
    let region = crate::memory_region::create(BUDGET_PAGES).expect("no region for the exerciser");
    let tables =
        crate::memory_region::create(OWN_TABLE_PAGES).expect("no table region for the exerciser");
    let endpoints = crate::memory_region::create(1).expect("no endpoint region");
    let report = sched::create_rendezvous_from(endpoints).expect("no report rendezvous");
    let faults = USER_FAULTS.load(Ordering::Relaxed);

    let tid = sched::spawn(move || {
        run(
            image,
            Spawn {
                arg0: 0,
                arg1: 0,
                arg2: 0,
                grants: &[
                    memory_region_cap(region),             // slot 0: the budget
                    rendezvous_cap(report, Rights::WRITE), // slot 1: the report
                    memory_region_cap(tables),             // slot 2: its own page tables
                ],
                maps: &[],
            },
        )
    })
    .expect("could not spawn the scratch window exerciser");

    // Bounded, so a builder that faults says so here rather than as the suite's watchdog.
    let start = crate::arch::timer::now();
    let deadline = start + WAIT_SECS * crate::arch::timer::frequency();
    while sched::rendezvous_waiting_senders(report) == 0 && crate::arch::timer::now() < deadline {
        sched::yield_now();
    }
    assert!(
        sched::rendezvous_waiting_senders(report) > 0,
        "the exerciser never reported: it faulted ({} faults) or is still building after \
         {WAIT_SECS} s",
        USER_FAULTS.load(Ordering::Relaxed) - faults,
    );
    let [built, pages, window, ..] = sched::ipc_receive(report);
    crate::println!(
        "    {built} builds, {pages} scratch pages, in {} s",
        (crate::arch::timer::now() - start) / crate::arch::timer::frequency(),
    );

    assert_eq!(
        built,
        BUILDS,
        "build {} of {BUILDS} failed after {pages} scratch pages, against a window of {window}: \
         the loader did not reuse the pages a destroyed region gave back",
        built + 1,
    );
    assert!(
        pages > window * 3 / 2,
        "{pages} scratch pages is not one and a half laps of the {window}-page window, so \
         this run proves less than it claims",
    );

    assert!(
        wait_for(|| !sched::is_thread_present(tid)),
        "the exerciser reported and did not exit",
    );
    assert_eq!(
        USER_FAULTS.load(Ordering::Relaxed),
        faults,
        "the exerciser faulted",
    );
    sched::reclaim_region(region).expect("the exerciser's budget did not come back");
    sched::reclaim_region(tables).expect("the exerciser's table budget did not come back");
    sched::reclaim_region(endpoints).expect("the endpoint region did not come back");
}
