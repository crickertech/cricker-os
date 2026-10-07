---
status: REFUSED
raised: 2026-09-20
refused_by: 86, 448
---
# 490. CPU time in `time`

Refused by milestone 86 (design/roadmap/86-time-command.md), and recorded there
on 2026-09-03. Backfilled here on 2026-09-20 by
milestone 448 (design/roadmap/448-a-refusal-gets-a-number.md), which gave a refusal that names work a number, a
status and a condition that would change it. *(Number provisional until the merge queue lands it.)*

**The date is when the refusal was written down, not necessarily when it was made.** Most of this
tree's `- **Refused.**` bullets were written in one sweep on 2026-09-03, so the decision is usually
older than the bullet and its reasoning sits in the block's own prose above it.

## The refusal, in its own words

From '86. `time`: the shell times a command', under `## Follow-on`:

> CPU time. It is the scheduler's knowledge and nothing queries it today, and the number this
> command reports is deliberately wall clock between spawn and the exit arriving on the
> supervision endpoint. If CPU time ever arrives it is an extension of the same command rather
> than a rival to it.
>
> -- design/roadmap/86-time-command.md

## Why it is here rather than only there

`time` reports wall clock between spawn and the exit arriving on the supervision endpoint, and that
is a deliberate choice rather than a limitation of the shell. CPU time is the scheduler's knowledge
and nothing queries it today, so reporting it would mean adding a query before adding a column.

## Revisit

- **Condition.** The scheduler exposing CPU time to a query. The refusal is explicit that this is
  additive rather than competing: "If CPU time ever arrives it is an extension of the same command
  rather than a rival to it", so nothing about `time` has to be unwound to get there.

## Index row

A missing column in a command, refused because the number behind it is not exposed to anything that
could ask. The condition is a scheduler query, and the refusal records that the command's shape
would extend rather than change.
