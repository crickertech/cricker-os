---
status: BUILT
raised: 2026-08-30
built: 2026-09-02
---
# 195. Finish the UEFI boot path: the suite under firmware, the memory it gives back, and a second core

Built (2026-09-02). Minted 2026-08-30 from milestone 87's (the x86_64 bare-metal machine)
lane, whose three ordinary handoffs otherwise lived only in a pull request body.

In brief. The UEFI loader boots the kernel's tour under OVMF and, once calef has run it, on the
OptiPlex. Three things it did not do yet, each small and each already scoped by the lane that found
it. All three are done, and the first one was not small.

## The three, and what each turned out to be

### Run the kernel suite under firmware, not just the tour

`cargo xtask uefi-test` does, and `script/test --arch x86_64` runs it after the tour. 192 passed,
68 skipped under OVMF, which is the same 192 and the same 68 names as under PVH, because the runner
now attaches the PVH runner's `virtio-blk-pci` disk, NVMe controller and `intel-iommu` too. The tree
can now say *"it passes under real firmware"* rather than only *"it boots under real firmware"*.

**It was scoped as a two-line change to `uefi_image` and was not**, for a reason only a bigger kernel
could show. The test build's physical span reaches 10 MiB where the tour's reaches 2.3, and OVMF keeps
ACPI NVS at 8 MiB and its own boot-services allocations from 9 to 23.5, so
`AllocatePages(AllocateAddress)` refused the whole range and the firmware said `Load Error` and
nothing else.

Two things came of that:

- `PHYS_START` moved from 1 MiB to 32 MiB. 1 MiB is the *lowest* address multiboot permits, never
  the only one, and under a hypervisor's loader nothing else is in low memory to say otherwise. This
  is a larger gap rather than a fix, and the linker script says so where the constant is.
- The loader names what is in the way. `uefi_loader::say_conflict` walks the memory map on that
  failure and prints every descriptor overlapping the range that is not free RAM, with its UEFI type.
  On the bench that is the difference between `Load Error` and a sentence naming an address.

### Reclaim boot-services memory

Re-measured rather than inherited, because the runner's default moved to 2 GiB. Boot-services
code and data, plus `EfiLoaderCode` (the loader's own PE image and its one-shot trampoline, both dead
by then), are reported as RAM now: 2032128 KiB of usable RAM on a 2 GiB machine becomes 2068244,
and 206684 becomes 233148 on a 256 MiB one. `EfiLoaderData` stays reserved and must: the
`hvm_start_info`, the memory map, the module list, the archive and the kernel image are all in it.

The asymmetry holds without anyone remembering it, which is why it is worth stating: the only
allocation this loader makes as `LOADER_CODE` is the trampoline, and it asks for that because
firmware sets the execute-disable bit on data.

### SMP under UEFI

The loader asks the firmware for physical `0x8000` by name instead of assuming it. Two cores come
up under OVMF, five runs out of five, and `cargo xtask uefi-boot` gates it. A refusal of that page
is a printed warning rather than a failed boot, because a single-core boot on a machine whose
firmware wants it is worth more than none.

It was grouped here as "may not be small" against milestone 161's two open `ap_boot` defects. It met
one of them: `every_secondary_runs_scheduled_work` fails about half the time at two cores, which is
why the suite stays at one core and the tour is where the second one is gated. Neither
defect was touched.

## What it took off xenon's evening

Milestone 215's (a PCI function's interrupt on x86_64) `BUGS` listed three things only the OptiPlex
could confirm. **Two are now answered on patagonia**, both of them because real firmware enumerates
the bus and places every BAR before nife exists:

- a PCI function's MSI-X table is reachable once *firmware* placed its BARs (`pci::bar_census` reports
  five of eight functions outside this kernel's window, so `place_bars` moved five, and both of
  milestone 215's MSI-X tests pass afterwards);
- a machine with more than one local APIC still delivers to the boot core's id.

**The third is still xenon's**: whether the OptiPlex's firmware leaves VT-d interrupt remapping off.

## BUGS

- None of this is proved on a Dell. OVMF is not a vendor firmware, and the one new thing this
  milestone asks of the bench is its own: whether the OptiPlex leaves 32 MiB free.
- The kernel is placed at one link-time address. 32 MiB clears every low reservation OVMF makes
  and nothing more. A physically relocatable image is the real answer and is a milestone rather than a
  constant: `.boot` is linked at its physical address because a 32-bit instruction stream cannot name
  a 64-bit one, and its absolute self-references would have to become position-independent.
- **Two firmware boots per `script/test --arch x86_64`**, the tour and the suite, because they carry
  two different kernels and the tour is the one that goes on the stick. The suite alone would leave
  the shipping image gated by nothing.

## Follow-on

- **Milestone 161.** The two open `ap_boot` defects this milestone met and deliberately did not
  touch. `every_secondary_runs_scheduled_work` fails about half the time at two cores, which is why
  the suite stays at one core and the tour is where the second one is gated.
- **Recorded.** `notes/x86-port/acpi-and-pci.md` carries the one thing only xenon can confirm out of milestone
  215's three: whether the OptiPlex's firmware leaves VT-d interrupt remapping off. The other two
  were answered on patagonia by this milestone.
- **Recorded.** `design/roadmap/195-uefi-boot-finish.md`'s own `BUGS`: none of this is proved on a
  Dell. OVMF is not a vendor firmware, and whether the OptiPlex leaves 32 MiB free is a question
  for the bench.
- **Recorded.** `design/roadmap/195-uefi-boot-finish.md`'s own `BUGS`: the x86_64 leg boots
  firmware twice, once for the tour and once for the suite, because they carry two different
  kernels and the tour is the one that goes on the stick. Gating the suite alone would leave the
  shipping image gated by nothing.
- **Milestone 348.** Make the x86_64 kernel
  image physically relocatable instead of linked at one address. `PHYS_START` moved from 1 MiB to 32
  MiB to clear OVMF's low reservations, which buys a larger gap rather than a fix, and `.boot`'s
  absolute self-references have to become position-independent because a 32-bit instruction stream
  cannot name a 64-bit one.

## Index row

The kernel suite runs under OVMF now (`cargo xtask uefi-test`): the same 192 passes and the same
68 skips as under PVH. Boot-services memory and the loader's own code come back, 36 MiB of a 2 GiB
machine. Two cores come up under firmware and the tour gates it. It also moved `PHYS_START` to 32
MiB, because the test build would not load at 1 MiB under real firmware at all, and closed two of
the three things milestone 215 recorded as answerable only at a bench.
