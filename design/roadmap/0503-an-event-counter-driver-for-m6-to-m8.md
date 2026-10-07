---
status: NOT-STARTED
raised: 2026-09-19
promoted_from: an-event-counter-driver-for-m6-to-m8
milestone_dependencies: none
decision_dependencies: none
machine_requirements: aarch64 and riscv64 silicon; PMU event counters
specific_machine: none
needs_person: yes
---
# 503. An event-counter driver, so milestone 134 (the register of measures)'s M6 to M8 have an instrument

*(Number provisional until the merge queue lands it.)* Promoted from the
proposal `an-event-counter-driver-for-m6-to-m8`, filed 2026-09-19, on calef's instruction of
2026-09-20 to give every proposal on `main` a number. The text below is the proposal's own, unedited
except for this paragraph: the argument is its author's and promotion is not the moment to improve
it. Written by milestone 134's per-IPC stack-depth lane, which re-checked tier B against the tree
after milestone 74 (cycle counters)'s aarch64 half landed (PR #972).

The code can be written and exercised under QEMU, but QEMU counts no cache or
TLB event that means anything; the reading needs radon and argon.

In brief. Milestone 134's M6 (I-cache misses per IPC), M7 (D-cache misses, and whether they fall
in the stack region) and M8 (TLB misses) have no instrument on any architecture. Checked
2026-09-19: nothing in `kernel/src` or `crates/` writes `PMEVTYPER<n>_EL0` or configures an SBI PMU
cache event. The hardware is there: aarch64's boot line reports six event counters visible since
milestone 74's `HPMN` fix, and the SBI PMU extension radon's firmware implements can match
`SBI_PMU_HW_CACHE_*` events.

What to build, in 74's own shape: per ISA, one L1I refill, one L1D refill and one TLB refill
event, started once, read before and after, kernel-internal, printed as `bench-probe:` lines. No
generic event interface; milestone 74's scope note held that back until a second consumer, and
these three measures are that consumer. Milestone 147's profiler would be a third.

**First step, and it is not code:** find out which of these events radon's OpenSBI and argon's
Cortex-A57 actually count, and whether they count them correctly. Milestone 134's own BUGS expects
some not to. That is the same order milestone 74's riscv64 half took.

Two things to settle as it goes. Each `PMEVTYPER<n>_EL0` carries its own exception-level
filter, so the question calef is deciding for `PMCCFILTR_EL0`
(`design/roadmap/proposals/the-aarch64-half-of-74.md`, decision A) comes up again here, and the
answer should be the same one. And M7's attribution half (do the missing lines fall in the stack
region) wants a data-address sampler that neither the A57 nor the U74 has; expect M7 to become
"misses rise with thread count" read beside the per-IPC stack depth, not an attribution.

## Index row

Milestone 134's M6 (I-cache misses per IPC), M7 (D-cache misses, and whether they fall in the stack
region) and M8 (TLB misses) have no instrument on any architecture.
