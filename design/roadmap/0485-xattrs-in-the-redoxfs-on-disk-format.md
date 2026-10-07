---
status: REFUSED
raised: 2026-09-20
refused_by: 57, 448
---
# 485. Extended attributes in the RedoxFS on-disk format

Refused by milestone 57 (design/roadmap/57-partitioning-and-xattrs.md), and
recorded there on 2026-09-03. Backfilled here on 2026-09-20 by
milestone 448 (design/roadmap/448-a-refusal-gets-a-number.md), which gave a refusal that names work a number, a
status and a condition that would change it. *(Number provisional until the merge queue lands it.)*

**The date is when the refusal was written down, not necessarily when it was made.** Most of this
tree's `- **Refused.**` bullets were written in one sweep on 2026-09-03, so the decision is usually
older than the bullet and its reasoning sits in the block's own prose above it.

## The refusal, in its own words

From '57. Partitioning and formatting a real drive, and extended attributes', under `## Follow-on`:

> Extending the RedoxFS on-disk format for extended attributes, in favour of layering them in the
> FS server. Normally the layer is dismissible because on Linux anything can open the file
> directly and bypass it; here nothing can, since all access goes through `fs_proto`. The format
> extension would also be a materially larger divergence from the 0.9.1 pin that every future bump
> pays for.
>
> -- design/roadmap/57-partitioning-and-xattrs.md

## Why it is here rather than only there

Extended attributes are layered in the FS server rather than written into the on-disk format. On
Linux that layering would be dismissible, because anything can open the file directly and bypass it;
here nothing can, since all access goes through `fs_proto`, so the layer is as strong as the format
would be. The format extension would also be a materially larger divergence from the 0.9.1 pin, paid
at every future bump.

## Revisit

- **Condition.** Something bypassing `fs_proto`, which is what would make the layer weaker than the
  format. Nothing can today, and the capability model is why, so this refusal holds exactly as long as
  that stays true. The pin cost is the second half and it moves in the other direction: the longer the
  divergence stays small, the more a format change costs.

## Index row

A layering that is only sound because the capability model makes it sound, recorded with the
condition that would undo it. The second argument, divergence from a vendored pin, is the kind of
cost that grows quietly, which is why it is written down rather than assumed.
