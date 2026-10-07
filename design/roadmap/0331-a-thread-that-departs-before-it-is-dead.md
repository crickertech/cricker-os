---
status: NOT-STARTED
raised: 2026-09-03
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 331. A thread is published `Dead` while it is still executing on its own kernel stack

Filed 2026-09-03 as an unnumbered proposal by the milestone 247 sweep,
from milestone 124's block; numbered 2026-09-19 by milestone 433. **Premise re-checked 2026-09-19 and
it holds.** `crates/thread_wake_handshake`'s `RunState` has six variants (`Embryo`, `Ready`, `Running`,
`Blocked`, `Finished`, `Dead`) and none of them is `Departing`; `depart()` in `kernel/src/sched.rs` still publishes `Dead`, and the window is still
guarded rather than closed, by `region_reap_verdict(state, on_cpu)` refusing while `on_cpu` is set.
The crate is still the one loom searches.

No decision is owed and nothing else is missing. It touches the death protocol and
`RunState` in `crates/wake_handshake`, where loom searches the transitions, so it wants a lane with
the loom search in its gate rather than a hotfix on somebody's way past.

**In brief.** `depart()` publishes a thread as `Dead` at a moment when the thread is still running
on its own kernel stack, and the code that would otherwise free that stack refuses inside the
window instead. Close the window rather than guard it: mark the thread `Departing` in `depart()`,
and promote it to `Dead` from `finish_switch`, which already holds `SCHED` and already runs at the
instant the stack goes free. A remover then never observes a thread whose state says the stack is
free while the stack is in use, and the transient `NotPermitted` that milestone 124 records as a
race stops existing rather than being handled.

## Why this matters

The current shape is rung two of AGENTS.md's ladder, and milestone 124 wrote that down about
itself: the `on_cpu` guard is a condition in one function, and *"the next out-of-band remover can
forget the check exactly as this one did"*. It already happened once. A new state makes the wrong
observation unrepresentable, which is rung one, and it does so at no ongoing cost because the
promotion point is a function that already runs at exactly the right instant with exactly the right
lock held.

The visible symptom today is a caller getting `NotPermitted` from an operation that is not actually
forbidden, only early. That is a defect with a bad shape: it is timing-dependent, it reports the
wrong reason, and a caller that retries succeeds, which is the pattern that survives review and
then wedges a machine once under load.

`RunState` is loom-searched, so the transition set is the thing under test, and adding a state is a
change loom will check rather than a change loom will miss. That is the argument for doing it in a
lane: the mechanism to prove the new protocol correct is already wired to the crate being changed.

## Where it came from

Milestone 124's Follow-on: *"Delete the window in which a thread is published `Dead` while it still
executes on its own kernel stack, instead of refusing inside it: mark it `Departing` in `depart()`
and promote it to `Dead` from `finish_switch`, which already holds `SCHED` and already runs at the
instant the stack goes free. No caller would then see the transient `NotPermitted` this block
records as a race."*

The same block's `Recorded.` bullet is the companion fact: this tree has no type that cannot name a
thread still standing on its stack, and `Departing` is the smallest thing that gets close.

## Index row

`depart()` publishes a thread as `Dead` at a moment when the thread is still running on its own
kernel stack, and the code that would otherwise free that stack refuses inside the window instead of
the window not existing. Marking the thread `Departing` in `depart()` and promoting it to `Dead` from
`finish_switch`, which already holds `SCHED` and already runs at the instant the stack goes free,
closes it: a remover then never observes a thread whose state says the stack is free while the stack
is in use, and the transient `NotPermitted` milestone 124 records as a race stops existing rather
than being handled. That is rung one of AGENTS.md's ladder replacing rung two, and milestone 124
wrote the weakness down about itself: the `on_cpu` guard is a condition in one function, and the next
out-of-band remover can forget it exactly as the last one did. `RunState` is loom-searched, so adding
a state is a change loom will check rather than one it will miss.
