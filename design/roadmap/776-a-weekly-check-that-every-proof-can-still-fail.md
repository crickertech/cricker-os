---
status: NOT-STARTED
promoted_from: a-weekly-check-that-every-proof-can-still-fail
raised: 2026-10-04
milestone_dependencies: 741
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 776. A weekly check that every proof can still fail

Raised by the risk-2 reach study (`lane/kani-reach`, milestone 741 (does a standing proof notice a
regression)) on 2026-10-04 (UTC). Title and slug are drafts.

The study is a single measurement. A harness that can fail today can be weakened tomorrow (an
added `kani::assume`, a bound lowered). Nothing would say so. `.github/workflows/kani-reach.yml`
already measures it at about 30 Kani CPU-hours for the whole tree. That is too dear to run every
week. A cheap version would be a sample: for each harness, the mutants it killed last
time, proved again on a schedule. The job would fail when a harness that used to kill something now
kills nothing.

## Done when

- A scheduled job proves a stored sample of each harness's previous kills and fails loudly when
  a harness kills none of them.
- Its cost is measured and stated, and it runs at a concurrency that leaves the shared runners free.

## Index row

A Kani harness that can fail today can be weakened tomorrow by an added assumption or a lowered bound. Nothing would say so. A scheduled sample of each harness's previously killed mutants, proved again, would fail when a harness that used to kill something kills nothing, at a cost far below the full 30 CPU-hour sweep.
