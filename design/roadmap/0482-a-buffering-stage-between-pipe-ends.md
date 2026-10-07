---
status: REFUSED
raised: 2026-09-20
refused_by: 50, 448
---
# 482. A buffering stage between pipe ends

Refused by milestone 50 (design/roadmap/50-pipes-and-redirection.md), and
recorded there on 2026-09-03. Backfilled here on 2026-09-20 by
milestone 448 (design/roadmap/448-a-refusal-gets-a-number.md), which gave a refusal that names work a number, a
status and a condition that would change it. *(Number provisional until the merge queue lands it.)*

**The date is when the refusal was written down, not necessarily when it was made.** Most of this
tree's `- **Refused.**` bullets were written in one sweep on 2026-09-03, so the decision is usually
older than the bullet and its reasoning sits in the block's own prose above it.

## The refusal, in its own words

From '50. Pipes and redirection: one sink protocol, and `|` turns out to be an endpoint', under `##
Follow-on`:

> A buffering stage. The block said measure before deciding, it was measured on 2026-08-03, and
> the verdict is build nothing: the lockstep is not the bottleneck, the sixteen-byte register-only
> sink message is, and a buffer costs roughly double for decoupling rather than bandwidth.
> `notes/pipes.md` carries the numbers and the honest caveat that the benchmark did not measure
> the case a buffer is actually for.
>
> -- design/roadmap/50-pipes-and-redirection.md

## Why it is here rather than only there

The block said measure before deciding, it was measured on 2026-08-03, and the verdict was build
nothing: the lockstep is not the bottleneck, the sixteen-byte register-only sink message is, and a
buffer costs roughly double for decoupling rather than for bandwidth. `notes/pipes.md` carries the
numbers and one honest caveat that matters more than the numbers.

## Revisit

- **Condition.** A workload that wants decoupling rather than bandwidth, which is exactly what the
  measurement did not cover. `notes/pipes.md` says so itself: the benchmark did not measure the case a
  buffer is actually for, so the refusal is sound about what was measured and silent about what was
  not.

## Index row

A refusal made on measurement, with the measurement's own limits recorded beside it. The condition
is the case the benchmark could not reach, which is the most valuable kind of revisit condition
because the block that wrote it knew what it had not proved.
