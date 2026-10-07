---
status: BUILT
raised: 2026-08-01
built: 2026-08-23
---
# 62. Tests that assert on time: make a red run mean something

Built (2026-08-23). Raised 2026-08-01, from evidence rather than from taste. The token
read `NOT-STARTED` until 2026-08-17, by which point most of what this block asks for had been built
by other lanes, chiefly milestone 78's four rounds and milestone 50's shell work, and nobody came
back to this file. That is why it went PARTIAL rather than NOT-STARTED, and stayed PARTIAL rather
than BUILT for a time; the precedent is milestone 40, whose status said `NOT-STARTED` with two phases
shipped for the same reason. Closed 2026-08-22/23 by the confirmation run described below: 45 of
45 green, the block's own acceptance standard, met on the tree as it now stands.

The premise this block stated was wrong, and the correction matters more than the disposition.
The paragraph below used to say that `script/icount` "asserts zero missed ticks on both ISAs, which
a contended host cannot falsify and which is strictly stronger than either of the two wall-clock
assertions". Milestone 62's lane checked that by injecting the drift defect rather than by reading
the code, and the instrument was blind to it: `script/test` went red and `script/icount` went
green with every number byte-identical to a clean run. It was strictly stronger than the
handler-latency assertion and weaker than the drift one. The instrument now carries the re-arm law
as its fourth claim, and only then was the disposition made. notes/instruction-clock.md has the
numbers.

What is built. The prescribed fix exists by name: `sched::wait_for`
(`kernel/src/sched.rs:3233`), "bounded by the CLOCK rather than by a yield count", and
`sched::within_ticks` (`kernel/src/sched.rs:3260`), budgeted in guest timer ticks, which is this
block's own guest-ticks prescription. The example this block names is converted:
`threads_round_robin` (`kernel/src/sched.rs:3809`) calls `wait_for(all_ran)` instead of giving twenty
yields. `ticks_arrive_at_the_configured_rate` was rebuilt on both ISAs against the re-arm law rather
than against elapsed counter time (`kernel/src/arch/aarch64/timer.rs:382`,
`kernel/src/arch/riscv64/timer.rs:423`). And the watchdog progress heartbeat this block asks for is
`testing::note_progress` (`kernel/src/testing.rs:288`), bumped on every IPC rendezvous, wake and
console line, beside a per-test wall-clock ceiling.

What was left after 2026-08-18, and why it took until 2026-08-22/23 to close. Both items in the
gate line above were answered on 2026-08-17, and the block still was not done, which was worth
saying plainly rather than rounding up. The icount instrument landed with milestone 78 (the
load-sensitive assertions), as `script/icount`; its note is notes/instruction-clock.md. The
acceptance run happened, and **it did not pass on the first attempt**, which is the section below.

So the residual was no longer a missing instrument; it was a **disposition**, and it was made on
2026-08-18. Both assertions are gone from the failing path, neither by a wider bound: one deleted,
one converted to a reported non-measurement, with the arguments per assertion in
notes/load-sensitive-assertions.md and summarized in "The disposition" below.

The heartbeat that landed credits work by *any* thread rather than per test, and
`kernel/src/testing.rs:48` records that this blinded it once for real; that limitation is stated where
a reader meets the feature, which is what this project asks of a known cost.

## The acceptance run, 2026-08-17: 45 runs under load, 36 green

The evidence this block asks for, taken by `script/repeat-under-load` (one busy loop per host core,
the full suite, the load average sampled every ten seconds), on an eight-core Mac at a one-minute
load average between 26.1 and 63.0, over 108 minutes. The full per-run table, the diagnosis of
every red, and the honest limits are in notes/load-sensitive-assertions.md. Nine of forty-five runs
went red, and the shape of those nine is the result rather than the count:

- Eight were two assertions, twice each, on both ISAs. `ticks_arrive_at_the_configured_rate`'s
  eight-attempt retry budget, and `the_handler_keeps_up_when_no_lock_is_held`'s taxonomy cut. Both
  were already named in that note's BUGS section as residuals that only the icount instrument can
  close, and neither had ever been observed before this run; one of the two entries called itself
  "rarer by orders" and has been corrected in place. That instrument now exists, which is what turns
  these eight reds from a wait into a decision. The perfect ISA symmetry is what says these are
  properties of the assertions rather than of either architecture.
- The ninth was a real kernel bug: a double free of a frame during `DESTROY`'s reclaim of a
  region whose resident was blocked in `recv` (riscv64, one occurrence). Recorded in the BUGS section
  of notes/object-revocation.md. *It got its own lane, and it is fixed: PR #316* (2026-08-18).
  **It was not riscv64-only**, which is the correction worth carrying rather than the fix: the
  ownership defect reproduces deterministically on aarch64, and riscv64 is merely where the timing
  exposed it. Two ingredients were needed. `user_aspace_create` builds an address space inside a
  region the caller already holds an `Untyped` to, where `AddressSpace::new` carves its own, and
  both stored a bare `u64` whose `Drop` called `untyped::destroy` unconditionally: two names for one
  run of memory, with a pin argument that holds only for a drop under `SCHED`, which
  `sched::finish_switch` releases before dropping. And `untyped::destroy` released the `REGIONS`
  lock between deciding and removing the slot, so two callers could both pass the refusal check.

  So this run's ninth red is gone and the eight timing reds are the whole of it, which makes
  this block a pure disposition question with no correctness bug behind it. That is a better
  outcome than it looks: the run's own conclusion was that "the one red worth reading arrived
  wearing the same color as eight that were not", and the eight are now dispositioned and the one
  is fixed.

Nothing was widened to make this green, and the run is the argument for not widening: the retry
budget's implicit second claim is already asserted properly by the sibling test, which passed in the
same log a line before the panic.

**The most useful thing the run measured was not the load average.** Runs sharing the host with
another lane's emulator went red 6 times in 17; runs with only their own went red 3 times in 28. Run
42 passed at the highest peak load in the table (63.0) and run 11 failed near the lowest (33.8). A
competing emulator predicts these failures where a load average does not.

The count in this block is stale, and left below as written rather than edited into the prose,
because the argument it supports is history: `sched.rs` holds **6** of these spins now, not nineteen.
Tree-wide the shape matches 19 sites, but 9 are not test code at all, and none of the 10 in tests has
the flake-prone shape. They are all "let it settle, then prove nothing more happened", which is a
negative assertion a loaded host cannot fail in the failing direction.

## The disposition, 2026-08-18: one deleted, one told to say when it measured nothing

The acceptance run ends by naming the question rather than answering it. This is the answer, and it
is two answers, because the two assertions are not the same kind of thing. The full arguments, the
injections and the transcripts are in notes/load-sensitive-assertions.md; this is what the block is
owed.

The diagnostic this round adds, and it generalizes past these two. The first round sorted this
family by the **direction** of a failure. That is still the first question to ask. The second is
what band of the measured quantity makes the assertion fire, and what else lands in that band.
Where the defect and the host produce the same band, no threshold inside it separates them, and the
only choices are a flake or a false pass. That is not a bound to tune; it is an assertion measuring
a quantity it cannot attribute.

`ticks_arrive_at_the_configured_rate`: the law is untouched, the retry budget stops failing. The
re-arm law is exact and a contended host cannot falsify it, because a deschedule long enough to slip
the grid increments the miss count and the window is thrown away and retried. Only the budget's
exhaustion was load-sensitive, and the assertion said so in its own words ("either the host is too
contended to observe the grid, or the handler is slower than a whole tick period"): the deciding
word is that "or", and the guest is the one party that cannot read it. Exhaustion now prints an
`UNMEASURED` line carrying the miss count and the last miss's lateness, and returns without a
verdict rather than inventing one.

`the_handler_keeps_up_when_no_lock_is_held`: deleted, both ISAs. Its true-positive band and its
false-positive band are the same band, measured rather than argued. A handler slow by 1.5 tick
periods failed it correctly. A handler slow by **2.5** periods **passed** it, printing "the emulator
was descheduled; not this kernel's bug, not failed": a false exoneration, on the worst timer defect
this kernel could have. A real host deschedule inside that same band failed it twice per ISA in the
acceptance run above, at 0.56 and 0.83 of an interval. `script/icount` bounds the same handler at
2,500 instructions against a measured 1,056 and 900 with zero variance across 64 ticks.

The honest cost, run rather than estimated. With both applied, the full aarch64 suite exits 0
under a handler slow by 2.5 periods on every other tick, printing `UNMEASURED` where it used to go
red. The suite has stopped claiming anything about handler latency. That trade is only defensible if
the instrument that does catch it is run, so `script/gates` now runs `script/icount` (about
seven seconds for both ISAs, above `script/test` on that script's own cheapest-first rule); CI
already ran it on every non-documentation change.

The first installment of the evidence, taken the same day (`script/repeat-under-load -n 18 -s 8`,
tree `01474c8e`, full table in notes/load-sensitive-assertions.md):

| | 2026-08-17, before | 2026-08-18, after |
|---|---|---|
| runs / ISA legs | 45 / 90 | 18 / 36 |
| 1-min load average, min to peak | 26.1 to 63.0 | **16.2 to 116.6** |
| red at either dispositioned assertion | 8 legs | **0** |
| green runs | 36 of 45 | 16 of 18 |
| re-arm law reported `UNMEASURED` | n/a | **9 of 36 legs, 25%** |

The load was harsher than the run that produced the finding, which is the direction that makes a
green result mean something. Both reds are a **different** assertion, `smp.rs:1010`'s
`a_migrated_kernel_thread_keeps_its_hart_pointer`, a new member of the family this block is about:
it gives 60 migration workers two seconds of counter time to drain and 59 arrived. Its fix is this
block's own prescription, a budget in delivered guest ticks rather than in counter time
(`sched::within_ticks` exists), and it wants a lane.

What was still needed to move this block off PARTIAL, at the time this was written. Eighteen
runs is not forty-five, and the 25% figure was the one that wanted the bigger count: it was the cost
this disposition introduced, higher than a reader would assume, and it came from one host. The
candidate fix was identified and deliberately not taken here (the quarter-second measurement window
is far longer than the law needs, and shortening it is not a wider bound because the `assert_eq!` is
untouched and the defect fails on the first tick), because it would have been a change justified
entirely by a rate resting on 18 runs. The confirmation run below is that bigger count, and it
found the 25% figure did not survive it: zero of 90 legs reported `UNMEASURED` at 45 runs.

## The migration drain, 2026-08-18: the family's newest member, fixed in the prescribed unit

The disposition's own acceptance run found `smp.rs`,
`a_migrated_kernel_thread_keeps_its_hart_pointer`, red twice in eighteen loaded runs with
`migration workers never drained (59/60 done)`, and handed it on. It is fixed, in this block's own
words rather than by a wider bound: the per-wave drain is budgeted in delivered guest ticks
(`2 * TICK_HZ`) instead of two seconds of `timer::now()`, so a descheduled emulator stretches the
budget instead of spending it. The arithmetic was already written once for `sched::within_ticks` and
is now `testing::TickBudget`, which both waits share.

Both halves of the assertion were watched failing, which is what this block asks of a bound that
moves: the stale-`tp` defect injected inside the test's own window is red at the drain loop's line
with the message naming a stale `tp` across a hart migration, and a wave rigged never to finish is
red with `migration workers never drained (5/12 done) within 200 delivered ticks`. The unscoped
version of the first injection never reached the target test at all, which is its own finding and is
recorded in notes/load-sensitive-assertions.md.

**This did not by itself make the block BUILT.** It closed one site the acceptance run found; the
scope note's backlog of unaudited sites was untouched, and the acceptance question ("does the suite
survive repeated loaded runs") still belonged to whoever ran the instrument next. That is the
section below.

## The confirmation run, 2026-08-22/23: 45 of 45 green, the block closes

The one item left after the disposition and the migration-drain fix: a repeat count under load of
the tree as it now stands, at the block's own 45-run standard, because a flake cannot be shown fixed
by a green run and the same rule applies to the change that removed one. Full detail, every run in
order, and the confidence-interval arithmetic are in notes/load-sensitive-assertions.md ("The
confirmation run, 2026-08-22"); this is the number the block is owed.

`script/repeat-under-load -n 45 -s 8`, the same recipe and the same host as 2026-08-17, run against
tree `50a0e7cb` (both the assertion disposition and the migration-drain fix in place). Host: Mac15,3,
8 cores. 45 runs, 112 minutes of wall clock. One-minute load average across the whole loop: 4.8
low, 90.1 peak, a wider band on both ends than the first acceptance run's 26.1 to 63.0.

Result: 45 of 45 green. `ticks_arrive_at_the_configured_rate` ran on both ISA legs in every run
(90 legs) and never printed `UNMEASURED`. `the_handler_keeps_up_when_no_lock_is_held` is gone from
the suite, so it could contribute neither a red nor a false pass.
`a_migrated_kernel_thread_keeps_its_hart_pointer`, the migration-drain fix's own target, was green
in all 45 runs. No other assertion in the suite went red.

**The honest bound this run sets, not a claim wider than that.** Zero red in 45 trials does not
prove the true rate is zero; by the rule of three it bounds it at roughly 6.7% (95% confidence) at
the full-suite-run granularity, and roughly 3.3% at the 90-leg granularity of the specific
assertion. Both are compatible with the 2026-08-17 run's own observed rate if the disposition
lowered it rather than eliminating the failure mode outright. Forty-five was the number the first
acceptance run set as this block's standard, and this run met it; a tighter bound is not what the
block asked for.

This closes the block. The status above moves to BUILT.

## The problem

A population of tests assert on elapsed time or on a fixed number of yields. `sched.rs` alone
holds about nineteen `for _ in 0..N { yield_now() }` spins, and the shape is always the same: give
the scheduler N chances, then assert something happened. `threads_round_robin` gives twenty yields
and asserts every thread ran at least once. `ticks_arrive_at_the_configured_rate` and the riscv
timer-drift assertion compare guest ticks against elapsed counter time.

None of them is wrong about what it wants to prove. All of them fail when the host is busy, because
a yield is not a guarantee and the guest's clock is the host's clock.

## Why this is worth a milestone rather than tolerance

It makes a real regression invisible. On 2026-08-01 it cost the integrator three separate
diagnosis cycles, and two of those ended in the wrong conclusion before being re-run. The credentials
lane hit three flakes, the xattr lane two, the CPU-matrix lane two, and the integrator hit three more
in different tests each time. A suite that fails for reasons unrelated to the change trains everyone
to re-run rather than to read, which is the exact habit that lets a genuine failure through.

Milestone 59 multiplies it fivefold. The CPU matrix runs the same suite five times, so every
timing test now has five chances per run to be unlucky, on a shared CI runner nobody controls.

And the honest diagnostic we rely on is expensive. The current rule is "a green run under load is
conclusive, a red one is not, so re-run quiet." That works, and it costs a full suite run every time,
and it depends on a human remembering to apply it.

## What the fix probably is, per class

- The bounded spins are the easy majority. Waiting for a condition with a deadline is a different
  thing from taking N turns: the test wants "eventually", not "within twenty yields". An
  event-driven wait, or a bound expressed in guest ticks rather than host-scheduler turns, makes
  them insensitive to what else the machine is doing.
- The genuinely temporal tests cannot be made deterministic and should not pretend to be.
  `ticks_arrive_at_the_configured_rate` is *about* the clock. These want an explicit, stated
  tolerance and a recorded retry budget, so a flake is a documented cost rather than a surprise.
- A third class may want to move off the emulator entirely. Scheduling policy is pure logic and
  some of it could be host-tested against a simulated clock, which is where this project already puts
  logic it wants to check in milliseconds.

## The watchdog cannot tell "stuck" from "slow", and it is the same defect one level up

Added 2026-08-04 from `notes/semihosting.md:82`, which records the limitation and names the fix
nobody owns. The suite's own deadlock detector has exactly the problem this milestone is about.

The heartbeat is bumped once per test, at the test's start, in `Testable::run`, and never while
a test runs. So "no progress for 60 s" cannot distinguish a genuine deadlock from a test that is
simply *slower* than 60 s. Both look identical from outside: the heartbeat stops advancing because
no *new* test started.

This has already cost a diagnosis. Milestone 32's FS-server test tripped the watchdog as a false
deadlock. It was not stuck, it was starved: leaked spinning driver threads crammed onto core 0 slowed
the RedoxFS mount past 60 s. Raising the limit made it pass, **which is exactly what a deadlock can
never do**, and that is the tell the current instrument cannot produce on its own. The thread dump
reinforced the wrong read, because a starved thread and a deadlocked one both sit `Blocked`/`Ready`
with nothing obviously moving.

The fix, per the note. A per-test *progress* heartbeat: the test, or the IPC and scheduler paths
under it, bumps a counter as work happens, so a slow test keeps the watchdog fed and a wedged one
does not. The other half already exists and was built while chasing that false deadlock: the
enriched `sched::dump_threads` reports each thread's EL0 PC and the per-endpoint sender, receiver
and pending counts, so two dumps a few seconds apart show whether the pipeline is changing state
(starved but progressing) or frozen.

Until the heartbeat is per-progress, the operating rule is what the note says to do by hand: read a
watchdog trip as "stuck **or** slow", and confirm which by raising the limit before assuming a lost
wakeup. That is a human step in the loop for exactly the reason the rest of this milestone gives, a
red run whose meaning has to be argued about trains everyone to re-run rather than to read.

It belongs here rather than in a harness milestone because it is the same failure the bounded
spins have: a bound expressed in something other than the property under test. Nineteen tests take
N turns and hope; the watchdog takes 60 seconds and hopes. Whatever answers the first should answer
the second.

## BUGS

- Fixing this cannot be verified by running the suite once. A flake that fires one run in six is
  indistinguishable from a fixed one until you have run it many times, so the acceptance evidence is
  a repeat count, not a green run. *(Answered 2026-08-17 by the section above, and the first answer
  was "not yet". `script/repeat-under-load` is the instrument, so the next person did not have to
  build one to re-ask the question. The 2026-08-22/23 confirmation run re-asked it at the same
  45-run standard and got 45 of 45 green, which is what closed the block.)*
- A green acceptance run proves less than this block wants, on its own. Forty-five passes are
  forty-five draws from one host, one QEMU build and one load shape; they say nothing about a GitHub
  runner, and nothing about the assertions that simply did not get unlucky. Milestone 124's lane took
  45 loaded full-suite runs without reproducing the fault it was hunting, which is the standard of
  evidence here and also the warning attached to it. The confirmation run's own BUGS entry in
  notes/load-sensitive-assertions.md states the same limit in numbers: 0 of 45 bounds the true
  failure rate at roughly 6.7% (95% confidence), not at zero.
- Deleting the timing assertions would be worse than the flakes. `ticks_arrive_at_the_configured
  rate` is the test that catches re-arming the timer from `now()` inside the handler, which is a real
  bug this project has a comment about. The goal is tests that fail only when something is wrong, not
  fewer tests. *(Held, and it decided the 2026-08-18 disposition. That test was kept, in full, and
  the injection above proves it still catches exactly that bug where the instrument did not. The
  assertion that WAS deleted is the other one, and the argument for deleting it is that it did not
  fail only when something was wrong: it failed on a busy host and passed on a handler slow by 2.5
  tick periods.)*
- The re-arm law now has two graders and they disagree about which is authoritative.
  `script/test` measures it on a wall clock and may report `UNMEASURED`; `script/icount` measures it
  in instructions and always answers. A reader who sees the suite's `UNMEASURED` line has to know
  that the second one exists, which is why the line names it. Nothing checks that the two bounds
  stay in step, and if the tick interval or the counter frequency ever changed on one path only,
  they would drift apart silently.
- The `UNMEASURED` rate is a new unknown, deliberately taken. Nothing was tuned to reduce how
  often the suite fails to find a miss-free window; the quarter-second window and the eight attempts
  are exactly what they were, so the population of runs that used to go red now goes unmeasured
  instead. That is the right first move (it changes what is claimed rather than how loosely), but it
  means the number to watch has changed and the old measurement does not predict it.

**Effort: not estimated**, and deliberately: the count is known (~19 spins plus a handful of clock
assertions) but how many are mechanical and how many need a rethink is not.

## Follow-on

- **Recorded.** `design/roadmap/0062-time-sensitive-tests.md` BUGS: forty-five green runs bound the
  true failure rate at roughly 6.7% (95% confidence), not at zero, and they are forty-five draws
  from one host, one QEMU build and one load shape. They say nothing about a GitHub runner, and
  nothing about the assertions that simply did not get unlucky.
- **Recorded.** `design/roadmap/0062-time-sensitive-tests.md` BUGS: the re-arm law has two graders and
  nothing keeps them in step. `script/test` measures it on a wall clock and may report `UNMEASURED`;
  `script/icount` measures it in instructions and always answers. If the tick interval or the
  counter frequency changed on one path only, the two bounds would drift apart silently.
- **Recorded.** `design/roadmap/0062-time-sensitive-tests.md` BUGS: the `UNMEASURED` rate is a new
  unknown, taken deliberately. Nothing was tuned to reduce how often the suite fails to find a
  miss-free window, so runs that used to go red now go unmeasured, and the old measurement does not
  predict the new number.
- **Recorded.** `design/roadmap/0062-time-sensitive-tests.md`: with the handler-latency assertion
  deleted, the suite claims nothing at all about handler latency and `script/icount` is the only
  grader left, which is why `script/ci-build` runs it.
- **Recorded.** `kernel/src/testing.rs` records that the progress heartbeat credits work by any
  thread rather than per test, and that this blinded it once for real.
- **Refused.** Shortening the quarter-second measurement window to cut the `UNMEASURED` rate. The
  candidate fix was identified and not taken, because the 25% figure justifying it rested on
  eighteen runs from one host; the forty-five-run confirmation then reported zero of ninety legs
  unmeasured, so there was nothing left for the change to buy.

## Index row

~19 bounded spins (`for _ in 0..N { yield_now() }`) and wall-clock assertions flake under load.
Four separate lanes and the integrator hit them on 2026-08-01; the CPU matrix multiplies the
exposure fivefold. The prescribed fixes shipped via milestone 78's rounds and milestone 50's shell
work; the first acceptance run ran 2026-08-17 (`script/repeat-under-load`, 45 full-suite runs at
load average 26 to 63) and **did not pass**: 9 red, 8 of them the two timing assertions, twice
each on both ISAs, and 1 a real double free of a frame in DESTROY's reclaim path (fixed
2026-08-18 in PR #316, an aspace-ownership defect reproducible on both ISAs rather than a
riscv64 one). The disposition was made 2026-08-18: `the_handler_keeps_up_when_no_lock_is_held` **deleted** on both ISAs, its firing band being exactly the band in which it cannot attribute what
it saw, and `ticks_arrive_at_the_configured_rate`'s retry budget converted to a loud `UNMEASURED`
report, its law untouched. Neither was widened. `script/icount` gained the re-arm law as a fourth
claim first, because an injection showed it was **blind** to the drift bug the record said it
subsumed, and `script/gates` now runs it. A migration-drain assertion found in the disposition's
own 18-run interim check (`smp.rs`, `a_migrated_kernel_thread_keeps_its_hart_pointer`) was fixed
the same day, budgeted in delivered guest ticks rather than counter time. The block's own
standard, a repeat count under load of the current tree, was taken 2026-08-22/23 at the same
45-run scale: 45 of 45 green, load average 4.8 to 90.1, zero `UNMEASURED` results across 90
legs of the retry-budget assertion. Closes the block
