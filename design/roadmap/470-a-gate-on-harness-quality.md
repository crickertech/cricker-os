---
status: REFUSED
raised: 2026-09-20
refused_by: 211, 213, 448
---
# 470. A gate on harness quality: self-reference and re-implementation

Refused by milestone 211 (design/roadmap/211-self-referential-harnesses.md),
milestone 213 (design/roadmap/213-harnesses-that-duplicate-the-implementation.md), and recorded
there on 2026-09-03. Backfilled here on 2026-09-20 by
milestone 448 (design/roadmap/448-a-refusal-gets-a-number.md), which gave a refusal that names work a number, a
status and a condition that would change it. *(Number provisional until the merge queue lands it.)*

**The dates are when the refusals were written down, not necessarily when they were made.** Most of
this tree's `- **Refused.**` bullets were written in one sweep on 2026-09-03, so a decision is
usually older than the bullet recording it.

## The refusal, in its own words

From '211. A harness that states its property through the function under test cannot see that
function break', under `## Follow-on`:

> A lint for this defect. No check distinguishes "asserts through the function under test" from
> "legitimately asserts agreement", and agreeing with itself is sometimes the property you want,
> so a gate that flagged every such harness would be wrong more often than right. The output is
> eleven falsification patches and a worklist instead.
>
> -- design/roadmap/211-self-referential-harnesses.md

From '213. A harness that re-implements the code instead of calling it proves nothing about the
code', under `## Follow-on`:

> A gate. No check tells a deliberate model apart from an accidental copy, which is milestone
> 211's reason and it held through the sweep. The discriminator the sweep worked out is a question
> a person answers, "which side of the assertion did the crate produce?", and both sides look like
> arithmetic beside a call to any pattern a lint could match.
>
> -- design/roadmap/213-harnesses-that-duplicate-the-implementation.md

## Why it is here rather than only there

Two sweeps found two defects in the proof suite, two months apart: harnesses that state their
property through the function under test, and harnesses that re-implement the code instead of
calling it. Both sweeps refused a gate, with the same reason arriving twice independently. No check
distinguishes "asserts through the function under test" from "legitimately asserts agreement", and
none tells a deliberate model apart from an accidental copy; both distinctions are answered by a
person reading which side of the assertion the crate produced.

## Revisit

- **Condition.** A mechanical discriminator. Both refusals turn on the same missing thing, and both
  shipped the honest alternative instead: eleven falsification patches and a worklist from one, 148
  harnesses read by hand by the other. A candidate check that can tell a deliberate model from an
  accidental copy is what would reopen this.

## Index row

Two independent sweeps of the proof suite reached the same refusal about gating their own findings,
because the defect is a judgment a lint cannot make. They are gathered here because one refusal
repeated is stronger evidence than two bullets nobody connects.
