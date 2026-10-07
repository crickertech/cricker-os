---
status: SUPERSEDED
raised: 2026-08-23
superseded_by: 209
---

# 116. Live component state handoff is declined, for want of a customer

(2026-09-23: reopened and decided. A customer existed one day after
this was written, and §209 (state handoff is an opaque blob over a granted frame, and it is optional) rules the transport this
section sketched. The residual this closes is milestone 23 (a capability-routed component OS with live replacement)'s. Kept as written, because the framing check below outlives the decline.)

Status when written: DECIDED. calef, 2026-08-23, on milestone 23's remaining residual: *"Since there is no
customer, maybe we should defer implementation."*

## The question

Milestone 23's console hot-swap (DECISIONS §41) is built and proven, but the console is
near-stateless. A filesystem server or a network stack cannot be kill-and-replaced without losing
open handles, caches, or live connections; live-swapping one needs some way to move that state from
the outgoing instance to the incoming one. The roadmap block calls this "the real engineering" and
names it as a wire format, and so calef's.

## The decision

**Declined for now.** No component with meaningful live state is built or being built, so there is
nothing this would unblock today. Same shape as `std::thread::spawn` (DECISIONS §105) and hard
links (DECISIONS §110): revisit when a customer needs it, not before.

## What the framing check found, worth keeping even though nothing is being built

The roadmap's own words ("a serialise-old / absorb-new protocol") suggested one wire format to
design. That framing does not fit: every component's live state is different in shape (a
filesystem server's open handles look nothing like a network stack's connection table), so there is
no single byte layout to specify. What a future decision would actually settle is narrower: the
transport mechanism state moves over, not its content.

Checked rather than designed, for whoever revisits this:

- This tree's own analogous pattern. Every existing case of moving data between two processes
  without going through IPC's word-limited messages already uses the same shape: a granted shared
  `Frame` (the clock page, DECISIONS §111's env-config page, `block_roster`, `system_initializer`).
  Capabilities already move between processes via `GRANT`/`CAP_INSERT`, which the component
  manifest's wiring already uses.
- The closest prior art, checked rather than recalled. Erlang/OTP's `code_change` is the same
  shape: the VM does not serialize a module's state, it calls a function the module's own author
  wrote, opaque term in and opaque term out. The mechanism is generic; the content is the
  component's own business. CRIU and VM live migration are memory-page-level and assume identical
  layout between old and new, which does not hold here (a swap can cross versions, and can be to a
  different implementation entirely, over §31's C seam) -- worth ruling out explicitly rather than
  citing uncritically, since the roadmap block's own prior-art list named them without this caveat.
- Nothing today needs this. `notes/fs-server.md`'s only mention of hot-swap is a passing
  observation that endpoint-only naming makes it possible in principle, not a live requirement.

If it is picked up later: an opaque blob over a shared page for state, `GRANT` for capabilities,
brokered through `swap_proto`'s existing `OP_QUIESCE` handshake, is the shape this note recommends
starting from. **This is guidance, not a commitment** -- the decision here is to decline, and
whoever eventually has a real component to swap should re-check this reasoning against what that
component actually needs rather than build to a description with no customer to correct it.

## What this does not decide

Dependency-aware orchestration, milestone 23's other named residual, needs no decision (only a
manifest extension so a component can declare what it needs, which nothing here touches) and stays
open as ordinary unblocked work.

## What it unblocks

Nothing directly; it closes an open question so milestone 23's status can say plainly that the
remaining residual is declined-for-now rather than pending.
