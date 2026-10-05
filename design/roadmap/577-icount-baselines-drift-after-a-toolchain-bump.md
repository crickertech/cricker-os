---
status: SUPERSEDED
raised: 2026-09-15
superseded_by: 300, 302, 415
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 577. The icount baselines predate the pinned nightly, so the tripwire's headroom is eroding unseen

Superseded 2026-09-23, by milestone 300 (decompose the icount baseline drift, and
re-baseline only what is proven), milestone 302 (a baseline records what it was saved against, and a
stale one fails loudly) and milestone 415 (sub-tripwire drift accumulates across baseline saves).
Promoted and superseded in one act, the shape milestone 431 (the ACPI walk is reachable by the
prover for the first time, and proved by nothing) used, because a proposal cannot be disposed of in
place: it was filed 2026-09-15 by the lane of milestone 299 (the x86 port-range capability), sat on
PR #883, and both halves of it were answered while it sat. *(Number provisional until the merge
queue lands it.)*

Nothing gates a block whose work has homes. What is still owed is owed by 302 and by
415's item 2, and both of those carry their own gates; this one exists so the number is spent and
the evidence below has somewhere to live.

## Why this is SUPERSEDED rather than BUILT, PARTIAL or REFUSED

Nothing here was built under this number and nothing was declined. Both items were answered
elsewhere, which is what the word is for:

- Item 1, re-measure and re-save. Done by milestone 300, which decomposed the drift instead of
  blessing it, and found the toolchain term across `nightly-2026-08-27` and `nightly-2026-09-15` was
  ~0: byte-identical icount on the same code. So this proposal's central measurement was a false
  attribution, and 300's finding is the correction.
- Item 2, a bump must re-baseline or refuse to be silent. calef ruled on 2026-09-16 and it is
  milestone 302, the fail-loudly half. Most of 302 then landed on 2026-09-21 as commit `99f13dad`.
- The fastpath-footprint half, added to this file on 2026-09-21, belongs to
  milestone 361 (the riscv64 and x86_64 fastpath residuals are unattributed), not to this block.

A third block for the same mechanism is the failure this tree keeps paying for, so this one is
disposed of.

## What the existing gate covers, and what it does not

`script/lint`'s baseline-toolchain check (commit `99f13dad`, 2026-09-21) is the fail-loudly half in
the tree today. Read it before assuming anything is unguarded.

What it covers. Each `bench/baseline-*.txt` carries a `# toolchain: <channel>` line written by
`cargo xtask bench --save` from the pin rather than from whatever compiler happened to run, and the
check fails when that line and `rust-toolchain.toml` disagree. It names the three commands that
re-record the floors, and it says in the failure text that re-recording without reading the delta is
the one outcome it refuses to automate. It works: it fired on PR #1112 (the 2026-09-23 nightly bump)
exactly as designed, and that pull request's own body tells the reader why the red is a step rather
than a defect.

**What it does not cover, and each of these is deliberate or known:**

- It can only fail on a branch that raises the pin. Every other branch carries a baseline and a
  pin that already agree. That is the design, chosen over a general staleness check that would fail
  branches which caused nothing, and it means the check says nothing about a floor that is stale for
  any reason other than the compiler.
- **It checks that a stamp matches, never that the numbers under it are right.** A `--save` run on
  the correct nightly satisfies it whatever it writes, including a regression blessed into the floor.
  That is 415's whole subject.
- It records no reason. The header holds `# toolchain:` and `# qemu:` and nothing else. 302's own
  `## What to build` asks for a date and a `# why:` line beside them, and those are not in the file
  `99f13dad` produces. So the ledger half of 302 is unbuilt while 302's block still reads
  `NOT-STARTED`, which understates it in one direction and overstates it in the other.
- `NIFE_BUMP_IN_PROGRESS=1` skips it entirely, which the check's own comment marks as a foot gun.

## Milestone 415 already owns the attribution work, and this block does not duplicate it

Checked 2026-09-23. 415's item 2 is exactly "make a save record its own attribution, beside the
number", and it is `PARTIAL` with that item listed as outstanding under `## Follow-on`. Its
decision is §190 (must an icount baseline save record why it moved), in
[design/decisions/190-what-a-baseline-save-must-record.md](../decisions/190-what-a-baseline-save-must-record.md).
302 asks for the same line in the same header, having been ruled on by calef in the same breath. Two blocks is already one too many; this block writes no third
version of it and proposes no mechanism of its own.

What this block adds is evidence that the item is worth more than it looked, recorded where the
next reader of 302 or 415 will find it.

## The evidence tonight produced, which is about attribution and not staleness

Measured 2026-09-23 by the lane clearing PR #1112, and corroborated here from git alone:

- riscv64 `rfence_self` returned to 5991, to the tick, in three reproducible runs. That is the
  value the row held before the 2026-09-22 bump put it back to 6476.
- The 2026-09-22 lane called 5991 a stale floor carrying dead margin. That reading was wrong, and
  the wrongness is not the interesting part. The row has now sat at 5991 and 6476 alternately across
  a handful of consecutive saves, which git shows without running anything:

  | commit | date | `# toolchain:` | `rfence_self` |
  |---|---|---|---:|
  | `e6833cd2` | 2026-09-20 | (none yet) | 5991 |
  | `28e165e2` | 2026-09-20 | (none yet) | 6476 |
  | `99f13dad` | 2026-09-21 | nightly-2026-09-20 | 6476 |
  | `84e29394` | 2026-09-21 | (hand edit) | 5991 |
  | `d14ad408` | 2026-09-21 | nightly-2026-09-20 | 6476 |
  | `483382c9` | 2026-09-21 | nightly-2026-09-20 | 5991 |
  | `c64af73e` | 2026-09-22 | nightly-2026-09-22 | 6476 |

- `spawn_el0` is up ~6% on both aarch64 and riscv64 under `nightly-2026-09-23`, bit-identical
  across repeats, and under the 10% tripwire. So it will be absorbed into the next floor by a save
  whose only account of itself is a commit message.

**Two competent lanes reached opposite conclusions about the same counter four days apart, and the
file could not tell either of them anything.** `notes/benchmarks/rfence-self-row.md`'s 2026-09-21
reading attributes 6476 to a run made at the wrong hart count and shows the row is hart-sensitive
(5991 at one hart, 6432 at two, 7153 at four). Tonight's reading attributes the same oscillation to
the compiler. Both are argued from outside the file, because the file records a number and not why
it holds that value, and that is the gap: not that a floor goes stale, which now fails loudly,
but that a floor cannot say what it means. It is the `BUGS`-section posture applied to a data file,
and it is already item 2 of 415 and the `# why:` line of 302.

The concrete cost is in notes/benchmarks.md already, from the lane that hand-edited the row: the `#`
comment it wrote above `rfence_self` explaining the correction **did not survive the next `--save`**,
because the save path rebuilds the file from a fixed header plus one `name ticks iters` line per
result and preserves nothing else. A row that cannot carry a reason cannot carry a correction either.

## Recommendation, which is a disposal rather than a plan

1. Take 302. It is misrecorded as `NOT-STARTED` while two of its three items landed in
   `99f13dad`; its remaining item is the `# why:` line, calef ratified the format on 2026-09-16, and
   415's item 2 is the same work stated once more. Correcting that status is 302's own lane's to do.
2. **Do not open a fourth block for the attribution ledger.**
3. **Do not re-save any floor to settle the `rfence_self` question.** A save is a committed floor and
   is an architect's, and the two readings above disagree about cause, which is the one condition
   under which blessing a number is worst.

## BUGS

- The 2026-09-23 numbers are another lane's, reported rather than re-run here. This block
  re-derived the oscillation table from committed text, which is checkable and cheap, and did not
  boot anything: several lanes share this machine and a bench run is not free. So the table says what
  each save *recorded*, never what the tree measured between saves, which is the same limit 415's
  audit states about itself.
- The original proposal's measurements are left above in git and not reproduced here, because
  milestone 300 measured the toolchain term at ~0 and superseded them. Reading the pre-promotion text
  as current is the mistake this note exists to prevent.
- "Compiler-dependent" is tonight's reading, not a settled cause. The hart-count reading in
  notes/benchmarks.md fits the same data. Deciding between them needs an A/B across two nightlies at
  a fixed hart count, which nobody has run, and neither reading is a reason to move a floor.

## Index row

`bench/baseline-*.txt` are the icount tripwire's committed floors and a new nightly revalues every
number in them, so this was filed on 2026-09-15 after a bump left the floors alone and most of the
headroom went with it. Both of its items were answered while it sat unpromoted on a branch: milestone
300 decomposed the drift and measured the toolchain term at ~0, and milestone 302 is an architect's
fail-loudly ruling, most of which landed as `99f13dad`'s `script/lint` check that fires when a
baseline's `# toolchain:` line and `rust-toolchain.toml` disagree. Promoted and superseded in one act
so the number is spent and the evidence has a home: that check covers staleness and nothing else, and
riscv64's `rfence_self` oscillating between 5991 and 6476 across consecutive saves, read as a wrong
hart count by one lane and as compiler drift by another four days later, is what a floor that records
a number without recording why looks like from the outside. The fix is already owned twice, by 302's
`# why:` header line and by milestone 415's item 2, and this block deliberately adds no third
statement of it.
