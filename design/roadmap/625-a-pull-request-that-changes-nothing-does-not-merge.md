---
status: PARTIAL
raised: 2026-10-02
promoted_from: a-pull-request-that-changes-nothing-does-not-merge
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 625. A pull request that changes nothing does not merge

**Approved by calef on 2026-10-02 (UTC), option A below**, and promoted in the same pull request that
proposed it. The number 625 is provisional until the queue lands it (the integrator mints at merge;
624 is already claimed by `milestone/624-paint-path`). The title and the two files `.github/workflows/empty-diff.yml`
and `helpers/empty-diff-check.sh` are provisional names. calef has not ratified them.

## What is being decided

Whether to add a required check that fails a pull request, and a merge-group entry, whose diff
against its base is empty. The instance is #1460 ("fatal risk colours, counted by week"). It merged on
2026-09-30 carrying only its §90 (the claim is a draft pull request) `claim:` commit, while the finished work sat on a differently named
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
  claimed under one branch name and worked under another. The pull request GitHub was tracking
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
  pull request is empty. It also runs from the lane's worktree before the lane has pushed anything,
  so it would fire on the claim commit every lane makes first on purpose.
- **C. Teach `merge-drain` and `queue-eligible.jq` to refuse enqueue.** Refused. It covers only what
  the drain enqueues; #1460 went in by hand, and the manual path is the one that failed.
- D. Extend `lane-claim-check.sh` to flag a work branch descending from a claim whose pull request
  has not moved. Worth doing, but it is a report (rung 4) and finds the stranded work after the
  fact. It is the companion that catches the cause; this proposal catches the consequence. Left to
  a lane of its own.

## The recommendation: option A

Where it runs. A new workflow, `.github/workflows/empty-diff.yml`, shaped like
`architect-hold.yml`: small, read-only, `contents: read`, `ubuntu-24.04-arm`, a 5 minute timeout,
triggers `pull_request` (`opened`, `reopened`, `synchronize`, `ready_for_review`), `merge_group` and
`workflow_dispatch`. It is a separate file from `ci.yml` on purpose: `ci.yml`'s draft gate skips
everything while a pull request is a draft, and this check has no business waiting for the
suite. It lists `ready_for_review` explicitly because `ci.yml`'s own header records the draft guard
failing open for five days on that trigger. Its job name is then added to the `main` ruleset's required
checks. Only calef can make that settings change. Until it is added the check reports and
does not gate, and the PR that lands the workflow says so.

What it tests. The merge's tree against its first parent's, as two tree ids
(`helpers/empty-diff-check.sh BASE TIP`). The run checks out a merge commit in both events: the
platform's merge ref on `pull_request`, the queue's entry commit on `merge_group`. BASE is that
commit's first parent. The first draft of this proposal said `merge_group.base_sha`. Building it
corrected that, because `base_sha` is the base branch's tip when the group was built, so for a batch it spans
every earlier entry and an empty entry behind a non-empty one would pass. The entry's own first
parent is the previous entry, so the comparison covers that pull request alone. The merge-group half
is the one that matters, because it holds even when the `pull_request` run was stale.

How it identifies claim commits. It does not need to, and that is deliberate. An empty diff is
the defect; a claim-only pull request is its common cause, not its only one (a lane that commits
work and then reverts it is the same hazard). A commit counts as a claim for the *message* only: a
subject starting `claim:` whose `git diff-tree` is empty. When every commit net of merges from
`main` is one of those, the message says so. That is the case that needs a pointed hint.

What it says when it fires.

```
empty-diff: this pull request changes no file against main.
Its only commits are claim commits (<sha> "claim: fatal risk colors by week").
If the work exists, it was probably pushed to another branch. Check:
    git ls-remote --heads origin '*<milestone-slug>*'
and either push that work to this branch or open a pull request from it.
If nothing was meant to land, close this pull request.
```

Fail direction. An unreadable revision fails the check, loudly.
A required check that fails open is the shape `ci.yml`'s header already paid for.

How it is falsified. Two observations, both left beside the check, per
`a-gate-is-not-evidence-until-it-has-failed`:

1. `helpers/empty-diff-check.sh` holds the predicate, and `helpers/empty-diff-selftest.sh` builds
   fixtures in a throwaway repository and is wired into `script/lint` beside
   `blocked-by-selftest.sh`. Cases that must fail: the #1460 shape (one empty `claim:` commit),
   claim plus a merge from `main`, claim plus a commit and its revert. One case must pass: claim
   plus one real file change. An unresolvable revision must exit 2.
2. A live run: a throwaway draft pull request of one empty commit, based on this branch so the
   workflow file is in its merge. Its run id is recorded on #1482. The `merge_group` half, in
   particular the first-parent claim for a batch of two, is asserted here and not yet observed.

## The exception surface

None to start with. A merged pull request that changes nothing records nothing in the tree, so a
legitimate one needs a reason a reader can check, and none is known today. If one appears, the
exception is a label (`empty-ok`) with a `## Why this is empty` comment, read on the
`pull_request` event only. The workflow header must then say it is a foot gun: the label is a
memory-based escape, and an unmarked one would read as a design. It is not built until needed. The
first request for it is a signal to ask what the empty pull request was for.

## What it costs

Seconds of runner time per event, one more required context, and one false-positive class to
watch: a revert-then-reapply pair that nets to nothing. That is correct to refuse. Reversibility is
high: a workflow file and one ruleset line, nothing two programs agree on. Would we choose it if
both options cost the same? Yes; D alone is cheaper and is not a substitute.

## What is blocked

Nothing. The ruleset change is calef's. The workflow can land first as a reporting check.

## Prior art in the tree

`architect-hold.yml` (small required check with a PR-number resolver for `merge_group`),
`helpers/blocked-by-selftest.sh` (a predicate with fixtures, wired into lint), and
`helpers/lane-claim-check.sh` (the §90 claim and why it was never a gate).

## Follow-on

- Outstanding. Adding the check's job name to the `main` ruleset's required checks, which only
  calef can do; checked 2026-10-02 by reading the ruleset's required contexts through the API, where
  it is absent. Until then the check reports and does not gate.
- Outstanding. The live falsification: a throwaway draft pull request whose red run is linked
  in `.github/workflows/empty-diff.yml`'s header. Not yet linked there.
- Proposed. `design/roadmap/proposals/a-claim-with-work-on-another-branch-is-reported.md` is the
  `lane-claim-check.sh` companion (option D above). It is written here so it has a home.

## Index row

A merge that changes no file is the one defect every content gate passes, because it has no content
to fail on. #1460 landed that way on 2026-09-30 and its work sat unmerged for three days. This is
the required check that compares a merge's tree with its first parent's.
