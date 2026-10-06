---
status: PARTIAL
raised: 2026-09-02
milestone_dependencies: none
decision_dependencies: none
machine_requirements: aarch64, riscv64 and x86_64 silicon
specific_machine: none
needs_person: yes
---
# 225. Run the soak on radon, argon and xenon, which is the only place its answer means anything

Partial as of (2026-09-25). radon has run it, clean, for 8 h 09 m; argon and xenon have
not. `PARTIAL` rather than `BUILT` because the block names three machines and one is done. Minted
2026-09-02 by the maintainer, carrying forward the follow-on that
milestones 219 (the boot tour ends and the kernel halts, so there is nothing to soak) and 221 (the
soak never crosses cores, so build the hook that makes it) both proposed. *(Number provisional until
the merge queue lands it.)*

In milestone 53's sense: the boards are on the desk and this needs hands on
them. Nothing else blocks, and nothing more can be built for it.

In brief. Fatal risk 5's entire premise is that the defects appear only on silicon, and as of
2026-09-23 that premise has zero confirmed instances. This block said until then that radon had
produced one, a receiver woken with nothing delivered on three harts that no emulator run had shown.
**That reading was retracted on 2026-08-15**, the day after it was recorded and two weeks before this
block was written, by `notes/visionfive2.md`'s fifth bench stop: the dumps are the terminal state of
a completed tour, identified five independent ways. Every multicore defect this project has found was
found without silicon, including both x86_64 `ap_boot` bugs, which were found under QEMU TCG.

That strengthens the case for running this, rather than weakening it. A premise with no instances
is untested, not disproved, and this milestone is the experiment that would test it.

Everything needed to run it now exists, and none of it existed on 2026-09-01:

- A workload that lasts (milestone 219 (the boot tour ends and the kernel halts, so there is nothing
  to soak)), with a heartbeat on the wall clock so a crawling machine
  still reports on time.
- A hook that makes it cross cores (milestone 221 (the soak never crosses cores, so build the hook
  that makes it)), on the real `irq_notify` to `wake_load_aware`
  path. That path was once read as where a radon defect lived; the reading is retracted (the fifth
  bench stop in `notes/visionfive2.md`, 2026-08-15), so it is the path worth stressing, not the site
  of a known defect.
- A console that watches and judges (milestone 216 (nothing in this tree can read a board)), with a sustained mode and a stage that
  re-arms the quiet check a completed boot tour suppresses.
- A boot that needs nobody typing (milestone 218 (every boot of the VisionFive 2 needs a human
  typing four commands into U-Boot)), unconfirmed on the board itself.

## What it needs

Bench evenings, one per machine, and the discipline to read the first heartbeat before walking
away. Milestone 221's procedure is explicit about this and it is the part most likely to be
skipped: `wakerate` should be about `100 * harts`, and `crossings` must be rising between beats
rather than frozen. Eight hours of a non-crossing soak is eight hours of milestone 219's experiment
rather than 221's, and the difference is invisible afterwards.

Record `rounds`, `rate`, `wakes` and `crossings` for every run, in `notes/soak.md`'s table.

## What an answer would and would not be

A clean run licenses one sentence, which milestone 219's tooling prints on every green result:
this machine did N cross-core round trips without the wake gate refusing one, without a wrong reply,
and without a worker stalling. It is not proof the concurrency is correct, and the risk's own text is
honest that this class of question "produces a confidence rather than a verdict".

A failure is worth far more, and is the outcome to hope for. It would be the first confirmed defect
this risk has produced, and the first found by an instrument rather than by somebody watching a bench.

## radon, 2026-09-25: clean, 8 h 09 m, 4.1 million crossings

One boot, netbooted, built at `9e879f1e7`, watched by `script/board-console --for 490m --until
none` to its deadline (exit 0). First beat checked before calef left: `wakerate=430/s` settling to
403, `crossings` 747 then 1,445 then rising about 140 a second. Last beat:

| rounds | rate | wakes | crossings | refused / mismatch / stalled |
|---|---|---|---|---|
| 10,193,815,048 | 350,753/s | 11,747,350 (404/s) | 4,108,581 | 0 / 0 / 0 |

radon did 4.1 million cross-core thread handoffs and 10.2 billion IPC round trips over 8.16 hours
without the wake gate refusing a wake, without a wrong reply, and without a worker stalling. That
is the sentence and all of it. No red means the QEMU cross-check a red would have needed never
arose. The account, the anomalies (none of them a failure) and what it does and does not rule out
are `notes/visionfive2.md`'s "The eight-hour soak, 2026-09-25"; the log is
`bench/radon-2026-09-25/soak-8h.log`; the exposure row is E4 in `notes/multicore-defect-curve.md`.

Why eight hours: This block prescribes no duration, so the lane proposed one against crossings
rather than clock time, per `notes/soak.md`'s duration section. On a fast draw 8 hours is 1.4 to 5.4
million crossings, against 5,507 in the only earlier multi-hour run. Past that, a second boot buys
a new draw of the placement lottery, which is worth more than a ninth hour. The maintainer approved
it. What remains on radon is more boots, not longer ones.

## argon, 2026-10-05: not yet a one-command run, and exactly why

calef ruled argon first on 2026-10-05. This lane set out to make its soak one command and found
the premise false one step earlier than this block's `BUGS` says: argon has never booted nife,
**and as built it cannot**. The aarch64 kernel is linked, mapped and consoled for QEMU `virt`
only (RAM at 0x4000_0000, a PL011 at 0x0900_0000), and tegra210 puts DRAM at 0x8000_0000 and a
16550 at 0x7000_6000. A `booti` today would be silent, and that silence would be misread at the
bench as cabling or firmware. Milestone 127's two named prerequisites are built; the third, the
board memory map, is unbuilt and unowned.
`design/roadmap/proposals/argon-boots-the-aarch64-kernel.md` is that work, with the fork (one
binary or two) that is the architect's.

What this lane built instead, which every board's soak uses: `script/board-console --exposure
<log> --machine <name> --build <sha> --start '<utc>'` reads a capture into the curve's exposure row
in `notes/multicore-defect-curve.md`, with the four figures above and milestone 221's two bench
checks redone after the fact. Its test reads radon's eight-hour log back into E4.

### What calef can do at argon today, with no nife image (about 20 minutes)

These are milestone 127's steps 1 and 2 and its USB question, and they need nothing built. They
answer the proposal's one open fact (the DRAM base) and capture the prologue `script/board-console`
needs before it will accept `--board argon`.

1. Cable: a 3.3 V USB-TTL adapter on argon's J21 header (TX, RX, GND; adapter VCC unconnected), on
   patagonia. Check nobody holds the port: `lsof /dev/cu.usbserial-*`.
2. `screen /dev/cu.usbserial-* 115200`, then `Ctrl-a H` to log to `screenlog.0`.
3. Power argon with no SD card. Expected: U-Boot's banner and an autoboot countdown. Silence is the
   cable, the baud, or an L4T too old to reach U-Boot; stop there and say which.
4. Press a key to stop autoboot, then type: `bdinfo`, `printenv fdt_addr_r kernel_addr_r`, and,
   with a FAT32 stick in, `usb start`, `usb storage`, `fatls usb 0:1 /`, `help bootefi`,
   `printenv boot_targets`.
5. `Ctrl-a k` to quit. Hand the lane `screenlog.0`; it goes to `bench/argon-<date>/uboot.log`.

Red here is any of: no U-Boot banner, a DRAM bank in `bdinfo` that is not at 0x8000_0000, or a
hang on `usb start`. Each is a fact for 127, not a failure of this milestone.

### The soak itself, once argon prints the banner

Unchanged from radon's procedure (`notes/soak.md`, "On radon at a bench"), with argon's names. The
two commands marked *not yet* are the proposal's items 4 and the profile.

1. *Not yet:* `script/board-image --soak` for argon, onto the SD card.
2. *Not yet:* `script/board-console --board argon --for 8h --until none --log
   target/argon-soak-$(date +%s).log`, started before power-on.
3. Read the first beat: `wakerate` about 400 (four A57s at 100 Hz) and `crossings` rising.
4. Afterwards: `script/board-console --exposure target/argon-soak-<stamp>.log --machine argon
   --build <sha> --start '<utc>'`. Exit 0 is clean, 1 is a failure to classify.

Eight hours, for radon's reason. Red is `soak-test: FAILED`, a `[PANIC]`, three missed beats, or
`--exposure` exiting 1. Any of them is the outcome worth hoping for: the first defect on silicon.

## BUGS

- **No duration is prescribed**, because nobody knows what would be persuasive, and milestone 219's
  block says the same thing for the same reason. The radon run chose 8 hours against a crossing
  count and says why above; that is a choice, not a standard.
- One radon boot is one draw. It drew the fastest arrangement seen so far, and a slow draw
  crosses about 275 times less often, so the clean result says little about slow arrangements.
- A hung board needs a person, since nothing can power-cycle radon remotely (milestone 224) and
  `script/board-console` reads without writing.
- **The crossing count varies by more than 2x between identical runs**, recorded in milestone 221's
  BUGS, so it is not a figure to compare machines on without more care than a single run affords.
- **argon has never booted nife at all**, so its soak sits behind milestone 127 (the seL4 machine)
  rather than beside radon's. Since 2026-10-05 also behind the unbuilt board memory map in
  `design/roadmap/proposals/argon-boots-the-aarch64-kernel.md`: the aarch64 kernel only fits QEMU
  `virt`.

## Follow-on

- **Outstanding.** xenon's soak. It is rank 2 in `briefs/bench-session.md`'s ready list, behind
  milestone 261 (the NVMe driver leaves the kernel, on the machine that can finally confine it).
  Checked 2026-09-25 against that list.
- **Outstanding.** argon's soak, behind milestone 127 (the seL4 machine), since argon has never
  booted nife. Checked 2026-09-25: 127 is NOT-STARTED. Checked again 2026-10-05: still
  NOT-STARTED, and behind it the proposal `argon-boots-the-aarch64-kernel.md`, which a lane can
  build without the board.
- **Outstanding.** More radon boots, because one boot is one draw and a slow draw has never been
  soaked for long with this build. Checked 2026-09-25: E4 is the only radon row with a log.
- **Done.** The false `NOT SEALED` on a soak build that cost this run half an hour. Milestone 563
  (a seal check that reads bytes cannot see a check that was dropped) carried it, merged
  2026-09-26: soak builds now seal, and `script/board-image --soak` printed `SEALED` on a rebuild.

## Index row

radon ran it clean on 2026-09-25: 8 h 09 m, 4.1 million cross-core handoffs, no refusal; argon
and xenon have not run it
