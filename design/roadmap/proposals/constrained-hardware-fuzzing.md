---
status: PROPOSED
raised: 2026-09-29
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# Constrained hardware fuzzing for the boot path

Randomized configurations from a constrained model (machine, CPU count, memory, firmware, devices
present and absent) drawn over the QEMU boot matrix's axes, with every seed recorded so every
failure reproduces exactly.

It rides the falsification cadence (`falsifications.yml`), not per-PR CI. It is a sweep over a
configuration space in the same spirit as the mutation and falsification sweeps the tree already
runs: a finding is a finding, not a red trunk, and the sweep cadence exists precisely so that
breadth does not tax every merge.

## Done means

A generator whose config space is the boot matrix's axes, constrained to combinations QEMU accepts.
Every generated config either boots to the suite's pass/fail answer or produces a recorded refusal.
A seed appears in every result, and every failure comes with one command that reproduces it.

Dependencies: the QEMU boot matrix (the axes must exist and be named before they can be drawn
from). Name provisional.
