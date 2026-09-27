---
status: PROPOSED
raised: 2026-09-27
milestone_dependencies: 611
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# Contracts leave the implementation crates they live in

Raised by the lane of milestone 611 (every program and crate belongs to a package). Its gate refuses a
link to another package's internal crate, and four of the kernel's links were the same finding: a
layout two programs agree on lives inside the crate of one of them. Pull request #1392 (the system
tests leave the kernel) made the `ps` and `pmap` links dev-dependencies, which the gate does not
check, so only the two driver rows are still exceptions. The finding is unchanged for all four.

## The four

| the kernel uses | from | what it is |
|---|---|---|
| `ps::Row`, `ps::collect`, `ps::MAX_ROWS` | `procps` | the survey record the kernel writes and `ps` reads |
| `pmap::Row`, `pmap::Listing` | `procps` | the mapping listing, the same shape |
| `non_volatile_memory_express::Handoff` and its constants | `drivers` | what the kernel hands the NVMe driver |
| `jh7110_entropy::discover`, `regs` | `drivers` | how the TRNG is found in the device tree |

Each wants its shared half in a crate in `contracts`, leaving the tool or driver with its own logic.
Rule 7 of `AGENTS.md` already says this ("anything two binaries must agree on is a crate"). These
satisfy its letter, being crates, and miss its intent, being the wrong crates.

## Done when

The two driver `exception` lines in `packages/kernel.package` are deleted, the kernel's tests take
`ps::Row` and `pmap::Row` from `contracts`, and `script/lint` passes.
The gate fails any exception whose link is gone, so it will say when each one can go. A name for
each new crate is calef's.
