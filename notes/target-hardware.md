# Where nife could actually run

> Recast (2026-07-27): milestone 16 is now RISC-V-first (design/roadmap/0016-real-hardware-iommu.md): first
> silicon is a VisionFive 2-class board, whose OpenSBI/NS16550/PLIC/Sv39 contract the kernel
> already speaks exactly, and the IOMMU work targets QEMU's emulation of the ratified RISC-V
> IOMMU before any silicon. This note's Pi-first analysis predates the riscv port reaching
> parity and stands as the aarch64-side record; its "the ISA is almost never the constraint"
> thesis is exactly why the cheaper-contract board wins.

## The three machines have names: argon, radon, xenon

Ratified by calef, 2026-09-01, once all three existed and "the board" had stopped being an
unambiguous phrase in a conversation. One noble gas per architecture:

| name | architecture | machine | state, 2026-09-01 |
|---|---|---|---|
| **argon** | aarch64 | Jetson TX1, milestone 127 (the seL4 machine) | awaited (2026-10-06) |
| **radon** | riscv64 | StarFive VisionFive 2, JH7110 | boots nife, wired as a bench target |
| **xenon** | x86_64 | Dell OptiPlex, serial port and null modem (milestone 87) | **boots nife**, first light 2026-09-05 (the cell said "no first light yet" until 2026-09-05) |

Why they earn names rather than descriptions. This project's own tenet is that a name is a
claim and a reader meets it before anything else. "The board" was serviceable while there was
one; with three it silently means whichever the speaker had in mind, and the cost lands on
whoever reads the sentence later. The names are also stable in a way the descriptions are not:
the aarch64 board may not stay a Jetson, but argon stays argon.

They are deliberately not architecture names. `riscv64` already names the ISA, and a machine
name has to survive the day a second machine of the same architecture arrives.

The development machines keep their existing names and are not part of this scheme:
**patagonia** (the Mac everything is built on) and **cordoba** (the always-on x86 box).

## Item 0's alternative: isolation without an MMU, and what each version costs

Recorded 2026-09-04, because item 0 as written reads as though nobody had considered it, and a reader
who knows Tock or seL4's MPU variant will assume we had not. None of these is being taken, and
the reason in each case is a cost rather than an oversight.

An MPU instead of an MMU. Cortex-M and Cortex-R carry Memory Protection Units: region-based
permissions, no address translation, typically eight to sixteen regions. seL4 has an MPU variant
and Tock OS pairs an MPU with Rust's type system, so this is proven rather than theoretical. It is
also the only one of the three that preserves this project's thesis, because it isolates compiled
binaries rather than requiring the system to have compiled them.

What it costs: no virtual addresses, so every program is position-independent or linked per
deployment; a hard ceiling of eight-ish simultaneously separated things, where the capability model
assumes as many address spaces as there are processes; and no demand paging, no copy-on-write, no
shared mappings. It is a port rather than a configuration, and most of `kernel/src/arch/*/mmu.rs`
and the `paging` crate would have no counterpart.

**Software fault isolation** (Native Client, WebAssembly): bounds-check every memory access, enforced
by the compiler.

**Language-based isolation** (Microsoft Research's Singularity): software-isolated processes sharing
one address space, safe because every binary is verified type-safe before it runs.

Those last two are foreclosed by DECISIONS §14 rather than by taste. Both isolate only code the
system itself compiled or verified. §14's demonstrator claim is a kernel that runs real workloads
which were not written for it, and design/fatal-risks/README.md's first risk is exactly *"only software
written for nife runs on nife"*, measured on 2026-08-31 with an unmodified `ripgrep`. An SFI or
language-based nife would make that risk permanently red by construction, which is a stranger thing
to ship than a kernel that needs an MMU.

**Why this is a note and not a milestone.** There is no machine. The form factors an MPU port would
open are the ones where no open MMU-class device exists today, and a proposal nobody can act on is
the backlog graveyard milestone 247 exists to avoid. What would change that is a specific device,
and at that point this section is the starting analysis rather than a blank page.

## The ISA is almost never the constraint

"Does it run aarch64" is the wrong question. And the answer splits in two, which this note ran
together until 2026-09-04 (calef: *"nife should be able to run on hardware without a serial
console"*). Some of these are properties a machine must have to run nife at all; one is a property
*we* need to bring nife up on it, and confusing them makes the supported set look smaller than it is.

To run nife, and item 0 was missing from this list until 2026-09-04 although it excludes more
than the rest put together:

0. Is there an MMU? nife is a capability microkernel with per-process address spaces; every
   driver and server is an EL0 process behind its own page tables. There is no configuration in
   which it runs without one. This is what excludes the whole microcontroller class (Cortex-M,
   Cortex-R, RISC-V E-series). The alternatives exist, are proven elsewhere, and are described in
   their own section below; none is free and one costs the thing fatal risk 1 measures.
1. Can you get code to execute at boot? Unlocked bootloader, or no secure boot at all.
2. Are the peripherals documented? You need an interrupt controller, a timer, and eventually
   storage. The CPU is standardized. The stuff bolted around it is not, and that's where the work is.
3. At least 32 KB of L1 instruction cache, for the reason in the next section. A requirement on
   the claims rather than on the boot.

**To bring nife up on it, which is ours and not the machine's:**

4. Can you physically reach a serial console? Without one you are debugging a black box.

Today these are the same list, and that is a defect this project owns rather than a fact about
hardware. Every word nife has ever said went down a UART: the boot tour on all three machines, the
console server and the shell, kernel fault reports, and every automated gate that reads any of them.
So a machine with no serial port cannot currently *tell us* it is working, which is not the same as
being unable to work. Milestone 243 (a machine with no serial port has no way to say anything, and
no gate can read it) is the milestone that separates them, and until it lands, requirement 4 is
doing the job of requirements 1 to 3 by proxy.

**The reason it matters is not tidiness.** calef's fleet argument on milestone 241 names Graeme's
laptop and desktop, an Intel MacBook, cordoba and Clay's desktop as machines a USB stick could boot
(milestone 87 boots from `\EFI\BOOT\BOOTX64.EFI`, the removable-media fallback every UEFI firmware
looks for). **Not one of them has a serial port.** Read as a running requirement, item 4 excludes the
entire fleet; read correctly, it says only that we cannot yet watch them.

A device can be aarch64 and still be completely useless to us by failing any of items 0 to 3, and
useless *for development* by failing item 4.

**Which item actually does the excluding, because the order is not what it looks like.** Written out,
this list reads as though an unlocked bootloader were the wall. It is not:

- Item 0 excludes the most, and silently, because a microcontroller never appears in a
  conversation about operating systems in the first place.
- Item 2 is the real filter for everything else. Phones with unlockable bootloaders are a genuine
  class rather than an exception (Pixel, Fairphone, Sony's open-device program, and the several
  hundred devices postmarketOS supports), so item 1 is often satisfiable. What defeats a phone is
  that its SoC is undocumented: no public reference for the interrupt controller, clocks, power
  domains or display, a downstream device tree describing what the vendor's kernel happens to do
  rather than what the hardware is, and firmware the application processor must cooperate with. That
  is years per device, which is why postmarketOS is a years-per-device project.
- Item 3 excludes almost nothing that passes item 0. Cortex-A53 and every x86 since about 2006
  sit at 32 KB, phone big cores and server cores at 64 KB. The cache floor is a real requirement and
  a nearly free one.

And a note for a future reader, from calef, 2026-09-04: *"These things change over time. I just
want to ensure that because we cannot today we don't make decisions that block it in the future."*
Nothing on this list is a decision this project made; items 0 to 3 are properties of machines. The
place where a decision of ours could foreclose a small target is the kernel model, and
DECISIONS §96's memory input is recorded there as closed at this project's scale rather than
absolutely, for exactly this reason.

**Requirement 3 is a *development* requirement, not a running one** (milestone 243, 2026-09-04), and
the distinction was invisible for as long as nife had exactly one way to say anything. A system that
boots, runs and serves needs no serial port; a system somebody is *bringing up* needs a way to watch
it fail. The two lists looked identical because every word nife had ever said went down a UART.

They do not any more, on one architecture. `notes/serial-less-output.md` records what changed: on a
UEFI machine the loader asks the firmware where the linear framebuffer is and the kernel paints the
boot tour into it, so requirement 3 is met by a monitor on those machines rather than by a cable.
What that does *not* yet do is meet the half of requirement 3 an unattended gate needs, which still
wants a serial line or a postmortem log. So: a screen is enough to bring a machine up by hand, and
not yet enough to put one in a test loop.

## A fourth requirement, and this one filters silicon rather than firmware

At least 32 KB of L1 instruction cache, and it is a requirement on the *claims* rather than on
the boot. nife will start on less; what will not survive is the reason a microkernel is supposed to
be fast.

Where the number comes from. Liedtke's *On micro-Kernel Construction* (SOSP 1995) argued Mach's
IPC was slow because of the cache footprint of its hot path rather than anything inherent to
microkernels: a kernel touching a lot of memory per IPC evicts the *application's* working set, so
the cost appears as capacity misses spread through the workload instead of as time in the kernel.
`script/fastpath-footprint` is the gate that keeps this honest and notes/benchmarks.md carries the
argument.

Measured 2026-09-04, the fastpath's upper bound per architecture, against the L1i of the machine
this project runs that ISA on:

| target | fastpath | machine | L1i | fastpath as a share |
|---|---|---|---|---|
| x86_64 | **8,404 B** | xenon, Core i5-7500T | 32 KB* | **26%** |
| aarch64 | 9,156 B | argon, Cortex-A57 | 48 KB | 19% |
| riscv64 | 7,174 B | radon, SiFive U74 | 32 KB | 22% |

At 32 KB the hot path takes about a quarter of the cache and leaves the application three quarters,
which is the regime Liedtke's argument assumes. At 16 KB it would take half, and the thing the
gate exists to protect stops being true. That is the filter: 16 KB L1i is where nife stops being
able to claim what it claims, and 32 KB is the floor at which the claim is comfortable.

What this rules in and out. Every frontier core clears it easily: notes/benchmarks.md's survey
puts Zen 5 at 32 KB, Intel's Lion Cove and Arm's Cortex-X925 and SiFive's P870 at 64 KB, and Apple
at 192 KB. What it rules out is the small end, and that is the end a capability microkernel is
otherwise attractive at: deeply embedded Cortex-M and Cortex-R parts, older in-order cores, and
microcontroller-class RISC-V. A machine can satisfy all three requirements above and still fail
this one, which is why it is stated separately rather than folded into "the peripherals are
documented".

**The honest caveat.** No cache is modeled by icount, and the development host's L1i is several
times the boards', so nothing in this tree has yet *observed* the effect this requirement protects
against. It is an argument from a 1995 paper plus a measured code size, not a measured miss rate.
The experiment that would settle it wants real silicon and performance counters, which milestone 74
has just made possible on radon.

## Trap: "ARM" is not "aarch64"

Cortex-M microcontrollers (STM32, most Arduino-adjacent parts) are 32-bit and have no
MMU. They cannot run the OS we are building. Ever. No virtual addresses, no isolation, no
user mode as we mean it. They can run an RTOS; that is a different thing.

We need Cortex-A53 or newer, in 64-bit mode. Same reason the RISC-V hardware we
considered had to be JH7110-class or better (see [mmu.md](mmu.md)).

## The realistic targets

| Device | Boot access | Peripheral docs | Verdict |
|---|---|---|---|
| **Raspberry Pi 4** (~$60) | Wide open. GPU firmware loads `kernel8.img` off a FAT32 SD card. No signing, no lock. | Excellent, plus the largest bare-metal community anywhere | **The next port.** Serial is a $10 USB-TTL cable on GPIO 14/15. |
| **Raspberry Pi 5** | Same | Worse. I/O routes through the RP1 southbridge over PCIe, less documented, less trodden for bare metal | Doable, but Pi 4 is the safer first port |
| **Rockchip / Allwinner SBCs** (Orange Pi, Radxa Rock, Pine64) | U-Boot from SD | Decent TRMs, much thinner community | Fine. Similar difficulty, less help when stuck |
| **NVIDIA Jetson** | Possible | Good docs, but the TegraBoot chain is genuinely complicated | More work than it's worth as a first port |
| **AWS Graviton bare-metal EC2** (`c7g.metal`) | UEFI. Rent by the hour; no hardware to buy or brick | SBSA-standard server ARM | **Interesting for a specific reason.** See below. |
| **Ampere Altra** workstations / dev kits | UEFI | Standards-compliant | Same category, but you'd own it |
| **Android phones** with unlockable bootloaders (Pixel, Fairphone) | fastboot | Poor. Qualcomm peripherals are barely documented, and you fight TrustZone | Painful. Possible, rarely rewarding. |
| **iPhone, iPad, Apple TV** | Locked, signed |: | No |

## The wild one: an Apple Silicon Mac

This is real, and it is not a jailbreak.

Apple deliberately permits booting non-Apple kernels on Apple Silicon. There is a
documented "permissive security" mode, and Asahi Linux is built entirely on it. Their
bootloader, m1n1, runs as an Apple-signed payload, then loads an arbitrary kernel image.
It also gives you a serial console over USB-C and a hypervisor mode you can use to trace
what macOS itself does to the hardware.

So an M-series Mac is genuinely, legitimately bootable with our own OS.

The catch is brutal: Apple documents none of the peripherals. Asahi reverse-engineered
the interrupt controller, the UART, the display, and everything else over several years.
We'd be leaning entirely on their documentation and would be a long way off the beaten path.

Filed as: not the second port. Possibly the fifth. A genuinely impressive one.

## The reframe worth taking seriously

Go back to the Alpha lesson in [portability.md](portability.md): the second port should be
as alien as possible, because that is what forces hidden assumptions into the open. Porting
to something *similar* teaches you very little.

Now look at what the Pi actually is:

| | Hardware discovery |
|---|---|
| QEMU `virt` | **Device Tree** |
| Raspberry Pi | **Device Tree** |
| Graviton / Ampere (UEFI + ACPI) | **ACPI tables + PCIe enumeration** |

The Pi is different peripherals inside the *same worldview*. Valuable, and it will shake out
real bugs, but it is a port within one model.

A UEFI + ACPI server ARM machine is a genuinely different world: a different firmware
handoff, and hardware discovered by walking ACPI tables and enumerating PCIe rather than
reading a flattened tree. *That* is the port that finds our hidden assumptions, and it is
where the `arch/` boundary either holds up or gets exposed as fiction.

Graviton bare metal costs a few dollars an hour, with no hardware to buy or brick.

## The plan, and what actually happened

This section was written before the recast at the top of the page and was never reconciled with
it. It is kept because the reasoning was sound and the outcome is a better answer to it than the
plan was, but read it against the record rather than as a queue.

1. ~~Raspberry Pi 4 is the next port.~~ **It was not.** The recast of 2026-07-27 made first
   silicon a VisionFive 2 instead, on the argument this note's own thesis supplies: the ISA is
   almost never the constraint, so the board whose firmware contract the kernel already speaks
   wins. radon booted nife and delivered the "I ran my OS on a computer I can hold" moment this
   item was really about. argon is the aarch64 board, and no Pi was bought.
2. Then a UEFI/ACPI target, precisely because it is alien. This one happened, and it did
   exactly what it was predicted to do. xenon (milestone 87) boots from real firmware, and the
   places it broke were the boundary: a loader that had to place an image at physical addresses,
   a low-memory range the firmware holds, and a framebuffer console because the serial chain was
   not the channel that worked. notes/x86-uefi-boot.md and notes/xenon-firmware.md.
3. Apple Silicon as the trophy. Still the trophy, still not attempted bare metal. What exists
   is the HVF leg (notes/hvf-leg.md): the aarch64 suite on the physical core under Apple's
   hypervisor, which is a flag rather than a port and was never claimed to be the trophy.

---

*Add to this file as new targets come up.*
