Gate a branch's work in GitHub Actions rather than on this laptop. You are in the branch's git
worktree, your work is written, and you are ready to prove it. Do the work; do not ask questions.

Why this brief exists rather than "run `script/test`". The gates are the heaviest thing this
project does: three architectures of QEMU, and `script/verify`, whose own header records a Kani
harness reaching 3.5 GB. Running them here is what caps concurrent lanes at three or four, and
it has killed a session outright. CI has more memory than this machine and nobody is waiting on a
lane, so the wall-clock a dispatched run costs is free and the memory it saves is not.

## Step 1: run the cheap gates locally, because a 25-minute run must not die on a typo

These take seconds, need no emulator, and catch most of what fails:

    script/lint
    script/roadmap --check
    script/fmt
    script/citations --ratchet

On a fresh clone, including a cloud session, run `script/bootstrap` first. Without it `script/lint`
exits 1 at `cargo-machete not found`. Read each gate's own exit status, never one piped through
`tail`: a cloud lane reported a failed lint as green that way on 2026-09-18.

`script/citations --ratchet` reads the committed state, so commit before running it. Fix what
they report and commit again. Do not push until all four are clean.

## Step 2: push, then dispatch the suite explicitly

    git push -u origin HEAD
    gh workflow run ci.yml --ref "$(git branch --show-current)"
    gh workflow run verify.yml --ref "$(git branch --show-current)"

The second and third commands are not optional, and this is the whole trap. §90 (the claim is a draft pull request; the status flip is a gate) opens every lane
as a draft pull request, and both workflows begin with a `draft gate` job that asks the API
whether the pull request is a draft right now and skips the entire suite when it is. So a push to
a lane branch raises `synchronize`, the gate reads `draft: true`, and all thirteen checks report
"skipped". A skipped check still posts a conclusion and still satisfies a required check, which
is how #567 once sat mergeable with nothing verified. Pushing is therefore not gating. A
`workflow_dispatch` run is not a `pull_request` event, so the gate returns `run=true` and everything
actually executes.

## Step 3: capture the run ids, end your turn, and do not dispatch twice

    gh run list --branch "$(git branch --show-current)" --limit 5

Take the two run ids from Step 2's dispatch and stop there. Do not watch them. A lane never
polls CI (`briefs/session-start.md` has the rule): every poll re-reads this whole session's cached
context, and one lane burned 309 tool calls over six hours doing exactly that. End your turn with
one line instead:

    WAITING <ci-run-id> <verify-run-id> on ci.yml + verify.yml for <branch>

The maintainer adds those ids to the watch list; a watcher wakes the maintainer when both finish, or
when either fails, and resumes you with the result. Both workflows set `cancel-in-progress` for any
ref that is not `main`, so a second dispatch cancels the first. If a run looks stuck, that is the
maintainer's call on resumption, not a reason to re-run it yourself.

Expect 23 to 29 minutes for `ci.yml` on real code, 2 to 3 on a documentation-only branch where
nearly every job skips on scope. `verify.yml` takes about 47 minutes and is the long pole; it is
a separate workflow precisely so a flaky three-minute job can be retried without waiting for it.

## Step 4: read a failure with `briefs/triage-a-failing-check.md`

That brief has this repository's traps: that `test exited abnormally` is the consequence rather than
the cause, that a `bench: CHECK FAIL` line carries both numbers, that `fastpath-footprint` names an
ISA, and that a cancelled job is not evidence of a defect. Use it rather than reading the log raw.

Fix, commit, push, dispatch again. Each round costs about half an hour of wall-clock and nothing
else, which is the trade this brief is making: rounds are cheap because no person is waiting, and
the laptop's memory is not spent at all.

## Step 5: once WAITING resolves green, mark ready, don't poll to find out

The order is push, then let the run settle, then `gh pr ready`. The wait is the point, not the
order. `ready_for_review` and a `synchronize` from a push you just made land in the same per-ref
concurrency group as `ci.yml`, `verify.yml`, and `architect-hold.yml` (Step 3's group, the one that
cancels a second dispatch). If `ready_for_review` arrives while that push's own run is still
starting or still running, GitHub cancels the run, not the event: the older run in the group dies
and posts CANCELLED, permanently, with no way to hide it from the pull request's check list.
Measured across the open pull requests on 2026-09-23: 15 cancelled runs against 1 apparent
failure, and every cancelled one traced back to a push and a `ready` landing within a second or
two of each other (confirmed on #1123: a force-push and `ready_for_review` a second apart, three
jobs cancelled at the same timestamp). `briefs/triage-a-failing-check.md` already names the reading
half of this trap, that a cancelled job is not evidence of a defect; this is the writing half, which
is to stop generating them.

Push, name the run ids, and end your turn on the same WAITING line as Step 3:

    git push -u origin HEAD
    gh run list --branch "$(git branch --show-current)" --limit 5
    # WAITING <run-id> <run-id> on ci.yml + verify.yml before marking #<N> ready

When the maintainer resumes you with both runs green, mark it ready and arm it in one command:

    gh pr ready <N> && gh pr merge <N> --auto --merge

Arm it only after your last push: a push to a queued branch removes the pull request from the queue,
throws away its group build and cancels its auto-merge (the pre-push hook refuses it; see
`notes/merge-queue.md`, "A push to a queued branch").

Nothing else arms it. The merge drain stopped arming on 2026-10-03 (milestone 727 (a queue eviction goes to a maintainer session), provisional),
and a ready pull request left unarmed for 30 minutes is labelled `needs-maintainer` for a
maintainer session to pick up.
Nothing here is watched with `gh run watch` or `gh pr checks --watch`; those poll, and polling is the
rule this brief and `briefs/session-start.md` both refuse.

A lane that opens as a draft (§90, the claim is a draft pull request, the status flip is a gate)
and never pushes again before its one `gh pr ready` call has nothing to race against and can skip
this step entirely. The trap is specifically a push closely followed by a ready; a lane whose last
push and only `ready` are not close in time never produces it. It bites hardest on the common case
this brief itself creates: a fix pushed in response to Step 4, then marked ready right after, with no
wait in between.

## Pausing on another pull request, rather than on CI

`WAITING <run-id>` is for CI, and only for CI: it names a run the maintainer's watcher is polling on
your behalf. Your draft might instead be parked on another pull request: stacked on its branch,
sharing a file another lane is mid-edit on, or waiting on a decision only that other pull request
settles. That is not a `WAITING` line, and CI is not what will unblock it.

Say so where a machine can act on it instead: add `Blocked-by: #N[, #M ...]` to this draft's own
body before ending the turn. `helpers/merge-drain.sh` labels the draft `unblocked` and comments once
every listed pull request has merged or closed. `notes/blocked-by-drafts.md` has the mechanism;
`notes/merge-queue.md`'s `Blocked-by:` section has why a plain "held" label was refused, and what
the line does on a ready pull request. A listed pull request closed without merging is
called out as such rather than silently released, because it usually means the plan changed. The
label clears itself the moment you push a new commit or mark the draft ready, so resuming needs
nothing more than doing the work.

Without it, the pause lives only in the pull request's own prose, which nothing re-reads. That is
exactly what happened to #1289 in 2026-09: paused
waiting on #1288, which merged an hour later, and left alone for two days because the pause was
never machine-readable.

## What you must not do

- Do not run `script/verify` or `script/test` locally to "check first". That is the cost this
  brief exists to avoid, and two of them at once is the out-of-memory kill.
- Do not mark the pull request ready to make CI run. Dispatch instead. Ready means the work is
  finished and asking to merge; a lane still gating is neither.
- Do not mark ready right after a push, even once gating is green. Wait for that push's own run
  to finish first (Step 5); a `ready` a few seconds behind a push cancels the run instead of skipping
  it, and the cancellation is what reads as a failure.
- Do not push to a branch that is in the merge queue. GitHub refuses it with `GH006`, and
  `auto=false` does not mean unqueued.
- Do not enqueue anything. Gating proves the work; merging is the maintainer's.
- Do not poll, with `gh run watch`, `gh pr checks --watch`, or a `sleep` loop around `gh run
  view`/`gh run list`/`gh pr checks`/`gh pr view`. End your turn with the WAITING line (Step 3, Step
  5) and let the maintainer's watcher resume you. A PreToolUse hook refuses the common polling shapes
  outright; see `briefs/session-start.md`.

## BUGS

- A dispatched run and a push-triggered run share a concurrency group, so pushing immediately
  after dispatching cancels your own run. Push first, then dispatch, in that order.
- The same group is what makes Step 5 necessary, and the fix there is to wait, not to reorder:
  there is no event ordering that avoids the collision, only a gap wide enough that the earlier
  event's run has already finished before the later one starts.
- Only some workflows are actually reachable by this trap on a normal lane. A file carrying
  `cancel-in-progress` is not enough by itself; the workflow also has to trigger on `pull_request`
  for a lane's own ref, which only `ci.yml`, `verify.yml`, and `architect-hold.yml` do
  unconditionally (`coe-architect-label.yml` triggers the same way but does not cancel;
  `stick-maker-hosts.yml` cancels but only fires on a change under `crates/stick_maker/`). The rest
  of this tree's `cancel-in-progress` workflows are schedule- or `workflow_dispatch`-only and never
  see a lane's `synchronize` or `ready_for_review` at all. Count workflow files with the string
  before trusting the count; it overstates the exposure.
- Nothing here measures what a CI round costs against a local one. The claim that rounds are
  free rests on lanes being asynchronous, which is true today because one person reviews them. It
  stops being true the moment a lane blocks something a person is waiting for.
- The scope guard can skip jobs on a branch that genuinely needed them if the change only
  touches paths it considers documentation. A suite that finishes in three minutes on a code change
  has skipped, not passed.
