---
status: BUILT
raised: 2026-10-04
built: 2026-10-04
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 742. Every test is falsified as routine

Asked for by calef on 2026-10-04 (UTC), for fatal risk 3 (the tests do not test anything). *(Number
provisional until the merge queue lands it; title and slug are drafts.)*

In brief: Recent escapes were tests and gates that could not fail: a confinement test that
passed vacuously on two cores (#1547, #1551), and `timetable`'s ten falsification records gone
stale when d0b1ff821 dropped `mod proofs;` (#1554). The aim is that a test which cannot fail is
caught by a mechanism, not by somebody remembering to check.

## What the measurement found

The census, per kind of test, is [notes/falsification-coverage.md](../../notes/falsification-coverage.md).
The finding that decided what to build: the replays existed and their verdicts went nowhere.

- The per-pull-request replay in verify.yml was not a required check. 4 of the last 60 merged pull
  requests merged with it red, all four from the one real `timetable` defect, none a flake.
- The weekly sweep was `continue-on-error`. Its 2026-09-28 run found a stale record and reported
  success.
- A swish-check record could not be replayed at all (row 31, from milestone 673 (the confinement table lists the unvouched child)).

## Built

1. `verify (Kani proofs)` now requires `re-falsify the harnesses this change can reach` to succeed
   or skip. Rung 2. No ruleset change, following milestones 587 and 589. Measured over 10 merge-group
   runs, it lengthened the critical path once, by about four minutes.
2. The weekly sweep's run fails when the sweep has a survivor, a stale record or no verdict, after
   the summary publishes.
3. `script/falsifications` sweeps a record above a function in `xtask/src/swish_check.rs` with
   `script/swish-check --arch <a>`, red only when swish-check's FAILED report names the patch's
   `Line:` (field name provisional). The one existing record, row 31's, carries `Line:
   installed/unvouched` and a block above `swish_check_boot`. In CI (run 37171273807) it went red
   on aarch64 in 35.3 s on that line, and the sweep's other 187 records stayed clean.
4. A host `#[test]` carrying a block is swept by `cargo test -p <pkg> --test <file> <name> --exact`
   (or `--lib`), weekly and per pull request, since a replay is seconds. #1596's four `paging`
   records replayed red this way, 3.6 s for the four.

## BUGS

- A kernel or swish-check record is replayed weekly only, so it can rot between Mondays. Recorded in
  `script/falsifications`' header.
- One swish-check record at most, until §134 (a harness carries a machine-replayable falsification record)'s path can spell a line: proposal
  `a-swish-check-line-has-a-falsification-path` (an architect's call).
- New kernel tests owe no record: proposal `a-new-confinement-test-carries-a-falsification-record`.
- Host unit tests are opt-in: a record is swept, and nothing asks a test to carry one. Mutation
  testing stays their main mechanism, and notes/falsification-coverage.md says what that misses.

## Follow-on

- **Proposed.** `design/roadmap/proposals/a-swish-check-line-has-a-falsification-path.md`.
- **Proposed.** `design/roadmap/proposals/a-new-confinement-test-carries-a-falsification-record.md`.
- **Recorded.** The weekly-only replay of kernel and swish-check records, in
  `script/falsifications`' BUGS and notes/falsification-coverage.md's.

## Index row

The falsification replays existed and reported into nothing: the per-PR replay was not required (4 of
60 merged pull requests merged red) and the weekly sweep reported success over a stale record. Made
the per-PR replay part of `verify (Kani proofs)`, made the weekly run go red on a finding, and made a
swish-check record replayable by the sweep.
