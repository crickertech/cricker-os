---
status: DECIDED
raised: 2026-08-28
decided: 2026-08-28
ratified_by: calef
---

# 133. Whether an idle core should drain its own inbox before parking

calef, 2026-08-28: no. Raised by the lane that found and fixed the
`place_on` stale-locality lost wakeup (PR #576), which named the fork, recommended declining, and
correctly declined to decide it alone.

## What is being decided

`run_idle` is `try_initiate_steal(); wait_for_interrupt(); yield_now()` forever. It does not look
at its own core's inbox, and neither does `schedule()`. `drain_inbox` has exactly one caller per
architecture, the reschedule-SGI handler, so a missed SGI is permanent rather than late.

The question is whether `run_idle` should drain its own inbox before parking in `wfi`, which would
make any future missed poke self-healing.

## Why it was asked

PR #576 fixed a real lost wakeup. `place_on` decided local-or-remote under `IPC_TABLES` with
interrupts masked, and both callers recomputed that comparison with interrupts enabled; a work
steal between the two reads makes them disagree, and in the bad direction the thread lands in a
remote inbox with no SGI behind it. The target then refuses to rescue itself, because
`try_initiate_steal` returns early when `runnable() > 0` and `runnable()` counts the inbox: the
guard that normally prevents a redundant steal instead pins the core in idle.

An idle-time drain would have turned that wedge into a hiccup.

## Why it was declined, and the reason is about the instrument rather than the cost

A self-healing drain converts a loud failure into a quiet one. The bug above announced itself
as a 60-second watchdog with a full scheduler dump naming the stranded thread, the undrained
inbox, and the core that owed the poke. With an idle-time drain the same bug would have surfaced as
*some threads occasionally start late*, which is a latency artifact nobody files, nobody can
bisect, and no gate measures.

That trade is bad here specifically because the loud failure is the only instrument that found
this. The window is a few instructions wide: the lane measured 0 crossings in more than 1,600
`spawn_on` calls across five instrumented runs, on a quiet host and again under ten burners. It
was not found by reproduction; it was found because one wedge produced a dump precise enough to
name the bug from the trace rings alone. Blunting that is paying a real diagnostic capability for a
class of bug the fix has already removed.

The general form, worth stating because it will come up again: a mechanism that papers over a
missed notification hides the defect rather than the symptom, and this tree's own ladder prefers a
wrong state that cannot be represented, or a gate that fails loudly, over a recovery that makes the
wrong state survivable. An idle drain is the fourth rung wearing the first rung's clothes.

## The case for it, which is real and is why this was a fork

Robustness against the *next* stale-locality bug, which the `place_on` fix does not prevent in
general: `#[must_use]` on an `Option<usize>` makes the current callers say what they owe, but a
future placement path can still forget to poke. The lane recorded the caveat that goes with this:
**"It has not hung" is not evidence that a placement path pokes correctly**, since a skipped SGI is
normally invisible because the next SGI to that core, for any reason, sweeps the whole inbox.

That caveat stands and is not answered by this refusal. What answers it is a gate or a type, not a
recovery, and nobody has built one.

## What would reopen this

- A second missed-poke bug found in a placement path, which would make the pattern rather than the
  instance the problem.
- Real tenancy, where an idle core parking with work in its inbox costs a customer latency rather
  than a test a watchdog. The measurement that would settle it is milestone 74's PMU counters on
  milestone 127's board, not an argument.

## Where this is recorded

`kernel/src/sched.rs`'s `run_idle`, next to the code a reader meets first, and notes/scheduler.md's
account of the lost wakeup.

A note on how this nearly went unrecorded. PR #576's report stated the argument was written
into notes/scheduler.md. It was not: a full-tree grep found `run_idle` in exactly two places, and
neither carried the fork. The argument existed only in a lane report, which is rung four on
CLAUDE.md's own ladder, read once by one person on the day it was written. This section exists
because the decision was checked against the tree before it was recorded, and that is the check
worth repeating rather than the failure worth blaming.
