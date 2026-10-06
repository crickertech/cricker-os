---
status: NOT-STARTED
raised: 2026-09-19
promoted_from: the-boards-screen-under-uefi
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 511. The boards' screen comes from the firmware, not from a `ramfb`

*(Number provisional until the merge queue lands it.)* Promoted from the
proposal `the-boards-screen-under-uefi`, filed 2026-09-19, on calef's instruction of 2026-09-20 to
give every proposal on `main` a number. The text below is the proposal's own, unedited except for
this paragraph: the argument is its author's and promotion is not the moment to improve it. Found by
milestone 243's lane while closing that block's two Outstanding items, and filed because milestone
441 changed the premise underneath it while the lane was running: `uefi_loader` gained aarch64
and riscv64 boot files, so on those architectures there is now a firmware stage that has already lit
a display.

Everything is in this tree; the only outside dependency is a machine to try it on,
and QEMU with the board firmwares is enough to start.

In brief. Milestone 243 (a machine with no serial port) gave aarch64 and riscv64 a screen
through QEMU's `ramfb`, which is what `virt` can present when nothing lit a display: the guest
supplies 1.9 MB of `.bss` and tells the emulator where it is. That was the right answer for a board
booted from `-kernel` and it is the wrong one for a board booted from a stick, because a UEFI
firmware on those architectures has a
`EFI_GRAPHICS_OUTPUT_PROTOCOL` exactly as an x86 one does, with a real aperture behind it.

## The two pieces, and the second is the interesting one

1. The handoff banner, on all three architectures. `uefi_loader`'s `find_screen` and its
   `screen::paint_handoff` call are under `arch/x86_64/` because that is where they were written.
   Nothing in either is x86-specific: `find_screen` is one `LocateProtocol` call, and the painter
   is `screen_console`. Moving `find_screen` up to `uefi_loader/src/arch/mod.rs` and calling
   `paint_handoff` from the aarch64 and riscv64 `hand_over` functions, after `exit_boot_services`,
   gives those boards the same bounded window milestone 243 gave x86_64. **Small, and it is the
   part that needs no agreement between stages.**

2. Carrying the screen to the kernel, which is a wire format and therefore the real work. The
   x86 path rides PVH's `cmdline_paddr`, which the boards do not have: they are handed a **device
   tree**, and `uefi_loader` already copies and patches one (`uefi_loader::device_tree_patch`, for
   the initrd). So the loader could **synthesize a `simple-framebuffer` node** into that copy, which
   is *the same node* milestone 157 (real display output on the board) will read from U-Boot on
   the VisionFive 2. One parser in `machine_discovery::framebuffer` would then serve both stages,
   and neither would need a second spelling of a screen.

## Why it is worth doing rather than leaving `ramfb` in place

- It costs the `.bss`. `kernel/src/screen.rs` holds 1.9 MB on every board boot, including the
  VisionFive 2 boots where `ramfb` cannot exist. A firmware aperture needs none.
- `ramfb` is QEMU's. No board has one, so the screen milestone 243 proved on aarch64 and
  riscv64 is an emulator's screen and says nothing about silicon. A GOP aperture on a board booted
  from milestone 441's stick is a real machine's screen.
- It is the same node as 157's, so it is not a competing mechanism: it is 157's parser, reached
  a boot stage earlier, and the two would gate each other.

## What has to be decided before it is built, and by whom

This is a wire format between two boot stages, which the *move fast on what can be undone* tenet
puts in the expensive column, so it should not be invented by a lane in passing. The questions:

- Synthesize the node, or add a property to `/chosen`? Linux reads `simple-framebuffer` as a
  node with `reg`, `width`, `height`, `stride` and `format`, and that is the convention every other
  bare-metal OS booted by U-Boot or GOP relies on. The refusal to weigh: a `/chosen` property would
  be less code in the loader and would match nothing anybody else writes.
- Is `machine_discovery::framebuffer`'s five-field type enough, or does `format` need more than
  its two byte orders? `simple-framebuffer` names formats as strings (`a8r8g8b8`, `r5g6b5`), which
  is a third spelling beside UEFI's enum and DRM's `fourcc`. Milestone 243 already carries the
  second-to-third mapping (`firmware_configuration::RamFramebuffer::fourcc`).
- Whose milestone is it? It is most of milestone 157's discovery half, arriving through a
  different firmware, so the honest answer is probably that 157 grows a second source rather than
  that this becomes its own block.

## Where the pieces are

- `uefi_loader/src/arch/x86_64/mod.rs`: `find_screen`, and the `paint_handoff` call this would move.
- `uefi_loader/src/arch/aarch64/mod.rs`, `.../riscv64/mod.rs`: `hand_over`, where the banner goes.
- `uefi_loader/src/device_tree_patch.rs`: the copy the node would be written into.
- `crates/machine_discovery/src/framebuffer.rs`: the type both stages already share.
- `kernel/src/screen.rs`: where a second branch would go, above the `ramfb` one.
- `design/roadmap/157-uboot-framebuffer-handoff.md`: the milestone this most likely belongs to.

## Index row

`uefi_loader` gained aarch64 and riscv64 boot files in milestone 441 (the program that makes the
stick), so the boards now have a firmware stage that has lit a display: it could paint milestone
243's handoff banner there too, and it could hand the kernel a real aperture by synthesizing the
`simple-framebuffer` node milestone 157 will read from U-Boot, retiring the `ramfb`'s 1.9 MB of
`.bss`. The second half is a wire format between two boot stages and wants a decision rather than a
lane.
