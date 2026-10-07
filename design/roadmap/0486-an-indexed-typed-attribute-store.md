---
status: REFUSED
raised: 2026-09-20
refused_by: 57, 448
---
# 486. An indexed, typed attribute store

Refused by milestone 57 (design/roadmap/57-partitioning-and-xattrs.md), and
recorded there on 2026-09-03. Backfilled here on 2026-09-20 by
milestone 448 (design/roadmap/448-a-refusal-gets-a-number.md), which gave a refusal that names work a number, a
status and a condition that would change it. *(Number provisional until the merge queue lands it.)*

**The date is when the refusal was written down, not necessarily when it was made.** Most of this
tree's `- **Refused.**` bullets were written in one sweep on 2026-09-03, so the decision is usually
older than the bullet and its reasoning sits in the block's own prose above it.

## The refusal, in its own words

From '57. Partitioning and formatting a real drive, and extended attributes', under `## Follow-on`:

> An indexed, typed attribute store in the BFS shape. Only opaque blobs are needed today, and
> every attribute already carries a `u32` type code the layer stores, returns and never
> interprets, so an indexed store later is a change of implementation rather than a format
> migration plus a wire break.
>
> -- design/roadmap/57-partitioning-and-xattrs.md

## Why it is here rather than only there

The attribute layer stores opaque blobs, and each attribute already carries a `u32` type code the
layer stores, returns and never interprets. An indexed store in the BFS shape would let a caller
query by type, and nothing today wants to.

## Revisit

- **Condition.** A consumer that queries by type. The refusal is explicit that meeting it is cheap,
  because the type code is already carried: an indexed store later is a change of implementation
  rather than a format migration plus a wire break.

## Index row

A refusal that costs nothing to reverse, because the field the richer design would need is already
on disk and already round-tripped. It is recorded so that the next person to want a typed query
finds the groundwork rather than the argument.
