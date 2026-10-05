---
status: AMENDED
raised: 2026-08-23
decided: 2026-08-23
ratified_by: calef
---

# 120. A QEMU-only virtio-rng stopgap for the interactive boot

2026-08-26. Originally declined, calef, 2026-08-23: *"Mint the TRNG milestone,
defer the stopgap for now."* Reversed, calef, 2026-08-26, on being asked directly whether he was
the customer this decision was waiting on: *"I want to be able to login on the QEMU implementation,
so it seems like we have a customer."* Sharpened when the milestone-159 alternative was raised and
did not actually answer what he wanted: *"My thinking is that as we build we are going to want to
continue to work on things in QEMU past the login because not being gated on a human taking action
as part of the development cycle means we can go faster. You can't login to a board without me. You
can login to a QEMU hosted implementation."* See "The amendment" below; the original reasoning is
kept in full above it, unedited, because it was correct on the premise it had.

## The question

Milestone 49's boot-wiring lane traced "not wired into the interactive boot" down to a real
blocker: `login` needs `credentialer`, which refuses to start without the entropy service
(DECISIONS §42, no silent degradation), which needs a `Virtio` capability onto a virtio-rng
device. Neither aarch64's `spawn_init` nor riscv64's `riscv_shell_boot` grants one, and the
interactive boot's QEMU invocation does not attach the device either -- a deliberate, existing
minimal-device-surface choice (`NIFE_RNG`, like `NIFE_GPU`/`NIFE_KBD`/`NIFE_NVME`, is test-leg
only). The fork the lane surfaced rather than decided: add virtio-rng to the interactive boot now,
as a QEMU-only stopgap, or wait.

## The decision

Declined for now. No customer needs interactive login working before real hardware entropy is
sorted. Same shape as `std::thread::spawn` (§105), hard links (§110), state handoff (§116), and
`OutOfMemory`'s cause collapse (§119): revisit when a customer needs it, not before.

## Why, beyond just deferring

**This was never really "add entropy or not," it was two separable questions wearing one fork.**
Checked before accepting the lane's framing: the JH7110's TRNG is already named as the real
hardware answer in two places (`notes/entropy.md`, milestone 56's own doc), verified nowhere,
tracked by no milestone -- the same shape of gap milestones 53 (board network/storage) and 157
(board display) already carry: virtio only exists in QEMU, real silicon needs its own driver.
Milestone 159, a real hardware entropy source (the JH7110's TRNG, minted alongside this decision),
tracks the real answer. This decision covers only
the second, separable question: whether to grant a QEMU-only stand-in *before* that real answer
lands.

**The lane's own cost accounting holds up and is the real reason to wait, not just "no customer."**
Granting virtio-rng in the interactive QEMU boot is cheap by itself (one flag, matching the three
that already exist for test legs), but it is coupled to two expensive things: a new permanent
kernel-to-init capability grant on a `BootEndowment` cspace already documented as one slot from the
wall at peak (restructuring which capabilities init holds simultaneously, not adding a field), and
a claim about what hardware the interactive boot represents that milestone 55's real Pi-class
target contradicts if answered the QEMU-only way. Per the *move fast on what can be undone* tenet,
a new capability two programs (kernel and init) must agree on is exactly the expensive category,
and there is no customer forcing the question yet.

## What this does not decide

Milestone 159's real-hardware TRNG driver is not blocked by this decision either way, and building
it does not require revisiting this fork -- the two were only coupled by both routing through
`entropy_service`, not by any shared mechanism.

## The amendment, 2026-08-26: reversed

Grant the QEMU-only virtio-rng stopgap. The premise the original decision turned on, "no
customer needs interactive login working before real hardware entropy is sorted," no longer holds:
calef is that customer, for a reason specific to this project's own method rather than a change of
mind. Milestone 159 was raised as the alternative and does not substitute: it gives real login on
the real RISC-V board, not under QEMU, and a board needs calef's own hands to reach (`Gate:
HARDWARE`, notes/visionfive2.md), where a lane or an agent can drive a QEMU boot unattended. His own
words are the reasoning, not a gloss on it: every future milestone that needs a logged-in session to
test past is otherwise gated on a human being available, which is exactly the bottleneck this
project's whole method (AGENTS.md, "The method is a result") exists to avoid.

**The two costs the original decision named are still real and were checked again, not waved past,
before this reversal.** `crates/system_initializer`'s own doc comment (2026-08-17, unchanged since)
still says init's capability table is nine at rest and fifteen at peak, one slot from the wall of
sixteen; fitting a new permanent virtio-rng grant is still a restructuring, not a field addition, and
the honest candidates it names for buying the slot (the readiness endpoint retyped later, or the
file page's second frame retired) are unchanged. That is real work, owed to whoever builds this, and
this amendment does not do it. The second cost, that granting it answers "what does the interactive
boot represent" as the demonstrator's own boot rather than milestone 55's real Pi-class target, is
the claim calef is the right person to make and has now made: the interactive boot is the
demonstrator's, and a real customer target with no virtio-rng is a fact for whoever eventually ships
against real hardware, not a reason to block development velocity today.

## What it unblocks

Milestone 49's login-boot-wiring piece (its own BUGS section can move from a recorded DECLINED to a
build), and by extension [journey 1](../journeys/01-login-to-kilo.md)'s step 4.
Milestone 159 is unaffected either way, exactly as "what this does not decide" already said.
