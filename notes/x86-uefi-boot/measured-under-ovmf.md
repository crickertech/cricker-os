# What booting under OVMF proves, measured

An appendix to [notes/x86-uefi-boot.md](../x86-uefi-boot.md), which keeps the conclusion. This file
holds the PVH-against-UEFI table, the four code paths it exercised for the first time, and the RAM
milestone 195 (finish the UEFI boot path) reclaimed.

## The comparison

The table below was measured with both boots at `-m 256M`, before milestone 195 reclaimed
boot-services memory. The `usable ram` row is the one that moved; the next section records both
figures. The runner's default is now 2 GiB (`NIFE_MEM=256M` reproduces the table). Firmware places
its ACPI tables just under the top of RAM, so the memory size decides what physical addresses the
kernel is asked to read. At 256 MiB they land low enough that a reach bug in the ACPI walk cannot
show. One did, for as long as this script matched its PVH sibling; see notes/x86-port.md's BUGS.

The same kernel, same machine model, same `-m 256M`, booted twice. Everything that differs is the
firmware.

| | PVH (`-kernel`) | UEFI (OVMF) |
|---|---|---|
| memory map | 9 regions | 118 regions |
| `rsdp` in the handoff | `0x0` | `0xfb7e014` |
| RSDP revision / root | 0, RSDT | 2, XSDT |
| PCIe ECAM (from the MCFG) | `0xb0000000` | `0xe0000000` |
| usable RAM | 261627 KiB | 206684 KiB |
| RAM regions the allocator got | 1 | 7 (8 with an archive) |

Four of those are code paths that had never executed:

- The non-zero `rsdp`. `notes/x86-port/acpi-and-pci.md` records `rsdp 0x0` under QEMU's PVH loader.
  So `arch::x86_64::machine::find_rsdp` has always fallen back to scanning the BIOS area for
  `"RSD PTR "`. Under UEFI the pointer arrives in the handoff, and the scan is skipped.
- The XSDT walk. A scanned ACPI 1.0 RSDP has revision 0 and a 32-bit RSDT root. Firmware hands over a
  revision-2 RSDP with a 64-bit XSDT, which is a different branch in `machine_discovery::acpi`. This
  is the assertion `cargo xtask uefi-boot` gates on, because it is the one string that cannot be
  printed by the PVH path.
- An ECAM window that is not the hardcoded constant. Milestone 165 (x86_64 PCI enumeration) made
  `memory::record_pci_regions` follow the MCFG rather than `arch::mmu::PCI_ECAM_PHYS`. Under PVH the
  two agreed, so nothing distinguished "read the table" from "used the constant". Under UEFI they
  disagree, and the kernel follows the table.
- A fragmented memory map: 118 descriptors instead of 9, and the frame allocator comes up over seven
  RAM regions instead of one.

And the userspace archive arrives the same way it does under PVH, through the module list this
loader writes:

```text
  initrd      : 5229568 bytes at 0xd9bc000, 68 programs, from the PVH module list
```

## The RAM that was left on the table, and how much came back

Before milestone 195 it was 206684 KiB against PVH's 261627 KiB. So about 54 MiB of a 256 MiB machine
was reported reserved that a Linux-style loader would reclaim. That was a choice rather than a defect,
in the conservative direction on purpose. Claiming less RAM than exists costs megabytes; claiming
more corrupts something, on hardware nobody can attach a debugger to.

After it: 233148 KiB on the same 256 MiB machine, and 2068244 KiB against 2032128 on a 2 GiB one. Two
classes moved, both of them dead by the time the kernel reads the map:

- `EfiBootServicesCode` and `EfiBootServicesData`, which the UEFI specification says are free the
  moment `ExitBootServices` returns. 26 MiB of the 2 GiB machine.
- `EfiLoaderCode`, which is the loader's own PE image and its one-shot mode-switch trampoline. The
  image is the whole embedded payload (9 MiB for the tour build, 19 MiB for the test build). The
  kernel and the archive were copied *out* of it before boot services ended. 9 MiB more.

`EfiLoaderData` stays reserved, and must. Every allocation this loader makes that the kernel reads
later asks for it: the `hvm_start_info`, the memory map, the module list, the archive, and the kernel
image itself. The asymmetry is not a rule anyone has to remember, which is what makes it hold. The
only thing asking for `LOADER_CODE` is the trampoline, and it asks because firmware sets the
execute-disable bit on data allocations.

What is still reported reserved on the 256 MiB machine is 28 MiB, against PVH's 0.5 MiB. That is the
firmware's own runtime services, ACPI NVS, its reserved ranges, and the loader's `EfiLoaderData`, and
none of it is ours to take.
