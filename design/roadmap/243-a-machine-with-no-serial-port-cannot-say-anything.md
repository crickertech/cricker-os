---
status: BUILT
raised: 2026-09-03
built: 2026-09-19
---
# 243. A machine with no serial port has no way to say anything, and no gate can read it

<!-- writing-standards: exception. Granted 2026-10-04 (UTC) by the maintainer for one correction of a false claim (the scroll read-back bullet), not ratified by an architect. Reason: this block stands over the bold density, and a correction must not wait on a rewrite (the class the 2026-09-29 note in milestone 586 (a prose ratchet in lint) records). Bringing it under the density is the debt, and the exception goes when that is done. -->

Built 2026-09-19. Minted 2026-09-03 by calef, from asking how nife reaches commodity
hardware. Built in two passes: 2026-09-04 put the boot tour on a UEFI machine's framebuffer and
gave a gate a way to read it under QEMU; 2026-09-19 closed the block's two remaining Outstanding
items, **early boot** and **the other two architectures**. See `notes/serial-less-output.md`.

**What BUILT means here, stated plainly, because two of the three problems below are answered
rather than solved.** A kernel that can write its diagnostics somewhere other than a UART now does
so on all three architectures; the window before the kernel exists is *bounded* rather than
narrated, and it cannot be narrated (see below); and a gate can read a screen under emulation on
all three, while reading a *real* serial-less machine is milestone 369 (the fleet is not virtual)
and has always been its own block.

## What was built, and what it does not cover

**The mechanism chosen is the firmware's own linear framebuffer.** UEFI's
`EFI_GRAPHICS_OUTPUT_PROTOCOL` reports one, and **its address survives `ExitBootServices`**, because
the aperture is a BAR on the display adapter rather than firmware memory: what ends is the firmware's
*console*, not the *display*. That single fact is why this milestone is a week rather than a quarter.

| Piece | Where |
|---|---|
| The sentence one boot stage says to the next | `machine_discovery::framebuffer`, riding PVH's own `cmdline_paddr` (a field the format has always had and nothing ever read) |
| Asking the firmware | `uefi_loader::find_screen`, one `LocateProtocol` call |
| Painting text | `crates/screen_console` (**name provisional**), sharing `bitmap_font` with the graphical terminal and sharing nothing else with `video_terminal` |
| The kernel seam | `console::attach_screen`, teed under the lock `print!` already takes; `arch::x86_64::mmu` carries the aperture into the fine map |
| **A gate reading a screen** | `board_console::screen`: a screendump decoded back into text by exact glyph match, judged by `board_console::progress` unchanged |

**Measured under OVMF, 2026-09-04:** 1280x800 bgrx at `0x80000000`, 182x100 cells, and
`cargo xtask uefi-boot` reads **96 non-blank rows of the boot tour back off the framebuffer**. That
assertion is the one the serial transcript could not make: the transcript would be identical if the
screen were black.

**One thing it still does not cover, and two that were closed on 2026-09-19:**

- **Problem 2 on real hardware, still open and now its own block.** A gate can read a *virtual*
  machine's screen. Nobody can ask Graeme's laptop for a screendump, so the fleet's record is still
  a photograph and a person. That is **milestone 369**, and it is blocked on USB mass storage.
- **Problem 3, early boot: bounded, and it cannot be more than bounded.** `uefi_loader` clears the
  screen and writes two lines into the framebuffer as its last act, after `ExitBootServices` and
  before the jump, so "the kernel never reached its first statement" is now a thing a person at a
  monitor can *see* rather than a black screen it shares with three other failures. See the section
  below.
- **The other two architectures: a screen under the emulator, and none on silicon.** `ramfb` closed
  the QEMU half with no change at all to the arch-neutral console, which is what those halves were
  written for. The board half is milestone 157's U-Boot handoff and stays there. See the section
  below.

**In brief.** Every word nife has ever said, it said down a UART.

- The boot tour, on all three machines.
- The console server and the shell (DECISIONS §21's line discipline).
- Kernel fault reports, which milestone 230 (`script/shell-check` is red on `main`, on both
  architectures, and nothing says so) found interleaving with the console server's own output because
  two address spaces drive the one device.
- **And every automated gate**: `script/board-console` (milestone 216), the soak's heartbeat, and
  milestone 218's (every boot of the VisionFive 2 needs a human typing four commands into U-Boot)
  boot script all read that line.

**A commodity machine does not have one.** xenon does, and that was chosen rather than lucky:
milestone 87 (the x86_64 bare-metal machine) picked a Dell with a C4PDJ serial module and a null
modem. So this was consciously deferred, and this block is where the deferral stops being implicit.

## Three problems wearing one name

**1. A human watching a machine boot.** Answered by the framebuffer chain: milestone 157 (the U-Boot
framebuffer handoff), milestone 242 (USB host and HID) and milestone 177 (wire the graphical terminal
stack into the real interactive boot). That path exists and is partly built.

**2. A gate reading a machine.** **No answer exists.** Every instrument this project has for
unattended hardware reads a serial line. The candidates each cost something:

- **A network console.** `smoltcp` works and radon's tree describes two Ethernet controllers, but it
  needs a NIC driver per board and says nothing until the stack is up.
- **Postmortem to storage**, which is what a headless panic actually needs and what nothing here
  does.
- **The firmware's own console.** UEFI `ConOut` works before `ExitBootServices` and therefore covers
  the loader and not the kernel.

**3. Early boot, which is the hardest.** **Before the framebuffer exists, a serial-less machine is
silent**, and a panic there produces nothing at all. This is not a nife peculiarity: it is why
commodity operating systems ship a firmware console, a splash, or postmortem logging. **nife has
none of the three.**

## The fleet this unlocks, which calef named on the day this was minted

> if I can boot a nife system off of a USB drive, then that opens up a lot of different hardware in
> our house that I can use for testing: Graeme's laptop, his desktop, cordoba, two MacBooks, Clay's
> desktop
>
> -- calef, 2026-09-03

**That fleet is already reachable and nobody had noticed.** Milestone 87 (the x86_64 bare-metal
machine) boots from a FAT32 stick at `\EFI\BOOT\BOOTX64.EFI`, which is the removable-media
fallback **every** UEFI firmware looks for with no configuration. Nothing about it is specific to
xenon.

**And `uefi_loader` already writes to the firmware's `ConOut`**, so on any UEFI x86_64 machine a nife
stick prints to the monitor with no driver of ours, right up to `ExitBootServices`. After that the
kernel takes over and the machine goes silent, which is exactly this milestone.

**So the blocker on using those machines is this block and not milestone 242** (USB host and HID,
because on commodity hardware the keyboard is not a UART). **A boot test needs output, not input.**

Two qualifications, both worth stating rather than discovering:

- **The Apple Silicon MacBooks are not in this fleet, and not for the reason first given.** They boot
  non-Apple kernels legitimately, and `notes/target-hardware.md` records that Asahi Linux is built on
  a documented permissive-security mode with an Apple-signed `m1n1` payload. But they have **no
  UEFI**, so a `BOOTX64.EFI` stick will not start one: that is a port, filed in that note as
  "possibly the fifth", not a boot. An Intel MacBook is in the fleet. Worth knowing for later: m1n1
  offers a serial console over USB-C, which is more than any commodity x86 machine here offers.
- **Milestone 195 (finish the UEFI boot path) created a question each machine answers for itself**:
  `PHYS_START` moved from 1 MiB to 32 MiB because OVMF holds ACPI NVS and its own allocations across
  the low range. Whether a given machine leaves 32 MiB free is that machine's answer, and the loader
  now prints which range it wanted and which descriptors are in the way rather than `Load Error`.

**The cheap experiment this makes available:** build a stick, boot one of those machines, and read
what the loader says on the screen. Even with the kernel silent afterwards, that establishes whether
the firmware finds the stick, whether Secure Boot refuses it, and what the memory map looks like away
from OVMF. **A loader that prints and a kernel that then goes quiet is this milestone confirmed by
evidence rather than by reasoning.**

**That experiment is now worth more than it was when this was written**, and the procedure for it is
`notes/serial-less-output.md`'s bench section. The kernel no longer goes quiet: it clears the screen
and prints its whole tour there. So the same stick, on the same machine, now distinguishes four
outcomes rather than two, and each is a different bug: the firmware not finding the stick, the loader
refusing and saying why, the screen clearing and staying black (the kernel armed its console and died
after), and the tour.

## Why it is worth one block rather than three

Because the answer might be one mechanism. A kernel that can write its diagnostics somewhere other
than a UART serves the human, the gate and the panic at once, and choosing three separate answers
would be the expensive way to discover that. **This block does not pick the mechanism**, and the
choice is the milestone.

The constraint that should drive it: **whatever it is has to work when the thing being reported is
the reason the machine is broken.** A network console needs a working stack; a filesystem log needs
a working filesystem; the UART needed neither, which is why it was the right first answer and why
replacing it is harder than it looks.

## Follow-on

- **Milestone 369.** A gate that can read a serial-less machine on real hardware, which is the half
  of this block that QEMU answered and silicon did not.
  `design/roadmap/369-a-gate-that-can-read-a-machine-with-no-serial-port.md`.
- **Milestone 377.** One screendump decoder rather than two: milestone 177's graphical
  `swish-check` leg carries `parse_ppm`/`decode_cell`/`scanout_rows` inside `xtask` and this
  milestone wrote a second, more general one in `board_console::screen`.
  `design/roadmap/377-one-screendump-decoder-not-two.md`.
- **Done.** Problem 3, early boot, closed 2026-09-19. The premise was checked and half of it is true:
  `kernel/src/arch/x86_64/boot.s` writes to no device and cannot, because it is a 32-bit
  instruction stream with no idea where the screen is and no IDT, so a fault in it is a triple
  fault and a reset. What *can* be done is done by the stage before it. See the section below.
- **Milestone 511.** milestone 511 (the boards' screen comes from the firmware), `design/roadmap/511-the-boards-screen-under-uefi.md`. The boards' screen under
  UEFI, which milestone 441 (the program that makes the stick) made reachable while this lane was
  running. `uefi_loader` now has aarch64 and riscv64 boot files, so on those architectures
  there is, for the first time, a firmware stage that has already lit a display and can be asked
  about it. Two things fall out and neither is built: the loader could paint this block's handoff
  banner on those architectures too (`find_screen` is under `arch/x86_64/` today), and it could
  carry the screen to the kernel by synthesising a `simple-framebuffer` node in the device tree it
  already copies, which is the *same* node milestone 157 will read from U-Boot. That second one is
  the interesting half: it would give the boards a real firmware framebuffer with no `ramfb` and no
  `.bss`, and 157's parser would serve both. Not done here because it is a wire format between two
  boot stages and belongs with 157's own premise check.
- **Done.** aarch64 and riscv64 have a screen under QEMU, closed 2026-09-19,, through `ramfb`. The
  arch-neutral halves needed no change, which was the claim they were written to make good on.
  Milestone 157 remains the board half. See the section below.
- **Done.** 2026-10-04 (UTC): the scroll redraws from cells in RAM and never reads the aperture,
  which `x86_64` now maps write-combining (#1645), and the screen terminal fills the screen
  (#1648). `crates/screen_console/src/lib.rs`'s `BUGS` has the counts.
- **Recorded.** Only ASCII reaches the screen, so `§` in the tour's last line is two blanks there and
  correct on the UART. `notes/serial-less-output.md`'s `BUGS`.
- **Refused.** A network console, on the block's own argument: it needs a driver per machine, says
  nothing until the stack is up, and cannot report the failure of the thing carrying it.
- **Refused.** Reading a *photograph* of a screen. `board_console::screen` works because a screendump
  is pixel-aligned with exact glyph matches and no threshold; a camera has none of those properties
  and making it work is optical character recognition, which is a different project.

## Early boot, 2026-09-19: bounded rather than narrated

**The window cannot be made to speak, and that is a finding rather than a concession.** From
`ExitBootServices` to `console::attach_screen` there is no console at all: the firmware's is gone by
specification, the kernel's is not up, and everything in between (the loader's mode-switch
trampoline, `boot.s`'s 32-bit half, the page tables, the long-mode jump) runs 32-bit with no IDT.
A fault there is a triple fault and a silent reset. No code anybody could write in that window knows
where the screen is.

**So the window is bounded instead.** `uefi_loader` clears the screen and paints two lines as its
final act, after `ExitBootServices` and before the jump:

```text
nife loader: firmware released, entering the kernel.
If this line is still here, the kernel stopped before its console came up.
```

The screen is therefore never blank across the handoff, and five outcomes are distinguishable where
four used to share one appearance: the firmware never started the stick; the loader refused and said
why over `ConOut`; **the kernel never reached its first statement** (this line, still there); the
kernel armed its console and died after (black); the boot tour. **The third is the one that did not
exist**, and it is the whole of what is buyable.

**Proved rather than reasoned.** With a temporary halt in place of the jump, `cargo xtask uefi-boot`'s
screendump had ink in exactly the top sixteen pixel rows of a 1280x800 screen, reading those two
lines and nothing else.

**The boards get nothing equivalent and the reason is the boot chain.** Under QEMU they are entered
from `-kernel` with no stage before the kernel to paint anything, and the kernel's own `ramfb`
framebuffer is in `.bss`, which `boot.s` has not finished zeroing at the point a banner would have to
be written. On the VisionFive 2 the stage that could speak is U-Boot, which is milestone 157. Their
dark window is `boot.s` plus one `fw_cfg` conversation, a few hundred instructions.

## The other two architectures, 2026-09-19: `ramfb`

**`x86_64` gets a screen because the firmware already lit one.** The two `virt` boards are entered
from `-kernel` with nothing configured, so there was nothing to discover and no amount of
arch-neutral console code could help. What those boards can present is `ramfb`, which inverts the
arrangement: **the guest owns the pixels** and tells the emulator their physical address over
`fw_cfg`, after which QEMU scans them out continuously. Nothing sits between a `println!` and the
picture.

| Piece | Where |
|---|---|
| The `fw_cfg` wire format, host-tested (10 tests) | `crates/firmware_configuration` (**name provisional**) |
| The register poking, two DMA transactions before `arch::mmu::init` | `kernel/src/drivers/ramfb.rs` |
| The memory and the wiring | `kernel/src/screen.rs` (**name provisional**), `console::peek_screen` (**provisional**) |
| Painting text | `crates/screen_console`, **unchanged** |
| The gate | `cargo xtask screen-boot <arch>` (**name provisional**), `uefi-boot`'s twin, decoding with `board_console::screen` **unchanged** |

**Measured 2026-09-19**, and both legs run inside `script/test`:

```text
screen-boot: read 33 non-blank row(s) of the aarch64 tour back off a ramfb, ending
screen-boot:   | nife self-test: 5 of 5 passed
screen-boot: read 63 non-blank row(s) of the riscv64 tour back off a ramfb, ending
```

**The surprise worth recording is what the inversion broke.** Milestone 400's handover maps the
screen's physical range into a userspace driver, which is right for a display adapter's BAR and is a
hole for a framebuffer that is the kernel's own `.bss`. `user::boot_screen_terminal` now refuses a
screen inside the kernel image, checked *before* the yield so a refusal leaves the kernel still
painting rather than a cleared screen nobody owns. It is a range test rather than a flag, so
milestone 157's U-Boot aperture will pass it with nothing to remember.

**It is a boot of its own rather than a stage of the suite, and that is forced.** `ramfb` adds a QEMU
console, `screendump` with no device argument writes console 0, and the suite's machine already has a
virtio-gpu there.

### The x86_64 gate was asserting a race, and a measured premise was wrong

**Found 2026-09-20 while re-verifying after a rebase, and it is not this lane's code.**
`cargo xtask uefi-boot`'s tour stage failed intermittently on a loaded dev Mac: four consecutive
runs read 19, 27, 40 and **0** rows. Milestone 400's own BUGS predicted it in those words ("a much
faster guest or a slower screendump could miss it").

**The lane's first instinct was that its own loader banner had caused it, and that was tested rather
than assumed**: with the banner's paint disabled and nothing else changed, the same gate failed
**two runs in four**, which is worse. The banner is not the cause, and the ninety seconds that test
cost were the cheapest ninety seconds in this lane.

**The actual cause is a premise that was false when it was written.** The marker was
`boot_ladder::SELF_TEST`, chosen "near the end of the boot on purpose" because a 1280x800 screen is
100 character rows and the boot was believed to be longer, so early lines would have scrolled off.
**Measured, the tour tops out at 98 non-blank rows**, so nothing scrolls and the first line is on
the screen the whole time. What the marker actually selected for was the *last* line before the
handover's clear, a window of a few hundred milliseconds, which is exactly the race the comment
claimed to be avoiding.

**The marker is now the banner, the first line.** Five consecutive runs read 52, 68, 78, 84 and 89
rows and all passed, where the same machine had been failing one run in two.

**The total claim is unchanged**, which is the thing to check before believing any gate got easier.
The screen's job is to prove the *pixels*: `LocateProtocol`, the byte order, the stride, the mapping
surviving `mmu::init`, and the glyphs. Any decoded row proves all five. That the boot reached the
self-test is asserted separately and unconditionally on the **serial** transcript a few dozen lines
above. So the change trades nothing away; it stops the gate asserting something it never meant to.
How deep the dump caught the tour is now *reported* rather than required.

**This edits the gate of milestone 400 (the shell on the firmware's screen), not this one's**, and
400's `BUGS` entry about that window is now stale in its last clause. A lane may not edit another
milestone's block, so it is named here and in this lane's report for the integrator to strike.

### The gate's first red run, and the line that made it readable

**CI found a wiring bug this lane had already been told about and had dismissed**, which is worth
recording at more length than the fix deserves, because the dismissal is the reusable part.

`cargo xtask test`'s riscv64 leg exports `NIFE_INITRD` pointing at the riscv64 archive, explicitly,
with a comment saying it has to; `cargo()` exports the **aarch64** one, because it is the aarch64
path's helper. `screen_boot` went through `cargo()` and never overrode it, so the riscv64 leg handed
a riscv kernel an aarch64 archive.

**That bug had two faces and only the harmless one was visible locally.** With the aarch64 archive on
disk (which it always is, after any other leg has run) the boot carried on and printed
`MEASURED BOOT REFUSED: 'progenitor' is not what this kernel image was built against`. The lane read
that as noise, because the gate's own assertion still passed: the tour was on the screen, which is
what the gate checks. In the `cpu matrix (riscv64)` job, which builds no aarch64 archive, the second
face appeared and QEMU would not start at all: `could not load ramdisk`. Five models, one cause, none
of it about a CPU model.

**The lesson is the one this tree keeps relearning**: a line the machine printed outranks a gate that
went green, and a refusal message inside a passing run is still a refusal. `MEASURED BOOT REFUSED`
was telling the truth an hour before CI repeated it more loudly.

**What the fix is:** the archive is chosen per architecture at the spawn, the way every other riscv64
caller in `xtask` already does it. Verified by reproducing CI's condition rather than by reasoning,
with `target/initrd.img` moved out of the tree: `cargo xtask screen-boot riscv64` then exits 0 and
reads 45 rows off the screen, and the tour reaches further than before, because the progenitor now
composes userspace instead of refusing the archive.

**And the screen legs no longer run under `--cpu`**, which is a cost decision made on its own merits
rather than a way around the failure (the failure reproduces on `rv64` too, and is fixed). The matrix
runs this suite five times to narrow the **ISA**; nothing on the screen path varies with `-cpu`. The
`fw_cfg` conversation is byte moves and MMIO stores and `screen_console` is integer arithmetic and
byte stores, and every instruction either one uses has already been executed on that same model by
the three hundred tests above it. Five more emulated boots would buy a claim that cannot differ
between models. The leg still runs on every ordinary `script/test`, `--arch riscv64` included.

**The gate's own second diagnostic line is why this was quick to read**, and it is the shape worth
copying into other gates:

```text
screen-boot: nothing decodable was ever on the screen (did QEMU get a ramfb and a monitor?)
screen-boot: the tour never reached the SERIAL line either, so this is a boot failure and not a screen one
```

The second line is a **control**, not a detail. A screen gate that only says "the screen was blank"
sends the next reader into the framebuffer path, which is where this lane would have gone and where
nothing was wrong. Checking the serial transcript for the same marker costs one `contains` and
separates "this milestone's mechanism is broken" from "the machine did not boot", which are different
people's afternoons. Any gate that reads a machine through one channel should assert the same fact
through the channel it is replacing, and say which one failed.

## BUGS

- **The mechanism is one idea and two discoveries.** A linear framebuffer painted by
  `screen_console` is what every architecture ends at; how one is *found* is per-machine, and there
  are two answers rather than one (a UEFI aperture, a `ramfb` the guest supplies) with a third owed
  by milestone 157.
- **Nothing here has run on real silicon except the UEFI half, and that half shows a grid rather
  than text on the one machine that has run it** (`screen_console`'s BUGS, xenon, 2026-09-04 and
  2026-09-17). The `ramfb` half is QEMU-only by construction: no board has the device.
- **A `ramfb` screen costs 1.9 MB of `.bss` on every board boot**, including the VisionFive 2 boots
  where the device cannot exist. `kernel/src/screen.rs`'s BUGS has the reason and the exit.
- **The loader's handoff banner is unobservable in a healthy boot**, so no gate asserts it: the
  kernel clears within milliseconds. What is gated is that it compiles and that `uefi-boot` stays
  green; what proves it is the halted-loader experiment above. Rung four, honestly.
- **It does not cover the interleaving defect** milestone 230 found, where the kernel and the console
  server both drive one UART with nothing arbitrating. That is a live bug on the machines that *do*
  have a serial port and it has its own home.
- **A machine that can only report over a network is a machine whose failures you cannot see when
  the network is the failure**, and no answer here escapes that entirely.

## Index row

the boot tour is on the screen on all three architectures, read back off the framebuffer by a gate
that decodes the glyphs: a UEFI aperture on x86_64 and a `ramfb` the guest supplies on the two
boards. The window before the kernel exists is bounded by a line the loader paints and cannot be
narrated further; a gate reading a real serial-less machine is milestone 369
