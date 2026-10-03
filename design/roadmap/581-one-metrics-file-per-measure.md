---
status: BUILT
raised: 2026-09-23
built: 2026-09-23
---
# 581. One metrics file per measure, so a new measure is a new file

The number is provisional: the integrator mints it at merge. 580 was the highest in the tree when
this lane cut its branch. The file name and every name this block invents are provisional too.

calef asked the question on 2026-09-23 (UTC), reading a batch of rebases: *"Maybe we need multiple
files rather than a single file with all the metrics?"* He asked it because the file had just
aborted two rebases in one batch.

## The problem, which was a conflict class rather than a conflict

`notes/project-metrics/weekly.csv` was 58 columns wide and held every measure side by side. Every
branch that added a measure added columns to it, so it rewrote the header line and all eleven data
lines. Two branches adding measures that had nothing to do with each other still collided, and
collided with `main` as well.

That is not hypothetical. PR #1179 (model attribution) and PR #1184 (peak context) each aborted a
rebase on this file in one batch, each adding disjoint columns while `main` had added
`coverage_min_file_pct`. Neither lane had read the other's columns and neither needed to.

The directory already half-agreed. `effort.csv`, `mutation-census.csv`,
`mutation-census-renames.csv` and `ci-log-baseline.csv` are per-measure files, and the charts are
one SVG per measure. The main data file was the only thing in `notes/project-metrics/` that was not.

## What this does

One file per measure, `notes/project-metrics/<measure>.csv`, each carrying the week key and that
measure's own columns. `MEASURES` in `script/metrics` is the table, and `FIELDS` is derived from it
rather than maintained beside it, so the two cannot disagree about which columns exist.

This is the top rung of AGENTS.md's ladder rather than a reduction. A new measure is a new file, and
a new file cannot conflict.

The boundaries are the charts'. `notes/project-metrics.md` already presents these as separate
panels, so a reader who knows the page knows the files:

| file | what it holds |
|---|---|
| `weeks.csv` | the index: which commit and date each week was read at |
| `milestones.csv` | milestones by status, the total, and unnumbered proposals |
| `velocity.csv` | milestones built that week |
| `pull-requests.csv` | pull requests merged that week |
| `decisions.csv` | architecture decisions by status |
| `names.csv` | names by what the tree records about them |
| `lines.csv` | code and comment lines, `kernel/src` split out |
| `bugs.csv` | `BUGS` sections in markdown and in Rust |
| `harnesses.csv` | Kani harnesses and what can falsify them |
| `unsafe.csv` | the `unsafe` census and its density |
| `unsafe-trust.csv` | the same census by trust boundary |
| `fatal-risks-colors.csv` | the nine risks by verdict colour, and the total |
| `coverage.csv` | line coverage and the per-file minimum |
| `prose-budget.csv` | words over the 3,000-word prose cap, and documents over it |
| `cost.csv` | the five cost columns of milestone 519 (what this project costs, tracked where it cannot rot) |

`cost.csv` rather than `effort.csv`, because `effort.csv` in that directory is `script/effort`'s
output and an input to this script. The chart it feeds keeps its name, `effort.svg`.

## Absent rather than blank, which is what the data actually says

A row is written only when its measure has something to say for that week. A measure introduced in
2026W39 used to leave ten empty cells behind it, and an empty cell is ambiguous between "measured
zero" and "nobody looked". `coverage.csv` now starts at 2026W30, because 2026W29 predates
`script/coverage` and there was never a number to record.

The ambiguity that is real stays visible. `coverage.csv` carries 2026W30 through 2026W38 with an
empty `coverage_min_file_pct`, because those weeks do have a coverage number and do not have a
per-file minimum. A week a measure answered partially still gets its row.

## The new measure, which is what proves the split

calef ratified a prose budget on 2026-09-23 (UTC): 3,000 words of main body per document, with
appendices under the same cap. He asked for a graph in the same breath. A ratchet nobody plots is a
rule that holds for months with nobody able to say whether the tree is gaining ground. That series
was added here, created the way
any future measure would be: a function, an entry in `MEASURES`, a new file. No existing file's
header moved.

Two series. The excess above the cap, in words, is the debt: what would have to move into
appendices. The count of documents over the cap is where that work sits. Two panels rather than
one, because the series stack with nothing and one split book moves the first without moving the
second.

### Every document is counted, a marked exception included

The exception mechanism
belongs to the gate that ruling asks for, which is a different milestone and does not exist yet. A
chart
that subtracted exempted documents would hide the debt, and the debt is what the chart is for. When
the gate arrives it wants its own column for what it excuses; it does not get to edit this one.

Measured on this branch's base at 2026-09-23: 578,737 words over the cap, across 175 documents,
against 1,011 documents and 2,089,538 words in scope. The ratified figures are 569,775 and 174,
taken at `29fa47181`. The difference is `AGENTS.md`, which they were measured without and this
counts, at
10,967 words: 10,967 minus the 3,000 cap is 7,967, and 175 minus 174 is one. The two reconcile
exactly. A cap the tree's own rulebook is exempt from is not a cap.

## How the migration was verified

Every cell was moved rather than recomputed, then checked twice.

The first check reads the old `weekly.csv` and every new file and compares cell by cell. It reports
no column with two homes, no column with none, no value changed, no non-empty cell absent from every
new file, and no cell in a new file that `weekly.csv` did not have. Two empty cells were dropped
with their row, both of them 2026W29 coverage, which is the absent-rather-than-blank rule working.

The second check is stronger and was not planned. Running `script/metrics --backfill` over the
migrated files recomputes all eleven weeks from git history. Every week from 2026W29 to 2026W38 came
back byte-identical. Only 2026W39 moved, because this branch's base is a later commit of the
same week than the one `weekly.csv` was last written at. An independent recomputation reproducing
ten weeks exactly is better evidence than any diff of the migration itself.

## `weekly.csv` is deleted

Keeping it would keep the conflict class, which is the whole milestone: a joined file regenerated
beside the per-measure ones still has a header every metrics branch edits.

Nothing outside this repository consumes it. It was reachable through `README.md` on a public
repository, and that pointer is updated in the same commit; the path stays readable at any earlier
ref, so no URL breaks that this commit does not also fix.

## Records that still name it, on purpose

`design/decisions/177`, `210`, and the roadmap blocks for 234, 276, 350, 434 and 519 still say
`weekly.csv`. They describe what was true when they were written, and this file's own blind-`sed`
scar is the argument for leaving them. A record rewritten to match today is a record that can no
longer be checked.

## BUGS

### `--check` now names every stale file rather than one

The message lists them,
because a caller told one name at a time learns nothing and `--update` rewrites them all anyway. The
pre-existing staleness-after-a-commit caveat is unchanged and is in the script's own header: the
current week's row records HEAD's sha, so committing anything makes `weeks.csv` read stale until the
next run.

### The split does not make two branches measuring the same week safe

It dissolves the
disjoint-columns collision and nothing else. Two branches that both run `--update` in the same week
still conflict on that week's row, in whichever measures moved. `briefs/rebase-onto-main.md` carries
the resolution, which is to take `main`'s side and re-run the measurement.

### A measure's file is not gated against drifting from `MEASURES`

`read_csv` reads a column only
from the file `MEASURES` assigns it to, so a stray column left by a hand edit is ignored and the
next `--update` drops it. Nothing warns that it was there.

### A reader wanting one week's every measure has to join fifteen files

That was one line in
`weekly.csv`. `script/metrics --table` still prints the joined row, and that is the whole of the
answer today.

## Follow-on

- **Recorded.** A reader wanting every measure for one week joins fifteen files; the limitation is
  in this block's `BUGS` section and in `script/metrics`' own header, beside `--table`, which is
  the answer today.
- **Recorded.** Two branches measuring the same week still conflict on that week's row. The
  resolution is in `briefs/rebase-onto-main.md`, where a rebasing lane meets it.
- **Refused.** The word-count gate and its exception mechanism stay out of this block. A gate
  is enforcement and this is measurement; building both in one lane would let the exception
  mechanism edit the number that exists to show what the exceptions cost. The two series will
  answer for the ratchet when the gate arrives as its own milestone.
- **Done.** PR #1179 (model attribution) and PR #1184 (peak context) each add a measure and each
  aborted a rebase on `weekly.csv`. Each is now a new file, so both rebase without touching a
  header the other wrote. Neither branch was touched by this lane.

## Index row

`notes/project-metrics/weekly.csv` was 58 columns wide, so every branch that added a measure
rewrote the same header and the same eleven data lines, and two branches adding measures with
nothing to do with each other still conflicted. The data is now one CSV per measure, with
`MEASURES` in `script/metrics` as the table and `FIELDS` derived from it, so a new measure is a new
file and a new file cannot conflict. A measure's rows begin at the week it was first measured
rather than trailing blank cells, which is what the record actually says. Every cell was migrated
rather than recomputed and checked twice: cell by cell against the old file, and then by a full
`--backfill` that reproduced 2026W29 through 2026W38 byte-identically. The prose budget calef
ratified on 2026-09-23 was added through the new mechanism, as the proof that it works. It stands at
578,737 words over the 3,000-word cap across 175 documents, which reconciles exactly with the
ratified 569,775 and 174 once `AGENTS.md` is counted. `weekly.csv` is deleted, because keeping a
joined file would keep the conflict class.
