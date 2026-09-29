# Fatal risk 6's bench evening on xenon, and how to read its verdict

*(Milestone 261 (the NVMe driver leaves the kernel). Names in this page are provisional, per the naming tenet; calef names things.
`disk_throughput`, `cargo xtask disk-throughput`, `DmarUnits`, `Scope` and this page's stem among
them.)*

This page was written on 2026-09-24, before any boot of the instrument it describes. Nothing
here has touched xenon. Everything is either code in this tree, built, host-tested and rehearsed
under QEMU with OVMF and `-device intel-iommu`, or it is a question for the bench, marked as one. The
Results table is empty on purpose: the outcomes below were written before the numbers exist, so the
numbers cannot choose their own interpretation.

## What the evening is for, in one paragraph

`design/fatal-risks/README.md` risk 6 asks whether a capability-confined userspace driver can drive real
hardware at real speed. Its decisive experiment is one real, non-virtio device on real silicon,
confined, at throughput. Milestone 261 built the driver: an EL0 process holding one page of the
NVMe's BAR0 and a confined DMA window serves the block verbs. xenon has the device (a Micron 2450
behind a PCIe root port) and a VT-d IOMMU, and calef wiped its disk on 2026-09-17. What was missing
was a boot that measures, and a way to know on the night whether the measurement means what risk 6
needs it to mean. That boot is the `disk_throughput` kernel feature.

## The two night-of conditions, and what changed so neither can pass silently

The risk entry names two things that must hold on the night and that no code can make true.

**1. The DMAR's device scope must cover the NVMe function.** A VT-d unit translates only the
requesters the firmware's DMAR gives it. Until this page, the kernel brought up the DMAR's *first*
unit and reported the device confined whenever any unit was translating (`confined_by_iommu` was
`iommu::is_active()`). That is the wrong question on a client Intel machine. On the Skylake OptiPlex
7040, the same family as xenon's 7050 with the same register addresses, Linux prints:

```text
DMAR: DRHD base: 0x000000fed90000 flags: 0x0
DMAR: DRHD base: 0x000000fed91000 flags: 0x1
```

Read on 2026-09-24 from <http://linux-hardware.org/index.php?probe=a94acdc2f3&log=dmesg>. Flags
`0x0` with a scope naming only the integrated GPU, then the catch-all (`INCLUDE_PCI_ALL`). xenon's
DMAR is 204 bytes and its first unit is `0xfed90000`
(`bench/xenon-2026-09-17/tour-display-225100.log`), which is the size two units plus RMRRs take.
If xenon matches its sibling, every earlier xenon boot translated the graphics unit, and the NVMe
test would have asserted "confined" about a device nothing was translating. That is inference
from a sibling machine, not a reading of xenon; the preflight below is the reading.

What changed:

- `machine_discovery::acpi::DmarUnits` decodes every DRHD and PCI device scope, and answers which
  unit owns a function by VT-d 3.x section 8.3's rule (an explicit scope, then a bridge's
  sub-hierarchy, then the catch-all). A table it could not fully record answers "unknown", never
  "the catch-all".
- Since milestone 594 (every VT-d unit translates its own devices), the kernel brings up every unit
  and routes each device to its owner. The RMRRs are identity-mapped first. On QEMU's `q35` there
  is one unit and no RMRR, and nothing changes.
- `iommu::scope_of(rid)` asks the DMAR whether a unit that is up owns the requester.
  `confined_by_iommu` is that answer, so the ordinary NVMe boot test now fails on such a machine
  instead of passing.

**2. The LBA size must give `blocks_per` in `1..=8`.** The driver moves 4096-byte blocks, so the
namespace must be formatted with 512, 1024, 2048 or 4096-byte LBAs. Before this page a refused
namespace and an absent controller were both `None`, and the boot test printed both as `skipped: no
NVMe controller came up`. That is exactly what xenon's second attempt on 2026-09-17 printed about a
disk that was there. Now the refusal carries the LBA size, the test fails on a present-but-refused
controller, and the bench boot prints the size either way.

## What the boot prints

After the ordinary tour (unchanged, so its `vt-d` lines are still evidence), every line prefixed
`disk-throughput:` so a photograph and a capture read the same:

```text
disk-throughput: fatal risk 6 bench boot (milestone 261). WRITES to the NVMe disk.
disk-throughput: preflight first; a skip is not a pass. release build, 2713000000 Hz counter.
disk-throughput: measured    non_volatile_memory_express sha256 f62871988f413c27.. matches the table this kernel vouches for
disk-throughput: preflight 1/2 dmar scope : PASS  nvme 01:00.0 (rid 0x0100): drhd 0xfed91000 is the catch-all and no other unit names it
disk-throughput: preflight 2/2 lba size   : PASS  512-byte lbas, blocks_per 8 (needs 1..=8); namespace N bytes
disk-throughput: ipc floor   2000 SIZE round trips in N us (N ns each, no device)
disk-throughput: write       16384 x 4096 B in N us = N B/s (N ns per block)
disk-throughput: flush       one FLUSH in N us
disk-throughput: read        16384 x 4096 B in N us = N B/s (N ns per block)
disk-throughput: verified    16384 of 16384 blocks came back stamped with their own number (window 256..16640 in 4096-byte blocks)
disk-throughput: verdict CONFINED-AT-RATE: read N B/s, write N B/s, EL0 driver; drhd 0xfed91000 is the catch-all and no other unit names it
disk-throughput: done, halting.
```

The unit address and the requester id above are predictions, not readings: `0xfed91000` is the
7040's catch-all, and `01:00.0` assumes the root port's secondary bus is 1. The line shapes are exact; every one of them has been printed under QEMU.

What a figure counts. One command in flight, 4096 bytes per command, polled completion, and two
context switches per block, because the client is the kernel's boot thread `CALL`ing the EL0 server
once per block. That is a lower bound on the device and an honest measure of this driver. It is not
comparable to `fio` at queue depth 32, and the `ipc floor` line is there so whoever quotes a figure
can subtract the IPC share. The server's measurement is checked against the archive's table before
it runs, because this boot replaces the hand-over that would otherwise do that check.

## Rehearsing it, which needs no machine

```sh
cargo xtask disk-throughput                  # all four cases, about a minute on patagonia once built
cargo xtask disk-throughput --case bypass    # one of them
```

Each case boots the release image under OVMF with `-device intel-iommu`, and each has the verdict it
must produce. The command fails if any produces another, or if a verdict line appears without a
preflight above it.

| case | machine | must print |
|---|---|---|
| `root-port` | NVMe behind a PCIe root port, 512-byte LBAs: xenon's shape | both PASS, owner by bridge scope, `CONFINED-AT-RATE` |
| `lba-4096` | 4096-byte LBAs | both PASS, `blocks_per 1`, `CONFINED-AT-RATE` |
| `lba-8192` | 8192-byte LBAs | lba FAIL, `SKIPPED` |
| `bypass` | `default_bus_bypass_iommu=on`: the DMAR names no unit for the NVMe | scope FAIL, `UNCONFINED` |

Rehearsed 2026-09-24 on patagonia, all four as expected. QEMU's DMAR has one unit with flags `0x0`
and explicit scopes, so `root-port` exercises the bridge path and `lba-4096` the named-endpoint
path. The catch-all path is exercised by host tests only (`cargo test -p machine_discovery`), since
QEMU cannot present two units. QEMU's rates (2 to 17 MB/s under TCG across five runs on 2026-09-24,
moving with host load) are a statement about TCG, the same warning `notes/job-mix.md` gives.

## The evening, start to finish

What it needs from calef: time at xenon, about thirty minutes for three boots and more if a
preflight fails. A monitor and a USB keyboard on it (it halts at POST without one, per
`notes/x86-uefi-boot.md`), the FAT32 stick, and a phone for the photographs. Nothing new to buy or
plug in. The disk was wiped on 2026-09-17; **this boot writes 64 MiB from 1 MiB in**, so if anything
has been put on that disk since, stop here.

### 0. Before power

- `pgrep -l qemu` on patagonia: nothing should be running while the stick is built.
- On xenon, nothing to change in firmware. `VT for Direct I/O` is already enabled
  (`notes/xenon-firmware.md`).

### 1. Build the image once, and note the commit

```sh
git log -1 --format=%h                        # goes in the Results row
cargo xtask disk-throughput --stage-only
cp target/esp-disk-throughput/EFI/BOOT/BOOTX64.EFI /Volumes/NIFE/EFI/BOOT/BOOTX64.EFI
diskutil eject /Volumes/NIFE
```

`target/esp-disk-throughput`, not `target/esp`, and the difference is the point: this kernel writes
to the disk and the ordinary stick must never be it. A netboot (`script/board-netboot --root
target/esp-disk-throughput`) serves the same file if milestone 260 (boot xenon over the network)'s two firmware settings are in.

### 2. Boot, and photograph the last screen

Power on with the stick in, F12 if the firmware does not pick it. The tour scrolls, then the
`disk-throughput:` block ends the boot and stays on screen. Photograph the whole screen, not
just the verdict: the `vt-d` lines above the block are the other half of preflight 1. The
measurement takes seconds on silicon; `done, halting.` is the end.

Watch the screen as the `vt-d        : drhd ... up` lines print. That is when the graphics unit
starts translating, and the display engine keeps working only if the graphics RMRR covers the
memory it scans. Three things can happen:

- The tour carries on as before. The RMRR covers the scanout, and the graphics unit is proven.
- The screen goes black, freezes or tears at that line. The graphics unit is faulting the display.
  Photograph the last readable screen, which should show the `vt-d rmrr` lines. Then rebuild
  from the tree as it was before milestone 594, which translates only the catch-all, and run the
  evening on that: `git checkout "$(git log --merges --grep=milestone/594 --format=%h -1)^1"`.
- The machine resets or hangs with the screen intact. A unit's register writes did not take, and
  the kernel panicked on the screen it cannot redraw. Photograph it; it names the unit.

### 3. Read the two preflight lines before the numbers

Both must read `PASS` for the verdict to be `CONFINED-AT-RATE`. The kernel enforces this; the
reason to read them first is that a FAIL is the evening's most important result and must not be
skimmed past on the way to a number.

### 4. Boots: three, power-cycling between

Three because each boot is one sequential pass with no warm-up. Record all three; quote the
median and the spread, never one boot.

### 5. Optional: the denominator, if a Linux live stick is to hand

The honest comparison is the same disk, the same window and the same shape under Linux:

```sh
sudo fio --name=r --filename=/dev/nvme0n1 --direct=1 --rw=read --bs=4k --iodepth=1 \
    --offset=1M --size=64M --ioengine=psync
sudo fio --name=w --filename=/dev/nvme0n1 --direct=1 --rw=write --bs=4k --iodepth=1 \
    --offset=1M --size=64M --ioengine=psync
```

Queue depth 1, 4 KiB, O_DIRECT, one pass: the shape nife's driver has. Without it, the verdict
answers "confined, at a measured rate" and leaves "at *real* speed" to a judgement. With it, the
ratio is the number. **Never quote nife's figure against a queue-depth-32 Linux figure.**

### 6. Record it

Transcribe the `disk-throughput:` block from each photograph into
`bench/xenon-<date>/disk-throughput-boot<N>.log`, commit the photographs beside them, and fill the
Results row below. The verdict goes into risk 6's appendix
(`design/fatal-risks/the-confined-driver.md`) by whoever holds `design/`, with this page's caveats
attached.

## What each outcome means

| What the photograph shows | What it means | Where it routes |
|---|---|---|
| `verdict CONFINED-AT-RATE` | a confined EL0 driver moved verified blocks on real silicon at the stated rate | risk 6's decisive experiment ran. Record it. Whether the rate is "real speed" is the Linux ratio's question (step 5), not this line's |
| preflight 1 `FAIL ... drhd X owns it, but it did not come up` | the unit that owns the NVMe was refused | the numbers below it are labelled `UNCONFINED` and are not risk 6's answer. The `vt-d ... NOT up` line gives the reason. The fix is a kernel change and a lane, not a bench step |
| preflight 1 `FAIL ... no drhd owns it` | no unit's scope names the NVMe and there is no catch-all | same routing. It would also mean firmware leaves the NVMe untranslated under any OS, which is itself worth writing down |
| preflight 1 `FAIL ... the dmar did not fit` | the DMAR held more DRHDs or scopes than `DmarUnits` records | raise `MAX_DRHDS`/`MAX_SCOPES` in `crates/machine_discovery`; the photograph of the tour's `vt-d` lines says by how much |
| preflight 2 `FAIL  N-byte lbas` and `verdict SKIPPED` | the Micron is formatted with an LBA size this driver cannot serve | a skip, not a pass. Reformatting the namespace (`nvme format` from a Linux live stick, LBA format 0) is calef's call; so is teaching the driver PRP lists for larger LBAs |
| `verdict SKIPPED: no NVMe controller on the bus` | the bus walk found no NVMe class code | the 2026-09-17 root-port fix regressed, or the firmware hid the device (SATA mode `RAID On` would) |
| `verdict FAILED: ...` with a controller or server step | bring-up or the server failed on real hardware | the line names the phase. `ControllerTimeout` or `CompletionTimeout` on silicon but not under QEMU is the finding; photograph and stop |
| `verified N of M` with N < M | blocks came back from the wrong place or not at all | a correctness failure, worse than any rate. The verdict is `FAILED`; stop |
| `MEASURED BOOT REFUSED` | the stick carries a kernel and an archive from different builds | rebuild with `cargo xtask disk-throughput --stage-only`, which packs them in order |
| the tour, then nothing after `vt-d` or the scheduler | a release-only defect, or the catch-all unit's default-deny broke something the firmware left running | rebuild with `--debug` and boot again; if that boots, the release build is the finding. If neither does, see this page's `BUGS` on RMRRs |

## Results

| Date (UTC) | Commit | Boots | Preflight 1 | Preflight 2 | Read B/s | Write B/s | IPC floor | Linux qd1 read/write | Notes |
|---|---|---|---|---|---|---|---|---|---|
| | | | | | | | | | |

## What this cannot settle, said plainly

It does not measure the device at its rated speed. One command in flight and polled completion
cap it well below what the Micron can do at depth; the `components/src/non_volatile_memory_express.rs`
`BUGS` section says why the server is built that way. A good number here is "this driver, confined,
on silicon"; a bad one may be the driver's shape rather than confinement's cost, and the Linux qd1
ratio is what separates them.

It confines DMA, not interrupts. The server polls and holds no interrupt capability, so interrupt
remapping (off in this kernel, offered by xenon's unit) is not on this experiment's path.

## BUGS

- The two-unit route and the RMRR maps have never run against a real DMAR. Host tests over the
  7040's table cover them; QEMU presents one unit and no RMRR. The first xenon boot is their
  first real input, and the screen is what shows whether the graphics RMRR is enough.
- The domain-id width, the `GCMD` read-modify-write and the protected-memory switch-off are code
  paths QEMU does not reach. Milestone 594's block says why each matters on xenon.
- One pass per boot, no warm-up. A second boot is the repeat, and three is this page's floor.
- The client is in the kernel. A client process would add its own context switches; the server,
  which is what risk 6 is about, is the same process either way.
