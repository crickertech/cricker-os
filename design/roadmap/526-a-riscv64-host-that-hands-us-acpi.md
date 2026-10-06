---
status: NOT-STARTED
raised: 2026-09-20
promoted_from: a-riscv64-host-that-hands-us-acpi
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 526. A riscv64 host that hands this kernel ACPI instead of a device tree

*(Number provisional until the merge queue lands it.)* Promoted from the proposal `a-riscv64-host-that-hands-us-acpi`, filed 2026-09-20, on calef's instruction of 2026-09-20 to give every proposal on `main` a number. The text below is the proposal's own, unedited except for this paragraph: the argument is its author's and promotion is not the moment to improve it. Filed by `maintainer/riscv-summit-research` from the RISC-V Summit
Europe 2026 reading (`notes/riscv-summit-2026.md`). *(Number provisional until the merge queue lands
it; the slug is provisional too, like every name a lane mints.)*

Nothing has to be bought, rented or decided first. QEMU's `riscv64` `virt` machine
already generates the tables: its manual documents an `acpi` machine option and says *"When this
option is 'on' (which is the default), ACPI tables are generated and exposed as firmware tables
etc/acpi/rsdp and etc/acpi/tables"*
(https://www.qemu.org/docs/master/system/riscv/virt.html, read 2026-09-20). So the whole of this is
testable on the development Mac today.

## The premise, and where it comes from

The RISC-V Server Platform specification reached 1.0 and is ratified. Verified away from the
talk: `riscv-non-isa/riscv-server-platform` tag `v1.0`, published `2026-05-06T20:48:48Z`, release
note *"First ratified release."*
(https://github.com/riscv-non-isa/riscv-server-platform/releases/tag/v1.0). Radim Krčmář's Bologna
deck carries "Ratified May 2026" on its task-group slide
(https://riscv-europe.org/summit/2026/presentations#P-SPMRUA).

The content slide lists what a conforming platform must present, and two of the entries are the
point of this proposal: BRS-I 1.0, spelled out as UEFI and ACPI, and IOMMU. The stated goal
is that *"OS and hypervisor developers [can] target a single portable binary."*

**This kernel cannot boot that machine.** Every riscv64 entry path in this tree is device-tree:
`notes/visionfive2.md` records the contract as *"entered in S-mode with OpenSBI behind the SBI
calls, `a0` = boot hart id, `a1` = device-tree pointer"*, and `kernel/src/arch/riscv64/` reads a DTB
for the harts, the PLIC, the UART and the timebase. ACPI in this tree is an x86_64 path
(`kernel/src/memory.rs`, `kernel/src/smp.rs`, `kernel/src/pci.rs`), reached through
milestone 322 (one machine matrix for three architectures).

## Why it is worth a milestone rather than a `BUGS` line

Three reasons, in the order this project ranks things.

1. It is the only thing at that summit that lands directly on a port item. Everything else in
   the note is context or absence. This one says: the first riscv64 machine anybody rents as a
   *server* will describe itself the way an ARM server does, and this kernel will not understand it.
2. It is on the rented-capacity path. Milestone 88 (nife on rented silicon) and
   milestone 89 (Scaleway EM-RV1: a second RISC-V implementation, rented) are both gated on hardware
   calef has to provide, and the RV1's TH1520 is a pre-RVA23 embedded part that boots from a device tree, so it
   does **not** exercise this. The machine that will is the one Asanović said the ecosystem still
   lacks (*"Need large-scale deployments, cloud instances to support devs"*, State of the Union
   slide 5), which means the port should be done *before* the box exists rather than in the hour it
   is rented.
3. milestone 322's seam is the reason this is cheap. The machine matrix already exists and
   already has an ACPI consumer. This is a second producer behind an existing seam, not a new
   subsystem, and DECISIONS §19 (architectural parity is a tenet) is what makes "x86_64 can and
   riscv64 cannot" a gap rather than a preference.

## What the work is

Deliberately small, and it ends with a transcript rather than a design:

- Boot `qemu-system-riscv64 -M virt` with ACPI on and read RSDP, RSDT/XSDT and MADT through the
  existing x86_64 parsing, confirming what riscv64's ACPI actually offers (RINTC entries for harts,
  where the IMSIC/APLIC live, whether PCIe ECAM comes from MCFG rather than from the DTB).
- Decide the precedence rule and write it down: a machine may present **both** a DTB and ACPI, and
  the kernel has to say which it believes and why. That choice belongs in the block, not here.
- Make the riscv64 tour boot both ways, DTB and ACPI, from the same binary, and record both
  transcripts.

What would falsify the proposal. If QEMU's riscv64 ACPI tables turn out to describe a machine so
close to the DTB's that the kernel learns nothing new, this shrinks to a scope note under milestone
322 and should be closed as such. That is a cheap thing to find out and it is the first step above.

## What is explicitly not in scope

UEFI. BRS-I names UEFI beside ACPI and this tree has a `uefi_loader`, but booting riscv64 under
EDK2 is a second piece of work with its own firmware problem, and milestone 88's block already
carries a UEFI stage. Doing ACPI alone keeps this to one thing.

The IOMMU line on the same slide. That is milestone 143 (silicon IOMMU: carrying 16b's driver to
a board that ships the ratified spec), which is hardware-gated and stays there; `notes/riscv-summit-2026.md`
records that nothing at the summit moved it.

## Index row

The RISC-V Server Platform specification reached 1.0 and is ratified. Verified away from the talk:
`riscv-non-isa/riscv-server-platform` tag `v1.0`, published `2026-05-06T20:48:48Z`, release note
*"First ratified release."*...
