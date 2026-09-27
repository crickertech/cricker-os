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

**State handoff was ruled on 2026-09-23 and is no longer declined**
([DECISIONS §209](../decisions/209-state-handoff-is-an-opaque-blob-and-it-is-optional.md), state handoff is an opaque blob over a granted frame, and it is optional).
The transport is the shape §116 (live component state handoff is declined, for want of a customer) sketched and did not commit to: an **opaque blob over a granted
shared `Frame`** for state, **`GRANT`** for capabilities. **Handoff is optional**, declared in the
component manifest beside `depends_on`; a component that declares nothing is kill-and-replaced, as
the console already is. **Failure does not commit**: an incoming instance that cannot absorb the
state leaves the swap incomplete, which is a revoke of the new grant under §16 (object revocation). The manifest
field's name is an architect's and is unratified.

The premise §116 declined on did not survive the week it was written in, and that is worth keeping
here: it said no component with meaningful live state existed or was being built, and
`redoxfs_server`'s first commit is dated 2026-08-24, one day later. `notes/fs-server.md` calls a
filesystem server with open handles this milestone's hardest state-handoff case. So the package
ruling that reopened this was the **second** customer, not the first. §209 (state handoff is an opaque blob over a granted frame, and it is optional) carries the full record.

**Building it is ordinary unblocked work.** All four residuals this block named now have their
decisions; none of the four is waiting on an architect.

**The component manifest is built** (2026-08-17, `crates/component_plan`,
notes/component-manifest.md), which is the piece milestone 39's packaging analysis leans on. **The
hung-component case is demonstrated** (2026-08-17, notes/hung-component.md), and it took a decision
*out* of this milestone rather than adding one: see the section below. **Dependency-aware
orchestration is built** (2026-08-23, `component_plan::depends_on` and `dependents`,
notes/dependency-orchestration.md): a component declares which contracts it cannot silently tolerate
the absence of, and a supervisor asks the graph who must be warned before a swap rather than hard-coding
it per system. The finding worth a sentence here: the field could not live on a capability need the way
the roadmap's own phrasing suggested, because a role's supplier is the supervisor's choice and varies
by wiring (§41's `use`/`offer` split, one level out), so it names contracts on `Requirements` instead,
and it turns out only a component that itself forwards synchronously while serving others ever needs
one populated: a pure consumer already degrades for free on the endpoint's own sender queue, which is
§41's central mechanism doing a second job nobody had asked of it yet.

**The status does not move.** Three of the four residuals this block named are built, and the fourth
is decided rather than built. The status line has tracked state handoff since it was declined, not a
residual count; as of 2026-09-23 it tracks an unbuilt ruling instead of an open question.

**In brief.** Every userspace component (driver, server, app) is a swappable, vendor-shippable unit behind a stable contract; operators replace them live, no reboot. The console hot-swap is instance one; a durable queue-broker decouples component lifecycles (opt-in per channel, for latency)

**Why it matters.** **the flagship payoff and a product ambition:** competing vendor components, confined by the kernel and swapped live; the verified core is the one fixed thing

**Status (2026-07-30): the mechanism is built and proven on both ISAs; the generalisations below are
not.** DECISIONS §41, notes/live-replacement.md. What landed: the four steps, an unprivileged
operator (`swapper`) that runs them, a client (`chatty`) that talks across the swap and is its own
witness, an attacker holding the client's exact capabilities that cannot become the server, a control
that must fail (the outgoing instance reads a UART register after the revoke and faults, at the
device's own page, with the kernel as the witness), and a replacement written in **C** over §31's
seam, so what held across the swap is the contract rather than a recompile. Both rungs of the ladder
that this milestone specified are built (`broker` is the opt-in one), and priced: `broker_rtt`.

**Three things the build settled, all in §41.** The block imagined a forwarding *process* as the
broker; it does not need one, because §12's endpoint-only naming already makes the endpoint object
the stable name, so the swap costs **zero** in steady state and the kernel's sender queue buffers the
down window. The block's step order (start the new server, then revoke) does not survive contact:
revocation is by physical page, so the endowment has to move to the far side of the revoke, though
the *build* does stay first. And revoking a **device** had to mean take-back rather than destroy,
which is the "deferred CDT finally earns its keep" this block predicted, at one level of the tree.

**The manifest landed 2026-08-17, and the defect it named is gone.** `swapper` no longer contains
an endowment: grep it for `abi::rights` or `abi::aspace::MAP_R` and there is nothing left. The
capability half of the contract lives in `swap_protocol` beside the wire half, which is where §41's own
sentence puts it (a program that speaks the protocol **and holds the right capabilities** is the
component), and the operator declares only which of *its own* objects answers to which role name, per
child. Three things fell out that were not obvious from this block. **A component manifest is a
sibling of `grant_plan::Manifest` and not a subtype**, because that one declares what a human at a
prompt may designate and this one declares what a supervisor must route, and the argument is in
notes/component-manifest.md. **A manifest is a request and the provisions are the authority** (the
Fuchsia `use`/`offer` split), which is what stops a vendor's declaration being a privilege-escalation
surface. And the role name is the *component's* while the object is the *supervisor's*, so one
declaration wired two ways makes a component's **peer** substitutable too: `chatty` asks to use
`service` and gets the shared endpoint on one channel and the queue broker's front endpoint on the
other, without noticing. What is **not** done is shipping a manifest as its own artifact rather than
compiling it in; the options and their costs are in `notes/component-manifest.md`'s `BUGS`, and that
note's own recommendation is to leave this decided-not-yet rather than raise it now: "decide it when
a second supervisor or an out-of-tree component actually exists, because until then the parsed
format would be a format with one producer and one consumer that are compiled together." Nothing
has shipped outside this tree, so the trigger (also `design/what-a-distribution-packages.md`'s: "a
component first built outside this tree, or a binary first distributed to someone who cannot rebuild
it") has not fired. Not a live question awaiting an architect's call; a correctly-deferred one.

**The hung component was demonstrated 2026-08-17, and it corrects a sentence this block and two
DECISIONS sections all repeat.** §32 declined the case with "a supervisor that must restart a *hung*
child still needs the stronger right", and §41 and notes/live-replacement.md both cite that. It is
right about **reclaiming the hung component's memory** and wrong about **restarting its service**,
and those are different acts. `swapper` grew a third role that runs the four steps against an
incumbent which swallows one request and stops answering, and three of the four are unchanged: the
one that needs the incumbent's cooperation (`OP_QUIESCE`) is the one a hang makes **redundant**,
because quiescing exists to make a component stop receiving and a hung component already has. The
device comes back from a live, wholly uncooperative holder by §41's `GRANT`-gated take-back, and the
replacement drains what queued behind the silence. So a service is restored with no authority the
operator did not already hold.

The harder half is now measured rather than asserted, and it is three findings. **The domain does not
report a hang**: `abi::endpoint::SURVEY` (milestone 126) says `BLOCKED`, which is byte for byte what a
healthy server parked in `RECV_CAP` says, and no death message ever arrives. **`Endpoint::REAP` is
refused for every member**, `StillAlive`, which is §32 working as designed and leaving a supervisor
with a verb and nothing to apply it to. And **`abi::Error::Gone` does not reach a caller stranded
mid-`CALL`**: it is woken by `ipc_reply` and by nothing else, the one-shot Reply capability naming it
is `WRITE` without `GRANT` inside the hung component's own cspace, so freeing it needs the cooperation
whose absence *is* the hang. Two decisions are named and not taken (how a supervisor notices, which is
milestone 106's timed wait; and what may be done to a component that never cooperates, where the
finding is that the stronger right is not merely large but **insufficient**, since a permanently
blocked thread never reaches `schedule()` to spend the kill a `DESTROY` arms). No watchdog program was
built, deliberately: both its halves are behind those decisions.

**What remains:** state handoff, now decided and not yet built (DECISIONS §209, state handoff is an opaque blob over a granted frame, and it is optional;
the component here is near-stateless, which is what makes kill-and-replace sufficient and why it
would declare no handoff at all). Dependency-aware
orchestration is built (2026-08-23) but the hung-component work already sharpens both residuals in
the same way, and for the graph the sharpening is now a recorded gap rather than a prediction: the
quiescence protocol orchestration needs (telling a dependent to degrade) is exactly the step a hang
makes unavailable, so `component_plan::dependents` can name *who* to warn but has nothing to say when
the named dependent is itself hung and cannot be warned. `broker` is not `Endpoint::REAP`-collectable
any more than the console component was; a non-cooperative fallback for a dependent, per
notes/hung-component.md's own account, is the same open decision (a timed wait, and what may be done
to a component that never cooperates) applied one level out, not new work. And a hung component
cannot be asked to serialise its state either, so a state-handoff protocol that needs the outgoing
instance's cooperation would recover a planned swap and not the failure it is most wanted for. Also
the console proper: the component swapped owns the real
UART and is shaped like a console server, but `line_editor`/`display_terminal`/`compositor` are not themselves swapped,
because the interactive stack is not running under the test harness.

**The destination the design points at, and a product ambition.** A client names an *endpoint*,
never a peer (the milestone 7-8 decision), so a component's identity is invisible to the code that
uses it: any program that speaks the protocol and holds the right capabilities *is* the component.
That decoupling is what makes running components replaceable at all, and it generalizes: the aim is
a system where **every userspace component (driver, server, app) is a swappable, vendor-shippable
unit behind a stable contract, and operators replace them live, no reboot** -- with the verified
kernel as the one fixed thing underneath an entirely swappable userland. This is Fuchsia's shape
(capability-routed components, stable protocol interfaces) on a verified core.

**Instance one: hot-swap the console server (the mechanism).** Replace a running server with a new
version, no reboot, with a client that never notices. Four steps, each on earlier machinery:

1. **Start the new server** (a supervisor builds it via the granular verbs, endows it fresh).
2. **Revoke the old server's device capability** so there are never two owners of one device's
   registers (the interleaving hazard): milestone 13's revocation extended from frames to *device*
   capabilities, where the deferred CDT (capability-derivation tree) finally earns its keep.
3. **Redirect clients through a broker.** Clients hold a cap to a stable *broker* endpoint, not to
   the server; the broker re-points on a swap, so substitution is invisible. A userspace naming
   service.
4. **Drain in-flight requests and tear the old server down** (the reaper plus revocation).

**The broker as a queue, and its latency (the concern that governs where this is used).** The
instance-one broker just re-points; the general form *buffers* -- a **durable queue server** that
holds messages in its own budget while a backend is down (crashed, restarting under supervision, or
being swapped), so a producer never blocks on an absent consumer and the new consumer drains the
backlog. This is the OS analogue of a distributed message queue (Kafka/RabbitMQ): a stable, always-up
broker decouples the *lifecycles* of the two ends, which is what makes crash-restart and live swap
seamless rather than merely possible. The kernel does not change -- it keeps synchronous rendezvous
(tiny, verified, no allocation); the queue is userspace policy, its buffer bounded by the server's
own untyped, so a runaway producer hits backpressure or a drop policy, never unbounded kernel memory.

Latency is the price, and it dictates where the queue is wired. Interposing a queue server turns one
rendezvous (one IPC, one switch, register transfer) into **two IPCs, two switches, and a copy**
through the server's buffer -- roughly a 2x IPC tax plus a scheduling hop. On a microkernel where
IPC is the hot path, that is not paid everywhere:

- **Opt-in per channel, never the default.** Direct synchronous rendezvous stays the fast path;
  queuing is chosen only for channels that cross a lifecycle boundary (components that restart or
  swap), where the decoupling is worth the tax.
- **Pass-through when both ends are up.** The broker buffers only during the down window; in steady
  state, with a live consumer waiting, it forwards directly, keeping the common case near direct IPC.
- **A latency ladder, not one point.** Fastest: a shared-memory ring buffer + async notification
  (the io_uring / virtio shape nife *already runs* for device I/O; the notification primitive
  is a generalisation of the endpoint's async-signal count) -- no middleman process, decouples in
  rate. Middle: a queue-server process -- decouples lifecycle, one extra hop. Slowest: a durable
  queue server that writes to storage -- survives its own crash. The rung is a per-channel choice.
- **Measure it, do not argue it.** Milestone 21's benchmark harness is the instrument: add a
  queued-IPC round trip beside the direct one, so the tax is a committed baseline number and a
  regression in it surfaces proximate to its cause.

Prior art for the queue itself: Mach ports (kernel message queues, macOS's foundation), Unix pipes,
POSIX/SysV message queues, and every distributed broker (Kafka, RabbitMQ, SQS); the shared-memory
ring variant is io_uring, DPDK, and virtio.

**Generalising to all components: what the console case does not yet need.**

- **A uniform component contract + manifest.** **Built 2026-08-17**, `crates/component_plan` and the
  four declarations in `crates/swap_protocol`. Each component declares the capabilities it needs (this
  device, these endpoints) and the supervisor wires it from the declaration, with a typed refusal
  before anything is built when it cannot. seL4 CapDL / Fuchsia territory as this block predicted, and
  Fuchsia's `use`/`offer` split turned out to be the load-bearing half. Still compiled in rather than
  shipped beside a binary: notes/component-manifest.md.
- **State handoff, ruled 2026-09-23 (DECISIONS §209, state handoff is an opaque blob over a granted frame, and it is optional; the account below is the superseded 2026-08-23 decline).** The console is easy because it
  is near-stateless. A filesystem server (open handles, caches, in-flight writes) or a network
  stack (live connections) cannot be kill-and-restarted without losing state, and live-swapping
  them would need moving that state from outgoing to incoming instance over a supervisor-brokered
  channel -- but no such component exists or is being built, so this is deferred rather than
  designed. §116 found the roadmap's own "serialise-old / absorb-new protocol" framing did not fit
  (every component's state is a different shape; there is no one wire format to specify) and
  records a transport shape as non-binding guidance for whoever eventually has a real component to
  swap: an opaque blob over a shared page for state, `GRANT` for capabilities. Prior art checked
  rather than cited uncritically: Erlang/OTP `code_change` is the closest match (opaque term in,
  opaque term out, generic mechanism); VM live migration and CRIU are memory-page-level and assume
  identical old/new layout, which does not hold across a version swap here.
- ~~**Dependency-aware orchestration.**~~ **Built 2026-08-23**, `component_plan::depends_on` and
  `dependents`, notes/dependency-orchestration.md. If B is a client of A, swapping A means quiesce B,
  swap, resume, and the graph is what decides *who* rather than a supervisor hard-coding it: `broker`
  declares it forwards synchronously to `backend`, and `queued()`'s `BOP_DOWN`/`BOP_UP` are now sent
  to whoever the graph names rather than unconditionally. Direct dependents only, on purpose: whether
  a dependent's own dependents need telling in turn is a property of that dependent's decoupling
  mechanism, not something two contract names can express. A fallback for when a node will not
  quiesce is still open, per notes/hung-component.md: the one step this graph asks of a dependent
  (degrade) is exactly the step a hang makes unavailable.
- ~~**The hung component.**~~ **Demonstrated 2026-08-17**, notes/hung-component.md, with the two
  decisions it cannot pass without stated there rather than in a chat message.

**The fixed core, stated honestly.** Two things are deliberately *not* hot-swapped this way, and
that boundary is a feature. The **kernel** is the verified TCB enforcing everything; you do not
live-swap it (changing it is a reboot; seamless kernel update is a separate, heavier problem). A
**minimal init / root supervisor / broker** is the fixed point that makes swapping everything else
possible -- pushed as tiny and stable as it can be, but you cannot swap the swapper infinitely.

**Why this is the selling point, and safe.** Because the kernel confines every component to exactly
the capabilities it was granted, **untrusted, competing vendor components run safely**: a Linux
vendor kernel module is ring-0 and can do anything; a nife vendor component is a confined
process that can touch only what the operator handed it. A malicious console driver scribbles on the
UART it was given and nothing else -- it cannot read another component's memory, forge authority, or
reach the kernel. That is what makes "different vendors ship competing components, operators swap
them live" not merely possible but *safe*, and it is the payoff of the capability model plus
milestone 22's authority-minimisation. It also connects directly to the parked competitor ambition
([competitor-question.md](../competitor-question.md)): this component model *is* a general-purpose product story, on the verified
core the demonstrator earns first.

**Prior art.** Fuchsia (the closest match: capability-routed, manifest-declared, swappable
components); MINIX 3's reincarnation server (live driver replacement in userspace); QNX
(hot-swappable drivers); Erlang/OTP hot code loading and supervision. The common thread is ours:
components are isolated processes, named through indirection and confined by capability, so one can
be swapped under the others.
## Follow-on

- **Decision.** Ruled 2026-09-23 in
  `design/decisions/209-state-handoff-is-an-opaque-blob-and-it-is-optional.md`: an opaque blob over a
  granted shared frame, optional per component, and a swap that does not commit on failure. It
  supersedes `design/decisions/116-state-handoff-declined.md`, whose want-of-a-customer premise had
  already expired when it was written.
- **Recorded.** A manifest is still compiled in rather than shipped beside a binary.
  `notes/component-manifest.md`'s `BUGS` carries the two candidate shapes and the reason to decide
  later, and nothing has shipped outside this tree.
- **Milestone 106.** How a supervisor notices a hang is the timed wait,
  `design/roadmap/106-deadline-wait.md`, NOT-STARTED and itself a kernel-surface fork for calef.
- **Outstanding.** What may be done to a component that never cooperates has no answer.
  `notes/hung-component.md`'s finding stands: the stronger right is not merely large but
  insufficient, since a permanently blocked thread never reaches the scheduler to spend the kill a
  destroy arms. Checked 2026-09-03.
- **Outstanding.** The non-cooperative fallback for a dependent that will not quiesce is the same
  open question one level out. The component plan names who to warn and has nothing to say when the
  named dependent is itself hung. Checked 2026-09-03.
- **Recorded.** `broker` is no more reapable than the console component was, and a gone error does
  not reach a caller stranded mid-call, because freeing the one-shot reply capability needs the
  cooperation whose absence is the hang.
- **Outstanding.** `line_editor`, `display_terminal` and `compositor` are still not themselves
  swapped by `swapper`, but the stated reason is no longer the true one: all three run under the
  kernel test harness today (`kernel/src/user/pipeline_tests.rs`,
  `kernel/src/user/display_tests.rs`, `kernel/src/user/compositor_tests.rs`) and milestone 177
  wired all three into a real boot path on 2026-08-27. What is missing is a swap role for them,
  not a harness. Checked 2026-09-03.

## Index row

the flagship payoff, and a product ambition
