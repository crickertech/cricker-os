---
status: BUILT
raised: 2026-09-21
built: 2026-09-22
---
# 521. Does an AI review of a pull request catch anything the gates and the maintainer do not

Minted 2026-09-21 by calef, who asked for an experiment rather than an
opinion. *(Number provisional until the merge queue lands it.)*

## The question, stated so it can come back no

**Does an AI review of a pull request's diff catch defects that this tree's gates and its maintainer
do not, at a false-positive rate worth paying?**

Both halves are load-bearing. A reviewer that catches real defects and also produces three plausible
findings per clean pull request costs more attention than it saves, and this tree has refused gates
on exactly that ground before: `git grep -w TODO` measured **82% false positives**, and a proposed
gate on the lane line measured **two false positives in four hundred pull requests with zero true
ones**.

## Why the obvious experiment is the wrong one

**Turning review on and reading what it says produces plausible findings with no way to separate the
true ones from the confident wrong ones.** That is the failure mode this tree keeps meeting: a lane
reported a `script/names` blind spot that had been closed for a month; a maintainer measured an hour
of benchmarks against a tree they had clobbered; a census misattributed 77 survivors to a pull
request that introduced four. **Every one of those was plausible.** An experiment whose output is
prose that somebody has to believe is not an experiment.

## The design: retrospective, with ground truth, pre-registered

### Arm 1: can it catch what actually got through?

Take defects whose **introducing commit is identifiable**, reconstruct the diff as it was **without
the fix**, and run the review blind. Score one question: did it name the defect?

`notes/proof-retrospective.md`'s corpus is eighteen entries, each with what found it, and this week
alone added more with known introduction points:

- `drain_sink` ending only on a marker a dying program never sends, so a panic message was delivered
  and discarded.
- All three link scripts discarding `*(.eh_frame*)`, throwing away the compiler's unwind information.
- Milestone 447 (a thread's vector registers are its own)'s stack-frame regression, where a 544-byte
  field cost 1,120 bytes of frame.
- `arch::init` not running on every core on RISC-V, which was green on two architectures and silently
  wrong on the third.
- `compositor`'s `Rect::area`, caught in August and surviving in September.

**The reviewer must not be able to see the future.** Reconstruct at the parent commit, strip anything
that references the fix, and say in the record how that was enforced.

### Arm 2: what does it cost in noise?

Run the same review on pull requests with **no subsequent fix commit touching them**, and adjudicate
every finding as true, false, or unfalsifiable. **This is the arm that decides the milestone**, and
it is the one that will be tempting to skip because it is tedious and its result is likely to be
unflattering.

### The threshold, registered before the runs

**Write it down before running anything**, because a threshold chosen afterwards is a rationalisation
with a number in it. A starting proposal, to be argued with before the first run rather than after:

- **Adopt** if it names a third or more of arm 1's defects at under one false positive per clean
  pull request.
- **Refuse** if it names under a fifth, or exceeds two false positives per clean pull request.
- **Anything between is a null result**, which is a legitimate outcome and must be reported as one.

### Registered 2026-09-22, before the first call, for the run recorded below

The corpus the run above uses did not exist when this block was written, so the threshold is
restated here against it rather than left to be read across from a different corpus. **Written and
committed before any model was called**; `git log` on this file is the evidence, since the commit
that added this section precedes every transcript commit.

- **The corpus is `notes/delegation/ledger.tsv`'s three defect rows of 2026-09-22**, every one of
  which passed `script/lint`, `script/citations --ratchet` and `script/roadmap --check`. The gates
  are therefore not a control group here; they are the thing that already failed.
- **Arm 1 scores one question per defect: did the reviewer name it?** Named, or not. A finding that
  the diff deleted text without saying why the deletion matters is scored **partial** and counted
  against the threshold as a miss, because the class of defect here is a true sentence removed and
  noticing a deletion is not noticing a loss.
- **Arm 2 counts every finding on a clean diff**, adjudicated true, false, or unfalsifiable, with
  false positives per clean diff the denominator.
- **Adopt** at a third or more of arm 1 named, with under one false positive per clean diff.
  **Refuse** under a fifth, or over two false positives per clean diff. **Anything else is null.**
- **Per model, never averaged**, because the point of running two is that they may differ.

## The variable worth building in

**The reviewer is the same model family that wrote the code**, so its blind spots may be the lane's
blind spots, and a clean result might mean agreement rather than correctness. Run a subset with an
**adversarial posture** ("find the defect this diff introduces") against the same diffs reviewed
neutrally ("review this diff"), and report whether detection moves. If it does not, that is a fact
about the ceiling rather than about the prompt.

## The honest prior, recorded in advance so the result can contradict it

**The defects this tree actually found were found by running things**: a QEMU boot, a mutation sweep,
a GDB backtrace, a bench on three harts. `design/fatal-risks/README.md`'s risk 2 already records the
strongest version of this, that no Kani harness has caught a defect after the day it was written.

So the maintainer's prior is that review's plausible niche is the class the gates cannot see: a
comment that is wrong, a record that has gone stale, a gloss that resolves to the wrong thing. **That
is also precisely the class where a plausible-but-wrong reviewer does the most damage**, because
nothing downstream checks it. The experiment exists to find out whether that prior is right.

## Prior art in this tree

- **`notes/stranger-test.md`** already runs an agent against this tree as an experiment, with a
  protocol, a journal and costs recorded per run. It is the shape to copy, including that it records
  what the harness itself cost.
- **Milestone 191 (did the proofs catch the bugs?)** is the retrospective-against-a-corpus shape, and
  it returned an uncomfortable answer that the tree kept.

## The result, 2026-09-22

**Run and recorded in `notes/delegated-review/README.md`**, against the three defect rows and two
clean rows the rented models produced that morning, every one of which passed `script/lint`,
`script/citations --ratchet` and `script/roadmap --check`. Fifty-five reviews, two models, both
postures, three replicates a cell, every transcript kept unedited in an archive beside the note.

**The answer to the question this block asks is yes, and the evidence is not the score.** On the
465-line documentation diff the ledger had marked **clean**, `moonshotai/kimi-k3` produced fifty
findings across six reviews: **thirty-eight true, zero false**. Among them a justifying paragraph
that names the mean where its own table reports the median, an estimator described as unbiased in a
sentence that also says it converges from above, and a correction notice certifying as exact a
comparison its own methods line shows spans two boots, each with its own
calibration error. None of that was caught by a gate or by the human who signed the diff off.

**And the prior in this block survives anyway, because the same model missed the four-line defect.**
Nothing named D1, the diff that replaced *(Number provisional until the merge queue lands it.)*.
Two of `kimi-k3`'s adversarial reviews **reached the observation and rejected it** in their own
reasoning, verbatim:

> Is there an issue with the renumbering note *replacing* rather than *supplementing* the
> provisional note? ... That's actually a real, if subtle, point: the original note existed
> precisely because numbers minted on unmerged branches can collide, which is exactly what
> happened. ... Thin.

> I'm confident this is fine. ... A reviewer nitpicking this would be wrong to block.

Perception was not the failure, judgement was, which is the ceiling fact the adversarial arm was
built to expose. Reviewing the **clean** version of the same renumber, the same model three times
recommended removing the provisional sentence, which is the defect.

| model | arm 1: defects named | arm 2: false per clean diff | against the registered threshold |
|---|---|---|---|
| `qwen/qwen3-coder` | 0 of 2 | 0.58 | **REFUSE** |
| `moonshotai/kimi-k3` | 1 of 2 | 0.67 | **ADOPT** |

**The threshold is met and should be distrusted, in that order.** Two defect diffs is a denominator
where one scoring call moves the verdict, and one of the two clean diffs turned out not to be clean,
which makes arm 2's denominator wrong in the direction that flatters the reviewer. The finding worth
carrying is not the verdict but its shape: **findings scale with reviewable surface, true and false
alike.** A large prose diff is where delegated review earns its cost; a four-line diff is where it
passed the defect and invented concerns on the clean one, in the same pair of runs.

## BUGS

- **The corpus is small and not evenly distributed.** `notes/proof-retrospective.md` says so of its
  own eighteen: five are concurrency. A result on this corpus is a result about this project's defect
  history, not about code review in general.
- **Adjudicating arm 2 is a judgement call**, made by the same maintainer whose attention the
  reviewer would be spending. State who adjudicated and record the disputed cases rather than only
  the totals.

## Follow-on

- **Recorded.** `notes/delegated-review/README.md`'s `BUGS` carries what this run cannot support: an
  agent adjudicated its own kind, the corpus is five diffs of which one was mislabelled, every defect
  in it is of one shape (a true sentence removed or two meanings folded into one), the reviewer saw
  one commit rather than the worktree, and the run's cost was never measured.
- **Refused.** Excluding the transcripts from the citation, counted-claim and em-dash gates. The
  first attempt at this run did exactly that, one exception per gate, and calef refused it on
  2026-09-22: *"I don't want to change the style rules."* A style rule that grows an exception per
  corpus is a list rather than a rule. The evidence is kept unedited in
  `notes/delegated-review/transcripts.tar.gz` instead, where no prose gate has an opinion about it,
  and the note says why a reader is looking at an archive.
- **Milestone 575.** The corrections themselves are milestone 575 (a delegated reviewer found ten
  defects in a diff a human had passed as clean),
  `design/roadmap/575-a-delegated-reviewer-found-ten-defects-nobody-was-going-to-find.md`. It
  carries the ten true findings the reviewer made in a commit a human had passed as clean, and the
  routing question they raise. They are not this lane's to fix: the commit is another lane's and is
  not on `main`.
- **Refused.** Adopting delegated review as a standing step is not this milestone's to decide. The
  experiment was built to inform that call and the threshold it registered is met by one model of
  two, on two defect diffs, one of which every model missed. A recommendation from the lane that ran
  the experiment would be the lane grading its own result.

## Index row

An experiment rather than an opinion: whether an AI review catches defects the gates and the
maintainer miss, scored against defects whose introducing commits are known, with the false-positive
arm that decides it and a threshold registered before the first run.
