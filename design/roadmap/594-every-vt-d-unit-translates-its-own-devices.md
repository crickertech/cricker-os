---
status: BUILT
raised: 2026-09-24
built: 2026-10-04
milestone_dependencies: none
decision_dependencies: none
machine_requirements: x86_64 silicon with two VT-d units and RMRRs
specific_machine: none
needs_person: yes
---
# 594. Every VT-d unit translates its own devices

*(Number provisional: minted by the lane, to be confirmed at merge.)* Promoted
on 2026-09-25 from the proposal `every-vt-d-unit-translates-its-own-devices`, which milestone 261
(the NVMe driver leaves the kernel)'s bench rehearsal filed on 2026-09-24. It also closes milestone
378 (read the DMAR on xenon)'s third item, carrying more than one DRHD. Built and proven on
patagonia, then BUILT on 2026-10-04 by bench boot 1 on xenon
(`bench/xenon-2026-10-04/boot-e-main-clflush-1.log`): both units translating, the screen intact,
the NVMe confined by the catch-all. That boot needed the VT-d table write-back (`Unit::publish` in
`kernel/src/arch/x86_64/iommu.rs`), which lands with this status in the same pull request; on xenon
this milestone does not work without it.

Of the second kind: the machine exists and somebody has to be at it. QEMU
presents one VT-d unit and no RMRRs, so the two-unit route and the RMRR path are proven on host
tables and run for the first time on xenon.

## What it does

The x86_64 kernel used to bring up one DRHD, the `INCLUDE_PCI_ALL` unit since milestone 261's
rehearsal. It now brings up every unit the DMAR names, each with its own root table. Every
`attach` goes to the unit that owns the device, by VT-d 4.1 section 8.3's rule. Each RMRR is
identity-mapped for the devices it names before any unit translates. The boot tour prints one line
per unit and per RMRR:

```text
                vt-d drhd at 0xfed90000, named devices only
                vt-d drhd at 0xfed91000, all (the catch-all)
                vt-d rmrr 0xdd800000..=0xdfffffff (40960 KiB), identity-mapped for the device(s) it names
  vt-d        : drhd 0xfed90000 up (1 of 2, named devices only), translation enabled (gsts.tes confirmed), 1 rmrr device(s) identity-mapped
```

Those addresses are the OptiPlex 7040's, a prediction for xenon rather than a reading of it.

## The pieces

- `crates/machine_discovery/src/acpi.rs`: `DmarUnits` decodes RMRRs beside the units and scopes,
  refuses a malformed RMRR (Linux's `rmrr_sanity_check`), and answers `owner_index` and
  `reserved_for`. `Drhd` carries its register-file size from the `Size` byte.
- `kernel/src/arch/x86_64/iommu.rs`: one slot per unit, `init` in three steps (root tables, RMRR
  domains, then translation), `attach` routed to the owner, faults drained from every unit.
- `kernel/src/iommu.rs`: `confine` adds a device's RMRRs to every domain it builds, as section 3.16
  asks. The SMMUv3 and RISC-V IOMMU report none.
- `memory::vtd_regions` and `mmu::map_everything`: every unit's register file is mapped.

## A device no unit owns

The specification does not say, because by its rules there is none. Section 8.3 wants a DRHD per
segment and a catch-all that takes every unnamed device; section 8.4 wants every RMRR device under
some unit. Linux's answer when firmware breaks this is `-ENODEV` from `intel_iommu_probe_device`:
the device is left out of the IOMMU layer and its DMA is untranslated. This kernel does the same.
`attach` writes no context entry for it, and `scope_of` answers `no drhd owns it`, so nothing can
claim it confined. Refusing it bus mastering was considered and refused. It confines nothing, and
it would silence the preflight that reports the firmware's gap. The rehearsal's `bypass` case is
this path.

## Three driver defects found by reading the specification

Each was latent on QEMU and would have fired on xenon.

1. Domain ids were the requester id. The 7040's units report `CAP.ND` = 2, which is 8-bit
   domain ids, and section 9.3 treats the unused high bits as reserved. The NVMe at 01:00.0 would
   have been domain 0x100, a reserved-field fault on every DMA. Each unit now hands out its own
   ids inside its width.
2. `GCMD` was written one bit at a time. Writing a lone command bit writes zero to the rest,
   so a write-buffer flush would have turned translation off. Section 11.4.4's read-modify-write
   is now the only way a command is issued.
3. The write-buffer flush waited for `GSTS.WBFS` to be set. Hardware clears it on
   completion. Neither QEMU nor the 7040 needs the flush, so it has never run.

It also switches off protected memory regions once translation is on, as Linux does. The 7040's
units offer them and QEMU's does not.

## What proves it

- Host, `cargo test -p machine_discovery`: the 7040's whole DMAR, rebuilt from Linux's log and
  checked by its length (`0xA8`). Each device routes to its owning unit, and each RMRR reaches only
  the device it names. Bridge-scoped RMRRs, malformed RMRRs, a ninth RMRR and the `Size` byte are
  covered too.
- QEMU, `script/test --arch x86_64` and `cargo xtask uefi-test`: the existing suite unchanged, plus
  `every_unit_the_dmar_names_is_translating` and `a_device_no_unit_owns_gets_no_context_entry`.
  Both green on 2026-09-25 (288 passed under PVH, 257 under OVMF).
- QEMU with OVMF, `cargo xtask disk-throughput`: milestone 261's four cases, unchanged. The first
  run lost `lba-4096` to a disk image still locked by the previous case's QEMU. The harness now
  waits for that QEMU to exit, and all four passed in one run on 2026-09-25.
- xenon: see Follow-on.

## BUGS

Each is also written beside the code, in `kernel/src/arch/x86_64/iommu.rs`'s BUGS or
`crates/machine_discovery/src/acpi.rs`'s.

- Only segment 0 is brought up. A unit elsewhere is reported and refused.
- An RMRR that names a bridge is mapped when the device below it is confined, not before
  translation turns on. No machine this tree has met writes one.
- A unit whose scope names only absent devices is still brought up. Linux ignores it; here it
  translates with nothing attached.
- ANDD structures and ACPI-namespace scopes are not decoded, so an I2C or serial controller that
  DMAs is not confined. This kernel drives none.
- The graphics unit translates with nothing but its RMRR mapped. The screen survives only if that
  RMRR covers what the display engine scans; Linux translates the same unit on Skylake and Kaby
  Lake without a quirk. notes/risk-6-bench-evening.md says what to watch.
- **xenon's first boot under this kernel (2026-10-04, at a08efc8dc) tore the screen** as the
  `vt-d ... up` lines printed, the outcome step 2 of the bench note predicted for an RMRR that
  misses the scanout. The same evening showed a likelier cause: the catch-all unit does not snoop
  (`ECAP.C` = 0) and this driver never wrote its tables back, so the graphics RMRR was mapped into
  tables the graphics unit could not read either. That is inference, since the graphics unit's
  `ECAP` was not read. Fixed in `kernel/src/arch/x86_64/iommu.rs` (`Unit::publish`).
  **Answered the same evening:** main with the write-back booted on xenon with both units
  translating and the screen intact (`bench/xenon-2026-10-04/boot-e-main-clflush-1.log`). The
  tear was the cache defect; the graphics RMRR covers the scanout.

## Follow-on

- **Done.** On 2026-10-04 xenon booted this kernel with the table write-back: both units up and
  translating, the screen alive, and the NVMe owned by the catch-all (preflight 1 PASS). The
  first boot, without the write-back, tore the screen (BUGS).
  It must show both units up, the screen alive past the `vt-d` lines, and preflight 1 passing with
  the NVMe owned by whichever unit the DMAR names. That boot is milestone 261's bench evening; no
  second trip is needed.

## Index row

The x86_64 kernel brings up every VT-d unit the DMAR names and routes each device to its owner. It
identity-maps the firmware's RMRRs before translation. Reading the specification found three
driver defects that would have fired on xenon, the worst a domain id too wide for its unit.
Proven on host tables, under QEMU, and on xenon on 2026-10-04 once the tables were written back.
