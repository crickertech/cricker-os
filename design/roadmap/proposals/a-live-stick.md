---
status: PROPOSED
raised: 2026-10-04
milestone_dependencies: 242, 400
decision_dependencies: none
machine_requirements: x86_64 UEFI silicon with a USB keyboard and a monitor
specific_machine: none
needs_person: yes
---
# A live stick: boot a PC into nife without touching its disk

Asked for by calef on 2026-10-04 (UTC). Written by the lane `lane/live-stick-proposal` at base
`2d154f848`. Title, slug and every new name here are provisional.

## What "live stick" means

A stick that boots a PC such as xenon into a usable nife, runs from memory, and leaves the PC's own
disk exactly as it found it. Removing the stick and rebooting gives back whatever the PC ran before.

Exit criteria, each something a person at xenon can see happen:

1. With Secure Boot off, xenon starts the stick from the firmware's one-time boot menu.
2. The boot tour and then `$` appear on the monitor.
3. `echo hello` typed at a USB keyboard prints `hello`. No serial cable is attached.
4. A file written in the session reads back in the same session (`ls`, `wc`).
5. With nife installed on xenon's disk, the stick still boots the stick's own system. The loader
   says so on screen, and the installed system's files and boot-slot tries are unchanged when it
   next boots.
6. With the stick removed, the next boot is whatever xenon booted before.
7. Nothing is written to the internal disk unless the person types the install confirmation.

Criterion 5 is checked under QEMU as well: a gate hashes the NVMe image before and after a stick
boot that writes files, and requires the two hashes to match.

## What is already built

More than the brief assumed. The tree already has a live stick, less the gaps in the next section.

- The one-file boot. `\EFI\BOOT\BOOTX64.EFI` carries the kernel and the archive, sealed together
  (milestone 87 (the x86_64 bare-metal machine), BUILT). `stick_maker` writes it from macOS, Linux
  or Windows without erasing the stick (milestone 441 (the program that makes the stick), BUILT).
  It writes `BOOTX64.EFI`, `BOOTAA64.EFI` and `BOOTRISCV64.EFI` side by side, so one stick boots
  all three architectures. That is proved under QEMU only: no physical stick has booted argon or
  radon yet (`notes/boot-stick.md`).
- It runs from memory. The loader hands the archive over as a PVH module and reads its own file back
  as a second module while the firmware is still up (milestone 515 (the installer), R1;
  `notes/installing.md`). Nothing reads the stick after `ExitBootServices`. The stick can be pulled
  once the prompt appears.
- A boot with no disk reaches the prompt. `crates/system_initializer` gives the shell four slots
  without a filesystem and five with one.
- The installer stick is already a live stick when nobody types `INSTALL`. The offer waits 30
  seconds, then "anything else continues the boot" (`kernel/src/user/install_service.rs`). That
  ordinary boot is the live system.
- The prompt on the screen: milestone 400 (the shell on the firmware screen) is PARTIAL, built and
  gated under OVMF, xenon outstanding.
- The keyboard: milestone 242 (USB host and HID) is in the merge queue as PR #1629. Its BUGS say
  root ports only, one controller, one keyboard.

## The gaps, and who owns each

| Gap | What happens today | Owner |
|---|---|---|
| G1. A stick booted on an installed machine starts the installed system | The stick's loader is the same binary as the disk's. Its chooser finds boot slots on the NVMe, may spend a try (a GPT write), and chain-loads the slot (`uefi_loader/src/chooser.rs`, `choose`) | Nobody |
| G2. A stick boot mounts the internal disk | With no virtio disk, `fs_service::wire_servers` falls through to `nvme_disk()`. If nife is installed there, `redoxfs_server` opens it with `cleanup: true`, which commits on mount (`redoxfs_server/src/lib.rs`, `open`) | Nobody |
| G3. A boot with no disk has no filesystem at all | The shell gets no directory capability, so criterion 4 fails | Nobody |
| G4. The install offer cannot be answered at a USB keyboard | The kernel reads the UART before the progenitor exists (`install_service.rs` BUGS; `notes/xenon-bench-2026-10.md`) | Nobody |
| G5. The prompt on xenon's monitor | Built under OVMF only | Milestone 400, bench step |
| G6. A USB keyboard | In the queue | Milestone 242, PR #1629 |
| G7. Secure Boot on | Must be off | Milestone 500 (a stick that boots with Secure Boot on), NOT-STARTED |
| G8. Network | virtio-net only | Milestone 494 (the network card a PC has), PR #1632; Wi-Fi is a proposal in progress on PR #1650 (`wifi-on-a-pc-that-has-no-ethernet.md`) |
| G9. Saving files back to the stick | No USB mass storage, no writable FAT | Nobody; see Persistence |
| G10. The install path on aarch64 and riscv64 | No install offer: a device-tree handoff has no slot for the boot file, and `BOOTRISCV64.EFI` is not an 8.3 name the FAT writer accepts. Both run live meanwhile | Milestone 568 (the boot file has nowhere to go on a device-tree machine); milestone 560 (a long file name, or riscv64 cannot be installed) |
| G11. Each board's install-target disk driver | Unsurveyed. Nobody has listed which disk argon and radon would install onto, or whether nife has a driver for it | Nobody |

G1 and G2 are the findings of this proposal. Both were read in the code, not run on silicon.

G1 means criterion 5 fails outright today. It also corrects the premise of milestone 572 (there is
no way back from the stick). That block says an installed disk is "never offered an install again"
because the survey sees nife. The survey never runs: the chooser starts the disk's slot first, and
"an image started from a slot has no boot file" (`chooser.rs` BUGS), so the offer is skipped before
any survey. 572's REPLACE question would not be reached on any machine that has slots.

On a disk carrying Windows, G2 is probably harmless. `FileSystem::open` should find no RedoxFS
header and fail before it writes. That was read, not run. The install survey also holds a
write-capable whole-disk endpoint. Its role is what keeps it read-only, which the
`install_service.rs` BUGS already record.

### The fix for G1 and G2, recommended

One fact closes both: which disk this file came from. The loader already holds it, as its
`LoadedImage::device_handle` and that handle's device path.

- The chooser boots slots only from the disk its own file lives on. That is exactly what the
  chooser's first BUGS entry already asks for, for a different reason ("boot the slots on *this*
  disk"). A stick is never that disk, so a stick never chain-loads.
- The loader tells the kernel whether the boot medium is the NVMe disk, as one more field in the
  `machine_discovery` handoff. The kernel mounts the NVMe filesystem only when it is. Loader and
  kernel are sealed into one file (`refuse_an_unsealed_pair`), so nobody outside this tree reads
  the field, and it can change freely.

Considered and lost: using the firmware's `RemovableMedia` flag. It is cheaper, but a USB SSD often
reports a fixed disk (recalled, not measured), and the question is "is this the disk", not "can it
be unplugged". A build-time "live" flag also lost: the installer copies the stick's own file to the
disk, so the installed copy would inherit the flag.

### G3: a filesystem in memory, recommended

A block server over a run of RAM, serving the `filesystem_protocol::blk` wire, with `mkfs` run on it
at boot. `redoxfs_server` and `mkfs` are reused unchanged. The live session then has a writable
filesystem that is gone at power-off. A provisional name for the server: `memory_disk`.

Size: a fixed 256 MiB, provisional. xenon has 16 GB. The QEMU gates run with far less, so the size
should come from the memory map with a floor, and the lane measures it.

Prior art, read: Redox's `lived` (`redox-os/drivers`, `storage/lived/src/main.rs`, MIT) serves the
live image from physical memory with a `HashMap` of overwritten blocks on top. nife does not need the
overlay: its programs come from the archive, not from a disk image, so a blank RAM disk is enough.

## Persistence: ruled

**Ruled by calef on 2026-10-04 (UTC), late evening, on PR #1651: no persistence in v1.** The live
system's files live in memory only. His reason, in his words: *"Persistence seems neat. However it
seems only useful if programs carry over. Without the programs, the files aren't typically very
useful."*

### What a persistent stick is gated on

A persistent stick boots on any architecture and carries work between machines, of the same
architecture or a different one. By the ruling, that needs programs to carry over, not only files.
So it depends on milestone 198's packages being usable on every architecture from one stick. Three
options were recorded; two remain, and neither is chosen:

- (a) One package holds all three architectures' builds.
- (b) The stick records which packages are installed, and each architecture fetches its own build.
- (c) Programs in an architecture-neutral form, such as WebAssembly. Refused by calef on
  2026-10-05 (UTC): *"Wasm sounds like a horrible performance trade off for a neat feature. No
  thanks."*

It also needs a USB mass storage driver, a stick layout with a persistent area, and encryption at
rest. A lost stick holding someone's work is exposed, which is why Tails encrypts its area.

The stick layout v1 writes is provisional. Today that is `stick_maker`'s files in `\EFI\BOOT\` on
whatever FAT volume the stick already has, plus `NIFE.TXT`. v1 must not grow anything a persistent
area would have to work around.

The analysis below is what was put to calef.

| | Shape | What it needs | Verdict |
|---|---|---|---|
| P0 | None. Everything is RAM; the stick is read once, by the firmware | G3 only | Ruled for v1 |
| P1 | A nife data partition on the stick, RedoxFS, mounted at boot. Ubuntu's `writable` partition, Tails' Persistent Storage | USB mass storage (bulk-only transport and SCSI); the xHCI driver split into a host driver and class drivers, because the keyboard and the stick share xenon's one controller; hubs; `stick_maker` partitioning the stick | The named second rung |
| P2 | A filesystem image in a file on the stick's FAT volume. Fedora's old `--overlay-size-mb`, Ubuntu's `casper-rw` file | All of P1, plus a FAT32 writer for a foreign volume (milestone 140 (mount a drive this system did not create), read half NOT-STARTED) | Lost: P1's cost plus a second filesystem underneath |
| P3 | The loader reads a saved state file before `ExitBootServices` | Nothing new to read; no way to write back once the firmware is gone | Lost: read-only persistence is not persistence |

Why P0 for v1. P1 is the largest new driver work on the list. Milestone 242 itself declines storage,
and its xHCI driver owns the controller alone. Splitting it is the shape Redox already has (`xhcid`
serves class drivers such as `usbscsid` over a client handle), so the design is known. It is still a
milestone or two. P0 ships the moment G1 to G3 are closed.

P1 also changes a promise. `stick_maker` today copies files and says "Nothing on it is erased". A
data partition means repartitioning the stick, so the program would need a second, destructive mode.

Prior art agrees with P0 as a default. Fedora Media Writer, Fedora's own stick maker, writes no
persistence ("not yet", per its developer in Fedora Magazine's *How to make a Fedora USB stick*).
Redox's live ISO runs from RAM. Tails makes persistence opt-in, encrypted, and unlocked per boot.
Tails also never mounts the internal disk unless the person sets an admin password and mounts it by
hand (`tails.net`, *Accessing the internal hard disk*), which is the rule criterion 7 asks for.

Would P0 still be chosen if P1 cost the same? No. A live system that can keep a file is better.
**The P0 recommendation is about effort and sequencing**, stated as such. The non-effort argument is
that P1 is a write path to a stranger's stick, with its own crash-consistency question, and v1 has
no customer asking for it yet.

## One image or two: ruled

**Ruled by calef on 2026-10-04 (UTC), late evening, on PR #1651: O1 on the universal stick.** One
image for all three architectures, built on milestone 441's `stick_maker`. It boots live by
default and offers the install at boot. On aarch64 and riscv64 there is no install offer until
milestones 568 and 560 land (G10); those two architectures run live meanwhile.

The options as they were put to him follow.


How the live stick relates to milestone 515's installer stick. The installer writes a whole disk,
so the shape a stranger meets is a published fact once release images exist.

| | Shape | For | Against |
|---|---|---|---|
| O1 | One image. It boots live; the install offer is a question at boot with a timeout, as today | Already built. One download, matching §157 (a trivial install is a web page, a USB drive, and packages)'s one program. Haiku's shape: "Install Haiku" or "Try Haiku" at start | A 30-second pause on every live boot of a machine with an NVMe disk. G4 must move the question into userspace |
| O2 | One image. It boots live; installing is a program typed at the prompt (`install`, provisional) | Fedora's and Redox's shape: the installer is an app on the live desktop. No pause | The disk grant moves from a kernel service to the progenitor. The property "the authority to wipe the disk does not exist until the answer does" (`notes/installing.md`) has to be rebuilt there |
| O3 | Two images: a live stick and an installer stick | A live stick that cannot wipe a disk by any keystroke | Two artifacts from one sealed file differing by a flag. `stick_maker` grows a choice. The installer copies its own file, so an install from the installer image installs the installer |

**Recommendation: O1 for v1, with O2 as the later polish.** O1 is what exists. G4 is needed by
both O1 and O2, since a USB keyboard only works after the progenitor starts its driver. Once the
question is asked in userspace, O1 and O2 differ only in when it is asked.

Would O1 still be chosen at equal cost? Against O3, yes: one artifact, and every live system read
(Fedora, Ubuntu, Haiku, Redox) ships one image. Against O2, not obviously. O2 is the more ordinary
experience, and its cost is the authority rework. So O1 over O2 is partly effort, and it is
reversible: the offer is a few lines of a kernel service, and nobody outside the tree has a stick.

## The seven questions

1. Considered. Persistence P1 to P3 and image shapes O2 and O3 above, with reasons. For G1 and
   G2, `RemovableMedia` and a build flag, also above.
2. What the tree does in the analogous case. It already boots one file and runs from memory,
   and its installer stick already continues to a live boot. The chooser BUGS already ask for the
   device-path match G1 needs.
3. Prior art, read on 2026-10-04. Tails (`tails.net` persistent storage and internal disk
   pages). Redox (`doc.redox-os.org/book/installing.html`; `lived` and `usbscsid` source on
   `gitlab.redox-os.org`). Haiku (`haiku-os.org/get-haiku/installation-guide/`). Fedora (Fedora
   Magazine on Media Writer). Ubuntu persistence was read only in secondary sources:
   `help.ubuntu.com/community/LiveCD/Persistence` returned 503. They describe a partition labelled
   `writable` (`casper-rw` before 20.04) and a `persistent` boot parameter.
4. Is the premise true? Mostly, and not where it matters most. "A stick boot runs from memory"
   is true. "It never writes the internal disk" is false on an installed machine (G1, G2).
5. Cost. Not measured; no code was written. Sizes are judgements: G1 and G2 are one device-path
   walk and one handoff field. G3 is one small block server. G4 is moving one question. P1 is a
   driver family.
6. Reversibility. All of v1 is code in one sealed file. The irreversible part is publication
   (§157 rung 4), which is calef's, and the O1/O2/O3 shape once it is published.
7. Equal cost. Answered per fork above. P0 and O1-over-O2 are partly effort and say so.

## Reuse

Under §46 (thin primitives or whole subsystems) as amended 2026-10-04 (PR #1637, in flight): take or adapt by default outside the kernel
and the Kani-proved crates.

- `memory_disk` (G3): Reuse: adapted from Redox `lived`'s shape. It speaks nife's own `blk` wire,
  which nothing upstream reads, and it is small. `redoxfs_server` and `mkfs`: taken, unchanged.
- G1, G2: Reuse: write. A device-path walk in `uefi_loader` and a handoff field; loader-to-kernel
  wire (rule 7).
- USB mass storage (P1, not v1): Reuse: adapt Redox `usbscsid` (MIT; `protocol/bot.rs`,
  `scsi/cmds.rs`). Its host-driver split is the model for splitting milestone 242's driver.
  `crab-usb` 0.12.1 (Apache-2.0) was read and has no mass-storage class. The `xhci` crate has not
  released since 2023-07, and PR #1629 already wrote its own.
- FAT (P2 only, which lost): `embedded-sdmmc` 0.10.0 (MIT or Apache-2.0, `no_std`, no `alloc`,
  read and write FAT16/32 over a `BlockDevice` trait, released 2026-08-10) is a better `no_std`
  candidate than `fatfs` 0.3.6, which needs `core_io` and last released 2023-01. Recorded for
  milestone 140's lane, which owns that choice.

## Ranking

The live stick is milestone 198 (a package manager, and the trivial install)'s rung 1 (1a to 1d) with G1 to G4 added. §157 judged it "right
destination, wrong first rung" because the blockers were input drivers. Milestone 242 is now in the
queue, so that blocker is closing.

On principle 1 it sits on the customer path, ahead of rung 2b. A stranger who has Windows on a PC
will try a stick that leaves the disk alone long before one that offers to wipe it. And the installer
is whole-disk only (515 BUGS). So the live stick is the first thing a second customer can safely
meet. G1 and G2 rank first among the gaps: they are small, they are defects in what already
ships, and every bench boot of a stick on an installed xenon hits them. Persistence ranks after
rung 3c; nothing on the customer path waits on it.

Proposed order: G1 and G2 together (one lane, loader and fs service), G3, then G4 after 242
lands, then the xenon bench run of criteria 1 to 7. Each is a provisional milestone the integrator
numbers.

## BUGS

- G1 and G2 are read from code, not seen on silicon. One bench boot of a stick on an installed
  xenon confirms them: the loader would print `boot slots on one disk`.
- G1 can write a stranger's disk today. Spending a try rewrites the GPT on a machine whose slot is
  in trial, from a stick the person thought was read-only.
- The live session loses everything at power-off under P0. The prompt should say so on boot.
- A disk carrying Windows is assumed safe under G2 from reading `FileSystem::open`. Nobody has run
  a stick boot against a non-nife NVMe image and compared hashes. Criterion 5's gate should include
  that case.
- The 256 MiB RAM disk size is a guess.
- G1 to G4 were read on the x86_64 path only. PR #1652 (open), which builds G1 and G2 as calef's
  option K2, never mounts the internal NVMe on aarch64 or riscv64 builds. Its
  `memory::booted_from_nvme` is false unless `arch::x86_64::machine` records it, and
  `fs_service::wire_servers` reaches `nvme_disk()` only when it is true. Checked against the diff,
  not run. The install offer is x86_64 only until G10 closes.
- SATA and RST/VMD disks are not touched by any of this, which is safe by accident: nife has no
  driver for them (515 BUGS).
- A keyboard behind a hub is not found (242 BUGS). Many PC keyboards are behind one.

## Names (provisional)

"live stick", `memory_disk`, the `install` program in O2, and the handoff field for the boot medium.
None is ratified.
