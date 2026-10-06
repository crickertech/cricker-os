---
status: NOT-STARTED
raised: 2026-09-27
promoted_from: the-spawn-service-runs-outside-boots-frame
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 696. The spawn service runs outside `boot`'s frame

Promoted from `design/roadmap/proposals/the-spawn-service-runs-outside-boots-frame.md` on 2026-10-03 (UTC). The number 696 was minted by the maintainer in a batch promotion of the proposal pile and is provisional until the queue lands it. *(Title and slug are drafts.)*

Raised by lane `milestone/progenitor-stack`, which measured the progenitor's stack for the first time
(notes/stack/progenitor-stack.md) and raised it from eight pages to twelve rather than do this,
because four open lanes were editing `system_initializer::boot` and `spawn_service` that day.

## The finding

`system_initializer::boot` builds the whole interactive system and then calls `spawn_service`, which
never returns. So every construction local `boot` has stays on the stack for the life of the
machine, under every request the service handles. Measured on aarch64:

- `boot`'s frame is **12,848 bytes** in debug and 3,200 in release. Unoptimised code gives each local
  its own slot, and `boot` is 1,460 lines.
- It is most of why debug peaks at twice release (32,440 bytes against 16,432 at `package
  install`). Without `boot`'s frame, debug's peak would be about 19.6 KB against release's 13.2 KB.

## What to build

`boot` returns what the spawn service needs (its channels, the filesystem handles, the job pool, the
catalog), and `_start` calls `spawn_service` with it. The construction locals die with `boot`'s
frame. Expected saving is about 12.5 KB in debug and 3 KB in release, which the gauge in
`kernel::progenitor_stack` will measure rather than this estimate.

Then decide, with the new number, whether `INIT_STACK_PAGES` goes back to eight. The gate
(`HEADROOM_FLOOR`) stays either way.

## Why it waits

It restructures the two functions milestones 205, 595 and 23 were editing on 2026-09-27. Wait for
those to land, then it is one lane with no collision.

## Index row

`system_initializer::boot` never returns, so its 12,848-byte debug frame stays on the progenitor's stack under every request. Proposed: run the spawn service outside `boot`'s frame.
