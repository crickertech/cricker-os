---
status: BUILT
raised: 2026-09-27
built: 2026-09-27
promoted_from: x86-iommu-first-dma-before-context
milestone_dependencies: 609
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---

# 612. The IOMMU escape-fault test could lose its fault to an unconfined neighbour disk

Promoted 2026-09-27 (UTC) from the proposal `x86-iommu-first-dma-before-context`, approved by
calef 2026-09-27T15:20Z. Built the same day by lane `proposal/x86-iommu-first-dma`. *(Number and
title provisional: the integrator mints the number at merge, and the title is a draft until an
architect names it.)*

## The finding, and a correction to the proposal's premise

The proposal's theory: the test's own PCIe disk made one DMA before its own VT-d context entry
existed. It read that as the device being "for that one access, unconfined". Tracing the boot under
QEMU's `-d trace:vtd_dmar_translate,vtd_dmar_fault,vtd_ce_not_present,vtd_re_not_present` refutes
that framing. So does matching the trace against kernel-side prints at each step of
`register`/`confine`/`attach`. **A request with no context entry is not passed through.** It is
blocked and faulted, reason `0x2` (context entry not present), exactly as VT-d's default-deny is
supposed to do. Default-deny holds throughout. The confinement claim was never at risk.

The real cause is a different device. The test registers *one* PCIe disk (its rid depends on bus
layout; `0x18` in the run that found this). This test's own runners also attach a *second*,
unrelated virtio-blk-pci disk: the RedoxFS fixture from milestone 303 (`x86_64`'s FS service has a
server and no disk it can find). Nothing in a kernel-unit-test boot filtered to this one test ever
registers that second disk. The trace shows why it matters anyway. The moment *any* device's VT-d
context-cache/IOTLB is globally invalidated (`attach`'s own `invalidate_all` always does this),
every virtio-blk-pci function on the bus attempts a real access near the top of guest RAM. Confined
or not:

- The unconfined second disk faults default-deny, reason `0x2`, and keeps refaulting the same
  address on every retry.
- The device under test, now holding a present, correctly-scoped context entry, faults reason `0x1`
  (present context, address outside its own domain) against the same neighbourhood of addresses.
  Its own instance of the same phenomenon, also correctly refused.

Both are the hardware confining exactly what it should. The bug is elsewhere. `CAP.NFR` reports one
fault-recording register (already documented below). The unconfined disk's fault can occupy that
one slot indefinitely, and a fault arriving while the register already holds one is dropped
(`FSTS.PFO`) rather than queued. The escape fault this test provokes on its own device could be the
one silently lost. That is what made the test fail on `x86_64` and nowhere else: aarch64's SMMUv3
and riscv64's IOMMU do not show this behaviour. Neither drives a never-touched virtio-blk device
into a phantom access on a global invalidate the way QEMU's Intel-IOMMU model does here. That
mechanism was traced far enough to fix reliably. It was not chased into QEMU's own source, which is
out of tree and not this milestone's to fix.

## The fix

`the_iommu_faults_a_dma_that_escapes_the_domain` now resets every *other* block device it finds on
the bus before it registers and provokes its own. The reset is `STATUS = 0`, an ordinary virtio
reset, the same operation `provoke_iommu_escape` already uses to stop a queue an earlier test left
running. A plain reset stops a device's DMA outright, regardless of the mechanism behind it. So the
fix does not depend on ever fully explaining QEMU's behaviour. `kernel/src/arch/x86_64/iommu.rs`'s
BUGS section carries the corrected account, cross-referenced from the fault-register entry beside
it.

No syscall surface, no wire format, no new dependency: the fix is nine lines in one test.

## The fault-record drop, as its own finding

The task that raised this asked whether a real bug hides in "the unit has one fault record, so a
second fault is always lost while the first sits undrained." It does not, past what is already
recorded. `CAP.NFR` reports one register on every unit this driver has met (see the BUGS entry),
and this driver cannot grow past that. There is no "read the next fault for rid X" operation a
single hardware record can satisfy. Filtering by rid at read time cannot recover a record the
register already overwrote or dropped in favour of a different one. So it is recorded as a caller
obligation instead: code that reads faults for one device must first make sure no other device on
the bus is still faulting. That is exactly what this fix now does, for the one caller it has
bitten. The BUGS section says so explicitly, so the next caller does not rediscover it.

## The test

`kernel::virtio::tests::the_iommu_faults_a_dma_that_escapes_the_domain`, un-skipped on `x86_64`.
Run by name (`script/test --arch x86_64 --test the_iommu_faults`) five times across two runs: twice
with the RedoxFS fixture attached, twice on the bridged/UEFI leg without it. No failure.
`aarch64`/`riscv64` legs re-ran the same filter to confirm no regression there. `pgrep -l qemu`
clean after each run.

## BUGS

- The exact reason QEMU's Intel-IOMMU/virtio-blk-pci models attempt this access on an unconfined,
  never-driven device was traced (`-d trace:vtd_*`) but not chased into QEMU's own source. It
  reproduces every time on this QEMU version (11.1.1) and is worth a QEMU-side report. Not filed as
  part of this milestone.
- The quiescing loop is bounded at 8 block devices, a sanity cap rather than a measured maximum. No
  runner this tree has attaches more than two.

## Follow-on

- **Recorded.** The QEMU device-model quirk itself: see BUGS above. Nobody owns a QEMU bug report
  for it yet.

## Index row

`the_iommu_faults_a_dma_that_escapes_the_domain` was skipped on `x86_64`. An unrelated,
never-driven second disk on the test bus could win the VT-d unit's one fault-recording register and
starve the escape fault the test provokes on its own device. Both devices' accesses were being
correctly refused all along; default-deny was never breached. The test now resets every other block
device before it runs, and is green on all three architectures.
