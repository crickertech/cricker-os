---
status: PROPOSED
raised: 2026-09-29
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# Fatal risk colors, counted by week

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

## BUGS

- The backfill parses the status-line prose of past blobs, and prose is not data: risk 2's line
  says "AMBER. The red half is that...", so the parse keys on the first color word after the
  status marker, and the backfilled grid is read against the note before it lands. Where a past
  week's color cannot be read honestly, the row starts later and says so rather than guessing.
- Counts of nine are coarse. A week where one risk turned amber moves a bar by one, and which risk
  it was needs the main file; this chart is the glance, not the record.

Name provisional.
