---
status: NOT-STARTED
raised: 2026-09-19
promoted_from: a-due-cadence-reaches-nobody
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 495. A cadence that says "due" reaches nobody, and the watcher cannot tell due from dead

*(Number provisional until the merge queue lands it.)* Promoted from the
proposal `a-due-cadence-reaches-nobody`, filed 2026-09-19, on calef's instruction of 2026-09-20 to
give every proposal on `main` a number. The text below is the proposal's own, unedited except for
this paragraph: the argument is its author's and promotion is not the moment to improve it. Written
by milestone 117 (the stranger test)'s run 6 lane, from what the run showed about the cadence that was supposed to have
started it.

A lane can start today. Every input is already on disk or one `gh` call away.

In brief. Two scheduled workflows exist to say "a person must now do something": the stranger
cadence (`script/stranger-test --due`) and the audit cadence (`script/audits`). Both say it by going
red in the Actions tab. Nothing carries that red to a person. `script/cadence-check`, which
`helpers/trunk-health.sh` runs so that scheduled failures stop being invisible, reports a workflow
only once it has gone fifteen days without a *success*, so a job that is correctly reporting "due"
and a job that is broken arrive as the same verdict, DEAD, a fortnight after the fact.

## The evidence

- **The stranger cadence was never tested.** Run 5 was 2026-08-18, so a run came due on
  2026-09-17. The workflow asks on Mondays: green on 2026-09-14 (27 days), and it could not go red
  before 2026-09-21. Run 6 happened on 2026-09-19 because a maintainer briefed a lane, which is the
  "somebody thought of it" this mechanism was built to replace.
- The audit cadence shows what happens when one does go red. On 2026-09-19 it had failed on
  every Monday since 2026-08-17, five in a row, and `script/cadence-check` lists it as
  `DEAD: never succeeded`, which is true and says the wrong thing: the job works, and what it has
  been saying for five weeks is "audits are due".

## What it would take, and the options

The question is what `due` should look like to the one watcher a person reads. Three shapes, each
priced by what it adds:

1. **Ask the scripts directly, not the workflows.** `trunk-health.sh` already runs on patagonia
   every ninety seconds with a checkout in hand; `script/stranger-test --due` and the audits'
   equivalent need no network, no `claude` and no toolchain. A `DUE:` line on transition, beside
   the existing `DEAD:` ones, closes the gap without touching GitHub. Recommended, because it
   reuses the watcher a maintainer session is already told to confirm is alive.
2. Teach `cadence-check` a third verdict, reading a job's own summary to tell "failed because
   something is due" from "failed". Keeps one source, but it parses step summaries, which is
   fragile, and still inherits the weekly lag.
3. Open an issue from the workflow when it goes red. Durable and visible, but it needs the
   workflow to hold `issues: write`, which every workflow here has declined on least-privilege
   grounds, and it is a second channel to the one the watcher already is.

Reversible: all three are tooling on one machine or one workflow, and nobody else has acted on
the current behaviour except by not acting.

## Scope note

This is not a stranger-test fix. The stranger cadence is one of two consumers, and fixing it only
there would leave the audit cadence red for a sixth week.

## Index row

Two scheduled workflows exist to say "a person must now do something": the stranger cadence
(`script/stranger-test --due`) and the audit cadence (`script/audits`).
