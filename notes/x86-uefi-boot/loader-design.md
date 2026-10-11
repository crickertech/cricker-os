# The loader's design, in detail: leaving long mode, and the ELF reading

An appendix to [notes/x86-uefi-boot.md](../x86-uefi-boot.md), which keeps the design and the trade
it rests on. This file holds the mode switch step by step, and why the loader reads the ELF itself.

## Leaving long mode, in the order the CPU requires

UEFI hands over in 64-bit long mode, with the firmware's identity page tables live. The kernel wants
32-bit protected mode with paging off. `leave_long_mode.s`:

1. `lgdt` a GDT carrying a 32-bit code descriptor. That is the thing the firmware's GDT does not
   have, and the only reason a GDT is loaded at all.
2. Far-*return* into compatibility mode. A far return is the shape that works from 64-bit mode:
   `jmp far ptr16:32` is invalid there. The instruction is hand-encoded (`48 CB`), for exactly the
   reason `boot.s` hand-encodes its far jumps. LLVM's Intel-syntax parser spells it several mutually
   incompatible ways, and getting it wrong is a triple fault with no output.
3. Reload the data selectors, because they still name descriptors in the GDT we just replaced.
4. Clear `CR0.PG`. This is what actually leaves long mode: `IA32_EFER.LMA` is not a bit software
   writes, it is `LME && CR0.PG`. It is safe only because the trampoline is executing from an
   identity-mapped page, so the next instruction is fetched at the same address either way.
5. Clear `LME`, set `eax`/`ebx`, jump to the kernel.

It is copied to a page below 4 GiB before it runs, and both halves of that matter. It is below 4 GiB
because the second half executes with paging off, where a linear address is a physical one, and the
firmware may well have loaded this image above 4 GiB. And the page is allocated as `EfiLoaderCode`
rather than `EfiLoaderData`. Firmware with a memory-protection policy (OVMF has one, and so does
every recent vendor firmware) sets the execute-disable bit on data allocations. The first instruction
of the copy would then fault, with boot services still live and nothing watching.

## Why the ELF reading is not `crates/elf`

That crate is the kernel's *user-program* loader, and a program is placed at its `p_vaddr`. A boot
loader places an image at its `p_paddr`. On this kernel the two are not related by any offset a
caller could apply:

```text
.text            p_vaddr 0xffffffff80109000   p_paddr 0x109000
.ap_trampoline   p_vaddr 0x0000000000008000   p_paddr 0x165000
```

`elf::Segment` does not carry `p_paddr` at all, so there is nothing to subtract. Adding it is the
tidier answer, and wants its own lane. That struct is public and the kernel's loader is its consumer,
so widening it is a change to a shared definition.

The other trap the physical span has to survive is `NOLOAD`. `.boot_scratch` (the boot page tables)
and the two per-CPU stack areas arrive as `PT_LOAD` segments with `p_filesz == 0` and a real
`p_memsz`. They are address space to reserve, rather than bytes to copy. A loader that skipped them
would leave the firmware free to hand that memory to something else, while the kernel's trampoline
zeroes its own page tables on top of it.
