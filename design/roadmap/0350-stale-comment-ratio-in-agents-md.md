---
status: SUPERSEDED
raised: 2026-09-03
superseded_by: §177
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 350. The comment ratio AGENTS.md quotes is wrong, and it climbs every week

Superseded 2026-09-24, by §177 (whether AGENTS.md quotes measured numbers at all), which calef decided that day: AGENTS.md drops its counts and points at `notes/project-metrics.md`, so the sentence this block would have corrected no longer exists (commit `e23df4a4d`, PR #1198). The rest of this block is the record as it stood before that ruling. Filed 2026-09-03 as an unnumbered proposal by the sweep of milestone 247 (follow-on work named by a finished milestone goes nowhere),
from milestone 234's block; numbered 2026-09-19 by milestone 433. **Premise re-checked 2026-09-19 and
it holds, and the number has drifted further.** `AGENTS.md` line 111 still reads "`kernel/src`
measures 40% of them". `notes/project-metrics/weekly.csv`'s latest row (2026W38, 2026-09-14) gives
`kernel_code_lines` 36,157 and `kernel_comment_lines` 31,309, which is 46.4%; 2026W36 was 46.0%.
So the figure a reader meets in the front-door file is six and a half points low and still
climbing.

Nothing is left to start: the ruling answered the question and the edit it asked for was calef's
commit. What follows is the gate as it read before 2026-09-24. `AGENTS.md` is calef's file and the
correction is a one-line edit only he makes.
The measurement is already done and stored, so nothing else is owed: what is missing is the edit and
a decision about whether a hand-copied number belongs in that file at all.

**In brief.** `AGENTS.md` tells a reader that `kernel/src` "measures 40% of them" when discussing
comments in the line count. Milestone 234's weekly series measures that ratio directly: 39.3%
eight weeks ago and 45.3% now, climbing every week. The figure was true when it was written and is
not true today. The work is to correct it, and to decide whether the sentence should quote a number
at all or point at `notes/project-metrics/weekly.csv`, which cannot go stale.

## Why this matters

This is the project's front-door file. A newcomer reads it before they read anything else, and
AGENTS.md's own third principle is that a stranger must be able to succeed on the tree alone. A
wrong number there is the exact defect that principle names: *"a newcomer who hits a limitation the
docs named will trust the docs. One who hits a limitation the docs hid will not trust anything
again."*

It is also drifting rather than merely stale. Correcting 40% to 45.3% buys a few weeks and then the
same bullet is true again, which is why the second half of this is a question and not just an edit.
A pointer at the series is rung one, since the series recomputes itself; a hand-copied percentage is
rung four and has now demonstrated its failure mode.

## Where it came from

Milestone 234's `## Follow-on`: *"Correct AGENTS.md's 'kernel/src measures 40% of them' comment
ratio. The series puts it at 39.3% eight weeks ago and 45.3% now, climbing every week, so the figure
a reader meets in the project's own front-door file is wrong and drifting further from true.
AGENTS.md is calef's file, so the one-line edit is his to make; this page flags the staleness and
cannot fix it."*

## Index row

`AGENTS.md` tells a reader that `kernel/src` "measures 40% of them" when discussing comments in the
line count, and milestone 234's weekly series measures that ratio directly: it was 39.3% eight weeks
before the proposal was written, 45.3% when it was written, and 46.4% on 2026-09-14. The figure was
true when it was written and is not true now. This is the project's front-door file, read before
anything else, and AGENTS.md's own third principle is that a stranger must be able to succeed on the
tree alone, which makes a wrong number there the exact defect that principle names. It is drifting
rather than merely stale, so correcting 40% to the current figure buys a few weeks and then the same
bullet is wrong again; that is why the second half of this is a question rather than an edit. A
pointer at `notes/project-metrics/weekly.csv` is rung one, because the series recomputes itself, and
a hand-copied percentage is rung four and has now demonstrated its failure mode twice. `AGENTS.md` is
calef's file, so the one-line edit is his.
