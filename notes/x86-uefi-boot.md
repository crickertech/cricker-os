# Booting x86_64 from real firmware

Milestone 87 (the x86_64 bare-metal machine). The x86_64 port boots under QEMU by PVH, and `notes/x86-port.md`'s `BUGS`
already said what that costs:

> PVH is a hypervisor protocol and no real firmware speaks it. Milestone 87's OptiPlex will need a
> UEFI stub or GRUB's Multiboot.

This note is the answer: what was chosen, what the alternative actually cost when it was priced
rather than argued about, how the thing works, and the exact procedure for the bench, which is
the part of this milestone that a person has to carry out by hand.

## The fork, and it was decided by two commands

Both options were real, both were priced, and this is what the pricing found. The rule the tenets
give here is *recommend on reversible forks*: the two paths can coexist (a GRUB Multiboot 2 header
is thirty lines and does not disturb what is below), so this is a recommendation acted on rather
than a question sent up.

### What UEFI costs

```console
$ ls /opt/homebrew/share/qemu/ | grep edk2-x86_64
edk2-x86_64-code.fd
edk2-x86_64-secure-code.fd
```

OVMF, the open-source UEFI implementation, ships with the QEMU this project already pins.
Nothing to install. And the FAT filesystem the firmware reads is QEMU's own `vvfat` block driver,
which synthesises one out of a host *directory*, so there is no image-building step and no
`mtools`:

```console
$ command -v mformat xorriso
$                                   # neither is installed
```

On the machine end, the OptiPlex 7050 is UEFI-native, so the operational cost is **copying one
file to a FAT32 stick** as `/EFI/BOOT/BOOTX64.EFI`. That path is the removable-media fallback every
UEFI implementation looks for with no configuration at all.

The engineering cost is what §46 would have to weigh if a dependency were involved, and there is
none: this loader speaks six firmware functions and two GUIDs, hand-written in
`uefi_loader/src/efi.rs`. The `uefi` crate was not taken, and the reason is that crate's size
against this need rather than anything about its quality.

### What GRUB costs

```console
$ brew info grub
Error: No available formula with the name "grub".
```

GRUB cannot be installed on this development machine at all. Homebrew has no `grub` formula on
macOS, so `grub-mkrescue` cannot be run here, and neither can `xorriso`, which it needs. The GRUB
path could still be *written* on patagonia, but it could not be proved on patagonia: gating it
would mean a Linux container in the loop for every run, or building GRUB for the `x86_64-efi`
target from source.

On the machine end GRUB also costs calef more, not less: a bootloader installed on the box, or a
rescue ISO written to a stick with a tool he does not have.

### The decision

UEFI. It is testable today with what is installed, it is the shorter path to the machine, and
it is what the target hardware natively is. GRUB Multiboot 2 stays available and cheap to add if a
BIOS-only machine ever turns up: the 32-bit trampoline in `kernel/src/arch/x86_64/boot.s` is
already the entry state GRUB delivers, so the delta would be a header and a second handoff decoder.

And the Multiboot hazard was checked rather than assumed. `notes/x86-port/boot.md` records QEMU
refusing an image over a Multiboot 1 header, fatally. Nothing in this milestone adds a
Multiboot header of any version, so that hazard is untouched: the PVH note is still the only boot
header in the image, and `script/test --arch x86_64` still boots through it.

## The design, and the one decision the rest follows from

The kernel is not modified. It is entered through its existing `_start`, in 32-bit protected
mode, with PVH's register contract: `eax` = `0x336EC578`, `ebx` = the physical address of an
`hvm_start_info`. The kernel cannot tell which loader started it.

That was chosen against the obvious alternative, which was a second 64-bit entry point in the
kernel that rebuilt the boot page tables in long mode. Two entries would have meant two contracts,
two page-table builders, and a real chance of breaking the PVH path that every
`script/test --arch x86_64` run rides. **Thirty-two instructions in the loader buy one entry point
in the kernel**, and that is the whole trade.

Everything else falls out of it:

| The loader does | Because |
|---|---|
| synthesises an `hvm_start_info` | `machine_discovery::x86_64` already decodes it, host-tested, and `arch::x86_64::machine` already consumes it |
| places the kernel at `p_paddr` | that is what a boot loader does, and this kernel's `p_vaddr` and `p_paddr` are unrelated (see below) |
| leaves long mode as its last act | that is the *only* state difference between what UEFI hands over and what QEMU's PVH loader hands the kernel |
| embeds the kernel and the archive | one file on the stick, and no `SimpleFileSystem` protocol to speak |

The pieces:

- `uefi_loader/src/lib.rs` and its three modules are the pure half: the firmware table layouts
  (`efi`), the `hvm_start_info` writer (`handoff`), and the physical-address ELF reading (`image`).
  All of it compiles for the host and is tested there, for the reason `crates/device_tree_blob` and
  `machine_discovery` exist rather than living inside `arch/`: a structure layout proved only by
  booting is proved by nothing that runs in milliseconds.
- `uefi_loader/src/main.rs` is the half that cannot be: it calls firmware and it changes CPU
  mode.
- `uefi_loader/src/leave_long_mode.s` is the mode switch.

### The handoff is the kernel's own, which is the hazard this closes

Milestone 87's brief named the risk directly: *make both entries produce the same internal
structure for `kernel_main`*, because a divergence would first show up on hardware nobody can
attach a debugger to. Synthesising an `hvm_start_info` is how that is made true rather than
promised. There is one structure, one decoder, and one set of tests, and `uefi_loader::handoff`'s
tests decode their own output with the crate the kernel decodes with, so the writer and the
reader cannot drift apart without a host test failing in milliseconds.

### Leaving long mode, in the order the CPU requires

UEFI hands over in 64-bit long mode with the firmware's identity page tables live. The kernel wants
32-bit protected mode with paging off. `leave_long_mode.s`:

1. `lgdt` a GDT carrying a 32-bit code descriptor, which is the thing the firmware's GDT does
   not have and the only reason a GDT is loaded at all.
2. Far-*return* into compatibility mode. A far return is the shape that works from 64-bit mode:
   `jmp far ptr16:32` is invalid there. The instruction is hand-encoded (`48 CB`) for exactly the
   reason `boot.s` hand-encodes its far jumps: LLVM's Intel-syntax parser spells it several
   mutually incompatible ways and getting it wrong is a triple fault with no output.
3. Reload the data selectors, because they still name descriptors in the GDT we just replaced.
4. Clear `CR0.PG`. This is what actually leaves long mode: `IA32_EFER.LMA` is not a bit
   software writes, it is `LME && CR0.PG`. It is safe only because the trampoline is executing from
   an identity-mapped page, so the next instruction is fetched at the same address either way.
5. Clear `LME`, set `eax`/`ebx`, jump to the kernel.

It is copied to a page below 4 GiB before it runs, and both halves of that matter. Below 4 GiB
because the second half executes with paging off, where a linear address is a physical one and the
firmware may well have loaded this image above 4 GiB. And the page is allocated as
`EfiLoaderCode` rather than `EfiLoaderData`, because firmware with a memory-protection policy
(OVMF has one, and so does every recent vendor firmware) sets the execute-disable bit on data
allocations, and the first instruction of the copy would then fault with boot services still live
and nothing watching.

### Why the ELF reading is not `crates/elf`

That crate is the kernel's *user-program* loader, and a program is placed at its `p_vaddr`. A boot
loader places an image at its `p_paddr`, and on this kernel the two are not related by any offset a
caller could apply:

```text
.text            p_vaddr 0xffffffff80109000   p_paddr 0x109000
.ap_trampoline   p_vaddr 0x0000000000008000   p_paddr 0x165000
```

`elf::Segment` does not carry `p_paddr` at all, so there is nothing to subtract. Adding it is the
tidier answer and wants its own lane: that struct is public and the kernel's loader is its
consumer, so widening it is a change to a shared definition.

The other trap the physical span has to survive is `NOLOAD`. `.boot_scratch` (the boot page
tables) and the two per-CPU stack areas arrive as `PT_LOAD` segments with `p_filesz == 0` and a
real `p_memsz`. They are address space to reserve rather than bytes to copy, and a loader that
skipped them would leave the firmware free to hand that memory to something else while the kernel's
trampoline zeroes its own page tables on top of it.

## What it proves, measured

The table below was measured with both boots at `-m 256M`, before milestone 195 reclaimed
boot-services memory (the `usable ram` row is the one that moved; see "The RAM that is deliberately
left on the table" below, which now records both figures). The runner's default is now 2 GiB
(`NIFE_MEM=256M` reproduces the table): firmware places its ACPI tables just under the top of RAM, so
the memory size decides what physical addresses the kernel is asked to read, and at 256 MiB they land
low enough that a reach bug in the ACPI walk cannot show. One did, for as long as this script matched
its PVH sibling; see notes/x86-port.md's BUGS.

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

- The non-zero `rsdp`. `notes/x86-port/acpi-and-pci.md` records `rsdp 0x0` under QEMU's PVH loader, so
  `arch::x86_64::machine::find_rsdp` has always fallen back to scanning the BIOS area for
  `"RSD PTR "`. Under UEFI the pointer arrives in the handoff and the scan is skipped.
- The XSDT walk. A scanned ACPI 1.0 RSDP has revision 0 and a 32-bit RSDT root. Firmware hands
  over a revision-2 RSDP with a 64-bit XSDT, which is a different branch in
  `machine_discovery::acpi`. This is the assertion `cargo xtask uefi-boot` gates on, because it is
  the one string that cannot be printed by the PVH path.
- An ECAM window that is not the hardcoded constant. Milestone 165 (x86_64 PCI enumeration) made
  `memory::record_pci_regions` follow the MCFG rather than `arch::mmu::PCI_ECAM_PHYS`; under PVH
  the two agreed, so nothing distinguished "read the table" from "used the constant". Under UEFI
  they disagree and the kernel follows the table.
- A fragmented memory map. 118 descriptors instead of 9, and the frame allocator comes up over
  seven RAM regions instead of one.

And the userspace archive arrives the same way it does under PVH, through the module list this
loader writes:

```text
  initrd      : 5229568 bytes at 0xd9bc000, 68 programs, from the PVH module list
```

### The RAM that was left on the table, and how much came back

Before milestone 195 (finish the UEFI boot path): 206684 KiB against PVH's 261627 KiB, so about 54 MiB of a 256 MiB machine
was reported reserved that a Linux-style loader would reclaim. That was a choice rather than a
defect, in the conservative direction on purpose: claiming less RAM than exists costs megabytes,
claiming more corrupts something, on hardware nobody can attach a debugger to.

After it: 233148 KiB on the same 256 MiB machine, and 2068244 KiB against 2032128 on a 2 GiB one.
Two classes moved, both of them dead by the time the kernel reads the map:

- `EfiBootServicesCode` and `EfiBootServicesData`, which the UEFI specification says are free the
  moment `ExitBootServices` returns. 26 MiB of the 2 GiB machine.
- `EfiLoaderCode`, which is the loader's own PE image and its one-shot mode-switch trampoline.
  The image is the whole embedded payload (9 MiB for the tour build, 19 MiB for the test build), and
  the kernel and the archive were copied *out* of it before boot services ended. 9 MiB more.

`EfiLoaderData` stays reserved and must. Every allocation this loader makes that the kernel reads
later asks for it: the `hvm_start_info`, the memory map, the module list, the archive, and the kernel
image itself. The asymmetry is not a rule anyone has to remember, which is what makes it hold: the
only thing asking for `LOADER_CODE` is the trampoline, and it asks because firmware sets the
execute-disable bit on data allocations.

What is still reported reserved on the 256 MiB machine is 28 MiB, against PVH's 0.5 MiB. That is the
firmware's own runtime services, ACPI NVS, its reserved ranges, and the loader's `EfiLoaderData`, and
none of it is ours to take.

## Running it

```console
$ cargo xtask uefi-image                       # kernel + archive + loader, staged at target/esp
$ helpers/qemu-uefi-x86_64.sh target/esp       # boot it under OVMF
$ cargo xtask uefi-boot                        # the same pair, plus the assertions
$ cargo xtask uefi-image --features soak_test  # a non-default kernel for the stick (notes/soak.md)
```

`uefi-boot` stages its own copy at `target/esp-screen`, and that one is not for a stick.
Milestone 445 (the screen check stops sampling and starts asking) gave the screen check a
handshake instead of a race: the loader there is built with
the `screen_hold` feature, so it writes one extra word on the kernel's boot command line and the
kernel stops at the screen handover until the gate has photographed the framebuffer. A machine
booting that image with nobody listening waits ten seconds and carries on, which is a pause nobody
asked for. `target/esp` is the one the bench procedure copies from, and `cargo xtask uefi-image`
is what fills it. The kernel in the two directories is byte-identical; only the loader differs.

```console
$ cargo xtask uefi-test                        # the kernel's TEST binary under the same firmware
```

Both run inside `script/test --arch x86_64`, after the PVH suite, and they are two boots rather than
one because they carry two different kernels.

`uefi-boot` boots the tour, the same kernel and archive `uefi-image` stages for the USB stick
and that calef carries to the bench, behind the screen-hold loader described above. It runs at two cores, which no other x86_64 boot in this tree does;
see the SMP section below.

What it asserts, chosen so it cannot pass for the wrong reason:

- `(xsdt)` in the ACPI line, which only the firmware path can print.
- no `rsdp 0x0`, which is PVH's own tell.
- the tour's completion line, which is everything in between: the fine W^X page tables, the APIC,
  the timer, the scheduler, and two ring-3 processes, on a memory map from firmware.
- `smp: 2 core(s) online`, and a PIT interrupt still reaching the boot core with two local APICs on
  the machine.
- the tour's tail on the SCREEN, read back glyph by glyph while the kernel is holding it there
  (milestone 445). Its tail rather than its banner, because the tour is taller than OVMF's 1280x800
  console and scrolls: held still, the screen's first row is the middle of the firmware memory map.

`uefi-test` boots the kernel's test binary, which is the same kernel with `test_main()` on the
end of the same tour, so the boot prints every line above and then runs the suite. Milestone 195
added it, and until it existed *"it boots under real firmware"* and *"it passes under real firmware"*
were different claims with only the first one made. It asserts the harness's verdict **and QEMU's
exit status**, because a transcript scan alone would pass a run that printed its verdict and then
faulted on the way out.

The two boots are the same machine except for their devices: the suite gets the PVH runner's
`virtio-blk-pci` disk and NVMe controller, the tour gets neither. That is what makes the numbers
comparable, and on 2026-09-02 they were identical: 192 passed and 68 skipped under both PVH and
OVMF, with the same 68 test names skipped on each side.

### What running the suite under firmware cost, and what it found

It was not the two-line change to `uefi_image` this was scoped as, and the reason is the one
thing a tour boot cannot show: the test build is bigger. Its physical span reaches 10 MiB where the
tour's reaches 2.3, and OVMF keeps ACPI NVS at 8 MiB and its own boot-services allocations from 9 to
23.5, so `AllocatePages(AllocateAddress)` refused the whole range and the firmware printed nothing
more useful than `Load Error`.

Two things came out of that, and the second matters more than the first.

`PHYS_START` moved from 1 MiB to 32 MiB (`kernel/link-x86_64.ld`). 1 MiB is the *lowest* address
multiboot permits, never the only one, and under a hypervisor's loader nothing else is in low memory
to say so. This is a larger gap rather than a fix: the image is still placed at one address
chosen at link time, so a firmware that wants 32 MiB refuses the boot exactly as OVMF refused 1 MiB.
The real answer is a physically relocatable image, which is a milestone rather than a constant,
because `.boot` holds 32-bit absolute references to its own labels.

And the loader now names what is in the way. `AllocatePages` reports one status and no address,
so on that failure `uefi_loader::say_conflict` walks the memory map and prints every descriptor
overlapping the range that is not free RAM, with its UEFI type. On the bench that is the difference
between a `Load Error` and a sentence:

```text
uefi_loader: wanted 0x0000000000100000..0x0000000000add000
uefi_loader:   in the way: 0x0000000000800000..0x0000000000808000
uefi_loader:   memory type 0x000000000000000a
uefi_loader:   in the way: 0x0000000000900000..0x0000000001780000
uefi_loader:   memory type 0x0000000000000004
```

### SMP under firmware

`arch::x86_64::ap_boot` copies its real-mode trampoline to physical `0x8000`, because a STARTUP IPI
can only name a page below 1 MiB. Until milestone 195 the loader never mentioned that page, so
secondary cores under firmware worked or did not by luck: OVMF happens to leave the first 640 KiB
conventional. The loader asks the firmware for it by name now, and a refusal is a printed warning
rather than a failed boot, because a single-core boot on a machine whose firmware wants that page is
worth more than no boot at all.

Two cores come up under OVMF, five runs out of five, and `uefi-boot` gates it. **The suite stays at
one core**, and that is a known defect rather than a preference: `every_secondary_runs_scheduled_work`
fails about half the time at two cores on this architecture (`arch::x86_64::ap_boot`'s `BUGS` #3),
which is why the PVH runner defaults to one as well. The tour does not run that test.

`NIFE_OVMF_CODE` and `NIFE_OVMF_VARS` name the firmware images on a machine that keeps them
somewhere other than Homebrew's QEMU share; `NIFE_UEFI_TIMEOUT` moves the bound.

## First light on xenon, 2026-09-05

It has now been done, and the machine talked without a serial cable. Photograph:
`art/bench/xenon-2026-09-05-first-light.jpg`, which is the transcript, because patagonia could not be
moved to the bench and the Dell's video output was the only channel.

**Milestone 243's framebuffer console carried the whole boot tour on its first contact with real
firmware.** It had never run outside OVMF:

```
screen : 1920x1080 bgrx @ 0xd0000000, 274x135 cells (boot cmdline)
```

How far it got, all of it read off the screen: long mode, high-half kernel, the long-mode jump
landed, a breakpoint caught and stepped over, 38 memory regions from the PVH handoff, ACPI
revision 2 with a real XSDT at `0xcae090b8` and the full table list, 4 cores enumerated, 4 enabled,
PCI ECAM, TSC and PIT-measured timers at 100 Hz, and 17,119 MiB total.

Three things are firsts against real firmware rather than OVMF, and each was predicted here:

- `8259s present (must be masked)`. Real legacy PICs.
- `PCI_ECAM_PHYS says 0xb0000000` while the MCFG reports `0xe0000000`. The hardcoded constant
  disagrees with the firmware table and the kernel follows the table, which is exactly what the
  "What it proves, measured" section above says the UEFI path exercises and the PVH path cannot.
- 38 regions against OVMF's 118 and PVH's 9. *(Read as 148 when this section was first
  written, and corrected 2026-09-04 against both photographs: the count is 38 in the first-light
  shot and 38 again on the second boot. The misreading was mine and it propagated into the memory-map
  fixture's own doc comment, where it understated how much of the machine that fixture covers.)*

### What the boot menu documents, which is more than it looks

`art/bench/xenon-2026-09-05-boot-menu.jpg`, the F12 menu, is the first record of this machine's own
identity rather than of the model:

- OptiPlex 7050, BIOS revision 1.27.0.
- Boot mode UEFI, Secure Boot OFF, which is what steps 1 and 2 below ask for and confirms they
  were reachable as written.
- The stick enumerates as `UEFI: SanDisk Ultra 1.26`, so the removable-media fallback at
  `\EFI\BOOT\BOOTX64.EFI` is found with no boot entry created.
- A `Windows Boot Manager` entry, so this machine dual-boots and its internal disk holds
  somebody's installation. Worth knowing before anything here writes to a disk.
- `UEFI: Micron 2450 NVMe 256GB`.

That last line matters beyond the bench. DECISIONS §86 (whether an NVMe driver can leave the
kernel, and what capability would let it) was decided on 2026-09-03, and its own research recorded
that no board this project owns has an IOMMU in front of a real NVMe controller. xenon has both:
milestone 87's requirements list says it was selected partly for VT-d, and this photograph shows the
NVMe. So the confined-driver experiment §86 exists to enable has real hardware to run on, which
nobody had established.

### And then it panicked, in the one place a bigger machine would find

```
[PANIC] panicked at kernel/src/arch/x86_64/mmu.rs:325:33:
failed to build the kernel page tables: AlreadyMapped
```

Diagnosed 2026-09-05 from this photograph, and the leading hypothesis was wrong. It read the
framebuffer aperture at `0xd0000000` against 17 GB of RAM and concluded the two had met. They had
not, and the memory map on the screen is what says so: the aperture sits in the 32-bit MMIO hole,
which this firmware's map does not describe at all. The last entry below it ends at
`0xd0000000`, the next begins at `0xf0000000`, and the aperture is in no RAM region and never was.

What actually collided is the local APIC, and the mechanism is one line older than milestone
243. `map_firmware_regions` direct-mapped every loader-reserved entry below the top of RAM,
cacheably, and its own comment said why that bound was the load-bearing part: the reserved entries
*above* the top of RAM are the MMIO windows, and those must be device-typed. That is a true
statement about a machine whose RAM ends below the hole, which is every machine this tree had
booted. xenon's RAM ends at `0x42e000000`, so *every* MMIO window it has is below the bound:

| Reserved entry, from the photograph | What it is | Below `0x42e000000`? |
|---|---|---|
| `0xcbe00000..0xd0000000` | firmware's carve-out at the top of low DRAM | yes |
| `0xf0000000..0xf8000000` | PCH decode | yes |
| `0xfe000000..0xfe011000` | PCH | yes |
| `0xfec00000..0xfec01000` | IO APIC | yes |
| `0xfee00000..0xfee01000` | local APIC | yes |
| `0xff000000..0x100000000` | SPI flash | yes |

So the cacheable fill claimed `0xfee00000` a few lines before step 5 asked for the same page
device-typed, and the mapper refused, which is exactly what it is for. The framebuffer never got
that far: the local APIC is the first window step 5 maps.

Two things are worth separating here. The panic is the smaller half. The larger half is that on
this machine the fill was also mapping the IO APIC, the SPI flash and 128 MiB of PCH decode
cacheably, which is a write that can sit in a cache line and never reach the device. Nothing
had touched those yet, so nothing had failed; the panic is what made it visible.

The fix, on `maintainer/already-mapped-on-real-ram`. The fill's bound is now the address at
which the firmware's map stops describing memory, walked as a chain upward from the low megabyte:
DRAM and the carve-outs firmware takes out of it are contiguous, and the MMIO hole above them is a
gap. On xenon that is `0xd0000000`; on every machine whose RAM ends below the hole it is the same
number the old bound produced. The device windows are also enumerated before the fill now, so
device typing wins by construction rather than by that bound being right.

And the panic now names both ranges. `AlreadyMapped` on its own cost this session a diagnosis:
`map_everything` maps eleven kinds of range and the message distinguished none of them. Every
direct-map range now comes out of one enumeration that the failure path also walks, so the message
is `AlreadyMapped mapping local apic 0xfee00000..0xfee01000: 0xfee00000 is also claimed by firmware
reservation 0xfee00000..0xfee01000`. That is the difference between a bench session that ends in a
diagnosis and one that ends in a hypothesis, and it is the half worth keeping regardless of whether
the fix is right.

### Confirming the fix, which only xenon can do

A green QEMU run is not a confirmation and should not be reported as one. The runner boots with
2 GiB, `notes/x86-uefi-boot.md`'s own comparison table was taken at `-m 256M`, and neither reaches a
memory map with RAM above the MMIO hole. `-m 17G` on patagonia (16 GB) swaps rather than reproduces.
What the suite does prove is that the rule answers the old bound's number on a small machine, and
the `x86_64` kernel leg carries xenon's map as a fixture (`map_tests` in
`kernel/src/arch/x86_64/mmu.rs`, transcribed from the photograph) so the 17 GiB case is asserted
without the machine.

The bench session that confirms it, in the shape of the procedure below:

1. Build the stick exactly as step 3 below says, from `maintainer/already-mapped-on-real-ram` or
   from `main` once it has landed.
2. Boot it. The line to watch for is `mmu`, which the boot tour prints after `map_everything`
   returns:
   `mmu : fine W^X 4-level map installed (cr3 ...), image 0xffffffff80000000, direct map 0xffff888000000000`
   followed by the page-table cost in KiB. Reaching that line at all is the confirmation: it is one
   line past where the machine stopped on 2026-09-05.
3. Photograph the whole screen anyway, not just that line. The page-table cost on the second
   line is the number nobody has ever seen from a real machine, and this module's `BUGS` prices 4
   KiB leaves at 0.2% of RAM, so ~33 MiB is the prediction to check against.
4. If it panics again, the message is now the deliverable. Photograph it and stop; it names the
   two ranges, so no further bench time is needed to say what happened.
5. The tour continues past the `mmu` line into ring 3 and the scheduler. Everything after it is new
   ground on this machine and none of it has been seen on real firmware, so expect the next stop
   somewhere else and treat that as progress rather than as this fix failing.

### What it settles about the procedure below

The procedure worked, including its warning that firmware-menu wording would differ. calef also
photographed every page of the BIOS configuration, which is the first real record of this machine's
settings; those 70 photographs are now transcribed in `notes/xenon-firmware.md`, the `BUGS`
entry saying every setting name here came from Dell's documentation is retired to the extent the
transcription earns, and the four firmware steps below now say which of them the machine agreed
with.

And one thing it changes: step 1's expectation that the loader's two lines are "the only sign the
stick was read at all" is no longer true. With milestone 243 the screen carries the whole tour, so a
serial-less bring-up is a real bench session rather than a stare.

## The bench: booting nife on the OptiPlex 7050

**This section was written before first light and is kept as the procedure. It has now been run
once**; see above. Everything above is QEMU with real firmware in the loop, which is as
far as this lane could get; the machine is calef's bench. This section is the procedure, written to
be followed rather than interpreted.

### What you need

- The OptiPlex 7050 Micro, its Dell C4PDJ serial module installed, and the dev-side RS-232 chain
  (FTDI USB adapter, StarTech NM9FF null-modem barrel) already on the desk (milestone 87's own
  block).
- A USB stick, formatted FAT32 with a GPT or MBR partition table. macOS Disk Utility: *Erase*,
  format MS-DOS (FAT), scheme GUID Partition Map.

### Build and copy

```console
$ cd /path/to/nife
$ cargo xtask uefi-image
wrote .../target/esp/EFI/BOOT/BOOTX64.EFI (9110528 bytes: the loader, the kernel and the archive)

$ mkdir -p /Volumes/NIFE/EFI/BOOT
$ cp target/esp/EFI/BOOT/BOOTX64.EFI /Volumes/NIFE/EFI/BOOT/BOOTX64.EFI
$ diskutil eject /Volumes/NIFE
```

The path and the capitalisation are the interface. `\EFI\BOOT\BOOTX64.EFI` is the removable-media
fallback the firmware looks for with no configuration; anything else needs a boot entry created on
the machine.

That is the whole of it: one file. There is no kernel to copy separately and no configuration
file, because the loader carries both inside itself (`uefi_loader/build.rs`).

### Firmware settings, and the one that will bite

Enter setup with F2 at the Dell splash; F12 is the one-time boot menu.

These four steps have now been checked against the machine's own menus rather than against
Dell's documentation, from the 70 photographs calef took on 2026-09-04. Three were right as
written and one was worded differently; both facts are recorded below, and every value each step
asks for is already set on xenon, page by page, in `notes/xenon-firmware.md`.

1. Secure Boot: off. This image is unsigned and nothing in this tree signs it. On a 7050 that
   is *Secure Boot → Secure Boot Enable → Disabled*, and it may require *Boot List Option* to be
   UEFI first. Expect to have to do this; a Secure Boot machine will refuse the stick with a
   security-violation message and no other explanation. Right as written: the panel is
   `Secure Boot Enable` with a Disabled/Enabled radio pair, and its own help text states the UEFI
   precondition. It is already Disabled.
2. Boot List Option: UEFI, not Legacy. Legacy/CSM boot would look for an MBR boot sector, which
   this stick does not have. Right as written, with one refinement: `Boot List Option` is not a
   page of its own, it is the lower half of *General → Boot Sequence*, and on xenon the Legacy
   radio is greyed out because `Enable Legacy Option ROMs` is off. Already UEFI.
3. Serial port: COM1. The C4PDJ module presents COM1 at I/O port `0x3f8`, which is where the
   kernel's console driver looks (`arch::x86_64::port`). **This is the one that was worded
   differently.** The old wording said "Serial port: enabled" and guessed that the firmware might
   offer an address or IRQ choice. It does not: *System Configuration → Serial Port* is a single
   five-way radio, `Disabled` / `COM1` / `COM2` / `COM3` / `COM4`, with no separate enable, and the
   page's own help says `COM1 = Port is configured at 3F8h with IRQ 4`. So there is nothing to
   enable and nothing to enter; pick COM1 and the address and IRQ follow. Already COM1.
4. Leave the rest alone on the first attempt. In particular do not disable the integrated NIC
   or change the SATA mode: nothing here needs them, and a changed setting is one more variable in
   a bring-up that already has enough. Right as written: both settings exist under
   *System Configuration* (`Integrated NIC`, `SATA Operation`), and on xenon they are `Enabled` and
   `AHCI`. AHCI rather than `RAID On` is the reason the NVMe appears as an ordinary PCIe function.

And one hazard these four steps do not name, which only the transcription found. xenon's
`POST Behavior → Warnings and Errors` is set to `Prompt on Warnings and Errors` and
`Keyboard Errors → Enable Keyboard Error Detection` is ticked, so **the machine stops at POST and
waits for a keypress when it finds no keyboard**, which its own event log shows it doing eight
times over the past year. That did not bite during first light because a keyboard was attached. It
will bite the first time anybody tries to power-cycle this machine and let it boot unattended,
which is what `notes/bench-runbook.md` and `notes/serial-less-output.md` both want next. Changing
it is two settings and it is calef's call, because it changes the machine's behaviour for
everything else it is used for.

### Watch it

On the Mac, before powering the machine on:

```console
$ ls /dev/cu.usbserial-*                      # the FTDI adapter
$ screen /dev/cu.usbserial-XXXX 115200        # exit with ctrl-a k
```

115200 8N1, which is what `drivers/ns16550.rs` programs.

### What you should see, in order

1. On the video output, before anything reaches serial: `nife uefi_loader: milestone 87`, then
   `uefi_loader: kernel placed, exiting boot services`. The Dell's serial port does not carry
   *firmware* output, so this is the only sign the stick was read at all, which is exactly why the
   loader prints it. Attach a monitor for the first attempt.
2. On serial, immediately after: the kernel's own tour, beginning

   ```text
   nife on x86_64 (long mode, ring 0, 4-level paging)
     cpu 0 booted: high-half kernel, .bss, and the 16550 console are up.
   ```

   and ending `nife x86_64: boot complete, halting.` **The first line is milestone 87's own
   completion criterion**: the machine has printed a byte over serial.

### If it does not

Triage in this order, because each step rules out everything above it.

| Symptom | What it means | What to do |
|---|---|---|
| Firmware says "security violation" or silently skips the stick | Secure Boot is on | Disable it (above) |
| Firmware boots to its own shell or to the internal disk | the stick was not seen as bootable | Check the path and case: `/EFI/BOOT/BOOTX64.EFI`. Re-format FAT32, not exFAT |
| `nife uefi_loader: milestone 87` and then a message beginning `uefi_loader:` | the loader ran and refused, and the message says why | Every one of those strings is a literal in `uefi_loader/src/main.rs`; read it there |
| The loader's two lines and then nothing, ever | the handoff or the mode switch failed, or the kernel died before the console | See below |
| Video lines, but serial silent while the machine is clearly alive | the serial chain, not the software | Loop back the null-modem barrel's pins 2 and 3 and confirm `screen` echoes typing |
| The machine reboots in a loop | a triple fault | See below |

A triple fault or a dead machine after the loader's lines is the hard case, and the honest
answer is that this is where the QEMU work stops helping. What is available: boot with the archive
left out (the loader builds fine with no `NIFE_UEFI_INITRD`, and the tour says
`initrd: none`), which removes five megabytes of copying and the whole userspace half of the tour.
And `-cpu qemu64` under QEMU is the nearest thing to "a CPU that has told us no" (`notes/cpu-models.md`);
`-cpu max` on the dev machine has never refused this kernel anything.

### If it works

Two things are then worth doing, in this order, and neither is in this lane's scope:

1. Record the numbers, the way `notes/visionfive2.md` does for the VisionFive 2: the memory map
   the firmware reports, the ACPI tables it carries, the measured TSC rate against the PIT (this
   will *not* be QEMU's 1001 MHz; it is an i5-7500T, and `user_mode_runtime::cntfrq`'s hardcoded constant will
   be wrong with no way for a caller to tell, which `notes/x86-port/user-mode-runtime.md` already records), and
   whether the DMAR is present so VT-d can come up.
2. Flip milestone 87's status and open the two follow-ups the bench will inevitably produce.

## The bench: booting xenon over the network, with no stick at all

**Built by milestone 260, rehearsed on patagonia and never yet run on xenon.** Everything below
except the two firmware settings and the router edit has been exercised end to end under OVMF
(`script/netboot-rehearsal`), against the same `script/board-netboot` that will serve xenon, with
the DHCP answers parsed out of the same `bench/xenon-netboot/dnsmasq.conf` calef is about to apply.

Why bother, when copying one file to a stick already works. Two reasons, and the second is the
one that grows.

- The copy is the tax, and it is paid per boot. The stick procedure is eject, walk, insert,
  power on, and then walk back and do it again for the next build. milestone 257 measured what that
  costs on the other board: the 2026-09-04 radon session wrote the card **six times, once per
  boot**, because comparing two builds means interleaving them.
- It removes the co-location. First light is on record as happening because the video output
  was the only channel: *"patagonia could not be moved to the bench"*. A netboot does not need it
  to be. xenon needs an ethernet cable to the house LAN and patagonia needs to be on the same LAN,
  from wherever it is.

### What you need

- xenon, on the house LAN by cable into the motherboard's ethernet. Not the Wi-Fi card in
  Slot2_M.2: `Integrated NIC` switches the LOM and nothing else, and only the LOM can PXE.
- patagonia on the same LAN, and its address. `script/board-netboot` prints every private address
  this machine has, which is a list of candidates rather than an answer; see the failure table.
- The house router, once, to take the DHCP lines. Nothing in this repository has ever talked to it.
- A monitor on xenon, exactly as for the stick procedure, because the Dell's serial port does not
  carry firmware output and the firmware is the half that is new here.

### Step 1: the two firmware settings, which are calef's

Both are on one page, `System Configuration → Integrated NIC` (IMG_4031 in
`notes/xenon-firmware.md`), and today it reads `Enable UEFI Network Stack` unticked with the
radio on Enabled.

1. Tick `Enable UEFI Network Stack`. Without it the firmware has no UEFI networking at all, so
   there is nothing for a PXE boot option to be built on.
2. Move the radio from `Enabled` to `Enabled w/PXE`. This is what puts `UEFI PXEv4` on the boot
   list.

They are calef's for the reason `notes/xenon-firmware.md` gives about firmware generally: a setting
changes this machine's behaviour for everything else it is used for. **Edit that note when you
change them**, because the next lane reads it instead of walking to the machine.

Nothing else in the transcription is in the way: Boot List Option is already UEFI, Secure Boot is
Disabled, legacy option ROMs are unticked, and `UEFI Boot Path Security` has no effect while no
admin password is set.

### Step 2: the router, once

`bench/xenon-netboot/dnsmasq.conf` is the whole of it, with the reason for every line beside it.
`dhcp-boot` carries patagonia's address, `192.168.8.138` as of 2026-10-04 (it was `.216`; corrected). Check it against patagonia's actual address first, then apply it.

On OpenWRT, through UCI, which is the form that survives a reboot:

```console
# uci add dhcp host
# uci set dhcp.@host[-1].name='xenon'
# uci set dhcp.@host[-1].mac='D8:9E:F3:74:B2:A2'
# uci set dhcp.@host[-1].tag='xenon'
# uci add dhcp boot
# uci set dhcp.@boot[-1].filename='EFI/BOOT/BOOTX64.EFI'
# uci set dhcp.@boot[-1].serveraddress='192.168.8.138'
# uci set dhcp.@boot[-1].servername='patagonia'
# uci set dhcp.@boot[-1].networkid='xenon'
# uci commit dhcp
# /etc/init.d/dnsmasq restart
```

The architecture match has no UCI form, so the two `dhcp-match` lines go in a file dnsmasq
reads directly. On OpenWRT that is `/etc/dnsmasq.conf`, which the UCI-generated configuration
includes:

```
dhcp-match=set:efi-x86-64,option:client-arch,7
dhcp-match=set:efi-x86-64,option:client-arch,9
```

and the `dhcp-boot` above then needs `tag:efi-x86-64` as well, which UCI's `networkid` takes only
one of. If that turns out to be awkward, **write all four lines into `/etc/dnsmasq.conf` and skip
UCI entirely**; the config file in this tree is already in that form, and it is the form the
rehearsal parses and proves.

Do not skip the architecture match to save the trouble. It is what stops this configuration
handing 9 MB of x86-64 UEFI image to a machine that is in Legacy boot mode, which includes xenon
itself the moment anyone sets `Boot List Option` back to Legacy. `script/netboot-rehearsal --check`
lists all six cases and takes under a second.

### Step 3: serve, and boot

On patagonia, two commands in two terminals:

```console
$ cargo xtask uefi-image                        # every time the kernel changes
$ script/board-netboot --root target/esp        # in the other terminal, while you work
```

Then power xenon on. There is no third step and no copy. Every later boot is
`cargo xtask uefi-image` and a power cycle; the file the firmware fetches is the same file the
stick procedure copies, at the same path, staged once.

### What you should see, in order

1. On xenon's video output, the firmware's own PXE progress, which is EDK2's and reads exactly
   as it does under OVMF:

   ```text
   >>Start PXE over IPv4.
     Station IP address is 192.168.8.NN
     NBP filename is EFI/BOOT/BOOTX64.EFI
     NBP filesize is 9210880 Bytes
    Downloading NBP file...
     NBP file downloaded successfully.
   ```

   The byte count is not a constant and is not worth checking against. It is whatever
   `cargo xtask uefi-image` last wrote, and it moves with every kernel and archive change: two runs
   an hour apart on 2026-09-05 measured 9,210,880 and 9,797,120. What matters is that the number the
   firmware prints and the number `board-netboot` prints are the same number.

2. In patagonia's `board-netboot` terminal, two requests and not one, in this order:

   ```text
   board-netboot: 192.168.8.NN <- EFI/BOOT/BOOTX64.EFI (9210880 bytes, blksize 1468)
   board-netboot: EFI/BOOT/BOOTX64.EFI cancelled by the client after the option acknowledgement (a UEFI client's file-size probe does exactly this)
   board-netboot: 192.168.8.NN <- EFI/BOOT/BOOTX64.EFI (9210880 bytes, blksize 1468)
   board-netboot: EFI/BOOT/BOOTX64.EFI done, 9210880 bytes in N.NNs
   ```

   `blksize 1468` is EDK2's own ask and is what a full-size ethernet frame holds once the IP,
   UDP and TFTP headers are out of it. A client that negotiated 512 instead would still work and
   would take about three times as long.

   The cancellation line is normal and is the boot working. EDK2 asks for the file twice: once
   to learn its size, which it does by starting a read and then stopping it, and once to fetch it.

3. Then `nife uefi_loader: milestone 87` on the video output, and the boot tour after it,
   identical to the stick path from here on. The loader carries the kernel and the archive inside
   itself, so nothing after the transfer knows or cares where the image came from.

### If it does not

| Symptom | Most likely cause, in order |
|---|---|
| No `>>Start PXE over IPv4` on the screen at all | The two firmware settings did not take, or took on the wrong machine. `Enabled w/PXE` is what creates the boot entry; `Enable UEFI Network Stack` is what lets it exist. Check `F12`'s one-time boot menu for a `UEFI PXEv4` entry: no entry means step 1 |
| `PXE-E16: No valid offer received` | The router is answering, and answering with no boot file. Either the MAC in `dhcp-host` is wrong (read it off `General → System Information`, `LOM MAC Address`), or the `dhcp-match` tag is not being applied and the `tag:efi-x86-64` requirement is failing. This is the same output `script/netboot-rehearsal --mac <not-xenon>` produces, deliberately |
| `PXE-E18: Server response timeout` | The offer arrived and the transfer did not. patagonia's address in `dhcp-boot` is stale or is an address xenon cannot route to (a Tailscale address, or the wrong side of a Wi-Fi/ethernet split); or `board-netboot` is not running; or macOS's firewall is dropping inbound UDP to python3, which is the failure this rig has that radon's did not, because radon's traffic was on a port macOS had already been asked about |
| `board-netboot` says `NOT FOUND` with a name in it | The filename in `dhcp-boot` and the layout under `--root` disagree. It is a path relative to the served directory, so `--root target/esp` wants `EFI/BOOT/BOOTX64.EFI` and nothing else |
| Transfer completes, then nothing | The firmware executed the image and the loader's video output is the next thing. If the screen is blank, that is milestone 87's territory rather than this one, and a stick will reproduce it without the network in the way |
| The machine sits at POST waiting for a keypress | `Alert! Keyboard not found`, which this rig runs straight into and does not fix. See below |

### What this does not fix, and it is the thing that will bite next

xenon halts at POST without a keyboard. `notes/xenon-firmware.md` records
`Warnings and Errors` at `Prompt on Warnings and Errors`, `Enable Keyboard Error Detection` ticked,
and eight `Alert! Keyboard not found` entries in the machine's own event log across a year. A rig
whose point is an unattended power cycle runs into that, and the two settings that would change it
are calef's for the same reason as the two above. **Network boot without them is a faster bench
session, not an unattended one.**

## What xenon still has to establish, and what it no longer has to

Milestone 215 listed three things only the OptiPlex could confirm. **Two of them are now answered on
patagonia**, because the suite runs under firmware with the same devices the PVH runner attaches:

- A PCI function's MSI-X table, once *firmware* rather than this kernel placed its BARs. OVMF
  enumerates the bus before nife exists, and `pci::bar_census` reports 5 of 8 functions with a BAR
  outside the window `mmu::map_everything` maps, so `place_bars` moved five rather than assigning
  them. The two milestone 215 tests that reach a `virtio-blk-pci` function through its MSI-X table
  pass on the far side of that move.
- A machine with more than one local APIC still delivering to the boot core's id. The tour boots
  at two cores under OVMF and its `device irq` line shows the PIT's interrupt arriving 20 times in
  0.2 s at 100 Hz, with two APICs in the MADT.

The third is still open, but it is no longer the shape this paragraph gave it: whether the
OptiPlex's firmware leaves interrupt remapping off. This said the answer is a setting in somebody
else's firmware, and the 2026-09-04 transcription found no such setting exists. The whole of
the 7050's Virtualization Support menu is three pages (`Virtualization`, `VT for Direct I/O`,
`Trusted Execution`), and `VT for Direct I/O` is enabled. So the answer is not in a menu; it is
in the DMAR the firmware publishes, which any kernel can read and which QEMU synthesises too. That
moves the question off the bench and into code: `design/roadmap/378-read-the-dmar-on-xenon.md`.

And this milestone added one of its own for the bench, which is the more interesting of the two:
whether the Dell's firmware leaves 32 MiB free. OVMF's low-memory habits are OVMF's. If it does
not, the loader now says which ranges it wants and which descriptors are in the way, and that message
is the whole difference between a bring-up and a stare.

## BUGS

- **The bench procedure's firmware steps are now checked against the machine; the rest of it is
  still one run old.** The old entry here said every firmware-menu path, key and setting name above
  came from the 7050's documented behaviour rather than from this machine, and that the first person
  to follow it should expect one of them to be worded differently on the screen. That prediction was
  right, once: step 3's "Serial port: enabled" is a single five-way radio with no enable, and
  picking `COM1` is the whole of it. Steps 1, 2 and 4 were correct as written. `notes/xenon-firmware.md`
  is the full transcription and is what a bench session should read instead of guessing. What is
  not retired: the F2 and F12 keys, the `\EFI\BOOT\BOOTX64.EFI` fallback and the triage table
  below are still from one successful run and Dell's documentation, not from repeated use.
- The kernel is placed at one address chosen at link time, and 32 MiB is not a guarantee. It
  clears every low reservation OVMF makes and nothing more; a firmware that wants that range refuses
  the boot. The image is not physically relocatable, and making it so is a milestone rather than a
  constant: `.boot` is linked at its physical address because a 32-bit instruction stream cannot name
  a 64-bit one, and its absolute self-references are what would have to become position-independent.
- The bench procedure still boots one core, unlike the runner. Nothing stops two, and two work
  under OVMF; the procedure is written for a first bring-up where every variable costs.
- The suite under firmware runs at one core, so nothing exercises AP bring-up under UEFI *and* the
  scheduler's cross-core tests together. That is `ap_boot`'s open two-core defect rather than
  anything about firmware, and it is the PVH runner's situation too.
- Nothing verifies what the loader hands over. The kernel and the archive are bytes the loader
  was compiled with, so the trust boundary is the build; `measured_boot`'s manifest is not consulted
  and the image is not signed. That is also why Secure Boot has to be off.
- A stale `.efi` on a stick is silent when the stick was made by hand. The loader embeds the
  kernel, so a stick that was written last week boots last week's kernel with nothing to say so.
  `cargo xtask uefi-image` rebuilds both every time, which moves the hazard to the copy step rather
  than removing it. A stick written by `stick_maker` carries `NIFE.TXT` with the build and each
  file's digest (notes/boot-stick.md), which makes it checkable, not fresh.
- **`uefi-test` can go red after its own suite has passed, and the message around it points at the
  wrong cause.** Milestone 117's sixth stranger run (2026-09-19, on the dev Mac, two other lanes
  gating beside it) saw the kernel suite under OVMF print `test result: ok. 215 passed, 71 skipped`
  and then `uefi-test: qemu exited Some(1), not 3`, so `script/test` failed at its last step. The
  lines QEMU printed just before it, `vtd_iova_to_sspte: detected sspte permission error` and
  `vtd_iommu_translate: detected translation failure`, read as the cause and are not: they are the
  deliberate DMA-escape tests' faults, and a clean re-run of `cargo xtask uefi-test` an hour later on
  the same machine and the same Homebrew QEMU 11.1.1 printed the same two lines and passed. The
  stranger concluded the QEMU version was to blame, which that re-run does not support. That the two
  lines are expected is written in milestone 215's block and nowhere a reader meets the failure. What made QEMU exit 1 rather than 3 is unmeasured: one red
  in two runs is a rate nobody has taken yet.
