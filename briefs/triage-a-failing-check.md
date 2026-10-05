Find out why a GitHub Actions check failed and report the one line that matters. You are in the
repository's git worktree. Do the work; do not ask questions. Change no files. This is a
read-only investigation and its whole output is your final message.

The pull request number is in the first line of the invocation, or take the branch you are on and
find it with `gh pr list --head "$(git branch --show-current)"`.

## What to do

1. `gh pr checks <N>` to see which checks failed.
2. For each failure, get its job id from the link and read the failing step:

       gh run view --job <job-id> --log-failed

3. Read past the noise. These logs end with cleanup steps that always look alarming. The failing
   assertion or error is usually well above the tail. Search the log for `error[`, `error:`,
   `FAILED`, `panicked`, `assertion`, `CHECK FAIL`, and `exit code`.

## The traps, which are specific to this repository

- **`test exited abnormally` is not the failure**, it is the consequence. The real cause is higher
  in the log, often a QEMU boot that never reached its exit.
- A `bench: CHECK FAIL` line names the benchmark and both numbers. Report both, and the
  percentage, because whether it is over or under the bound decides what happens next.
- `fastpath-footprint` failures name an ISA and a symbol. Report which ISA; the same change
  frequently passes on two architectures and fails on the third.
- A cancelled or timed-out job is not evidence of a defect. Say it was cancelled rather than
  reporting it as a failure.
- A missing or expired log is a real outcome. Say the log could not be read; do not infer a
  cause from a non-zero exit, which is how a maintainer built a wrong theory on 2026-09-21.

## What to report, and nothing else

- which checks failed, by name
- for each, the single line that is the actual cause, quoted exactly
- whether it looks like the branch's own doing or something already broken on `main`, and say which
  evidence told you, since "it fails on main too" is a different problem from "this branch broke it"
- if you cannot tell, say you cannot tell

Do not propose a fix. Do not edit anything. Do not commit.
