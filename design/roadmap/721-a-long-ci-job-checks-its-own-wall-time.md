---
status: BUILT
raised: 2026-10-03
built: 2026-10-03
promoted_from: a-long-ci-job-checks-its-own-wall-time
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 721. Each merge-group CI job has a 20-minute budget

Raised 2026-10-03 (UTC) by the maintainer session writing
the merge-rate correction (`notes/coes/2026-10-03-the-merge-rate.md`, on PR #1513's branch until it lands). The budget is
calef's ruling the same day: "I think 20 minutes is a decent target", then "Yes, revise the COE to
the 20 minute budget". Number 721 is provisional until the queue lands it; title and slug are drafts.

## Why

Nothing in the tree reads CI wall time: not `script/metrics`, not `notes/project-metrics.md`, not
`script/bench`, not `trunk-health`, not `merge-drain`. The long merge-group job went from 12 to 22
minutes in one merge on 2026-09-19 (#987) and stayed there. When it hit its 30-minute bound on
2026-09-26 the bound was raised to 45 (commit 2556dd307). On 2026-09-30 the job ran into the new
bound 29 times, and only notes landed for 18.2 hours.

The bound was ours to raise. `timeout-minutes` in `ci.yml` is this tree's setting; GitHub ends a
hosted job only at six hours. So raising it was one line and cost nothing, which is why it was the
answer.

A 20-minute budget would have fired at 2026-09-19 21:13, on the first 22.3-minute run, about 10.5
days before the stop. Relative rules were measured as well (1.5 times the trailing median fires,
twice does not) and lost to an absolute number because a reader can check a number.

## What to build

- Each merge-group CI job has a budget of 20 minutes of wall time.
- A final step, `if: always()`, reads the job's start time. At 15 minutes it warns.
- Above 20 minutes it fails, unless a committed ratchet file (name provisional) raises that job's
  budget and states the reason in the same diff. A budget can grow, but only by saying why.
- `timeout-minutes` drops to 25 on every merge-group job, the budget plus margin, so creep stops a
  job at 25 minutes instead of burning 45 a group.

## Where main stands

The last 25 green merge-group CI runs, created 03:25 to 08:00 UTC on 2026-10-03 (`gh run list
--event merge_group --workflow CI`, then per-job start and completion), jobs over two minutes:

| job | median, min | max, min |
|---|---|---|
| cpu matrix | 15.0 | 16.6 |
| test | 10.3 | 11.4 |
| fuzz | 7.0 | 7.5 |
| clippy | 4.9 | 5.4 |
| swish-check | 4.6 | 23.6 (one outlier) |
| swish-check-x86_64 | 3.9 | 5.5 (17 runs; a newer job) |
| swish-check-graphical | 3.0 | 3.4 |
| boot-check | 2.6 | 3.0 |
| bench | 2.2 | 3.5 |

Main meets the budget. cpu matrix has about 3.4 minutes of headroom at its maximum and will hit it
first; splitting it across jobs is the expected remedy. The swish-check outlier would have warned
and failed once, which is the check working: one 23.6-minute run is worth a look.

## What shipped

`helpers/job-budget.py` (`check`, `lint`, `--selftest`), `.github/ci-job-budgets` (the ratchet, empty),
the two steps in every `ci.yml` job that checks out the tree, and a `script/lint` section that fails
a job added without them. `timeout-minutes` is 25 on every `ci.yml` job (from 30 to 60 on the long
ones), 15 and 20 where it was already lower. `gate` has no checkout, so it has the timeout and not
the two steps. Two deviations from the proposal, both small. The clock is a first step writing `JOB_STARTED_AT`,
not a read of the API's job start, because the API names a job by display name and a matrix leg
shares its key. The ratchet is a plain text file, because the lint runs on a bare checkout with no
YAML library.

## BUGS

- Only `ci.yml` is covered. `verify.yml` also runs on `merge_group` with 45 and 60 minute bounds, but
  no median for its jobs was measured, so a 25-minute cap there would be a guess.
- The first deploy may not hold: the `cpu-matrix` maximum is 16.6 minutes, and a cold QEMU build the
  old comments call the honest worst can add 10 to 20. A job that fails on that gets a ratchet line
  with its reason, which is the mechanism working.
- A job killed at 25 minutes reports the timeout, not the budget, because its last step never runs.

## Follow-on

- **Recorded.** `verify.yml` is not under the budget, because its job medians have not been measured. The first BUGS entry above is where a reader meets it, and the fix is adding the file to `WORKFLOWS` in `helpers/job-budget.py` once they exist.

## Index row

BUILT on `milestone/721-job-budget` (PR #1530). Each merge-group CI job has a 20-minute budget, warns at 15, fails above 20 unless a committed ratchet raises it with a reason, and times out at 25 rather than 45; calef ruled it 2026-10-03.
