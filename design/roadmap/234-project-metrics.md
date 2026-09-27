---
status: BUILT
raised: 2026-09-02
built: 2026-09-02
---
# 234. The project's own numbers, one row per ISO week

Asked for by calef the same day, built in one lane, and minted after the
fact because the work was small enough to outrun its own block. *(Number provisional until the merge
queue lands it.)*

The gate is discharged, and recorded rather than deleted: it read `Gate: NONE`, because
nothing external blocked it and nothing does now.

In brief. `notes/register-of-measures.md` opens with the complaint this milestone answers:

> This tree measures a great deal and remembers almost none of it. A number gets taken once, written

So: `script/metrics` (name provisional) walks git history and writes one row per ISO week into
`notes/project-metrics/weekly.csv`, and renders eight hand-written SVGs into
`notes/project-metrics.md`, linked from `README.md`. A weekly workflow appends the current week and
opens a pull request, modelled on `toolchain-bump.yml`. The whole backfill is `git ls-tree` and
`git cat-file --batch` with no build and no boot: eight weeks in six seconds, and no plotting
dependency, per DECISIONS §46 (thin primitives or whole subsystems; we write everything in between).

## What it measures

Milestones and decisions by status, Rust lines split into code and comments, `BUGS` sections, the
falsification ratio, `unsafe` density outside `kernel/src/arch/`, and fatal-risk status. Coverage has
a column and no history, for a reason recorded below.

`BUGS` rising is good, and the page says so where a reader meets the chart. AGENTS.md calls those
sections "not modesty, they are the mechanism", and a reader watching a "bugs" line climb will
otherwise read the tree's growing honesty as decay.

## Three things the series said that nobody had written down

- `NOT-STARTED` grows faster than `BUILT` drains it, and the gap widened most in the last two
  weeks. The roadmap generates work faster than lanes consume it.
- Total Rust fell for the first time (192.0k to 189.0k) in a week that was still adding code,
  which is milestone 54's (a network file service a Mac can mount) removal showing up as a line count
  going down.
- AGENTS.md's "kernel/src measures 40% comments" is stale. It was 39.3% eight weeks ago and is
  45.3% now, rising every week. Flagged on the page and deliberately not corrected in that file,
  which is an architect's.

## The honesty the page carries, which is the point of it

A reconstruction applies today's definitions to old commits. That is right for a trend and it is
not what was reported at the time. The page says so on its face, because the day this was built
produced three separate cases of this tree's instruments being wrong: milestone 214 (a test that
prints "skipping" and returns is counted as passed) found 80 such sites, milestone 212
(`script/falsifications` walks `crates/` only) found a denominator excluding the kernel and `user/`,
and DECISIONS 139 (who may read the cycle counter, and by what authority) corrected a figure that
confused a counter's tick resolution with the cost of reading it.

And every historical figure in this tree is agent-derived. calef holds no independent record, so
agreement with `notes/unsafe-obligations.md`'s seven columns is a **consistency** check and is
described as one. The independence came from elsewhere: a second `unsafe` counter written from
scratch as a character scanner tracking Rust's real lexical states, differing deliberately from the
regex where the regex is loose (block comments nest in Rust; `'a` is a lifetime). Over every in-scope
file it agreed exactly: **701 outside `arch/`, 253 inside, zero files disagreeing.**

## BUGS

- The shared derivations are copied, not imported. The `unsafe` census, the code and comment line
  split and the harness count are duplicated into `script/metrics` from inside `script/lint`'s and
  `script/falsifications`' shell heredocs, where nothing can import them. **If a gate changes its
  definition this page keeps the old one silently**, and no check fires. That is rung four holding a
  measurement page together and it is proposed as its own milestone.
- Coverage has no history, because it is computed per pull request in CI and stored nowhere;
  backfilling means an instrumented build at every checkout. The column is visibly empty with the
  reason beside it rather than omitted.
- Milestone statuses before 202631 are unrecoverable. `design/roadmap.md` had ten rows and no
  status column; state was prose inside a cell. That is the sharpest restatement artifact on the page.
- **`--check` is deliberately not in `script/lint`**, because any commit moves `HEAD` and the row
  records the commit it was taken at, so gating it would fail every pull request.

## Follow-on

- **Milestone 236.** The shared derivations are copied into `script/metrics` from inside
  `script/lint`'s and `script/falsifications`' shell heredocs, where nothing can import them, so a
  gate that changes its definition leaves this page holding the old one silently and no check fires.
  This block proposed it as its own milestone and 236 is that milestone.
- **Recorded.** `design/roadmap/234-project-metrics.md` BUGS: coverage has no history, because it is
  computed per pull request in CI and stored nowhere, and backfilling means an instrumented build at
  every checkout. The column is visibly empty with the reason beside it rather than omitted.
- **Recorded.** `design/roadmap/234-project-metrics.md` BUGS: milestone statuses before 202631 are
  unrecoverable, because the single roadmap file this directory replaced had ten rows and no status column, with state as prose
  inside a cell. That is the sharpest restatement artifact on the page.
- **Recorded.** `notes/project-metrics.md` says on its face that a reconstruction applies today's
  definitions to old commits, which is right for a trend and is not what was reported at the time.
  It also says that every historical figure in this tree is agent-derived, so agreement with
  `notes/unsafe-obligations.md` is a consistency check rather than independent confirmation.
- **Recorded.** `design/naming.md` holds `script/metrics`, which shipped provisional the way a lane's
  names do; `script/names --unratified` is the worklist and it does not block anything.
- **Refused.** Putting `--check` in `script/lint`. Any commit moves `HEAD` and a row records the
  commit it was taken at, so gating it would fail every pull request.
- **Milestone 350.** Correct AGENTS.md's
  "kernel/src measures 40% of them" comment ratio. The series put it at 39.3% eight weeks before this
  was written and 45.3% then, and it reached 46.4% on 2026-09-14, climbing every week, so the figure a reader meets in the project's own front-door file
  is wrong and drifting further from true. AGENTS.md is an architect's file, so the one-line edit is
  an architect's to make; this page flags the staleness and cannot fix it.

## Index row

`notes/register-of-measures.md` opens by saying this tree measures a great deal and remembers
almost none of it
