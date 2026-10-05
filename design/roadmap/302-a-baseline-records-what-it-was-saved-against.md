---
status: BUILT
raised: 2026-09-16
built: 2026-09-23
---
# 302. A baseline records what it was saved against, and a stale one fails loudly

Corrected 2026-09-23 from `NOT-STARTED`, which it had been since promotion
while two of its three items were already in the tree. Promoted 2026-09-16 from
`design/roadmap/proposals/a-toolchain-bump-that-leaves-the-baselines-stale.md`, which was Decision 2
of the finding milestone 299's lane surfaced (originally
`design/roadmap/proposals/icount-baselines-drift-after-a-toolchain-bump.md`, which is now
milestone 577 (the icount baselines predate the pinned nightly), promoted and superseded in one act
on 2026-09-23, on PR #883). Milestone
300 executed Decision 1, decompose and re-baseline. *(Number provisional until the merge queue lands
it.)*

**No gate, and there was one.** calef ruled on 2026-09-16, which cleared the `DECISION` token this
carried as a proposal; the status above now says the rest.

## What landed, and when

Two commits, six days apart, and the block said `NOT-STARTED` through both of them. That misrecord
is why the block on PR #1122 (the icount baselines predate the pinned nightly) was filed and then
superseded in one act: a reader with only this file could not tell that most of the mechanism existed.

- **2026-09-21, commit `99f13dad`** ("A toolchain bump cannot silently revalue the icount floors"):
  items 2 and 3, plus the toolchain half of item 1. `cargo xtask bench --save` writes a
  `# toolchain:` line read from `rust-toolchain.toml`'s pin rather than from whatever compiler
  happens to be running, and `script/lint` fails when that line and the pin disagree. It has fired
  in anger: PR #1112, the 2026-09-23 nightly bump, went red exactly as designed. A `# qemu:` line
  had arrived earlier by the same argument, and `--check` compares it against the emulator in front
  of it rather than against `.qemu-version`, for the reason [notes/benchmarks.md](../../notes/benchmarks.md) gives: the counter reading a floor
  should be the counter that produced it.
- 2026-09-23, this lane: the rest of item 1, the `# why:` line and the date, which is the
  ledger half and the thing `99f13dad` did not build.

## What this lane built

`--save` now refuses to run without at least one `--why "<reason>"`, and writes one `# why:` line
per reason into the header, above the numbers, alongside a `# date:` line. `--check` prints those
reasons back when a benchmark has moved, because a person reading a red gate in CI scrollback is not
reading the file and is holding exactly the question they answer.

Three choices, with their reasons:

- A flag, and a refusal rather than a default. An environment variable is invisible in the
  scrollback of the person debugging the save, and an optional flag is a field that is empty in
  every save made in a hurry, which is every save that later turns out to matter. The command line
  is strings, so a reason cannot be made unrepresentable the way `InputSpec::Required` made a
  missing `writes_while_reading` unrepresentable; refusing at the boundary is the highest rung this
  one reaches. The cost is real and is the one §190 (must an icount baseline save record why it moved) hesitated over: it changes what a person types
  on every bench evening on every board. The refusal happens before the kernel build and the
  emulator run, so the cost is a second rather than a wasted run.
- **Per file, not per counter, which is where 302 and 415 word it differently.** Milestone 415 item
  2 says "beside the rows that moved" and the format calef ratified here is comment lines in the
  header; this follows the ratified one. It is also the only one that survives a save: the writer
  rebuilds the file from a fixed header plus one `name ticks iters` line per result and preserves
  nothing else, which is how the hand-written `#` correction above riscv64's `rfence_self` was
  destroyed on 2026-09-21. `--why` is repeatable so a save covering several moves records several
  reasons, and a reason about one counter names it in its own prose.
- The three existing files say `unrecorded`, the posture the `# qemu:` line already takes and
  milestone 115 (the names that were ratified, and the ones that were refused) takes for names.
  Their numbers were measured by other lanes on other days and inventing a reason now would be
  worse than the gap.

Why the ratified `# saved:` line is still three lines rather than one. calef ratified
`# saved: <nightly>, qemu <version>, <date>` plus a `# why:` line. `99f13dad` implemented the first
of those decomposed into `# toolchain:` and `# qemu:`, one fact per line, and `script/lint` now
matches `# toolchain:` with an anchored regex. Reuniting them would break that gate to gain nothing
a reader can see, so this adds the missing third field as `# date:` and leaves the decomposition
alone. `--why` and `# date:` are this lane's names and are provisional.

The evidence that made the item worth more than it looked is in
[notes/benchmarks.md](../../notes/benchmarks.md), and is not restated here: riscv64's `rfence_self` has oscillated between
5991 and 6476 across seven consecutive saves under two pins, one lane read the low value as a stale
floor carrying dead margin on 2026-09-22, and another measured it back to the tick four days later.
Two competent lanes, opposite conclusions, and the file could tell neither of them anything.

## What was ruled, and why it is one mechanism rather than two

Two rulings arrived together and turned out to be the same change:

- **Fail loudly**, option 2 below, rather than option 1's automatic re-baseline.
- An attribution ledger: a `--save` records its reason *in the baseline file*, which is rung
  three, a record where the reader already is, instead of rung four, a commit message nobody
  re-reads.

**They are one mechanism, because fail-loudly is not implementable without the ledger.** A check can
only say "these numbers are stale for this nightly" if the file records which nightly produced them,
and nothing does: `bench/baseline-<arch>.txt`'s header names `.qemu-version` as the thing the numbers
are relative to and is silent about the toolchain. So the ledger is not a prerequisite to sequence
first, it is the half that makes the other half possible. Filing them separately would put two lanes
in one header format, which is the collision this tree pays for repeatedly.

## The finding, and why it survives its own correction

`script/toolchain-bump` / `toolchain-bump.yml` raises the pinned nightly and changes only
`rust-toolchain.toml`. It does not re-save `bench/baseline-{aarch64,riscv64,x86_64}.txt`, on the
theory that a new nightly's codegen may move the deterministic icount counts the baselines gate
against. When the bump moves the numbers and the floor is not moved with it, the tripwire's headroom
erodes silently, and the next unrelated PR that adds a few percent trips the +10% gate for a reason
that is not its own.

Milestone 300 measured this window and found the drift was **not** the nightly: `nightly-2026-08-27`
and `nightly-2026-09-15` emit byte-identical icount on the same code, and the whole ~+6.5% move was
one code change (milestone 139's cycle-counter grant at the context switch). So the specific alarm
that motivated the proposal was a false attribution. The proposal survives anyway, because the
mechanism it names is real and unguarded: a nightly that *does* move codegen would erode the headroom
exactly as feared, and nothing in the tree would notice until a lane chased an unrelated regression
into it months later. The fix should not depend on which cause happens to bite.

## What to build

The format is comment lines, ratified by calef 2026-09-16. The baseline files already carry a `#`
header and every existing reader skips it, so this is additive rather than a format break:

    # saved: nightly-2026-09-15, qemu 11.1.1, 2026-09-16
    # why: milestone 300 recovered the cycle-counter switch-tuple residual (~33 ticks/switch)

A structured `key: value` block was considered and refused: stricter and uglier for a file a human
reads, where prose stays readable and `--check` needs only the first line.

1. `--save` writes the provenance, into the header of each `bench/baseline-<arch>.txt` it
   rewrites: the pinned nightly, the QEMU version, the date, and one line saying why this save
   happened. That `why` is the ledger, and it is precisely what milestone 300 had to reconstruct by
   hand out of commit messages.
2. `--check` fails loudly on a stale baseline, comparing the recorded nightly against
   `rust-toolchain.toml`'s pin and failing in those words when they disagree. This is Decision 2: it
   turns silent erosion into a red gate a human clears deliberately, at the cost of a manual step on
   every bump that moves the numbers and a no-op on every bump that does not.
3. One writer, one reader. The header is a format two programs agree on, so it is written in
   `xtask`'s save path and parsed in its check path, and not re-derived anywhere else.

Option 1, a bump that re-baselines itself, was refused, and the refusal is now measured rather
than predicted. `44890a8a` blessed a ~5 to ~8% x86_64 regression into the floor while attributing it
to the nightly, a cause milestone 300 (decompose the icount baseline drift, and re-baseline only what is proven) later
  measured at **~0**. An automatic re-baseline is that
failure with nobody in the loop at all.

## BUGS

- A recorded reason cannot be made true. `44890a8a` attributed a 5 to 8% x86_64 move to the
  toolchain, and milestone 300 later measured that toolchain term at ~0; with this in the tree it
  would have written the same false reason, into the file instead of into a commit message. What
  changes is that the next reader meets it in a grep rather than a bisect. This is rung three and
  does not claim to be rung one.
- The reason is per file, so a save that moves twelve rows for one reason and one row for another
  records both in the header and relies on prose to say which is which. Milestone 415's item 2
  asks for the finer thing. A per-row note would have to survive the save path rebuilding the file
  from scratch, which is a format change rather than a flag, and it is not built here.
- Nothing checks that a reason says anything. `--save --why x` passes. A lint that tried to
  judge prose would be the 82% false-positive shape AGENTS.md already refuses; the gate is that a
  human had to type something, and that the something is then in front of the next reader.
- **`# date:` is the date of the save, not of the measurement**, and on a long bench evening those
  differ by a day either side of midnight UTC. AGENTS.md asks for UTC and this writes UTC; where
  the hour matters, say it in the reason.
- Option 1 lets automation commit a floor without a human reading the diff, which is exactly the
  care a baseline save has always carried (`bench/baseline-aarch64.txt`'s own header: "a statement
  that a performance change is intended and understood"). A re-save that also launders a real
  regression into the floor is the failure milestone 300 exists to prevent, one level out; option 1
  would need the decomposition 300 did by hand to be at least partly automatic, or it re-opens the
  hole.
- This is measured on one machine, one QEMU. The numbers are TCG icount on the dev Mac. A
  different runner moves them again, which is the argument *for* mechanizing the re-baseline rather
  than trusting a human to eyeball a percentage, and also the argument that the mechanized floor is
  only as portable as the runner that saved it.

## Index row

`bench/baseline-<arch>.txt` records what its numbers mean relative to QEMU and says nothing about the
toolchain, so nothing can tell a baseline that is stale for the current nightly from one that is
current. This makes `--save` write its own provenance into the file's comment header (nightly, QEMU,
date, and one line of why) and makes `--check` fail loudly when that recorded nightly and
`rust-toolchain.toml`'s pin disagree. Promoted from Decision 2 of milestone 300's finding with
calef's attribution-ledger ruling folded in, because the two are one mechanism: fail-loudly cannot be
implemented without the ledger that records the nightly. Option 1, an automatic re-baseline on every
bump, was refused, and `44890a8a` is the measured vindication.

## Follow-on

- **Recorded.** The reason is per file rather than per row, in `script/bench`'s `BUGS` and in this
  block's, beside the flag a reader meets. Milestone 415 (sub-tripwire drift accumulates across
  baseline saves) item 2 asks for the finer shape and keeps it.
- **Milestone 415.** Item 3, a cumulative check against a fixed historical anchor, is the work this
  ledger was meant to feed. Its own recommendation is to hold it until the attributions collected
  here are good enough to gate on, and none exist yet: every floor in the tree still says
  `unrecorded`.
