---
status: NOT-STARTED
raised: 2026-09-03
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 253. A status that can only become wrong after the merge, so no gate can catch it in time

Minted 2026-09-03 by calef, after `main` went red on a status the tree
already had a check for. *(Number provisional until the merge queue lands it.)*

`script/roadmap` already holds the check that fires too late.

In brief. AGENTS.md's rule is that a milestone branch lands its status flip in the same merge
as the work, so `main` never claims something is built while its code is on a branch. Milestone
193's block records the last violation and calls it the integrator's mistake.

On 2026-09-03 it happened again, and this time nothing could have caught it, which is what makes
it a milestone rather than a repeat. Milestone 247 merged carrying:

```
**Status: IN-PROGRESS** on `milestone/247-buried-followon`.
```

`script/roadmap` refuses that by name: *"IN-PROGRESS on a branch which this history has already
merged."* It is a good check and it did fire. It fired after the merge, on somebody else's pull
request.

## Why the existing check cannot be moved earlier

**The condition it tests does not exist until the merge happens.** While milestone 247's branch was
in the queue, the branch had *not* been merged, so "on a branch this history has already merged" was
false and the check passed correctly. The merge is what makes it true. A check on the fact is
unfalsifiable before the event and unavoidable after it.

So the cost lands on whoever runs `script/lint` next, which is an unrelated lane, and the person who
caused it is already gone. On 2026-09-03 that cost was worse than usual: `main` being red
deadlocked its own fix, because every pull request's clippy runs `script/lint` against the merged
tree, so the branch carrying the correction could not go green either. It took a separate minimal
pull request to break it.

## What is checkable, and it is the form rather than the fact

A block on branch `B` whose status reads `IN-PROGRESS on B` is guaranteed to be wrong the moment
`B` merges. That is decidable on the branch, before the merge, with no network and no knowledge of
the future: the branch name is `git rev-parse --abbrev-ref HEAD`, and the claim is in the file.

So the rule a gate can enforce: **a lane may not merge a block that names its own branch as
in progress.** Set the final status first, which is what AGENTS.md already asks for; the gate just
makes the moment unmissable instead of remembered.

This is the ladder's first rung reached by a different route. The wrong state is not quite
unrepresentable, but it is refused at the last moment anybody can act on it, which is the best
available when the fact itself is unobservable until too late.

## What to check before choosing the shape

- `script/lint` check 4b already knows a lane's branch and its milestone, since it requires a
  lane to edit its own milestone's block. This is the same derivation with one more assertion, and
  it should live beside it rather than as a new check somewhere else.
- The merge queue's group build is the other candidate, and it is worse: a group rebases several
  branches together, so "my branch" is ambiguous there in exactly the way it is not on a lane.
- `IN-PROGRESS` naming a branch is the tree's own convention and is useful while a lane is
  running: it is how another session sees who holds a milestone. The gate must not force that
  convention out, only refuse it at the merge.

## The proof that this milestone worked

A branch whose block names it as in progress fails its own pull request, demonstrated by writing
one and watching it go red, and passing once the status is set.

Not a check that only fires on `main`, which is the one that already exists.

## BUGS

- **It does not cover the general case**, which is any status that is wrong for a reason only visible
  after the merge. `IN-PROGRESS on <this branch>` is the one instance anybody has hit twice; a
  `BUILT` claim whose code did not land is the same family and is not addressed here.
- A lane can still pick the wrong final status, and nothing here reads the diff to see whether
  the work is really done. This closes a form that is provably wrong, not a judgment that might be.
- The cost of the failure is what made this urgent, and the gate does not reduce it. A red `main`
  deadlocking its own fix is a property of running `script/lint` on the merged tree, and that stays
  true for every other check in it.

## Index row

milestone 247 merged reading IN-PROGRESS on its own branch; the check that refuses it can only
fire afterwards, on somebody else
