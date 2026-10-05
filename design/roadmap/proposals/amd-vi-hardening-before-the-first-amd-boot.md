---
status: PROPOSED
raised: 2026-10-05
milestone_dependencies: 764
decision_dependencies: 20
machine_requirements: an AMD machine with an IOMMU, to confirm on silicon; QEMU for the rest
specific_machine: none
needs_person: no
---
# AMD-Vi hardening before the first AMD boot

Raised by lane/633-outsider-2, milestone 633 (an outside agent attacks the confinement claim)'s
second pass, on 2026-10-05 (UTC). Title and slug provisional.

**Reuse:** the VT-d driver's `VTD_PERMITTED_BITS` literal and
`no_vtd_entry_ever_sets_a_reserved_bit` harness are the model for the entry proof; QEMU's
`pci-bridge` device is what makes the alias item testable before a board; Linux's AMD-Vi driver
clears the exclusion registers at init and is the prior art for item 1 (read from memory, to be
confirmed against the source before building). Nothing external is taken.

Milestone 764 (AMD-Vi confines device DMA on x86-64) built the driver and
`notes/confinement-claims.md` has no row for it. The second outsider pass read the driver as that
row's boundary and recorded five gaps in `kernel/src/arch/x86_64/amd_vi.rs`'s `BUGS`. Three of them
are things a first AMD board would meet on its first boot, and none of them can be seen under QEMU,
which models none of the firmware state involved. This proposal is those three, as acceptance
items, so the first AMD boot is not also the first time they are discovered.

## Acceptance items

1. **Clear the firmware exclusion range.** `set_up` must write the Exclusion Base and Limit
   registers to zero (or verify `ExEn` is clear) before enabling the unit, and the boot line must
   print what it found. A firmware-set range with `Allow` is a passthrough for every device, under
   the driver's default deny, that the guest cannot otherwise detect. A test under QEMU can at
   least assert the registers are written; only silicon proves the effect.
2. **Quarantine and attach through aliases.** `attach` and `quarantine` must treat the alias
   source id as part of the device's identity: two devices sharing a source id must not share a
   domain, and a quarantine must reset every entry the device's DMA can arrive under. A device
   behind a PCIe-to-PCI bridge is the case; QEMU can model one (`pci-bridge`), so this is testable
   before the board.
3. **Honour an IVMD's read-only bit.** `build_identity_domain` maps every region read-write. A
   firmware IVMD marked read-only must be mapped read-only (`Flags::user_read_only` or its
   equivalent), and the IVRS parser's `BUGS` entry that admits the bit is dropped closes with it.

## Also worth doing, not gating

- A literal permitted-bits mask and a Kani harness for the device table entry, mirroring
  `no_vtd_entry_ever_sets_a_reserved_bit` (the entry builders move into a crate for that). This is
  the claim 32 that `notes/confinement-outsider-pass-2.md` proposes.
- An inverse of `confine` at the seam, so a device whose driver dies loses its domain, and domain
  ids are reused rather than leaked. This is the revocation claim the same note proposes, and it is
  a cross-IOMMU question (VT-d, SMMU and the RISC-V IOMMU have the same gap), so it may want a
  decision rather than a lane.

## What it does not do

It does not write the `notes/confinement-claims.md` row; that note is over its budget and the
claims are proposed in `notes/confinement-outsider-pass-2.md` for an integrator to place.
