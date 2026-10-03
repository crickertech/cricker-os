---
status: PROPOSED
raised: 2026-10-03
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# The merge queue reports its ejection share and its time to merge

Raised 2026-10-03 (UTC) by the lane revising
[the merge-rate correction](../../../notes/coes/2026-10-03-the-merge-rate.md) after calef's second
review of #1513. Name provisional, this file's alone.

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

## Index row

The weekly metrics carry the merge queue's ejection share and median time to merge, fetched by the workflow, and trunk-health reports a day over 20%; the share went from 8% to 43% unread.
