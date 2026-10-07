# The weekly falsification sweep replayed nothing and reported success

**Correction of error, 2026-09-23**, to the shape set by §210 (a correction of error, and its
action items are decisions, proposals or milestones).

## What happened

A Kani harness is a proof that some property of the code holds, checked by a model checker rather
than by running the code. A proof like that has a failure mode a test does not: it can be green
because nothing it asserts can ever fail, and from outside that looks exactly like a proof of
something. So each harness here carries a falsification record, a patch that deliberately breaks
the code the harness covers. Applying the patch must turn the harness red. If it does not, the proof
was never evidence about that code and nobody would have known. That is §134 (a harness carries a
machine-replayable falsification record, or it is not evidence), and it is the reason the sweep
exists at all.

`.github/workflows/falsifications.yml` is that sweep, run weekly: it replays every recorded
falsification patch against its own harness and reports any that stayed green. Milestone 194 (build
§134: the falsification record, its lint, and the sweep that replays it) built it, and pull
request #603 merged it to `main` on 2026-09-01 at 02:16. Its first execution of any kind was the
cron six days later, which refused.

**The workflow was never tested when it was deployed.** No run of `falsifications.yml` has ever
replayed a record, let alone watched one go red on purpose: its Actions history holds seven runs as
of 2026-09-24, the first of them the cron of 2026-09-07, and six refused while the seventh was
cancelled before its sweep step finished. The six records pull request #603 reports "all replaying
red" were replayed by `script/falsifications --sweep` outside Actions, which tested the script and
never the workflow that runs it.

The sweep step is

    script/falsifications --sweep 2>&1 | tee sweep.txt

`tee` creates `sweep.txt` in the repository root, and it creates it before the script it is piped
from has read anything. `script/falsifications --sweep` refuses to run on a dirty working tree,
because it applies and reverts patches with `git apply` and a failure mid-run would otherwise leave
a deliberate defect in somebody's source. That refusal is its own `BUGS` entry, and it is correct:
you cannot sweep while you work. On a runner, the untracked `sweep.txt` is the work.

So the script printed its refusal and exited non-zero. The step carries `continue-on-error: true`,
which was put there for a stated reason: a survivor is a finding to publish rather than a red check
somebody has to silence. The non-zero exit was swallowed, the next step read `sweep.txt` and
published its contents to the job summary, and the job went green. The published report was the
refusal.

Three lines from run 35592675887, the scheduled run of 2026-09-21, in full:

    script/falsifications --sweep refuses to run on a dirty working tree: it applies and
    reverts patches with `git apply`, and a failure mid-run would leave a deliberate
    defect in your source. Commit or set the change aside first.

    ?? sweep.txt

`?? sweep.txt` is the whole of the dirty tree. The file the sweep's own output was being captured
into is the file that stopped the sweep.

## Timeline

| When | What |
|---|---|
| 2026-08-31 18:18 | `24a1e0a7b` is written: the lint hook, the weekly sweep, the per-pull-request gate and the note. The `tee` and the `continue-on-error` are both in that first commit. |
| 2026-09-01 02:16 | Pull request #603 merges it to `main`, which is when the workflow is deployed. The file is not touched on `main` again until the fix on 2026-09-24. |
| 2026-09-01 to 2026-09-07 | Six days in which the workflow is not run at all. There is no dispatch, no trial, and no run of any kind. |
| 2026-09-07 10:47:57 | First scheduled run, 34113314569, and the first execution of this workflow in any form. Refuses at 10:48:35.396, 0.5 s after the step starts. Reports success. |
| 2026-09-14 11:01:12 | Second scheduled run, 34836043991. Refuses at 11:01:46.017. Reports success. |
| 2026-09-21 11:10:46 | Third scheduled run, 35592675887. Refuses at 11:11:24.014. Reports success. |
| 2026-09-23 20:30:04 | `falsify/the-single-harness-crates` dispatches the workflow by hand to check its new records. Refuses at 20:30:52. That lane reads the log rather than the green tick and finds it. |
| 2026-09-23 20:32:30 | The same lane pushes `401203294`, which sends the transcript to `$RUNNER_TEMP` instead of the checkout. Pull request #1156. |
| 2026-09-23 20:32:35 | A dispatch on the fixed commit starts. It is the only run of this workflow that would have swept anything. |
| 2026-09-23 20:36:38 | `falsify/crates-paging` dispatches from its own branch. The workflow's `concurrency` group is the bare string `falsifications`, with no ref in the key, so `cancel-in-progress` cancels the fixed run above. It never replays a patch. |
| 2026-09-23 20:37:53 | That dispatch refuses, on a branch without the fix. |
| 2026-09-23 20:49:25 | `falsify/machine-discovery` dispatches. Refuses. |
| 2026-09-23 20:50:57 | `falsify/crates-paging` posts the same finding as a comment on pull request #1159, deliberately not fixing it, because `.github/` is shared and two siblings were dispatching that workflow at the time. |
| 2026-09-23 21:16:38 | This record's own pull request, #1166, trips another instance of the same shape while being written. `coe-architect-label.yml`, merged hours earlier to put `needs-architect` on every COE by default, detects the new file correctly and then fails to apply the label, because `gh pr edit` was called without `--repo` in a job that never checks the repository out. The step's `if` takes the else arm as designed, the job reports **pass**, and no label appears. |
| 2026-09-24 16:41:34 | Pull request #1156 merges the `$RUNNER_TEMP` fix to `main`. The fixed workflow has not run since; the next scheduled run is Monday 2026-09-28. |
| 2026-09-24 17:22:38 | The maintainer dispatches the fixed workflow on `main` (run 36031723556), and it is the first run in this workflow's life to replay a record: 73 of the 144 replayable records go red as required in about fifteen minutes, then `nifefs::a_short_image_is_refused_not_indexed` hangs from 17:37:58 until the job's 120-minute `timeout-minutes` cancels it at 19:22:23, so the remaining 70 are never reached and the run reports **cancelled**, not success. |
| 2026-09-24 20:15:20 | After #1217 bounds the nifefs record, the maintainer's second dispatch on `main` (run 36053521935, at `ce46107c1`) replays 76 of 144 red, nifefs's `a_short_image_is_refused_not_indexed` now among them in 2.5 s, then hangs on `package_archive::a_short_file_is_refused` from 20:31:45 until the 120-minute job limit cancels it at 22:15:06, leaving 67 unreached; #1243 (that record's bound) and #1242 (a per-record time limit in `script/falsifications`) merged while it ran, but after its commit. |

Six completed runs, six refusals. The count of falsification patches this workflow has replayed in
its lifetime is zero. The one run that carried the fix was cancelled by its own concurrency group
before it got there.

## Impact

Every scheduled sweep the mechanism has ever had produced no evidence, and published a refusal as
though it were a report. That is 22 days from the workflow landing on `main` to the finding, covering the
three Mondays of 2026-09-07, -14 and -21.

**The population it was not checking was growing the whole time**, which matters because the value
of a sweep is in the records it did not see last week:

| Date on `main` | Falsification patches | of which kernel-test | `#[kani::proof]` harnesses |
|---|---|---|---|
| 2026-09-07 | 40 | 4 | 156 |
| 2026-09-14 | 40 | 4 | 156 |
| 2026-09-21 | 74 | 18 | 184 |
| 2026-09-23 | 78 | 18 | 184 |

Nineteen commits touched `*/falsifications/*` inside that window. **Three of them redid a patch
against code that had moved**: `651a3fa43` regenerated the x86 port-revoke patch that pull request
#1059 staled, `00189fefa` redid the W^X record against the encoder as it then stood, and
`8470caa21` refreshed the span falsification against a simplified harness. Stale patches are exactly
what the weekly sweep exists to report. All three were found by a lane already working in that
crate, which is luck rather than mechanism, and is the coverage the sweep was supposed to replace.

**The kernel-test half had no second checker at all.** The per-pull-request half,
`script/falsifications --affected-since` in `ci.yml`, did run throughout and is unaffected, but it
covers only the Kani half and only harnesses a diff can reach. `script/falsifications`' own `BUGS`
section already says a kernel `#[test_case]` falsification is re-checked by a full `--sweep` and by
nothing else. Four such records existed for the first two weeks of the window and 18 for the third.
Nothing looked at any of them.

**What is not claimed.** No survivor and no stale patch is on record for this window that the sweep
would have caught, and none can be, because the instrument produced no reading either way. The
honest statement is an absence of evidence rather than evidence of absence, and the three redone
patches above show the class was live.

**The population moved again on the day of the finding.** `main` today carries 63 of 180 Kani
harnesses replayable, 35%, plus a separate 15 of 16 kernel-test records. Three open falsification
lanes roughly double the first number. The first sweep that actually runs will be the first reading
ever taken, against a record set that has changed more in one day than in the three weeks before it.

**And the worst of it is what the mechanism is for.** `design/fatal-risks/README.md`'s risk 2 is that the
proofs prove trivia and the real bugs live where Kani cannot reach. This sweep is part of how that
risk is answered. The instrument built to ask whether our proofs are evidence was producing none
itself, and saying so in green.

## Root cause: five whys

1. Why did three weekly sweeps replay nothing?
Because `script/falsifications --sweep` refused to run and its non-zero exit was discarded by
`continue-on-error: true`, so the job's only assertion was one that had been deliberately switched
off.

2. Why did it refuse?
Because `tee sweep.txt` wrote an untracked file into the checkout before the script read the tree,
and the script's dirty-tree guard is a guard on the whole tree rather than on the files it patches.
The guard is right to be that broad: it cannot know in advance which files a patch will touch.

3. Why did nobody notice a report about 40 records that contained no records?
Because the report has no reader with an expectation. It is published to a job summary, consumed by
a human on a Monday if anyone opens it, and nothing downstream reads its contents. Its emptiness was
not a signal that anything consumed, so it was not a signal at all. This is the first half of what
went wrong, and it is a property of the design rather than of anyone's attention.

4. Why was a job with no assertion on its content built that way on purpose?
Because `continue-on-error` was implementing a genuine ruling about the **verdict**: §134 says a
survivor is a worklist entry and not a defect in whatever commit happened to precede the cron, so a
survivor must not go red. The mistake is that a step's exit status was carrying two different claims
at once. "The sweep ran and found something to look at" and "the sweep did not run" are not the same
fact, and once they share one channel, suppressing the first necessarily suppresses the second.
Nothing in the workflow could tell them apart afterwards, because the only thing that could have is
the report's own contents, which nothing reads.

**5. Why does this tree keep building mechanisms that cannot tell "checked and found nothing" from
"checked nothing"?**
Because a gate is written against the defect it hunts, and the empty-input case is the one state its
author is not thinking about while writing it. From outside, checking nothing is indistinguishable
from finding nothing, and it is what every new gate does on the day it lands, before its subject
exists. The review that follows asks whether the gate catches the defect; nothing asks what it does
when its input set is empty. This was the fourth instance of the shape found in a single day, and two more arrived while
this document was being written; that recurrence, not the `tee`, is the finding:

- Milestone 401 (a gate that selects the set it judges can pass by checking nothing), pull
  request #1130, went looking for the class in `script/` and found it. Eight selectors now assert
  they selected something, enumerated in `notes/empty-selectors.md`.
- **`script/ci-build`'s tier selector.** It picks its checks by matching a literal tier string in
  its own table with `awk`. Retagging every `local` row to `default` makes the command a developer
  runs before pushing run zero checks and print `ci-build: all pass`, exit 0. Reproduced twice
  independently, by 401's lane and by pull request #1134.
- The draft gate. `ci.yml`'s own comment names it: a skipped job still posts a conclusion and
  still satisfies a required check, so skipping is never the safe default. That file's `gate` job
  exists because it had to learn it more than once.
- This sweep, where the skipped work is the whole subject of the workflow.
- And the falsification ratio itself, which is the strongest member because it is a measurement
  rather than a gate. It counts harnesses, so code carrying no harness is not in its denominator and
  a crate can read 100% falsified while proving nothing about most of itself. Milestone 524 (the
  three x86_64 boot gates) put 338 new lines into `crates/machine_discovery/src/x86_64.rs` on
  2026-09-21 with no harness of their own and the number stayed clean; the milestone 319 (the crate
  that parses firmware had no proofs, and three of its first ones were false) lane on
  pull request #1155 closed that one hole and the blind spot is still open. `design/fatal-risks/README.md`'s
  risk 2 is answered by this number.
- One found by this document's own pull request, which is the strongest evidence in it that the
  shape is a habit rather than four coincidences. The workflow that labels a COE for
  calef's attention reported success while applying no label. Its detection step names the
  repository explicitly and worked; its labeling step inferred the repository from a git remote
  that a checkout-free job does not have, and its deliberate report-and-continue arm turned the
  failure into a pass. Every individual choice there is one this tree argues for on purpose, in
  that file's own header: best-effort, never fatal, a COE must not fail to exist because a label
  API call failed. The result is still a green tick over nothing.

The ladder in `AGENTS.md` ranks how hard to make a rule hold. It has nothing to say about a
mechanism proving it had something to hold. A gate that reported clean should have to say over how
many units, and zero should be loud. That is a rung-two artifact this tree does not have a
convention for, and building one is what the second action item below is.

**And there is a sharper cause than any of the above, which is that this workflow was never once
observed working.** It landed on `main` at 02:16 on 2026-09-01 and its first execution of any kind
was the cron six days later, which refused. No dispatch, no trial run, nothing. A defect that would have
shown itself in 35 seconds of somebody's attention instead took three weeks and a lane reading a log
for another reason.

This tree already knows the argument, in the one place where it is a ruling. §134 (a harness
carries a machine-replayable falsification record, or it is not evidence) says a Kani proof is not
evidence until somebody has made it go red on purpose, because a proof that cannot fail proves
nothing and looks exactly like one that can. A CI gate is a claim of the same kind, and this tree
requires nothing of it. A gate ships green against a tree where its defect is absent, which is
indistinguishable from a gate that cannot fire. That is the fifth why's shape one level out: the gate's
first green is the "checked nothing" case, and it is the case everybody reads as success.

`coe-architect-label.yml` is the second instance, and the contrast is the measurement. It shipped
the same way, unexercised, and reported success while applying no label. It was caught in hours
rather than in three weeks only because a human happened to be reading its log for another reason,
which is the same luck that found the three redone patches above and is not a mechanism.

## Action items

- **Done.** The transcript goes to `$RUNNER_TEMP` instead of the checkout, so the sweep's own
  logging can no longer dirty the tree it is about to patch. Carried by pull request #1156, commit
  `401203294`, on branch `falsify/the-single-harness-crates`; that pull request owns
  `.github/workflows/falsifications.yml` and this record deliberately does not touch it.
- **Milestone 640.** Milestone 640 (a gate is not evidence until somebody has watched it fail). `design/roadmap/0640-a-gate-is-not-evidence-until-it-has-failed.md`: a
  workflow that gates something is not deployed until it has been observed failing
  on purpose, once, and the record of that observation lives with it, the way §134's falsification
  patch lives beside its harness. It may fold into the denominator proposal below rather than
  standing alone; that is calef's call.
- **Milestone 641.** Milestone 641 (a mechanism that reports clean says over how many units, and zero is loud). `design/roadmap/0641-a-mechanism-reports-its-denominator.md`:
  survey the 5 `continue-on-error: true` steps and 19 `|| true` constructs across the 14 workflows,
  decide for each whether it suppresses a verdict or an outcome, and give every job that publishes a
  report a denominator it must assert is non-zero. It is the workflow-level counterpart of what
  milestone 401 (a gate that selects the set it judges can pass by checking nothing) built inside
  `script/`.
- **Done.** `coe-architect-label.yml`'s labeling step now passes `--repo "$GITHUB_REPOSITORY"`, so
  it no longer depends on a git remote that a job without a checkout does not have, and the reason
  is written beside the flag. Carried on this branch, `maintainer/the-sweep-that-swept-nothing`.
  The label on this pull request was applied by hand in the meantime, and it will stay hand-applied
  here: the labeler's `synchronize` path diffs only the push that raised it, deliberately, so that
  a label a human removed does not silently return. No later push to this branch adds a file under
  `notes/corrections/`, so nothing on this pull request will exercise the fixed step. The next COE
  is the first run that can.
- **Recorded.** The `concurrency` group in `falsifications.yml` is the bare string `falsifications`
  with no ref in the key, so a hand dispatch from any branch cancels a running one from any other.
  That is what killed the only run of this workflow that would have swept anything. The limitation
  now sits beside the feature in `notes/falsification.md`'s `BUGS` section, where a lane about to
  dispatch the sweep will meet it.
