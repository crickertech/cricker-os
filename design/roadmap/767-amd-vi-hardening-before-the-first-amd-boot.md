---
status: PARTIAL
raised: 2026-10-05
promoted_from: amd-vi-hardening-before-the-first-amd-boot
milestone_dependencies: 764
decision_dependencies: 20
machine_requirements: an AMD machine with an IOMMU, to confirm on silicon; QEMU for the rest
specific_machine: none
needs_person: no
---
# 767. AMD-Vi hardening before the first AMD boot

Raised 2026-10-05 (UTC) by lane/633-outsider-2, milestone 633 (an outside agent attacks the
confinement claim)'s second pass, and promoted from `design/roadmap/proposals/` the same day by
`lane/amd-vi-hardening`, which calef approved. The number 767 is provisional until the queue lands
it. *(Title, slug and every name below are drafts.)*

Milestone 764 (AMD-Vi confines device DMA on x86-64) built the driver. The second outsider pass
read it as a confinement boundary and recorded five gaps in `kernel/src/arch/x86_64/amd_vi.rs`'s
`BUGS`. Three are things a first AMD board would meet on its first boot, and QEMU models none of
the firmware state behind them. This milestone is those three, so the first AMD boot is not also
the first time they are discovered.

**Reuse:** the VT-d driver's `VTD_PERMITTED_BITS` literal and its Kani harness are the model for
the entry proof. QEMU's `pci-bridge` device makes the alias item testable before a board. Linux's
AMD-Vi driver clears the exclusion registers at init, the prior art for item 1 (from memory, not
re-read). Nothing external is taken.

## Acceptance items

1. Clear the firmware exclusion range. `set_up` writes the Exclusion Base and Limit registers to
   zero before enabling the unit, and the boot line prints what it found.
2. Confine and quarantine through aliases. Two devices sharing a source id never share a
   domain silently, and a quarantine resets every entry the device's DMA can arrive under.
3. Honour an IVMD's read-only bit. A DMA mapping carries the rights its source asks for, rather
   than always read-write.

## Follow-on

- **Outstanding.** Items 1 and 2, in progress on `lane/amd-vi-hardening` (checked 2026-10-05).

## Index row

An AMD machine's firmware can leave a hole in the IOMMU (an exclusion range), alias two devices onto
one table entry, or ask for memory to be read-only, and the AMD-Vi driver honoured none of it. This
closes all three before the first AMD board boots.
