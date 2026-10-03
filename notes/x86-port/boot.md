# The x86_64 port: how the machine is entered

*An appendix to [`notes/x86-port.md`](../x86-port.md), which is the page to read. This file holds
the PVH entry, the real-firmware path, and the 32-bit trampoline. It exists to verify or challenge
the main page. A reader who only needs to build, boot or test the x86_64 port should not have to
open it. Moved from the main page on 2026-09-25 (UTC), verbatim apart from links that had to follow
it. The directory `notes/x86-port/` and this file's stem are provisional names, minted by the lane
that split the file; naming is calef's.*

*Records cited below: milestone 87 (the x86_64 bare-metal machine).*

## How the machine is entered, and why it is not multiboot

`-kernel` on x86 wants a Multiboot 1 image, and Multiboot 1 cannot boot a 64-bit kernel: the
specification says the OS image is ELF32 and QEMU enforces it, refusing the image with

```
qemu-system-x86_64: Cannot load x86-64 image, give a 32bit one.
```

Multiboot 2 lifts that restriction and QEMU 11 does not implement it (its x86 loader is one file,
`hw/i386/multiboot.c`, version 1 only). The header is *fatal* rather than ignored, so an image
cannot carry one and fall back to something else.

So this kernel boots by PVH, the direct-boot protocol Xen defined and which QEMU, Firecracker
and cloud-hypervisor all implement. Its whole interface is one ELF note:

| Field | Value |
|---|---|
| name | `Xen` |
| type | 18 (`XEN_ELFNOTE_PHYS32_ENTRY`) |
| descriptor | the physical address to enter at |

QEMU walks the PT_NOTE program headers to find it. That is why `link-x86_64.ld` gives
`.note.Xen` an output section of its own. lld only emits a PT_NOTE for an output section whose type
is SHT_NOTE. Folding the note into `.boot` would make that section PROGBITS, produce no PT_NOTE,
and leave QEMU reporting nothing more informative than a guest that never runs.

PVH turns out to be a better fit than Multiboot quite apart from the ELF64 question, because of what
rides in `ebx`. `hvm_start_info` carries the memory map and the ACPI RSDP address, which is the
root of every table x86 uses in place of a device tree. One pointer in, everything discoverable from
it: exactly the shape `kernel_main(dtb)` already has.

### And real firmware, since 2026-08-30

PVH is a hypervisor protocol and no real firmware speaks it, which is what milestone 87 was for.
The answer turned out to need no change to this file's subject at all. `uefi_loader` is a UEFI
application that places this kernel at its `p_paddr`, builds an `hvm_start_info` out of what the
firmware knows, leaves long mode, and enters the same `_start` with the same register contract
QEMU's PVH loader delivers. So there is one entry point and one handoff structure rather than two,
which is why the suite (the [main page](../x86-port.md)'s status) cannot regress under it.

What that buys, beyond a bootable stick: four code paths on *this* side of the boundary had never
executed, because a hypervisor never took them. `rsdp` arrives non-zero, so `find_rsdp`'s BIOS-area
scan is skipped. The RSDP is revision 2 with an XSDT root rather than revision 0 with an RSDT.
The MCFG's ECAM window is `0xe0000000` rather than the `0xb0000000` that `arch::mmu::PCI_ECAM_PHYS`
also happens to say, so "read the table" is finally distinguishable from "used the constant". And
the memory map is 118 regions rather than nine. See notes/x86-uefi-boot.md, which also carries
the bench procedure for the OptiPlex.

## The trampoline, and why this file is three times its RISC-V twin

PVH (and Multiboot, and GRUB) enter in 32-bit protected mode. A 32-bit instruction stream cannot
name a 64-bit address at all, so unlike the other two architectures the image cannot simply be
linked high and use PC-relative addressing until the MMU comes on. The image has two worlds:

1. `.boot`, linked at its physical address (VA == PA, low), holding the PVH note, the 32-bit
   trampoline, the boot GDT and the boot page tables.
2. Everything else, linked high at `KERNEL_VA_BASE + PA`, with `AT()` telling the loader where
   the bytes go.

The trampoline zeroes the boot page tables (they are `NOLOAD`, so nothing else has), fills them,
and turns on PAE then CR3 then EFER.LME then CR0.PG in that exact order. It then far-jumps through a code
descriptor whose `L` bit is set, and only then jumps to the high alias of itself.

`KERNEL_VA_BASE = 0xffffffff80000000` is not a free choice. `x86_64-unknown-none` has
`code-model: kernel`, which promises LLVM that every symbol is in the top 2 GiB of the address space
so it may use the sign-extended 32-bit relocations that make kernel code compact. Moving the base
breaks every relocation in the image, silently, in a way that looks like random corruption.

The boot map is 4 GiB identity-mapped with 2 MiB pages, the first gigabyte aliased at
`KERNEL_VA_BASE` (where the image is linked), and the same 4 GiB aliased a third time at
`DIRECT_MAP_BASE` (where `phys_to_virt` points; see "One base cannot do both jobs" in [what-does-not-fit.md](what-does-not-fit.md)). 4 GiB rather than the 1 GiB
the image needs, because everything x86 talks to early is above 1 GiB and below 4: the local APIC at
`0xfee00000`, the IO APIC at `0xfec00000`, and q35's PCIe ECAM window at `0xb0000000`. The identity
map is dropped by `mmu::init`; the other two survive it, and the direct-map alias costs eight bytes
because it points at the page directories the identity map already built.

When it fails, it fails silently. A wrong page table means the instruction after `mov cr0, eax`
is fetched through a broken mapping, the CPU takes a page fault with no IDT, escalates to a double
fault with no IDT, and triple-faults. On QEMU that is a machine reset with no output. The
diagnostic is `-d int,cpu_reset -no-reboot`, which prints the full register state at each
escalation.
