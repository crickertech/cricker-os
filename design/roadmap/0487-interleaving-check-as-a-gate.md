---
status: REFUSED
raised: 2026-09-20
refused_by: 80, 448
---
# 487. `script/interleaving-check` as a gate

Refused by milestone 80 (design/roadmap/80-loom.md), and recorded there on
2026-09-03. Backfilled here on 2026-09-20 by
milestone 448 (design/roadmap/448-a-refusal-gets-a-number.md), which gave a refusal that names work a number, a
status and a condition that would change it. *(Number provisional until the merge queue lands it.)*

**The date is when the refusal was written down, not necessarily when it was made.** Most of this
tree's `- **Refused.**` bullets were written in one sweep on 2026-09-03, so the decision is usually
older than the bullet and its reasoning sits in the block's own prose above it.

## The refusal, in its own words

From '80. Loom: the hand-rolled atomic protocols, model-checked', under `## Follow-on`:

> Making `script/interleaving-check` a gate. A loom model's search cost is exponential in the
> interleavings, so its runtime is a step function, and a gate whose cost is a step function is a
> gate that gets skipped. Revisit when there is a CI job that can absorb it.
>
> -- design/roadmap/80-loom.md

## Why it is here rather than only there

A loom model's search cost is exponential in the interleavings, so its runtime is a step function
rather than a curve. A gate whose cost is a step function is a gate that gets skipped: it is cheap
until somebody adds one more atomic, and then it is not, and the person it blocks is the one who
cannot wait.

## Revisit

- **Condition.** Stated in the refusal: "Revisit when there is a CI job that can absorb it." That is
  the same condition two other refusals in this backfill state, and the three of them together are the
  argument for a cadence this tree keeps reaching for and has not named.

## Index row

A model checker refused as a gate on the shape of its cost rather than its size, which is the more
durable argument: a step function does not become affordable by getting faster. Its condition is a
job that can absorb it, stated in the original.
