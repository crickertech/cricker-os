---
status: BUILT
raised: 2026-09-01
built: 2026-09-16
---
# 218. Every boot of the VisionFive 2 needs a human typing four commands into U-Boot

Minted 2026-09-01 by the maintainer, after driving the board from a
script made the cost of the manual path concrete. The artifact shipped 2026-09-02 and **sat
unconfirmed for fourteen days for want of a powered board**; the boot that confirmed it happened on
2026-09-16. *(Number provisional until the merge queue lands it.)*

## The boot, 2026-09-16

Transcript: `bench/radon-2026-09-16/tour-083200.log`. The countdown expired with nobody typing:

```
Hit any key to stop autoboot:  2   1   0
Scanning mmc 1:1...
Found U-Boot script /boot.scr.uimg
1595 bytes read in 8 ms (194.3 KiB/s)
## Executing script at 43900000
nife: boot.scr is driving this boot, milestones 218 and 257
```

and ran through to `nife: the capability core runs on RISC-V` and the progenitor handoff. That is
this milestone's one phase, and nothing about it needed a person.

**What the block predicted and what it got right.** The 2026-09-02 lane's diagnosis held up
unchanged: U-Boot's distro boot scans extlinux first and boot scripts second, so a card carrying
`boot.scr.uimg` and no `extlinux.conf` runs our script. The `### ERROR ### Please RESET the board
###` hang is gone because the condition that caused it is gone, not because anything worked around
it.

**One thing it did not predict, and it is the reason this took a second reading to confirm.** The
same boot exercised milestone 257's network-first path, which **failed**:

```
nife: tftp server is 192.168.8.206, setenv nife_boot_server to point somewhere else
ethernet@16030000 Waiting for PHY auto negotiation to complete......... TIMEOUT !
...
nife: payload came from card
```

That is a dead link (no cable, or no link partner), not a defect: the same board negotiated and took
a DHCP lease on 2026-09-04. It is recorded here rather than only in 257 because it is the first
evidence that the fallback works, which is the property that makes this card safe to leave in the
board. The script tried the network, failed, fell back to the card, said which path it took, and
booted. A boot script that could strand the board on a network problem would not be worth the
hands-free boot it buys.

A route was taken on 2026-09-02 and the board was unreachable to try it on, so the status
did not move and everything below the routes list is that lane's report. Read "What was built
on 2026-09-02" before the paragraphs above it: they describe the state this milestone was
minted in, and two of the three routes are refused there on evidence.

In brief. The board's own autoboot fails. Captured 2026-09-01 on real hardware:

```
Found /extlinux/extlinux.conf
Retrieving file: /nife-vf2.img
225280 bytes read in 31 ms (6.9 MiB/s)
Moving Image from 0x40200000 to 0x80200000, end=802ff000
Device tree not found or missing FDT support
### ERROR ### Please RESET the board ###
```

So the only path that reaches nife is the manual one: interrupt the two-second countdown and type
four commands (five with the archive). `script/board-image` prints them and
`notes/visionfive2.md` explains why: with no `fdt` line in the extlinux label, U-Boot falls back to
`fdt_addr_r`, then `fdt_addr`, then `$fdtcontroladdr`, and those addresses land outside the kernel's
boot page table. The manual path exists to control the DTB address exactly.

## Why this is worth a milestone rather than a note

It is the difference between a bench session and a test target. A boot that needs a person at a
keyboard cannot be repeated overnight, and `design/fatal-risks/README.md` risk 5 (it cannot be made
reliable on multicore, and the bugs appear only on silicon) names *sustained* stress as its decisive
experiment. Sustained is exactly what the manual path forecloses.

It was proved on 2026-09-01 that a script can drive the manual path over the serial console, so this
is not a blocker to automation today. But a rig that works by typing at U-Boot's prompt is
recreating in a script what the boot loader is supposed to do, and it fails differently and worse:
it depends on catching a two-second window.

## The routes, as they were minted (one was taken; see the next section)

- An `fdt` line in `extlinux.conf`, if U-Boot will then load the DTB somewhere the kernel's boot
  map covers. Cheapest if it works; needs checking against what `fdt_addr_r` actually is on this
  board rather than assumed.
- Repair the U-Boot environment and set `fdt_addr_r`. The board reports
  `* Warning - bad CRC, using default environment` on every boot, so the environment in SPI flash
  is already degraded. This route fixes that as a side effect and is persistent, but it writes to
  the board's flash, which is the least reversible thing on this list.
- **Widen the kernel's boot page table so the fallback address is inside it.
  `notes/visionfive2.md` refers to this as the gigapage-1 fix, which makes it the route the tree has
  already been contemplating; it is also the only one that fixes the class rather than this board.

## What was built on 2026-09-02, and why the status did not move

**A lane replaced the extlinux path with a U-Boot script on the card and could not power the
board on**, which is the whole reason this section exists rather than a status word. radon was
down, its Kasa plug unreachable from the development Mac, and its serial console needs a person.

`NOT-STARTED` is wrong about the artifact and right about the outcome, and that is the honest way
to leave it. `PARTIAL` means some phase shipped and more remains, and this milestone has exactly
one phase, which is that a boot happens with nobody typing. No such boot has happened. Nor is
there anywhere else it could have: `PARTIAL` earns itself elsewhere in this tree by running end to
end in QEMU first, and QEMU's `virt` machine has no U-Boot, no SD card and no distro boot, so
there is no rehearsal available. The token stays `NOT-STARTED` and this paragraph is what the
reader should believe instead of it.

That was true until 2026-09-16 and is kept as the account it is. The boot happened, on the
artifact this section describes and with no change to it, and the status is `BUILT`. What the
paragraph got right is worth keeping visible: it refused to call a shipped artifact `PARTIAL` when
the one phase that defines the milestone had not occurred, and fourteen days later the only thing
that had to change was that somebody powered the board on.

### The route, and what killed the other two

The captured failure decides more than it looks like it does. Read it again with the last two
lines in mind:

```
Retrieving file: /nife-vf2.img
225280 bytes read in 31 ms (6.9 MiB/s)
Moving Image from 0x40200000 to 0x80200000, end=802ff000
Device tree not found or missing FDT support
### ERROR ### Please RESET the board ###
```

U-Boot loaded the image, relocated it, and then stopped without jumping to it. The message is
RISC-V's `boot_prep_linux` refusing a `bootm` that carries no device tree at all, and the
`### ERROR ###` is its `hang()`, which only the reset button clears. So this is not the boot-map
caveat arriving as a firmware error, which is how `notes/visionfive2.md` first read it: no
instruction of ours ran, and the kernel's page table was never consulted.

That kills the **gigapage route** outright, and cheaply. Widening the kernel's boot map cannot
affect a failure that happens before the kernel exists, and the widening this block pointed at had
in any case already landed on 2026-08-14: gigapage 1 covers 0x4000_0000..0x8000_0000, which is
where `fdt_addr_r` would have put a tree. The route is not merely unnecessary here, it was
addressing a problem the board did not have.

It also makes the flash route unnecessary, which is the one that mattered most to refuse.
Repairing the environment in SPI flash would have set `fdt_addr_r` on a board this project owns
exactly one of, and AGENTS.md's test for that class of decision is not "can I revert the commit"
but "who else has already acted on it". Nothing needed it: everything below is a file on a card,
and a card can be rewritten with `cp`.

The `fdt` line in `extlinux.conf` was the cheap candidate and lost on a supply problem rather
than a design one. It needs a device tree *file* on the card, and this repository does not have
one: `crates/machine_discovery/tests/fixtures/visionfive2-uboot-control.dtb` is 1,699 bytes,
trimmed down from the real capture for a host fixture, against the ~54 KB tree the board actually
boots with. Shipping a snapshot would also be a step down from what every successful board boot
so far has used, which is U-Boot's own live control DTB, the tree whose `riscv,plic0` spelling
already cost this project a bench cycle to discover.

**What shipped instead is `boot.scr.uimg`, and its argument is that it is not new.** U-Boot's
distro boot scans for extlinux first and boot scripts second, so the card now carries a script and
no `extlinux.conf`. The script issues the sequence from the *successful* capture of the same day,
line for line, with two edits that each remove a dependency: the load device comes from the
variables distro boot has already set for the script it is sourcing, and the archive's length is
stashed under a name of ours the moment `load` reports it, so nothing later that touches
`filesize` can change what `booti` is handed. `cargo xtask board-script` writes it and
`target/board/boot.cmd` is the same text in readable form.

`mkimage` is not a host package this project needs anywhere else, and a build step that works on
the machine that has it and fails on the machine that does not is the third principle's newcomer
trap exactly. The legacy U-Boot header is 64 bytes and two CRCs, so it is written in `xtask`
against `gpt`'s CRC-32, which is the tree's one definition of that checksum and is Kani-proved
against its own bitwise form.

### What is proven, on the host, and it is only the format

`cargo test -p xtask` reads the produced image back the way `source` does: magic, the type field
`source` actually checks, the payload's size table, and both CRCs recomputed with the header's own
field zeroed. A second test guards the script's vocabulary rather than its outcome, because
U-Boot's parser is a cut-down hush whose failures are silent: no comments, no shell punctuation,
and no verb the bench transcript does not already show working.

Two independent readers agree with the writer. `file(1)`, which has never heard of this project,
reports `u-boot legacy uImage, nife board boot, Linux/RISC-V, Script File (Not compressed)`, and
Python's `zlib.crc32` reproduces both stored CRCs. That is a real check on the format and no
check at all on the claim this milestone makes.

### The bench procedure, in order

Everything above is reasoning. This is what settles it. Steps 1 and 2 need no board.

1. Build and write a card. Format it once by hand, then:

   ```
   script/board-image --card /Volumes/NIFE
   ```

   Expect three files copied as a set and, if a previous nife card is being reused, a line saying
   the stale `extlinux/extlinux.conf` was removed. *If it refuses*: the path is not a mounted
   directory. *If it copies but prints no removal line on a reused card*, look at what is in
   `extlinux/` by hand; a leftover config there boots first and hangs the board.

2. Read the script that will run, so step 4's transcript is being compared against something:
   `cat target/board/boot.cmd`.

3. Attach the console before power, since the SPL banner is gone within a second:

   ```
   script/board-console --until banner --for 120s --log target/board/boot.log
   ```

4. Power the board and type nothing. The expected transcript, in order: the SPL banner,
   OpenSBI v1.2, U-Boot 2021.10, the `Invalid partition 3` noise this board always prints, the
   autoboot countdown running out, `Scanning mmc 1:1...`, `Found U-Boot script /boot.scr.uimg`,
   then `nife: boot.scr is driving this boot, milestone 218`, which is the first line that
   exists only because of this milestone. Then two `load` lines, `## Flattened Device Tree blob at
   86000000`, `Starting kernel ...`, and the kernel's own banner.

   The failure modes, and what each one means:

   - **`Found /extlinux/extlinux.conf`, then `Device tree not found`, then `### ERROR ###`.** The
     old config is still on the card. Step 1 did not remove it, or the card is not the one written.
   - No `Found U-Boot script` line at all, and the board sits at `StarFive #`. Either this
     vendor U-Boot's `scan_dev_for_boot` does not scan for scripts, or `scriptaddr` is unset in the
     default environment it fell back to. Read `printenv scriptaddr` and `printenv fdt_addr_r`
     and record both: they are the two facts this lane could not get, and the second one also
     explains the original extlinux failure.
   - `Found U-Boot script`, then `SCRIPT FAILED: continuing...`. The image was rejected or a
     line did not parse. `iminfo ${scriptaddr}` says whether the header and CRCs survived the card;
     if they did, the parser is the suspect and the offending line is the last one echoed.
   - The echo appears and then a `load` fails. A file is missing or misnamed on the card;
     `ls mmc 1:1 /` names what is actually there.
   - The kernel banner appears and the tour then halts at `MEASURED BOOT REFUSED`. The pair on
     the card is mismatched, which is milestone 217, not this one, and means step 1 was not the
     source of those files.

5. Once it boots hands-free, prove it repeats, because that is the property fatal risk 5 wants
   and a single boot does not show it: power-cycle three times without touching the keyboard, and
   then run a soak card (`script/board-image --soak --card ...`) long enough to leave the room.

## BUGS

- Nothing here has run on the board. The status section above says why, and the procedure above
  is what would change it.
- Three facts about this vendor U-Boot are still assumed: that its distro boot scans for boot
  scripts, that `scriptaddr` is set in the default environment, and that its parser takes the seven
  lines as written. Every one of those failures leaves the board at the `StarFive #` prompt rather
  than hung, which is already better than what it replaced, and the manual commands still work from
  there.
- **The `Invalid partition 3` complaints are not explained here.** Every boot prints
  `** Invalid partition 3 ** / Couldn't find partition mmc 1:3` several times before finding
  `mmc 1:1`. Harmless so far, unexamined, and recorded so nobody reads them as caused by whatever
  this milestone changes.
- **The environment is still degraded and this does not repair it.** `bad CRC, using default
  environment` on every boot is untouched, deliberately: the repair writes to the SPI flash of the
  only board of its kind this project owns, and nothing here needs it. It stays available if the
  script route turns out not to work.
- A boot script is a second thing that can be stale on a card. It changes far less often than
  the kernel and the archive, and `--card` rewrites all three every time, but a card written by
  hand can now be wrong in one more way.

## Follow-on

- **Done.** The confirming boot, radon 2026-09-16, transcript `bench/radon-2026-09-16/tour-083200.log`.
- **Recorded.** The same boot found milestone 257's network-first path failing at
  `Waiting for PHY auto negotiation to complete......... TIMEOUT !`, a dead link rather than a
  defect, and the limitation is recorded beside the feature in this block and in
  `notes/visionfive2.md`'s bench runbook. What it proves is the fallback: the script tried the
  network, failed, fell back to the card, named the path it took and booted.
- **Milestone 306.** The measurement this boot also carried, the hw-entropy rate, closing fatal risk
  6's third half.
- **Recorded.** A hands-free boot is necessary for the sustained overnight runs
  `design/fatal-risks/README.md` risk 5 names as its decisive experiment, and is nowhere near sufficient:
  power cycling is still manual by calef's own 2026-09-04 ruling in milestone 224, so a wedged board
  still ends a run. That limitation lives beside the decision, in milestone 224's block.

## Index row

the board cannot boot unattended and fatal risk 5 wants sustained runs. A route was taken
2026-09-02 and radon was unreachable to try it on, so the token still says what the outcome is
rather than what was built: the card now carries a U-Boot script issuing the same sequence the
successful bench capture proves, and no `extlinux.conf`, which U-Boot hangs on. The captured
failure refuted the premise this row used to state: U-Boot handed `bootm` no device tree at all,
so nothing of ours ran and the DTB address was never reached, which retires the gigapage route and
makes the SPI-flash one unnecessary. The block carries the ordered bench procedure and what each
failure would mean
