---
status: PROPOSED
raised: 2026-09-29
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# An SMP bring-up matrix, honestly scoped

One, two, four and eight CPUs across the QEMU boot matrix's machine models, exercising bring-up
and topology: `ap_boot.rs`, per-CPU structures, IPIs, and the placement of CPU-local state.

What this is not: a multicore reliability claim. Fatal risk 5's own text says the concurrency bugs
are ones QEMU cannot show; the instrument for those is radon's soak curve, milestone 201 (is
multicore reliability converging). This matrix catches configuration and bring-up defects, and
every cell's scope note says reliability coverage is silicon's job, so the matrix cannot drift
into overclaiming.

## Done means

Each CPU count boots the suite or scope-notes the gap. A kernel built for SMP never hangs at
bring-up on any cell. No cell reports reliability.

Gated on the multicore work's state: an eight-CPU matrix against a kernel whose multicore story is
mid-flight produces noise, not signal. Dependencies: the QEMU boot matrix. Name provisional.
