---
status: BUILT
raised: 2026-08-31
built: 2026-08-31
---
# 204. A pushed lane branch with no draft pull request is a claim nobody can see

Built (2026-08-31). Minted 2026-08-31 by calef, after two lanes in one session pushed
their branches and never opened the draft pull request their briefs named as the first act. *(Number
provisional until the merge queue lands it.)*

In brief. AGENTS.md §90 says a lane's first act is a draft pull request, and says why: the draft
is the claim. It is how two lanes cannot silently take the same milestone, the board is
`gh pr list --draft`, it costs one command, and a draft cannot be stuck in the merge queue because a
draft cannot be merged.

Nothing checks it. On 2026-08-31 the lanes for milestones 121 and 194 both pushed
`milestone/*` branches and opened nothing. The board was empty while two milestones were being
worked, and it was noticed only because calef asked.

## Why prose was never going to hold this

The instruction was in both briefs, in the section headed *First act*, with the exact command to run.
Both lanes read it and neither did it. That is rung four behaving exactly as AGENTS.md says rung
four behaves, and it is the second instance of the same shape in this project's short history: the
first was lanes ending their turn mid-gate, which that file already records as a recurring failure
needing a standing rule rather than a per-brief reminder.

A brief is prose. Prose in a brief is not a weaker version of a mechanism; it is the same rung as
prose anywhere else.

## The check

Compare the pushed lane branches against the open drafts, and report the branches with no claim:

```sh
git ls-remote --heads origin 'milestone/*'   # what is being worked
gh pr list --draft --json headRefName        # what has claimed it
```

A report rather than a gate, in `helpers/merge-drain.sh`'s family: nothing should fail a build
over it, because the lane that most needs telling is one that is mid-work and about to open its pull
request anyway. What it must do is be visible without anyone asking, which is the property the
current arrangement lacks entirely.

## What it must not do

- **It must not fire on a branch that has just been pushed.** A lane pushes and then opens the
  pull request, so there is a legitimate window of seconds. A grace period is required, and picking
  it badly makes the check either useless or noisy.
- **It must not treat a merged lane's leftover branch as a live claim.** A branch whose pull request
  merged and which nobody deleted is hygiene, not a missing claim, and reporting it as one trains
  people to ignore the report.
- **It must not be a gate.** See above.

## What shipped

`helpers/lane-claim-check.sh` (provisional name), called once per pass from `helpers/merge-drain.sh`
before its own empty-queue return. That siting is the answer to the open question below, and the
reason is that the drain is the only unattended runner this project has: it fires every five
minutes under `launchd` on patagonia, and a report nothing runs is the state this milestone was
minted to end. A GitHub Actions cron was the alternative and is strictly better on one axis (it
survives patagonia being asleep, which AGENTS.md records as an accepted gap rather than a solved
one); it was not taken because the branch that built this was fenced out of `.github/workflows/`,
and because adding a second unattended runner to say one more sentence is more machinery than the
sentence is worth. If the drain's own launchd job ever goes away, this goes with it, and that is the
coupling to know.

The grace period is 15 minutes, and the number was measured rather than picked. The branch that
built this took 3 minutes from `branch_creation` to its draft pull request, and that included
writing the file that made the branch non-empty, which is not optional: GitHub refuses a pull
request with no commits between the head and `main`, so the literal first-act command block in every
brief cannot be run straight through. 15 is five times the observed case and well under
`merge-drain.sh`'s 75-minute stale-draft threshold, which is the neighboring report and the one
this must not shadow.

**The clock runs from branch creation and a later push does not reset it**, which is the opposite
choice from `stale_drafts` next door and is deliberate: a lane that keeps committing is exactly the
lane whose missing claim matters, so a last-commit clock would go quiet for the branches being
worked hardest. The birth time comes from the repository activity feed, because a commit date cannot
answer the question (a branch pushed empty carries `main`'s commit date and would be reported the
instant it existed).

It found a live case on its first run, which is the only evidence worth having:
`milestone/194-falsification-roadmap-status` had been pushed with no pull request, and
`milestone/121-ripgrep` was correctly reported as a merged lane's leftover branch rather than as a
missing claim.

## BUGS

- **It detects the absence of a claim, not a collision.** Two lanes on the same milestone with two
  drafts is the case §90 actually fears, and this check would call that fine. Detecting the real
  collision needs the milestone number parsed out of the branch name, which is a convention nothing
  enforces.
- **It cannot see a lane that has not pushed at all**, which is the more dangerous state, because
  AGENTS.md says a lane's pushed branch is the only ledger another session can read and uncommitted
  work in a worktree is the one thing no part of this system protects.
- It is only as alive as `merge-drain.sh` is. Where it runs was decided by siting it inside the
  drain's pass, so it inherits the drain's own recorded gap: patagonia asleep or shut down means
  nobody is watching. A GitHub Actions cron would close that and remains available.
- `milestone/*` only. A lane on `fix/`, `roadmap/` or `maintainer/` is invisible to it, and
  those are legitimate lane prefixes `script/lint` accepts. Widening the pattern would also sweep in
  short-lived maintainer branches that are not claims, so the narrow version shipped.
- The activity feed is read one page deep, so a branch created more than 100 repository events
  ago has no visible birth. Such a branch is reported rather than skipped: the fallback errs loud,
  because an old branch with no claim is the case worth seeing.
- It reports to stdout only. The drain can comment on the pull request it is complaining about;
  a branch with no pull request has nowhere to be told.

## Follow-on

- **Recorded.** `notes/merge-queue.md` documents the check and what it does not see. It detects the
  absence of a claim, not a collision: two lanes on the same milestone with two drafts is the case
  §90 actually fears and this check calls it fine, because catching it needs the milestone number
  parsed out of a branch name and nothing enforces that convention.
- **Recorded.** `notes/merge-queue.md` also names the blinder that matters more: it cannot see a
  lane that has not pushed at all, which is the more dangerous state, because a pushed branch is the
  only ledger another session can read.
- **Recorded.** `AGENTS.md` carries the gap this inherits and the fact that it was accepted rather
  than solved: the check is only as alive as `helpers/merge-drain.sh` is, and patagonia asleep means
  nobody is watching. A GitHub Actions cron would close it and was declined here.
- **Recorded.** `notes/merge-queue.md` says which prefixes it watches, which is `milestone/*` only.
  A lane on `fix/`, `roadmap/` or `maintainer/` is invisible to it, and widening the pattern would
  sweep in short-lived maintainer branches that are not claims.
- **Recorded.** `notes/merge-queue.md` carries the activity-feed bound: the feed is read one page
  deep, so a branch born more than 100 repository events ago has no visible birth time and is
  reported rather than skipped. The fallback errs loud on purpose.
- **Recorded.** `notes/merge-queue.md` records the reporting asymmetry. It writes to stdout only:
  the drain can comment on the pull request it is complaining about, and a branch with no pull
  request has nowhere to be told.

## Index row

Minted by calef on 2026-08-31, after two lanes in one session pushed `milestone/*` branches and
never opened the draft pull request their briefs named as the first act. §90 says the draft is
the claim and nothing checks it; the board was empty while two milestones were being worked. Prose
in a brief is rung four and behaved like it, for the second time in this project's history. A
report in `helpers/merge-drain.sh`'s family, never a gate, comparing `git ls-remote --heads origin
'milestone/*'` against `gh pr list --draft`. Shipped as `helpers/lane-claim-check.sh`, called once
per pass from the merge drain, which is the only unattended runner this project has. Grace period
15 minutes, measured: the branch that built it took 3 minutes from branch creation to draft, and
that included writing the file GitHub requires before it will accept a pull request at all. The
clock runs from branch birth and a later push does not reset it, which is the opposite of the
stale-draft report next door and for the opposite reason. Found a live unclaimed branch on its
first run.
