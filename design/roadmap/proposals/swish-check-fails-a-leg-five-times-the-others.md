---
status: PROPOSED
raised: 2026-10-03
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# swish-check fails a leg that costs five times the others per line

Raised 2026-10-03 (UTC) by the maintainer session writing
[the merge-rate correction](../../../notes/coes/2026-10-03-the-merge-rate.md). Name provisional,
this file's alone.

## Why

Behaviour across architectures is gated by §19 (architectural parity is a tenet); nothing gates parity of cost. The
x86_64 leg of swish-check cost 7.7 s a line against aarch64's 0.19 s, about 40 times, for two weeks.
The cause was a kernel scheduling defect, fixed by #1487, milestone 628 (the x86_64 swish-check leg
costs what the others do). A per-line ratio is a better tripwire than the job's wall time for this
class, because it does not move when lines are added and it points at the architecture.

## What to build

- swish-check already times each line. Each leg reports its median seconds per line.
- The check fails when one leg's figure is over five times the median of all legs.
- Compare like with like. After #1487 the x86_64 leg runs under KVM and the others under TCG, and a
  local x86_64 TCG run still costs 2.85 s a line. So the rule compares legs under the same
  accelerator, or each leg against its own committed baseline when no peer shares it.

## Index row

swish-check fails any architecture leg whose per-line time is over five times the median leg's under the same accelerator; x86_64 ran at about 40 times for two weeks unnoticed.
