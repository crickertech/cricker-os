---
status: REFUSED
raised: 2026-09-20
refused_by: 126, 448
---
# 455. `kill`, `pkill`, `skill` and `snice`: the signaling stratum of `procps`

Refused by milestone 126 (design/roadmap/0126-who-else-is-running.md), and
recorded there on 2026-09-03. Backfilled here on 2026-09-20 by
milestone 448 (design/roadmap/0448-a-refusal-gets-a-number.md), which gave a refusal that names work a number, a
status and a condition that would change it. *(Number provisional until the merge queue lands it.)*

**The date is when the refusal was written down, not necessarily when it was made.** Most of this
tree's `- **Refused.**` bullets were written in one sweep on 2026-09-03, so the decision is usually
older than the bullet and its reasoning sits in the block's own prose above it.

## The refusal, in its own words

From '126. The `procps` package: who else is running, and who is allowed to ask', under `##
Follow-on`:

> The signalling stratum (`kill`, `pkill`, `skill`, `snice`) stays unbuilt: a survey returns a
> tid, a tid is not a capability, and killing stays with whoever holds the child's region.
>
> -- design/roadmap/0126-who-else-is-running.md

## Why it is here rather than only there

`procps` shipped the survey half: a program can ask who else is running and get back thread ids. The
signaling half did not ship, and the reason is the capability model rather than the effort. A tid
is a number. Nothing in this kernel accepts a number as authority over a thread, and the right to
end one lives with whoever holds the child's region.

## Revisit

- **Condition.** A capability that names a thread's life. The refusal states exactly what is missing
  ("a tid is not a capability, and killing stays with whoever holds the child's region"), so either a
  verb on a thread handle or a supervision-domain right to end a child is what would make `kill`
  expressible at all. Both are syscall-surface questions, which is an architect's call rather than a
  lane's.

## Index row

A capability system cannot implement `kill` by handing a program a number, which is why the survey
half of `procps` shipped and the signaling half did not. The refusal names precisely the missing
primitive, so the condition is concrete even though meeting it is a syscall-surface decision.
