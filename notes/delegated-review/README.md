# Does a delegated AI review catch what the gates and the maintainer miss

*Name: provisional, minted 2026-09-22 by milestone 521's lane (`3bea19adf`), for
`notes/delegated-review/`.*

This is the run of milestone 521 (does an AI review
of a pull request catch anything the gates and the maintainer do not), executed 2026-09-22 against
the defects three rented models had produced the same morning. The threshold was written into that
milestone's block and committed **before the first model call**, which `git log` on the block can be
checked against: the registering commit precedes every transcript commit on this branch.

**The short version, and it is not the one the block's own prior predicted.** The expensive
reasoning model, `moonshotai/kimi-k3`, found **ten true defects in a diff a human reviewer had
already judged clean**, and produced **no false positive at all** on that diff across six reviews.
It **missed** the defect on the four-line diff entirely, while reaching the right observation in its
own reasoning and talking itself out of it. The cheap coding model, `qwen/qwen3-coder`, said
**NO CONCERNS** to the same ten-defect diff on five of six reviews.

So the finding is not "AI review works" or "AI review is noise". It is that **findings scale with
reviewable surface, and both the true and the false ones do.** On 465 lines of prose the reasoning
model was better than the humans and the gates combined; on four lines it manufactured concerns on
the clean diff and passed the defective one.

## How blindness was enforced, since nothing else here matters if it was not

The reviewer was never told what it was looking for, was never told which arm it was in, and was
never given this note, the ledger of verdicts, or the brief that named the defects.

- **The prompt came out of two scripts and nothing else.** `helpers/review-bundle.sh` takes a
  worktree and a commit and emits `git show` plus each changed file's pre-image. It holds no table of
  defects and takes no argument that could tell it which commit is which, so it cannot leak an answer
  it does not have. `helpers/review-trial.sh` wraps that in one of two fixed paragraphs and posts it.
- **Every prompt is reconstructible and every answer is committed verbatim**, in
  `transcripts.tar.gz`, one file per model, diff, posture and replicate:

      tar xzf notes/delegated-review/transcripts.tar.gz -C /tmp

  It unpacks to `transcripts/` (the 48 uniform-budget runs) and `transcripts-48k/` (the seven
  supplementary ones). Re-run `helpers/review-bundle.sh` on the same commit and diff it against
  what the transcripts were answering.

**Why it is an archive and not fifty-five files, which a reader meeting a `.tar.gz` in a notes
directory is owed.** Loose, the transcripts failed four gates: the citation scan wanted the
milestone numbers a model cited on an unmerged branch to resolve on `main`, `script/citations`
wanted glosses on them, the counted-claims check measured a reasoning dump's 4735-byte line against
the corpus `documentation`'s renderer is sized for, and the em-dash rule wanted a rented model's
punctuation rewritten. The first attempt at this note added an exclusion to each. **calef refused
that on 2026-09-22:** *"I don't want to change the style rules."* A style gate that grows an
exception per corpus stops being a rule and becomes a list, and the em-dash rule in particular
exists so that prose here reads as a person wrote it, which an exemption for prose a model wrote
would empty out. A gzip is not prose, so no prose gate has an opinion about it and **nothing in the
evidence had to be edited to pass a check.** That is `script/citations`' own argument for the
captured `*.log` fixtures, that nobody wrote those lines as prose, carried one step further:
unlike a machine log, model output contains the very characters the style rule forbids.

**Quotations in this note are verbatim**, on their own `>` lines, which `script/lint`'s em-dash
rule permits since 2026-09-22: a quotation is not the quoter's prose, so a source's punctuation
survives being quoted here. The reasoning is at the check. The archive still holds the originals,
because bulk evidence is not quotation and does not belong inline.
- **The same prompt went to defective and clean diffs alike**, so nothing in the wording separates
  the arms. The two postures differ by one sentence, quoted in `helpers/review-trial.sh`.
- **The adversarial posture does not assert that a defect exists.** It says one *may* be hiding and
  that there may be none. Asserting one would have guaranteed false positives on the clean diffs and
  made the arm that decides this milestone meaningless.
- **What the reviewer did not get**, recorded because it bounds the result: the rest of the tree, the
  branch's sibling commits, and `AGENTS.md`. Two findings below are marked unfalsifiable precisely
  because a single-commit bundle cannot show a reader that the code landed in an earlier commit on
  the same branch. A reviewer with the whole worktree would score differently, in an unknown
  direction.

## The corpus, and why the gates are not its control group

Three defect rows and two clean rows from the delegation ledger of 2026-09-22, which a sibling lane
is landing as `notes/delegation/ledger.tsv`. **Every row in it passed `script/lint`,
`script/citations --ratchet` and `script/roadmap --check`.** That is what makes it the right corpus:
the gates are not the thing being compared against, they are the thing that already failed.

| id | what it is | verdict when it was written |
|---|---|---|
| **D1** | a two-file, four-line renumber that **replaced** the sentence *(Number provisional until the merge queue lands it.)* with its own renumbering note | defect |
| **D2** | an install-offer implementation that emitted `" (unpartitioned or unreadable)"`, folding "safe to wipe" and "we could not read it" into one string on the prompt that asks consent to destroy a disk | defect |
| **C1** | the same renumber done by a different model, which kept the provisional sentence and appended instead | clean |
| **C2** | a 465-line documentation commit recording a calibration fix, carried through a rebase | clean |

**The third defect row is not a separate diff.** A second model made D1's replacement independently,
and the ledger records it as the same replacement; its branch was reset and the commit does not
survive, so it is reported as a repeat of D1 rather than scored twice against an artifact that no
longer exists.

**What D2 is missing, and it bounds the arm.** The ledger's verdict on D2 also records that the lane
skipped the tests its brief required. Nothing in a diff can show that, so only half of D2's recorded
failure was reviewable at all.

## The runs

Two models, four diffs, two postures, three replicates: 48 reviews, plus seven supplementary runs.
Replicates are not decoration. A pilot at a lower token cap got a substantive review out of the same
model and bundle that later answered `NO CONCERNS` three times in a row, and a single call would have
reported whichever one it drew.

**One condition differs and it is recorded rather than hidden.** At the uniform 16000-token budget,
`kimi-k3` spent the entire budget thinking on the largest diff and returned **no answer at all** in
all six cells. Those transcripts are kept with a header saying so, because scoring a model's silence
as "no concerns" is the easiest available way to make this experiment lie. They were re-run at 48000
tokens in `transcripts-48k/` and that is the set scored. `qwen3-coder` never exceeded 693 output
tokens on any cell, so the budget was never binding for it and its runs are unchanged.

## Arm 1: did it name the defect?

Six reviews per model per defect diff. The question is not whether it said something sensible.

| model | D1 (the removed sentence) | D2 (the consent string) | defects named |
|---|---|---|---|
| `qwen3-coder` | **0 / 6** | **0 / 6** strict, 1 / 6 partial | **0 of 2** |
| `kimi-k3` | **0 / 6** | **5 / 6** | **1 of 2** |

**`kimi-k3` named D2 in the ledger's own terms, once, unprompted.** In one neutral review:

> Likewise "unpartitioned or unreadable" conflates two distinct findings. The module itself calls
> this "the single most load-bearing sentence a stranger reads"; it must not be self-contradictory.

Four other reviews reached the same string by a different and arguably better route, that
`format_disk_description` never consults `F_MBR`. Verbatim, and this is the shape of all four:

> 1. **kernel/src/user/install_service.rs — `format_disk_description` reports MBR and backup-GPT disks as
> "unpartitioned or unreadable".** The function only recognises partitions via `F_PRIMARY` (a valid
> primary GPT). The commit defines `F_MBR` and `F_BACKUP`, so the surveyor detects those cases, but the
> formatter never consults them: a disk carrying an MBR partition table — the most common Windows/Linux
> layout a user will actually meet, and the exact case the cited proposal exists to warn about — falls
> through to " (unpartitioned or unreadable)". The module's own docs call this sentence "the single most
> load-bearing sentence a stranger reads"; as written it actively tells a person that a partitioned disk
> is empty right before they type INSTALL.

Same expression, same harm, by a mechanism the ledger had not recorded.

**`qwen3-coder`'s one partial** flagged the same branch as misinterpreting `F_SIZE` and as
"misleading user feedback", which is the right line for a wrong reason and does not reach the
consent stakes. It is scored partial, which the registered threshold counts as a miss.

### The D1 miss is the result worth reading twice

Nothing named D1. But `kimi-k3`'s two adversarial reviews of it **found the observation and
discarded it**, at length, and then spent their whole budget doing so, so the answer never surfaced:

> *"In the diff, the note REPLACED the provisional note. Fine."*

> *"Is there an issue with the renumbering note replacing rather than supplementing the provisional
> note? ... That's actually a real, if subtle, point: the original note existed precisely because
> numbers minted on unmerged branches can collide, which is exactly what happened. ... Thin."*

> *"I'm confident this is fine. ... A reviewer nitpicking this would be wrong to block."*

**Perception was not the failure; judgment was.** A prompt that asked harder would not have fixed
it, because the model already looked harder and then argued itself out of the answer. That is a
fact about the ceiling rather than about the wording, which is exactly what milestone 521's block
said an adversarial arm was for.

**And the same model recommended D1's defect on the clean diff.** Reviewing C1, which kept the
provisional sentence, three of six reviews objected that the paragraph was now self-contradictory
and that the provisional parenthetical *should have been removed*. The reviewer prescribed, on the
correct diff, the exact edit that made the defective one defective.

## Arm 2: what does it cost in noise, and the arm that decided this

Every finding on a clean diff, adjudicated true, false, or unfalsifiable. Fifty-four findings were
adjudicated one at a time; the recurring ones were verified against the files rather than judged from
the prose.

| model | diff | reviews | findings | true | false | unfalsifiable | **false per review** |
|---|---|---|---|---|---|---|---|
| `qwen3-coder` | C1 | 6 | 4 | 0 | 4 | 0 | **0.67** |
| `qwen3-coder` | C2 | 6 | 3 | 0 | 3 | 0 | **0.50** |
| `kimi-k3` | C1 | 6 | 12 | 0 | 8 | 4 | **1.33** |
| `kimi-k3` | C2 | 6 | 50 | 38 | **0** | 12 | **0.00** |

### C2 was not clean, and that is the finding that outranks the score

`kimi-k3` found the following in a commit that passed every gate and that a human reviewer had
already signed off as clean. Each was checked against the file before being counted:

- **`design/roadmap/526`'s justifying paragraph names the wrong statistic.** Checked against the
  file and correct. The reviewer's own words, quoted whole because its punctuation is its own:

> 3. **design/roadmap/0526-….md** — "By the mean, the defect was fixed at three windows and had never been
> very bad at one" mislabels the statistic. The table reports the *median* (+0.36% at one window, +0.00%
> from three up); the *mean* at one window is dominated by the tail — the +1153% and +884% outliers alone
> contribute ~10 points over 200 boots — so the mean would have made the defect look glaring, not
> invisible. The paragraph that justifies the whole estimator choice gets its own statistics backwards.

  The sentence teaches the reverse of its own lesson, in the record the next cap decision will be
  made from.
- **"An average would be a biased estimator for precisely the reason the minimum is an unbiased
  one"** contradicts the same paragraph, which says every window is an upper bound and the minimum
  "converges on the truth from above". An estimator that is always an upper bound is biased upward at
  any finite sample; the property being claimed is consistency.
- **The correction notice over-certifies the one comparison a reader will quote.** It declares the
  debug-versus-release argument safe because the calibration "cancels exactly" within a boot, while
  the methods line four lines below it says *"Six boots debug, five release."* A ratio spanning two
  boots carries the quotient of two unknown calibration errors and does not cancel.
- **A cross-reference resolves to the wrong place.** "the 2026-09-21 section above records that
  CoreMark 'reports correctness, not yet a score'" points at a section that does not contain that
  sentence; it lives in an undated section, and the file has nine other sections dated 2026-09-21.
- **Two deliverables in one commit contradict each other.** `notes/tsc-under-tcg.md`'s `BUGS` still
  says the `qemu-bounded.sh` killer did not fire and sends the reader to another branch, while the
  milestone added in the same commit records the root cause and the runner fix.
- **A paragraph dangles.** `notes/tsc-under-tcg.md` still says "the calibration fix proposed above",
  after this commit deletes the proposal and marks the finding fixed.
- **The milestone's own inventory of its diff omits a file the same commit changes**, and the new
  heading is appended with no blank line before it.

**None of that is arguable and none of it was caught.** The honest consequence is that arm 2's
denominator is wrong: C2's false-positive count is zero partly because the diff was not clean, so
the corpus's clean label is the thing this run falsified first.

### Where the noise actually lives

**On C1, the four-line renumber, `kimi-k3` produced 1.33 false findings per review and no true
ones.** It asked for a reference sweep the bundle could not show it (counted unfalsifiable), and it
asserted three things that are simply not true of this tree: that the renumber should have updated
`design/roadmap/README.md`, which `script/roadmap` regenerates and which a lane is told never to
touch; that the new numbers needed a recorded reservation, which is what the integrator and the gate
already do; and that the appended sentence broke the file's hard-wrap convention, in a paragraph
whose own first line is 458 columns.

**`qwen3-coder`'s four false findings on C1** were all of one shape: asking the commit to prove a
premise it had already stated, and claiming the old number was unrecoverable in a repository that
had just recorded it in the text and in the rename. Its three on C2 include an arithmetic objection
to a figure that is correct: it read 3+3+4+3 windows as 130 ms where the text says 90 ms, which is
the delta over the one-window design, as the sentence it was objecting to says.

### The one disputed finding

`kimi-k3` called *"up to one second of boot, a hundred times this fix's cost"* wrong. It is right if
"this fix's cost" means the full sixteen-window calibration (160 ms, so about six times) and right as
written if it means one window (10 ms, so exactly a hundred). The sentence is ambiguous rather than
wrong, and it is recorded here as disputed rather than resolved in the reviewer's favor.

## The threshold, and what it says

Registered before the run: **adopt** at a third or more of arm 1's defects named with under one false
positive per clean diff; **refuse** under a fifth, or over two false positives per clean diff;
anything else **null**.

| model | arm 1 | arm 2, mean per clean diff | verdict against the registered threshold |
|---|---|---|---|
| `qwen3-coder` | 0 of 2, which is 0% | 0.58 | **REFUSE**, on arm 1 |
| `kimi-k3` | 1 of 2, which is 50% | 0.67 | **ADOPT**, on both arms |

**And the threshold should be distrusted on this corpus, in that order.** `kimi-k3` clears both
bars, and the number that clears the second one is an average of 1.33 and 0.00 over exactly two
clean diffs, one of which turned out not to be clean. Two defect diffs is a denominator where one
scoring call moves the verdict from adopt to null. **Read the D1 transcripts and the C2 findings;
do not read the table.**

## What this says about routing delegated work

`notes/open-model-lanes.md` states the rule that makes a cheaper model safe: the gates are the
oracle, so work with a crisp gate goes to the open model and work whose output is a judgment stays
on Claude. **This run does not overturn that and it does add a third case.** The gates could not see
any of the five defects here, the human reviewer could not see the ten in C2, and a rented reasoning
model could see those ten and not the four-line one.

So the shape to take from it, stated as a hypothesis this corpus is too small to have proved:

- **A large prose diff is where delegated review earns its cost.** Fifty findings, thirty-eight true,
  zero false, on a document nobody was going to reread.
- **A small diff is where it is worst**, in both directions at once. It passed the defective
  four-line diff and it invented concerns on the clean one, and the model's own reasoning shows why:
  with little to read it looks for something to say.
- **It cannot be trusted to weigh a finding it has already made.** The strongest evidence in this run
  is a reviewer holding the right answer and rejecting it as pedantic. Review that surfaces a
  candidate for a human to weigh is a different and more defensible product than review that decides.

## BUGS

- **The adjudication was done by an agent, which is the conflict milestone 521's block named.** The
  block asked for the adjudicator to be stated and the disputed cases recorded rather than only the
  totals; both are done above, and the recurring findings were each checked against the file rather
  than accepted from the prose. It is still one party grading a model of its own kind, and nothing
  here fixes that. The transcripts are committed so the grading can be disagreed with.
- **The corpus is five diffs, of which one turned out to be mislabelled.** No rate computed here
  should be quoted as a rate. Two defect diffs means a single scoring call moves a verdict.
- **The defects are all of one kind**: a true sentence removed, or two meanings folded into one.
  Nothing here says anything about review catching a race, a lifetime bug or an off-by-one.
- **The bundle is one commit and its pre-images, not the worktree.** Several findings ask for
  evidence from files the reviewer was never shown, and they are counted unfalsifiable rather than
  false. A reviewer with the repository would score differently in a direction this run cannot
  predict.
- **Temperature is the gateway's default**, so the transcripts are a record and not a reproducible
  build. The replicate spread is large enough that a single re-run will not match them.
- **Cost was not measured.** `helpers/open-lane.sh` reads OpenRouter's credit balance around a lane
  and this harness does not, so what these 55 reviews cost is not recorded and the adopt case is
  missing its price.
