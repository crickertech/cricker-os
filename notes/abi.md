# The native ABI

*(Milestone 19e, "Decision 2". The contract a nife program runs against: how it starts, how
it makes syscalls, and how it meets its capabilities. `crates/abi` is the machine-readable half;
this note is the prose half. See DECISIONS §10 for why the model is capability-based, and §14 for
why "native ABI" and not Linux-compat.)*

The big fork was already settled at milestone 7 (§10): the process model is capability-based, not
Unix. So this is not a decision about `fork` versus capabilities. It is the smaller, still-open
question the 19f split forced into the open: now that we deliver and run distinct programs, what is
the *contract* between a program and the system? Three parts: the syscall convention, the object
surface reached through it, and how a program meets its world at startup.

The decision here is to write down and commit the convention we already run, rather than build a
self-describing environment (a BootInfo page). Hardcoded, out-of-band agreement on the initial
capability layout between a parent and the children it builds is the normal microkernel pattern
(seL4 hands a BootInfo only to its *root* task; every other task gets caps placed by its parent per
a private layout). Our progenitor is that parent. A BootInfo mechanism earns its keep when a loader must
start programs whose layout it cannot know in advance, which is milestone 23 (live component replacement), with competing
vendors, not now. See "What is deliberately deferred".

## 1. The syscall convention

One instruction, `svc #0`. The kernel reads the registers, does the work, and returns.

| register | on entry | on return |
|---|---|---|
| `x8` | the syscall number | unchanged |
| `x0` | first argument | the `i64` result |
| `x1`–`x4` | further arguments | (see the specific syscall) |

Four syscall numbers, and that is the whole width of the trap:

| `x8` | name | meaning |
|---|---|---|
| 0 | `SYS_EXIT` | terminate this thread; the kernel reaps it and frees its address space. Never returns. |
| 1 | `SYS_YIELD` | give up the CPU voluntarily. |
| 2 | `SYS_INVOKE` | invoke a capability. **This one carries the entire capability world** (see §2). |
| 3 | `SYS_CAP_DELETE` | drop the capability in a capability table slot, so the slot can be reused. |

That narrowness is deliberate (DECISIONS rule 3: the syscall surface stays a boundary, not a habit).
Everything a program can do to another object goes through the single `SYS_INVOKE` door; adding a
capability *type* or a *method* does not widen the trap, it adds a row to a table the kernel already
dispatches. `crates/user_mode_runtime` is the userspace side of this: `invoke`, and `send`/`recv`/`exit` built
on it.

## 2. The object surface, reached through `SYS_INVOKE`

`invoke(cap, method, a0, a1, a2)`: `cap` names a capability in the calling thread's capability table (a small
integer, like a file descriptor), `method` selects an operation on the object that capability points
at, and `a0..a2` are the operation's arguments. The kernel checks that the slot holds a capability,
that its *rights* permit the method, and that the object's type understands it. The method numbers
live per object type in `crates/abi`:

- Endpoint (`endpoint::`): `SEND`, `RECV`, `CALL`, and the capability-passing pair `SEND_CAP` /
  `RECV_CAP`. The synchronous-IPC primitive the whole system talks over. `WRITE` rights permit
  `SEND`; `READ` rights permit `RECV`; `GRANT` permits passing a capability along.
- Reply (`reply::REPLY`): the one-shot return leg of a `CALL`.
- Untyped / objects (`objtype::`): `RETYPE` an untyped region into an `ENDPOINT`, `ADDRESS_SPACE`,
  or `TCB`. This is how a process builds new kernel objects out of a raw memory budget it holds.
- TCB (`tcb::`): `CONFIGURE` (entry, stack, address space), `CAP_INSERT` (place a capability into
  the child's capability table), `START` (see §3).
- AddressSpace (`address_space::MAP_INTO`, with modes `MAP_RO` / `MAP_RW` / `MAP_CODE`): map a
  frame into an address space at a chosen virtual address with chosen permissions.
- Irq (`irq::WAIT` / `ACK`): block until an interrupt the capability names fires, then
  re-enable it. This is how a userspace driver owns its device's interrupt.
- Rights (`rights::READ` / `WRITE` / `GRANT`): the authority a capability carries, checked on
  every invoke. A capability can be delegated with *narrowed* rights but never widened.

A program never sees a raw pointer to any of these. It sees a slot number, and the kernel is the
only thing that can turn that number into the object. That is the §10 thesis in one sentence.

## 3. The entry contract

A program is an ordinary aarch64 ELF, linked in the low half (TTBR0, at `0x6000_0000`; see
notes/address-space-map.md). The loader lays out its segments, gives it a stack, populates its capability table
(§4), and enters it at the ELF's `e_entry` with three register arguments:

```
_start(x0, x1, x2) -> !
```

`START` (the TCB method) is what hands those three words to the new EL0 thread; the kernel routes
them through `Thread::start_args` into `x0`/`x1`/`x2` at first entry (milestone 19e widened `START`
from one argument to three; see notes/tcb.md). Their meaning is the program's to define, with one
reserved case:

- For most programs, `x0`/`x1`/`x2` are plain arguments. A least_authority_demo takes its input `n` in `x1`. A
  standalone binary that needs no argument ignores all three.
- The progenitor is the exception the loader knows about: the kernel starts the progenitor with the initrd length in
  `x1`, because the progenitor must find the archive it loads everything else from (notes/progenitor-and-loading.md).
- Historically `x0` was a *role selector* for the one multi-tool `hello` binary. After the 19f split
  every program is its own binary, so `x0` is a free argument again, not a dispatch key.

A program never returns from `_start`. It runs until it calls `SYS_EXIT` (a least_authority_demo, when its job is
done) or loops forever serving requests (a driver). Returning would fall off the end of the world;
there is no runtime to catch it.

There is no libc, no `argv`/`envp` array, no dynamic loader, no `main` wrapper. `_start` *is* the
program. What a C runtime would do before `main` (zero `.bss`, set up the stack) is either done by
the loader (the stack) or unnecessary (a freshly mapped frame is already zero; `.bss` is a fresh
frame).

## 4. How a program meets its capabilities

Before `START`, the program's loader (the progenitor, or the kernel's own service wiring) has placed the
capabilities the program needs into low capability table slots, and mapped any shared pages it needs at agreed
virtual addresses. The program hardcodes which slot holds what and which VA is which. That agreement
is the contract, and it is per program, published in that program's own source:

- the least_authority_demo is granted one endpoint at slot 0 (its result channel).
- the console server gets its request endpoint at slot 0, its reply endpoint at slot 1, the
  shared text page read-only at `0x60_0000`, and the UART device frame.
- the input driver gets the line endpoint at slot 0 and its RX interrupt capability at slot 1.
- the shell holds five endpoints (slots 0–4) and two shared pages.
- a **std program** (milestone 27) gets an untyped budget at slot 0 (its heap) and a WRITE endpoint
  at slot 1 (stdout/stderr). A std program *given the network* (milestone 27 phase two) also gets a
  WRITE `Stack` endpoint at slot 2 (net_stack's socket contract, DECISIONS §25) and a second untyped
  budget at slot 3 (the per-socket shared frames `std::net` mints). Absent slots 2 and 3, `std::net`
  returns `Unsupported`: no ambient network, felt from inside the process. See notes/std.md.
- a std program *given a directory* (milestone 27 phase two, the FS half) gets a WRITE FS-service
  endpoint at slot 4, plus the page it shares with the FS server mapped at `0x1100_0000`. That
  endpoint **is** the directory capability (DECISIONS §27): the server it reaches is bound to one
  directory node, and every name `std::fs` sends is resolved under that directory, so
  `File::open("foo")` means "foo, under the directory I hold". A path that would leave it (an
  absolute path, any `..`) is refused as `InvalidFilename` rather than served, and an empty slot 4
  makes every `std::fs` call `Unsupported`: no ambient filesystem, the same shape as the network.

- a std program *given a wall clock* (milestone 51) gets a `PageFrame` capability naming the clock page
  at **slot 5**, with `READ`, plus a read-only mapping of that page at `0x1200_0000` (DECISIONS §43).
- a std program *given entropy* (milestone 56) gets the entropy service's request endpoint at
  **slot 6**, with WRITE, and **no mapping alongside it** (DECISIONS §44). The contrast with slot 5
  is the interesting part: reading the clock is a page because reading is near-harmless and wants to
  cost two loads, while obtaining randomness is a message because the service must be the only thing
  that can reach the device, and a page would be a place the bytes persist. An empty slot 6 makes
  `std::random::SystemRng` panic rather than return something predictable.

  A slot can be held without the ones below it, and the gap is load-bearing. A program granted a
  directory but no network holds 0, 1, and 4, with 2 and 3 empty, because empty 2 and 3 are exactly
  how `std::net` knows it has no network. `Spawn.grants` fills slots from zero in order and cannot
  express a gap, so the kernel-side wiring places slot 4 with `sched::grant_at` first and lets the
  ordered grants land at 0 and 1 behind it. That is the same explicit-target move `Tcb::CAP_INSERT`
  offers a userspace loader (§5's fault slot uses it), available to the kernel's own service wiring.

This is out-of-band agreement, not discovery: the program does not ask "what am I holding," it knows
by the contract it was built to. That is the same shape seL4 uses for every task below the root, and
it is honest to call it a convention rather than dress it up as an API. The convention *is* the ABI;
writing it down (here, and in each program's header comment) is what milestone 19e commits.

## 5. The fault endpoint: a thread's death as a message (milestone 22)

A supervised thread's death is delivered to its supervisor as an ordinary endpoint message, so
restart policy lives in userspace and the kernel never relaunches anything (DECISIONS §26). Two
conventions make it work, and neither adds a syscall or a method: a spawn-slot convention and a
message-format convention (both in `crates/abi`, module `fault`).

The spawn-slot convention. A supervised child is spawned with its supervision endpoint in the
reserved fault slot, `abi::fault::FAULT_EP_SLOT` (the last capability table slot, `CAPABILITY_TABLE_SLOTS - 1 = 15`).
A supervisor building a child through the TCB surface places it there with
`ThreadControlBlock::CAP_INSERT`'s explicit target argument (`invoke(tcb, CAP_INSERT, cap_slot,
rights, target)`, where `target` is `slot + 1` and `0` keeps the original first-free behaviour). At
`START` the kernel reads the fault slot: if it holds a `Rendezvous` capability the thread is
supervised, and the kernel records that endpoint as the thread's fault target and clears the
slot, so the child cannot forge fault
messages on it. An empty fault slot means the thread is unsupervised and gets the pre-milestone-22
behaviour: it dies and is reaped immediately, reporting to no one. The reserved slot is the *last*
one precisely so an ordinary child, whose grants fill the low slots from zero upward, never lands a
working endpoint there by accident and gets mistaken for supervised.

The message-format convention. When a supervised thread faults or exits, the kernel delivers one
five-word message to its supervision endpoint, taken by a plain `RECV`:

```text
  w0  event    fault::EVENT_FAULT (1) or fault::EVENT_EXIT (2)
  w1  tid      the dead thread's id, kernel-stamped
  w2  pc       the faulting instruction (0 for a clean exit)
  w3  addr     the faulting address (0 for a clean exit)
  w4  reserved 0 today; a fault-reply / resume protocol arrives here additively
```

`RECV` returns `w0` in the syscall's result register and `w1..w4` in the next four argument
registers (`x1..x4` on aarch64, `a1..a4` on riscv). Ordinary three-word IPC leaves `w3` and `w4`
zero, so a supervisor is the only receiver that reads the top two, and no other program's `RECV`
changes. The tid is trustworthy without a badge because the kernel is the only sender on this
path; seL4's badged-endpoint machinery is what you would reach for if untrusted senders ever
shared a supervision endpoint, and it returns as its own decision if that day comes.

The userspace side of that is two functions rather than one, and the split is not an ABI difference:
`user_mode_runtime::recv` reads three words and `user_mode_runtime::recv_fault` reads all five, both from the same `RECV`.
`recv_fault` arrived with milestone 36 (notes/c-seam.md), the first program to want `w3`: a restart
policy needs the event and the tid, but a *checker* needs the faulting address, because that is the
only word that says where the dead thread actually pointed.

The corpse is dead until reaped: after the message, the thread never runs again, but its TCB,
address space, and memory persist for postmortem until the supervisor reaps them with §16 revocation
(`Untyped::DESTROY` on the child's region). That is why the reserved `w4` can carry a resume protocol
later without a format change: the corpse it would resume is still there.

## The one ambient thing: reading the clock (milestone 19e / the primitive suite)

§10 says no ambient authority, and the object surface honors it: everything a program can *do* goes
through a capability. There is exactly one deliberate exception, and it is a read, not a do: EL0
can read the virtual counter (`CNTVCT_EL0`) and its frequency (`CNTFRQ_EL0`), via `user_mode_runtime::now`
and `user_mode_runtime::cntfrq`, no syscall. The kernel opens this in `timer::init` (`CNTKCTL_EL1.EL0VCTEN`);
without it the read traps.

It is an exception made with eyes open. A monotonic counter grants no authority to *affect*
anything, only to observe the passage of time, so it does not mint the kind of ambient authority §10
rejects (which was about *reaching resources* you were not handed). What it does cost is a timing
side channel, and that is a real cost every OS offering userspace timing accepts (Linux exposes the
same counter to its vDSO). We accept it too, knowingly, because the cross-OS primitive suite needs
userspace self-timing to be comparable to lmbench, which measures from userspace. The physical
counter and the timer control registers stay trapped; only the virtual counter opens. A stricter
build could revoke even this and route time through a capability; we have not, and this note is the
record of why.

### The fine counter is not ambient, and that took a decision (milestone 229)

The coarse counter above is ambient. The cycle counter is not, on two of the three
architectures, and the difference is deliberate: it is roughly 160x finer (0.25 ns against 41 ns),
so spending §10's exception a second time on it was not free. DECISIONS 139 (who may read the cycle
counter, and by what authority) decided it in three parts, milestone 228 made the closed default a
fact rather than an assumption, and milestone 229 built the grant.

The mechanism is a per-thread grant the context switch enforces: `sched::grant_cycle_counter`
sets a bool on an embryo and refuses any thread that is not one, so it belongs to the same
creation-time set as `CONFIGURE` and `CAP_INSERT` and a running program cannot ask for it. The
kernel writes the enable at the switch, beside the address-space root, comparing before it writes,
so a machine where nothing is granted never writes the register at all.

There is deliberately no syscall method to set it, and that is the part worth knowing. The
kernel half is complete; the surface is deferred. A method number is irreversible, and this one's
successor is already foreseeable: the prior art for a per-thread property as a TCB method is
`seL4_TCB_SetAffinity`, which MCS deleted and replaced with a field of `sched_control_cap`, and
milestone 147 (a profiler that holds exactly the counters it was granted) wants cross-thread
authority with a named target that no such method would provide. 139 also records that 147's
target-naming has no precedent here to price from, so the object is not buildable yet either. The
choice was method-now against not yet, and not-yet costs almost nothing while the grant has no
consumer. Whoever needs it mints it, with a requirement in hand. A field on `CONFIGURE` is not the
cheap way round: `invoke` has three argument registers and `CONFIGURE` and `START` spend all three
each, so it would mean widening `invoke` itself.

| ISA | What the grant opens | Closed by default | Cost when nothing is granted |
|---|---|---|---|
| aarch64 | `PMUSERENR_EL0.CR`, so EL0 may read `PMCCNTR_EL0` | yes, `timer::init` writes zero per core | a per-core load and a compare |
| riscv64 | `scounteren.CY`, so U-mode may read `cycle` | yes, `timer::init` writes `TM` alone per hart | a `csrr` and a compare |
| `x86_64` | nothing: `rdtsc` is already ambient | **no**, and that is a stated position | nothing; the call compiles away |

**The `x86_64` row is an exception §19 (architectural parity is a tenet) should read as stated
rather than as a gap.** `CR4.TSD` would close `rdtsc` to ring 3, and there is no coarse fallback on
that architecture the way `CNTVCT_EL0` and `rdtime` are fallbacks on the other two: `user_mode_runtime`'s
`now()` there is `rdtsc`, so closing it takes out `Instant`, `thread::sleep`, the random seed,
smoltcp's timestamps and the benchmark harness at once. DECISIONS 139 measured the alternatives
(trap-and-emulate at 1,667 ns, 4.1x the syscall it would be beating) and closed them.

**And the grant is only built when something asks for it** (milestone 237, the cycle-counter grant
as a measurement build). Every piece above is behind `any(test, feature = "cycle_counter_grant")`,
so a shipping kernel carries none of it: the field, the switch read, the register write and
`grant_cycle_counter` all leave the binary, and `sched::schedule` is 136 bytes smaller for it
(`script/fastpath-footprint`'s aarch64 `ipc_fastpath` closure, 5852 against 5988). `script/test`
compiles and runs milestone 229's proofs on all three architectures anyway, because `test` is in the
predicate, and `script/lint` clippies the feature without `test` on all three.

Two things that gate does **not** cover, both deliberately. Milestone 228 (the cycle counters are
closed by assumption)'s default write is in every build, granted or not: closing what we claim is
closed is right whether or not anyone can be granted an exception, so the "closed by default" column
above is true of a production kernel. And nothing here was deleted, because deleting it would make
the cycle figure milestone 25 (cross-OS performance comparison) publishes against seL4's
unreproducible and would quietly return DECISIONS 139's answer to "closed for everyone".

None of this is timing confinement, and nothing in the tree should be read as saying it is. Two
threads and a shared word reconstruct a 6.8 ns clock on any of the three architectures; see
notes/confinement-claims.md, which carries that row. What the grant buys is accountable
authority: the cheap accurate instrument is granted rather than ambient, and the kernel knows
which threads hold it.

## What is deliberately deferred

**Recorded-accepted by milestone 94's sweep** (2026-08-04). Both items below are deferrals with a
recorded trigger rather than work waiting for someone to notice it, so an audit may pass over them.
See notes/untracked-work-sweep.md for the inventory, and §71 for what would promote either one to a
roadmap row.

- A BootInfo / self-describing environment. A structured block the loader hands the program that
  lists its initial capabilities, their rights, and its arguments, so a program can *discover* its
  world instead of assuming a layout. This is what a generic loader needs when it starts programs it
  did not build and whose layout it cannot know. We do not have that situation yet (the progenitor builds every
  program and knows every layout), so a BootInfo would be a mechanism without a requirement. It lands
  when milestone 23 (live component replacement) creates the requirement.
- **A POSIX shim.** §10 records why this is *additive* and can come later without a rewrite: `open`
  / `read` / `write` over capability handles, the way Fuchsia's `fdio` does. Not needed to run a
  native workload, which is the whole point of doing the native ABI first.
