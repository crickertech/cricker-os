---
status: REFUSED
raised: 2026-09-20
refused_by: 15, 448
---
# 458. ASID generations and rollover

Refused by milestone 15 (design/roadmap/15-asids.md), and recorded there on
2026-09-03. Backfilled here on 2026-09-20 by
milestone 448 (design/roadmap/448-a-refusal-gets-a-number.md), which gave a refusal that names work a number, a
status and a condition that would change it. *(Number provisional until the merge queue lands it.)*

**The date is when the refusal was written down, not necessarily when it was made.** Most of this
tree's `- **Refused.**` bullets were written in one sweep on 2026-09-03, so the decision is usually
older than the bullet and its reasoning sits in the block's own prose above it.

## The refusal, in its own words

From '15. Tagged address spaces (ASIDs)', under `## Follow-on`:

> The generation and rollover scheme this block sketched from Linux. Milestone 14 bounds
> concurrent address spaces at 160, below the smallest hardware ASID space of 256, so the
> exhaustion path the generations guard is unreachable here, and machinery whose hard path can
> never run is machinery that rots. If `MAX_SPACES` ever passes 255 the first answer is 16-bit
> ASIDs, not a new algorithm.
>
> -- design/roadmap/15-asids.md

## Why it is here rather than only there

Linux keeps a generation counter beside its ASIDs so that when the hardware space is exhausted it
can roll over, bump the generation and invalidate lazily. Milestone 14 (kernel objects from untyped) bounds concurrent
address spaces at 160, below the smallest hardware ASID space of 256, so the exhaustion path those
generations guard cannot run.

## Revisit

- **Condition.** `MAX_SPACES` passing 255. The refusal names the answer as well as the trigger,
  which is unusual and worth keeping: "If `MAX_SPACES` ever passes 255 the first answer is 16-bit
  ASIDs, not a new algorithm." So the bell rings on the bound, and what it rings for is a different
  piece of work than the one refused.

## Index row

Generation-and-rollover machinery whose hard path can never run is machinery that rots, and this
kernel's address-space bound puts that path out of reach. The refusal names both the number that
would change it and the answer that would come first, which is why it is recorded as a door rather
than as a gap.
