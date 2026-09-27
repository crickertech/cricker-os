---
status: PROPOSED
raised: 2026-09-26
milestone_dependencies: none
decision_dependencies: unwritten
machine_requirements: none
specific_machine: none
needs_person: no
---
# Warn a dependent without blocking the supervisor

Raised by the lane for milestone 23 (a capability-routed component
OS with live replacement), answering the block's open question of what a supervisor does when a
dependent it must warn before a swap does not answer. notes/non-cooperative-fallback.md has the
options, the prior art and the recommendation.

Then milestone 151 (notification objects: async multiplexing without
wait-any), which is what the build needs. The decision is whether a dependent's warning is advisory. The measurement that says it
can be is built: `swapper`'s `ROLE_UNWARNED` swaps a backend without ever warning `broker`, and the
producer loses nothing.

## Ruled

calef, 2026-09-26: "Make the warning advisory." Build it once milestone 151 lands (#1351).

## What to build

- Replace `broker`'s `BOP_DOWN`/`BOP_UP` `CALL`s with a read-only state page the supervisor writes
  and a notification bound to `broker`'s thread that it signals. The supervisor never waits.
- Keep `ROLE_UNWARNED`'s test, and add one where the signal lands after the swap has finished.
- Close notes/dependency-orchestration.md's `BUGS` entry: a dependent that does not answer no longer
  hangs `swapper`.

## What it unblocks

One of milestone 23's three remaining `Outstanding` lines.
