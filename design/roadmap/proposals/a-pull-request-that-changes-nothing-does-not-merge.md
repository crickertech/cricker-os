---
status: PROPOSED
raised: 2026-10-02
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# A pull request that changes nothing does not merge

`a-pull-request-that-changes-nothing-does-not-merge`: provisional name, minted 2026-10-02 by
`maintainer/propose-claim-only-gate`; not put to calef. The workflow and helper names below are
provisional too.

## What is being decided

Whether to add a required check that fails a pull request, and a merge-group entry, whose diff
against its base is empty. The instance: #1460 ("fatal risk colours, counted by week") merged on
2026-09-30 carrying only its §90 (the claim is a draft pull request) `claim:` commit, and the finished work sat on a differently named
branch for three days.

## What happened, from the timeline

Read from `gh api repos/crickertech/nife/issues/1460/timeline`, not recalled.

- 2026-09-30 05:36 UTC: the claim commit is pushed on `lane/fatal-risk-colors` and the
  draft opens. Its one commit, empty, is the whole pull request.
- 2026-09-30 07:06 UTC: `merge-drain` posts STALE DRAFT, whose text says "If its lane is finished:
  gh pr ready 1460". That is the only prompt on the pull request.
- 2026-09-30 14:50 UTC: calef marks it ready and enqueues it, two seconds apart. The queue builds it
  and lands it at 15:30 UTC. `autoMergeRequest` is null. The steward did not land it.
- The work (two commits) was pushed to `origin/milestone/fatal-risk-colors`, a
  different ref that descends from the claim commit. The draft's head never moved. So the lane
  claimed under one branch name and worked under another, and the pull request GitHub was tracking
  had nothing in it.
- 2026-10-02: found by branch triage (#1475), three days on.

**The COE in 6331baa4e is not related.** That class is an ejection cancelling an auto-merge
request. #1460 was never ejected and never had auto-merge armed; a human enqueued it and the queue
did what it was asked.

## Why nothing stopped it

Every existing gate answers a question about content. An empty diff has none, so every one of them
passes: `ci.yml` skips its jobs while a pull request is a draft (by design, because of §90) and
then has nothing to fail on once it is ready. `helpers/lane-claim-check.sh` looks for pushed
branches with no pull request, and here the work branch was a different name from a pull request
that was open and then merged. `merge-drain` gates what it enqueues itself; this entry was enqueued
by hand. The suite ran for 40 minutes against a diff that could not differ from `main`, and its
green was the *checked nothing* case.

## The options

- **A. A required check on `pull_request` and `merge_group` (recommended).** Fails when the entry
  changes no file.
- **B. A check in `script/lint`.** Refused. Lint reads the tree, and a tree can be correct while its
  pull request is empty; it also runs from the lane's worktree before the lane has pushed anything,
  so it would fire on the claim commit every lane makes first on purpose.
- **C. Teach `merge-drain` and `queue-eligible.jq` to refuse enqueue.** Refused. It covers only what
  the drain enqueues; #1460 went in by hand, and the manual path is the one that failed.
- **D. Extend `lane-claim-check.sh` to flag a work branch descending from a claim whose pull request
  has not moved.** Worth doing, but it is a report (rung 4) and finds the stranded work after the
  fact. It is the companion that catches the cause; this proposal catches the consequence. Left to
  a lane of its own.

## The recommendation: option A

**Where it runs.** A new workflow, `.github/workflows/empty-diff.yml`, shaped like
`architect-hold.yml`: small, read-only, `contents: read`, `ubuntu-24.04-arm`, a 5 minute timeout,
triggers `pull_request` (`opened`, `reopened`, `synchronize`, `ready_for_review`), `merge_group` and
`workflow_dispatch`. It is a separate file from `ci.yml` on purpose: `ci.yml`'s draft gate skips
everything while a pull request is a draft, and this check has no business waiting for the
suite. It lists `ready_for_review` explicitly because `ci.yml`'s own header records the draft guard
failing open for five days on that trigger. Its job name is added to the `main` ruleset's required
checks, which is a settings change only calef can make; until it is added the check reports and
does not gate, and the PR that lands the workflow says so.

**What it tests.** The diff, not the commit messages. On `pull_request`, the changed file count
against the merge base (`gh pr view N --json changedFiles`, or `git diff --quiet base...head`). On
`merge_group`, the entry's own contribution, `git diff --quiet $head_sha^1 $head_sha`, since the
queue builds each entry as a merge commit on the one before it. The merge-group half is the one
that matters: it holds even if the `pull_request` run was missed or stale.

**How it identifies claim commits.** It does not need to, and that is deliberate. An empty diff is
the defect; a claim-only pull request is its common cause, not its only one (a lane that commits
work and then reverts it is the same hazard). A commit counts as a claim for the *message* only: a
subject starting `claim:` whose `git diff-tree` is empty. When every commit net of merges from
`main` is one of those, the message says so, because that is the case that needs a pointed hint.

**What it says when it fires.**

```
empty-diff: this pull request changes no file against main.
Its only commits are claim commits (<sha> "claim: fatal risk colors by week").
If the work exists, it was probably pushed to another branch. Check:
    git ls-remote --heads origin '*<milestone-slug>*'
and either push that work to this branch or open a pull request from it.
If nothing was meant to land, close this pull request.
```

**Fail direction.** An unreadable diff (API error, shallow clone too shallow) fails the check, loudly.
A required check that fails open is the shape `ci.yml`'s header already paid for.

**How it is falsified.** Two observations, both left beside the check, per
`a-gate-is-not-evidence-until-it-has-failed`:

1. `helpers/empty-diff-check.sh` holds the predicate, and `helpers/empty-diff-selftest.sh` builds
   fixtures in a throwaway repository and is wired into `script/lint` beside
   `blocked-by-selftest.sh`. Cases that must fail: the #1460 shape (one empty `claim:` commit),
   claim plus a merge from `main`, claim plus a commit and its revert. Cases that must pass: claim
   plus one real file change, a one-line change with a claim commit under it.
2. A live run: a throwaway draft pull request carrying only a claim commit, marked ready, with the
   red run linked in the workflow's header. Because it is the real trigger path, it also settles
   whether the `merge_group` half reads `$head_sha^1` correctly, which this proposal asserts but
   has not run.

## The exception surface

None to start with. A merged pull request that changes nothing records nothing in the tree, so a
legitimate one needs a reason a reader can check, and none is known today. If one appears, the
exception is a label (`empty-ok`) with a `## Why this is empty` comment, read by the check on the
`pull_request` event only, and the workflow header must say it is a foot gun: the label is a
memory-based escape, and an unmarked one would read as a design. Not built until needed; the
proposal's position is that the first request for it is a signal to ask what the empty pull request
was for.

## What it costs

Seconds of runner time per event, one more required context, and one false-positive class to
watch: a revert-then-reapply pair that nets to nothing. That is correct to refuse. Reversibility is
high: a workflow file and one ruleset line, nothing two programs agree on. Would we choose it if
both options cost the same? Yes; D alone is cheaper and is not a substitute.

## What is blocked

Nothing. The ruleset change is calef's, and the workflow can land first as a reporting check.

## Prior art in the tree

`architect-hold.yml` (small required check with a PR-number resolver for `merge_group`),
`helpers/blocked-by-selftest.sh` (a predicate with fixtures, wired into lint), and
`helpers/lane-claim-check.sh` (the §90 claim and why it was never a gate).
