# The bench runbook: which machine, in what order, and what an evening buys

*(Name **provisional**, per the naming tenet; an architect names things.)*

This page **points at procedures rather than repeating them.** Every step below lives somewhere
already, and a second copy would drift from the first, which is the defect milestone 236 (three
derivations are copied between scripts, and nothing notices when they drift) was minted for on the
same day this was written. What this page adds is the part nothing else holds: **which machine to
spend an evening on, in what order, and what a result would mean.**

## The three machines, honestly

| | what it is | has it booted nife? |
|---|---|---|
| **radon** | StarFive VisionFive 2, JH7110, riscv64 | **yes**, including userspace, `init`, a child process and a confined driver |
| **xenon** | Dell OptiPlex 7050 Micro, x86_64 | **yes**, first light 2026-09-05: the milestone 243 framebuffer console carried the whole boot tour on video, with no serial cable. `notes/x86-uefi-boot.md` |
| **argon** | NVIDIA Jetson TX1, aarch64 | **no.** Milestone 127 is first light |

`notes/target-hardware.md` records the names and why they exist.

## Spend the first evening on radon, and the reason is arithmetic

**radon is the only machine where an evening is likely to produce a risk answer rather than a
bring-up story.** It boots, so the failure modes ahead of the interesting part are already known and
written down. One card and one power-on can settle work on two fatal risks:

- **Fatal risk 5** (it cannot be made reliable on multicore, and the bugs appear only on silicon).
  Milestone 225 (run the soak on radon, argon and xenon) is the run; milestones 219, 221 and 216
  built the workload, the cross-core hook and the console that watches it. This entry used to say
  the risk had already fired once on this machine; that reading was retracted on 2026-08-15
  (`notes/visionfive2.md`, fifth bench stop), and every result goes into
  `notes/multicore-defect-curve.md`.
- **Fatal risk 6** (a capability-confined userspace driver cannot drive real hardware at real speed).
  Milestone 159 (a real hardware entropy source: the JH7110's TRNG) is written, host-tested and has
  never touched silicon. It is the tree's only confined driver for a real non-virtio device.

**xenon and argon each cost an evening to learn whether they boot at all.** That is worth doing and
it is not the same kind of evening.

## radon, in order

**The procedures are canonical elsewhere. Follow them there, in this sequence:**

1. **Build and write the card.** `script/board-image --card /Volumes/NIFE` copies all three files as
   a set (milestone 217). **The archive is not optional** and a mismatched pair halts at
   `MEASURED BOOT REFUSED`, which cost a boot on 2026-09-01.
2. **Attach the console before power.** `script/board-console --until banner --for 120s --log ...`,
   115200 8N1. The UART is a WCH CH343 at `/dev/cu.usbmodem*` on patagonia.
3. **Power on and type nothing.** Milestone 218 shipped a `boot.scr.uimg` boot script and **it has
   never run on the board**; the line that exists only because of it is
   `nife: boot.scr is driving this boot, milestone 218`. If it does not appear, interrupt U-Boot and
   type the five commands `script/board-image` prints, which is the path that is known to work.
   Milestone 218's block has the three named failure modes and what each means.
4. **Then the TRNG.** Milestone 159's block, "The bench procedure, in order", with a table mapping
   each of the five possible `hw entropy` lines to what it means. **One of them routes to milestone
   220** (this kernel drives no clock or reset controller) rather than to 159, and that routing is
   the point: an all-zero bring-up diagnostic means the clock or reset, anything else means the
   driver's sequence.
5. **Then the soak.** `notes/soak.md`'s "Running it". **Check the first heartbeat before walking
   away**: `wakerate` should be about `100 * harts`, roughly 400 on radon, and `crossings` must be
   **rising** between beats rather than frozen. Eight hours of a non-crossing soak is eight hours of
   milestone 219's experiment rather than 221's, and the difference is invisible afterwards.

**Record `rounds`, `rate`, `wakes` and `crossings`** in `notes/soak.md`'s table, and the `hw entropy`
line verbatim.

## What can go wrong that is not the board

- **A leaked QEMU on patagonia holds a disk image's write lock**, and the next build fails with
  `Failed to get "write" lock` naming nothing (milestone 226). `lsof` on the image names the holder.
- **The console output can interleave.** The kernel prints its fault reports with its own UART
  driver while the userspace console server drives the same device, with nothing arbitrating, so two
  writers' bytes shuffle (milestone 230's finding). A marker that looks corrupt may not be.
- **Nothing can power-cycle radon remotely** (milestone 224). Its Kasa KP303 answers the vendor app
  and is invisible to ARP from both patagonia and cordoba, so a hung soak needs a person.

## xenon, if there is a second evening

Milestone 87 (the x86_64 bare-metal machine) **was** first light, and it has now happened once,
on 2026-09-05. The procedure is `notes/x86-uefi-boot.md`'s "The bench: booting nife on the OptiPlex
7050", which is written to be followed rather than interpreted: `cargo xtask uefi-image`, one file
to a FAT32 stick at `\EFI\BOOT\BOOTX64.EFI`, and the serial chain already on the desk. A second
evening on xenon is now a bench evening rather than a bring-up one.

**Milestone 195 closed two of the three questions only xenon could answer**, on patagonia, on
2026-09-02: a real function's MSI-X table reachable once *firmware* placed the BARs, and a
multi-APIC machine still delivering to the boot core. **One remains and no emulator can answer it:
whether this firmware leaves VT-d interrupt remapping off.**

**And one question 195 created**: whether the Dell leaves 32 MiB free. `PHYS_START` moved from 1 MiB
to 32 MiB because OVMF holds ACPI NVS and its own allocations across the low range. If the Dell does
not, the loader now prints which range it wanted and which descriptors are in the way, rather than
`Load Error` and nothing else.

## argon, and why it is last

Milestone 127 (the seL4 machine) is first light, and it is the longest of the three because nothing
of nife has run on it. Both pre-board prerequisites are now built: the EL2 to EL1 entry drop
(2026-09-02, rehearsed under QEMU with `virtualization=on`), and the cycle-counter authority
question that milestone 74's aarch64 half was waiting on (DECISIONS 139, answered 2026-09-02).

127's own block lists what to verify at the bench, and it is the honest list of unknowns rather than
a procedure: kit contents, the flashed L4T revision and whether U-Boot comes up without a JetPack
detour, the entry exception level and the DTB register from *this* U-Boot, PSCI visibility to a
non-Linux payload, `PMCCNTR_EL0` readable at EL1, and what pins the 1.9 GHz clock.

**What argon is for** is milestone 25's comparison against seL4's published 413 and 426 cycle
figures on identical silicon, which is why the board was bought. Milestone 237 (the
cycle-counter grant costs 136 bytes of IPC fastpath for an instrument nothing can request)
made that instrument a feature rather than something production carries.

## BUGS

- **This page is an index and will rot if a procedure moves.** It cites by milestone and by note
  path rather than copying steps, which is the cheapest defence available and not a guarantee.
- **It assumes one person at one bench.** Nothing here says what to do if a machine needs two
  evenings, or what to abandon when time runs out.
- **No procedure here has been run end to end by its author.** Each was written by the lane that
  built the thing it tests, and the ordering is this page's own.
- **`script/board-console` writes bytes into its log that are not valid UTF-8**, under sustained
  board output. Found 2026-09-16 across a five-boot job-mix session: `tr` refuses the file with
  `Illegal byte sequence`, `awk` dies with `towc: multibyte conversion failure`, and, worst of the
  three, **`grep` silently reports nothing** because it decides the file is binary. The measured
  rows themselves were intact in every case; what breaks is every ordinary tool a person would use
  to read the transcript, and two of the three break *quietly*. Until it is fixed, read a board log
  with `LC_ALL=C` and `grep -a`, and strip it with `LC_ALL=C tr -cd '\11\12\15\40-\176'` before
  committing it to `bench/`. The cause is not diagnosed: it may be line noise on the UART at 115200
  with no flow control, or the tool's own write path.
- **Two `script/board-console` processes on one serial port silently split the byte stream.** Each
  gets a fraction and neither reports a problem, so a capture looks merely incomplete rather than
  wrong. This cost a whole job-mix boot on 2026-09-16, whose log was missing one sweep point's
  result and all of another's, and read exactly like a board that had wedged. The previous capture
  had been left running on its own `--for` timer. **Check `lsof /dev/cu.*` before starting a
  capture**, and kill the previous one rather than assuming its deadline has passed.
