---
status: REFUSED
raised: 2026-09-20
refused_by: 112, 448
---
# 453. A lint for a SAFETY comment on a safe fn

Refused by milestone 112 (design/roadmap/112-safety-comments-that-bind.md), and
recorded there on 2026-09-03. Backfilled here on 2026-09-20 by
milestone 448 (design/roadmap/448-a-refusal-gets-a-number.md), which gave a refusal that names work a number, a
status and a condition that would change it. *(Number provisional until the merge queue lands it.)*

**The date is when the refusal was written down, not necessarily when it was made.** Most of this
tree's `- **Refused.**` bullets were written in one sweep on 2026-09-03, so the decision is usually
older than the bullet and its reasoning sits in the block's own prose above it.

## The refusal, in its own words

From '112. The SAFETY comments that bind nobody', under `## Follow-on`:

> A lint for "a SAFETY comment on a safe fn". Neither unsafe lint can read a comment, and a check
> for this shape would fire on the legitimate uses the block separates out, where "caller" means
> the calling thread or process rather than a soundness obligation. If the distinction ever turns
> out to be mechanical, `design/decisions/0061-lints-on-evidence.md` is the ledger that adopts a
> lint on evidence from this tree.
>
> -- design/roadmap/112-safety-comments-that-bind.md

## Why it is here rather than only there

Neither of the unsafe lints can read a comment, so the check would have to be written by hand, and
the shape it would fire on is not the shape that is wrong: a `SAFETY` note on a safe function is
sometimes a soundness obligation nobody holds and sometimes an ordinary caveat where "caller" means
the calling thread or process.

## Revisit

- **Condition.** The distinction turning out to be mechanical, which is the refusal's own wording.
  `design/decisions/0061-lints-on-evidence.md` is named there as the ledger that adopts a lint on
  evidence from this tree, so the route is already written and what is missing is the evidence.

## Index row

A lint here would fire on the legitimate uses as well as the defective ones, because the difference
is what the word "caller" means in a sentence. The refusal names the ledger that would adopt it if
the difference ever becomes mechanical, which makes it a door with a bell rather than a dead end.
