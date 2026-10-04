//! **A timed window can exclude preemption, and this is the test that says so.**
//! Milestone 541 (a timed window that excludes preemption).
//!
//! `kernel/src/bench.rs`'s `map_new` masks interrupts across its timed window, because the window
//! is roughly 250 microseconds of guest time against a 10 ms scheduler tick (`TICK_HZ` is 100 on
//! all three architectures) and whether a timer interrupt lands inside it is therefore decided by
//! the phase the boot left the timer in. On 2026-09-21 a semantically identical rewrite of one
//! expression in `schedule()` moved that phase, one preemption landed in the window, and the row
//! read +26.4% and failed the tripwire while the map path's object code was byte-identical. See
//! notes/benchmarks.md and design/roadmap/541-a-timed-window-that-excludes-preemption.md.
//!
//! **The benchmark build is not the tested build**, which is the gap this file closes. `--features
//! bench` compiles the tour out and runs no tests, so nothing in the test suite ever executes
//! `map_new`'s masked window; what the suite *can* test is the primitive that window rests on,
//! which is that `arch::interrupts::disable` actually suppresses the scheduler tick and `restore`
//! actually brings it back. If either half stopped being true, `map_new` would go back to timing
//! the scheduler and would say nothing about it.
//!
//! These tests spin against the wall clock rather than counting instructions, so they hold under
//! `-icount` and under HVF alike: the counter `now()` reads advances in both.
//!
//! **What they do not trust the wall clock for is the tick being raised.** Both used to assume that
//! a masked spin of three tick periods guarantees a tick is pending when it ends. It does not: the
//! emulator raises the timer from its own main loop, and on 2026-09-24 that measured 1 to 5 ms
//! after the deadline typically and **86 ms** at worst, over 2,000 masked windows on the riscv64
//! `sifive-u54` model. So `unmasking_delivers_the_tick_that_was_held` failed CI once on `rv64`
//! (pull request #1195, a change to `design/decisions/` alone) saying the held tick had been
//! dropped. The same measurement found the kernel taking the tick at the unmask in every one of
//! 4,000 windows once it was pending, so the likely reading of that red is a tick not yet raised
//! rather than one dropped; the log cannot say which, and that is the gap. Each window now ends only once the
//! architecture says the tick is pending ([`until_the_tick_is_raised`]), which makes the first test
//! non-vacuous and takes the host out of the second. See notes/load-sensitive-assertions.md.

use crate::sched;

/// How long to spin, in counter ticks: three scheduler tick periods, so a tick is unambiguously
/// due rather than marginally due. `frequency()` is counter ticks per second and `TICK_HZ` is
/// scheduler ticks per second, so their ratio is counter ticks per scheduler tick.
fn three_tick_periods() -> u64 {
    3 * crate::arch::timer::frequency() / crate::arch::timer::TICK_HZ
}

/// How long a masked window waits for the host to raise a tick that is already due, in scheduler
/// tick periods: one second, against a worst case measured at under nine periods (86 ms).
///
/// **A leak trap, not a timing claim.** Nothing the kernel does is inside this bound: the deadline
/// has passed and the rest is the emulator getting round to it. A timer that is genuinely broken
/// never raises at all, and this is what turns that into a failure that says so rather than a hang.
///
/// Name: provisional, minted 2026-09-24 (`cda66d656`, waiting for the tick to be raised).
const RAISE_BOUND_PERIODS: u64 = 100;

/// With interrupts **already masked**, spin until this core's tick is architecturally pending, or
/// the bound runs out. Returns whether it was raised.
///
/// This is the difference between "a tick was held" and "enough wall clock passed that a tick
/// should have been held". Only the first is the property these tests are about.
///
/// Name: provisional, minted 2026-09-24 (`cda66d656`, waiting for the tick to be raised).
fn until_the_tick_is_raised() -> bool {
    let bound = RAISE_BOUND_PERIODS * crate::arch::timer::frequency() / crate::arch::timer::TICK_HZ;
    let start = crate::arch::timer::now();
    while !crate::arch::timer::is_tick_pending() {
        if crate::arch::timer::now().wrapping_sub(start) >= bound {
            return false;
        }
        core::hint::spin_loop();
    }
    true
}

/// The message both tests give when the host never raised the tick at all.
const NEVER_RAISED: &str = "the timer was never raised in a second of masked spinning, so this \
     window held nothing and proves nothing: the timer is not being armed, or the emulator has \
     stopped delivering it";

/// **Interrupts masked means no preemption is taken, however long the window is.**
///
/// This is the property `map_new` buys. The spin is three scheduler tick periods, so without the
/// mask the timer would have fired at least twice; with it, this core's count must not move at
/// all. The count is **per-core** (`cpu::PerCpu::preemptions`) for a reason this test found the
/// hard way: on a multi-core kernel the global `sched::preemptions()` reported nine preemptions
/// inside this masked window, every one of them another core ticking its own thread.
///
/// It asserts on the counter rather than on elapsed time on purpose. A window that merely *ran
/// fast* proves nothing, because a preemption that returns promptly is cheap and one that yields a
/// full quantum to another thread is not: the two differed by a factor of seven in the 2026-09-21
/// measurements (6,670 ticks against 47,752 for one preemption on `x86_64`). The count is the
/// thing that is either zero or not.
#[test_case]
fn a_masked_window_takes_no_preemption() {
    let was_enabled = crate::arch::interrupts::disable();
    let before = sched::preemptions_here();
    crate::arch::timer::spin_for(three_tick_periods());
    // **And a tick must actually be waiting**, or the window is vacuous: a host that had not yet
    // raised the timer would pass this test with interrupts wide open. Still masked, so still on
    // this core, and `preemptions_here` below reads the same counter as above.
    let raised = until_the_tick_is_raised();
    let during = sched::preemptions_here();
    crate::arch::interrupts::restore(was_enabled);

    assert!(raised, "{}", NEVER_RAISED);
    assert_eq!(
        during,
        before,
        "{} preemption(s) were taken inside a window with interrupts masked; map_new's timed \
         window is built on this not happening, and if it can happen the row is timing the \
         scheduler again",
        during - before,
    );
}

/// **And the interrupt is deferred, not lost**, which is the other half and the one a reader is
/// right to doubt.
///
/// Masking a window would be a bad trade if it dropped the tick: the scheduler would be starved
/// for as long as the window lasted and the benchmark would have bought its stable number by
/// breaking the machine. It does not. The timer interrupt is pending across the mask and is
/// delivered at `restore`, which is why `map_new`'s own probe can report a non-zero preemption
/// count for a window that took none.
///
/// **The tick is raised before the mask comes off, and that is what makes this test about the
/// kernel.** It used to spin three periods, unmask, spin one more and assert, which asks the host to
/// have raised the timer within forty milliseconds of wall clock; the host once did not, and this
/// assertion reported a dropped tick on `rv64` in a run that changed no code. Now the window waits
/// for the pending bit ([`until_the_tick_is_raised`]), so by the unmask the interrupt is already
/// waiting at this core and delivery is a matter of instructions. The spin after `restore` is kept,
/// bounded at one tick period, because real hardware may take a few cycles to take an interrupt
/// after unmasking, and a period is far beyond that.
#[test_case]
fn unmasking_delivers_the_tick_that_was_held() {
    let was_enabled = crate::arch::interrupts::disable();
    // **Which core is read is fixed under the mask, not before it.** The preemption this asserts
    // on may migrate this thread, and `preemptions_here()` would then read whichever core it
    // landed on: a counter that was never the one being watched. Read here, nothing can move this
    // thread before the interrupt is taken, and `preemptions_on` pins every later read to the core
    // that held the tick.
    let cpu = crate::cpu::id();
    let count = || sched::preemptions_on(cpu);
    let before = count();

    crate::arch::timer::spin_for(three_tick_periods());
    let raised = until_the_tick_is_raised();
    crate::arch::interrupts::restore(was_enabled);

    assert!(raised, "{}", NEVER_RAISED);

    let start = crate::arch::timer::now();
    let period = three_tick_periods() / 3;
    while count() == before && crate::arch::timer::now().wrapping_sub(start) < period {
        core::hint::spin_loop();
    }
    let after = count();

    assert!(
        after > before,
        "a tick was pending when the mask came off and no preemption followed within a tick \
         period, so the tick held across the mask was dropped rather than deferred; masking a \
         benchmark window would then be starving the scheduler rather than excluding it",
    );
}
