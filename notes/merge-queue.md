# The merge queue, and the three things that watch it

Three scripts, all maintainer tools rather than front doors. Two were born on 2026-08-04 out of the
same evening's failures: `helpers/merge-drain.sh` lands what does not need calef, and
`helpers/trunk-health.sh` says when `main` is red. `helpers/lane-claim-check.sh` joined them on
2026-08-31 and watches one step earlier, for work that has not reached the queue at all. A fourth,
`helpers/at-risk-check.sh`, joined on 2026-09-23 and watches earlier still, for uncommitted work
sitting in a lane worktree; it has no watcher of its own and is called from `helpers/trunk-health.sh`'s
loop instead, so "three things that watch" still names the count of things that run unattended. Names
are provisional.

## Why they exist rather than being someone's job

The roles in CLAUDE.md are Maintainer, Developer, Steward. On 2026-08-04 three things went wrong in
one evening and all three were the same shape: **a duty that belonged to whoever happened to notice.**

- **Two green pull requests sat unmerged for hours** because nobody armed auto-merge on them. Not a
  judgement call, not a policy: they were opened and forgotten.
- **`main` went red and nobody owned it.** A developer cannot see `main` by design. The steward
  watched pull request checks and never the trunk. The maintainer's hygiene list is prune the
  worktree, delete the branch, relink `nife-dev`, leave no QEMU, and does not mention it.
- **Merging one pull request staled the other eight** under the new up-to-date rule, and nothing
  picked them back up until calef asked.

The pattern is the one milestone 92 argues about audits: a practice that lives in memory gets skipped
exactly when it matters, and the maintainer is structurally worst at this particular duty because
merging happens *between* conversations rather than during them.

The steward was supposed to cover that and did not, for a reason worth recording: **it reported and
never acted.** "The queue is stalled" arriving in a message is only useful if someone reads the
message and does something. These two scripts act.

## `helpers/merge-drain.sh`

```console
$ helpers/merge-drain.sh --once
merge-drain: DEQUEUED #918 (needs-architect arrived after it was enqueued): the loader stops guessing
merge-drain: STALLED. #213 is failing cpu matrix (riscv64 across QEMU CPU models) (§69 decided: Endow becomes ChildEndowment)
merge-drain: ARMED #214 (the caretaker outlives its job)
merge-drain: ENQUEUED #1207 (armed and green for 5 minutes with no queue entry; the platform had not) (the metrics page is a deck)
merge-drain: RERAN #1203 run 36022256151 (CI was cancelled as a same-second duplicate and hid the green one) (the loader stops guessing)
merge-drain: 4 armed, 1 stalled, of 5 unheld

$ helpers/merge-drain.sh            # loop until nothing is left to enqueue
merge-drain: 2 armed, 0 stalled, of 2 unheld
merge-drain: queue empty; nothing open that does not need calef
```

**Two of those lines are events and the rest are snapshots, and only the events can be counted.**
`ARMED`, `DEQUEUED`, `ENQUEUED` and `RERAN` say what this pass *did*; every other line says what was *true* when the pass
ended. Summing `4 armed` across passes double counts every pull request that was still armed on the
next pass, which is why 3,355 passes of this log could not answer "how often does the drain act"
when calef asked on 2026-09-23. `STALLED.` has the same defect: a stall that persists is re-printed
every pass, which is why `notify` deduplicates the pull request comment and the log line does not.

So:

```console
$ grep -c 'merge-drain: ARMED #' ~/Library/Logs/nife/merge-drain.log      # enqueues, countable
$ grep -c 'merge-drain: DEQUEUED #' ~/Library/Logs/nife/merge-drain.log   # withdrawals, countable
$ grep -c 'merge-drain: ENQUEUED #' ~/Library/Logs/nife/merge-drain.log   # the platform's promise, kept by the drain, countable
$ grep -c 'merge-drain: RERAN #' ~/Library/Logs/nife/merge-drain.log      # cancelled duplicates rerun, countable
$ grep -c 'merge-drain: [0-9]* armed' ~/Library/Logs/nife/merge-drain.log # passes, not enqueues
```

`ARMED` prints only on the transition: arming is attempted on every eligible pull request on every
pass, and the script suppresses the line where the pull request was already armed when the pass
began. Without that suppression it would be a snapshot with a new name on it. This tree made the
same mistake once before, in `script/metrics`, and the correction is written up there and in the
"The only flow on this page" section of [project-metrics.md](project-metrics.md): a stock read late
is merely stale, a flow read late lands in the wrong bucket.

**The summary line stays**, because it answers a question the events cannot, which is whether
anything is stuck right now.

It takes the open pull requests **without** the `needs-architect` label, skips drafts, arms
auto-merge on every one of them, and names anything that is conflicted or failing. That is the whole
script. Arming is one API call that changes nothing until the checks pass, so there is no reason to
ration it, and an armed pull request enters GitHub's merge queue on its own when it goes green.

**It never merges anything labelled `held-for-red-trunk` either**, added 2026-09-23 after it
re-enqueued a held set three times in one evening: that label means `main` is broken and one fix is
landing alone, which is a reason about the queue rather than about a pull request, and the drain's
admission policy had no way to express one. See [notes/main-is-red.md](main-is-red.md).

**It never merges anything labelled `needs-architect`**, which is the one policy the platform does
not know. That label means the work is outside standing merge authority: it touches the syscall
surface, adds a dependency, or owes a `DECISIONS` section.

**It stops rather than guessing, per pull request rather than per pass.** A conflict or a failing
check is reported with the pull request named, and the pass carries on arming the others. Both need
a person, and a loop that retries them just burns CI. A pass where nothing could be armed ends the
loop, because re-printing the same stall lines every 150 seconds is not watching.

## `helpers/lane-claim-check.sh`

```console
$ helpers/lane-claim-check.sh
lane-claim-check: LEFTOVER. milestone/121-ripgrep's #600 is MERGED; delete the branch
lane-claim-check: UNCLAIMED. milestone/194-falsification-roadmap-status has no pull request after 16 minutes. AGENTS.md §90: gh pr create --draft
```

Both of those lines are from the first run it ever did, which is the only evidence worth quoting.

**It watches the gap the two scripts above cannot see.** Everything they do starts from
`gh pr list`, so a lane that pushed a branch and opened nothing is invisible to both, and invisible
is exactly what it was: on 2026-08-31 two lanes did that and it was noticed because calef asked. The
rule they broke is AGENTS.md §90, *a lane's first act is a draft pull request*, and the reason that
rule exists is that the draft **is** the claim: it is the whole mechanism preventing two lanes from
silently taking the same milestone, and the board is one command.

Both briefs said so, in a section headed *First act*, with the command spelled out. That is rung
four behaving the way AGENTS.md says rung four behaves, and it was the second instance of the shape
in this project's history; the first was lanes ending their turn mid-gate.

**It runs from `merge-drain.sh`'s pass**, once, before the drain's own empty-queue return. The
ordering is not cosmetic: an empty queue is exactly when an unclaimed lane is easiest to miss,
because nothing else on that pass prints a word. The drain is also the only unattended runner this
project has (`launchd`, every five minutes, patagonia), so siting it there is the difference between
a report and a report that happens.

**Three false positives were designed out**, because a report that cries wolf gets ignored and then
the real case goes unread with it.

| Shape | Why it is not a missing claim | What the script does |
|---|---|---|
| A branch pushed seconds ago | The lane is between its push and its create | 15-minute grace period |
| A merged lane's leftover branch | Hygiene, not a claim | Its own `LEFTOVER` line, with the pull request number |
| A branch with a ready (non-draft) pull request | A louder claim than a draft, not a quieter one | Any open pull request counts |

**The grace period was measured, not picked.** The branch that built this took three minutes from
`branch_creation` to its draft, and that included writing the file that made the branch non-empty:
GitHub refuses a pull request with no commits between the head and `main`, so the literal first-act
command block cannot be run straight through and every lane has that delay. Fifteen minutes is five
times the observed case, and comfortably under the 75 minutes `stale_drafts` waits, which is the
neighbouring report and the one this must not shadow.

**The clock runs from the branch's birth, and a later push does not reset it.** That is the opposite
of `stale_drafts` next door, and the pair is worth reading together: a stale draft is one that
stopped moving, so it watches the last commit; a missing claim is due from the moment the branch
exists, and a lane hard at work committing is precisely the one whose absent claim matters most. A
last-commit clock would go quiet for the branches being worked hardest, which is backwards. The
birth time comes from the repository activity feed (`repos/{owner}/{repo}/activity`), because a
commit date cannot answer the question at all: a branch pushed empty carries `main`'s commit date
and would be reported the instant it existed.

**It is a report and not a gate**, and `script/lint` was refused for a stated reason rather than by
taste: the lane that most needs telling is one mid-work and about to open its pull request anyway,
and failing its build would be the least useful moment to interrupt it. Nothing was missing from the
enforcement; what was missing was anything that looks.

## What the merge queue took over, and the four shapes that preceded it

**GitHub's merge queue was enabled on this repository by milestone 120's organization move** (the
setting exists only for organization-owned repositories, which is why it used to be absent rather
than hidden), and on 2026-08-16 this script lost about 150 lines to it. The queue serializes
candidates, tests each against the tip, and ejects what fails, which is precisely what the script
had been reconstructing from outside. Three things changed at once:

- **Ordering stopped being ours.** Enqueue everything eligible and let the queue decide.
- **Updating a branch became neither necessary nor possible.** The queue builds the merge candidate
  itself, and GitHub answers `update-branch` on a queued pull request with a 422.
- **"Arm exactly one" became the wrong answer** rather than a redundant one, because it holds ready
  work back for a cycle when arming is free.

**The history is kept because it is evidence about the up-to-date rule, not about this script.** The
merge queue can be turned off, and if it is, every one of these failures returns. The loop took four
shapes and three of them starved something:

1. **Arm the head only.** #134 sat CLEAN with twelve green checks behind a lower-numbered pull
   request that was still building. calef found it, not the script.
2. **Arm everything.** That starved the head instead. Under the up-to-date rule a merge stales every
   other branch, so a small doc-only pull request goes green during a big one's thirty-minute cycle,
   merges, and sends the big one back to the start. **#117 was re-updated twice that way.**
3. **One target.** Both failures are one fact from two sides: a merge is exclusive, so the queue can
   only land one thing at a time and the only question is which.
4. **Whatever is in flight finishes first.** The third shape preferred a CLEAN pull request on the
   reasoning that it lands in minutes. Wrong: merging the cheap one **stales the one in flight**, so
   a five-minute merge costs a thirty-minute one a whole further cycle and saves nothing, because
   the cheap one would have landed straight afterwards anyway. **#120 paid three cycles** while #137
   and #139 went past it.

The rule those four shapes were groping toward: **order the two operations by what they cost the
queue, not by what they cost themselves.** A merge queue is that rule implemented by the platform,
which is why the script no longer needs to hold it.

## How they run: in Actions, as `nife-smelter[bot]` (2026-09-24)

Moved here from `AGENTS.md` by `design/decisions/` §155's principle, which the naming pass
established: the constitution keeps the duty, this document keeps the mechanism. What `AGENTS.md`
still says is that a session confirms the watchers are alive and acts on what they found.

**Two scheduled workflows, owned by the organization rather than by a laptop**, every five minutes:

| Workflow | Runs | Identity |
| --- | --- | --- |
| `.github/workflows/merge-drain.yml` | `helpers/merge-drain.sh --once`, which calls `helpers/lane-claim-check.sh` inside its own pass | `nife-smelter[bot]` |
| `.github/workflows/trunk-health.yml` | `helpers/trunk-health.sh --once`, and **fails the run** when `main` is red or a cadence is dead | `nife-smelter[bot]` |
| `launchd`, per developer | `helpers/at-risk-check.sh`, which reads that machine's own worktrees | nobody: it needs no credential |

Each workflow mints a one-hour installation token with `actions/create-github-app-token` from the
organization secrets `AUTOMATION_APP_ID` and `AUTOMATION_APP_KEY`. **No key is at rest on anybody's
machine**, which is the property that made this the recommendation over putting the App's private
key on patagonia: the credential-at-rest question does not arise for anything that runs here.

**What this bought, in the order the proposal argued it.** Everything these do is attributed to
`nife-smelter[bot]` rather than to `calef`, so what is still attributed to calef is genuinely calef,
which is the negative half a local log can never give. The singleton is a singleton by construction
(`concurrency:`), rather than because one laptop happened to be awake. The run list is a log every
contributor can read, with timestamps and exit codes, where `~/Library/Logs/nife/` on one Mac was
readable by one person. And a stopped watcher shows as a disabled workflow in the Actions tab
instead of living in one session's transcript, which is how the drain's unloading on 2026-09-23 was
knowable to exactly one reader.

### The premise this rested on, tested before anything was written

`cli/cli#7213` reports `gh pr merge --auto` failing under a GitHub App installation token where a
personal token succeeds. **The merge drain's entire job is arming pull requests**, so if that were
true here, every option that authenticates the drain as `smelter` loses and the fork collapses back
to a machine account or the status quo.

Measured 2026-09-24 in a throwaway workflow, under a token minted from the `nife-smelter` App,
against a throwaway pull request that was closed and deleted the same minute:

```console
$ gh pr merge 1176 --repo crickertech/nife --auto --merge
! The merge strategy for main is set by the merge queue      # exit 0, armed
$ gh api graphql -f query='mutation{enablePullRequestAutoMerge(input:{pullRequestId:"...",mergeMethod:MERGE}){clientMutationId}}'
{"data":{"enablePullRequestAutoMerge":{"clientMutationId":null}}}
$ gh api graphql -f query='mutation{enqueuePullRequest(input:{pullRequestId:"..."}){clientMutationId}}'
gh: Pull request 14 of 14 required status checks have not succeeded: 2 expected.
```

The issue does not reproduce on this repository with this App. The third line is the interesting
one and is why it was run: a permission denial and an eligibility refusal read differently, and
`enqueuePullRequest` and `dequeuePullRequest` both reached GitHub's business logic, which is the
answer "permitted" in the only form the API gives. `gh pr list --json` read, and `gh pr comment`
posted, rendering as `nife-smelter[bot]`.

### What a person must run on patagonia to retire the old jobs

**This is not optional and it is not automatic.** Until it is done there are two drains, one in
Actions and one on a laptop, both arming the same pull requests. Nothing in this repository can do
it: `launchd` jobs live in `~/Library/LaunchAgents/` on one machine.

```console
$ launchctl unload -w ~/Library/LaunchAgents/com.nife.merge-drain.plist
$ launchctl unload -w ~/Library/LaunchAgents/com.nife.trunk-health.plist
$ rm ~/Library/LaunchAgents/com.nife.merge-drain.plist ~/Library/LaunchAgents/com.nife.trunk-health.plist
$ launchctl list | grep nife          # expect nothing yet
```

Then install the one watch that stays per developer, because retiring `com.nife.trunk-health`
retires the at-risk check with it (it was folded into that script's loop) and that is the watch
AGENTS.md calls the more valuable of the two. Write `~/Library/LaunchAgents/com.nife.at-risk.plist`,
substituting the path to your own main checkout:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>com.nife.at-risk</string>
  <key>ProgramArguments</key>
  <array>
    <string>/bin/sh</string>
    <string>/Users/calef/projects/nife/helpers/at-risk-check.sh</string>
  </array>
  <key>WorkingDirectory</key><string>/Users/calef/projects/nife</string>
  <key>StartInterval</key><integer>300</integer>
  <key>StandardOutPath</key><string>/Users/calef/Library/Logs/nife/at-risk.log</string>
  <key>StandardErrorPath</key><string>/Users/calef/Library/Logs/nife/at-risk.log</string>
</dict>
</plist>
```

```console
$ launchctl load -w ~/Library/LaunchAgents/com.nife.at-risk.plist
$ launchctl list | grep nife
-	0	com.nife.at-risk
```

**An already-installed plist names its script by absolute path, so the rename of `scripts/` to
`helpers/` on 2026-09-23 breaks it silently**: `launchd` logs the missing file and the watch simply
stops reporting. Fix an existing one in place, or reinstall it from the block above.

```console
$ sed -i '' 's|/scripts/|/helpers/|' ~/Library/LaunchAgents/com.nife.at-risk.plist
$ launchctl unload ~/Library/LaunchAgents/com.nife.at-risk.plist
$ launchctl load -w ~/Library/LaunchAgents/com.nife.at-risk.plist
```

**`launchd` is the loop here**, which is why `helpers/at-risk-check.sh` did not grow one: it does a
pass and exits, `StartInterval` runs it every five minutes, and a script with no loop cannot be
killed mid-loop by a prune of the checkout it was launched from (the failure that killed both
watchers on 2026-08-18).

### What is lost, and it is smaller than the proposal priced it

**Cadence.** GitHub's shortest `schedule` interval is five minutes, and scheduled runs are delayed
under load and dropped at peak, which is a documented behaviour rather than a caveat. The proposal
priced this against the script's own 150-second loop and that comparison was wrong: the `launchd`
jobs already fired `--once` every five minutes, so the real loss is only the delay and the drops,
not two and a half minutes. calef accepted it on 2026-09-23.

**Transition reporting.** `trunk-health.sh`'s loop said "red" once and "recovered" once because it
remembered the previous poll. A scheduled run remembers nothing, so a trunk red for an hour is
twelve failed runs. The workflow's own BUGS section says so; carrying state in an artifact was more
machinery than the fact is worth.

**The gap that was accepted and is now closed.** Patagonia asleep meant nobody was watching. That
was named rather than hidden, and a cron on cordoba was declined in 2026-08-26 in favour of the
simpler thing on the machine already in use. Actions closes it for the two that moved and leaves it
exactly where it was for the at-risk check, which is correct: a laptop that is asleep has no lane
worktree being edited on it.

**A restraint that was reweighed rather than ignored.** calef declined an unattended scheduled agent
on 2026-08-26, preferring that this shut down when the session driving it does. His 2026-09-23
approval supersedes that for these two, and the distinction he drew in September holds here as well:
what runs on a timer is a shell script reading GitHub and arming what is eligible, with no judgment
in it. **A queue reports, it does not resolve** is still the boundary. Neither workflow resolves a
conflict, retries a failed check, or marks anybody's draft ready.

**Why `notify()` speaks once per stall.** `merge-drain.sh` posts a PR comment on a conflict, a check
failure or a stuck check, and then goes quiet. That is deliberate, so a stalled pull request does not
re-announce itself every five minutes. The consequence a maintainer has to hold is the other half of
it: **nothing re-announces the stall to a session that opens later**, so reading the queue is a
standing duty rather than something the watcher does for you. Read
`gh pr list --json number,mergeStateStatus,statusCheckRollup` for `DIRTY`/`CONFLICTING` or a
`FAILURE` conclusion.

**Which drain spoke.** Every line the drain prints, and every comment it posts, is prefixed
`merge-drain[<instance>]`: `actions:<run id>` from the workflow, the hostname from a laptop. An
installation token carries the App and not the caller, so GitHub cannot tell a reader which instance
acted once the automation runs in more than one place. The `notify()` dedupe markers are
deliberately untagged, so two instances cannot each post the same stall once.

**Two fields that lie to a session watching one pull request**, both met on 2026-09-19 watching
#965. `autoMergeRequest` goes **null the moment GitHub enqueues** the pull request, so "auto-merge
is off" reads exactly like "dropped from the queue" when it means the opposite. And
`statusCheckRollup` keeps every run, including the ones a newer push **cancelled**, so a
`CANCELLED` conclusion is usually a superseded run sitting beside its own `SUCCESS`. Ask the queue
itself instead: `gh api graphql` for
`pullRequest(number: N) { mergeQueueEntry { state position } }`, where no entry while open means
out of the queue, and treat only `FAILURE` and `TIMED_OUT` as failures.

## `helpers/trunk-health.sh`

```console
$ helpers/trunk-health.sh --once
main is green at 5a09f754

$ helpers/trunk-health.sh
MAIN IS RED at d1e6b1e9 -- failing: CI -- nobody is assigned to this
main recovered at 38dc6473
```

It reports the *transition* to red and the transition back, never every red poll: a trunk broken for
an hour is one fact, not twenty-four. It reports recovery deliberately, because a watcher that only
speaks on failure teaches its reader that silence means health, and silence is also what a dead
watcher produces.

The phrase "nobody is assigned to this" is not filler. A red trunk with an owner is a task; a red
trunk without one is the failure being surfaced.

**What to do once it speaks is [notes/main-is-red.md](main-is-red.md)**, added 2026-09-23 because
calef asked whether the response existed and it did not: this watcher reported a red trunk and
`helpers/merge-drain.sh` carried on arming pull requests into it every five minutes. The response is
`helpers/queue-hold.sh` (hold the queue, land one fix alone, release) with the judgement in
[briefs/main-is-red.md](../briefs/main-is-red.md). It stays a person's to run, for the reason this
note gives throughout: a queue reports, it does not resolve.

**And this watcher has a blind spot worth knowing here**, now recorded in its own `BUGS`: it reads
CI's *conclusion*, and a required check whose steps were skipped posts `success` having run nothing.
On 2026-09-23 a documentation-only commit broke a `crates/documentation` test that `ci.yml` had
skipped, and `main` was red for hours while this script said green.

## `helpers/at-risk-check.sh`, the one watch that stays on your own machine

AGENTS.md gives the steward a second watch, named beside the idle-lane one and called the more
valuable of the two: "a lane worktree with modifications and no commit in half an hour is
uncommitted work one prune away from gone, which is the only failure in this system that destroys
rather than delays." Nothing built it. `launchctl list` showed `com.nife.merge-drain` and
`com.nife.trunk-health` on patagonia and nothing else, so the duty AGENTS.md assigns by name had no
mechanism behind it.

The cost was measured, not hypothetical: in one session on 2026-09-23 three pieces of work were
found only by luck rather than by anything watching. A fix to `helpers/open-lane.sh` (later #1097)
surfaced while pruning merged worktrees. A fix to `kernel/src/user/live_swap_tests.rs` (later #1101)
survived two prunes uncommitted and had to be recovered twice. Sixty-two lines of a decisions
amendment sat unsaved on `maintainer/202-four-tiers` for hours after the conversation had moved on.
The maintainer pruned worktrees twice that same session; any of the three could have been destroyed
outright.

```console
$ helpers/at-risk-check.sh
at-risk-check: UNCOMMITTED. /Users/calef/projects/nife-worktrees/atrisk (maintainer/work-one-prune-from-gone) has 2 changed file(s), newest touched 41 minutes ago. One prune away from gone; commit and push.
```

**It reads every worktree but the main checkout** (`git worktree list --porcelain`), skips any
already `prunable` (nothing left in them to lose), and for the rest reads `git status --porcelain`.
The clock is the newest modification time among the changed files, tracked or untracked, **not the
branch's last commit date**: a worktree can carry a commit from hours ago and be mid-edit again a
moment later, and the fact that matters is how long the current uncommitted state has sat, not when
it was last saved. `AT_RISK_MINUTES` (default 30, AGENTS.md's own "half an hour") overrides it, the
same convention `GRACE_MINUTES` already sets in `helpers/lane-claim-check.sh`.

**It reports and never acts**, the same boundary `helpers/lane-claim-check.sh` holds. It does not
commit on a lane's behalf, and it does not use `git stash`: the stash stack is per-`.git`, shared
across every worktree of this repository rather than scoped to one, so one worktree's `git stash`
can be popped by a session working in a completely different worktree, which is action at a distance
of exactly the kind this script exists to warn about rather than to commit. AGENTS.md's own note,
"`git stash` is unsafe in these worktrees, for the same reason one level over," is the same finding
from the other side.

**Folded into `helpers/trunk-health.sh`'s loop on 2026-09-23, unfolded on 2026-09-24, and the
reasoning is worth keeping because it was right both times.** The fold was to avoid a third watcher
that could die silently, reusing a job already firing on the right interval. That holds only while
both halves run on the same machine. When the trunk half moved to Actions they stopped doing so:
everything else in `trunk-health.sh` reads GitHub, and this reads **this machine's** worktrees, so
carrying the fold into a runner would have produced a check reporting nothing forever while the
hazard sat on a laptop unwatched. It now has the third `launchd` job the fold avoided, with the
cost that decision was avoiding accepted explicitly: nothing reports its death either. The plist and
the commands are in "What a person must run on patagonia" above.

Unlike RED/GREEN and unlike the cadence check beside it, this does **not** dedupe by transition. A
worktree still at risk on the next poll is still exactly as at risk, distinguishing "still true" from
"newly true" would need a second piece of state this script does not otherwise keep, and the cost of
not deduping is log lines rather than anything a reader has to act on twice.

Verified against the live tree on 2026-09-23: of 42 lane worktrees, only this lane's own carried
uncommitted work, and it was minutes old, inside the grace window. Nothing else was flagged; no
false positive against a lane legitimately mid-work was observed, because nothing was over the
threshold to test that against. `AT_RISK_MINUTES=0` against the same tree correctly flagged this
lane's own worktree, confirming the mechanism fires.

## The prevention half, which is not these scripts

`main` went red on 2026-08-04 because two pull requests, each green against the base it was cut from,
merged in an order **neither had ever been tested in**: one added `script/citations`, the other added
a gate requiring every `script/` entry point to carry a provenance block. Neither branch ever
contained the other.

No per-pull-request check can see that, because the failing input is the merge order, which is not a
property of either branch. GitHub's **require branches to be up to date before merging** is the
mechanical answer and was applied the same evening (§73). It converts that failure from a red trunk
into one re-run. These scripts are the detection half; that rule is the prevention half, and it is the
better one.

**The merge queue is the same prevention with the cost removed** (2026-08-16). Up-to-date-before-merge
buys the guarantee by making every author pay for it serially, in full CI cycles, which is what made
the ordering brain above necessary and what milestone 119 measured as the bottleneck. The queue tests
the same thing, the candidate against the tip, without staling anybody's branch to do it. Same
prevention, one rung up: the platform holds it rather than a rule everybody has to route around.

## A push to `main` cites the merge group instead of repeating it (A′, 2026-09-24)

calef approved this on 2026-09-24 ("Proceed with A′"). On a `push` to `main`, the `draft gate` job in
`ci.yml` and `verify.yml` asks the API whether a `merge_group` run of the **same workflow** concluded
`success` at **exactly** `github.sha`. If one did, it sets `run=false`, prints that run's ID and URL,
and every gated job skips. If there is no such run, the run did not succeed, or the API errors, it
runs everything. A commit that reaches `main` outside the queue has no merge-group run and still gets
the full suite.

**Why.** The runners were starved. 31 runs were queued at 16:40 UTC that day, and every landed batch
paid for its suite twice: once on the `gh-readonly-queue/` ref, then again, identically, on the push
to `main`. A full verify is the project's long pole on its own.

**The premise, checked against history first.** For each of the twelve first-parent commits on
`main` from `aee5b141` to `47c3a3c9`, a successful `merge_group` CI run has `head_sha` equal to that
commit. The queue builds each entry's merge commit on its own ref, and the commit that lands is that
very object. Inside a batch only the tip gets a `push` run: `ac04fb01` (#1179) and `b9b0d4bb` (#1202)
got none, while the batch tips `0b72f673` and `47c3a3c9` did. The tip's merge-group run tested the tip
with every earlier entry of its batch already beneath it, so an exact-SHA lookup answers "was this tree
tested" without inferring anything about neighbours.

```console
$ gh api "repos/crickertech/nife/actions/workflows/ci.yml/runs?event=merge_group&head_sha=47c3a3c9d01e96c8d007a4b41ff9d2adc8858f10&status=success&per_page=1" \
    --jq '.workflow_runs[0].id'
36025842548
```

A push that finds one logs `==> <sha> was tested by merge group run <id>; skipping the suite` in its
`draft gate` step, with the run's URL on the next line.

`helpers/trunk-health.sh` reads a skipped push run as green, since the run concludes `success`, and
names the merge-group runs behind it. That keeps this green distinct from the skipped-docs green its
own `BUGS` still warns about.

## The queue's check timeout is 240 minutes, not 60 (2026-09-24)

The maintainer raised the ruleset's `check_response_timeout_minutes` from 60 to 240 after #1213 was
evicted with reason `checks_timed_out`. Measured from the timeline and the run data:

| event | UTC |
|---|---|
| #1213 added to the queue | 16:53:43 |
| merge-group CI 36030550892 and verify 36030550968 created | 16:54:01 |
| first verify job past the gate starts (a runner freed) | 17:13:05 |
| last prove shard finishes | 17:48:23 |
| **evicted, `checks_timed_out`** | **17:54:14** |
| required `verify (Kani proofs)` aggregate starts, then reports `success` | 17:58:53 |

**The build did run.** What timed out was the wait: about nineteen minutes queued before any real job,
then a five-second aggregate job that sat ten minutes for a runner after the last shard and reported
four minutes past the deadline. CI had finished green at 17:45:20. At 60 minutes the timeout measured
runner supply rather than the change, so it evicted a green pull request and made it queue again.

## `script/preflight-queue`: the group build, run here first

**The queue's prevention has a price, and a red member is where it is paid.** A group of up to five
is built as one, and a member that is red on top of the entries ahead of it fails the whole group,
which is then rebuilt without it. Every other member's build is thrown away. Per-pull-request CI
cannot see this case at all, because the failing input is the stack, not the branch: on 2026-09-24 a
lane replayed the queue by hand (23 entries, about 40 minutes) and found three pull requests, #1194,
#1222 and #1182, green alone and red on top of what was ahead of them. Each would have cost a group
roughly an hour of runner time per member.

`script/preflight-queue` makes that replay a command. It walks the queue in order, then the pull
requests armed but not yet queued, merges each onto the green ones ahead in a scratch worktree, and
runs the cheap end of `script/ci-build` plus one aarch64 suite for anything that touches code. Its
header has the ladder, what it skips and why, and its `BUGS`.

**When to run it:**

- **Before enqueueing a batch.** When a session is about to arm several pull requests at once, and
  especially when two of them touch the test-wiring hotspot, run it with the batch armed but before
  the queue has formed groups. A dry run costs this machine minutes per code entry and seconds per
  documentation entry.
- **When a group has just failed and the queue is deep.** The failure evicted one member; the rest
  are rebuilt, and a second red member behind it costs another group. A dry run says whether there is
  one.
- **Not beside a `script/verify` or a mutation sweep**, which is AGENTS.md's memory ceiling; the
  script already skips its own falsification rung when a solver is running, but its aarch64 suite
  still competes for cores.

It defaults to `--dry-run`. `--act` comments on each red entry, with the failing command, an excerpt
and the stack it was tested on, and dequeues it (or disables auto-merge if it was not yet queued).
If `main` itself is red at the baseline, it acts on nothing and exits 3, because every entry would
inherit that failure.

## What the queue bought, measured

Taken 2026-08-16, and it is milestone 119's (the merge queue is the bottleneck) own definition of
done: the block already had the before-median and the sharding, and said plainly that the after-median
had to come from a run of pull requests rather than from the first one on the new path. There are
eighteen now.

**Where the numbers come from.** The GitHub REST API, three endpoints: the merged pull requests, each
one's timeline events, and every workflow run's jobs with their start and finish. Nothing here is read
off a dashboard or remembered.

**The two windows, and why the before one starts where it does.** The proof shards landed on `main`
with #159 at 2026-08-14T05:22Z; the queue's first group build ran at 2026-08-15T21:46Z. So the before
window is the 40.4 hours between them, which holds the *current* prover constant and measures only
what the queue changed. The after window is the 10.2 hours from the first group build to
2026-08-16T08:00Z, where this snapshot stops.

| | before: sharded, no queue | after: the queue |
|---|---|---|
| window | 08-14T05:22 to 08-15T21:46 (40.4 h) | 08-15T21:46 to 08-16T08:00 (10.2 h) |
| pull requests landed | 39 | 18 |
| **"land this" to merged, median** | **17.0 min** (n=29, auto-merge armed) | **12.3 min** (n=17, last enqueue) |
| gap between consecutive merges, median | 15.8 min (n=34) | 10.8 min (n=17) |
| merges per elapsed hour | 0.97 | 1.76 |
| opened to merged, median | 47.0 min (n=38) | 160.9 min (n=18) |
| CI job-minutes per landed pull request | 122 | 157, or 109 with the storm hour removed |
| runs on `main` that went red | 2 of 120 since 08-13 | 0 of 30 |

**The row that got worse says nothing about the queue, and saying so is the point.** Opened-to-merged
counts everything that happened to a pull request, including how long it sat before a person enqueued
it. Its after-window median of 160.9 minutes decomposes: the median from *first* enqueue to merged is
112.1 minutes and from *last* enqueue to merged is 12.3, and the difference is one afternoon's storm
of evictions plus eleven re-enqueues that were operator error. The queue's own cost is the 12.3.

### EXAMPLE: five pull requests, one cycle

The clearest thing in the data. At 2026-08-15T23:42:51 through :57, five pull requests were enqueued
within six seconds of each other. GitHub built five chained candidates concurrently from 23:43:07,
each containing one more entry than the last. #204 and #205 landed at 23:50:37; #207, #208 and #209
landed at 00:03:32.

**Five pull requests, 20.6 minutes from enqueue to the last merge.** At the before-window median of
17.0 minutes each, serialized, the same five would have been about 85 minutes, and under §73's
up-to-date rule each merge would have staled the other four at least once, so the real before-cost is
higher than that and is the thing the ordering brain above was written to manage.

### The caveats, and there are five

- **The samples are small and each is one afternoon.** 39 landings against 18, both from the same
  week, both from lanes run by the same architect. This is a measurement of this tree in August, not
  a general result about merge queues.
- **Runner contention varies and is not controlled.** The same `CI` job, on candidates that differ
  only in which pull requests they contain, ranged from 6.6 to 23.5 minutes across the 44 group
  builds. Any single comparison of two runs is inside that noise; only the medians are worth reading.
- **One storm inflates every early after-number.** Between 21:46 and 22:34 on 08-15, twenty-five
  candidate builds failed CI for one reason: `script/lint`'s branch-prefix check rejected the
  queue's own `gh-readonly-queue/*` branches, so every candidate was ejected and rebuilt. 678
  job-minutes, and the pull requests caught in it carry a two-hour first-enqueue-to-merged that is
  the gate's bug rather than the queue's behaviour. #217 fixed it and was merged directly, outside
  the queue, because the queue could not land anything until it was.
- **Several re-enqueues on 08-16 were operator error, not eviction.** Eleven re-enqueues across
  seventeen pull requests, nine of which needed more than one. Some were the queue ejecting a
  candidate; others were a person removing and re-adding one. The timeline records both as the same
  event pair, so this measurement cannot separate them and does not try.
- **The after window's composition is not the before window's.** It holds the day's largest change
  (#210, the SMB service) and three that waited on calef for a decision. That pulls
  opened-to-merged up and leaves the enqueue-to-merged numbers alone, which is why both are in the
  table.

### The prover is the long pole, but only for the changes that reach it

Group builds run `CI` and `verify` concurrently, so the landing waits on whichever finishes last.

| | CI, median | verify, median |
|---|---|---|
| all 44 group builds | 12.2 min | 3.6 min |
| the 19 after the storm | 10.7 min | 0.6 min |
| the 6 where the proofs actually ran | 11.2 min | 16.7 min |

Twelve of the nineteen post-storm builds finished `verify` in under two minutes, because the scope
job proved that nothing in the change could reach a harness. **So the median landing is now CI-bound
rather than prover-bound**, which is the scoping and the sharding working exactly as milestone 119
predicted, and it is a real change from the block's 2026-08-05 measurement that "a merge cycle is the
Kani job plus noise".

What is left is the tail, and in the tail the prover decides the landing: in those six builds it ran
a median 5.6 minutes past a `CI` that was already green.

**And that tail is almost entirely false positives.** Re-running the `--affected-since` predicate over
the seventeen changes that landed since 08-14 having run the full suite: for all five of the
post-storm ones, **no file in the change was inside any harness crate's dependency closure.** They
proved everything because of files the predicate cannot attribute to a crate, and so runs by default:

| landing | what made it prove the whole suite | harness crates it could reach |
|---|---|---|
| #207 | `Cargo.lock`, `Cargo.toml` (a new workspace member) | none |
| #208 | `art/cobble-first-draft.jpg` | none |
| #210 | `Cargo.lock`, `Cargo.toml`, `scripts/qemu-runner-*.sh` | none |
| #218 | `Cargo.lock`, `Cargo.toml` | none |
| #219 | `scripts/merge-drain.sh` | none |

Three levers follow, ranked by what the counts say and by how much judgment each needs. Together they
account for twelve of the seventeen:

1. **`scripts/` is not `script/`, and the predicate only knows the singular.** A change to
   `merge-drain.sh` proves twenty crates. Nothing under `scripts/` is an input to `cargo kani`: the
   QEMU runners belong to `xtask test` and `kani-lint-shim/` belongs to `script/lint`'s clippy pass,
   which is the same argument the existing `script/` case already makes. **Three of seventeen**, and
   it is one branch in the `elif` that handles `script/` today.
2. **`Cargo.lock` and the workspace `Cargo.toml`: seven of seventeen**, the largest bucket and the
   one that needs judgment rather than a line. Adding a workspace member cannot change a harness's
   closure; bumping a dependency version can. The honest version parses the lock diff for changed
   package entries and tests those against the closure, and it wants its own lane.
3. **Binary and data files: two of seventeen** (`art/`, `bench/baseline-*.txt`). Same shape as the
   documentation case the predicate already handles.

**More shards is not the lever, and the block already measured why.** `glob` is atomic at 15.0 minutes
of a 30.3-minute serial suite, so two shards reach 15.1 and four reach 15.0. The measured group-build
`verify` when the proofs run is 16.7 minutes, which is that floor plus Kani's install. Nothing under
it comes from arranging CI differently; it comes from the unwind bound in one `glob` harness, or from
not proving crates a change provably cannot reach.

### What the measurement corrected about the milestone

**The queue does not amortize CI cost across a group; it amortizes wall clock.** Milestone 119's
block expects "N pull requests cost one test cycle instead of N", and the ruleset as configured
(`max_entries_to_build: 5`) builds one candidate *per entry*, concurrently, each running the full
`CI` and `verify`. That is why cost per landed pull request is flat across the two windows (122
minutes before, 109 after with the storm removed) while wall clock nearly halved. The saving is real
and it is a different saving from the one predicted.

Cost still matters even though this repository is public and its Actions minutes are free, because
what it actually buys is concurrent runners, and the spread in identical `CI` jobs above is what
contention looks like. Trading it back is a setting rather than a project: `max_entries_to_build: 1`
would build one group of up to five pull requests once, at one fifth the cost, and would pay for it
in bisection when a group fails. Nobody has needed that yet.

### Do the two watchers still earn their keep

**`merge-drain.sh` does, and its job changed rather than ended.** It is now the enqueuer: every
landing in the after window entered the queue through the arming call it makes.

**`trunk-health.sh` is closer to superseded, and the number is honest about how little it proves.**
Zero of the thirty runs on `main` since the queue went live were red, against two of a hundred and
twenty in the three days before. Ten hours is not evidence that the trunk cannot go red, and the
class of failure that remains is one the queue never sees: a flake, and the scheduled workflows
(toolchain bump, drift, mutation) which run against `main` on a timer and are not part of any merge.
Keep it; expect it to speak rarely.

## Squash and rebase merging are disabled at the repository, not only in the queue

**Decided by calef 2026-08-18**, closing a gap between a rule and its enforcement. `AGENTS.md` has
said *never squash-merge a branch* since the convention was written, and the reason is `git blame`:
milestone 96's lane put the loader unification in its own commit **ahead of** the migration so that a
boot failure could not be ambiguous between two changes, and a squash-merge would have destroyed
exactly that. The merge commit already carries the pull request's title, so `git log --first-parent`
reads as one entry per piece of work while the detail stays reachable underneath. The clean log costs
nothing.

**What was actually enforcing it until now was the merge queue's `merge_method: MERGE`**, plus people
having read `AGENTS.md`. The repository still had `allow_squash_merge` and `allow_rebase_merge` set,
so the buttons were there and the rule held by configuration coincidence and memory. That is rung two
propped on rung four, and the same day measured what unenforced policy is worth here: a CI gate that
could not block a merge let `main` go red for hours, which is the advisory-checks decision waiting
on its own branch as this is written.

Now `allow_squash_merge=false`, `allow_rebase_merge=false`, `allow_merge_commit=true`. Reversible in
one API call; the previous settings are recoverable from any repository snapshot.

**This does not change how a lane works, and the distinction is the one worth keeping straight.**
Squashing *within* a branch is still the rule: commit early and push often, because a pushed branch
survives a dead session and nothing else does, then squash the checkpoints into the **purposes**
before reporting. A checkpoint has no reader; a purpose commit has one. What is now impossible is
collapsing those purposes into one at merge, which is a different act on a different object.

Two exceptions stay unsquashed inside a branch, and they are why this matters: a commit that records
a correction or a surprise, and a commit whose separateness is itself the argument.
## `Blocked-by: #N`: a hold that releases itself

**The problem it solves is ordering, not attention.** `needs-architect` means a person must decide
something, and it is the only lever the drain had. On 2026-08-18 that lever was reached for twice
where no decision was owed, and both times it put a false entry on the one queue in this project that
must not accumulate noise.

The live case: #329 and #324 each carried a file named `97-*.md` under `design/decisions/`. Both were
green alone. A merge-queue group containing both fails the decisions gate, because two sections
cannot share a number, so #329 was evicted as `UNMERGEABLE` while its own page reported `CLEAN`.

**Green alone, green alone, red together is not a state any per-branch check can see.** #274 is the
expensive version of the same shape: it was enqueued and evicted **29 times, 26 of them inside a
3.5-hour loop**, because #271 landed a doctest calling a method whose arity #274 was changing, and
git merged both with no conflict marker.

**A generic hold label was considered and refused**, on the failure mode rather than on tidiness. A
manual label must be removed by whoever remembers, and this tree has the receipts: `needs-architect`
was left on both #320 and #329 after each had been answered, on one day, and calef found both. **A
hold that outlives its reason is a false blocker, and a false blocker is worse than none, because it
is believed.**

So the hold names its own release condition:

```
Blocked-by: #324
```

in the pull request body. The drain skips that pull request while #324 is open, and arms it on the
first pass after #324 merges, **with nobody acting**. If #324 is *closed* without merging, that is
reported loudly rather than silently released, because it means the thing this was sequenced behind
is not coming.

**Use it for a mechanical constraint and nothing else.** If a person must decide, the label is still
the right answer, and the two must not be conflated: one is a queue for calef's attention, the other
is a fact about two branches.

## BUGS

**The event lines start from the day they landed, and the 3,355 passes before it cannot be
backfilled.** The old log holds snapshots only, so the drain's action count begins on 2026-09-23 and
any comparison across that boundary is between two different measurements.

**`ARMED` counts what the DRAIN did, which is not the same as what happened to the queue.** A pull
request a person armed by hand is already armed when the next pass begins, so the pass stays silent
and no line records the arming at all. That is the right behaviour for the question the line exists
to answer and the wrong one for "how did this pull request get into the queue". The queue's own
timeline on the pull request is the record for the second question.

**And this log can only ever say what the machinery did, never what a person did.** An action absent
from it is ambiguous between "the drain did not do this" and "somebody did it by hand", and the
ambiguity is worst exactly when it matters, which is when something unexpected happened. Closing
that needs distinct GitHub identities rather than a better log; the proposal is
[design/roadmap/proposals/who-took-the-step.md](../design/roadmap/proposals/who-took-the-step.md)
(name provisional) and it is calef's call.

- **A′ lets `main`'s Actions caches go stale.** `Swatinem/rust-cache` saves on the ref that ran, and
  a pull request can restore only its own ref's caches and the base branch's. Merge-group refs are
  neither, so once most pushes to `main` skip, the `main` caches that pull requests fall back to stop
  being refreshed by CI and verify. Pushes that still run in full refresh them: commits landed outside
  the queue, and any tip whose merge-group run did not succeed. Expect slower cold builds on pull
  requests, not wrong ones. Nobody has measured it yet.
- **A′ trusts a merge-group run's overall conclusion, which is stricter than what the queue
  requires.** The queue lands on the required checks alone. A merge-group run can conclude
  `cancelled` or `failure` because of a non-required job and still land: `0b72f673` (#1156) did,
  when verify's falsify job hit its 45-minute timeout. That push then re-runs the whole workflow,
  and it did (verify 36029132635, cancelled by the same timeout). This errs toward running, which
  is the right direction, but a flaky non-required job costs a full re-run on `main`.
- **A 240-minute check timeout is four hours in which a genuinely hung group blocks everything
  behind it.** Sixty was right for the work and wrong for runner supply. The honest fix is fewer
  runs competing for runners, and A′ is the first of those. When the queue stops starving, lower
  the timeout again, because the cost of a long timeout only shows up when a group is actually stuck.
- **One push can raise two `synchronize` events, and when the cancelled copy is the newer run the
  pull request is stranded, green and armed, outside the queue** (#1203, 2026-09-24). Every
  workflow on `e0875d56` ran twice at 15:43:24, including `coe architect label`, which listens only
  for `opened`, `reopened` and `synchronize`, so it was not `ready_for_review` (that fired at
  15:30:28). The concurrency group cancelled one copy of each within a second and before any job
  existed. For verify the cancelled copy was the older id (36022255755) and nothing broke. For CI it
  was the newer id (36022256151, over 36022255835's success), and a forced enqueue answered "11 of
  13 required status checks are expected": CI's eleven. So GitHub reads the newest suite per
  workflow, and an empty cancelled suite there hides a green one. Rerunning the cancelled run fixed
  it, at the cost of a whole second suite (attempt 2 ran 16:41 to 17:37).

  **No workflow-level fix is sound.** The duplicate event is GitHub's. A pending run is always
  cancelled when a newer one joins its group, whatever `cancel-in-progress` says. Putting the SHA in
  the group key would stop the duplicates cancelling each other, but a new push would then stop
  superseding the old one. Skipping when a same-SHA run already succeeded would also skip after a
  draft run that concluded `success` with every job skipped, which is #567 again. **The detection
  belongs in the drain**, which can rerun the run it finds:

  ```sh
  sha=$(gh pr view "$pr" --json headRefOid --jq .headRefOid)
  gh api "repos/crickertech/nife/actions/runs?head_sha=$sha&event=pull_request&per_page=100" --jq '
    .workflow_runs | group_by(.name)[] | sort_by(.id) as $r | ($r | last) as $n
    | select($n.conclusion == "cancelled")
    | select([$r[] | select(.id != $n.id and .created_at == $n.created_at
                            and (.conclusion == "success" or .status != "completed"))] | length > 0)
    | $n.id'          # each id printed: gh run rerun <id>
  ```

  The same-second condition is what separates this from the ordinary supersede. #1207, #1209 and
  #1211 each have a cancelled CI run followed 20 to 66 seconds later by a successful one, which is
  a draft marked ready, and none of them was stranded.

  **The drain does this now** (#1252, 2026-09-24): the query above is `helpers/cancelled-duplicate.jq`,
  spliced into `merge-drain.sh`, and a pull request in this shape gets its cancelled duplicate
  rerun once, logged as `RERAN #N run <id>`. Once is decided by the run's own `run_attempt`, so no
  file or label holds the state; a duplicate already at attempt 2 is a `STALLED.` line for a person.
  The rerun uses the workflow's own token with `actions: write`, because the App's token cannot
  rerun a workflow; adding `Actions: read/write` to the App is calef's, and would let the rerun
  carry the App's identity.
- **A branch stacked on another pull request, then merged with `main`, has two merge bases, and
  GitHub calls that a conflict that git does not see** (#1220, 2026-09-24). #1220 was cut from
  #1213's branch. After #1213 landed and `main` was merged back in, `git merge-base --all` gave
  both `47c3a3c9` and #1213's own commit. `git merge-tree` merged cleanly against every entry ahead
  of it in the queue, but the queue marked it `UNMERGEABLE`, built no group for it, and evicted it
  with `merge_conflict` at 20:00:51. Its page said `CONFLICTING`. **Once the base pull request
  lands, rebase the stacked commits onto `main` rather than merging `main` in.** The tell is
  `git merge-base --all origin/main HEAD` printing more than one line.

**A branch in the merge queue cannot be pushed to, and the error names the fix without naming the
cost.** `git push` is rejected with `GH006: Protected branch update failed ... Branches that are
queued for merging cannot be updated. To modify this branch, dequeue the associated pull request.`
Met on 2026-09-20 by a maintainer who armed auto-merge, then found a defect in the committed tip: the
fixed commit could not be pushed, `gh pr merge --disable-auto` did not release it, and the
`merge-queue` REST endpoint answered `Not Found`. **What worked was waiting for the group build to
evict the branch**, then pushing and re-arming.

**The order that avoids it: gate, then arm.** An armed pull request is one whose content you have
stopped editing. That sounds obvious and is exactly what a maintainer fixing a gloss at the last
moment forgets.



- **`Blocked-by:` is matched anywhere in the body, including inside a code span or a quotation.** A
  pull request that *discusses* the convention, as opposed to using it, will be held. The
  counted-claims check in `script/lint` solved the same problem by blanking code spans first; this
  does not, and the cheap fix is available if it ever bites.
- **It holds the drain, not the queue.** Anyone who enqueues by hand, or arms with `gh pr merge
  --auto` directly, bypasses it entirely. The drain is the normal path and this covers the normal
  path; it is not an interlock.
- **Only the first `Blocked-by:` is read.** A pull request sequenced behind two others can only say
  so once, and the honest workaround is to name the later one.
- **A push after the pull request is enqueued is silently discarded, and the pull request keeps
  reporting the newer commit as its head** (found 2026-09-16, milestone 304). The queue merges the
  **SHA it enqueued**. Push again before the group build lands and GitHub updates
  `headRefOid` to the new commit, shows the PR as merged, and then deletes the branch, so the
  merge commit's second parent is the *old* SHA and the last commit exists nowhere but a local
  object store. Milestone 304 lost a documentation commit this way and found it only because the
  lane happened to diff `origin/main` for its own content afterwards; it was recovered by
  cherry-picking out of the pruned worktree's objects and landed as its own pull request.

  **The tell is that `gh pr view --json headRefOid` disagrees with
  `git log -1 --format=%P <merge commit>`.** Nothing reports it, no check fails, and both halves
  look correct in isolation: the PR says merged, `main` is green, and the diff a reader compares
  against is the one that was enqueued.

  Two habits cover it, and the first is nearly free. **After a merge, confirm the work is on `main`
  by content rather than by the PR's state** (`git merge-base --is-ancestor <your last SHA>
  origin/main`). And **treat the moment a pull request is marked ready as the end of pushing**: a
  lane that wants one more commit should expect to open a second pull request for it, which is
  cheaper than the recovery above and is what happened here anyway.

- **A watcher started from a lane worktree dies when that worktree is pruned, and now refuses to
  start there** (2026-08-18). `/bin/sh` reads a script lazily, so deleting the file under a running
  shell can kill it mid-loop. It happened twice in one day: the merge drain died when the worktree
  it was launched from was pruned, and `trunk-health.sh` died the same way during a 24-worktree
  cleanup, **silently, while `main` was red for hours on a gate nobody was watching**. The drain
  survived the second sweep only because it had been relaunched with an absolute path into the main
  checkout. Both scripts now refuse the watching form outside the main checkout and say why;
  `--once` is still allowed anywhere, because it exits long before a prune could reach it.


- **Neither script survives the session that starts it.** They are ordinary loops, not services.
  CLAUDE.md's session-start list is what makes them run; nothing enforces it, and a session that
  forgets has exactly the gap they were written to close. A launchd job or a scheduled workflow would
  fix this and neither has been built.
- **`merge-drain.sh` trusts the label.** A pull request that *should* be held but was never labelled
  will be merged by it. The label is applied by hand at the moment the decision to hold is made, so a
  maintainer that forgets the label has bypassed the gate rather than tripped it.
- **The reduced `merge-drain.sh` had not been run against a live queue, and the first time it was,
  it enqueued nothing for three hours.** Fixed 2026-08-17; the entry is kept because the prediction
  that preceded it was right and was not acted on. It said every claim about the script was read
  from the source rather than observed, because it was written and shellchecked in a container with
  no `gh` at all. What the source could not show: **`gh pr merge --auto --merge --delete-branch`
  fails outright when a merge queue is enabled** (`Cannot use -d or --delete-branch when merge queue
  enabled`), and the call site sent its output to `/dev/null` under `|| true`. So every pass printed
  `9 armed, 0 stalled` while the queue sat empty and nothing merged. The flag was redundant as well
  as fatal: this repository sets `delete_branch_on_merge`, so the platform deletes the head branch
  itself.

  **Two lessons, and the second is the reusable one.** A count of *attempts* was being printed as a
  count of *results*, which is the shape AGENTS.md's ladder calls rung zero wearing a uniform. And
  **nothing on a pull request object says it is in a merge queue**: a queued pull request reports
  `mergeStateStatus: CLEAN` with a **null** `autoMergeRequest`, because arming became membership.
  `mergeQueue.entries` is the only authority, and the obvious field looking authoritative while
  being wrong is what cost the three hours. The verification now asks the queue.
- **`merge-drain.sh` re-arms what is already armed, forever.** A pass counts an arming call as work
  whether or not it changed anything, so one pull request that never merges (a required check that
  was removed, a broken workflow file, a queue that is wedged) keeps the loop alive at 150-second
  intervals with nothing happening. It is cheap and it is silent, which is the bad combination: the
  script cannot tell a queue that is moving from one that is stuck, and neither can its reader.
- **It reports what the queue is about to reject, not what the queue did.** Stalls are read from the
  pull request's own checks. A candidate that fails *inside* the merge queue, against the tip rather
  than against its own base, is ejected by GitHub and this script says nothing about it; the next
  pass simply arms it again.
- **`lane-claim-check.sh` is only as alive as the drain is.** It runs from the drain's pass and has
  no schedule of its own, so it inherits the recorded gap AGENTS.md already accepts: patagonia
  asleep means nobody is watching. It also reports to stdout only, because a branch with no pull
  request has nowhere to be commented on, so its findings reach whoever reads the drain's log and
  nobody else. Its own header carries the rest (`milestone/*` only, one page of activity feed, and
  that it sees a missing claim rather than the duplicate claim §90 actually fears).
- **`at-risk-check.sh` is only as alive as `trunk-health.sh` is**, the same dependency
  `lane-claim-check.sh` has on the drain, for the same reason: it has no schedule of its own. It also
  only sees the worktrees on the machine it runs from, so patagonia asleep means a worktree on a
  laptop taken elsewhere is unwatched regardless, which the recorded gap above already names.
- **`trunk-health.sh` polls at 90 seconds and reads only `main`.** A release branch, if this tree ever
  grows one, is invisible to it.
- **Neither reports its own death, and on 2026-08-18 that cost hours of red trunk.** If the process
  is killed, both simply stop saying anything, and the failure mode is indistinguishable from a
  healthy quiet queue. The entry above removes the commonest *cause* of the death; it does nothing
  about the silence, which is still unfixed. This is the same defect the
  scripts exist to fix, one level up, and it is not fixed. The queue has taken over most of what
  `merge-drain.sh` did and the whole class of failure `trunk-health.sh` watched for, so this defect
  now costs less than it did; it costs more than nothing, because the arming call is still what puts
  a pull request into the queue and a dead drain still looks exactly like an empty one.
- **The measurement above is a snapshot and nothing re-derives it.** Every number in it was taken by
  hand from the API on 2026-08-16 and pasted into prose, which is precisely the class milestone 125
  (a number in the prose is a claim) exists to fix. Re-take them rather than trusting them once the
  windows are wider than a day; the scripts that produced them were a lane's scratch files and were
  not kept, deliberately, because a throwaway analysis committed as a tool is a tool nobody
  maintains.
- **"Opened to merged" is mostly a measurement of people.** It is in the table because leaving it out
  would be picking the flattering metric, but it is dominated by how long a pull request waited for a
  human to enqueue it, and no arrangement of CI moves it. Read the enqueue-to-merged rows for
  anything about the queue.
- **The eviction and the re-enqueue are the same timeline event pair.** `added_to_merge_queue` and
  `removed_from_merge_queue` do not distinguish a candidate GitHub ejected from one a person removed
  and re-added, so the eleven re-enqueues counted above cannot be attributed. The honest reading is
  an upper bound on the queue's own churn.
