---
status: DECIDED
raised: 2026-09-02
decided: 2026-09-02
ratified_by: calef
---

# 138. How a saturated workload is made to hand threads across cores

Answered by calef on 2026-09-02, on the one question that was blocking.
Written by a research lane
on 2026-09-01, after milestone 219 (the boot tour ends and the kernel halts, so there is nothing to
soak) measured that it never happens today.

calef approved option D, the `--features soak` hook that signals a routed rendezvous from
`sched::on_tick`, so soak workers block on it through the `Irq::WAIT` that already exists. It adds
no syscall and nothing of it exists in a production build. That unblocks the second half of fatal
risk 5's decisive experiment, which had never been runnable.

Two things stay open on purpose and neither is blocking.

**Option A, thread affinity, is not refused; it is not yet asked.** It is syscall surface, so this
section gives options rather than a recommendation, and it should be taken on its own merits rather
than carried in on the back of an experiment D can run without it. What the prior art narrowed:
the core belongs in a capability rather than in a message, and DECISIONS §28 already chose the shape
at ratification. What is still calef's: singleton per core or narrowable set, and whether it is a
new object or a field of one that exists.

Option B, a periodic rebalancer, is declined rather than dismissed, and the Linux sourcing
weakened the case against it. nife already has counterparts for three of Linux v6.12's four
balancing moments, all event-driven, and a saturated rendezvous workload starves all three of their
triggers, which is why the crossing count freezes. B is the fourth moment, the only one that fires
on a clock and the only one that workload cannot starve, so calling it "importing Linux's answer"
overstated what it is. The refusal now rests on DECISIONS §28's own reopening trigger, a real
workload where fairness visibly fails, which has not fired. A reader who believes it has should
reopen B, and that sentence is here so nobody has to relitigate the whole section to do it.

**The number is provisional.** This lane cannot see the other lanes running beside it, so the
integrator mints the real section number at merge, the same as every other name global to the tree.

## What is being decided

`design/fatal-risks.md`'s fifth entry, *it cannot be made reliable on multicore, and the bugs appear
only on silicon*, names its decisive experiment as sustained multi-core stress on the boards. Milestone
219 built that stress and found the experiment is two experiments, of which it delivers one:

- Contention on shared kernel state at rate. Four harts entering `IPC_TABLES` tens of thousands
  of times a second. The soak sustains this.
- Cross-core handoff under load. Threads actually moving between cores while the machine is
  busy. This is where the risk's only observed defect lived: on radon, `sched::wake_load_aware` made
  a receiver `Ready` without a delivery, on three harts, found at a bench rather than by any test.
  **The soak does not sustain this, and cannot.**

So: what mechanism gets threads crossing cores under load, and what does the tree pay for it
permanently?

## The premise, verified by this lane rather than taken from the milestone

Confirmed on patagonia, 2026-09-01, `script/soak --for 40s` on aarch64, four cores, twenty workers:

```
soak: t=5s  beat=1 rounds=320605  rate=64121/s ... crossings=15 remote=23 steals=2
soak: t=35s beat=7 rounds=2182566 rate=63512/s ... crossings=15 remote=23 steals=2
```

`crossings` is frozen at 15 across seven beats and 2.18 million IPC round trips. It is not slow, it
is stopped. Milestone 219's riscv64 row (21 crossings) and its explanation both survive
re-measurement, and its three structural causes are each visible in the code:

1. A rendezvous wake is local by design. `sched::wake` queues the woken peer on the *waker's* own
   core (§28, part 2: the message is in registers and the cache is warm). A communicating set
   converges onto one core within a few exchanges and stays.
2. `wake_load_aware` has exactly one caller, `sched::irq_notify`, which is called from the three
   architectures' interrupt dispatchers and from nowhere else. `git grep wake_load_aware` is the
   whole proof.
3. A work steal needs an idle core and a queued thread elsewhere. A rendezvous keeps at most two
   threads runnable per group, so queues are empty; add grinders to fill them and no core is idle.
   A steady state holds neither end.

Nothing dissolves the question. This lane looked for an existing way to make a saturated workload
cross cores and found none that a user program can reach. That is the finding that makes this
decision necessary rather than optional.

## What this tree already does in the analogous case

Four greps, and they move the argument more than any of the options do.

§28 already deferred this exact thing, with a trigger. Its deferred list reads: *"an explicit
placement grant in the spawn manifest (milestone 23's contract; overrides the default, recovering
seL4's userspace-owns-placement story for pinned components)"*. So affinity is not a new idea here.
It was considered at ratification, deferred deliberately, and its shape was already chosen: a
grant in the spawn manifest, not a syscall on a running thread. Any proposal that reaches for a
`Thread::SET_AFFINITY` method is reopening a decision rather than making one, and owes a reason.

This tree already has affinity, and it is kernel-internal. Device-line IRQ affinity picks which
hart's PLIC context a source lands on (`kernel/src/drivers/plic.rs`, `kernel/src/arch/riscv64/irq.rs`).
It is policy the kernel decides and remembers; no program asks for it and no capability names it.
That is the precedent for "affinity exists here" and it is on the opposite side of the syscall
boundary from option A.

This tree has already refused to pin threads once, on instrument grounds. `kernel/src/cpu.rs`,
in the per-CPU test: *"The alternative was giving kernel tests boot-core affinity, and it is the
wrong trade. Pinning tests to core 0 would buy this one assertion back at the cost of running the
whole suite on one core, which is precisely where the placement bugs §28 introduced would hide. A
harness that avoids the scheduler it is meant to test is not a harness."* The reasoning transfers
with the sign flipped: a *pinned* soak would be running a workload the scheduler is not free to
place, which is one interleaving explored very thoroughly.

A feature-gated kernel affordance built only to make an experiment possible is established
practice. `kernel/Cargo.toml` carries `fastpath_pad`, which adds a large reachable-but-never-taken
function to the IPC fast path so milestone 134's footprint experiment has something to compare, and
`watchdog_probe`, which adds a test that deliberately livelocks and is expected to fail the run.
Neither ships. `soak` itself is the third. So an affordance that exists only under `--features soak`
crosses no line this tree has not already crossed on purpose, and milestone 219 established the
hygiene that goes with it: `script/lint` clippies the feature, and `script/fastpath-footprint` proves
the production binary is byte-identical.

## The prior art, read rather than recalled

Every quote below was fetched and read, and every one was re-checked against the downloaded source
before it went in this file: the seL4, Barrelfish, L4Re and QNX material on 2026-09-01, the Linux
material on 2026-09-02. The seL4 manual quotes are from the PDF's extracted text (the ligature in
"affinity" is why a naive grep misses them); the source quotes are from `raw.githubusercontent.com`.
Every Linux claim in this document names the version it was checked against, because a function
name in that tree is a fact with an expiry date and three of the ones in this argument expired in
v6.10.

### seL4 does not migrate threads at all, and says why

This is the finding that reframes the whole question, because nife's measured behavior is seL4's
deliberate design. From Elphinstone, Zarrabi, Danis, Shen and Heiser, *An Evaluation of
Coarse-Grained Locking for Multicore Microkernels*, https://arxiv.org/pdf/1609.08372, section 3.3:

> This partitioned scheduling implies that threads can only migrate between cores if explicitly
> requested by the user, which is consistent with seL4's general philosophy of having all resource
> management under user control (and also helps reasoning about real-time properties).

Corroborated in the kernel source: `chooseThread` in `src/kernel/thread.c` reads only
`NODE_STATE(ksReadyQueues)`, this core's own queues, and an idle core runs its idle thread rather
than pulling from a neighbor. There is no stealing path and no rebalancer. seL4 does have one
implicit migration, and it follows authority rather than load: the manual's section 6.1.11 says
*"Passive threads will run on the CPU node that the scheduling context was configured with, and will
be migrated on IPC"*, because a scheduling context is donated over `seL4_Call`, so a passive server
runs on its caller's core.

### seL4 has two answers on affinity, and the newer one makes the core a field of a capability

Non-MCS. The manual, section 6.1.2: *"the default thread affinity is the node the thread's TCB
object was created on, and `seL4_TCB_SetAffinity()` can be used to explicitly set the affinity. On
MCS configurations, the affinity is derived from the scheduling context object."* The authority is
the TCB capability and nothing else: `decodeSetAffinity` in `src/object/tcb.c` checks only that the
core number is in range. That is an asymmetry seL4 itself later fixed, because the same kernel
gates *priority* with a second capability (the manual, section 6.1.6: a thread changing another's
priority *"must provide a thread capability from which to use the MCP from"*) and gates *placement*
with nothing.

MCS, and this is the precedent worth having. The manual, section 6.1.10: *"To populate a
scheduling context with parameters, one must invoke the appropriate SchedControl capability, which
provides access to CPU time management on a single node. A scheduling control cap for each node is
provided to the initial task at run time. Threads run on the node that their scheduling context is
configured for."* Under MCS, `seL4_TCB_SetAffinity` does not exist at all; it is compiled out.

The capability's whole content is the core. `include/object/structures_64.bf`, verified by fetching
it:

```
block sched_control_cap {
    field core     64

    field capType  5
    padding        59
}
```

and `src/object/schedcontrol.c` reads the core off the capability rather than off the message
(`cap_sched_control_cap_get_core(cap)`). So there is no message that can name a core. You choose
a core by choosing which capability you invoke, delegating "may run on core 3" is a copy of one
capability, and revoking it is a revoke. That is rung one of `AGENTS.md`'s ladder: the wrong state
is unrepresentable rather than checked.

### L4Re shows the set-shaped version of the same idea, narrowable at mint time

`L4::Scheduler` is a kernel object, and https://l4re.org/doc/classL4_1_1Scheduler.html describes
`run_thread` as launching *"a thread on a CPU determined by the scheduling parameter `sp.affinity`"*.
One invocation carries a CPU set, a priority and a timeslice together. The authority is possession
of the Scheduler capability, and Moe's documentation
(https://l4re.org/doc/l4re_servers_moe.html) shows it being minted narrower: a scheduler proxy is
created with a priority limit, a priority offset and a CPU bitmap, and *"`L4::Scheduler` objects can
be only created through the user factory provided by Moe to the initial application. Other factory
instances cannot create this object."* Hold, narrow, delegate, in the tree's own vocabulary.

Whether Fiasco's kernel ever rebalances on its own was **not established**; the documentation
describes migration only as an explicit act and no statement either way was found. Recorded as
unverified rather than guessed.

### Barrelfish dissolves the authority question by making placement structural

*Barrelfish Architecture Overview*, TN-000, https://barrelfish.org/publications/TN-000-Overview.pdf:
*"Dispatchers do not migrate between cores."* A domain that wants to run on a second core creates a
second dispatcher there, and thread migration is a userspace protocol between the two dispatchers'
schedulers. There is no affinity call because there is nothing to set, and the right to run on a
core is the right to create a dispatcher on it.

### Linux balances continuously, at four moments, because nothing ever tells it anything

Version, because "Linux does X" without one rots. Everything below is Linux v6.12
(`Makefile`: `VERSION = 6`, `PATCHLEVEL = 12`), fetched from
`raw.githubusercontent.com/torvalds/linux/v6.12` on 2026-09-02 and read. Three of the names a
reader may have in their head were changed in v6.10 and are wrong for any tree newer than that:
`rebalance_domains` became `sched_balance_domains`, `newidle_balance` became
`sched_balance_newidle`, and `scheduler_tick` became `sched_tick`. All three carry the old names in
v6.9 and the new ones in v6.10, checked in both. The rot is not hypothetical either: the comment
above `sched_balance_softirq` in v6.12 still says `scheduler_tick()`, two releases after that
function was renamed.

**EEVDF did not change this.** `Documentation/scheduler/sched-eevdf.rst`: *"The Linux kernel began
transitioning to EEVDF in version 6.6 (as a new option in 2024), moving away from the earlier
Completely Fair Scheduler (CFS)"*. Every function named below is still in `kernel/sched/fair.c` in
v6.12 under `fair_sched_class`. That EEVDF replaced the *pick* rather than the *balancer* is an
inference from the file rather than a quoted claim, and is marked as one.

1. Periodically, off the tick. `sched_tick()` in `kernel/sched/core.c` ends, under
`CONFIG_SMP`, with `rq->idle_balance = idle_cpu(cpu); sched_balance_trigger(rq);`, and
`sched_balance_trigger` in `kernel/sched/fair.c` raises the softirq only when the deadline has
arrived (`if (time_after_eq(jiffies, rq->next_balance)) raise_softirq(SCHED_SOFTIRQ);`).
`Documentation/scheduler/sched-domains.rst` states the whole chain:

> In kernel/sched/core.c, sched_balance_trigger() is run periodically on each CPU through
> sched_tick(). It raises a softirq after the next regularly scheduled rebalancing event for the
> current runqueue has arrived. The actual load balancing workhorse,
> sched_balance_softirq()->sched_balance_domains(), is then run in softirq context (SCHED_SOFTIRQ).

2. When a CPU is about to go idle. The comment on `sched_balance_newidle` in
`kernel/sched/fair.c`: *"sched_balance_newidle is called by schedule() if this_cpu is about to
become idle. Attempts to pull tasks from other CPUs."* Its two callers are `balance_fair` and the
`idle:` arm of `pick_next_task_fair`, both in `kernel/sched/fair.c`.

3. On behalf of a tickless idle CPU, by a different CPU. A CPU with its tick stopped cannot
reach moment 1, so someone else reaches it for them. The comment on `sched_balance_softirq`:

> This softirq handler is triggered via SCHED_SOFTIRQ from two places:
>
> - directly from the local scheduler_tick() for periodic load balancing
>
> - indirectly from a remote scheduler_tick() for NOHZ idle balancing through the SMP cross-call
>   nohz_csd_func()

and inside it, *"If this CPU has a pending NOHZ_BALANCE_KICK, then do the balancing on behalf of
the other idle CPUs whose ticks are stopped."*

4. At wake-up, fork and exec. `select_task_rq_fair`'s comment: *"Select target runqueue for the
waking task in domains that have the relevant SD flag set. In practice, this is SD_BALANCE_WAKE,
SD_BALANCE_FORK, or SD_BALANCE_EXEC. Balances load by selecting the idlest CPU in the idlest group,
or under certain conditions an idle sibling CPU if the domain has SD_WAKE_AFFINE set."* The
affine heuristic is deliberately cheap, and says so: *"The purpose of wake_affine() is to quickly
determine on which CPU we can run soonest. For the purpose of speed we only consider the waking and
previous CPU."*

And a push, for the thread a pull cannot reach. A running task is not on a run queue to be
stolen from, so `sched_balance_rq` asks `need_active_balance` and, if it says yes, queues work on
the busiest CPU with `stop_one_cpu_nowait`: *"active_load_balance_cpu_stop is run by the CPU
stopper. It pushes running tasks off the busiest CPU onto idle CPUs."* The stopper thread really is
the migration thread of the folklore; `kernel/stop_machine.c` names it `"migration/%u"`.

All of it hangs off a per-CPU domain hierarchy, and every level carries its own constants.
`sched-domains.rst`: *"Balancing within a sched domain occurs between groups"*, and
`sched_balance_domains` *"checks each scheduling domain to see if it is due to be balanced, and
initiates a balancing operation if so"*, walking `for_each_domain(cpu, sd)` up the parent chain.
The tuning surface is four fields per level in `include/linux/sched/topology.h`:

```
	unsigned long min_interval;	/* Minimum balance interval ms */
	unsigned long max_interval;	/* Maximum balance interval ms */
	unsigned int imbalance_pct;	/* No balance until over watermark */
	unsigned int balance_interval;	/* initialize to 1. units in ms. */
```

One correction to the shape of the hierarchy, which a maintainer stated from memory as "SMT,
then cache-sharing cores, then NUMA nodes". `kernel/sched/topology.c` names three default levels,
not two, and NUMA is not one of them:

> By default (default_topology[]) these include:
>
>  - Simultaneous multithreading (SMT)
>  - Multi-Core Cache (MC)
>  - Package (PKG)
>
> Where the last one more or less denotes everything up to a NUMA node.

NUMA levels are built above `PKG` from the distance table, and an architecture may replace the
array entirely with `set_sched_topology()`.

The maintainer's three-moment summary was substantively right, and this is a re-cut rather
than a correction: it folded the tickless case into the idle one, where the source puts it beside
the periodic one, since a NOHZ kick makes a *busy* CPU run the balancer for idle neighbors.
`sched_setaffinity` is not on this list because it is a request from userspace rather than a moment
the kernel chooses.

**What this is evidence of, and it is not that Linux is careless.** Linux has to guess continuously
because almost nothing ever tells it anything. It schedules unknown binaries for users who never
declare a placement. The interface that would let them exists (`sched_setaffinity`), but the kernel
cannot assume it was called, so its working inputs are load, topology and history. Four sampling
points plus a push is what it takes to keep an arbitrary workload spread without being told. That is a harder
scheduling problem than nife has, and the machinery is proportionate to it rather than excessive.
The contrast with seL4 and Barrelfish is about the premise, not about the quality of either
answer: they assume the application knows where its threads belong and make that a right it holds,
so there is nothing left for the kernel to infer. nife's §28 sits on the second premise, and
milestone 219's frozen counter is what the second premise looks like from a bench.

**And Linux itself treats "do not balance this" as a supported request rather than an escape
hatch**, which is the half of the comparison that argues against reading its balancer as
unconditional. `cpuset.cpus.partition` accepts `"isolated"`, documented in
`Documentation/admin-guide/cgroup-v2.rst` as *"Partition root without load balancing"*, and cgroup
v1's `cpuset.sched_load_balance` is the older spelling of the same control.

### QNX and Linux, for the authority question specifically

QNX (`ThreadCtl`, https://www.qnx.com/developers/docs/8.0/com.qnx.doc.neutrino.lib_ref/topic/t/threadctl.html)
requires no privilege at all and instead makes the dangerous case unreachable: its runmask control
sets the affinity of *the calling thread*, and doing it to a thread in another process is not
supported. The legal masks are further constrained to boot-defined clusters, which is a bounding set
expressed as static configuration.

Linux is the one to argue against, and it is walking away from itself. `sched_setaffinity(2)`
(man-pages 6.9.1, fetched 2026-09-02) needs *"an effective user ID equal to the real user ID or
effective user ID of the thread identified by `pid`, or it must possess the `CAP_SYS_NICE`
capability in the user namespace of the thread `pid`"*: one coarse ambient bit that also gates
`nice` and I/O priority, with nothing bounding which CPUs may be requested. The bounding lives in
a different subsystem, and the manual page is candid that it does not fail loudly:
*"These restrictions on the actual set of CPUs on which the thread will run are silently imposed by
the kernel."* cgroup v2 chose the other way, both in failing loudly on the exclusivity rule and in
making the carve-out right non-delegable: `cpuset.cpus.partition` is
*"owned by the parent cgroup and is not delegatable"*, which is the copy-versus-mint distinction a
capability system gets for free.

### What the prior art decides, and what it leaves open

- **Nothing here treats "a saturated workload does not migrate" as a bug.** seL4 and Barrelfish both
  refuse implicit migration on purpose, and seL4's stated reasons (user control of resources,
  real-time analysability) are reasons this project already holds. That is a strong argument
  against option B and it did not come from this tree.
- Linux is the instructive contrast rather than the cautionary tale, and reading it sharpened
  what option B actually is. nife is not innocent of load balancing: `pick_spawn_target` is the
  fork-time placement Linux does in `select_task_rq_fair` under `SD_BALANCE_FORK`,
  `try_initiate_steal` in `run_idle` is the going-idle pull Linux does in `sched_balance_newidle`,
  and `wake_load_aware` off `irq_notify` is a narrow version of its wake-time placement. Three of
  Linux's four moments already have a counterpart here. The one nife does not have is the
  periodic one, and it is the only one of the four that fires whether or not anything happened.
  That is exactly why the counter freezes: a saturated rendezvous workload starves the other three
  of their triggers (nothing forks, no core goes idle, no device interrupt lands), and an
  event-driven balancer with no events does nothing. This is a point in option B's favor and it
  is recorded as one, below.
- If affinity is built, the shape has a clear winner in the family this project belongs to: the
  core is a property of a capability, not an argument in a message. seL4 MCS and L4Re agree, from
  opposite directions (a singleton per core, a narrowable set), and seL4's own history is a
  worked example of the parameter form being the thing that got replaced.
- Still open, and calef's: whether nife wants the singleton or the set, and whether a placement
  authority is a new object or a field of something that exists. Nothing read here decides that.

## The options

### A. Thread affinity: a program says where a thread runs

What it is. A placement request a program can make. §28's own deferred wording puts it in the
spawn manifest, where it is a property of a thread being created rather than a method on one that
exists.

What the surface grows by, exactly. In the manifest form: one field on the spawn contract, plus
its validation, plus whatever names the authority. In the invocation form it would be one method on
the thread control block capability (`crates/abi`'s `thread_control_block` module) and one arm in
`kernel/src/syscall.rs`. Either way the thing that is irreversible is not the code. It is that every
future program is written against it and every future scheduler must honor it.

The capability question it opens, and this is the part with no default answer. Is pinning an
authority? Three readings, and the tree does not currently prefer one:

- **It is not an authority at all**, because a thread choosing its own core is choosing where to
  spend time it already has. This is roughly Linux's position for the self case.
- It is an authority over the machine, because a program that pins ten threads to core 0 has
  taken a scheduling decision away from everyone else. There is no CPU budget mechanism here to
  bound that; §28 defers budgets too, and with no priorities the scheduler is round-robin, so a
  pinning program's effect on its neighbors is unbounded by construction.
- It is an authority over a thread, in which case the thread control block capability already
  names it and the answer falls out with no new object. This is seL4's non-MCS answer, and seL4
  replaced it: see the prior art above, where the same kernel gates priority with a bounding
  capability and gated placement with nothing.

The prior art narrows this further than §28 did. Both microkernel families that solved it put the
core in a capability rather than in a message, and that form is available to nife with or without
a manifest field. It is also the form §28's own wording points at: a *grant* is a capability, and a
grant that names a core is exactly seL4's `sched_control_cap`.

Reversibility. Low. A manifest field is a wire format two programs agree on, which is the *move
fast on what can be undone* tenet's first irreversible category. Nobody has acted on it yet, which is
the one thing in its favor.

What it buys the experiment. A pinned caller and a pinned responder must cross on every round
trip. But the crossing is the caller's `CALL` blocking and its reply arriving on the responder's
core, which is the rendezvous path. The defect was on the `wake_load_aware` path. So A produces
sustained cross-core traffic adjacent to the defect rather than on it.

### B. A periodic rebalancer in the scheduler

What it is. Kernel policy: on some tick boundary, move a thread from a loaded queue to a lighter
one.

Reversibility. High, and it is the only option here that is. §28's own changeability note says
so: *"this is scheduler-internal policy. No ABI, no capability semantics, no baseline movement"*.

What the prior art says about it, and it is the strongest single argument in this document.
No capability microkernel read for this proposal rebalances threads at all. seL4 does not, on the
record and in the source, and its authors' stated reason is *"seL4's general philosophy of having all
resource management under user control (and also helps reasoning about real-time properties)"*.
Barrelfish does not, structurally. Linux does, and offers turning it off as a first-class control
(`cpuset.cpus.partition` set to `"isolated"`, *"Partition root without load balancing"*).
So the behavior milestone 219 measured and read as a gap is the design position of the kernel this
project measures itself against, and B would be the one option that moves nife away from it in
order to run a test.

The point in its favor, which reading Linux produced and which nothing in this tree had said.
B is not "add balancing to a kernel that has none". nife already balances at three of the four
moments Linux does; all three are event-driven, and B is the fourth, the one that fires on a clock
whether or not anything happened. Stated that way it is a smaller change than it looked, and it is
the *only* one of the four that a saturated workload cannot starve of its trigger. Anyone arguing
for B later should start here rather than from the numbers.

What it costs. A timer-driven path in the hottest subsystem, and tuning constants that are a
permanent maintenance surface with nothing to measure them against. Linux is the measurement of that
surface rather than an assertion about it: `include/linux/sched/topology.h` carries `min_interval`,
`max_interval`, `balance_interval` and `imbalance_pct` per domain level, and
`sched_balance_domains` walks the whole chain deciding at each level whether its interval has
elapsed. That is the shape the constants take once the mechanism exists, and Linux is where they
have a topology and thirty years of workloads to be tuned against. nife has neither. This tree also
has no workload whose fairness visibly fails, which is the trigger §28 named for reopening
scheduling policy, so B would be adding policy to make a test possible and then living with it in
every boot forever.

What it buys the experiment. Real migration of the workload's own threads, which is more than A
or D give. But it moves threads for reasons the defect had nothing to do with, and it changes the
system under test in the act of testing it, which is the `cpu.rs` objection above pointed the other
way.

### C. A userspace-drivable interrupt source

What it is, as the milestone framed it. A syscall a program calls that causes an interrupt, so
the workload itself drives `irq_notify` at rate.

Verified fact that reshapes this option: the userspace half already exists. `abi::irq::WAIT` is a
method on an `Irq` capability, `crates/user_rt`'s `irq_wait` calls it, and `kernel/src/soak.rs`
already hands workers capabilities at spawn. A program can already block on an interrupt route
with no new surface at all. What is missing is only the raise, and the raise does not have to be
a syscall, because the kernel is already the thing that raises interrupts.

So the option as stated ("userspace can cause interrupts") is asking for the more expensive half of
a thing that is already half built, and it is dominated by:

### D. A soak-only, kernel-side interrupt raiser (this lane's finding, and it was measured)

What it is. Under `--features soak` and nowhere else, the kernel signals a routed rendezvous from
inside a real interrupt handler, and soak workers block on it through the existing `Irq::WAIT`. No
syscall is added. Nothing exists in a production build.

It reaches `wake_load_aware` on its real path, and the path was read rather than assumed.
`sched::on_tick` is called by all three timer interrupt dispatchers
(`arch/aarch64/exceptions.rs`, `arch/riscv64/exceptions.rs`, `arch/x86_64/exceptions.rs`), in real
interrupt context, on every core independently. A `#[cfg(feature = "soak")]` call from there into
`sched::irq_notify` runs the identical sequence a device interrupt runs:
`rendezvous.signal()` -> `handshake.serve()` -> `wake_load_aware` -> `pick_wake_target` ->
`place_on` -> the reschedule IPI. `irq_notify`'s own doc comment already states it is safe from
interrupt context, and the reason (`IPC_TABLES` is an `IrqSafeMutex`, §9) holds for the timer arm
exactly as it does for the device arm.

**What it does not reach**, said precisely because a claim of "the real path" is worth nothing
without it: the interrupt controller's own claim, mask and complete sequence (the PLIC on riscv64,
the GIC on aarch64) is not exercised, because the timer is not a controller-routed source. The
experiment is about the wake protocol, and that is what this runs.

The spike, and its numbers. Written, run, and thrown away on 2026-09-01. 53 lines added across
three files, no syscall change, no architecture-specific code, no change to any production path:
a routed rendezvous plus a tick hook in `kernel/src/soak.rs`, three lines in `kernel/src/sched.rs`,
and one role in `fixtures/src/soaker.rs` that loops on `irq_wait`. Same command, same host, same day as
the baseline above:

| Architecture | `crossings` at 25s, today | with the spike | round trips/s |
|---|---|---|---|
| aarch64, 4 cores | 15, frozen from beat 1 | 3,785 and rising linearly | 64,000 -> 44,500 |
| riscv64, 4 cores | 21, frozen (milestone 219) | 1,948 and rising linearly | 24,000 -> 10,600 |

`refused`, `mismatch` and `stalled` stayed at zero throughout both runs. **The spike is not
committed to this branch** and this document is the only thing that survives it; the patch is a
scratch file, deliberately, because shipping it is the decision being asked for rather than a thing a
research lane may do.

What the numbers cost, stated as honestly as the milestone states its own. The round-trip rate
falls by about 30% on aarch64 and about 55% on riscv64, because the waiters are extra threads and
every migration is real work. A soak with this on is therefore a *different* soak, and its `rounds`
figure is not comparable with a soak without it, which is one more row in `notes/soak.md`'s
comparability table rather than a new kind of problem.

And the honest limitation, which nobody should discover later. The threads that cross are the
waiters, not the rendezvous pairs. The `crossings` counter cannot say which thread moved, and the
mechanism says it must be the waiters: rendezvous wakes stay local whatever else is happening, so
the callers and responders stay exactly as pinned as they are today. **D sustains the wake protocol
across cores under load. It does not make the IPC workload itself migrate.** Only B does that.

Reversibility. Highest of the four. A feature-gated affordance is deleted by deleting it, nobody
outside the tree can act on it, and no program is written against it.

## Recommendation

On the reversible part, this lane recommends D, and recommends building it before deciding A.

The reason is not that D is cheaper, which would be an argument from implementation convenience and
this tree refuses those. It is that D tests the path the defect was on and A tests the path beside
it. Milestone 219's whole lesson was that an instrument *near* the question is not an instrument
that answers it, and choosing A here would repeat that mistake at the level of the experiment rather
than the counter. D also leaves the machine unchanged when nobody is testing, which was the property
the maintainer's provisional pick of A was reaching for, and D has it more completely: A is inert
only if no program uses it, and a mechanism that is only inert by convention is rung four.

**On the irreversible part, this lane gives options and does not recommend.** A is syscall surface
and its capability question is genuinely open. This document's contribution to it is to narrow what
must be decided rather than to decide it:

- §28 already chose the shape (a grant in the spawn manifest, not a method on a live thread), so
  that part is not open unless someone argues to reopen it.
- The authority question is open and has no default. The three readings are above.
- **A is worth having on its own merits and does not need this experiment to justify it.** A
  latency-sensitive server pinned away from a noisy neighbor, and a driver thread near its device's
  interrupt, are things real systems want, and §28 deferred it with a trigger rather than refusing
  it. That case should be made on its own, with its own workload, rather than being carried in on the
  back of a test that D can run without it.

B is declined, and the refusal is the valuable half. Not because it would not work, but for two
reasons that compound. It changes what the scheduler promises in every boot forever to make one
experiment possible, and §28 named the trigger for reopening scheduling policy as *a real workload
where fairness visibly fails*; there is no such workload. And the prior art says the thing it would
add is the thing seL4 and Barrelfish deliberately do not have, for reasons this project shares. If one appears, B comes back on its own merits with something to
tune against, which it does not have today.

The Linux reading weakened that refusal without overturning it, and the weakening is stated here
rather than buried. B is the fourth of four balancing moments, three of which nife already has, so
"importing Linux's answer" overstates it; and it is the only moment a saturated workload cannot
starve, which is a real argument that nothing in this tree had made. What survives is the second
reason and not the first: the cost is not the mechanism, it is the tuning surface, and Linux is the
evidence for how large that surface gets and for the fact that it is only tunable against workloads.
The refusal now rests on §28's trigger alone. A reader who thinks that trigger has been met should
reopen B, and this paragraph is the invitation.

C is declined in favor of D, having established that its userspace half already exists and its
kernel half does not need a syscall.

## What is blocked until this is answered

Answered 2026-09-02: D approved. The list below is what was blocked, kept because it records
what the decision bought rather than what remains.

- The second half of risk 5's decisive experiment. Until one of these exists, sustained
  cross-core handoff cannot be run on radon, argon or xenon at all, and the soak on those boards
  (milestone 219's other proposed follow-on) covers one of the two things the risk needs.
- Nothing else. The soak runs, the tooling is built, and the boards can be soaked for contention
  today. This is a decision about how much more the experiment can be made to cover, not a blocker
  on anything already planned.

## What was considered and refused

- Driving a real device at rate instead (issue file reads through the block server, so virtio
  completion interrupts hit `wake_load_aware` on the fully real path). Refused for the boards, which
  is where the experiment happens: radon is real hardware with no virtio, and the device it does have
  is a UART whose interrupt rate a soak cannot drive. It would also measure the block path rather
  than the wake path, at a fraction of the rate.
- Raising a software interrupt per architecture instead of using the timer. aarch64 can already
  do it (`drivers::gic::send_sgi`, which `user/src/tests.rs` uses to raise `INIT_TEST_SGI` at a
  chosen core) and x86_64 has a self-IPI vector, but riscv64 has neither: its software-interrupt
  arm is the reschedule IPI and never reaches `irq_route`, which `kernel/src/user/tests.rs` states
  as the reason that test is aarch64-only. Since radon is the riscv64 board and the one machine that
  has already produced a defect of this class, a mechanism that cannot run there is the wrong
  mechanism. The timer arm is the one thing all three architectures share, through a function
  (`sched::on_tick`) that is already architecture-neutral.
- Making the soak workers themselves cross by pinning them (option A applied only to the soak).
  Refused on `cpu.rs`'s recorded reasoning: a workload the scheduler is not free to place is one
  interleaving explored thoroughly, and this experiment's value is in the interleavings it has not
  seen.
- Counting `PlaceRemote` more carefully and calling that the second experiment. Refused because
  milestone 219 already tried exactly this and recorded why it fails: a placement counter is
  structurally blind to migration, since a rendezvous wake places locally even when the thread has
  moved. A better counter cannot substitute for a workload that does the thing.
