---
status: NOT-STARTED
raised: 2026-09-03
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 251. A `needs-architect` label outlives the question that earned it, so the queue lies

Minted 2026-09-03 by calef, immediately after a label stayed on a pull
request he had already answered. *(Number provisional until the merge queue lands it.)*

It needs `gh` and a delivery point that already runs.

In brief. AGENTS.md puts the `needs-architect` label at rung two on purpose, so that what is
waiting on an architect is `gh pr list --label needs-architect` rather than a paragraph somebody has
to have read:

> The `needs-architect` label, so the queue is `gh pr list --label needs-architect` rather than a
> paragraph somebody has to have read.

Nothing takes it off. Clearing it depends on a maintainer remembering, which is rung four wearing
rung two's clothes, and on 2026-09-03 pull request #692 sat labeled after calef had answered every
one of its asks. He noticed, not the tooling. A queue with a false entry is worse than no queue,
because the whole value of the label is that its absence means something.

## The hard part is that "answered" is not a thing a script can read

The obvious check is "has calef replied since the label went on", and it is wrong: he may have asked
a clarifying question, or answered one ask of three. Grepping a comment for agreement is the
`git grep -w TODO` mistake, which AGENTS.md already prices at an 82% false-positive rate, and a
check that cannot tell an answer from a question gets disabled in a week.

So this reports rather than decides, which is the same posture `script/cadence-check` took for
scheduled workflows (milestone 238) and `script/roadmap --proposed` took for the proposal pile
(milestone 247). Two signals are available and neither claims to know the answer:

- Activity: a labeled pull request whose newest comment is an architect's is *probably*
  answered and is worth a maintainer's eye. That is a prompt, not a verdict.
- Age: a labeled pull request nobody has touched in N days is stalled whether or not it was
  answered, and stalled is the thing the label exists to prevent.

## Where it goes, and where it must not

Not in `script/lint`. Lint reads the tree; this reads GitHub. A gate that needs the network and an
authenticated `gh` would fail in a lane's worktree, in CI, and on a plane, for reasons that have
nothing to do with the change under test. `script/audits` already records this boundary.

`helpers/trunk-health.sh` is the natural home: it runs under `launchd` on patagonia every five
minutes, it already reports queue conditions a session should act on, and milestone 238 delivered
`script/cadence-check` through it for exactly this reason. Whatever ships should follow that pattern
rather than invent a second one.

## The proof that this milestone worked

**A stale label is reported without anybody asking**, demonstrated by labeling a pull request,
answering it, and watching the report name it. And the converse: a genuinely open one is *not*
reported, so the signal stays worth reading.

## BUGS

- It cannot close the loop, only point at it. Removing the label stays a judgment, because
  deciding an ask is answered is the thing no script can do here. This shortens the gap between
  answered and cleared; it does not remove it.
- `notify()`'s once-per-stall discipline applies, and `notes/merge-queue.md`'s `BUGS` already
  records that neither watcher reports its own death. A report that fires once and is missed is a
  report nobody reads, and this inherits that limitation rather than fixing it.
- It says nothing about the `## What I need from you` comment, which is the other half of the
  mechanism and can go stale in the same way: written when the label goes on, and never revisited
  when one ask of three is answered.
- **Two sessions can both see the report and both act**, which is harmless for a label and worth
  knowing before the same shape is used for anything that is not idempotent.

## Index row

nothing takes the label off, so the one query that says what is waiting on an architect can be
false
