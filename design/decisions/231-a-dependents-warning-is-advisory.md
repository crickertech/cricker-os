---
status: DECIDED
raised: 2026-09-26
decided: 2026-09-26
ratified_by: calef
---

# 231. A swap's warning to a dependent is advisory, and the supervisor never waits for it

*Section number provisional until the merge queue lands it. §221 (the boot prompt is the owner's console) is on `main`
and §222 to §230 are claimed by open pull requests (#1350, #1357, #1359, #1361, #1368), so this took the next free
number on 2026-09-26 and may move at merge. The file name is a maintainer's coinage and provisional
too.*

Raised 2026-09-26 by the lane for milestone 23 (a capability-routed component OS with live replacement)
([block](../roadmap/23-component-os-live-replacement.md)). The options, the premise check, the prior
art and the costs are in two files that are not on `main` yet, so they are linked on the lane's
branch rather than relatively:
[`notes/non-cooperative-fallback.md`](https://github.com/crickertech/nife/blob/milestone/23-line-editor-swap/notes/non-cooperative-fallback.md) and
[`design/roadmap/proposals/warn-a-dependent-without-blocking.md`](https://github.com/crickertech/nife/blob/milestone/23-line-editor-swap/design/roadmap/proposals/warn-a-dependent-without-blocking.md).
Both sit on branch `milestone/23-line-editor-swap`, stacked on #1342, where both also land. This
section records the ruling and does not restate them.

## The ruling

calef, 2026-09-26 (UTC): *"Make the warning advisory."* Recorded by the maintainer at 18:22Z the
same day; that is the time of recording, not of the ruling.

- The supervisor writes the wanted state (down or up) on a read-only page it shares with the
  dependent, and signals a notification bound to the dependent's thread.
- It never waits for an answer. The swap proceeds whether or not the dependent has looked.
- `broker`'s `BOP_DOWN` and `BOP_UP` stop being `CALL`s.

The page carries the state and the signal only says "look", because §101 (notification objects:
async multiplexing without wait-any) has no badges yet, so one signal cannot tell down from up.

## Why the warning can be advisory

It was measured, not argued. `swapper`'s `ROLE_UNWARNED` swaps a backend without ever warning
`broker`, and the producer loses nothing: every request answered, in order, and none refused. An
unwarned dependent's forwarded call parks on the swapped endpoint's queue for the down window, and
the replacement drains it. That is §41 (the endpoint is the broker, and a device is revoked by
taking it back) doing for a dependent what it already does for a consumer. Skipping the warning
costs latency bounded by the down window, and no work.

## Refused

- Keeping the blocking `CALL` and bounding it with a timeout. It needs milestone 106 (a wait that
  ends on either the interrupt or the deadline) to put a deadline on `CALL`. It stalls every swap
  for the timeout whenever a dependent is slow. And the timeout is a magic number that nothing in
  the measurement can choose.
- The note's other refusals, for the reasons it gives: replacing the dependent as if hung, a
  supervisor right to force it, and a helper process that blocks in the supervisor's place.

## What it settles, and what it does not

It closes the stranded-supervisor defect: a dependent that never answers no longer hangs the
operator. The build waits on milestone 151 (notification objects: async multiplexing without
wait-any), which is #1351. What may be done to a component that never cooperates stays with §32 (a
supervisor may collect a corpse without being able to build one); this ruling shows the dependent
case does not need it.

Reversibility. `BOP_DOWN` and `BOP_UP` are spoken by two programs in the swap suite and nothing
else, so retiring them as `CALL` opcodes is cheap until a second forwarding dependent exists.
