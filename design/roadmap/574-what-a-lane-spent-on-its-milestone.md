---
status: NOT-STARTED
raised: 2026-09-21
promoted_from: what-a-lane-spent-on-its-milestone
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 574. What a lane spent on its milestone, joined from the branch it worked on

The number is **provisional**: the integrator mints it at merge. Promoted from the proposal `what-a-lane-spent-on-its-milestone` on 2026-09-22, filed 2026-09-21. Filed by `milestone/519-cost-inputs` from what milestone 519 (what
this project costs, tracked where it cannot rot) found while building the weekly cost columns.
*(Number and slug provisional until the merge queue lands it.)*

It was filed gated on milestone 519 (what this project costs, tracked where it
cannot rot), which is BUILT, so the capture it needs exists. The deadline in the proposal stands
as a fact rather than as a gate: this join can only ever be made over weeks somebody already
captured.

## The finding that makes this cheap, and it was a surprise

Milestone 519's own `BUGS` said machine effort could not be attributed to a milestone because
"nothing links a lane to the milestone it worked on". That turned out to be false. Every
assistant record in the harness's session files carries a `gitBranch` field, and `AGENTS.md` requires
a lane to work on a branch named for its milestone (`milestone/519-cost-inputs`). The join is one
regular expression over data that is already being read and already being committed.

519 did not build it anyway, and the reason is the proposal:

**A branch is not a milestone.** Maintainer branches carry no number. Records written on `main` carry
no lane at all, and there are a great many of them: a maintainer answering a question, a review, a
merge-conflict resolution. A lane that rebases, or one that is cut before its block exists, lands
work under a name that means something different. A wrong per-component cost is worse than no
per-component cost, because a figure in this shape is exactly the kind a stranger quotes.

## Why it is worth a milestone rather than a `BUGS` line

It is the number the comparison actually wants. Milestone 519's block opens by naming seL4 at
about eleven person-years plus nine more at a 20:1 proof-to-code ratio, and Atmosphere at 1.5
person-years on verification alone. Those are *per-artifact* figures. "This project has spent 22.3
billion tokens" is not comparable to either of them; "a verified capability-transfer path cost this
much to build and this much to prove" is. The same join is what would let the deck answer whether a
milestone that carried a Kani harness cost more than one that did not, which is a claim this project
makes and has never measured.

And it is the input `design/fatal-risks/README.md`'s counter-theses need in the unit they are argued in.

## What a lane would have to decide

These are the questions, not the answers. A lane taking this should expect to argue for a rule and
record the refusals.

1. What counts as a lane's work. A branch prefix and a number in the slug is the obvious rule and
   it is not obviously right: a lane that gates from the main checkout, a session that answers a
   question about a milestone from `main`, and a maintainer branch that finishes a lane's work are
   three different cases and only one of them is easy.
2. Where the unattributable goes. The honest options are a named bucket (`main`, `maintainer/`,
   unattributed) carried beside the attributed total, or nothing. It must not be spread across
   milestones pro rata, which would manufacture a number for every block in the tree.
3. Whether the record is per-milestone or per-branch. Per-branch is what the data says and is
   uninteresting; per-milestone is what a reader wants and needs the mapping above.
4. Whether a milestone's cost is ever final. A block that is corrected, extended, or reopened
   accrues more. A cumulative figure is honest and never settles; a first-build figure settles and
   understates.

## What it is not

**Not time tracking**, on the same grounds 519 refuses it: this is machine effort, and nothing here
asks a person to log anything.

Not a per-milestone dollar figure, unless it carries 519's own labels. This project pays a fixed
subscription, so the marginal cost of a milestone is zero dollars, and a retail figure is a shadow
price. A cost-per-milestone number that did not say which of the two it was would be the exact
failure 519 spends a section refusing.

## Index row

Every session record already carries the branch a lane worked on, so the join from machine effort to
milestone is on disk; what is missing is a rule for the cases where a branch is not a milestone, and
a wrong per-component cost is worse than none.
