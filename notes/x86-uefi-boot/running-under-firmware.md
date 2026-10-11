# Running under firmware: the two gates, the suite's cost, and SMP

An appendix to [notes/x86-uefi-boot.md](../x86-uefi-boot.md), which keeps the commands. This file
holds what `uefi-boot` and `uefi-test` assert and why, what running the suite under firmware cost,
and how secondary cores come up.

## The two images

`uefi-boot` stages its own copy at `target/esp-screen`, and that one is not for a stick. Milestone 445
(the screen check stops sampling and starts asking) gave the screen check a handshake instead of a
race. The loader there is built with the `screen_hold` feature. It writes one extra word on the
kernel's boot command line, and the kernel stops at the screen handover until the gate has
photographed the framebuffer. A machine booting that image with nobody listening waits ten seconds
and carries on, which is a pause nobody asked for. `target/esp` is the one the bench procedure copies
from, and `cargo xtask uefi-image` is what fills it. The kernel in the two directories is
byte-identical; only the loader differs.

Both gates run inside `script/test --arch x86_64`, after the PVH suite. They are two boots rather
than one because they carry two different kernels.

## What `uefi-boot` asserts

`uefi-boot` boots the tour: the same kernel and archive `uefi-image` stages for the USB stick, and
that calef carries to the bench, behind the screen-hold loader. It runs at two cores, which no other
x86_64 boot in this tree does; see SMP below.

What it asserts, chosen so it cannot pass for the wrong reason:

- `(xsdt)` in the ACPI line, which only the firmware path can print.
- No `rsdp 0x0`, which is PVH's own tell.
- The tour's completion line, which is everything in between: the fine W^X page tables, the APIC,
  the timer, the scheduler, and two ring-3 processes, on a memory map from firmware.
- `smp: 2 core(s) online`, and a PIT interrupt still reaching the boot core with two local APICs on
  the machine.
- The tour's tail on the SCREEN, read back glyph by glyph while the kernel is holding it there
  (milestone 445). Its tail rather than its banner, because the tour is taller than OVMF's 1280x800
  console and scrolls. Held still, the screen's first row is the middle of the firmware memory map.

## What `uefi-test` asserts

`uefi-test` boots the kernel's test binary. That is the same kernel with `test_main()` on the end of
the same tour, so the boot prints every line above and then runs the suite. Milestone 195 (finish the
UEFI boot path) added it. Until it existed, *"it boots under real firmware"* and *"it passes under
real firmware"* were different claims, with only the first one made. It asserts the harness's verdict
and QEMU's exit status, because a transcript scan alone would pass a run that printed its verdict and
then faulted on the way out.

The two boots are the same machine except for their devices. The suite gets the PVH runner's
`virtio-blk-pci` disk and NVMe controller; the tour gets neither. That is what makes the numbers
comparable. On 2026-09-02 they were identical: 192 passed and 68 skipped under both PVH and OVMF,
with the same 68 test names skipped on each side.

## What running the suite under firmware cost, and what it found

It was not the two-line change to `uefi_image` this was scoped as. The reason is the one thing a tour
boot cannot show: the test build is bigger. Its physical span reaches 10 MiB, where the tour's
reaches 2.3. OVMF keeps ACPI NVS at 8 MiB, and its own boot-services allocations from 9 to 23.5. So
`AllocatePages(AllocateAddress)` refused the whole range, and the firmware printed nothing more useful
than `Load Error`.

Two things came out of that, and the second matters more than the first.

`PHYS_START` moved from 1 MiB to 32 MiB (`kernel/link-x86_64.ld`). 1 MiB is the *lowest* address
multiboot permits, never the only one, and under a hypervisor's loader nothing else is in low memory
to say so. This is a larger gap rather than a fix. The image is still placed at one address chosen at
link time, so a firmware that wants 32 MiB refuses the boot exactly as OVMF refused 1 MiB. The real
answer is a physically relocatable image. That is a milestone rather than a constant, because `.boot`
holds 32-bit absolute references to its own labels.

And the loader now names what is in the way. `AllocatePages` reports one status and no address. So on
that failure `uefi_loader::say_conflict` walks the memory map, and prints every descriptor overlapping
the range that is not free RAM, with its UEFI type. On the bench that is the difference between a
`Load Error` and a sentence:

```text
uefi_loader: wanted 0x0000000000100000..0x0000000000add000
uefi_loader:   in the way: 0x0000000000800000..0x0000000000808000
uefi_loader:   memory type 0x000000000000000a
uefi_loader:   in the way: 0x0000000000900000..0x0000000001780000
uefi_loader:   memory type 0x0000000000000004
```

## SMP under firmware

`arch::x86_64::ap_boot` copies its real-mode trampoline to physical `0x8000`, because a STARTUP IPI
can only name a page below 1 MiB. Until milestone 195 the loader never mentioned that page. So
secondary cores under firmware worked or did not by luck: OVMF happens to leave the first 640 KiB
conventional. The loader asks the firmware for it by name now. A refusal is a printed warning rather
than a failed boot, because a single-core boot on a machine whose firmware wants that page is worth
more than no boot at all.

Two cores come up under OVMF, five runs out of five, and `uefi-boot` gates it. The suite stays at one
core, and that is a known defect rather than a preference. `every_secondary_runs_scheduled_work`
fails about half the time at two cores on this architecture (`arch::x86_64::ap_boot`'s `BUGS` #3),
which is why the PVH runner defaults to one as well. The tour does not run that test.

`NIFE_OVMF_CODE` and `NIFE_OVMF_VARS` name the firmware images on a machine that keeps them somewhere
other than Homebrew's QEMU share; `NIFE_UEFI_TIMEOUT` moves the bound.
