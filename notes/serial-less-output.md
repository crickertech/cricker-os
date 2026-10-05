# A machine with no serial port

Milestone 243 (a machine with no serial port). Every word nife had ever said, it said down a UART: the boot tour on all three
architectures, the console server and the shell, the kernel's fault reports, and every automated
gate that reads any of them (`script/board-console`, the soak's heartbeat, `script/swish-check`,
`crates/board_console`'s stage judging).

A commodity machine does not have one. xenon does, and that was chosen rather than lucky: milestone
87 picked a Dell with a C4PDJ serial module and a null modem. Meanwhile the same milestone made a
much larger fleet reachable without anyone noticing, because `\EFI\BOOT\BOOTX64.EFI` on a FAT32
stick is the removable-media fallback every UEFI firmware looks for with no configuration:

> if I can boot a nife system off of a USB drive, then that opens up a lot of different hardware in
> our house that I can use for testing: Graeme's laptop, his desktop, cordoba, two MacBooks, Clay's
> desktop
>
> -- calef, 2026-09-03

Not one of those machines has a serial port. This note is what was chosen, what it cost, what it
does not solve, and the procedure for the bench nobody in this lane could reach.

## The two halves, and they are not equally hard

The block separates them and the separation survived the work, so it is the first thing to say.

|  | a **human** watching a machine boot | a **gate** reading a machine |
|---|---|---|
| needs | pixels on a monitor, live | text, after the fact, judged by a program |
| has today | the framebuffer console below | a screendump under QEMU, and **nothing on real hardware** |
| fails when | the screen is not yet initialised | the machine is not a virtual one |

One mechanism serves both under QEMU and only one of them on a real machine, and pretending
otherwise would be the dishonest half of this note. Under OVMF a gate can ask the emulator for a
picture of the screen; on Graeme's laptop nobody can. That gap is real, it is named in `BUGS` below,
and the follow-on proposals are where it goes.

## What was chosen: the firmware's framebuffer, carried across the handoff

UEFI's `EFI_GRAPHICS_OUTPUT_PROTOCOL` reports a *linear framebuffer*, and its address survives
`ExitBootServices`. That is the fact the whole design rests on and it is worth being precise about
why: the aperture is a BAR on the display adapter, not firmware memory, so ending the boot phase
takes away the firmware's *console* and not the *display*. The pixels stay where they were and
anyone holding the address can keep writing to them.

So:

```text
  firmware ──GOP──► uefi_loader ──"screen=0x80000000,1280,800,5120,bgrx"──► kernel ──glyphs──► monitor
                         │                  in hvm_start_info's                 │
                         │                  cmdline_paddr                       │
                    machine_discovery::framebuffer  (one crate, both parties)   screen_console
```

Four pieces, and the split is the tree's usual one: what two programs agree on is a crate, the logic
is on the host, and only the address arithmetic is in the kernel.

| Piece | Where | What it is |
|---|---|---|
| the sentence | `crates/machine_discovery/src/framebuffer.rs` | `Framebuffer` and its one command-line token, **written** by the loader and **parsed** by the kernel, round-tripped in host tests |
| the question | `uefi_loader::find_screen` | one `LocateProtocol` call and three field reads |
| the painter | `crates/screen_console` | a cursor over a `&mut [u8]`, with `bitmap_font`'s glyphs |
| the seam | `kernel::console`, `arch::x86_64::machine::attach_screen` | tee `print!` to the screen; record the aperture so `mmu::init` carries the mapping |
| the gate | `board_console::screen`, `cargo xtask uefi-boot` | a screendump, decoded back into text, judged by `board_console::progress` |

### Why it rides a command line and not a new structure field

The x86_64 handoff is PVH's `hvm_start_info`, and PVH already carries a command line:
`cmdline_paddr` at offset 24, which `machine_discovery::x86_64::BootInfo` has decoded since milestone
87 and which nothing had ever read. `uefi_loader`'s own `BUGS` called that a gap in as many words:
*"there is nowhere yet for a boot argument to come from or go to."*

The alternative was appending a field to `hvm_start_info`. That structure is Xen's, versioned by
Xen, and a field added below the last one is a fork of somebody else's layout that looks exactly like
the real thing to whoever reads it next. A `key=value` in the field the format already provides for
exactly this is smaller, more reversible, and what Linux does with `video=`.

Is it a wire format, and therefore calef's? The tenets say anything two programs agree on is
expensive. This one is unusually cheap and the reason is structural rather than an argument for
leniency: `uefi_loader` embeds the kernel inside itself (`uefi_loader/build.rs`), so the writer
and the reader ship as one file and are rebuilt together. There is no version of this system in which
one side has the new spelling and the other does not. The token is recorded here and in the crate,
and renaming it costs one commit.

### Why the console is not `video_terminal`

`video_terminal::Vt` is the real terminal and it is the right engine for milestone 177's interactive
one. It is deliberately not this, for three reasons that all point the same way:

- It is a value of several hundred kilobytes (its own documentation warns readers off putting one
  on a stack), which in a kernel means a `.bss` static of that size on every architecture for a
  diagnostic path.
- It would put an escape-sequence parser in the TCB, over bytes, when the kernel's own `println!`
  emits no escape sequences to parse.
- The thing being reported is often the reason the machine is broken. That is the block's own
  constraint, and it argues for the console with the least state that could work. `ScreenConsole`
  holds a cursor and a geometry: five `u32`s and no buffer.

What *is* shared is the font, so the letters on an early boot screen and the letters in the
graphical terminal are the same letters, which is what makes the gate below possible.

## What the alternatives cost, priced rather than argued

The block listed candidates and endorsed none. Each was priced against both halves.

| Candidate | The human | The gate | Verdict |
|---|---|---|---|
| **Firmware framebuffer** (chosen) | yes, from the first tour line | under QEMU only | the only one that answers the human half at all, and the only one whose address survives `ExitBootServices` with no driver of ours |
| **Keep `ConOut` alive longer** | until `ExitBootServices` and no further | no | not a candidate, it is a fact: boot services *are* the console, and a kernel that never exits them never gets its memory map. The loader already prints there and this milestone kept that |
| **Postmortem to the boot medium** | no (it is postmortem) | **yes, and on real hardware** | the gate half's real answer, and it needs a **USB mass-storage driver**, which nife does not have. Milestone 242 is USB host. Deferred as a proposal, not refused |
| **A network console** | no | yes, and unattended | needs a NIC driver per board and says nothing until the stack is up, which excludes every failure before it. The block's own objection stands: a machine that can only report over a network cannot report a network failure |
| **A photograph of the screen** | it *is* the human | no | not pixel-aligned and not a screendump; `board_console::screen` cannot read one, and saying so is cheaper than discovering it |

The honest summary is that the two halves want different mechanisms, which the block anticipated
and this lane confirms. The framebuffer is the human's answer and is the right first increment
because it is the one a person needs standing in front of a machine that will not boot. The gate's
answer on real hardware is postmortem to storage, and it is blocked on a driver.

## The gate: a program reading a screen

`board_console::screen` turns a screendump back into text. It is not optical character recognition
and the difference is the whole reason it is trustworthy: `bitmap_font` is a constant table of
monochrome 7x8 glyphs, so a cell either matches a glyph bit for bit or matches nothing. There is no
threshold to tune and no confidence to report.

That gives `cargo xtask uefi-boot` an assertion nothing else in this tree could make. The serial
transcript it already checked would read identically if the screen were black; the screen check fails
if the loader's `LocateProtocol` regressed, if the pixel order flipped, if the stride were taken as
the width, or if `mmu::init` stopped carrying the aperture's mapping.

Measured, 2026-09-04, OVMF on QEMU 11.x, `-display none`:

```console
$ cargo xtask uefi-boot
nife uefi_loader: milestone 87
uefi_loader: screen at 0x0000000080000000..0x00000000803e8000
uefi_loader:   1280x800, stride 5120, bgrx
uefi_loader: kernel placed, exiting boot services

nife on x86_64 (long mode, ring 0, 4-level paging)
  screen      : 1280x800 bgrx at 0x80000000, 182x100 cells (boot cmdline)
  ...
nife x86_64: boot complete, halting.
uefi-boot: read 96 non-blank row(s) of text back off the framebuffer, ending
uefi-boot:   |   next        : real ELF user programs (user_mode_runtime has no x86_64 arms), ...
uefi-boot:   | nife x86_64: boot complete, halting.
uefi-boot:   |   capability slots: 2 of 24 at peak
uefi-boot: booted under OVMF from \EFI\BOOT\BOOTX64.EFI
```

`-display none` suppresses the host window, not the emulated adapter, which is why any of this
works headlessly: OVMF finds a GOP here for the same reason a real machine's firmware finds one.

## The shell on the screen, too

Everything above is the kernel's voice. Since milestone 299 (x86) the console is a userspace process
writing COM1, so until milestone 400 (the shell on the firmware's screen) the tour reached the screen and the shell's prompt did not. Now
it does, beside the serial console rather than instead of it:

```text
  swish ─► line_editor ─► console ──out──► COM1
                             └──OPERATION_WRITE──► display_terminal ──FLUSH──► framebuffer_driver ──copy──► the aperture
```

- `framebuffer_driver` serves the same framebuffer contract `gpu_driver` does, over the screen
  the firmware left running: a flush is a CPU copy from the surface into the aperture
  (`screen_console::Aperture`, host-tested). It holds only the aperture rows its 924x344 surface can
  reach, as a spawn-time mapping.
- The console server tees: every byte it writes to COM1 it then hands `display_terminal`, so both
  surfaces say the same thing and every serial gate is unchanged.
- The kernel hands the screen over once (`console::yield_screen`): it clears it under the console
  lock, stops painting, and only then is the driver spawned. After that the kernel's own lines reach
  the UART only, except a panic, which takes the screen back to be read.

The gate is `cargo xtask uefi-boot`'s second and third stages: the prompt on the screen, and the
answer to `echo typed on the wire` typed on the serial line. The block has the decisions, what lost,
and xenon's bench step:
[400-the-shell-on-the-firmware-screen.md](../design/roadmap/400-the-shell-on-the-firmware-screen.md).

## The boards' screen: `ramfb`, and why it is the other way round

Built 2026-09-19, closing the second of milestone 243's two outstanding items.

`x86_64` gets a screen because UEFI already lit one and the loader only has to measure it and say
where it is. QEMU's `virt` boards have no such stage: they are entered from `-kernel` with nothing
configured, so there is no framebuffer to discover and the arch-neutral halves of this path sat
unused on two of three architectures.

What `virt` can present is `ramfb`, and it inverts the arrangement: the guest owns the pixels and
hands the device their physical address, after which QEMU scans them out continuously. So there is no
flush, no doorbell and no interrupt between a `println!` and the picture, which is exactly the
property a console wants and exactly what a display *device* would not give.

The pieces, and note that only the first two are new:

| Piece | Where |
|---|---|
| The `fw_cfg` wire format, host-tested | `crates/firmware_configuration` (**name provisional**): every field big-endian, encoders that take native values and return bytes |
| The register poking | `kernel/src/drivers/ramfb.rs`: two DMA transactions, before `arch::mmu::init`, through the coarse boot map |
| The memory and the wiring | `kernel/src/screen.rs` (**name provisional**): 800x600 of `.bss`, the device-tree lookup, and the same `console::attach_screen` call `x86_64` makes |
| Painting text | `crates/screen_console`, unchanged |
| The gate | `cargo xtask screen-boot <arch>` (**name provisional**), `uefi-boot`'s twin, decoding with `board_console::screen` unchanged |

Measured 2026-09-19, `cargo xtask screen-boot`, inside `script/test`'s two board legs:

```text
--- the boot tour on a screen, aarch64 (QEMU virt + ramfb) ---
screen-boot: read 33 non-blank row(s) of the aarch64 tour back off a ramfb, ending
screen-boot:   |   self-test       : scheduler  ok  thread 5 ran and carried its captured state
screen-boot:   | nife self-test: 5 of 5 passed

--- the boot tour on a screen, riscv64 (QEMU virt + ramfb) ---
screen-boot: read 63 non-blank row(s) of the riscv64 tour back off a ramfb, ending
```

Why it is a boot of its own rather than a stage of the suite, which is forced rather than
chosen: `ramfb` adds a QEMU *console*, `screendump` with no device argument writes console 0, and
the suite's machine already has a virtio-gpu there. A boot carrying both would be photographing
whichever QEMU happened to order first, and the gate would mean something different depending on
QEMU's version.

Two things the first red CI run settled, both worth knowing before touching this gate.

The archive is chosen per architecture at the spawn. `cargo()` exports `NIFE_INITRD` pointing at
the aarch64 archive, and every riscv64 caller in `xtask` has to override it. `screen_boot` did not,
at first, which on a machine with the aarch64 archive already on disk produced
`MEASURED BOOT REFUSED` inside an otherwise passing run, and in a CI job that never built one
produced `could not load ramdisk` and no boot at all. A refusal message inside a green gate is still
a refusal.

The screen legs do not run under `--cpu`. `script/cpu-matrix` runs the riscv64 suite five times
to narrow the ISA, and nothing on this path varies with `-cpu`: byte moves, MMIO stores and integer
arithmetic, all of it already executed on that model by the suite above. The leg runs on every
ordinary `script/test`, `--arch riscv64` included.

The gate says which channel failed, and that is the part worth copying. Its second diagnostic
line checks the *serial* transcript for the same marker it could not find on the screen:

```text
screen-boot: nothing decodable was ever on the screen (did QEMU get a ramfb and a monitor?)
screen-boot: the tour never reached the SERIAL line either, so this is a boot failure and not a screen one
```

Without that line a blank screen sends the next reader into the framebuffer path. It costs one
`contains` and separates "this mechanism is broken" from "the machine did not boot".

What it does not claim. `ramfb` is QEMU's; no real board has one. It proves the arch-neutral
console and the arch-neutral discovery *type* on all three architectures, and it proves nothing about
the DC8200 on the VisionFive 2. That is milestone 157 (real display output on the board), and the shape of the change it needs is
one
branch above `screen::attach` and nothing below it.

## The bench: booting a serial-less machine

This has not been done. Everything above is QEMU with real firmware in the loop, which is as far
as a lane can get; the machines are calef's. This section is the procedure, written to be followed
rather than interpreted, and it is deliberately the same shape as `notes/x86-uefi-boot.md`'s.

### What you need

- Any x86_64 machine with UEFI firmware and a monitor. From the fleet calef named: Graeme's laptop,
  Graeme's desktop, cordoba, Clay's desktop, or an Intel MacBook. The Apple Silicon MacBooks are
  not in this fleet: they boot non-Apple kernels legitimately (Asahi's permissive-security mode)
  but they have no UEFI, so a `BOOTX64.EFI` stick will not start one. That is a port, not a boot.
- A USB stick, formatted FAT32 with a GPT or MBR partition table. macOS Disk Utility: *Erase*,
  format MS-DOS (FAT), scheme GUID Partition Map.
- No serial cable, no adapter, nothing else. That is the milestone.

### Build and copy

```console
$ cd /path/to/nife
$ cargo xtask uefi-image
wrote .../target/esp/EFI/BOOT/BOOTX64.EFI (the loader, the kernel and the archive)

$ mkdir -p /Volumes/NIFE/EFI/BOOT
$ cp target/esp/EFI/BOOT/BOOTX64.EFI /Volumes/NIFE/EFI/BOOT/BOOTX64.EFI
$ diskutil eject /Volumes/NIFE
```

One file. The path and the capitalisation are the interface.

### Firmware settings

1. Secure Boot: off. This image is unsigned and nothing in this tree signs it. Expect to have to
   do this; a Secure Boot machine refuses the stick with a security-violation message and no other
   explanation.
2. Boot from UEFI, not Legacy/CSM. Legacy boot looks for an MBR boot sector, which this stick
   does not have.
3. Leave everything else alone on the first attempt. A bring-up has enough variables.

The one-time boot menu is usually F12; on a Mac, hold Option at the chime.

### What you should see, in order

Everything is on the monitor. Nothing else is connected.

1. `nife uefi_loader: milestone 87 (x86_64 bare-metal)`, then three or four more `uefi_loader:` lines. This is the
   firmware's own console, so seeing it proves the firmware found the stick, Secure Boot did not
   refuse it, and the loader started. It prints the screen it found.
2. The screen clears, which is the kernel's console arming.
3. The boot tour, beginning `nife on x86_64 (long mode, ring 0, 4-level paging)`, with a
   `screen      :` line naming the geometry.
4. The screen clears a second time and the shell's banner and `$ ` fill it at the tour's
   scale (137x67 on 1920x1080). There is no keyboard yet on a machine without a
   serial port (milestone 242), so the prompt is as far as it goes.

**Photograph the screen at that point.** That is the record, and it is the only record this machine
can produce today (see `BUGS`).

### Triage

Each row rules out everything above it.

| What you see | What it means | What to do |
|---|---|---|
| Nothing at all, machine boots its own OS | the firmware did not see the stick | check `/EFI/BOOT/BOOTX64.EFI`, the case, and FAT32 rather than exFAT |
| "Security violation", or the stick is skipped | Secure Boot | turn it off |
| `nife uefi_loader:` then a message and a halt | the loader refused, and it says why | every string is a literal in `uefi_loader/src/main.rs` |
| `uefi_loader: no linear framebuffer` | the adapter is `PixelBltOnly` or `PixelBitMask` | this machine cannot use this milestone; record the model, it is the first one |
| `wanted 0x...` and `in the way:` lines | the firmware will not give up the kernel's 32 MiB load range | milestone 195's `BUGS`: the image is not physically relocatable. Record the descriptors printed |
| The screen clears and shows `nife loader: firmware released, entering the kernel.` and nothing more | the kernel never reached its first statement: the trampoline, `boot.s`, the page tables or the long-mode jump | build with no archive (`NIFE_UEFI_INITRD` unset); a triple fault here reboots instead, so a *stuck* banner is a hang rather than a fault |
| The loader's banner clears and then **nothing** | the kernel armed its console and died after | the window below `attach_screen`; on a machine with a serial port the transcript is the diagnosis |
| Text, but sheared or in the wrong colours | the stride or the pixel order | the `screen :` line says what the loader read; compare against the machine's real mode |
| The machine reboots in a loop | a triple fault | build with no archive (`NIFE_UEFI_INITRD` unset) to halve what is copied |
| The tour, the second clear, then **nothing** | the userspace terminal took the screen and drew nothing | milestone 400's defect; on a machine with a serial port its two `screen    :` lines say whether the driver came up |

## The window between the firmware and the kernel, and what can be bought in it

This is problem 3 of the block's three, early boot, and the first thing to say about it is that it
**cannot be narrated**. From `ExitBootServices` to the kernel's own `attach_screen` there is no
console: the firmware's is gone by specification, the kernel's is not up, and the code in between
(the loader's mode-switch trampoline, `boot.s`'s 32-bit half, the page tables, the long-mode jump) is
a 32-bit instruction stream with no idea where the screen is and no IDT, so a fault there is a triple
fault and a silent reset. Nothing a person could write would make that window speak.

**It can be bounded**, and since 2026-09-19 it is. `uefi_loader` clears the screen and writes two
lines into the framebuffer as its final act, after `ExitBootServices` and before the jump:

```text
nife loader: firmware released, entering the kernel.
If this line is still here, the kernel stopped before its console came up.
```

The screen is therefore never blank during that window, and the five things a person at a monitor can
be looking at are now distinguishable rather than four of them being "black":

| The screen shows | What ran, and what did not |
|---|---|
| the firmware's own splash or menu, untouched | the firmware never started this loader |
| `uefi_loader:` lines and a halt, over the firmware's console | the loader started and refused, and said why |
| **the two lines above, still there** | the kernel never reached its first statement |
| black | the kernel armed its console and died after |
| the boot tour | the window is behind us |

The third row is the one that did not exist before, and it is the whole of what this buys. It was
proved rather than reasoned: with a temporary halt in place of the jump, `cargo xtask uefi-boot`'s
screendump has ink in exactly the top sixteen pixel rows of a 1280x800 screen and they read those two
lines.

**It is painted after `ExitBootServices` on purpose.** Before that call the screen is the firmware's
console, and writing the aperture underneath it would race the firmware's own scrolling. The aperture
survives the call, which is this whole milestone's founding observation: what ends is the firmware's
*console*, not the *display*.

**The two boards get nothing equivalent, and the reason is the boot chain rather than effort.** Under
QEMU they are entered from `-kernel` with no stage before the kernel at all, so there is nobody to
paint a banner; and the kernel's own `ramfb` framebuffer is in `.bss`, which `boot.s` has not finished
zeroing at the point this would have to happen. On the VisionFive 2 the stage that could say something
is U-Boot, and that is milestone 157's `simple-framebuffer` handoff. The board window is therefore
`boot.s` plus one `fw_cfg` conversation, which is a few hundred instructions.

## BUGS

- **Nothing here has run on real silicon.** It is proved on the host and under OVMF. A framebuffer
  that works under QEMU's emulated adapter is not a framebuffer that works on Graeme's laptop: the
  aperture may be above 4 GiB (this loader refuses that, see below), the mode may be `PixelBitMask`,
  and the firmware may hand over a mode the monitor is not showing.
- **A gate cannot read a real machine.** The screendump path is QEMU's. On the fleet, the record is a
  photograph and a person, which is exactly the state milestone 216 got the VisionFive 2 *out* of.
  Postmortem to the boot medium is the answer and it needs a USB mass-storage driver; see the
  proposals.
- **Early boot is bounded, not narrated.** The loader's handoff banner (above) makes "the kernel
  never started" a distinguishable outcome; it does not make the window *say* anything about what
  went wrong there, and nothing can, for the reasons in that section. A fault in `boot.s` is still a
  triple fault and a reset, and what a person sees afterwards is the firmware starting over.
- **The banner is unobservable in a healthy boot**, which makes it a diagnostic nothing routinely
  exercises. `attach_screen` clears within milliseconds of it being painted, so no gate can assert it
  on a working machine; what is gated is that the loader still compiles it and that `uefi-boot` is
  green, and what proves it is the halted-loader experiment recorded above. Rung four, honestly.
- **The framebuffer must be below 4 GiB**, because everything the loader hands the kernel has to be
  nameable by a 32-bit instruction stream running with paging off. `uefi_loader` does not currently
  check that for the framebuffer specifically; the aperture is a BAR the firmware placed, and a
  machine that puts it high would be handing the kernel an address its early direct map does not
  cover. Nothing has been seen doing this and nothing would notice if one did.
- **The aperture is mapped uncacheable**, like every other device window, so writing a screenful is
  slow on real silicon in a way it is not under QEMU. Write-combining is a PAT entry and this kernel
  does not program the PAT at all; a framebuffer is the first thing in it that would care.
  Scrolling additionally *reads* the aperture back, which is the worse half. The boot tour is shorter
  than a 1280x800 screen is tall, so this does not bite during the boot it was built for.
- **Only ASCII appears.** A byte outside `0x20..0x7f` is drawn as a space, so `§121` in the tour's
  last line reads as two blanks and a number on the screen. The serial console shows it correctly and
  the two transcripts therefore differ by exactly the non-ASCII characters in them.
- **`board_console::screen` duplicates a decoder `xtask` already has.** Milestone 177's graphical
  `swish-check` leg carries `parse_ppm`, `decode_cell` and `scanout_rows` in `xtask/src/scanout.rs`,
  hardcoded to the compositor's geometry and the terminal's default colours. The two should be one
  crate; unifying them touches another milestone's gate and is a proposal rather than a drive-by.
- **The other two architectures have a screen under the emulator and none on silicon.** The
  arch-neutral halves (`machine_discovery::framebuffer`, `screen_console`) were written for exactly
  this and needed no change: what was missing was the discovery, and on QEMU's `virt` boards there is
  nothing to discover, because nothing lit a display. `ramfb` is what those boards can present
  instead, and it inverts the arrangement (see below). On the VisionFive 2 there is no `ramfb` and
  the answer is milestone 157's U-Boot `simple-framebuffer` handoff. Rule 5's scope note, recorded
  here: the *console* has parity, the *discovery* does not, and the gap is one board's firmware.
- **A `ramfb` screen costs 1.9 MB of `.bss` on every board boot**, including the VisionFive 2 boots
  where the device cannot exist and the region is never written. `kernel/src/screen.rs`'s own BUGS
  has the reason (the frame allocator does not exist yet and cannot serve 469 contiguous pages
  anyway) and the exit (157's handoff needs no buffer at all).
- **A `ramfb` screen cannot be handed to a userspace terminal.** Milestone 400's handover maps the
  screen's physical range into a driver, which is right for a display adapter's BAR and is a hole
  for a framebuffer that is the kernel's own `.bss`. `user::boot_screen_terminal` refuses a screen
  inside the kernel image, so on the two boards the shell's prompt is on the serial line and the
  kernel's tour is what stays on the screen. Milestone 157's aperture will pass that test without
  anything being changed.
- **`ramfb` is QEMU's and needs no cache maintenance**, which is the one shortcut in
  `kernel/src/drivers/ramfb.rs` that would be wrong on silicon. The device reads guest RAM while the
  guest has it mapped cacheable; an emulator has no cache to be stale. Nothing else in this tree may
  copy that pattern.
