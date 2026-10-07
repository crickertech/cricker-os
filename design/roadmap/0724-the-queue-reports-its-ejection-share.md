---
status: BUILT
raised: 2026-10-03
built: 2026-10-03
promoted_from: the-queue-reports-its-ejection-share
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 724. The merge queue reports its ejection share and its time to merge

Raised 2026-10-03 (UTC) by the lane revising
the merge-rate correction (`notes/coes/2026-10-03-the-merge-rate.md`, on PR #1513's branch until it lands) after calef's second
review of #1513. Number 724 is provisional until the queue lands it; title and slug are drafts.

## Why

calef asked whether merges were already slow before the Claude allowance ran out. They were, and
nothing in the tree could have said so. From GitHub's merge-queue events:

| days (UTC) | entries | ejected | share | median hours, opened to merged | group runs per merge |
|---|---|---|---|---|---|
| 09-15 to 09-19 | 134 | 11 | 8% | 0.9 | 1.05 |
| 09-22 to 09-27 | 551 | 235 | 43% | 2.4 | 1.80 |

The worst day was 09-22: 81 of 93 entries ejected. Milestone 630 (a merge-queue ejection is caught
before the queue, and recovered after it) handles each ejection as it happens. Nothing counts them,
so a week in which the share went from one in twelve to nearly half read as an ordinary busy week.

## What to build

1. A daily number: the share of merge-queue entries that left without merging, and the median
   hours from opening to merge, from the GitHub API. `script/metrics` reads git and nothing else,
   so this is the weekly metrics workflow's to fetch and pass in, the way coverage is passed in
   with `--coverage-from`. A new carried column, or its own CSV: an architect's call.
2. A chart on `notes/project-metrics.md`, beside merged pull requests.
3. `trunk-health` reports a day whose ejection share passes 20%, which no day from 09-15 to 09-19
   reached and every day from 09-22 to 09-26 did.

## What shipped

- `helpers/merge_queue_share.py`: `update` fetches GitHub's merge-queue events (GraphQL) and upserts
  `notes/project-metrics/merge-queue-daily.csv`: entries, ejections, share, merges and the median
  hours from opening to merge, per UTC day. `report` prints a warning for any recent day with at
  least ten entries and a share over 20%. Selftest is in `script/lint`.
- `script/metrics` sums the daily file into three weekly columns (`merge-queue.csv`) and draws
  `merge-queue.svg`, with the 20% line, on `notes/project-metrics.md` beside the merged
  pull requests.
- `metrics.yml` runs `update` before `script/metrics --update`. `trunk-health.yml` runs `report`.

**How the number reaches `script/metrics`, which the proposal left open.** Not as a carried column
and not as a flag. The fetch is an API call and `script/metrics` reads git and committed records, so
the helper writes a committed record and the script reads it the way it already reads `effort.csv`.
That is the cheapest honest path: no change to the carried-field machinery, one extra step in a
workflow that already runs daily, and a week the fetch never covered is empty rather than zero.
Refused: `--coverage-from`'s shape, because it passes one value per run and this is a daily series.

**The daily file reproduces the proposal's figures.** Entries and ejections for 2026-09-22 to 09-27
are 551 and 235 (43%), and 2026-09-22 alone is 81 of 93, all exact. 2026-09-15 to 09-19 gives 134
entries and 12 ejections (9%) against the proposal's 11 (8%): one event of difference, probably a
group counted at a day boundary. The weeks read 16% (2026W38), 41% (W39) and 26% (W40, partial).

## BUGS

- The median hours to merge is in the daily file only, not charted. A weekly median cannot be built
  from daily medians, and a chart of the daily series does not fit a chart module that draws one
  bar per week.
- Three pull requests have more than 100 queue events (the most is 119), and the helper reads the
  newest 100. Their early September days are undercounted; the fetch says so on stderr.
- `trunk-health` re-announces a bad day on every run, for the reason its own BUGS gives for a red
  trunk. Warning only.
- The paging stops at the first pull request last updated before the window, on the premise that a
  queue event updates it. The reproduction above supports the premise for September.
- 2026-10-02 has no row because nothing entered or left the queue, which is a day the allowance ran
  out and not a gap in the fetch.

## Follow-on

- **Recorded.** The median hours to merge has no chart, in this block's BUGS. A daily chart is a
  second chart module, and an architect's call whether the page wants one.

## Index row

BUILT on `milestone/724-ejection-share` (PR #1538). The weekly metrics carry the merge queue's ejection share and median time to merge, fetched by the workflow, and trunk-health reports a day over 20%; the share went from 8% to 43% unread.
