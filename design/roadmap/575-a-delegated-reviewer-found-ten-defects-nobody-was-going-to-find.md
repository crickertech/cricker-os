---
status: NOT-STARTED
raised: 2026-09-22
promoted_from: a-delegated-reviewer-found-ten-defects-nobody-was-going-to-find
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 575. A delegated reviewer found ten defects in a diff a human had passed as clean

The number is **provisional**: the integrator mints it at merge. Promoted from the proposal `a-delegated-reviewer-found-ten-defects-nobody-was-going-to-find` on 2026-09-22, filed 2026-09-22. Raised by the experiment in milestone 521 (does an AI review of a
pull request catch anything the gates and the maintainer do not), as work its own run turned up and
could not do. The findings, and the check of each against the file, are in
`notes/delegated-review/README.md`.

Every correction is in Markdown that already exists.

## What was found

Run blind against a 465-line documentation commit recording an x86 calibration fix, a rented
reasoning model produced fifty findings over six reviews, of which thirty-eight were true and none
were false. The commit had passed `script/lint`, `script/citations --ratchet` and
`script/roadmap --check`, and a human reviewer had already recorded it as clean.

The ones worth a block, each verified against the file rather than taken from the review:

- A justifying paragraph names the wrong statistic. It argues that "reporting the worst case
  rather than the mean was the instruction that mattered: by the mean, the defect was fixed at three
  windows and had never been very bad at one." Both properties belong to the median, which is the
  column in the table above it. A mean against a 99th percentile of +884% would have exposed the
  defect rather than hidden it, so the sentence teaches the reverse of its own lesson.
- An estimator is called unbiased in a paragraph that says it converges from above. "An average
  would be a biased estimator for precisely the reason the minimum is an unbiased one" is false for
  the minimum of upper bounds at any finite sample; the property is consistency.
- A correction notice over-certifies the comparison a reader is most likely to quote. It
  declares the debug-versus-release argument safe because the calibration cancels within a boot,
  four lines above a methods line reading "Six boots debug, five release."
- A cross-reference resolves to the wrong section, two deliverables in the same commit
  contradict each other about whether a bug is fixed, a paragraph still says "the fix proposed
  above" after the proposal is deleted, and the milestone's own inventory of its diff omits a file
  the commit changes.

## Why this is a proposal rather than a fix

**The commit is another lane's and is not on `main`**, so correcting it here would be a lane editing
work it does not hold. The corrections are cheap; deciding who makes them is the coordination.

## The second half, which is the more interesting one

Whether a large prose diff should be routed to a delegated reviewer as a matter of course. The
same run found that the same model, on a four-line diff, passed the defect and invented concerns on
the clean version, so this is not a proposal to review everything. It is a proposal to decide a
routing rule against the one thing measured: findings scale with reviewable surface, true and false
alike.

## BUGS

- One corpus, five diffs, one adjudicator who was an agent. Nothing here establishes a rate.
  `notes/delegated-review/README.md` records the rest of what the run cannot support.

## Index row

Run blind against a 465-line documentation commit that had passed `script/lint`, `script/citations --ratchet` and `script/roadmap --check`, and that a human reviewer had recorded as clean, a rented reasoning model produced fifty findings over six reviews, thirty-eight of them true and none false. The ten worth a block are corrections to Markdown that already exists: a paragraph that names the mean where the property belongs to the median, an estimator called unbiased in a sentence saying it converges from above, and a correction notice that over-certifies the comparison a reader is most likely to quote.
