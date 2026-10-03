---
status: PROPOSED
raised: 2026-09-23
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# A gate is not evidence until somebody has watched it fail

`a-gate-is-not-evidence-until-it-has-failed`: ratified 2026-09-23 (calef, reviewing
`notes/corrections/2026-09-23-the-sweep-that-swept-nothing.md` on pull request #1166). **It may
belong inside `design/roadmap/proposals/a-mechanism-reports-its-denominator.md` rather than standing
on its own**, since both come out of the same correction and the same fifth why. That is an
architect's call, and ratifying the name does not settle it. Raised by
`notes/corrections/2026-09-23-the-sweep-that-swept-nothing.md`, which asked whether the failing
workflow was ever tested when it was deployed. It was not.

No hardware, no other milestone, no decision owed.

## The argument, which this tree has already made once

§134 (a harness carries a machine-replayable falsification record, or it is not evidence) rules that
a Kani proof is not evidence until somebody has made it go red on purpose and left the patch that
does it beside the harness. The reason is that a proof which cannot fail proves nothing and is
indistinguishable, from outside, from one that can.

**A CI gate is a claim of exactly that kind, and this tree requires nothing of it.** A new gate
ships against a tree where its defect is absent, so its first result is green, and green is what it
would also report if it could not fire at all. Nobody can tell which they are looking at, and the
first green is the one everybody reads as confirmation that the thing works.

## The two instances that prompted it

**The weekly falsification sweep.** `falsifications.yml` landed on `main` on 2026-09-01. Its first
execution of any kind was the cron six days later, which refused to run and reported success. It
replayed zero patches in three scheduled runs over three weeks, and the population it was not
checking grew from 40 records to 74 in that window. A dispatch on the day it landed would have shown
it in 35 seconds.

**`coe-architect-label.yml`**, merged 2026-09-23 so every correction of error reaches calef by
default. Also shipped unexercised. On the first COE it ever saw it detected the file correctly,
failed to apply the label because `gh pr edit` was called without `--repo` in a job with no
checkout, and reported **pass**. It was found in hours rather than in three weeks only because a
human was reading the log for another reason.

Both are the same shape one level out from the correction's fifth why: the gate's first green is the
*checked nothing* case.

## The proposal

**A workflow that gates or reports on something is not deployed until it has been observed failing
on purpose, once, and the record of that observation lives with it.**

- **The observation is a run, not an argument.** A link to a run where the job went red for the
  reason it exists to catch, or, where a real failure cannot be staged, a run of the underlying
  script against a deliberately broken tree. §134's own standard is a patch that a machine can
  replay; this is weaker on purpose, because a workflow's inputs are not a source tree and a
  replayable version would cost more than it buys.
- **It lives with the workflow**, in the file's header beside the reasoning already there, which is
  rung three of `AGENTS.md`'s ladder: a record at the thing itself, read by the next person to touch
  it. The alternative, a registry, is the shape milestone 115 (the names that were ratified, and the
  ones that were refused) exists to refuse.
- **It applies to new workflows and to a step whose failure arm is added later**, since that arm is
  what carries the claim. It does not apply retroactively to the 14 workflows already here; auditing
  those is the denominator proposal's survey.

## What this is not

**Not a required check.** Nothing gates the gate. This is a deployment habit written where the
deployer reads, and a workflow whose header has no such record is a finding for whoever next opens
it rather than a build failure.

**Not a claim that testing a gate is hard.** Both instances above would have been caught by one
`workflow_dispatch` and 35 seconds of reading. The cost is not the obstacle; nobody asking is.
