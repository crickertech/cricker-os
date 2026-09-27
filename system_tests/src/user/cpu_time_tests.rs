//! **A thread's scheduled CPU time, as `SURVEY` reports it** (milestone 282 (a thread's CPU time, and the `top` it makes possible),
//! DECISIONS §150 (how does a thread's CPU time reach userspace?)).
//!
//! `survey_record_tests` proves the selector *mechanism*: that a record can be asked for, that an
//! unknown one is refused, that the walk is the same walk whichever record it is. This file proves
//! the one thing no mechanism test can, which is that the **number means what it says**. A record
//! that returned a constant zero, or that charged every tick to the wrong thread, would pass every
//! assertion in that file.
//!
//! Its own file, which is this tree's rule rather than a preference: `tests.rs` is the merge
//! hotspot every lane collides in.
//!
//! # The shape of the proof, and why it is a spinner against a sleeper
//!
//! CPU time is only meaningful as a comparison. An absolute figure could be wrong by any factor and
//! still look plausible, so the assertions here are all **differences between two threads in one
//! domain over one interval**: a runaway that never yields against a child parked in a send that
//! nobody receives. The runaway must gain; the sleeper must not. That pair also happens to be the
//! two ends of the accounting decision §150 made, since a wall-clock age would report the two
//! **identically** and is exactly what this catches.
//!
//! Arch-neutral (DECISIONS §19 (architectural parity is a tenet)), and here that is a claim and not a convenience. The counter
//! lives in `sched`, the increment is in `sched::on_tick`, and the only architecture-specific thing
//! in reach is the one-instruction spin loop, which `force_kill_tests` already keeps for all three.
//! So all three ISAs run literally these assertions.

use abi::survey::record;

use super::supervision_tests::{REPORT_STUB, build_child_in};
use super::survey_tests::{
    TEST_ROWS, arena, child_in, collect_all, drain, hold_supervisor, hold_view, rendezvous, tidy,
};
use crate::arch::exceptions::TrapFrame;
use crate::sched;
use crate::syscall::invoke;

/// A one-instruction runaway: branch (aarch64) or jump (riscv64) to self, forever. It never
/// yields, never syscalls and never touches a rendezvous, so every tick that finds it on a core is
/// a tick it earned.
///
/// The same stub `force_kill_tests` uses, deliberately duplicated rather than shared: that file's
/// copy is a fixture for a teardown test and this one is a workload, and a `pub(super)` shared
/// between them would make either file's edit the other's problem for no gain of three words.
#[cfg(target_arch = "aarch64")]
const SPIN_STUB: &[u32] = &[0x1400_0000]; // b .
#[cfg(target_arch = "riscv64")]
const SPIN_STUB: &[u32] = &[0x0000_006F]; // j .  (jal x0, 0)
/// `x86_64`'s is in `user::x86_programs`, shared with the boot tour; see that module's header.
#[cfg(target_arch = "x86_64")]
const SPIN_STUB: &[u32] = super::x86_programs::SPIN;

/// How long a measurement window is: half a second, which is **fifty timer ticks** at the 100 Hz
/// all three architectures run.
///
/// Long enough that a runaway cannot plausibly collect fewer than the handful of ticks the
/// assertions ask for, even sharing a core with the test thread and whatever else the suite left
/// running, and short enough to be a rounding error against a suite that already spends a second in
/// `force_kill_tests` alone. Expressed in timer counts rather than in iterations because a yield
/// loop's rate is a property of the host, not of the guest.
fn window() -> u64 {
    crate::arch::timer::frequency() / 2
}

/// Spend one [`window`] letting other threads run, then return.
fn spend_a_window() {
    let deadline = crate::arch::timer::now() + window();
    while crate::arch::timer::now() < deadline {
        sched::yield_now();
    }
}

/// `invoke(cap, SURVEY, cursor, record, _)` through the real dispatcher: the three words a
/// userspace caller reads out of its registers.
fn survey_record(slot: u64, cursor: u64, record: u64) -> (i64, u64, u64) {
    let mut frame = TrapFrame::for_user_entry(0, 0, [0, 0, 0]);
    match invoke(&mut frame, slot, abi::rendezvous::SURVEY, cursor, record, 0) {
        Ok(next) => (next, frame.arg(1), frame.arg(2)),
        Err(e) => (e as i64, 0, 0),
    }
}

/// What one record says about one tid, by walking the domain and joining on the tid: exactly what
/// `crates/ps` does, since the cursor and the tid are the same for every record and only the third
/// word moves.
///
/// Panics if the tid is not in the domain, because every caller here has just built it there and a
/// missing member is a failure rather than a `None` worth handling.
fn record_of(viewer: u64, tid: u64, record: u64) -> u64 {
    let mut cursor = 0;
    for _ in 0..TEST_ROWS {
        let (next, seen, word) = survey_record(viewer, cursor, record);
        assert!(next >= 0, "the walk was refused with {next}");
        let next = next as u64;
        assert_ne!(next, abi::survey::DONE, "tid {tid} is not in this domain");
        if seen == tid {
            return word;
        }
        cursor = next;
    }
    panic!("the domain outgrew this test's buffer");
}

/// **The headline: the thread that used the CPU is the thread that is charged for it.**
///
/// One runaway and one child parked in a send, under one supervision endpoint, over half a second.
/// The runaway is charged tens of milliseconds; the sleeper is charged essentially nothing, and in
/// any case strictly less.
///
/// **This is the assertion wall-clock age would fail**, which is why it is the headline rather than
/// a monotonicity check. §150 refused `spawn_instant` on the ground that a thread alive five
/// minutes and a thread that *ran* for five minutes read identically; these two threads are the
/// same age to within the microseconds it took to build the second one, so an age-based
/// implementation reports them equal and this test says so.
///
/// The sleeper is allowed a nonzero figure rather than pinned to zero: it runs briefly on its way
/// to its send, and a tick can land in that window. What it must not do is keep earning while it is
/// blocked, which is what the comparison catches.
#[test_case]
fn the_thread_that_ran_is_the_thread_that_is_charged() {
    let (budget, rendezvous_region) = arena();
    // This test's own two runs, by name. Both are root regions, so their frames are the
    // allocator's own bits and nothing any other core does can move them; the machine-wide count
    // this used to bracket could be moved by anything. See `testing::RegionRun`.
    let budget_run = crate::testing::RegionRun::of(budget);
    let rendezvous_run = crate::testing::RegionRun::of(rendezvous_region);
    let supervision = rendezvous(rendezvous_region);
    let parking = rendezvous(rendezvous_region);

    let sleeper = child_in(budget, REPORT_STUB, Some(parking), supervision);
    assert!(
        super::wait_for(|| sched::rendezvous_waiting_senders(parking) == 1),
        "the parked child never reached its send",
    );

    let spinner_region =
        crate::memory_region::split(budget, super::survey_tests::INSTANCE_PAGES).expect("region");
    let spinner = build_child_in(spinner_region, SPIN_STUB, None, Some(supervision));

    let viewer = hold_view(supervision);
    spend_a_window();

    let spun = record_of(viewer, spinner, record::CPU_TIME);
    let slept = record_of(viewer, sleeper, record::CPU_TIME);

    assert!(
        spun > slept,
        "a runaway ({spun} ms) was charged no more than a thread blocked in a send ({slept} ms), \
         which is what a wall-clock age would report",
    );
    // Five ticks of the fifty the window holds. Not fifty, because the runaway shares the machine
    // with this test thread's yield loop and with whatever earlier tests left running, and a bound
    // that assumed a core to itself would fail on a busy suite rather than on a bug.
    assert!(
        spun >= 50,
        "a thread that did nothing but spin for half a second was charged {spun} ms",
    );

    // Teardown. The runaway is forcibly killed the way `force_kill_tests` does it: the first
    // reclaim arms the kill and refuses, the next tick on whichever core holds it converts it to a
    // corpse, and a later reclaim succeeds. The wait is time-based for that reason.
    let deadline = crate::arch::timer::now() + crate::arch::timer::frequency();
    let mut reclaimed = false;
    while crate::arch::timer::now() < deadline {
        if sched::reclaim_region(spinner_region).is_ok() {
            reclaimed = true;
            break;
        }
        sched::yield_now();
    }
    assert!(reclaimed, "the runaway was never torn down");

    drain(parking, 1);
    let supervisor = hold_supervisor(supervision);
    collect_all(supervisor, &[sleeper]);
    tidy(budget, rendezvous_region, &[viewer, supervisor]);

    // **This test's own frames are back before it leaves**, asked about the two runs it allocated
    // rather than about the machine.
    //
    // This used to be `wait_for(|| free_page_frames() == frames_before)`, and it was written as a
    // courtesy as much as an assertion: a force-killed runaway is converted to a corpse by a tick
    // on whichever core holds it, so the last of its pages can land after this body returns, and a
    // *neighbouring* test measuring the global count either side of its own teardown would see
    // this test's pages arrive inside its window and report recovering more than it ever held
    // (notes/load-sensitive-assertions/unowned-reds.md, "two unowned reds"). The courtesy is
    // retired rather than
    // dropped: after the 2026-09-23 sweep no test in this suite brackets the global count, so
    // there is no neighbour left for a late page to confuse. The assertion half is kept, narrowed
    // to the pages this test is actually responsible for, where `tidy`'s two reclaims make it
    // deterministic instead of a wait.
    //
    // What it no longer covers, stated rather than hidden: the child's own non-region pages, the
    // current-cpu page among them, which belong to no region and so cannot be named from here
    // once the space is gone. `user/current_cpu_tests.rs` is where that frame is asserted on
    // directly, and the whole-boot frame ledger in `testing.rs` is what would catch a drift.
    budget_run.assert_returned("the builder's budget did not come back");
    rendezvous_run.assert_returned("this test's rendezvous region did not come back");
}

/// **A blocked thread's figure does not move**, which is the other half of the claim above and the
/// one a per-tick counter charged to the wrong core would break.
///
/// Read twice, half a second apart, with the thread asleep in a send for the whole interval. Equal,
/// exactly: this is not a tolerance, because a thread that is not on a CPU must be charged for
/// nothing at all.
#[test_case]
fn a_blocked_thread_earns_nothing() {
    let (budget, rendezvous_region) = arena();
    let supervision = rendezvous(rendezvous_region);
    let parking = rendezvous(rendezvous_region);

    let sleeper = child_in(budget, REPORT_STUB, Some(parking), supervision);
    assert!(
        super::wait_for(|| sched::rendezvous_waiting_senders(parking) == 1),
        "the parked child never reached its send",
    );

    let viewer = hold_view(supervision);
    let before = record_of(viewer, sleeper, record::CPU_TIME);
    spend_a_window();
    let after = record_of(viewer, sleeper, record::CPU_TIME);

    assert_eq!(
        before,
        after,
        "a thread blocked in a send for half a second was charged {} ms of CPU",
        after - before,
    );

    drain(parking, 1);
    let supervisor = hold_supervisor(supervision);
    collect_all(supervisor, &[sleeper]);
    tidy(budget, rendezvous_region, &[viewer, supervisor]);
}

/// **A corpse keeps the time it earned**, so a supervisor can still ask what a dead child cost it.
///
/// The reason the counter is zeroed when a slot is *filled* rather than when it is emptied. A
/// survey reports `DEAD` threads on purpose (that is the state `ps` exists to make visible), and a
/// corpse reporting zero, or reporting whatever its slot's next occupant has earned, would be the
/// plausible wrong number this tree has already ruled against.
#[test_case]
fn a_corpse_keeps_what_it_earned() {
    let (budget, rendezvous_region) = arena();
    let supervision = rendezvous(rendezvous_region);
    let parking = rendezvous(rendezvous_region);

    let sleeper = child_in(budget, REPORT_STUB, Some(parking), supervision);
    assert!(
        super::wait_for(|| sched::rendezvous_waiting_senders(parking) == 1),
        "the parked child never reached its send",
    );
    let viewer = hold_view(supervision);
    let alive = record_of(viewer, sleeper, record::CPU_TIME);

    // Let it complete its send and exit. A supervised death is a corpse, not a reap, so it is still
    // a member of the domain and still has a record to report.
    drain(parking, 1);
    assert!(
        super::wait_for(|| record_of(viewer, sleeper, record::STATE) == abi::survey::DEAD),
        "the child never became a corpse",
    );

    let dead = record_of(viewer, sleeper, record::CPU_TIME);
    assert!(
        dead >= alive,
        "a corpse reported {dead} ms, less than the {alive} ms it had already earned alive",
    );

    let supervisor = hold_supervisor(supervision);
    collect_all(supervisor, &[sleeper]);
    tidy(budget, rendezvous_region, &[viewer, supervisor]);
}

/// **The figure is milliseconds, and the granularity is the tick.**
///
/// Every answer is a whole number of tick periods, which is 10 ms everywhere today. Asserted
/// because the unit is a wire contract: a lane that later changed `TICK_HZ` without touching the
/// conversion, or that started reporting raw ticks, would be shipping a number whose meaning
/// changed under every reader, and nothing else in the tree would notice.
#[test_case]
fn the_figure_is_whole_tick_periods_of_milliseconds() {
    let per_tick = 1000 / crate::arch::timer::TICK_HZ;
    assert_eq!(
        per_tick, 10,
        "a tick is no longer 10 ms; check the record's unit"
    );

    let (budget, rendezvous_region) = arena();
    let supervision = rendezvous(rendezvous_region);
    let parking = rendezvous(rendezvous_region);
    let sleeper = child_in(budget, REPORT_STUB, Some(parking), supervision);
    assert!(
        super::wait_for(|| sched::rendezvous_waiting_senders(parking) == 1),
        "the parked child never reached its send",
    );
    let viewer = hold_view(supervision);

    let ms = record_of(viewer, sleeper, record::CPU_TIME);
    assert_eq!(
        ms % per_tick,
        0,
        "{ms} ms is not a whole number of {per_tick} ms ticks",
    );

    drain(parking, 1);
    let supervisor = hold_supervisor(supervision);
    collect_all(supervisor, &[sleeper]);
    tidy(budget, rendezvous_region, &[viewer, supervisor]);
}
