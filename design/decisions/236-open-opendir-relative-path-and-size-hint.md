---
status: DECIDED
raised: 2026-09-27
decided: 2026-09-27
ratified_by: calef
---

# 236. `OPEN`/`OPENDIR` take a relative path, and `OPEN`'s reply carries the size

*Section number provisional until the merge queue lands it.*

Raised on pull request #1387, milestone 606 (a directory walk costs what it does on Linux), which
measured nife's walk at 1.9x Linux's cost and prototyped three wire changes, none built, asking
which if any to take. calef ruled two of the three, in two separate comments the same session, and
they are recorded together here because both are built in the one PR that followed, #1406 (merged).

## The ruling

### A, at 2026-09-27T07:04Z

`OPEN` and `OPENDIR` accept a relative path, resolved one step at a time on the server exactly as a
hop-by-hop walk would be, each step needing `DESCEND`. calef:
*"Nobody is running nife yet. The cost of changes are still at the floor. A for sure."*

### B, form 2, at 2026-09-27T07:06Z

`OPEN`'s reply carries the file's size in its previously unused second word.
- The std overlay patches `std::fs`'s buffer sizing to ask the PAL for that size as a hint.
- `metadata()` stays honest and always asks the server.
- A stale hint costs at most one buffer resize.

Full semantics are documented on `filesystem_protocol::fs::OPEN` and `fs::OPENDIR` on `main`,
including the refusal an old server gives (`EINVAL` on a `/` in the name, which is how a client
learns to fall back to hop-by-hop). They also cover what "0 in the reply" means: not given, so ask
with `FSTAT`.

## What was measured

Release kernel, HVF, three `--smp` boots each, same session, 2026-09-27 (from #1387 and #1406):

| | walk | vs Linux 0.41-0.42 ms |
|---|---:|---:|
| before (#1387's own work: memo/cache, memcpy fix) | 0.785-0.795 ms | 1.9x |
| + A | 0.58-0.61 ms | 1.4x |
| + A + B | 0.515-0.53 ms | 1.26x |

Requests per walk fell from 1,259 to 692 (`walk_model`).

## Refused, or left open here

A third option, D (server-side narrowing that skips the caretaker entirely, measured at 0.35 ms
with A and B), was not ruled on in this comment. calef ruled it separately, on per-filesystem
terms; that ruling is §237 (subtree grants may skip the caretaker, per filesystem).

## Reversibility

Both changes are additive to the wire protocol: an old server refuses a path in `OPEN`/`OPENDIR`
loudly rather than misbehaving, and an old client ignores the size word in `OPEN`'s reply. Nothing
outside this tree has acted on either format yet, which is why calef weighed the decision as cheap
("the cost of changes are still at the floor").
