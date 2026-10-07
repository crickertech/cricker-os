---
status: BUILT
raised: 2026-09-29
built: 2026-10-02
promoted_from: fatal-risk-colors-by-week
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 625. Fatal risk colors, counted by week

Promoted 2026-10-02 (UTC) from the proposal `fatal-risk-colors-by-week`, raised 2026-09-29. The
number is provisional until the merge queue lands it; the title and slug are drafts. The first
attempt, pull request #1460, merged with only its claim commit, so none of it reached `main`;
calef ruled on 2026-10-02 to revive it, and pull request #1479 carries the work.

calef, 2026-09-29, two rulings. On the chart: "The chart should be a bar chart with a stack for
each color every week." On the data: "If it isn't already, the color should be in the frontmatter
for each fatal risk." It is not already; this builds both.

## The gap

`fatal-risks.svg` charts Experiment statuses, and no color has ever been recorded per week. The
colors live in prose in `design/fatal-risks.md`, on each entry's status line, and only in the
present tense. A reader can see that eight risks run and one cannot, and cannot see that a risk
has been amber for six weeks or that two turned the same week.

## The mechanism

1. Each of the nine appendix files under `design/fatal-risks/` gains YAML frontmatter: `risk:` (its
   number), `color:` (green, amber or red), `updated:` (the UTC date of the last color change). The
   main file's prose keeps the why and the dates; the frontmatter is the machine-read surface, so
   `MEASURED` and `AUDITED` stay prose qualifiers and never become a fourth column.
2. `notes/project-metrics/fatal-risks-colors.csv`: one row per week, three count columns (green,
   amber, red), appended by the weekly run from the nine frontmatters. History is backfilled once
   from git history, the way the unsafe-trust table was baked on 2026-09-20: derived offline,
   recorded, never re-derived.
3. `fatal-risks-colors.svg`: a bar chart, one bar per week, stacked green then amber then red, in
   the colors' own hues, drawn beside `fatal-risks.svg` in `notes/project-metrics.md`.

## Done means

The nine frontmatters exist and agree with the status lines a reader reads. The chart shows every
week on record, stacked by color. A week whose colors changed is visible at a glance, which the
status chart cannot show.

## What was built

- The nine appendices carry `risk:`, `color:` and `updated:`. The vocabulary is four words, not
  three. `none` is a risk with no verdict rendered, five of the nine. It is said outright rather
  than left absent, and it counts in no column. Each word was re-read from `design/fatal-risks/README.md` on
  2026-10-02. Risks 1 and 9 are green, 2 and 3 amber.
- `script/metrics` reads the frontmatter where a tree has it and the status-line prose where it
  does not, and fails the run naming the risk when the two disagree. That cross-check is the only
  gate an appendix has.
- `notes/project-metrics/fatal-risks-colors.csv` and `fatal-risks-colors.svg`, drawn under the
  status chart on `notes/project-metrics.md`. The history came from one `script/metrics
  --backfill` on 2026-10-02: 2026W36 and 2026W37 one green and one amber, risk 3's amber lands in
  2026W38, risk 9's green in 2026W39. No row was written by hand.
- [`notes/register-of-measures/fatal-risk-colours.md`](../../notes/register-of-measures/fatal-risk-colours.md)
  argues the measure and records the backfill's limits.

## BUGS

- The backfill parses the status-line prose of past blobs, and prose is not data. Risk 2's line
  says "AMBER. The red half is that...", so the parse keys on the first capitalized color word
  after the status marker. The backfilled grid is read against the note before it lands. Where a past
  week's color cannot be read honestly, the row starts later and says so rather than guessing.
- Counts of nine are coarse. A week where one risk turned amber moves a bar by one, and which risk
  it was needs the main file; this chart is the glance, not the record.


## Follow-on

- **Recorded.** The backfill's limits and the coarseness of a count of nine are BUGS in
  `notes/register-of-measures/fatal-risk-colours.md`, beside the measure.

## Index row

The nine fatal risks had a chart of whether each experiment ran and none of what it found. Each
appendix now carries its verdict color in frontmatter, cross-checked against the summary's status
line, and a weekly stacked bar counts green, amber and red with gray for no verdict, backfilled
from 2026W36. A risk turning color is now visible the week it happens. Name provisional.
