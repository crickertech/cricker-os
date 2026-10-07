# xenon bench card, October 2026: risk 6's throughput boots, then the install

*(Name provisional, per the naming tenet: a lane picked `notes/xenon-bench-2026-10.md` on
2026-10-04 and calef names things.)* One evening at xenon, two steps, in this order. Step 1 is
milestone 261 (the NVMe driver leaves the kernel)'s bench, the decisive experiment for
`design/fatal-risks/README.md` risk 6. Step 2 is milestone 515 (a stick that puts itself on the
machine's disk)'s exit criterion 2. Both procedures are written in full elsewhere and this card is
the evening's checklist: [risk-6-bench-evening.md](risk-6-bench-evening.md) for step 1 and
[installing.md](installing.md) for step 2. Where this card and those pages disagree, the machine
decides, and the page that was wrong gets corrected.

Prepared 2026-10-04 (UTC) at `a08efc8dc`. On that commit both images built, `cargo xtask
disk-throughput` rehearsed all four cases with the expected verdicts, and `cargo xtask
install-boot` passed (30 s, warm). Nothing in this card has touched xenon, a stick or the router.

## The order, and why it is not a preference

**Risk 6's bench first, the install second.** The bench kernel writes 64 MiB of stamped blocks at
4096-byte blocks `256..16640`, which is bytes 1 MiB to 65 MiB. The installer puts its data
partition at LBA 2048, which is also 1 MiB in, and `mkfs` writes the RedoxFS header there. So:

- Bench after install: the bench overwrites the installed filesystem and the installed machine no
  longer reads back `made-on-target`. Step 2 would have to be done again.
- Install after bench: the installer's survey reads the partition table at LBA 1, which the bench
  never touches, finds no nife there, and offers the install. The stamps are then overwritten by
  the install, which is fine because they were already photographed and checked on the night.

The bench also prints the namespace's exact size, which is the cross-check step 2 needs (below).

## What calef needs at the bench

- A monitor and a USB keyboard on xenon. Without a keyboard it halts at POST
  (`notes/x86-uefi-boot.md`).
- **The serial chain, for step 2 only**: the C4PDJ module's COM1, the StarTech null-modem barrel and
  the FTDI adapter, into a Mac running `screen`. The installer reads its confirmation from the
  UART (`kernel/src/user/install_service.rs`, `console::read_line`). There is no USB keyboard
  driver yet (milestone 242 (USB host and HID, because on commodity hardware the keyboard is not a UART) is a draft), so **typing `INSTALL` at xenon's own keyboard does
  nothing**. With no serial, the offer times out after 30 seconds and the boot continues without
  installing. That is the safety property working, and it means step 2 cannot be done without it.
- Two FAT32 sticks (GUID Partition Map, MS-DOS (FAT)), or one stick and the Mac between steps.
- A phone for photographs. About 45 minutes: three boots for step 1, two for step 2.

## Before the evening, on patagonia

Build on current `main` and note the commit for the Results rows. Each build takes about 20 seconds
warm. Prebuilt copies from `a08efc8dc` were at `xenon-bench-images-2026-10-04/` in the worktree root (`nife-worktrees` beside the main checkout)
with a `SHA256SUMS` file, for an evening where building is not convenient; a fresh build is
preferred because `main` moves.

```sh
cd <main checkout>
pgrep -l qemu                                  # nothing of yours should be running
git log -1 --format=%h                         # goes in both Results rows
cargo xtask disk-throughput --stage-only       # -> target/esp-disk-throughput/EFI/BOOT/BOOTX64.EFI
cargo xtask install-boot                       # builds target/esp-install and runs the QEMU gate; must end PASS
```

`install-boot` has no stage-only flag, so the gate is how the installer image gets built, and it
also proves that image installs under OVMF before anyone carries it to the machine. The install
image must be a release build: the loader refuses to offer a debug image for install
(`uefi_loader::image::carries_debug_info`), and `install-boot` forces release.

Write the sticks. **The two images must never be confused**: the first one writes to the disk
without asking, by design.

```sh
# stick 1: risk 6's bench (label it on the outside too)
mkdir -p /Volumes/NIFE/EFI/BOOT
cp target/esp-disk-throughput/EFI/BOOT/BOOTX64.EFI /Volumes/NIFE/EFI/BOOT/BOOTX64.EFI
diskutil eject /Volumes/NIFE

# stick 2: the installer
mkdir -p /Volumes/NIFE/EFI/BOOT
cp target/esp-install/EFI/BOOT/BOOTX64.EFI /Volumes/NIFE/EFI/BOOT/BOOTX64.EFI
diskutil eject /Volumes/NIFE
```

Netboot is possible for step 1 and not for step 2. Step 1 can be served with
`script/board-netboot --root target/esp-disk-throughput`. That needs milestone 260 (boot xenon
over the network)'s two firmware settings and router edit first, and that block is still
PARTIAL. Step 2 cannot netboot at all: the loader reads its own file back through the boot
volume's `SimpleFileSystem` to hand it to the installer, and a PXE boot has no such volume. The loader then
prints `uefi_loader: the boot volume has no filesystem protocol; cannot install` and boots
without an offer (`uefi_loader/src/main.rs`, `place_boot_file`).

## Step 1: risk 6's bench, three boots

Nothing changes in firmware: `VT for Direct I/O` is already enabled. For each of three boots:

1. Stick 1 in. Power on, F12, pick the `UEFI: <stick>` entry.
2. Watch the screen as the `vt-d        : drhd ... up` lines print. If the screen blacks out or
   freezes there, photograph the last readable screen and see "If the screen goes black" in
   [risk-6-bench-evening.md](risk-6-bench-evening.md); do not go on to step 2 that night.
3. Photograph the whole final screen, the `vt-d` lines and the `disk-throughput:` block
   together. The boot ends at `disk-throughput: done, halting.`
4. Power off fully before the next boot. Three boots, because each is one pass with no warm-up:
   quote the median and the spread, never one boot.

Optional, if a Linux live stick is to hand: the queue-depth-1 `fio` pair in that page's step 5, on
the same window, gives the ratio that says whether the rate is "real speed". Do it before step 2,
since it reads and writes the same 64 MiB.

Done on 2026-10-04, evening (bench clock 23:12 to about 23:25, zone not recorded): Fedora 44 Live,
fio-3.40, three runs (psync, pvsync2 `--hipri` which did not engage polling, io_uring `--hipri`
which did). Best Linux write is 425 MB/s (polled io_uring), so nife's median is 1.08x. Linux read varies
from 74 to 188 MB/s with drive state, and read median latency matches nife's. ASPM and APST were checked
and ruled out. Numbers and caveats are in
[the baseline's page](risk-6-bench-evening/linux-qd1-baseline-2026-10-04.md).
**Open: the Linux read tail and drive-state variance are unexplained (next check: `nvme smart-log`
temperature), and nife's counter frequency has not been cross-checked against wall time.**

### What each line means

| line | what it says | good looks like |
|---|---|---|
| `fatal risk 6 bench boot (milestone 261). WRITES to the NVMe disk.` | the right stick: this kernel writes without asking | present |
| `release build, N Hz counter` | the profile and the clock every figure below is converted with | `release` |
| `measured    non_volatile_memory_express sha256 ...` | the EL0 driver is the one this kernel vouches for | `matches the table` |
| `preflight 1/2 dmar scope` | the firmware's DMAR gives the NVMe's requester id to a VT-d unit that is up, so its DMA is translated | `PASS`, naming a `drhd` (predicted `0xfed91000`, the catch-all) |
| `preflight 2/2 lba size` | the namespace's LBA size lets 4096-byte blocks be 1 to 8 LBAs; also prints the namespace's size in bytes | `PASS  512-byte lbas, blocks_per 8`. **Write down the namespace bytes**: step 2 checks against it |
| `ipc floor` | 2000 round trips to the driver with no device work: the IPC share of every per-block figure | any value; subtract it before quoting |
| `write` / `flush` / `read` | 16384 blocks of 4096 bytes, one command in flight, polled | numbers. Queue depth 1, so never against a qd32 figure |
| `verified    16384 of 16384` | every block read back carried its own number | N equal to M. Anything less is a correctness failure: stop |
| `verdict CONFINED-AT-RATE` | both preflights passed and the data verified: a confined EL0 driver moved verified blocks on silicon | this is risk 6's decisive experiment running |
| `verdict UNCONFINED` | the data moved but preflight 1 failed: nothing translated the device's DMA | not risk 6's answer; a kernel lane, not a bench step |
| `verdict SKIPPED` | preflight 2 failed, or no NVMe on the bus | a skip is not a pass. The LBA case is a reformat (calef's call); no controller means the root-port fix regressed or SATA mode is no longer AHCI |
| `verdict FAILED: ...` | bring-up, the server or verification failed; the line names the phase | photograph and stop |
| `MEASURED BOOT REFUSED` | the kernel and archive on the stick come from different builds | rebuild with `--stage-only` and recopy |

The full routing for each FAIL wording is [risk-6-bench-evening.md](risk-6-bench-evening.md)'s
"What each outcome means".

## Step 2: the install, then the boot from disk

**Done 2026-10-04 (UTC).** Transcript: `bench/xenon-2026-10-04/install-and-disk-boot.log`. The
install offer named 256060514304 bytes, `INSTALL` was sent (by a watcher script on patagonia's
serial port, with calef's go-ahead), the install finished, and after the stick came out xenon
booted from the disk with `boot file : none` and no offer. **The card's install image was stale:**
the `a08efc8dc` copy predates the VT-d write-back fix (PR #1636) and would have hit the NVMe
CompletionTimeout. The image used was built from `lane/xenon-nvme-diag` at `0c16dcaac` (sha256
starts `69cc518b`). The read-back (item 7) passed at about 22:25 UTC: `1 10 57`, in `bench/xenon-2026-10-04/readback.log`. Photographs owed.
The steps below are the plan as written.

On the Mac, before powering xenon on, with the FTDI adapter plugged in:

```sh
cd <main checkout>/target
ls /dev/cu.usbserial-*                         # A28FR8LZ on 2026-09-17
screen -L /dev/cu.usbserial-A28FR8LZ 115200    # -L logs to ./screenlog.0; exit with ctrl-a k
```

`script/board-console` cannot be used here: it sends only its named modes' bytes and cannot
type `INSTALL` (the rule from milestone 324 (the bench console cannot speak to any of the three boards)).

Photograph each numbered step.

1. Boot the installer. Stick 2 in, power on, F12, pick the stick. The loader prints
   `nife uefi_loader: milestone 87` on the monitor, the banner it has carried since milestone 87 (the
   x86_64 bare-metal machine). If it prints
   `uefi_loader: ... cannot install`, the offer will not come; photograph it, that line says why.
2. Read the offer, on serial and on the monitor:

   ```text
     install     : this system was booted from a file and can install itself.
     install     :   TARGET: the NVMe disk attached to this machine, N bytes.
     install     :   EVERYTHING ON THAT DISK WILL BE DESTROYED.
     install     :   Type INSTALL and press return to proceed; anything else continues the boot.
   ```

   **The installer does not name the Micron.** It names the disk by size only; reading the model
   is milestone 569 (the disk an installer names has no model, only a size), not started. The
   check on the night is that N equals the namespace bytes step 1's preflight 2 printed. If they
   differ, do not type anything: let it time out and photograph the screen.
3. Confirm, within 30 seconds, in `screen` on the Mac: type `INSTALL` and return. Anything else
   or nothing continues the boot without writing.
4. Install. Expected, in this order:

   ```text
     install     : partitioning and copying the boot file...
     install     : installed. nife data at LBA 2048, EFI system at LBA <E>.
     install     : filesystem created.
     install     : DONE. Remove the installation medium and reboot.
   ```

   `<E>` depends on the disk size and is not predicted here. `FAILED (...)` means the disk may be in
   any state: photograph it and stop. `mkfs refused` means the table is written and the data
   partition is empty, which the second boot will show as a missing `made-on-target`.
5. Remove the stick. Then power off.
6. Boot from the disk. Power on with no stick and no F12 first. If the firmware finds the disk
   on its own, that is milestone 515's B1 measured on real firmware for the first time. If it says
   there is no boot device, F12 and pick `UEFI: Micron 2450 NVMe 256GB`, and record which it was:
   B1 holds only for the first. Expected on the monitor, then on serial:

   ```text
   uefi_loader: starting boot slot 0
   ...
   uefi_loader: started from boot slot 0
   nife: handing the system to the userspace progenitor.
   nife capability shell.
   ```

   There must be no install offer this time; an offer on an installed disk is a defect.
7. Read the file back, in `screen`: `ls` lists `made-on-target`, and `wc made-on-target`
   prints `1 10 57`. This is what makes it an install rather than a boot, and it is the line the
   QEMU gate checks.

## Recording it

Transcribe each photograph's `disk-throughput:` block into
`bench/xenon-<date>/disk-throughput-boot<N>.log` and fill the Results row in
[risk-6-bench-evening.md](risk-6-bench-evening.md). Commit `screenlog.0` from step 2 as
`bench/xenon-<date>/install.log` beside the photographs, and say in the commit whether step 6 needed
F12. The verdicts go to the milestone 261 and 515 blocks and to risk 6's appendix through whoever
holds `design/`.

## EXAMPLES

The step 1 block as the QEMU rehearsal printed it on 2026-10-04 (case `root-port`, xenon's shape).
QEMU's disk is 16 MiB, so its window is 3840 blocks rather than 16384, and its numbers say nothing
about a disk:

```text
disk-throughput: preflight 1/2 dmar scope : PASS  nvme 01:00.0 (rid 0x0100): drhd 0xfed90000 owns bridge 00:03.0 above it
disk-throughput: preflight 2/2 lba size   : PASS  512-byte lbas, blocks_per 8 (needs 1..=8); namespace 16777216 bytes
disk-throughput: verified    3840 of 3840 blocks came back stamped with their own number (window 256..4096 in 4096-byte blocks)
disk-throughput: verdict CONFINED-AT-RATE: read 106938625 B/s, write 101706067 B/s, EL0 driver; drhd 0xfed90000 owns bridge 00:03.0 above it
```

Step 2 as `cargo xtask install-boot` printed it the same day, on a 1 GiB QEMU disk:

```text
  install     :   TARGET: the NVMe disk attached to this machine, 1073741824 bytes.
  install     : > INSTALL
  install     : installed. nife data at LBA 2048, EFI system at LBA 1046528.
uefi_loader: starting boot slot 0
  1 10 57
install-boot: PASS
```

## BUGS

- Step 2 needs the serial chain at the bench, because the confirmation is read from the UART
  only. Until milestone 242 lands a USB keyboard, there is no other way to answer the offer.
- The offer names the disk by size, not model (milestone 569). On xenon there is one disk, so
  the size cross-check against step 1 is enough; on a machine with two it would not be.
- Running step 1 again after step 2 destroys the install, without asking. The bench stick
  must be labeled and kept apart from the installer stick.
- The install's second boot is unmeasured on Dell firmware. Only OVMF has found an installed
  disk's `\EFI\BOOT\BOOTX64.EFI` with no boot variable. Step 6 records which path the firmware took.
- The prebuilt copies are pinned at `a08efc8dc` and the install image among them is stale (see Step 2: it predates #1636), outside any worktree so a prune does not take
  them. Nothing deletes them either; remove the directory once the evening is recorded.
- Every xenon-side line above is a prediction. Booting a stick and printing the tour is the
  only part of this xenon has done before (2026-09-05 and 2026-09-17).
