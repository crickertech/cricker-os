---
status: PROPOSED
raised: 2026-10-06
milestone_dependencies: none
decision_dependencies: unwritten
machine_requirements: none
specific_machine: none
needs_person: no
---
# Every gate accounts for its time

Raised 2026-10-06 (UTC) by lane `gate-time-proposal`, a writing-only lane. calef asked for a
proposal measuring where the riscv64 suite's time goes, then: "Is there a more general milestone
here? Don't we want all of the gates to account for how they spend their time?" The maintainer
agreed. This is the general milestone. Its first slice, and the riscv64 answer, is
`the-kernel-suite-reports-what-each-test-cost.md`. Title and slug are drafts.

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

So the rule this milestone proposes: a gate that spends time says where, in a form a script can
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
   scrape. For the kernel suite that is the first slice. For `swish-check` and re-falsify it
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
seen it: only two new tests take about 10 seconds, and the rest of the growth is many small ones. A
budget on the sum would have. So the mechanism is a ratchet one level below the job, of the same
shape as `.github/ci-job-budgets`:

- Each multi-item suite has a committed budget per architecture, in seconds, set from its measured
  median with headroom. The kernel `system_tests` on riscv64 is the first.
- Over budget fails the job, and the failure prints the five slowest items and the five that grew
  most against the last record. The reader starts from the answer rather than from the log.
- A budget grows only by a line in the same diff that says why, as 721's ratchet does.

At the measured growth of roughly 5 s a day in riscv64 system tests, 20% headroom fires about every
five days. Each firing is a small recorded decision: raise the line with a reason, or fix
something. The job-level gate stays as the backstop. This one fires early and names the suite.

A `script/lint` check holds the rule itself: a `ci.yml` job that checks out the tree either uploads
the breakdown file or has a line in an exemption list with its reason. That is the same move
`helpers/job-budget.py lint` makes for the wall-time steps, so a new gate cannot arrive without
accounting.

## Where it lands

- A CI artifact per job, every run: the per-phase and per-item file.
- The job's step summary, every run: the five slowest items and the phase table. It costs nothing
  unless someone opens it.
- A weekly chart in `notes/project-metrics.md`: the median wall time of each merge-group job, from
  the daily record. It is the chart milestone 721 said was missing.
- The suite budgets above, as a gate.

The runner-minutes measurement pass calef approved on 2026-10-05 is a one-off report on
concurrency, and it stays owed. The daily record makes it repeatable by script.

## Parity

The per-step record and the ratchet are host-side, so they are architecture-neutral. The per-test
record is the kernel's shared test framework and lands on aarch64, riscv64 and x86_64 at once. Each
suite budget is per architecture, because the three emulators cost differently: riscv64 system
tests took 129 s against aarch64's 98 s in the run above. A suite that runs on fewer than three
architectures says why in its exemption line, which is §19 (architectural parity is a tenet)
applied to cost. Milestone 722 already compares cost across architectures for `swish-check`.

## Exit criteria a stranger can check

1. `notes/project-metrics/` holds a daily per-job, per-step record for green merge-group runs of
   `ci.yml` and `verify.yml`, written by a scheduled workflow. `script/metrics` charts the weekly
   median per job.
2. Every `ci.yml` job that checks out the tree uploads the breakdown file or carries an exemption
   line with a reason, and `script/lint` fails a job that does neither.
3. The kernel suite on all three architectures, `cpu-matrix`, `swish-check`, re-falsify and `fuzz`
   write per-item times into that file from their own frameworks.
4. The riscv64 `system_tests` budget exists. A host test proves the gate fails over budget and
   prints the five slowest and the five that grew most.
5. `verify.yml` joins milestone 721's budget, using the medians the record now has, which closes
   that milestone's first BUGS entry.
6. Milestone 663 (bound the host pass) takes its deadline from the host pass's measured time in
   the record, as its block asks.

## Cost

- Runner-minutes: the daily collector is one short job a day on the x86_64 pool, which is not the
  constrained one. Writing and uploading the file costs seconds per job. The step summary costs
  nothing. The bootstrap fix found on the first read returns about 8.5 arm64 runner-minutes per
  merge-group run.
- Claude tokens: zero per pull request, by design. Nothing is posted as a comment. The cost lands
  only when a budget fires, as about five lines in the failure an ejection triage already reads.
  Fork 3 prices the alternative.

## Ranking

This is not one of the nine fatal risks, and it is not on the customer path, which is vacant. It
serves principle 2: queue throughput is the method, and the bottleneck since 2026-08-04 has been how
fast one merge queue lands. A wall-time gate that fires late ejects pull requests, which is where
lanes and tokens are lost. So it ranks below work that moves a fatal-risk verdict and above work
that does neither. The bootstrap fix and the calendar fix are each under an hour of lane work and
should go now on cost alone. The first slice should go when a lane frees. The rest can wait for it.

## Forks, in the order to rule them

1. The kernel record's format. It is in the first slice, because that slice blocks on it.
2. What gates drift. Options: (a) a per-suite, per-architecture budget ratchet; (b) a tighter
   per-test ceiling; (c) report only, no gate. Recommendation: (a). The drift on record was
   distributed, and (b) would have seen about a third of it. (c) is the floor of the mechanisms
   ladder, and it is close to what the tree had while this drift ran for two weeks. The per-test ceiling stays the hang detector it
   is.
3. Where the per-run breakdown is shown. Options: (a) step summary and artifact only; (b) also a
   comment on every pull request; (c) artifact only. Recommendation: (a). With roughly 75 to 100
   queue entries a day, a ten-row comment of about 400 tokens, read twice by agents, is in the
   order of 60,000 tokens a day of context. That is an estimate, not a measurement, and most pull
   requests would show nothing new.
4. The daily record and chart. Recommendation: build it on milestone 724's pattern. It is
   reversible, and is listed only because it adds a scheduled workflow.

## Reuse

Taken: GitHub's jobs API for per-step times, `actions/upload-artifact` for the file, the step
summary for display, and the patterns of `helpers/merge_queue_share.py` (a daily record) and
`helpers/job-budget.py` (a ratchet with a lint). Written: the collector and the suite-budget check,
which are a few hundred lines of Python against this tree's own record formats. JUnit XML was
considered as the file's format and not taken, because no consumer here reads it.

## What was considered and lost

- Splitting more jobs into shards each time a budget fires. It shortens the pole and spends the
  scarce resource, arm64 runner concurrency. PR #1775's own table prices five shards at 87% more
  runner-minutes.
- Raising budgets. It moves the day the gate fails.
- cargo-nextest for host tests, which reports per-test time and JUnit XML. It is a dependency, which
  §46 (thin primitives or whole subsystems) makes a decision, and the host pass is 89 of 900
  seconds. libtest's own JSON output with per-test times, behind `-Z unstable-options` on nightly,
  is the in-tree route if the host pass ever matters. That flag is recalled, not re-read.

## BUGS

- A merge-group run's runner varies, and every number here is wall time on a shared hosted machine.
  The record takes medians over runs for that reason. One run, as in the table above, is a premise
  check and not a baseline.
