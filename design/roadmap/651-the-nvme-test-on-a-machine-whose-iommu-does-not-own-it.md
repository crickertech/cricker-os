---
status: PROPOSED
raised: 2026-09-24
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# The NVMe boot test on a machine whose IOMMU does not own the controller

Raised by milestone 261 (the NVMe driver leaves the kernel)'s bench rehearsal
(`notes/risk-6-bench-evening.md`). Name provisional.

Everything it needs exists under QEMU.

## What it is

The NVMe boot test asserts `confined_by_iommu`, and until 2026-09-24 that was `iommu::is_active()`,
true whenever any IOMMU unit was translating, whoever owned the controller. It is now
`iommu::scope_of(rid).is_confining()`. Nothing yet shows the assertion can go red: every leg the suite
runs has one unit that owns the whole bus.

`cargo xtask disk-throughput --case bypass` already builds the machine that would show it: `-machine
default_bus_bypass_iommu=on` under OVMF makes QEMU's DMAR name no unit for the root bus, and the
bench boot then prints `preflight 1/2 dmar scope : FAIL`. The work is to boot the ordinary test
kernel on that machine, run only the NVMe case (`NIFE_TEST_FILTER`), and require it to fail on the
`confined_by_iommu` assertion, recorded as the claim's replayable falsification in milestone 202 (every confinement test is a ritual until somebody breaks the confinement)'s
convention (`notes/confinement-claims.md`).

## Why it is worth a milestone

The previous assertion would have stayed green on that machine, and very likely on xenon. A
confinement test that cannot come back red is the failure risk 7's entry already found three times.
