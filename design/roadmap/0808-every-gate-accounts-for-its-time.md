---
status: PARTIAL
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

Run 37505448674 on 2026-10-06: 85 of `script/bootstrap`'s 92 seconds in the `test` job were
`cargo install` building `script/lint`'s tools. Six jobs that never lint did it, about 8.5
runner-minutes per merge-group run. It was cheap and reversible, so it did not wait for this milestone:
`NIFE_SKIP_LINT_TOOLS` (#1778) removed it.

## What catches the next drift before the wall-time gate fires

The drift PR #1775 found was spread across about a hundred tests, which a per-test ceiling would not
have seen. This block recommended a per-suite budget; calef refused it (§254, Fork 2), because a
second hard limit fails whichever pull request tips an aggregate over, as `cpu-matrix`'s budget did
to #1755 and #1766. Milestone 721's job budget stays the only hard limit. In its place, a warning
that fails nothing:

- A job past about 85% of its budget in `.github/ci-job-budgets` (17 of 20 minutes) warns and stays
  green. Its step summary shows its time against the budget and its slowest items.
- A routine running as `nife-smelter[bot]` opens one labeled tracking issue per job when it first
  crosses the line, updates it, and closes it when the job is back under. A label needs an object,
  and a commit on `main` has none, which is why the signal is an issue.
- calef ruled on 2026-10-06 (UTC) that the issue must reach a session (§254). It is a
  `needs-maintainer` cause, read with one command that lists pull requests and issues together, and
  a bullet under the weekly job-time chart while it is open.

At the measured growth of about 0.1 minutes per model per day, the longer `cpu-matrix` shard would
have opened its issue about three weeks before the hard limit. A `script/lint` check holds the rule
itself: a `ci.yml` job that checks out the tree uploads the breakdown file or has an exemption line
with its reason.

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

Taken: GitHub's jobs API for per-step times, `actions/upload-artifact` for the file, and the step
summary for display. Followed: `helpers/verify_times.py` and `helpers/merge_queue_share.py` for the
daily record, and `helpers/job-budget.py` itself for the budget, the line and the job parser, which
`helpers/ci_job_times.py` imports rather than copies. The issue routine calls `gh issue` directly;
`helpers/watcher_watch.py` (milestone 723 (a stopped merge watcher is reported within three of its own intervals)) was read and is shaped around watchers. Written: the collector, the warning and the issue routine, a few hundred lines of Python
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

## What was built

Lane `lane/808-gate-time`, pull request #1825, from 2026-10-07 (UTC), cut from `main` rather than
stacked on milestone 807 (the kernel suite reports what each test cost), because these pieces do not
read 807's record. Every name below is provisional.

- **The warning line.** `helpers/job-budget.py check` warns at 85% instead of 75%, so there is one
  line, and writes the job's minutes against its budget to the step summary. Its selftest proves the
  warning fires at 17 of 20 minutes, is quiet below, and never changes the exit status.
- **The daily record.** `helpers/ci_job_times.py update` reads every merge-group run of `ci.yml`
  and `verify.yml` from the jobs API and writes per-job and per-step daily medians to
  `notes/project-metrics/ci-job-times.csv`, seeded from 2026-10-06. `metrics.yml` runs it daily.
- **The chart.** `script/metrics` draws `ci-jobs.svg`: `test`, `cpu-matrix`, `fuzz` and
  `swish-check` side by side, the slowest job as a line, and the 20-minute ceiling.
- **The tracking issue.** `helpers/ci_job_times.py sync` opens one `near-budget` issue per job,
  rewrites it daily, closes it, and appends a week-notes row while it is open. It runs in
  `metrics.yml` as `nife-smelter[bot]`. Its selftest drives the three transitions against a
  recorded merge-group run (37648588582).
- **The route to a session.** A thirteenth `needs-maintainer` cause, `budget`, keyed on the issue's
  number and opening time. The drain finds open budget issues through a search of their own and
  closed labeled issues through its existing search, which gained an `... on Issue` arm.
  `briefs/session-start.md` and `briefs/survey-the-queue.md` read the listing through the REST
  issues endpoint.

### The damping, proposed as an amendment to §254

§254's Open section left it to this lane. **One run at or past the line opens the issue, and three
runs in a row under it close it.** Opening on one run keeps the warning as early as the ruling
wants, and it fails nothing. Closing on one would flap, since one job on one commit varies by about
30% across hosted runners. The maintainer applies the amendment to §254; this lane does not edit
`design/decisions/`.

## Exit criteria, so far

| | criterion | state |
|---|---|---|
| 1 | the daily CSV for `ci.yml` and `verify.yml`, and the weekly chart | met, per job and per step. Per test waits on 807's artifacts |
| 2 | every checkout job uploads the breakdown file or is exempt, held by `script/lint` | not yet: needs 807's file on `main` |
| 3 | per-item times from the kernel suite, `cpu-matrix`, `swish-check`, re-falsify and `fuzz` | the kernel suite and `cpu-matrix` by 807; the other three not yet |
| 4 | the 85% warning, its summary, its host test | met |
| 5 | the issue's three transitions, tested against a recorded fixture | met |
| 6 | `budget` in the `needs-maintainer` listing, the selftest, the briefs | met |
| 7 | a week-notes bullet while an issue is open | met, through `sync` |
| 8 | `verify.yml` joins the budget | not yet: the record now holds its medians. Over 60 runs to 2026-10-07, `prove` shards ran a 16.4-minute median (max 19.2). Re-falsify shards ran 16.1 (max 21.5) and need a ratchet line |
| 9 | milestone 663's deadline from the host pass's measured time | not yet: needs the per-phase file |

## BUGS

- The warning reads one run, and `sync` damps the issue (one run opens, three close). A job
  hovering at the line keeps one issue open rather than flapping, and may hold it open longer than
  its median deserves.
- `sync` runs once a day, so an issue opens up to a day after the crossing run.
- `helpers/nanny.py`, which wakes a running session, still reads pull requests only. A budget issue
  reaches a session at its next `needs-maintainer` read, not mid-session.

- A merge-group run's runner varies, and every number here is wall time on a shared hosted machine.
  The record takes medians over runs for that reason. One run, as in the table above, is a premise
  check and not a baseline.

## Follow-on

- **Outstanding.** Exit criteria 2, 3, 8 and 9 above, in this lane after milestone 807 merges. That
  is the per-phase timing of `script/ci-build test`, the upload-or-exempt lint, and per-item times
  from `swish-check`, re-falsify and `fuzz`. Then the daily CSV's per-test rows, `verify.yml`'s
  budget, and 663's deadline.

## Index row

Milestone 721 gave every CI job a wall-time budget that says a job is slow but not why. This makes every gate report where its time goes, per step, per phase and per item, in a step summary and an artifact, with a daily record and a weekly chart against the budgets. A job on main past about 85% of its budget warns and opens a labeled tracking issue rather than failing (§254), so the next drift is named weeks before it ejects a pull request.
