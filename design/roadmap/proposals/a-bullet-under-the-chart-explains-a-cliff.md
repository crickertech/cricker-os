---
status: PROPOSED
raised: 2026-09-29
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# A bullet under the chart explains a cliff

calef, 2026-09-29, after reading `lines.svg`: "I'd like to dive deep into such changes in the
future and then have the option of writing an annotation for the week explaining the metric
change. We can drop the annotations for the week when the week ages out of the graph." And on
rendering: "An annotation can just be bullets under the graph by week."

## The motivating cliff, on record

The kernel code series fell 46,665 lines in 2026W39 to 33,192 in 2026W40, a 29% drop, and the
chart explains none of it. The cause was milestone 609 (the system tests leave the kernel crate),
whose pull request #1404 moved 66 test-only files into a `system_tests` crate unchanged on
2026-09-27. No code was deleted; the series' kernel bucket is a path prefix, so a rehoming reads
as a shrink. The archaeology took one evening and lived in a conversation, which is rung four.
A stranger reading the chart sees a cliff and no reason.

## The mechanism

Three pieces, one milestone.

1. The weekly metrics run flags a week whose move in any charted series exceeds one tenth of the
   prior week's value. A flag is a flag: it never blocks, it never edits. The weekly pull
   request's body carries the flag and a digest: the five files that moved the flagged series
   most, with the commits that moved them, from `git log --stat` between the two weekly snapshots.
   That digest is the dive-deep, pre-chewed, so an annotator starts from data rather than from a
   hunch.
2. Annotations live in `notes/project-metrics/week-notes.csv`, one row per annotated week, written
   by hand or by a lane when a flag is read. The file is appended to and never regenerated, except
   that the weekly run drops rows whose week has aged out of the chart window, which is ten weeks
   (`CHART_WEEKS` in `script/metrics`). The drop is the whole point of aging out: a bullet that
   outlives its graph explains nothing to anybody.
3. The generator renders each row as a bullet under the chart it explains, in week order, in
   `notes/project-metrics.md`. Bullets under the chart, by week, per calef's ruling above; no
   markers inside the SVG.

## Done means

The first row is 2026W40's, naming milestone 609 and pull request #1404, and a stranger reading
`notes/project-metrics.md` sees it under the lines chart. A flagged week with no row makes the
weekly pull request's body say so in one line. A row whose week leaves the window disappears on
the next weekly run without touching any other row.

## BUGS

- The one-tenth threshold is a heuristic and a proposal, not a measurement. A drift that moves a
  series a twentieth each week never flags. The digest is `git log --stat` archaeology: it names
  movers, not causes, and the annotator still reads the commits.
- The digest lives in a pull request body, which the tree treats as a record nobody can rely on.
  The bullet under the chart is the record; the digest is scaffolding for writing it.

Name provisional.
