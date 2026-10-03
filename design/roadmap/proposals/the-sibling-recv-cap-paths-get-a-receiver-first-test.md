---
status: PROPOSED
raised: 2026-10-03
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# The sibling RECV_CAP paths get a receiver-first test

Raised by the lane for milestone 634 (a plain SEND received by RECV_CAP never hands the receiver a
sender-chosen slot), which fixed the plain-SEND leak but left two sibling paths reasoned from the
code rather than measured.

## The finding

Milestone 634 made `RECV_CAP` return `NO_CAP` in `x1` unless a capability was installed for the
delivery, through a `Thread::cap_delivered` default in `kernel/src/sched.rs`. Two other receive
paths flow through the same default and so are now also correct on the receiver-first order: an
interrupt signal's `x1` (which was `0` on that order before, `NO_CAP` on the other) and a death
message's `x1` (which carried the dead thread's id on that order). Neither has a test that drives it
through the receiver-first rendezvous order, so the guarantee rests on reading the code, the grade
`notes/confinement-claims.md` asks a reader to apply to an unmeasured verdict.

## What a test would do

The shape is milestone 634's `a_plain_send_to_recv_cap_delivers_no_cap_whichever_side_parks_first`,
one per sibling: a receiver parks in `RECV_CAP`, the signal or the death is delivered second, and
the test asserts `x1 == NO_CAP`. It should fail on the receiver-first order against a tree with the
`cap_delivered` default reverted (the milestone 634 falsification is the starting point) and pass
with it, on all three ISAs (DECISIONS §19 (architectural parity is a tenet)).
