---
status: NOT-STARTED
raised: 2026-09-23
promoted_from: a-mechanism-reports-its-denominator
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 641. A mechanism that reports clean says over how many units, and zero is loud

Promoted from `design/roadmap/proposals/a-mechanism-reports-its-denominator.md` on 2026-10-03 (UTC). The number 641 was minted by the maintainer in a batch promotion of the proposal pile and is provisional until the queue lands it. *(Title and slug are drafts.)*

<!-- writing-standards: exception. Granted 2026-10-03 (UTC) by the maintainer minting this milestone, not ratified by an architect. Reason: this block was promoted unedited from design/roadmap/proposals/, which the prose scope excludes, so it meets the sentence and bold limits only after an edit that promotion does not make. Trimming it is a separate pass, and the exception goes when it is done. -->

`a-mechanism-reports-its-denominator`: ratified 2026-09-23 (calef, reviewing
`notes/corrections/2026-09-23-the-sweep-that-swept-nothing.md` on pull request #1166). Raised by
`notes/corrections/2026-09-23-the-sweep-that-swept-nothing.md`, whose fifth why reaches a habit
rather than a bug and whose action items must resolve to something under §210 (a correction of
error, and its action items are decisions, proposals or milestones).

No hardware, no other milestone, no decision owed. This is work nobody has taken.

## The claim

**From outside, a check that examined nothing looks exactly like a check that found nothing.** That
is not an observation about one defect; it is the default state of every gate on the day it lands,
before its subject exists, and nothing in this tree's review asks about it. The reviewer's question
is "does this catch the defect". The question that goes unasked is "what does this do when its input
set is empty".

Six instances turned up in one day, 2026-09-23:

- **Milestone 401 (a gate that selects the set it judges can pass by checking nothing)** swept
  `script/` for the class and guarded eight selectors, enumerated in `notes/empty-selectors.md`.
- **`script/ci-build`'s tier selector** runs zero checks and prints `ci-build: all pass` with exit 0
  if the tier column is renamed. Reproduced independently twice, by 401's lane and by pull request
  #1134.
- **The draft gate.** `ci.yml`'s own comment: a skipped job still posts a conclusion and still
  satisfies a required check, so skipping is never the safe default.
- **The weekly falsification sweep**, which replayed zero patches in all three of its scheduled runs
  and reported success each time, because the transcript it was writing dirtied the tree its own
  guard protects and `continue-on-error` discarded the refusal.

- **The labeler for corrections of error**, `coe-architect-label.yml`, merged the same day. It
  detected a new COE record correctly and then failed to apply the label, because `gh pr edit` was
  called without `--repo` in a job with no checkout; its deliberate never-fail arm reported the
  failure to the log and the job to GitHub as a pass. Fixed on the branch that found it, which was
  the branch writing the correction of error about the other four.

- **And the falsification ratio**, which is not a gate at all but the same defect in a measurement.
  It counts harnesses, so code with no harness is absent from its denominator and a crate can read
  100% falsified while most of it is unproved. Milestone 524 (the three x86_64 boot gates) added 338
  lines to `crates/machine_discovery/src/x86_64.rs` on 2026-09-21 with no harness of their own and
  the number did not move; found and reported by the milestone 319 (the crate that parses firmware
  had no proofs, and three of its first ones were false) lane on pull request #1155, which
  closed that instance and not the class.

Milestone 401 fixed the `script/` half. **The workflow half is untouched**, and it is the half where
the swallowing is explicit and deliberate rather than accidental.

## What this would look for, measured rather than asserted

Counted on `main` at 2026-09-23 across `.github/workflows/`, 14 files:

| Construct | Count | Files |
|---|---|---|
| `continue-on-error: true` | 5 | `falsifications.yml`, `metrics.yml` (2), `toolchain-bump.yml` (2) |
| `|| true` | 19 | 8 workflows, most of them in `ci.yml` (6) and `toolchain-bump.yml` (4) |
| `if: always()` | 6 | `mutation.yml` (5), `falsifications.yml` |
| `tee` into the checkout | 1 | `falsifications.yml`, and it is the one this came from |

The `tee` row is worth keeping in the table precisely because it is now 1 and was the whole defect.
The other three constructs are the population, and none of them is wrong in itself.

## The distinction to make, per site

**Is this suppressing a verdict or an outcome?** They wear the same clothes and only one is safe.

- **A verdict.** "The sweep ran, and something in the result wants a human." §134 (a harness carries
  a machine-replayable falsification record, or it is not evidence) rules that a survivor is a
  worklist entry and not a defect in the preceding commit, so a survivor must not go red.
  Suppressing that is correct and stays.
- **An outcome.** "The thing did not run." Suppressing that is never correct, and today every
  `continue-on-error` site suppresses both, because a step's exit status is one channel carrying two
  claims.

The separation does not need new machinery. **A report that states its denominator is enough**: a
sweep that says it replayed N records cannot claim N when it replayed none, and a job that asserts
its own N is non-zero fails loudly on the one case `continue-on-error` must not hide, while staying
silent on the verdict it was written to let through.

## What building it looks like

1. **Walk the 30 sites above** and label each verdict or outcome, in a note the way
   `notes/empty-selectors.md` did for `script/`. The label is the deliverable; most sites will need
   no change.
2. **Give each reporting job a denominator.** `script/falsifications --sweep`, `script/mutation`,
   `script/audits --due` and `script/stranger-test --due` all already know how many units they
   examined. Have them print it in a shape a workflow can read back, and have the workflow assert
   it is not zero. This is rung two of `AGENTS.md`'s ladder, a gate that fails loudly, and it does
   not touch the verdict.
3. **Consider rung one where it is cheap.** A script that exits non-zero when its own unit count is
   zero needs no workflow cooperation at all, and makes the wrong state unrepresentable from the
   workflow's side. Where it is not cheap, say so and take rung two.

## And the same defect in the tree's own metrics

**A reported percentage must say what it is a percentage of.** The four workflow constructs above
hide a zero in an exit status; a ratio hides one in a denominator, and the second is worse because
the number is published, quoted and used to answer `design/fatal-risks/README.md`'s risk 2 (the proofs
prove trivia and the real bugs live where Kani cannot reach). `script/falsifications` reports 63 of
180 harnesses, `script/metrics` carries several ratios of the same family, and none of them states
the population it drew from or what it excluded.

**The survey is the deliverable, not the fix.** Walk the ratios this tree publishes, and for each
one write down what its denominator counts and what falls outside it, beside the number. Whether a
better denominator exists for the falsification ratio (lines, functions, public surface) is a real
question with real costs and this proposal deliberately does not answer it; naming which numbers
have the problem is what makes answering it possible.

## What this deliberately is not

**Not a ban on `continue-on-error`.** The reason it is in `falsifications.yml` is good and is
written in the file. A proposal that removed it would turn every survivor into a red check somebody
has to silence, which is the outcome §134 argued against.

**Not a sweep of every script.** Milestone 401 did `script/`. This is the workflows, and the two
together are the tree's mechanisms. Extending it further would be speculative.

**Not a claim that any other site is currently broken.** Only the falsification sweep is known to
have been. The other 29 are a population to label, and labelling them is most of the value, because
the label is what the next person to add a `|| true` will read.

## Index row

From outside, a check that examined nothing looks like a check that found nothing. Proposed: every mechanism that reports clean says over how many units, and zero is loud.
