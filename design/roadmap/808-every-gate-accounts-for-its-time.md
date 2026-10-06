---
status: NOT-STARTED
raised: 2026-10-06
promoted_from: every-gate-accounts-for-its-time
milestone_dependencies: 807
decision_dependencies: 254
machine_requirements: none
specific_machine: none
needs_person: no
---
# 808. Every gate accounts for its time

*(Promoted from the proposal pile on 2026-10-06 (UTC); number provisional until the merge queue lands it.)*

*(Ruled 2026-10-06 (UTC), calef on PR #1778, four forks. §254 (a gate prints what each item cost,
and a job near its budget warns rather than fails) records them. Fork 2 went against this block's
recommendation: per-suite budgets are refused, and a soft warning with a tracking issue replaces
them. The text below is rewritten to the rulings.)*

Raised 2026-10-06 (UTC) by lane `gate-time-proposal`, a writing-only lane. calef asked for a
proposal measuring where the riscv64 suite's time goes, then: "Is there a more general milestone
here? Don't we want all of the gates to account for how they spend their time?" The maintainer
agreed. This is the general milestone. Its first slice, and the riscv64 answer, is
milestone 807 (the kernel suite reports what each test cost), which this block depends on. Title
and slug are drafts.

## Why

Milestone 721 (each merge-group CI job has a 20-minute budget) gave every `ci.yml` job one number,
its wall time, and a gate on it. That gate works. On 2026-10-06 it fired on `cpu-matrix` at 20.1 to
21.0 minutes and ejected pull requests from the queue, which is what PR #1775 answered by splitting
the job in two.

A wall-time gate can say that a job is slow. It cannot say why. To answer that, PR #1775 sampled 31
runs over seventeen days and read per-step times from the Actions API. Per-model and per-test times
had to be read from log timestamps. Its finding was that no step regressed and the suite grew,
across many milestones, at about 0.1 minutes per model per day. The gate fired about two weeks into a drift
that every run in the window showed.

So the rule this milestone builds: a gate that spends time says where, in a form a script can
read, without anyone scraping a log.

## What the tree already accounts for

Some gates do this already, each its own way:

| gate | what it records | where |
|---|---|---|
| every `ci.yml` job | its own wall time, against 20 minutes | `helpers/job-budget.py`, `.github/ci-job-budgets` (milestone 721) |
| `swish-check` | seconds per line, per leg, against a baseline | `xtask/src/swish_check.rs`, milestone 722 (swish-check fails a leg that costs five times the others per line) |
| re-falsify | seconds per harness, used to balance shards | `notes/project-metrics/falsification-times.tsv` |
| `fuzz` | 60 s per target, fixed by design | `script/fuzz` |
| the kernel suite | seconds per test, printed only at 5 s and over | `kernel/src/testing.rs` |
| `cpu-matrix` | one log per model, as an artifact | `ci.yml` |

Nothing reads CI time over weeks. `script/metrics` charts the merge queue's ejections, from milestone 724 (the merge queue reports its
ejection share and its time to merge), but not the wall time of the jobs that cause them. `verify.yml` is outside milestone 721's
budget because nobody measured its job medians, which is that milestone's first BUGS entry.

## What "accounts for its time" means

Three levels. Each gate provides the ones that apply to it.

1. Per step, for every job. GitHub's jobs API already returns each step's start and end, so this
   needs no change to any job. A daily collector reads it for green merge-group runs and appends a
   committed record. It follows `helpers/merge_queue_share.py`, which writes
   `merge-queue-daily.csv` the same way for milestone 724.
2. Per phase, inside a step that does several things. `script/ci-build test` runs fifteen phases
   in one step: the host pass, `std-src`, three architectures, three x86_64 variants and the boot
   tour. `xtask` times each phase on the host and writes it to a machine-readable file per job
   (name provisional), uploaded as an artifact.
3. Per item, for a gate made of many items: tests per architecture, Kani harnesses, fuzz targets,
   `cpu-matrix` models, `swish-check` lines. The item's own framework reports the time, never a log
   scrape. For the kernel suite that is milestone 807, in §254's `time <ms> <test path>` format. For `swish-check` and re-falsify it
   already exists and only moves into the shared file.

## What the first read of per-step data already found

Run 37505448674 on 2026-10-06, the `test` job, 14.9 minutes, read from the API and its log:

| phase | seconds |
|---|---|
| checkout, caches, QEMU from cache | 58 |
| `script/bootstrap` | 92 |
| host tests and the redoxfs host passes | 89 |
| `std-src` | 60 |
| aarch64 suite | 134 |
| riscv64 suite | 161 |
| x86_64 suite, q35 | 93 |
| x86_64, AMD-Vi | 79 |
| x86_64 under OVMF, boot and both suites | 108 |
| the rest | 23 |

The surprise is `script/bootstrap`. 85 of its 92 seconds go to `cargo install` building
`cargo-machete` (30.4 s) and `typos` (55.1 s) from source. Those are `script/lint`'s tools, and
lint runs in `clippy`, not here. The same run built both in six jobs: `test`, `cpu-matrix`,
`bench`, `boot-check`, `swish-check` and `swish-check-x86_64`. That is about 8.5 runner-minutes per
merge-group run, and 1.4 minutes on every one of those poles, `cpu-matrix` included. It is larger
than any single test fix the riscv64 answer suggests.

It is cheap and reversible, so it should not wait for this milestone. Caching the two binaries, or
giving `script/bootstrap` a way to skip the lint tools in a job that never lints, removes it.
calef's 2026-09-13 ruling in `script/ci-build` (bootstrap runs first on the no-argument path) is
about the local path and is not touched by either. This lane did not build it; it is the first
instance of the mechanism finding something.

## What catches the next drift before the wall-time gate fires

The drift PR #1775 found was spread across about a hundred tests. A per-test ceiling would not have
seen it. A per-suite budget would have, and this block recommended one; calef refused it (§254,
Fork 2). A second hard limit fails whichever pull request tips an aggregate over, as `cpu-matrix`'s
20-minute budget did to #1755 and #1766. Milestone 721's job budget stays the only hard limit.

What this milestone builds instead is a warning that fails nothing:

- When a job's run on main passes about 85% of its budget in `.github/ci-job-budgets` (17 of 20
  minutes today), the job emits a warning and stays green. `helpers/job-budget.py check` already
  measures every job against that file, and already prints a warning at 75% (15 minutes) on every
  run, pull requests included, where nobody reads it. The lane moves that line to 85% so the tree
  has one warning line, not two, and says so in the helper's header.
- That run's step summary shows the job's time against its budget, and its slowest suites and
  tests from the per-item records. The reader starts from the answer rather than from the log.
- A routine running as `nife-smelter[bot]` opens one tracking issue per job when the job first
  crosses the line, updates it on later runs, and closes it when the job drops back under. The
  issue carries a label (name provisional). A label needs an object, and a commit on main has
  none, which is why the signal is an issue.
  Milestone 723 (a stopped merge watcher is reported within three of its own intervals) opens an
  issue the same way: `helpers/watcher_watch.py sync` opens or closes one issue per watcher, and
  is the one to reuse.

A label alone is read by nothing: today a session's queue is `needs-maintainer`, which lists pull
requests only, and nobody opens the step summary of a green run on main. calef ruled on 2026-10-06
(UTC) that the issue must reach a session by two routes (§254):

- **The `needs-maintainer` listing.** `helpers/needs-maintainer.jq` gains an eighth cause,
  `budget` (name provisional): an open issue carrying the budget label. Its key is the issue's
  number and the time it was opened, so the drain comments once per episode, and it clears the
  label when the issue closes. The drain's `NM_QUERY` in `helpers/merge-drain.sh` adds open issues
  with that label, through the `search` it already runs with an `... on Issue` arm. The selftest
  and `helpers/needs-maintainer-fixtures/every-cause.json` gain a labeled issue, open and closed.
- **One query for the session, not two.** `briefs/session-start.md` and
  `briefs/survey-the-queue.md` change from `gh pr list --label needs-maintainer --state all` to
  `gh api --paginate 'repos/nifeos/nife/issues?labels=needs-maintainer&state=all'`, which returns
  pull requests and issues together (a row with `pull_request` set is a pull request). A separate
  `gh issue list` beside the existing command was refused: a second command is one more thing a
  session must remember, which is rung zero. `gh search` was refused for the search index's lag,
  which is recalled, not measured.
- **A bullet under the chart.** While a budget issue is open, the weekly job-time chart in
  `notes/project-metrics.md` carries a bullet under it naming the job, its percentage of budget and
  the issue number. The routine appends the row to `notes/project-metrics/week-notes.csv`, so it
  renders and ages out under milestone 623 (bullet under the chart explains a cliff)'s rule, with
  no new mechanism.

At the measured growth of about 0.1 minutes per model per day, the longer `cpu-matrix` shard would
have opened its issue about three weeks before it reached the hard limit. Each issue is a small
recorded decision: fix something, split the job, or raise the budget with 721's reason line.

A `script/lint` check holds the rule itself: a `ci.yml` job that checks out the tree either uploads
the breakdown file or has a line in an exemption list with its reason. That is the same move
`helpers/job-budget.py lint` makes for the wall-time steps, so a new gate cannot arrive without
accounting.

## Where it lands

- A CI artifact per job, every run: the per-phase and per-item file.
- The job's step summary, every run: the slowest items and the phase table (§254, Fork 3). It costs
  nothing unless someone opens it. No pull request comment.
- A daily CSV under `notes/project-metrics/` (name provisional), appended by a scheduled workflow
  on milestone 724's `helpers/merge_queue_share.py` pattern: per-job, per-step and per-test times on
  main (§254, Fork 4).
- A weekly chart from `script/metrics`: each merge-group job's time against its milestone 721
  budget. It is the chart milestone 721 said was missing.
- The 85% warning and the tracking issue above. Neither fails a job.

The runner-minutes measurement pass calef approved on 2026-10-05 is a one-off report on
concurrency, and it stays owed. The daily record makes it repeatable by script.

## Parity

The per-step record and the warning are host-side, so they are architecture-neutral. The per-test
record is the kernel's shared test framework and lands on aarch64, riscv64 and x86_64 at once
(milestone 807). The step summary and the chart show suites per architecture, because the three
emulators cost differently: riscv64 system tests took 129 s against aarch64's 98 s in the run
above. A suite that runs on fewer than three architectures says why in its exemption line, which is
§19 (architectural parity is a tenet) applied to cost. Milestone 722 already compares cost across architectures for `swish-check`.

## Exit criteria a stranger can check

1. `notes/project-metrics/` holds a daily CSV of per-job, per-step and per-test times for green
   runs on main of `ci.yml` and `verify.yml`, appended by a scheduled workflow on the pattern of
   `helpers/merge_queue_share.py`. `script/metrics` charts each job's weekly time against its
   milestone 721 budget.
2. Every `ci.yml` job that checks out the tree uploads the breakdown file or carries an exemption
   line with a reason, and `script/lint` fails a job that does neither.
3. The kernel suite on all three architectures, `cpu-matrix`, `swish-check`, re-falsify and `fuzz`
   write per-item times into that file from their own frameworks.
4. A job's run on main past about 85% of its budget emits a warning, stays green, and lists its
   slowest suites and tests in the step summary. A host test proves the warning fires above the
   line, stays silent below it, and never changes the exit status.
5. The routine opens a labeled tracking issue for a job that crosses the line, updates the same
   issue on a later crossing rather than opening a second, and closes it when the job is back
   under. A host test drives all three transitions against a recorded API fixture.
6. An open budget issue appears in the session's `needs-maintainer` listing with cause `budget`,
   and leaves it when the issue closes. `helpers/needs-maintainer-selftest.sh` proves both from
   fixtures, and `briefs/session-start.md` and `briefs/survey-the-queue.md` give the one combined
   query.
7. While a budget issue is open, the weekly job-time chart carries a bullet naming the job, its
   percentage of budget and the issue number, written through `week-notes.csv`, and the bullet
   ages out with its week.
8. `verify.yml` joins milestone 721's budget, using the medians the record now has, which closes
   that milestone's first BUGS entry.
9. Milestone 663 (bound the host pass) takes its deadline from the host pass's measured time in
   the record, as its block asks.

## Cost

- Runner-minutes: the daily collector is one short job a day on the x86_64 pool, which is not the
  constrained one. Writing and uploading the file costs seconds per job. The step summary costs
  nothing. The bootstrap fix found on the first read returns about 8.5 arm64 runner-minutes per
  merge-group run.
- Claude tokens: zero per pull request, by design. Nothing is posted as a comment (§254, Fork 3).
  The cost lands only when a tracking issue opens, which a session reads once and acts on.

## Ranking

This is not one of the nine fatal risks, and it is not on the customer path, which is vacant. It
serves principle 2: queue throughput is the method, and the bottleneck since 2026-08-04 has been how
fast one merge queue lands. A wall-time gate that fires late ejects pull requests, which is where
lanes and tokens are lost. So it ranks below work that moves a fatal-risk verdict and above work
that does neither. The bootstrap fix and the calendar fix are each under an hour of lane work and
should go now on cost alone. The first slice should go when a lane frees. The rest can wait for it.

## The forks, ruled by §254

All four were ruled on 2026-10-06 (UTC), in the order this block asked. §254 has calef's words and
the refused options.

1. The kernel record's format: the printed `time <ms> <test path>` block, grammar in a shared crate.
   Milestone 807 builds it.
2. What gates drift: a soft warning at about 85% of the milestone 721 budget, raised as a labeled
   tracking issue per job. The per-suite budget this block recommended was refused.
3. Where the per-run breakdown shows: step summary plus artifact, no pull request comment.
4. The daily record and chart: a CSV under `notes/project-metrics/` on milestone 724's pattern,
   charted weekly by `script/metrics`.

## Reuse

Taken: GitHub's jobs API for per-step times, `actions/upload-artifact` for the file, the step
summary for display, and the patterns of `helpers/merge_queue_share.py` (a daily record) and
`helpers/job-budget.py` (a ratchet with a lint), and `helpers/watcher_watch.py` (milestone 723) for the
tracking issue. Written: the collector, the warning and the issue routine, a few hundred lines of Python
against this tree's own record formats. JUnit XML was
considered as the file's format and not taken, because no consumer here reads it.

## What was considered and lost

- Splitting more jobs into shards each time a budget fires. It shortens the pole and spends the
  scarce resource, arm64 runner concurrency. PR #1775's own table prices five shards at 87% more
  runner-minutes.
- Raising budgets. It moves the day the gate fails.
- A per-suite, per-architecture budget ratchet, which this block recommended. calef refused it
  (§254, Fork 2): a second hard limit ejects whichever pull request tips the sum.
- cargo-nextest for host tests, which reports per-test time and JUnit XML. It is a dependency, which
  §46 (thin primitives or whole subsystems) makes a decision, and the host pass is 89 of 900
  seconds. libtest's own JSON output with per-test times, behind `-Z unstable-options` on nightly,
  is the in-tree route if the host pass ever matters. That flag is recalled, not re-read.

## BUGS

- The warning reads one run. A job sitting near 85% may open and close its issue on alternate runs.
  §254 leaves the damping to this milestone's lane.

- A merge-group run's runner varies, and every number here is wall time on a shared hosted machine.
  The record takes medians over runs for that reason. One run, as in the table above, is a premise
  check and not a baseline.

## Index row

Milestone 721 gave every CI job a wall-time budget that says a job is slow but not why. This makes every gate report where its time goes, per step, per phase and per item, in a step summary and an artifact, with a daily record and a weekly chart against the budgets. A job on main past about 85% of its budget warns and opens a labeled tracking issue rather than failing (§254), so the next drift is named weeks before it ejects a pull request.
