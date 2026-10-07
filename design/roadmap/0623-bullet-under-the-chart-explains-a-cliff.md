---
status: BUILT
raised: 2026-09-29
built: 2026-09-29
promoted_from: a-bullet-under-the-chart-explains-a-cliff
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 623. Bullet under the chart explains a cliff

Promoted 2026-09-29 (UTC) from the proposal `a-bullet-under-the-chart-explains-a-cliff`, raised
the same day by the maintainer's lane recording calef's ask, after reading `lines.svg`: *"I'd like
to dive deep into such changes in the future and then have the option of writing an annotation for
the week explaining the metric change. We can drop the annotations for the week when the week ages
out of the graph."* And on rendering: *"An annotation can just be bullets under the graph by
week."* The number is provisional until the merge queue lands it; the title and slug are drafts.

## The cliff that motivated it

The kernel code series fell 46,665 lines in 2026W39 to 33,192 in 2026W40, a 29% drop, and the
chart explained none of it. The cause was milestone 609 (the system tests leave the kernel crate),
whose pull request #1404 moved 66 test-only files into a `system_tests` crate unchanged on
2026-09-27. No code was deleted; the series' kernel bucket is a path prefix, so a rehoming reads
as a shrink. The archaeology took one evening and lived in a conversation, which is rung four.
A stranger reading the chart saw a cliff and no reason.

## The mechanism

Four pieces, one milestone.

1. The weekly metrics run flags a week whose move in any charted series exceeds one tenth of the
   prior week's value. A flag is a flag: it never blocks, it never edits. `script/metrics --flags`
   prints every flagged week in the chart window with its biggest mover first, and a flagged week
   with no row in `notes/project-metrics/week-notes.csv` makes the weekly run's report say so in
   one line.
2. The digest, `helpers/week_digest.py`: given two weekly snapshot refs, the five files that
   moved a flagged series most, with the commits that moved them, from `git log --numstat` between
   the snapshots. It is a separate process on purpose, because the digest is log archaeology and
   `script/metrics` is a reader of blobs and series. The weekly pull request's body carries flag
   and digest, pre-chewed, so an annotator starts from data rather than from a hunch.
3. Annotations live in `notes/project-metrics/week-notes.csv`, one row per annotated week, written
   by hand or by a lane when a flag is read. The file is appended to and never regenerated, except
   that the weekly run drops rows whose week has aged out of the chart window, which is ten weeks
   (`CHART_WEEKS` in `script/metrics`), without touching any other row. The drop is the whole
   point of aging out: a bullet that outlives its graph explains nothing to anybody.
4. `script/metrics` renders each row as a bullet under the chart it explains, in week order, in
   `notes/project-metrics.md`. Bullets under the chart, by week, per calef's ruling above; no
   markers inside the SVG.

## Built

- The flag, in `script/metrics`: computed over every series the charts draw, stored and derived
  alike, comparing each charted week against the week before it in the series. A series born that
  week does not flag, because there is no prior value to move from; a measured zero moving does.
- `--flags`, a mode the weekly workflow embeds in the pull request body, and a one-line report in
  `--update` for every flagged week that carries no note. `--selftest` proves the flag, the prune
  and the render against fixtures.
- The prune, in `--update` and `--backfill`: rows whose week has aged out of the ten-week window
  are dropped line by line, and every other line of the file keeps its bytes.
- The renderer, run by every mode that redraws the charts: one bullet per row, under the chart
  image the row names, in week order, between generated markers.
- The first case, 2026W40's row, names milestone 609 and pull request #1404, and a stranger
  reading `notes/project-metrics.md` sees it under the lines chart.

## BUGS

- The one-tenth threshold is a heuristic, carried from the proposal verbatim, not a measurement.
  Measured 2026-09-29 over the ten charted weeks 2026W31 to 2026W40: every week flags, on 16 to 44
  series. Growth in a young project moves count series more than a tenth most weeks. The report
  stays one line per unannotated flagged week; the threshold wants an architect's ruling, with
  those numbers in hand.
- The digest names movers, not causes, and the annotator still reads the commits. Its
  series-to-path table maps `kernel_*` to `kernel/src/` and digests the whole tree for every other
  series, which for most series is the honest scope rather than a good filter.
- The digest lives in a pull request body, which the tree treats as a record nobody can rely on.
  The bullet under the chart is the record; the digest is scaffolding for writing it.
- `script/metrics --selftest` is not wired into `script/lint`, which was outside this lane's
  grant. Run it when touching the flag, the prune or the render.
- Each rendered bullet must find its words inside the page's cap, §212 (a prose budget), and this
  milestone trimmed `notes/project-metrics.md` elsewhere to make room for the first one.

## Follow-on

- **Milestone 698.** Milestone 698 (the week flag fires every week). The one-tenth threshold fires every week on this tree's counts; the measurement
  and the candidates are in
  `design/roadmap/0698-the-week-flag-fires-every-week.md`.

## Index row

A cliff in a weekly chart reads as a defect in an instrument until somebody says otherwise. A
week whose move in any charted series exceeds a tenth of the prior week is flagged, and the weekly
pull request carries the flag with a five-file digest. A hand-written row in
notes/project-metrics/week-notes.csv renders as a bullet under the chart it explains, until the
week ages out of the window. Milestone 609's test-file move, which cut the kernel code series by a
third in 2026W40, is the first case.
