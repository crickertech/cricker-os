---
status: PARTIAL
raised: 2026-08-30
milestone_dependencies: 242
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: yes
---
# 192. A keyboard on real silicon: the input half of every graphical story, which nothing owns

Minted 2026-08-30, discovered while tracing journey 3 (login to `kilo` on
real silicon) the way journey 1's trace discovered milestone 177 (wire the graphical terminal stack
into the real interactive boot). **Option A's wiring built 2026-09-02**
(`milestone/192-keyboard-silicon`); see "What option A's lane built" below, and the paragraph after
it on why the status stops here rather than at BUILT.

**Decided 2026-08-30 by calef:** *"we will do Option A first and follow with Option B
later."* Both options are in scope; the decision is their order, not a choice between them.

## What done looks like, and option A does not meet it

**calef, 2026-08-30, correcting this block within the hour it was written:** *"192 isn't done until
we can sit down at the keyboard connected to the computer and display the OS on the dedicated
monitor."*

So the completion criterion is literal and it is option B: **a keyboard plugged into the machine, and
the OS on a monitor plugged into the machine.** Option A is a first increment that reaches `PARTIAL`
and nothing more. A block that let serial input close this milestone would be claiming a story the
system cannot tell, which is what §39 says a name must never do and what a completion criterion must
never do either.

**Why A still goes first.** It gets pixels onto the real monitor and a usable keystroke path with no
new driver, so every other step of journey 3 (login to `kilo` on real silicon, on all three
architectures) can be exercised on real hardware while the USB work is still unwritten. It buys
sequencing, not completion.

**What that requires of A's implementation**, and this is the part that is easy to get wrong: nothing
in A may assume keystrokes arrive on a UART. Both paths feed the same `DECISIONS §21`
line-discipline contract, and **A is finished when the keystroke's source is the only thing B has to
change.**

**In brief.** Every graphical story this project tells assumes a keyboard, and on real hardware there
is no way to press a key. Milestone 29 (a display terminal: framebuffer, virtio-gpu, and a foreign
component) gets keystrokes from **virtio-input**, which is a QEMU device. None of the three
boards has one. **A search of the entire roadmap for `usb`, `xhci`, `hid` and `ps/2` returns
nothing**, so this is not a milestone that exists and is unscheduled; it is work nobody has written
down at all.

## Why it went unnoticed for so long

Because every consumer of input so far has been either a test harness or a serial console, and both
work. The plain UART `console`/`input` pair the real interactive boot spawns today reads keystrokes
over the UART, which is real on all three boards. Nothing broke, so nothing asked.

It only becomes visible when the story is *a person sitting at the machine*, which is what journey 3
is and what journey 1 is a QEMU-shaped rehearsal for.

## What option A's lane built (2026-09-02), and why the status did not move to BUILT

**The status is `PARTIAL` and stays there.** calef's completion criterion above is literal and it is
option B: a keyboard plugged into the machine and the OS on a monitor plugged into the machine.
Nothing in this lane's work lets anyone sit down at a keyboard, and flipping this block to BUILT
would claim exactly the story he said it does not tell. Milestone 159's lane wrote the same
paragraph for the same reason and it is the right precedent: a lane may advance a milestone without
being allowed to close it.

**What existed already, checked rather than assumed.** Input has an owner and milestone 177 already
built the graphical boot around it:

- `components/src/input.rs` is the UART receive driver, raw since milestone 28. It holds `WRITE` on one
  terminal endpoint, `READ` on the UART receive `Irq`, and one device-typed page of registers. It
  forwards bytes as `line_editor::proto::OPERATION_BYTES`, up to eight per `CALL`.
- `components/src/keyboard_driver.rs` gained `MODE_DIRECT` in milestone 177: a virtio-input driver
  holding `WRITE` on one fixed endpoint, sending **byte for byte the same `OPERATION_BYTES` framing**.
- So the two sources were already interchangeable at DECISIONS §21's line-discipline contract.
  Nothing had ever put them behind one choice, and the graphical boot's condition was "a GPU
  and a keyboard", which on argon, radon and xenon is never true.

What the lane added is one choice and its second arm. `kernel::user::boot_graphical_terminal`
now picks the keystroke source: the virtio keyboard when the bus has one, the board's own UART when
it does not. `kernel::user::input_service::start_direct` is `keyboard_service::start_direct`'s twin,
spawning `input` kernel-side against the endpoint the kernel creates before init exists (which is
the reason the keyboard is spawned there too, recorded in full in milestone 177's finding 1).
`crates/system_initializer` needed no line changed: it cannot tell which source it got, and
that it cannot is the property this block asked option A to be finished by.

Option B is a third arm of that `match`. Nothing else in the guest is downstream of the choice.

## What authority the serial source holds, which is the risk-6 claim

Stated precisely, because milestone 159's lane established that precision is what makes a
confinement claim evidence rather than a slogan (its own driver: one page of device memory and two
rendezvous capabilities, no DMA page and no `Irq` cap).

`input`, spawned by `input_service::start_direct`, holds two capabilities and one mapping:

| What | Rights | Why that and no more |
|---|---|---|
| the terminal endpoint (slot 0) | `WRITE` | it may `CALL` exactly one destination, fixed at spawn by the kernel, and can name no other |
| the UART receive `Irq` (slot 1) | `READ` | WAIT and ACK. No `GRANT`, so it cannot hand the interrupt on |
| one page of UART registers | device-typed | the receive side of a PL011 or an NS16550 |

It holds no DMA page, no `Virtio` transport, no budget, no report endpoint, and no
capability naming any other process. It cannot print, cannot spawn, and cannot read what anyone
else typed. This is `components/src/input.rs`'s documented authority unchanged; the only thing this lane
altered is who spawns it.

The honest comparison with option B, since risk 6 is what this is evidence for: a UART is a
memory-mapped register file with an interrupt, and confining one proves considerably less than
confining a bus that enumerates whatever a stranger plugs in. An xHCI driver needs DMA, and on a
board with no IOMMU (`notes/verification.md` on the JH7110) that is the whole question. Option A
is not evidence about xHCI. It is evidence that the *seam* holds: a real, non-virtio input source
reaches the line discipline through a capability naming one endpoint.

## What is proven, and where

In QEMU, on both architectures (`script/swish-check`, folded in 2026-10-03 UTC): a boot
with a virtio-gpu and no keyboard brings up the GPU driver and the display terminal (both
asserted before anything else happens), chooses the UART, spawns `input` against `line_editor`'s
endpoint, and prints `keystrokes: Serial (graphical boot)`. aarch64 and riscv64 both reach that
line. On x86_64 the serial source refused itself (`machine_has_no_device_page_for_the_console`),
and the whole graphical stack reported absent.

*Corrected 2026-10-03 (UTC): that refusal belonged to the retired kernel-side
`boot_graphical_terminal`. A `graphical_terminal` session reads the boot's line discipline raw, and
on x86_64 that is fed by `input` over a COM1 `PortRange` (milestones 299 and 505), so the serial
source needs no new driver. x86_64 also has an interactive boot, `swish-check-x86_64` in CI. It lacks
a virtio-gpu in its runners (milestone 270 (wire `virtio-gpu-pci` and `virtio-input` into the x86_64
test runner)), and milestone 632's BUGS lists what a graphical leg needs.*

*Corrected 2026-10-03: this paragraph said a keystroke could not reach the screen, blocked by
milestone 177's second-`FLUSH` hang. That hang was root-caused 2026-09-19, and both legs have been
green in CI since.*

And the lane found `script/swish-check` red on `main` for an unrelated reason. With a virtio-rng
attached (`NIFE_RNG=1`, which both plain legs set unconditionally) the interactive boot traps in
init on aarch64 and riscv64 alike, before the console prints anything; with the device absent the
same build reaches a prompt normally. Reproduced at `8167d806` on nightly-2026-09-01 as well as
-09-02, so it is not the toolchain bump; the shape matches the sixteen-slot wall
`crates/system_initializer`'s own comments describe around the entropy build. It has no owner and
is recorded in that crate's `BUGS` rather than held in this block, since a reader meets it there.

## The bench procedure, in order

It cannot be run yet, and the reason is milestone 157 (U-Boot's `simple-framebuffer` handoff) rather than this milestone. Option A is
framebuffer output plus serial input, and the framebuffer half is
[157](0157-uboot-framebuffer-handoff.md) (U-Boot's `simple-framebuffer` handoff), which is
NOT-STARTED. Until a board can put a pixel on a monitor there is nothing for a serial keystroke to
echo onto. Written now so that 157's lane inherits it rather than rederiving it.

Needs radon powered on (the Kasa plug), a serial console attached before power, a microSD card, and
a monitor on the board's HDMI. `notes/visionfive2.md` carries the board facts and the
failure-triage ladder.

1. Follow milestone 159's steps 1 to 3 verbatim (`script/board-image`, copy all three files from
   one run, DIP switches to QSPI, serial at 115200 8N1, interrupt U-Boot, the five `load`/`booti`
   lines). Nothing about this milestone changes the boot recipe.
2. Read the boot line `keystrokes:`, which is the whole of what this lane added and the first
   thing to check:

   | Line | What it means |
   |---|---|
   | `keystrokes: Serial (graphical boot)` | What a board should print. The framebuffer driver came up, the display terminal came up, and the UART is the keystroke source. |
   | `keystrokes: Keyboard (graphical boot)` | A virtio-input device was found, which on a board means something is wrong with device discovery, not that a keyboard works. |
   | no `keystrokes:` line at all | The graphical stack reported absent and the boot fell back to the plain `console`/`input` pair. Either no framebuffer was found (157's problem) or one of `gpu_driver`, `display_terminal`, `input` is missing from the archive. |

3. Look at the monitor. A prompt on the screen is 157's claim, not this one; record it either
   way, because everything below depends on it.
4. Type a character on the serial console. It must echo on the monitor, not on the serial
   line: `input` forwards it to `line_editor`, which echoes through `display_terminal`. If it
   echoes on the serial console instead, the boot took the plain fallback and step 2 said so.
5. Type `echo hello` and press return. `hello` on the monitor is option A complete on a real
   machine: a person at a laptop typing across a null-modem cable into a system whose output is on
   its own screen.
6. Failure modes, in the order they are worth checking. No echo at all and no `keystrokes:`
   line means the fallback (step 2). No echo with a `Serial` line means either the UART receive
   interrupt is not routed (the same PLIC-source hazard `uart_irq_and_source`'s own doc records for
   the JH7110, where the QEMU constant armed the wrong source) or the display path hung, which
   milestone 177's display blocker above would produce identically. A character echoing once and then nothing
   is the display driver's second-`FLUSH` bug, and it is the same bug QEMU shows.

## What option B will still need

Unchanged by this lane except that the wiring is no longer part of it: an xHCI driver, enough USB
core to enumerate and configure a device, and a HID keyboard driver, on three architectures, plus
the scoping pass that splits those into their own milestones. What it will not need is any
change to `line_editor`, `display_terminal`, `swish`, `crates/system_initializer`, or the boot's
capability layout: a HID driver that holds `WRITE` on one endpoint and sends `OPERATION_BYTES` is a third
arm of `boot_graphical_terminal`'s `match` and nothing else.

## The fork: what a keystroke arrives on

Option A: serial input, framebuffer output. Display on the board's own framebuffer (milestone
157, U-Boot's `simple-framebuffer` handoff), keystrokes over the existing UART. On the OptiPlex this
is already wired: milestone 87 (the x86_64 bare-metal machine) bought a Dell C4PDJ serial module and
an RS-232 chain precisely so a host can drive the box. Zero new drivers. The cost is that the story
becomes "a person at a laptop typing into a machine across a null-modem cable", which is a real
demonstration of a real system and is not the story journey 3's title claims.

Option B: a USB HID stack. An xHCI driver, enough USB core to enumerate and configure a device,
and a HID keyboard driver, on three architectures. This is the honest version of "sit down at the
machine", and it is large: xHCI alone is a substantial driver against a substantial spec, so plan in
months rather than weeks. It also lands in the most attacker-adjacent position in the system, a bus
that enumerates whatever a stranger plugs in, which is either a problem or the best confinement
demonstration available depending on how it is built.
Note the peer project Atom keeps xHCI in the kernel, which is exactly the contrast DECISIONS §31
(the foreign-language seam) already draws for FAT32.

Option C: split the difference by architecture. Serial input on the boards, a cheaper native
path where one exists. This had exactly one candidate and it is now closed: a PS/2 port on the
OptiPlex would have been a few dozen lines against xHCI's spec, and calef confirmed on 2026-08-30
that the 7050 Micro has no PS/2 port and that he owns neither a PS/2 keyboard nor an adapter. So
there is no cheap native shortcut on any of the three machines, and the fork is genuinely binary.
Recorded rather than deleted, because the next person will have the same idea.

When B starts is not decided here, only that it is coming. The trigger is the fatal-risk list
thinning out enough that months of bus-driver work is a reasonable thing to spend.

## Why A goes first, which was the recommendation

Recorded here rather than withheld, correcting this block's own first draft. It said "no
recommendation is offered, deliberately", and that was wrong by AGENTS.md's rule: recommend on
reversible forks, give options only on irreversible ones. This fork is reversible. Choosing
serial input forecloses nothing, because option B can be built at any point afterward, against the
same `DECISIONS §21` line-discipline contract that both the UART console and `display_terminal`
already speak identically.

The reasoning is about what journey 3 is for. That journey exists to test two of the nine
entries in design/fatal-risks/README.md: risk 9 (the HAL is a fiction and an architecture costs a
restructure) and most of risk 6 (a confined driver cannot drive real hardware). A framebuffer on
real silicon tests both of those. A keyboard adds almost nothing to the test. It adds to the
story, and the story matters, but not enough to spend months on a bus driver before the other seven
risks have been answered at all.

The honest cost, stated so it is a known trade rather than a hidden one: while only A exists the
demonstration is *a person typing into the machine across a null-modem cable* rather than *a person
sitting at the machine*. That is weaker, and anyone watching will notice. It is why A cannot close
this milestone.

What moves B up the queue: a customer, or an audience, for whom the difference between those two
sentences is the product. B is also the better answer the moment nife has survived enough of the
fatal-risk list that months of driver work is a reasonable thing to spend.

## What is true either way

- The framebuffer half is milestone 157 and is unaffected by this fork.
- Option A is a strictly smaller superset of nothing: it needs no new milestone at all, which is
  why it is worth pricing honestly against B rather than dismissing.
- If B is chosen, it is not one milestone. xHCI, USB core, and HID are three, and the scoping
  pass is the first piece of work.

## BUGS

- This block prices neither option. The xHCI estimate is an adjective, not a measurement, and
  nobody has looked at what the JH7110, the Jetson TX1 and the OptiPlex each expose.
- It says nothing about a mouse, which milestone 33 (a compositor: one screen, mutually distrusting clients)'s compositor will eventually want and which
  the same stack would carry.
- The decision is a sequencing call, not a claim that option B is wrong. A demonstration
  operating system that cannot accept a keyboard is limited in a way no scope note repairs, and this
  block should not be read as saying otherwise.
- Option A has still never been run on a board, and that remains where the unglamorous
  problems live. What changed on 2026-09-02 is narrower than it sounds: the wiring exists and comes
  up in QEMU on both architectures, so "zero new drivers" is no longer only a reading of the code.
  Nobody has yet displayed a terminal on a *board's* framebuffer while taking keystrokes over that
  board's UART, and nobody can until milestone 157 lands.
- *Corrected 2026-10-03 (this bullet said both graphical legs were red).* They are green in CI
  since milestone 177's 2026-09-19 root cause. Since milestone 632 (graphics on demand) they launch
  `graphical_terminal` from the normal UART boot, so they prove option A at launch rather than at boot.
## Follow-on

- **Milestone 242.** Option B, the xHCI driver, USB core and HID keyboard, plus the scoping pass
  this block asks for, is `design/roadmap/0242-usb-host-and-a-keyboard-that-is-not-a-uart.md`,
  minted 2026-09-03 by calef precisely so option B stops being priced as a line item here.
- **Milestone 230.** That is milestone 230 (`script/shell-check` is red on `main`), which keeps the
  name the script had then. `script/swish-check` red on `main` with a virtio-rng attached was not unowned:
  `design/roadmap/0230-shell-check-is-red.md` is BUILT 2026-09-02, root-caused to slot exhaustion
  four blocks later than this block guessed, and both plain legs are green.
- **Outstanding.** Option A has still never been run on a board and cannot be until milestone 157
  puts a pixel on a board's framebuffer. The six-step bench procedure is written and unexecuted.
  Checked 2026-09-03.
- **Outstanding.** No keystroke has reached a screen on real silicon; that waits on milestone 157.
  *Corrected 2026-10-03: the QEMU half of this bullet (red legs, nothing in CI) was overtaken.*
- **Milestone 179.** A mouse is `design/roadmap/0179-mouse-pointer-input.md`, minted 2026-08-26,
  rather than an unnamed gap this block has to carry.
- **Outstanding.** Neither option is priced. Nobody has looked at what the JH7110, the Jetson TX1
  and the OptiPlex each expose for USB, and milestone 242 declines to price xHCI in hours as well,
  so the adjective is still an adjective. Checked 2026-09-03.
- **Recorded.** The order is a sequencing call rather than a claim that option B is wrong. A system
  that cannot accept a keyboard is limited in a way no scope note repairs, and calef's completion
  bar for this block stays literal.
- **Recorded.** Option C is closed: the 7050 Micro has no PS/2 port and calef owns neither a
  keyboard nor an adapter, so there is no cheap native shortcut on any of the three machines.

## Index row

Discovered 2026-08-30 by tracing journey 3, the way journey 1's trace discovered milestone 177.
Nothing in nife can read a keypress on real hardware: milestone 29's driver is virtio-input, a
QEMU device none of the three boards has, and the whole roadmap returns nothing for `usb`, `xhci`, `hid` or `ps/2`. Decided 2026-08-30: both options are in scope and the decision is their
order. Option A (serial input, framebuffer output, no new driver) goes first so the rest of
journey 3 can be proved on hardware; option B (xHCI plus USB core plus HID, on three
architectures, months) follows. Done means a keyboard plugged into the machine and the OS on its
own monitor, so A reaches PARTIAL and cannot close this. The one cheap shortcut is closed: the
OptiPlex has no PS/2 port. Option A's wiring built 2026-09-02: the graphical boot's keystroke
source is now one choice in `kernel::user::boot_graphical_terminal` (a virtio keyboard when the
bus has one, the board's own UART when it does not), `crates/system_initializer` needed no line
changed, and option B is a third arm of that `match`. *2026-09-30: the choice moved to the launch
of a `graphical_terminal` session (milestone 632); a session without a keyboard reads the boot's UART raw.* The
board half waits on milestone 157.
