---
status: PROPOSED
raised: 2026-10-07
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# The Actions cache horizon is charted

Written by lane/actions-cache-budget, from its measurement in
[notes/actions-cache-budget-2026-10-07.md](../../../notes/actions-cache-budget-2026-10-07.md).

## In brief

A daily scheduled sample of the repository's Actions cache: total bytes against
GitHub's 10 GB limit, entry count, and the eviction horizon (hours since the least recently used
entry was last accessed). Appended to a committed CSV the way `merge-queue-daily.csv` is, and charted
weekly in notes/project-metrics.md by `script/metrics`.

## Why

The cache passed its limit and nobody saw it until #1810 went looking for Kani's install
time. By then the horizon was 12 hours, merge groups were writing 3.8 GB nobody could restore, and
`main` had no rust-cache entry at all. Bytes alone mislead, because an LRU cache sits at its limit
by design. The horizon is the number that says whether an entry lives long enough to be used twice.

## Done when

The workflow has run on at least three days, the chart shows bytes and horizon, and
notes/project-metrics.md says under the chart what a falling horizon means and which note to read.

## BUGS, expected

The cache list changes while it is paginated (469 entries listed against a usage
count of 455 on 2026-10-07), so a sample is approximate to a few entries.

**Reuse:** the merge-queue daily sampler's workflow and CSV shape, and `script/metrics`'s existing
chart path. Nothing new is written beyond the sampler's few lines of `gh api`.
