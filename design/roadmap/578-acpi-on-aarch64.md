---
status: NOT-STARTED
raised: 2026-09-23
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 578. An ACPI discovery path for aarch64, because every server machine has one

The number is **provisional**: the integrator mints it at merge. Promoted
2026-09-23 on calef's ruling, from the proposal the rented-metal pricing lane filed on PR #1152:
*an ACPI discovery path for aarch64, so a machine that describes itself with ACPI rather than a
device tree can boot nife.* The file name and every name this block invents are provisional too.

A lane can start today, on this laptop, with no account and no board. The failure
this milestone fixes reproduces under QEMU by changing one flag, and the first two items below are
host-tested table parsing. One item inside this block does reach a fork that is an architect's, and
it is named as such where it sits rather than gating the whole thing.

## What this is, and why it is not a purchasing problem

The pricing lane priced aarch64 cloud compute against four requirements and found that **every
aarch64 offer qualifies and nife boots none of them**: AWS Graviton at $0.073 an hour, Oracle's
Always Free Ampere A1 at nothing at all, Azure Cobalt, Google Axion. They take a custom image, they
have a serial console, they have a power API, they have cores. What they hand a kernel is ACPI, and
nife on aarch64 reads a device tree.

That is not a market gap, and it is not the cost of anything. It is one sentence that was already
written down in this tree before anyone went looking, in `uefi_loader/src/arch/aarch64/mod.rs`'s own
`BUGS` section:

> **The device tree must be offered by the firmware.** EDK2 on QEMU `virt` offers it only with
> `acpi=off`; an aarch64 machine that describes itself only with ACPI (most servers) cannot boot
> this kernel at all, which is a kernel limit rather than a loader one.

**Every aarch64 server cloud is such a machine by specification.** Arm's Server Base Boot
Requirements make UEFI and ACPI the boot contract for Arm servers, and the clouds comply: AWS
resolves an arm64 instance with no boot mode specified to UEFI, Azure's Cobalt 100 Arm series is
Generation 2 only, and Google's image-import requirements say the boot disk must support ACPI. The
sources and the dates they were read are in notes/rented-metal.md, which PR #1152 holds.

**The thing this unblocks is not one machine, it is the whole aarch64 column.** Milestone 88 (nife
on rented silicon: Oracle's free tier first, Graviton metal for the PMU) budgeted "a UEFI boot path
and an ACPI front door" in 2026-08-03 and got the UEFI half; milestone 100 (read the machine's PSCI
and its CPU list, not QEMU `virt`'s) said in its own scope note that it "gets the DTB path right and
leaves a clean seam for the second source; it does not build ACPI." This block is that second
source. It is the piece both of them named and neither built.

## The test bench, which costs nothing and already exists as a workaround

`helpers/qemu-stick.sh` boots the stick's `BOOTAA64.EFI` under EDK2, and line 72 carries the
workaround with its reason attached:

> `acpi=off`, because EDK2 withholds the device tree when it presents ACPI, and nife on aarch64
> reads a device tree (measured 2026-09-19; notes/boot-stick.md).

So the failure is an A/B on this laptop, one flag apart. Measured 2026-09-23 by this lane, with
`BOOTAA64.EFI` built from this branch's tip:

```
$ cargo xtask stick                                            # or just the aarch64 payload
$ helpers/qemu-stick.sh aarch64 target/stick                   # acpi=off, the script's default
...
nife uefi_loader: milestone 87
uefi_loader: kernel placed, exiting boot services
nife self-test: 5 of 5 passed

$ helpers/qemu-stick.sh aarch64 target/stick -machine acpi=on  # the failure
...
nife uefi_loader: milestone 87
uefi_loader: the firmware offers no device tree (on QEMU virt, boot with acpi=off; nife on
aarch64 reads a device tree, not ACPI)
Error: Image at 0004B6EA000 start failed: Load Error
BdsDxe: failed to start Boot0001 ... : Load Error
```

Two things about that transcript are worth saying out loud, because they are what make this a cheap
milestone rather than an expensive one:

- **The kernel is never entered.** The loader's own `discover` returns before boot services are
  exited, so the failure is a clean, named refusal rather than a machine that goes quiet. A lane
  working on this gets a readable error on every wrong turn.
- QEMU accepts the override. `-machine acpi=on` after the script's own `-machine virt,acpi=off`
  wins, so no edit to `helpers/qemu-stick.sh` is needed to reproduce the failure. When this
  milestone is done, that trailing flag is the test: the same transcript as the default boot.

No hardware, no cloud account, and no money are needed to do this work or to know when it is
done. The Oracle Always Free experiment the pricing lane proposed is still worth running, for a
different reason: it says what a real SBBR machine's firmware offers, which QEMU's EDK2 only
approximates. It is a corroboration, not a gate.

## What `crates/machine_discovery/src/acpi.rs` already covers

Read in full on 2026-09-23. It is 1,598 lines, it has no architecture in its name and none in its
code, and it is host-tested and Kani-proved (milestone 319 (the crate that parses firmware had no
proofs, and three of its first ones were false), milestone 431 (the ACPI walk is reachable by the
prover for the first time, and proved by nothing)). The generic half is done and this milestone
should add to it rather than start beside it.

| What | Where | aarch64 needs it |
|---|---|---|
| RSDP, both checksums, revision-1 and revision-2 shapes | `parse_rsdp` | yes, unchanged |
| The 36-byte SDT header, and the root table's entry list | `parse_sdt_header`, `root_entry_count`, `root_entry` | yes, unchanged |
| MADT fixed part, and a self-describing entry walk that refuses a zero length | `parse_madt`, `madt_entries` | yes, and the walk needs new arms |
| MADT entry types 0, 1, 2 and 5: local APIC, IO APIC, interrupt source override, local APIC address override | `MadtEntry` | **no. Every one of these is x86** |
| Legacy ISA IRQ routing through the overrides | `isa_irq_table` | no, x86 only |
| MCFG: the PCIe ECAM windows | `mcfg_entry` | yes, unchanged, and it is the same question on both |
| DMAR: Intel VT-d remapping units | `parse_dmar`, `dmar_structures`, `first_drhd` | no. aarch64's IOMMU is the SMMU, described by IORT |

So the table machinery is architecture-neutral and the table *contents* it decodes are entirely
x86. That is the honest shape of the gap: nothing has to be restructured, and four or five decoders
have to be written.

## What aarch64 additionally needs, item by item

Each of these is a table or a field that nothing in this tree reads today. Checked by grep on
2026-09-23: `GTDT`, `FADT` and `SPCR` appear nowhere in any `.rs` file in this repository.

1. MADT entry types 0x0B through 0x0F, which are where aarch64's CPUs and interrupt controller
   live. On x86 the CPU list is type 0 and the interrupt controller is type 1; on aarch64 they are
   GICC (0x0B, one per CPU, carrying the MPIDR and the ACPI processor UID and an enabled flag),
   GICD (0x0C, the distributor's base address and the GIC version), GIC MSI frame (0x0D),
   GICR (0x0E, the redistributor array a GICv3 needs) and GIC ITS (0x0F). Today all five fall
   through `madt_entries`' final arm to `MadtEntry::Other(kind)`, which is the decoder behaving
   correctly and telling you nothing. This is the direct counterpart of what
   `machine_discovery::gic::discover` reads out of the device tree, including the GICv2-versus-GICv3
   distinction whose cost `kernel/src/memory.rs` records beside that call: a kernel that booted,
   printed `interrupts ON`, and took none.
2. GTDT, the generic timer description, which has no decoder at all. It is the table that carries
   the secure EL1, non-secure EL1, virtual and non-secure EL2 timer interrupt numbers and their
   flags, plus `CntControlBase` and `CntReadBase`. A device tree states the same four interrupts in
   `arm,armv8-timer`. This is the gap the pricing lane's Graviton anecdote names precisely: an
   ACPI machine has no timer node, so the generic-timer interrupt IDs have to come from the GTDT or
   the kernel arms nothing and sees no interrupts and no error.
3. The FADT's Arm boot architecture flags, for PSCI. Milestone 100 (read the machine's PSCI and
   its CPU list, not QEMU `virt`'s) already recorded the shape of this: "an ACPI machine has no
   device tree at all and states PSCI in the FADT". Two bits decide whether PSCI exists and whether
   the conduit is `hvc` or `smc`, which is exactly what that milestone reads from `/psci`'s
   `method` property today. Nothing in this tree parses a FADT.
4. SPCR, or the console has nowhere to come from. `console::configure_from_dtb` finds the UART in
   the device tree. ACPI's answer is the serial port console redirection table, which names an
   interface type and a base address. Without it an ACPI boot has no console, which is the failure
   milestone 243 (a machine with no serial port has no way to say anything, and no gate can read it)
   is about, arriving by a different road.
5. **The memory map, which ACPI does not carry at all, and which is the item that makes this a design
   question rather than five parsers.** `kernel/src/memory.rs` gets RAM and the reservations from
   `dtb.memory_regions` and `dtb.reserved_regions`. ACPI has no equivalent: on a UEFI machine the
   memory map comes from the firmware's `GetMemoryMap`, which only the loader can call and only
   before boot services are exited. So this is not a table to add; it is a fact that has to cross the
   handover, and the handover on aarch64 has exactly one register in it.

IORT is deliberately out of scope, and is named here so the next reader does not think it was
missed. It is ACPI's description of the SMMU, the counterpart of the device tree's `smmuv3` node that
`memory.rs` reads today. A cloud instance presents no SMMU to a guest, so it costs nothing to leave
out, and adding it belongs with whatever milestone next touches the IOMMU.

## The loader half, and exactly what aarch64 would mirror

`uefi_loader/src/arch/x86_64/mod.rs` has already done this job once, and the shape is worth stating
because it is three lines of behavior rather than a subsystem:

- `find_rsdp(table)` walks the UEFI configuration table for `efi::ACPI_20_TABLE_GUID`, falling back
  to `ACPI_10_TABLE_GUID`, and keeps the address. The comment there records why 2.0 is preferred:
  its RSDP is revision 2 or later and carries the 64-bit `xsdt_address`.
- `discover` returns that address plus the framebuffer as "the two reads a kernel cannot make once
  the firmware is gone".
- The handover structure carries `rsdp` to the kernel, and `kernel/src/arch/x86_64/machine.rs`'s
  `read_acpi(hint)` walks from there: `find_rsdp` with the hint, then `table_at`, then `read_madt`,
  `read_mcfg`, `read_dmar`.

The aarch64 mirror of the first two is small. `discover` in
`uefi_loader/src/arch/aarch64/mod.rs` currently searches the same configuration table for
`efi::DEVICE_TREE_GUID` and errors when it is absent. Searching for the two ACPI GUIDs as well, and
carrying whichever it found, is the same loop with a second predicate. The x86 GUID constants already
exist in `uefi_loader::efi`.

The third is where the fork is, and this is the one item in this block that is an architect's:

> How does an ACPI machine's description reach the aarch64 kernel? The Linux arm64 boot
> contract this kernel implements is `x0` = the physical address of a device tree, with `x1` to `x3`
> zero, and there is no second register and no slot for a memory map. Three answers, none of them
> obviously right:
>
> 1. The loader synthesises a device tree from ACPI and the UEFI memory map, and the kernel is
>    not changed at all. Smallest diff by a wide margin, and it is what makes the whole handover
>    question disappear. It is also a fabrication: the kernel would print a machine description that
>    no firmware wrote, and the next person debugging a wrong timer interrupt has to know that.
> 2. The kernel grows a second front door, and `x0` points at ACPI's RSDP instead, distinguished
>    by the `RSD PTR ` signature at that address, which no device tree blob can be mistaken for
>    (its own magic is `0xd00dfeed`). Honest, and it means the memory map needs a place to live that
>    is not the device tree.
> 3. A nife-specific handover structure on aarch64, the way x86_64 has `hvm_start_info`. Most
>    room, most surface, and it is a thing two programs agree on, which this tree treats as the
>    expensive category.
>
> **This lane should not pick.** Option 1 is cheapest to build, which by this tree's own tenet is
> the argument that has to be stated as an effort argument rather than dressed as design. Option 2
> is what the device-tree-versus-ACPI split actually is. The decision is a boot contract, it is what
> every future aarch64 machine is entered through, and it belongs in `design/decisions/`.

## The link address, which is the second half and is not this block's to describe

The same `BUGS` section names a second blocker behind the first: the kernel is linked for
`0x4008_0000` (`kernel/link-aarch64.ld`, where `PHYS_START = 0x40080000`), which is QEMU `virt`'s
RAM, and a machine whose RAM starts elsewhere cannot place the kernel there. `ALLOCATION_CEILING` in
the loader is the same constraint from the other side: everything the kernel reads early has to be
below 2 GiB, because `boot.s`'s boot map is one 1 GiB block at `0x4000_0000`.

**That work is milestone 127 (the seL4 machine: a Jetson TX1, so identical silicon referees the
comparison)'s, and this block deliberately does not restate it.** 127's port has to lift exactly this
limit for argon, whose tegra210 DRAM starts at `0x8000_0000`, and the loader's `BUGS` section already
says so in those words: "the same limit milestone 127's port has to lift for `booti`, not one the
stick adds". What this block adds is one fact: the work serves both, and a cloud aarch64 instance
is a second machine that needs it, so whichever lane lifts it should know it has two customers and
not one. Nothing here should be read as re-scoping 127.

A lane taking the discovery items below can get a long way before this bites, because QEMU `virt`
with `acpi=on` has its RAM at `0x4000_0000` like every other `virt` boot. That is the whole reason
the test bench above is honest about what it does and does not prove: it proves the discovery path,
and it cannot prove the placement.

## What to build, as lanes, in order

Each is independently shippable and lands something. None of them needs hardware. The QEMU A/B
above is the test for all of them.

1. The aarch64 MADT entries, in `crates/machine_discovery/src/acpi.rs`. GICC, GICD, GICR, GIC
   MSI frame and ITS as new `MadtEntry` arms, host-tested against a table dumped from QEMU's own
   `virt` with `acpi=on` (which is how the x86 decoders were checked against `q35`), and proved the
   way milestone 319 (the crate that parses firmware had no proofs, and three of its first ones were
   false) proved the rest. Touches no kernel and no loader; merges on its own.
2. GTDT, in the same crate, the same way. A new table decoder with no entry list, so it is the
   smallest of these. Item 1 and item 2 can be one lane or two; they collide only in one file.
3. FADT's Arm boot flags and SPCR, again in the same crate. Smaller still, and both are a header
   plus a handful of fields.
4. The loader finds the RSDP on aarch64. Mirrors `find_rsdp` from the x86_64 half, carries what
   it found, and changes `discover`'s error message to distinguish "no device tree and no ACPI"
   from "ACPI, which this kernel cannot yet use". Half a lane's work, and it makes the failure
   message tell the truth even before anything downstream exists.
5. The handover, which needs calef's decision first. The fork above. A lane can write the
   proposal with the three options priced, which is the shape this tree asks a fork to arrive in;
   it should not pick.
6. The kernel's second front door, once 5 is answered: memory, GIC, timer, CPU list and console
   sourced from ACPI instead of the tree. This is the large one, and it is the only item here that
   touches `kernel/src/memory.rs`, so it wants the machine to itself.
7. Then, and only then, the free experiment: Oracle Always Free Ampere A1, `BOOTAA64.EFI` on an
   EFI system partition, read the serial console. It is not a gate on anything above and it needs an
   account, which is calef's to provide.

## BUGS

- **This block establishes a gap; it does not size the work.** Nothing here was built and no estimate
  is offered, because the handover fork in item 5 changes what items 1 through 4 are worth and
  nobody has decided it.
- **The ACPI table contents in section "What aarch64 additionally needs" are stated from the
  specification and were not read out of a running machine.** The entry type numbers, the GTDT's
  fields and the FADT's flag bits should be checked against a table dumped from QEMU with `acpi=on`
  before a decoder is written against them, which is what the x86 decoders did against
  `hw/i386/acpi-build.c`. This tree has carried a fabricated quotation through every gate before.
- The A/B was run with a debug-profile stick built by hand from this branch, not by
  `cargo xtask stick`, whose three-architecture build was more than this lane needed. The failing
  half is a loader message printed before boot services are exited, so the profile cannot be
  load-bearing, but the transcript above is not a `stick-boot` transcript and should not be quoted as
  one.
- **`-machine acpi=on` proves that EDK2 withholds the device tree when it presents ACPI. It does not
  prove that a cloud machine's firmware behaves the same way**, and the only thing that would is
  booting one. That is item 7 and it is the reason item 7 survives the list.

## Index row

Every aarch64 cloud offer qualifies on price, console, power and cores, and nife boots none of them,
because an SBBR server describes itself with ACPI and nife's aarch64 path reads a device tree. The
loader's own `BUGS` section said so before anyone went looking. `crates/machine_discovery/src/acpi.rs`
already parses the RSDP, the SDT headers, the root table, the MADT's entry walk and MCFG, all of it
architecture-neutral; what is missing is aarch64's own contents: MADT types 0x0B to 0x0F for the GIC
and the CPU list, a GTDT for the timer interrupts, the FADT's Arm boot flags for PSCI, SPCR for the
console, and a memory map, which ACPI does not carry at all and which is why the handover is a fork
for calef rather than a parser. The failure reproduces on a laptop one flag apart,
`helpers/qemu-stick.sh aarch64 target/stick -machine acpi=on`, so no hardware and no account are
needed to do this work or to know when it is done. The `0x4008_0000` link address is the second
blocker behind the first and belongs to milestone 127 (the seL4 machine: a Jetson TX1, so identical
silicon referees the comparison); what this block adds is that the same work now has two customers.
