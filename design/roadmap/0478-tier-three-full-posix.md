---
status: REFUSED
raised: 2026-09-20
refused_by: 36, 448
---
# 478. Tier three: full POSIX behind the foreign-language seam

Refused by milestone 36 (design/roadmap/36-foreign-component.md), and recorded
there on 2026-09-03. Backfilled here on 2026-09-20 by
milestone 448 (design/roadmap/448-a-refusal-gets-a-number.md), which gave a refusal that names work a number, a
status and a condition that would change it. *(Number provisional until the merge queue lands it.)*

**The date is when the refusal was written down, not necessarily when it was made.** Most of this
tree's `- **Refused.**` bullets were written in one sweep on 2026-09-03, so the decision is usually
older than the bullet and its reasoning sits in the block's own prose above it.

## The refusal, in its own words

From '36. A foreign-language component, seam first (spike; feeds 29 and 23)', under `## Follow-on`:

> Tier three, full POSIX (`open`, `fork`, `socket`, threads), stays out. It needs a real libc
> port, which DECISIONS §15 prices at "later, if ever", and a component that wants it is a
> different and much larger project than this one.
>
> -- design/roadmap/36-foreign-component.md

## Why it is here rather than only there

The foreign-language seam has tiers, and the third is `open`, `fork`, `socket` and threads: enough
POSIX that arbitrary C could be ported rather than adapted. It needs a real libc port, which §15 (the native ABI: formalize the convention) prices at "later, if ever", and a component that wanted it would be a different and
much larger project than this one.

## Revisit

- **Condition.** A component somebody needs that cannot be adapted to the narrower tiers. Nothing on
  the roadmap has asked, and the refusal's phrasing is a pricing rather than a prohibition, which is
  why this is recorded as refused rather than as impossible.

## Index row

The largest single thing this tree has declined to build, priced once in a decision record and then
refused in a bullet. It is not refused on principle: it is refused for want of a component that
would need it, which is a condition a stranger could meet.
