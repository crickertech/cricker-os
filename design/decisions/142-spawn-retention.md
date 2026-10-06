---
status: DECIDED
raised: 2026-09-04
decided: 2026-09-04
ratified_by: calef
---

# 142. What a spawner retains over a child after `START`

Answered 2026-09-03 by calef, after working the options through in
conversation: R4 declaring R0. Retention becomes a declared field on `ChildEndowment`, and what it
declares today is what the tree already does, which is retain nothing. The reasoning, including why
this unblocks milestone 133 rather than deferring it, is in "The decision" at the end. Written 2026-09-03 by the research lane `maintainer/research-spawn-retention`,
which was asked for options and costs and is forbidden to pick. *(The number 141 is **provisional**,
and it is knowingly contended: `maintainer/decision-141-application-is-grant` was also holding 141
when this was written, found by listing remote branches before briefing. It is numbered 141 here
anyway because `script/decisions --check` fails on a gap, so a lane cannot reserve a number by
skipping one. The integrator mints the real one at merge, per AGENTS.md, and one of the two will be
renumbered.)*

No recommendation, deliberately. This is upstream of a syscall-surface question and it is itself
a convention every future spawn path is written against, so it sits in the *move fast on what can be
undone; be methodical on what cannot* tenet's irreversible category. Each answer below says what it
would make true and what it would cost. Picking is calef's.

## What is being decided

**Does a spawner retain a lifetime handle on its child, and if so, is that handle the supervision
endpoint, the thread capability, or something that does not exist yet?**

Half of the convention is already written and is written well. `crates/supervision_proto`'s
`ChildEndowment` declares what a child *gets*, and it is built to be read as the whole story: its own
doc says reading one "is meant to tell you the complete authority of the thing about to run", with
`..ChildEndowment::new()` meaning that what is not listed is not granted. It carries `caps`, `blobs`,
`maps`, `placed`, `stack_pages`, and `fault: Option<u64>`, the supervision endpoint of
§26 (the fault endpoint).

**The reciprocal half has never been written.** What the parent keeps is decided today one call site
at a time, by whether that call site happens to call `cap_delete` after `START`.

### Why this is upstream of milestone 133, and why the order matters

Milestone 133 (ending a permanently blocked thread, and deciding who may) has four proposals in
`notes/blocked-thread-teardown.md`. Proposal B is a terminate verb on the thread capability, seL4's
`suspend()` transliterated, and the note's own account of how B fails ends on this:

> That may be exactly right (it separates policy from memory) or exactly wrong (it produces a stopped
> thread nobody can free), and which one it is depends on a spawn-time endowment convention that does
> not exist yet.
>
> Source: `notes/blocked-thread-teardown.md`, Proposal B

So B cannot be evaluated against a convention nobody has stated. And proposal A would state it by
accident. A needs no new authority at all, which is most of its appeal; taking it settles the
retention question in the direction of "no lifetime handle, the region capability is the whole
answer", after which every spawn path grows around that and B gets harder rather than easier. The
sequencing is the reason this file exists.

## The premise, checked rather than quoted

`kernel/src/syscall.rs:305` says every `ThreadControlBlock` method "is process-spawn machinery",
which reads as a claim that the capability is inert once the thread runs. It is true, and it is
enforced in the scheduler rather than in the dispatcher, which is a stronger place for it to live.
Read at `49db708f`:

| Entry point | Where the check is | The check |
|---|---|---|
| `thread_control_block::CONFIGURE` | `sched::configure_thread_control_block` | `if t.handshake.state != State::Embryo { return Err(WrongObject) }`, twice, before and after taking the address space |
| `thread_control_block::CAP_INSERT` | `sched::thread_control_block_insert_cap` | same, once |
| `thread_control_block::START` | `sched::start_thread_control_block` | same, once, then the whole-or-refuse gate |
| `sched::grant_cycle_counter` | itself | same, and there is **no method number for it**: §139 (who may read the cycle counter, and by what authority) declined to mint one |

There is no fourth method and no fifth entry point. A `ThreadControlBlock` capability held after
`START` authorizes nothing. Every method it names answers `WrongObject` for the life of the thread.
That property is what proposal B would end, and it is worth saying plainly that it is a real property
today and not a comment's optimism.

One consequence of the check's placement, since it bears on cost. The refusal is in `sched`, not
in the `match` arm, so a new verb that works on a running thread does not have to unpick a
dispatcher-level guard. It adds an arm and a scheduler function whose state check is different from
its three neighbours'. That is cheaper than it sounds and it is also the thing that makes the
capability's meaning state-dependent, which the ladder in AGENTS.md says to design out rather than
in.

### What the tree does today, counted

Thirty `START` sites in shipping and test code (`crates/system_initializer`, `user/src/*`; the
`supervision_proto` and `pgrep` matches are the wrapper definition and doc comments, excluded).

| | sites | where |
|---|---|---|
| `cap_delete` on the TCB slot immediately after `START` | **25** | all fifteen in `crates/system_initializer`, `root_supervisor` (2), `swapper` (3), `timetable` (2), `spawner`, `c_confiner`, `login` |
| the capability is kept | **5** | `fixtures/src/hello.rs` only, at 468, 500, 523, 671, 723 |

So the tree already has an unwritten answer, and it is "retain nothing". The five exceptions are
in one file, the kernel's own test program, whose parents `exit()` a few lines later; none of them
records a reason for keeping it, and nothing in the tree would notice if they stopped.

This inverts one of the costs that looks obvious from outside. Proposal B's own account says
"`crates/system_initializer` and every spawn path would need auditing for who ends up holding one
after `START`". The audit has now been run, and the finding is the opposite of a widening: almost
nobody holds one, so B does not hand an existing population of holders a new power. It requires
twenty-five call sites to *stop deleting*, which is an additive convention change rather than a
silent widening, and which has a price measured below.

## What this tree already does in the analogous case

Five records bear directly, and they do not all point the same way.

§26 (the fault endpoint) already decided the shape of the one handle that exists. Supervision is
granted at spawn only: the endowment goes in the reserved slot, `START` reads it, records
`Thread::fault_ep`, and clears the slot so the child cannot forge messages on its own death
channel. §26.2 is explicit that this is fixed for life, and that runtime reattach (it names
`Tcb::SET_FAULT_EP`) is deferred until something demands supervision handoff, "and it is a new
decision when it does". §26.5 makes the endpoint one per supervisor, shared by every child, with
the kernel stamping the identity per message.

§32 (a supervisor may collect a corpse without being able to build one) put a verb on that
endpoint and drew the line at the corpse. `REAP` is authorized by the kernel checking that the
named thread's `fault_ep` *is* the endpoint being invoked. Consequence 2 is the sentence this
decision has to be read against: "It authorizes collecting a corpse, not killing. The method refuses
a thread that is still alive."

§32.3 also answered the naming question once, narrowly, and said so. The tid is authorized
relative to the endpoint it arrived on, which is "the endpoint-only naming discipline applied
consistently: the name means something only to the holder of the capability it came through, and it
is not a global handle", with the standing instruction that a later operation needing to name a child
"is a fresh decision and should reach for the same shape first". This decision is that later
operation.

§40 (there is no reaper of last resort) makes ownership the answer instead of a handle. A child's
resources come from its supervisor's region, so destroying the region reclaims the child and
everything built from it, and there is no privileged collector. Its first recorded caveat is directly
about retention: "The chain holds only if children are built from the supervisor's own region," and
whether building a child from a delegated region is forbidden or merely discouraged "is not yet
decided, and it should be before someone writes a supervisor that does it."

§92 (a caretaker is supervised by the client it serves) is the adjacent lifetime decision, and it
chose supervision over membership. It also established a fact this decision needs: construction is
already splittable, because `build_child_space` returns a TCB and an address space and starting the
child is a separate step, so "build the client's space, build the caretaker beneath it, hand over the
endpoint, start the client" is expressible. Anything that has to be arranged before `START` can be.

Milestone 105 (the two forks milestone 22 named and left) has been holding half of this question
since 2026-08-04. Its fork two is `Tcb::NAME`, "a tid that becomes a handle", with three options
recorded and none chosen: a method, per-child fault endpoints (already refused by §26.5), or the
builder reporting the tid it created. Its framing is worth keeping: "Whether the kernel owes a
supervisor the ability to resolve the identity it already sends, or whether that is userspace
bookkeeping."

Milestone 126 (who else is running, and who is allowed to ask) set the looking-versus-acting line,
with `Rights::ENUMERATE` separating them. Any answer here has to say which side it falls on.

## Prior art, fetched rather than recalled

Every quote below was fetched on 2026-09-03 in this lane. Where a source would not yield a verbatim
sentence it says so rather than paraphrasing into quotation marks; this tree carried a fabricated
block quote for twelve days through every gate.

### seL4: there is no parent, only whoever kept the capability

`seL4_TCB_Suspend`, `seL4_TCB_Resume`, `seL4_TCB_Configure` and `seL4_TCB_SetPriority` all state the
same requirement, word for word:

> Capability to the TCB which is being operated on.
>
> Source: [seL4 API Reference](https://docs.sel4.systems/projects/sel4/api-doc.html) (fetched
> 2026-09-03)

Twenty TCB methods share one authority and one shape. There is no spawner in the picture at all: a
TCB is retyped from untyped, the capability lands in the retyper's cspace, and whether the retyper
keeps it is a fact about that program rather than about the kernel. **seL4's answer to "what does a
spawner retain" is "exactly what it did not delete", and the retention handle and the construction
handle are the same object.** That is the shape nife has today, minus the running-thread verbs.

The relevant caution is already in this tree: §139 records `seL4_TCB_SetAffinity` as the worked
example of a per-thread property expressed as a TCB method that MCS deleted outright.

### Genode: the parent keeps a session capability, and the power that comes with it is total

> Since the parent retains the PD-session capabilities of all its children, it can issue further
> quota transfers back and forth between the children's PD sessions and its own PD session, which
> represents the reference account for all children.
>
> Source: [Resource trading, Genode OS Framework
> Foundations](https://genode.org/documentation/genode-foundations/25.05/architecture/Resource_trading.html),
> "Trading memory between clients and servers" (fetched 2026-09-03)

> This includes the decision to destruct a child at any time in order to regain the resources that
> were assigned to the child.
>
> Source: [Recursive system structure, Genode OS Framework
> Foundations](https://genode.org/documentation/genode-foundations/25.05/architecture/Recursive_system_structure.html),
> "Component ownership" (fetched 2026-09-03)

And in the other direction, the child gets one thing:

> At creation time, the parent installs a single capability called _parent capability_ into the new
> protection domain.
>
> Source: same page (fetched 2026-09-03)

Genode is the system that answers this question hardest, and its answer is a per-child session
capability held for life. Note what it buys with it: not a kill verb, but *resources*. Retention in
Genode is about the quota account, and destruction falls out of owning what the child is made of.
That is §40's argument, reached by a different tradition, with a handle attached.

### Zircon: handles, and a deliberate refusal at the thread

> This asynchronously kills the given process or job and its children recursively, until the entire
> task tree rooted at handle is dead. Killing a thread is not supported.
>
> _handle_ must have `ZX_RIGHT_DESTROY`.
>
> Source: [`zx_task_kill`, Fuchsia](https://fuchsia.dev/reference/syscalls/task_kill) (fetched
> 2026-09-03)

The authority is a handle with a right, held by whoever holds it, and the *granularity* is the
process or the job. Zircon retains a handle and refuses to point it at a thread.

### L4Re: the retained thing is the capability, and destruction is deleting it

> If obj has the delete permission, initiates the deletion of the object. This implies that all
> capabilities for that object are gone afterwards.
>
> Source: [`l4_task_delete_obj`, L4Re](https://l4re.org/doc/group__l4__task__api.html) (fetched
> 2026-09-03)

> If the reference counter of a kernel object referenced in fpage goes down to zero (as a result of
> deleting capabilities), the deletion of the object is initiated.
>
> Source: [`l4_task_unmap`, same page](https://l4re.org/doc/group__l4__task__api.html) (fetched
> 2026-09-03)

L4Re's thread capability is a lifetime handle in the fullest sense: the group description says a
capability to a thread permits `l4_thread_ex_regs()`, which exchanges the instruction and stack
pointers and cancels ongoing IPC on a *running* thread
([Thread, L4Re](https://l4re.org/doc/group__l4__thread__api.html), fetched 2026-09-03). Retention and
construction are one capability, as in seL4, and the delete right is what turns holding into ending.

### QNX: the counterexample, and it is the ambient one

> To signal another process, either the user IDs must match, or the calling process must have the
> `PROCMGR_AID_SIGNAL` ability enabled.
>
> Source: [`kill()`, QNX Neutrino Library
> Reference](https://www.qnx.com/developers/docs/8.0/com.qnx.doc.neutrino.lib_ref/topic/k/kill.html)
> (fetched 2026-09-03)

The authority is a property of the *caller*, not of anything the caller was handed, and the target is
named by a global pid. A parent retains nothing in particular; it is simply one of the processes that
may signal. This is the model §82 and this whole tree exist to refuse, and it is included because it
is the one that makes the shape of the others visible: everybody else's answer is a held object,
and the only disagreement is which object.

### The survey in one line each

| System | What the spawner retains | Granularity | Ends a running child? |
|---|---|---|---|
| seL4 | whatever caps it did not delete, the TCB cap among them | thread | yes, `Suspend` |
| Genode | the child's PD-session capability, for life | component | yes, destruct |
| Zircon | a process or job handle with `ZX_RIGHT_DESTROY` | process/job, **never thread** | yes, and refuses at the thread |
| L4Re | the thread/task capability, plus the delete right | thread or task | yes, `ex_regs` or delete |
| QNX | nothing; the right is ambient and the name is a pid | process | yes, by pid |
| **nife today** | the region capability, the supervision endpoint, and an **inert** TCB cap that 25 of 30 sites delete | region, or corpse | **no** |

Nobody else's spawner retains nothing. nife is not unusual in refusing a kill verb; Zircon does
too. It is unusual in having no per-child object that outlives construction, and the region
capability is the thing standing in for one.

## The five answers, and what each costs

Ordered by how much new machinery they need, not by preference.

Two facts apply to all of them and are measured, so they are stated once here rather than five times.

Retention costs a capability-table slot per retained child, and the budget is nearly spent.
`CAPABILITY_TABLE_SLOTS` is 24. `CAPABILITY_TABLE_PEAK_MEASURED` is 21, in init, during
`build_child` for `credentialer`, instrumented over four boots by milestone 230 and kept honest by
`report_peak` and `script/swish-check` since milestone 231 (nothing counts how many capability slots
a boot actually uses). Three slots of headroom. `crates/system_initializer`'s `boot` starts
twelve children, all long-lived. Any answer that gives the spawner one more permanently held
capability per child asks init for twelve slots against three, so it is not an increment; it forces a
raise of `CAPABILITY_TABLE_SLOTS`, which that constant's own doc prices at 32 bytes a slot across
`MAX_THREADS` and whose history is five days of a silently halting boot. An answer whose handle is
one per supervisor rather than one per child costs zero slots, which is precisely the shape §26.5
already chose for the supervision endpoint and for a different reason.

**A new `Object` variant is cheap and is not the expensive part.** `Object` has nine variants and
`const _: () = assert!(core::mem::size_of::<Object>() == 24)` holds; a variant carrying a `ThreadId`
adds no size, since `Reply` and `ThreadControlBlock` already carry one. Sixteen `Object::` sites in
`kernel/src/syscall.rs`, fifteen in `cap.rs`, thirteen in `sched.rs`. The cost of a new capability is
the *slot* it occupies in the holder's table and the meaning it acquires, not the enum.

---

### R0. Nothing. The region capability is the whole answer

What it makes true. The spawner's authority over a running child is exactly what it has today:
`MemoryRegion::DESTROY` on the region the child was built from, plus `REAP` on the supervision
endpoint once the child is a corpse. The TCB capability stays construction scrap and is deleted at
`START`. Ending a child means ending its region, which is §40's ownership argument taken as far as it
goes.

Cost: zero. No code, no ABI, no slot. It is the tree's current behaviour in 25 of 30 sites; what
it costs is a paragraph, and the paragraph is the point, because right now the convention is
inferable only by grepping for `cap_delete`.

What it forecloses. Ending one thread without ending its region. The granularity of every
authority in the system stays the region, so a supervisor that wants to stop a misbehaving child
takes its memory with it, and a child sharing a region with a sibling cannot be singled out. It also
leaves milestone 105's fork two answered by default in option 3's direction: a tid is userspace
bookkeeping.

What it is exposed to. §40's own recorded caveat. Ownership is the whole answer only while every
child is built from its supervisor's own region, and nothing enforces that yet.

---

### R1. The supervision endpoint is the handle

What it makes true. The retained thing is the `Rendezvous` capability the spawner already holds
and already passes as `ChildEndowment::fault`. §32's `REAP` is already authorized off it by the
kernel matching `fault_ep`, so the mechanism for "this endpoint may act on this thread" exists and is
proven. A verb that acts on a *live* child would use the same check.

Cost. No new capability, and no slots at all: one endpoint per supervisor covers every child
(§26.5). A new method on the `Rendezvous` arm, and a `DECISIONS` section for its semantics under §10
(process model: capability-based, microkernel).

**What it costs that is not code.** Three recorded decisions push against it, and the note that
surveyed this said so first: §32 draws its line at the corpse in as many words, milestone 126 ruled
that a domain names its members and does not act on them, and §26.2 deferred even *reattaching*
supervision as a new decision. It also concentrates: the supervisor's one endpoint becomes authority
over every child that ever named it, and the identity separating them is a tid the kernel stamped,
which is fine for `REAP` on a corpse and is a wider blast radius on a live thread.

The asymmetry worth noticing. This is the answer a reader expects, because the supervisor is the
party that notices a hang. It is also the one that overturns the most.

---

### R2. The thread capability becomes a lifetime handle

What it makes true. seL4's answer and L4Re's: the object you build a thread with is the object
you hold it with. `ThreadControlBlock` stops meaning "a thread under construction" and starts meaning
"this thread, for its life". It is delegable, narrowable, names exactly one thread, and does not
breach endpoint-only naming, because a capability handed over is a name handed over.

Cost, measured. Twenty-five call sites stop calling `cap_delete` after `START`, which is where
the slot arithmetic bites: twelve retained capabilities in init against three spare, so it forces
`CAPABILITY_TABLE_SLOTS` up. Then a method number in `abi::thread_control_block`, a scheduler
function whose embryo check differs from its three neighbours', and a §-section.

**What it costs that is not code.** The clean property measured above stops being true: a
capability's authority becomes a function of the state of the object it names. That is the
state-dependent rule AGENTS.md's ladder says to design out, and the note on blocked-thread teardown
independently flagged the same shape in the `killed` flag.

And the widening is a real offer changing under existing holders, even though the audit found few
of them. A builder that hands out a `ThreadControlBlock` capability today is offering "you may
assemble this thread". Afterwards it offers "you may end this thread whenever you like, forever". The
five `hello.rs` sites that keep theirs would acquire that power without anyone deciding it, which is
an argument for the retention convention being written down in the same change rather than after it.

Two things learned on 2026-09-03, in conversation with calef, that raise R2's price above what
this entry first stated.

**The state-dependence is this proposal's, not the tree's.** The entry above says R2 makes a
capability's authority a function of the object's state. Re-read, that is backwards about what exists
today: `ThreadControlBlock`'s three methods are all construction verbs, and
`configure_thread_control_block`'s own comment gives the reason its refusal exists, which is a real
precondition rather than a smell:

> Refuses anything but an `Embryo`, so a running thread cannot be reconfigured under itself.

seL4 has no such rule because its TCB methods are *different verbs* (`Suspend`, `ReadRegisters`) that
are meaningful after a thread runs, not the same verbs applied later. So this tree does not have
state-dependent authority to remove; R2 is what would introduce it, by bolting one any-state verb
onto three embryo-only ones. That hybrid is more state-dependent than either model it sits between.

And seL4's verb set follows from a recovery model this tree has already declined. seL4 delivers a
fault to a handler that may read the faulting thread's registers, repair the cause and resume it,
which is why `ReadRegisters` and `Resume` exist and why a lifetime handle is load-bearing there. §26
(the fault endpoint) chose the other answer in as many words: **"restart policy stays in userspace,
and the kernel never relaunches anything"**, and *"restart policy is userspace code and stays there"*.
A supervisor here does not repair a broken thread, it builds a fresh one from the same region, and
`build_child` is that path. §26 reserved a word so a fault-reply/resume protocol could arrive
additively, so this is a deferral rather than an absence.

The consequence for this decision. A faithful R2 pays off only alongside seL4's recovery model,
which would mean adding `Suspend`, `Resume` and register access, new syscall surface with no customer.
A partial R2, one lifetime verb on a construction capability, buys the state-dependence without the
uniformity that justifies it. **R2 is therefore not "seL4's answer, available cheaply"; it is half of
seL4's answer without the half that makes it coherent.**

The mitigation available. Rights. A capability narrowed at `CAP_INSERT` time already exists as
machinery, so "construction-only" and "construction plus lifetime" could be two rights on one object
rather than two objects. That reuses `Rights` instead of teaching a new noun, and it costs a bit;
`Rights::ALL` is `0b1111` today, so a fifth bit is an ABI change to `abi::rights`.

---

### R3. A new capability, minted at `START`

What it makes true. Construction and lifetime become two objects, so nothing about the existing
one changes meaning. `START` would return the slot the new handle landed in, which is expressible:
`CAP_INSERT` already returns a child slot the same way, and `START` returns 0 today.

Cost. One `Object` variant (nine to ten, `size_of` unchanged), a new arm in the dispatcher, and
the semantics section. Then the slot arithmetic again, and worse than R2's, because this handle is
*additional* rather than a re-use: twelve in init against three spare, and the caller cannot decline
it without a flag, which means either `START` grows an argument (it has none spare; all three
registers are the child's `x0`, `x1`, `x2`) or the mint is unconditional and every existing caller
pays a slot it did not ask for.

**What it costs that is not code.** A third noun for a reader to learn in the area a newcomer already
finds hardest, and the one place this tree has explicitly warned against spending an irreversible
number: §139 declined a method whose successor was already visible on the roadmap, and the same
question applies here, since milestone 147's profiler wants cross-thread authority with a named
target and nobody has priced its shape.

---

### R4. Retention becomes a declared field, and the kernel does not change

What it makes true. `ChildEndowment` grows its reciprocal, so one struct literal states both what
the child gets and what the parent keeps, and `build_child` acts on it (deleting the TCB capability
unless the endowment says to keep it). The convention stops being inferable by grepping for
`cap_delete` and becomes the thing a reader already reads.

Cost. A field and one branch in `crates/supervision_proto`, no kernel change, no ABI, no slots
beyond whatever the declaration actually asks for. It is rung one of the ladder for the specific
failure that a spawn path forgets to decide: the field has no default that hides the question.

**What it does not do.** It cannot grant an authority the kernel does not offer, so on its own it
declares R0 and makes it checkable. It is **not an alternative to R1, R2 or R3; it is the shape any
of them would want anyway**, and it composes with 244's proposed follow-up (the largest crate in the
tree is proved by nothing a mutation can reach), which wants `boot`'s wiring to be a table a host
test can assert. That proposal is worth citing rather than absorbing: it is this subject from the
other side, and thirty-four of `boot`'s mutants delete a field from a `ChildEndowment` literal with
nothing noticing.

## The mapping: what each answer does to milestone 133

This is the table this decision exists to produce. Milestone 133's proposals, as
`notes/blocked-thread-teardown.md` states them: A finishes what `DESTROY` starts, on the region
capability; B puts a terminate verb on the thread capability; C frees the stranded caller
(its piece 1 is being split out as a
milestone of its own on the branch `maintainer/mint-254-stranded-caller`, so it is out of scope
here); D accepts the leak, bounds it, and records it.

| | A: completed reclaim | B: terminate verb | D: accept and bound |
|---|---|---|---|
| **R0** nothing retained | **enables**, and is A's own premise | **forecloses in practice**: B's authority is a capability nobody holds after `START` | **enables** |
| **R1** supervision endpoint | enables, unchanged | **replaces** B with a variant on a different object, and owes §32, §40 and milestone 126 an answer | enables |
| **R2** thread capability | enables, unchanged | **enables**, and is the convention B's own failure analysis says it is missing | enables |
| **R3** new capability | enables, unchanged | enables, on the new object rather than on `ThreadControlBlock` | enables |
| **R4** declared retention | enables | **neutral**: declares whichever of the above is chosen | enables, and is the honest way to record it |

Four readings, and they are the useful part.

**A is compatible with every answer, which is why it is not the safe one.** A needs no retention at
all, so it neither requires nor forbids any row. What makes it a decision by accident is the
sentence one line down: taking A without answering this leaves R0 in place by default, and R0 is
the only row that forecloses B. The risk is not that A conflicts with a retention answer; it is that
A makes answering feel unnecessary.

B is the only proposal this decision gates, and it gates it in one direction: B requires
retention to be R2 or R3, and under R1 it becomes a different proposal wearing B's name. That is
the whole of the sequencing problem calef identified.

D is orthogonal, and is worth taking as evidence rather than as an escape. If milestone 133 ends
as `RECORDED`, this decision still wants an answer, because R0 is a convention with consequences and
today it is unwritten rather than chosen.

Nothing here decides milestone 133, and R2 is not a vote for B. R2 says a spawner may keep a
handle; whether a verb on that handle may end a running thread is 133's question and stays there.

## How reversible this is, and who has already acted

By the *move fast on what can be undone* test, which is "who else has already acted on this":

- R0 and R4 are reversible. R4 is a field in one userspace crate. R0 is the status quo, and
  writing it down costs a paragraph that a later decision can supersede.
- **R2 and R3 are not.** Both change what an existing capability means or add a method number to
  `abi`, which is the tenet's "anything two programs agree on" category, and §139's standing note
  gives the worked example of a method number whose retirement was foreseeable.
- **R1 is the least reversible of all, and not because of code.** It would overturn §32's stated line
  and milestone 126's, and a decision that overturns two recorded decisions is read by everyone who
  meets either of them afterwards.

Who has acted already: 25 call sites, in the R0 direction, without anyone deciding. That is the
honest status of the question, and it is the argument for answering it now rather than after
milestone 133 has answered it sideways.

## What is blocked until this is answered

- Milestone 133's A/B/D choice. Not blocked in the sense of unable to proceed; blocked in the
  sense that proceeding decides this file's question without reading it.
- Milestone 105's fork two (`Tcb::NAME`, a tid that becomes a handle), which is the same question
  asked from the naming side and has been open since 2026-08-04.
- §26.2's deferred `SET_FAULT_EP`, which is the R1 row in miniature and was already deferred once
  with the note that reattachment "is a new decision when it does" arrive.

## What this decision deliberately does not touch

- Milestone 133 itself. Options and a mapping; no winner.
- Detection. Deciding a child *is* stuck is milestone 106's timed wait and milestone 23's
  residual.
- The stranded caller, which was being split out of 133 into a milestone of its own on
  `maintainer/mint-254-stranded-caller` as this was written.
- Milestone 244's endowment table. Cited above because R4 shares its shape; the two are the same
  subject from opposite sides, and folding them together would put a proof question and an authority
  question in one lane.


## The decision, 2026-09-03

R4 declaring R0, and milestone 133's proposal A ships alongside it rather than after it.

### What is decided

Retention becomes a declared field on `ChildEndowment`, so one struct literal states both what a
child is given and what its spawner keeps. `build_child` acts on it: the `ThreadControlBlock`
capability is deleted at `START` unless the endowment says otherwise. The field has no default that
hides the question, which is rung one of AGENTS.md's ladder for the specific failure that a spawn
path forgets to decide.

What it declares today is R0, which is what 25 of the 30 `START` sites already do. The five in
`fixtures/src/hello.rs` that keep their capability with no recorded reason become visible rather than
inferable, and each has to say why or stop.

### Why this releases milestone 133 instead of deferring it

**The objection to proposal A was silence, not the region capability.** A needs no new authority, and
that is most of its appeal; the risk recorded above is that taking it would settle retention *by
accident*, in R0's direction, after which every spawn path grows around it and R2 or R1 get harder.

A declared field removes the accident. With retention written down, A cannot settle anything
silently, because the convention is explicit and a later change is a change to what the field may
say rather than to thirty call sites and an unwritten habit. So the two are not sequential:

- Proposal A makes `MemoryRegion::DESTROY` finish what it starts, so a hung component's region
  becomes reclaimable. That is the capacity problem this whole area exists for.
- The stranded-caller half, split out of milestone 133 onto
  `maintainer/mint-254-stranded-caller` and not yet on `main`, frees its stranded callers: `Gone`
  does not reach a reply-parked caller today, so a server that merely died strands them too.
- Together they close the compounding: one hang stops costing one region per caller in flight, and
  stops costing the one.

Cost: zero capability-table slots, no new syscall, no ABI change, and nothing overturned. Not
§32's line at the corpse, not milestone 126's rule that a domain names its members and does not act
on them, not §26.2's deferral of reattaching supervision. Both A and 254 need the reply-capability
sweep, and it is the same sweep.

### What is deliberately not decided

A supervisor still cannot end one thread without taking its region. That is proposal B's job and
B stays open, behind a customer that does not exist yet: a supervisor that must stop one child while
a sibling shares the region, or a policy separating stopping from freeing.

R1 is named here as the likely successor, because it is the answer that fits this system's own
model rather than seL4's. The supervisor already receives the fault, already reaps the corpse and
already rebuilds the child; extending it from *notice death, reap, rebuild* to *notice hang, end,
rebuild* is the same job, it costs no slots (§26.5, one endpoint per supervisor), and §32's `REAP`
already proves the kernel can authorize an action on a thread off that endpoint. Its price is
overturning three recorded decisions, and that should be bought with a customer rather than
speculatively, which is the whole reason it is not being bought here.

**R2 and R3 are disfavoured on model grounds and not merely on cost**, per the two findings recorded
in R2's entry above: this tree has no state-dependent authority for R2 to remove, and seL4's lifetime
handle earns its keep only alongside a repair-and-resume recovery model that §26 declined in as many
words.
