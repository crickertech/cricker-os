---
status: NOT-STARTED
raised: 2026-08-23
milestone_dependencies: none
decision_dependencies: none
machine_requirements: a board booted by U-Boot with a display attached
specific_machine: none
needs_person: yes
---
# 157. Real display output on the board: U-Boot's `simple-framebuffer` handoff

Minted 2026-08-23, from calef thinking through what it would take to boot
directly into the terminal display on real hardware. The plan already exists, recorded in
[the display ladder](../display-ladder.md)'s rung five (struck 2026-07-28) but never turned into a
milestone: *"The board's standalone-display story is the DC8200 framebuffer path instead: U-Boot's
`simple-framebuffer` handoff first (zero display code), a mode-setting driver only if ever needed,
serial input until a USB HID milestone earns its own number."*

In milestone 53's sense: the board is on the desk and this needs hands on it.
The device-tree parsing and the driver's own logic can be written and host-tested without silicon,
but the handoff itself (does U-Boot actually hand over the buffer this milestone assumes, at the
address and format it claims) can only be verified by booting the real board.

## Why this is the path, not the bare-metal GPU driver

The display ladder already ruled out the alternative and said why: a bare-metal driver for the
VisionFive 2's BXE-4-32 3D core is a Linux-scale, multi-year effort (loaded firmware, thin
documentation, Mesa still maturing on Linux itself) that proves nothing rung four (virtio-gpu 3D,
milestone 34) does not already prove in QEMU. `simple-framebuffer` sidesteps mode-setting entirely:
U-Boot has already configured the display and put a linear framebuffer at a known physical address
before nife's kernel ever runs, the same convention Linux's own `simplefb`/`simpledrm` drivers and
every other bare-metal OS booted by U-Boot or UEFI GOP relies on. nife's job is to read where that
buffer is and write pixels into it, not to program a display controller.

## What it needs

- **Read the handoff, not assume it.** U-Boot advertises the framebuffer through a device-tree node
  (conventionally `compatible = "simple-framebuffer"`, with `reg`, `width`, `height`, `stride`, and
  `format` properties) or through the `chosen` node, depending on U-Boot's configuration on this
  board. Confirm which shape this board's U-Boot actually produces before writing a parser against
  an assumed one -- this is exactly the kind of premise DECISIONS's own six-questions discipline
  says to check rather than guess.
- **A driver, not a new protocol.** Rung one already built the framebuffer contract
  (`crates/gfx_proto`, notes/framebuffer-contract.md, DECISIONS §29): the pixels are proved by two
  witnesses in two address spaces, and the contract is deliberately transport-agnostic. This
  milestone's driver should be a new backend behind that same contract -- read a pre-set buffer
  instead of negotiating virtio queues -- not a reason to design a second contract.
- The DMA trust question, inherited rather than reopened. `notes/verification.md` already found
  this: the JH7110 has no IOMMU, so "a display driver on the VisionFive 2 is therefore either
  *trusted* with all of physical memory, or the transport grows a virtio-gpu-aware check and pays
  the §18 cost knowingly." A `simple-framebuffer` driver inherits the same fact -- it is a fixed,
  pre-negotiated buffer rather than a virtqueue, so there is no descriptor to validate against.
  State the resulting confinement (or its absence) plainly, the way `notes/verification.md` already
  asks whoever sequences board display work to.
- Serial input in the interim. No keyboard driver work here; USB HID gets its own milestone per
  rung five's own text. The terminal's input side keeps using the serial console until that lands.

## What this does not decide

**The frame-ceiling fork is independent of this milestone and does not block it.** `notes/frames.md`
found that a `Frame` capability costs a cspace slot and a real terminal-sized surface (800x600 needs
469 frames) cannot fit in the sixteen-slot budget -- but that fork is about surface *size*, not
about *transport*. This milestone can land at whatever resolution today's frame budget already
supports; a bigger, "worth living in" terminal (milestone 142) still needs that fork answered
separately, on real hardware or in QEMU alike.

Mode-setting (choosing or changing the display mode nife's side) is explicitly out of scope, per
rung five's own text: "a mode-setting driver only if ever needed." Build it only if
`simple-framebuffer`'s fixed mode turns out to be insufficient, not preemptively.

## What it unblocks

The first real-hardware display output this kernel has had. Sequenced before milestone 142 (a text
display good enough that people use it instead of a GUI) can mean anything on real silicon, and
before milestone 49's boot-wiring work can boot into a *display* rather than only a serial prompt.

## Prior art

Linux's `simplefb`/`simpledrm` drivers are the exact same shape: read a bootloader-provided
framebuffer description, do no mode-setting, defer everything else. U-Boot's own EFI GOP handoff on
UEFI-booting platforms is the same convention one layer up the boot chain.

## Index row

The display ladder's rung five (struck 2026-07-28) named the path and never got a milestone: no
bare-metal GPU driver, read U-Boot's pre-set framebuffer instead, zero mode-setting code. A new
backend behind rung one's existing framebuffer contract, not a new protocol. Gate: HARDWARE, since
the handoff itself can only be verified by booting the real board.
