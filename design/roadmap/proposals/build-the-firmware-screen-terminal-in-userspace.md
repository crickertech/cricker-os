# Build the firmware-screen terminal in userspace

**Status: PROPOSED 2026-09-26.** Raised by lane `milestone/600-userspace-graphical-stack` (milestone 600
(provisional), the graphical terminal stack is built in userspace), which moved the virtio-gpu stack's
construction into the progenitor and found the firmware-screen terminal still built by the kernel.

**Gate: NONE.** Ordinary work, unless the handover below turns out to need a new kernel method, in
which case that part is calef's.

## The finding

`kernel::user::boot_screen_terminal` (milestone 198 (a package manager)'s rung 1b, the shell on the firmware screen)
spawns `framebuffer_driver` and `display_terminal` kernel-side and grants the progenitor the
terminal's endpoint and page at slots 10 and 11. On x86_64 that is the only `display_terminal` a boot
has, so milestone 23 (a capability-routed component OS with live replacement) cannot swap it there, and milestone 600 could not move it with the gpu stack
because the reason it is kernel-built is different: the kernel must stop painting the screen itself,
under its console lock, before any driver paints it (`console::yield_screen`). The kernel also runs
both programs unmeasured (`kernel::user::program` checks nothing).

## What to find out, then build

1. Can the handover stay in the kernel while the spawn moves? The kernel would yield the screen and
   grant the progenitor a `DeviceFrame` run over the covered aperture, as it grants the gpu's
   surface. `display_service::start_screen_terminal` maps that aperture into the driver today; check
   whether a device run capability of up to `MAX_APERTURE_PAGES` exists or needs one.
2. If it does, build `framebuffer_driver` and `display_terminal` in `system_initializer` beside
   `build_graphical_stack`, and retire the kernel-side spawn.

## What it unblocks

Swapping `display_terminal` on x86_64, and measured boot for the two programs.
