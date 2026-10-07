---
status: NOT-STARTED
raised: 2026-09-18
promoted_from: a-lane-that-outlives-its-own-merge
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 429. A lane that outlives its own merge, and the checklist line that would end it

Promoted from the proposal `a-lane-that-outlives-its-own-merge`, filed
2026-09-18 by the maintainer when milestone 310's lane reported "ready for review, all gates green"
about a pull request that had merged 18 hours earlier. *(Number provisional until the merge queue
lands it.)*

It is a line on a checklist and a habit, not code.

Premise re-checked 2026-09-19 and still true. `AGENTS.md`'s merge checklist still reads prune
the worktree, delete the branch, relink `nife-dev`, `git worktree prune`, and every piece of
identified work has a home. It still does not say stop the lane, so the maintainer still tidies
everything a lane owns except the lane itself. Note that the line this block proposes lands in
`AGENTS.md`, which a developer does not edit, so the change is the maintainer's or an architect's to
make even though the gate is `NONE`.

## What happened

Milestone 310's lane ran for 18 hours. Almost all of it was blocked, by instruction, inside two
Miri runs: `script/undefined-behavior-check` locally at 3h 09m, then the same workflow in CI at
2h 58m. Miri interprets rather than executes, single-threaded, at roughly a thousandth of native
speed.

While it was blocked, the maintainer merged its pull request (#912, 2026-09-17T18:14:31Z), deleted
its branch, pruned its worktree and relinked `nife-dev`. The lane finished its second Miri run, saw
its own gates green, and reported that its work was ready to merge. Everything it said was true
when it started waiting and stale by the time it spoke.

The report then cost a second person's attention: the maintainer verified it four ways before
answering, and calef read it as an open lane needing a merge.

## Why no mechanism caught it

A lane has no visibility outside its own worktree, and that is the design rather than an
oversight. It is the isolation that lets several run at once without coordinating. The cost is
that a lane cannot notice its own milestone finishing, its branch being deleted, or the tree moving
under it.

Nothing ends a lane except the lane. AGENTS.md's merge checklist says to prune the worktree,
delete the branch, relink `nife-dev` and check that identified work has a home. **It does not say
stop the lane.** So the maintainer tidies everything the lane owns except the lane itself, and a
lane blocked in a three-hour gate keeps running against work that no longer exists.

## What it cost, measured

- 18 hours of wall clock on one lane, of which roughly 17 were waiting.
- 172,000 tokens.
- Two people's attention spent on a stale claim, and a verification pass to disprove it.

**What it did not cost is the Miri time itself**, and this proposal should not be read as arguing
against it. The lane's second run was its own call and the right one: milestone 310 existed because
that CI job had never once passed, so green on one laptop proves less than green in CI. It came back
green in 2:58:13, the workflow's first pass ever, and two machines agreeing within eleven minutes is
what makes three hours the job's cost rather than one machine's. That is the number the workflow's
header had been asking for.

## The proposal

One line on the merge checklist: when a pull request merges, stop or notify any lane still
running on that milestone, in the same breath as pruning its worktree.

Notify rather than kill, where the tooling allows it. A lane that is mid-thought may hold a
finding worth reporting, and killing it discards that. Telling it its work landed lets it report
what it knows and exit. Killing is the fallback for a lane that will not stop.

**A finished lane is parked, not torn down**, which is what makes notifying possible at all: its
transcript is retained and it can be resumed with its context intact. That is useful (it is how a
report gets read after the fact) and it is also the second half of this confusion: a completed
lane and a running one look identical in a list, exactly as a stale report and a fresh one look
identical in prose. Both are resolved the same way, by checking state rather than reading presence:
the agent list's status for the lane, `git merge-base --is-ancestor` for the merge.

Why it is rung three and cannot be higher. Nothing can make this state unrepresentable: a
long-running gate and an independent merge are both legitimate, and their overlap is a race no type
can rule out. A gate cannot see agent processes. So it is a checklist line, next to the four already
there, and it is honest about being one.

## What this is not

**Not an argument for lanes polling the tree.** That would trade a rare stale report for a broken
isolation model, and the isolation is what makes concurrent lanes safe at all.

Not an argument against long gates. The three-hour Miri run is the cost of the answer; see
milestone 428 (design/roadmap/0428-what-the-weekly-miri-run-should-cost.md).

Not specific to Miri. Any gate long enough for the tree to move under a lane produces this, and
`script/verify` is the queue's long pole for the same reason.

## Index row

Milestone 310's lane ran for 18 hours, roughly 17 of them blocked inside two three-hour Miri runs.
While it was blocked the maintainer merged its pull request, deleted its branch, pruned its worktree
and relinked `nife-dev`; the lane finished, saw its own gates green, and reported that its work was
ready to merge. Everything it said was true when it started waiting and stale by the time it spoke,
and it then cost a second person's attention and a four-way verification pass. No mechanism caught
it because a lane has no visibility outside its own worktree, which is the isolation that lets
several run at once, and because nothing ends a lane except the lane. The proposal is one line on
the merge checklist, notify rather than kill where the tooling allows it so a lane mid-thought can
report what it knows, and it is honest that it cannot be higher than rung three: a long gate and an
independent merge are both legitimate and their overlap is a race no type can rule out.
