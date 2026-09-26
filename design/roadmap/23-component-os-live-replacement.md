---
status: PARTIAL
raised: 2026-07-25
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 23. A capability-routed component OS with live replacement

Every residual this block named is built. What keeps it PARTIAL is three lines under
Follow-on: the interactive stack is not swapped, the fallback for a dependent that will not answer is
proposed and not built, and what may be done to a component that never cooperates has no answer.
Two of the three wait on an architect as proposals; the third belongs to §32 (a supervisor may
collect a corpse without being able to build one).

## The idea

Every userspace component (driver, server, app) is a swappable, vendor-shippable unit behind a
stable contract, and operators replace them live, with no reboot. A client names an endpoint, never
a peer (§12 (call/reply IPC: a one-shot reply capability)), so a component's identity is invisible to the code that uses
it: any program that speaks the protocol and holds the right capabilities is the component. The
kernel is the one fixed thing underneath an entirely swappable userland. This is Fuchsia's shape
(capability-routed components, stable protocol interfaces) on a verified core.

It is also the product story. The kernel confines each component to what it was granted, so
competing vendor components run safely: a Linux vendor module is ring 0, and a nife vendor component
is a confined process that can touch only what the operator handed it. That is the payoff of the
capability model and of milestone 22 (trusted init: verify it, and shrink what a broken one can do), and it connects to the parked
competitor ambition ([competitor-question.md](../competitor-question.md)).

Two things are deliberately not swapped this way. The kernel is the verified base and changing it is
a reboot. A minimal root supervisor is the fixed point that makes swapping everything else possible;
you cannot swap the swapper infinitely.

Prior art: Fuchsia (the closest match), MINIX 3's reincarnation server (live driver replacement in
userspace), QNX (hot-swappable drivers), Erlang/OTP hot code loading and supervision.

## What is built

### Instance one: swap a console-shaped server under a talking client (2026-07-30)

DECISIONS §41 (the endpoint is the broker, and a device is revoked by taking it back),
notes/live-replacement.md. An unprivileged operator (`swapper`) builds the replacement, drains the
incumbent, takes the device back, and starts the replacement. A client (`chatty`) talks across the
swap and is its own witness; an attacker holding the client's exact capabilities cannot become the
server; the outgoing instance reads the UART after the revoke and faults at the device's own page;
and the replacement is written in C over §31 (the foreign-language seam: C holds no capabilities and makes no syscalls), so what held across the swap is the contract.

Three things the build settled. No forwarding process is needed as the broker: endpoint-only
naming makes the endpoint object the stable name, so a swap costs nothing in steady state and the
kernel's sender queue buffers the down window. The roadmap's step order (start the new server, then
revoke) does not survive contact, because revocation is by physical page; the endowment moves past
the revoke while the build stays first. And revoking a device means take-back, not destroy.

### The latency ladder, and an opt-in queue broker

`broker` is the middle rung: a queue server that buffers in its own budget while a backend is down,
so a producer never blocks on an absent consumer. It costs a second hop, priced by `broker_rtt`, so it
is opt-in per channel and passes through when both ends are up. The fast rung is the direct endpoint;
the slowest, a durable queue that writes to storage, is not built. The kernel stays synchronous
rendezvous; the queue is userspace policy bounded by the broker's own untyped.

### The component manifest (2026-08-17)

`crates/component_plan`, notes/component-manifest.md. `swapper` holds no endowment literals: each
contract declares its capability half in `swap_protocol`, and the operator says only which of its
own objects answers to which role name, per child. A manifest is a request and the provisions are the
authority (Fuchsia's `use`/`offer` split), so a vendor's declaration cannot widen its own authority.
The role name is the component's and the object is the supervisor's, which makes a component's peer
substitutable too. A manifest is still compiled in rather than shipped beside a binary; see
Follow-on.

### The hung component (2026-08-17)

notes/hung-component.md. Against an incumbent that stops answering without dying, three of the four
steps are unchanged: the one that needs its cooperation, the drain, is the one a hang makes
redundant. So a supervisor restores the service with no authority it did not already hold, which
corrects §32's sentence that restarting a hung child needs the stronger right; that sentence is right
about reclaiming its memory. The harder half is measured: the domain reports a hang as `BLOCKED`,
exactly what a healthy idle server reports; `Endpoint::REAP` answers `StillAlive` for every member;
and `abi::Error::Gone` never reaches a caller stranded mid-`CALL`, because freeing it needs the
cooperation whose absence is the hang.

### Dependency-aware orchestration (2026-08-23)

`component_plan::depends_on` and `dependents`, notes/dependency-orchestration.md. A component names
the contracts it cannot silently tolerate the absence of, and a supervisor asks the graph who to warn
before a swap. Only a component that forwards synchronously while serving others ever needs an entry:
a pure consumer degrades for free on the endpoint's sender queue, §41's mechanism doing a second job.

### State handoff (2026-09-26)

notes/state-handoff.md, to the ruling in DECISIONS §209 (state handoff is an opaque blob over a
granted frame, and it is optional). `Requirements` gained the optional `handoff` field (provisional,
as §209 asked), `swap_protocol` a stateful contract, and `swapper` a fourth role, `ROLE_HANDOFF`,
that tries the swap twice: against a replacement that cannot read the incumbent's blob, which
refuses, is reaped, and leaves the incumbent serving with its state; then against one that can,
which commits. The client's unchanged sequence check is the continuity witness, because the
component answers with its tally. It runs on all three architectures, the first swap system to,
since it has no device; the queued system lost the same inherited x86 skip.

Two findings. The handoff page cannot be deferred past a revoke like a device, because
`PageFrame::REVOKE` is symmetric and would take the operator's own capability, so it is mapped at
build in both instances and sequenced by the drain. And §209 needs a refusal it does not state: a
supervisor that routes no handoff page is refused a component that declares one, since wiring it
anyway is a kill-and-replace that looks like a successful swap.

§116 (live component state handoff is declined, for want of a customer), which §209 supersedes,
declined on 2026-08-23 because no stateful component existed. `redoxfs_server`'s first commit is
dated the next day.

### The unwarned dependent, measured (2026-09-26)

`ROLE_UNWARNED` swaps the queued system's backend without ever warning `broker`. The producer loses
nothing and only waits for the down window. So a dependent that will not answer its warning needs
nothing done to it; the supervisor only has to not block on it, which today it does, because the
warning is a `CALL`. notes/non-cooperative-fallback.md.

## Follow-on

- **Recorded.** A handoff page is one page; `redoxfs_server`, §209's own motivating customer, will
  not fit. `component_plan`'s `BUGS` and notes/state-handoff.md's.
- **Recorded.** A manifest is compiled in rather than shipped beside a binary. The ELF-note manifest
  work, milestone 597 (a program carries its manifest in an ELF note), carries a program's grant
  manifest rather than a component's declaration; notes/component-manifest.md's `BUGS` carries the history.
- **Milestone 106.** How a supervisor notices a hang is the timed wait,
  `design/roadmap/106-deadline-wait.md`, NOT-STARTED and behind milestone 263 (can a userspace process hold a timer, on all three architectures?).
- **Outstanding.** What may be done to a component that never cooperates has no answer.
  notes/hung-component.md's finding stands: the stronger right is not merely large but insufficient,
  since a permanently blocked thread never reaches the scheduler to spend the kill a destroy arms.
  A hang can also cost two unreclaimable regions, the component's and its stranded caller's.
  Checked 2026-09-26.
- **Proposed.** `design/roadmap/proposals/warn-a-dependent-without-blocking.md`, PROPOSED 2026-09-26.
  The fallback for a dependent that will not answer is not "the same open question one level out":
  measured above, it needs only a warning that never blocks. Recommended: signal a notification
  bound to the dependent plus a read-only state page, after milestone 151 (notification objects:
  async multiplexing without wait-any). Until then a hung `broker` hangs `swapper`, recorded at the
  `CALL` in `swapper.rs` and in notes/non-cooperative-fallback.md.
- **Outstanding.** `line_editor`, `display_terminal` and `compositor` are not swapped. The 2026-09-03
  line that stood here was half right: all three run under the kernel test harness, but milestone
  177 (wire the graphical terminal stack into the real interactive boot) wired two into a boot path,
  not three, and the kernel builds `display_terminal`, so no userspace supervisor could swap it. The
  compositor runs only in the tests of milestone 33 (a compositor: one screen, mutually distrusting
  clients) and gets nothing until some boot runs it. notes/interactive-stack-swap.md, checked
  2026-09-26.
- **Proposed.** `design/roadmap/proposals/swap-line-editor-live-under-system-initializer.md`,
  PROPOSED 2026-09-26, behind two terminal-contract forks for an architect: an additive quiesce
  opcode, and what a reader parked in `OP_READLINE` is told when a swap begins (recommended: a new
  "ask again" flag).
- **Milestone 600.** Milestone 600 (the graphical terminal stack is built in userspace), promoted from this block's
  proposal. `MAP_INTO` already maps a frame run, so the eleven-slot reason the kernel builds
  `display_terminal` looks expired; whether the gpu's DMA pages are one run is the question.

## Index row

the flagship payoff, and a product ambition
