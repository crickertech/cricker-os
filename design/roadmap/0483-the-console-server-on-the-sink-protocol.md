---
status: REFUSED
raised: 2026-09-20
refused_by: 50, 448
---
# 483. The console server on the sink protocol

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

> Converting the console server to the sink protocol. `line_editor` is its only client and now
> speaks for two writers, so once the terminal adapter existed the page-plus-ack channel looked
> like the right answer rather than a gap; a second client of the console would hit the same
> one-page wall one layer down with nothing gained. `notes/sink-protocol.md` has the reasoning.
>
> -- design/roadmap/50-pipes-and-redirection.md

## Why it is here rather than only there

`line_editor` is the console server's only client and now speaks for two writers, so once the
terminal adapter existed the page-plus-ack channel looked like the right answer rather than a gap.
Converting the console server to the sink protocol would move a second client's problem one layer
down without solving it: it would hit the same one-page wall with nothing gained.

## Revisit

- **Condition.** A second client of the console server. The refusal's reasoning is entirely about
  there being one, and `notes/sink-protocol.md` carries the argument, so a second client is both what
  would reopen this and what would decide it.

## Index row

Two protocols for getting bytes out of a program, and a deliberate decision not to unify them while
one of them has a single client. The condition is a second client, which the refusal names as the
thing that would change both the answer and the reasoning.
