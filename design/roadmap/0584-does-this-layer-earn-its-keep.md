---
status: NOT-STARTED
raised: 2026-09-23
promoted_from: does-this-layer-earn-its-keep
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 584. A mechanism owes evidence that it caught something a person would have missed

*(Number minted at promotion.)* Promoted from the proposal
`does-this-layer-earn-its-keep`, filed 2026-09-23, on calef's instruction of 2026-09-24 to promote the proposals.
The text below is the proposal's own, unedited except for this paragraph, the
`## Index row` section, and the nine layer headings, which read `### 1. The watchers` and so
looked like milestone headings to `script/roadmap --check`; they now read `### Layer 1: the
watchers`: the argument is its author's and promotion is not the moment to
improve it. As filed: raised by calef the same day: *"I also want to have metrics on our
actions so that we can assess the effectiveness of our various layers. They need to demonstrate
their value."* Written by a lane after four mechanisms were found on one day reporting success while
checking nothing.

No hardware, no other milestone. One decision was put to calef and he deferred
it on 2026-09-24; the deferral, its reason and its trigger are the last section of this file, and
nothing in the first increment waits on it.

*The slug `does-this-layer-earn-its-keep` is **provisional**, and so is the instrument name proposed
below. Naming is an architect's; a lane ships a provisional name and says so.*

## This is the same argument, one rung up

Do not read this as a new enthusiasm. It is a rule this tree has already made twice, applied to the
next category up.

- §134 (a harness carries a machine-replayable falsification record) rules that a Kani proof is
  not evidence until somebody has made it go red on purpose and left the patch beside the harness.
  Its reason: a proof that cannot fail proves nothing and is indistinguishable, from outside, from
  one that can.
- **`a-gate-is-not-evidence-until-it-has-failed`** (ratified by calef 2026-09-23, in this directory)
  moves that from a harness to a CI workflow. A new gate ships against a tree where its defect is
  absent, so its first result is green, and green is what it would report if it could not fire at
  all.
- This is the third instance. A harness must show it *can* fail. A gate must show it *has*
  failed, once, on purpose. A mechanism must show that over its lifetime it has caught
  something a person would have missed, and it must be able to say so in a number rather than in
  its own header.

The escalation is in what the evidence is about. §134's record is about capability. The gate
proposal's record is about deployment. This one is about yield: not "could it fire" but "did it,
how often, and what did it catch."

## The four findings that force it, all 2026-09-23

Each is a mechanism that reported success while doing nothing, and no two are the same defect.

- `falsifications.yml` replayed nothing for three weekly runs and reported success each time.
  Twenty-two days. *(Corrected 2026-09-24 from "twenty-three": the window starts at the
  workflow's deployment, which is pull request #603's merge to `main` on 2026-09-01, not the
  commit's author time on 2026-08-31. Pull request #1166's correction-of-error record,
  `notes/corrections/2026-09-23-the-sweep-that-swept-nothing.md`, has the derivation.)* The population it was not checking grew from 40 records to 74 in that window.
  Nobody could tell *swept and found nothing* from *swept nothing*, because the workflow emitted the
  same result for both.
- `helpers/trunk-health.sh` reads CI's conclusion to say when `main` goes red. `ci.yml` skipped
  `build + test` for documentation-only commits, so `main` was red for hours with every signal
  green. The watcher's own header claims *"The signal was never missing."* Pull request #1170 is
  correcting that claim.
- **`helpers/merge-drain.sh` logs a snapshot and never an event.** A pass writes
  `10 armed, 7 stalled, of 17 unheld`. There are 3,355 passes on record in
  `~/Library/Logs/nife/merge-drain.log` and not one of them answers "how often did the drain act."
  The maintainer summed the snapshot across passes, got 4,967, and caught the meaninglessness only
  afterwards.
- The falsification worklist counts harnesses, so 342 lines of code carrying no harness at all
  left the ratio clean. The denominator was the population of the thing being measured rather than
  the population it was supposed to cover.

## The distinction that makes any of this possible: events, not snapshots

`notes/project-metrics.md` already carries this correction and it is not re-argued here. Its
velocity section states it in one line: a stock read late is merely stale; a flow read late lands
in the wrong bucket, which is worse than stale because it still looks like history.

The same page shows how rare flows currently are. `notes/project-metrics/weekly.csv` has 58 columns.
Three of them are flows (`milestones_built_this_week`, `merged_pull_requests`, and the cost
columns, which are a capture rather than a derivation). Every other column is a stock: what the tree
holds at a revision.

That is exactly the wrong shape for this question. "How many harnesses are falsified" is a stock and
it is already tracked. "How many times did the sweep replay one, and how many went red" is a flow,
it is the number that separates the two readings of `falsifications.yml`, and nothing in this tree
emits it.

So every measure proposed below is an event count, or the row says why it cannot be one. A
mechanism that can only report its current state is, for this purpose, mute. `merge-drain.sh`'s
3,355 snapshot lines are the proof: complete, honest, retained for weeks, and unable to answer the
one question asked of them.

## The taxonomy: the layers this tree actually has

Enumerated from the tree on 2026-09-23, not from memory. Nine layers, and the last four are where
the honesty is.

### Layer 1: the watchers

`helpers/merge-drain.sh`, `helpers/trunk-health.sh`, `helpers/lane-claim-check.sh`,
`helpers/at-risk-check.sh`, and `script/cadence-check`. Two run unattended under `launchd`; the rest
are run by a session.

What counts as a catch. The watcher acted or announced a transition that nobody was going to
notice: a pull request enqueued that had been sitting, a stall notice posted, a red-trunk transition
reported, a branch found with no claiming pull request, a worktree found holding uncommitted work, a
scheduled workflow found to have stopped producing a result.

### Layer 2: the queue hold

Pull request #1170 (`maintainer/when-main-goes-red`), unmerged as of this writing: hold the queue
when `main` is red and land the fix alone.

What counts as a catch. A hold placed, and under it, a pull request that would otherwise have
been enqueued onto a broken trunk. The second half is what makes it a catch rather than an action,
and it is countable at the moment the hold is placed because the eligible set is exactly what
`merge-drain.sh` computes.

### Layer 3: the blocking gates

`script/lint` carries roughly fifty named checks. `ci.yml` and its siblings put nineteen check names
on a pull request, of which eleven block, per
notes/check-inventory.md (does anything run it, does it block, and what does green mean).

What counts as a catch. A check went red on a pull request, and the branch then changed before
it merged. Red-then-fixed is the catch; red-then-abandoned is a different and also interesting
event; red-then-merged-anyway does not happen here because the required set blocks.

### Layer 4: the scheduled cadences

`falsifications.yml`, `mutation.yml`, `audit-cadence.yml`, `undefined-behavior-check.yml`,
`toolchain-drift.yml`, `vendor-watch.yml`, `stranger-cadence.yml`, `metrics.yml`, `verify.yml`.

What counts as a catch. A run that went red for the reason the cadence exists, or that opened a
pull request nobody had asked for. `script/cadence-check` already answers the weaker question, which
is whether a cadence is still producing a result at all, and its own header records that
`mutation testing` had failed four times out of four over four weeks with nothing turning red about
it.

### Layer 5: the proof and falsification machinery

145-plus Kani harnesses, §134's per-harness falsification records, `script/verify`,
`script/mutation`.

What counts as a catch. A harness that went red against a real change rather than against its
own replay patch. §134's records establish that each harness *can* fail; they say nothing about
whether any of them has ever *caught* anything, and that is the gap this layer has to close about
itself.

### Layer 6: the ledgers that already measure a layer

`script/rule-violations` totals strikes against each documented rule and flags the ones past three.
`script/redo-rate` measures how often delegated work has to be done again. Both are hand-fed, and
both say so in their own headers: `rule-violations` states plainly that *it cannot see a violation
happen*.

What counts as a catch. These are not mechanisms that catch; they are mechanisms that
measure mechanisms, and they are the closest prior art in the tree to what this proposal asks
for. Their design is the model to copy: a ledger a person appends to, honest that its numerator is
what somebody reported.

### Layer 7: the briefs

`briefs/gate-in-ci.md`, `merge-and-cleanup.md`, `rebase-onto-main.md`, `session-start.md`,
`survey-the-queue.md`, `triage-a-failing-check.md`.

What would count as a catch. A lane that did not go wrong. That is the honest answer and it is
also the reason this layer has no measure; see below.

### Layer 8: the role structure

Maintainer, developer, steward. §90 (the claim is a draft pull request), and its draft-as-claim
rule. The lane-count rule against the collision surface. The ladder in `AGENTS.md`.

What would count as a catch. A collision that did not happen, a milestone two lanes did not both
take, a piece of work that did not fall through a crack.

### Layer 9: the records layer

`design/decisions/`, the roadmap, the name provenance blocks, `BUGS` sections,
`design/roadmap/proposals/`, which exists because of
milestone 247 (follow-on work named by a finished milestone goes nowhere).

What would count as a catch. A decision that was not re-litigated because the reason was
written down. A name that was not re-refused. A limitation somebody did not rediscover.

## The measure per layer, and where there is none

| layer | measure | shape | honest? |
|---|---|---|---|
| 1. watchers | actions and transitions emitted as event lines, counted per week | event | **yes** |
| 2. queue hold | holds placed, releases, and the eligible set at the moment of each hold | event | **yes** |
| 3. blocking gates | per-check-name red conclusions on pull requests, and whether the branch changed after | event | **yes, with a clock** |
| 4. scheduled cadences | runs, red runs, and work items each run actually processed | event | **yes** |
| 5. proofs | harness failures against real changes, separated from replay-patch failures | event | **yes, and expected to read near zero** |
| 6. ledgers | already measured; extend nothing | event | **yes, already** |
| 7. briefs | **none** | none | **no, and no proxy is offered** |
| 8. role structure | **none** | none | **no, and no proxy is offered** |
| 9. records | misses only: corrections that cite a missing record | event, wrong sign | **partial, and the asymmetry is the finding** |

### Rows 1 and 2 are the cheap ones

The action sites already exist in the code. What is missing is one line written at each of them,
with a stable prefix a `grep` can count. `merge-drain.sh` computes `armed` and `stalled` on every
pass; what it does not do is say *this pass enqueued #1143*. The lane
`maintainer/what-the-machinery-did` is adding event lines to that file right now. **This proposal
does not edit `merge-drain.sh`**; it names the vocabulary those lines should share with the other
four watchers, so the second one to be instrumented does not invent a second format.

### Row 3 has a clock on it

GitHub retains Actions run history for ninety days. `notes/project-metrics.md` already learned
this the expensive way and says so: the merged-pull-request series is counted from git rather than
from the API precisely because an API-derived series *"would have started in June and could never
have been extended backwards."* Per-check-run conclusions are not in git and cannot be recovered
from it. So this row is measurable, it is measurable only forwards, and every week nobody starts
it is a week deleted rather than deferred.

### Row 5 will read near zero, and that is the point

A Kani harness in a tree where the property holds is silent by construction. Separating "went red
because a lane broke the property" from "went red because §134's replay patch was applied" is the
only way to tell a proof that guards something from a proof that restates a tautology, and §134's
own reverse pass already found one of the latter: `capability::subset_is_reflexive` proves
`a & !a == 0`, which no plausible implementation error breaks.

### Rows 7 and 8 have no measure, and inventing one would be worse

**A brief's value is a lane that did not go wrong, and that is unobservable.** The nearest available
proxy is `script/redo-rate`'s ledger, filtered by whether a lane read the brief. It should not be
used. Lanes are not randomized, n is small, the ledger is hand-fed by the same maintainer who writes
the briefs, and every one of those is a reason the number would move for reasons other than the
brief. A ratio computed that way would be quoted long after the caveat was forgotten.

The same holds for the role structure. Merge conflicts per week looks like a measure of it and
is not: `AGENTS.md`'s own lane-count rule says conflicts scale with *files* rather than with lanes,
so the number moves with which subsystems happen to be in flight.

This tree has a recorded case of exactly the failure a proxy would be. `notes/register-of-measures.md`
carries a row for an `unsafe`/`SAFETY:` parity count that was measured and refused: it disagreed
with the tree in 65 places and every one of them was a document that was right. A check that fails
correct work is not a weak check, it is a check that gets deleted, and the register states that
`script/lint` has already lost three with that signature. An admitted gap survives; a dishonest
measure gets deleted and takes the question with it.

### Row 9 can only report its misses, and the asymmetry is worth stating

A decision record's successes are silent. Its failures are loud and are already written down:
`notes/corrections/` holds them, one file per correction of error. So this layer's only honest
number is a failure count, which cannot be read as effectiveness in either direction. A quarter
with no corrections citing a missing record means either the records are working or nobody wrote a
correction. That is the same ambiguity `falsifications.yml` had, and naming it is all that can be
done about it here.

## The uncomfortable half

**A mechanism that has never caught anything is either unnecessary or blind, and from outside those
two are indistinguishable.** `falsifications.yml` looked like both for three weeks and was the
second one. A workflow that has never gone red is the same sentence: it might be guarding a
property nothing violates, or it might have a `gh` call missing `--repo`, which is what
`coe-architect-label.yml` turned out to be on its first real input.

So this proposal names the consequence and does not wire it up: removing a layer stays a
legitimate outcome of the measurement, and nothing automates it. If a `script/lint` check has
rejected nothing in six months, and the failure it guards against is one the tree can no longer
represent, the check is maintenance cost with a green badge on it. `script/lint` has deleted three
checks already and the tree is better for it.

**Whether a zero-catch count is a reason to delete or only a finding is deferred, not settled**
(calef, 2026-09-24). The last section of this file records the ruling, why the measurement had to
come first, and what brings the question back. Read that before quoting the paragraph above as a
policy: it is the hazard this proposal is watching for, not a rule anybody has adopted.

Two exemptions, written now because the test would otherwise delete the best machinery here.
They survive the deferral: they are facts about what a count can mean, not positions on what to do
about one, so they hold whichever way the deferred question is eventually answered.

1. Rung-one mechanisms are exempt by construction. `AGENTS.md`'s ladder puts *make the wrong
   state unrepresentable* at the top, and its worked example is
   milestone 50 (pipes and redirection), which turned
   `InputSpec::Required` into a variant that carries `writes_while_reading`, so a program that
   writes while it reads cannot be declared without saying so. That mechanism will catch zero
   things forever, because the defect it prevents cannot be typed. A zero-catch count on a rung-one
   mechanism is the mechanism working perfectly. This test applies only to mechanisms that operate
   after the fact, which is rungs two through four: gates, watchers, cadences, records. Deterrence
   and blindness produce the same number and only the rung tells them apart.
2. **A low-frequency, high-consequence mechanism is not judged on count.** `at-risk-check.sh` guards
   the only failure in this system that destroys rather than delays. One catch a year justifies it.
   The measure is still worth taking, because zero catches *and* a lane that lost work would mean it
   is blind.

The first thing the measurement will report is that most layers have no number, and that is
information rather than a reason to postpone.

## The first increment

One layer, instrumented, surfaced where the measures already live. Not nine.

Take layer 1, the watchers. It is the cheapest by a wide margin: the action sites exist, the
change is a line each, the log files already exist and are already retained, and one of the five is
being instrumented right now by another lane, which means the shape can be copied rather than
designed.

1. Fix the event vocabulary in a note, as a short table: one prefix per event kind, one line per
   event, the fields each carries. Do this first and in the same commit as nothing else, because the
   cost of a second format is paid by every later reader.
2. Emit events from the four watchers this proposal may touch: `trunk-health.sh`,
   `lane-claim-check.sh`, `at-risk-check.sh`, and `script/cadence-check`. **Do not touch
   `merge-drain.sh`**; `maintainer/what-the-machinery-did` holds it. Fold its format into the
   vocabulary table afterwards, or ask it to adopt the table, whichever lands second.
3. Count them. A provisional `script/what-fired` that reads the log files and prints events per
   kind per week. It is a reader, not a gate; nothing fails.
4. Surface it in the two places that already exist. Flow columns in
   `notes/project-metrics/weekly.csv`, drawn on
   notes/project-metrics.md (what moved, week by week), which is the page for numbers that move;
   and a row apiece in notes/register-of-measures.md (every number this kernel owes itself),
   almost certainly `dated` rather than `gated`, because nothing should fail when a watcher has a
   quiet week.

**The immediate follow-on, and it is a separate lane because it has a deadline the first does not:**
layer 3's per-check-run capture, since ninety days of history is being deleted while this sits.

**What is deliberately not in the first increment.** Layer 5 needs the replay-versus-real
distinction designed before anything is counted, and getting that wrong produces a number that
conflates the two, which is the defect this proposal is about. Layers 7 and 8 are refused above.

## What this does not solve

Attribution. Separating automation's actions from calef's is a different question, handled
separately by `maintainer/what-the-machinery-did`. It is a prerequisite for some of these
measures and a substitute for none.

Which measures depend on it:

- Layer 3's red-then-fixed count. Whether the fix came from the lane that broke it, from a
  maintainer session, or from calef changes what the number means, and today every commit and every
  pull request in this repository carries calef's name whether he wrote it or not.
- Layer 6's ledgers, which already record delegated work and are already implicitly about who
  did what.
- Layer 2's holds, if the question is ever "who held the queue" rather than "how often was it
  held".

Which do not, and this is why the first increment was chosen as it was:

- Layer 1's event counts. A watcher runs under `launchd` with no human in the loop; every event
  it emits is the machine's by construction.
- Layer 4's cadence runs. Same.

**And this proposal does not gate anything.** Nothing here fails a build. `AGENTS.md` names the
reason in the section on identified work: no check can tell an intention from an observation, and
the layer being measured here is partly prose. This is rung three, a record at the thing itself,
plus a reader that totals the records.

## The ruling: measure first, and the question that comes back

Put to calef as a binary (a reason to delete, or only a finding to write down). His answer on
2026-09-24 was neither, and recording it as either would misstate it:

> I think we want to measure as a first step and then follow up with remediation separately. We
> don't understand the data yet and need to process it. So I'd add it to our metrics page so that we
> can review it together and ask questions. An action might be to delete, but we probably won't
> automate that at first.

What that settles, and it is most of the proposal. Measure first: the first increment proceeds
exactly as written above. The numbers go on the metrics page so they can be read and questioned
together, rather than triggering anything. Remediation is separate work, not a consequence
wired into the measurement. And **deletion is explicitly not ruled out**; it is simply not
automated, and not yet.

What is deferred, stated as a deferral rather than as a posture. Whether a zero-catch count on a
rung-two or rung-three mechanism, over some window, is a reason to remove it. That is an open
decision, and this tree's convention is that an open decision lives in a file rather than in a
conversation. **"A finding only" is a settled posture and is not what was ruled**; this is an open
question with a trigger, and a reader who takes the difference casually will later quote a deferral
as a decision.

Why the measurement had to come first, and this defeats the argument this file originally made.
The proposal argued, and the maintainer agreed and went further by proposing that each mechanism's
window be stated in advance, that the question must be settled before the numbers exist or it gets
settled while looking at a specific check somebody is attached to. calef's counter is the better
argument: **we do not yet understand the data, and a threshold chosen in ignorance is no better than
one chosen under attachment.** Nobody can pick a defensible answer to "how long is too long with
zero catches" without knowing what the distribution looks like across nine layers that have never
been counted. The original argument is kept here rather than deleted, because it was wrong about the
ordering and right about the hazard.

The trigger that brings it back. When layer 1's event counts have been on
notes/project-metrics.md (what moved, week by week) long enough to show a shape across the
layers that have numbers, and calef has read them. That is a judgment about the data rather than a
week count, and putting a week count here would be the same threshold-in-ignorance the ruling
refuses. The first increment's own step 4 is what makes the trigger reachable.

The hazard to carry into that conversation, which the deferral does not dissolve: **decide on
the class, not on an instance.** The reason the question was raised early is that deciding about a
specific check while looking at it is deciding under attachment, and waiting for the data does not
remove that pressure, it postpones meeting it. Whoever takes this up should write the rule for
rung-two and rung-three mechanisms as a class first, then apply it, rather than reading nine rows
and arguing about the smallest number.

The two exemptions above survive the deferral unchanged. Rung-one mechanisms are exempt by
construction, because a defect that cannot be typed produces a zero that means the opposite of
blindness. And a low-frequency, high-consequence mechanism such as `at-risk-check.sh` is not judged
on count. Both are statements about what a count can mean, so neither waits on the deferred
question.

## Index row

Instruments the mechanisms this tree relies on (watchers, gates, cadences, proofs) to emit events rather than snapshots, so each can show it has caught something a person would have missed. Prompted by four mechanisms found on 2026-09-23 reporting success while checking nothing, it is the third step after §134's falsification records and the gate-must-have-failed rule; the first increment is the watchers alone, counted onto the metrics page, and whether a zero-catch count justifies removing a mechanism is deferred by calef until the data exists.
