---
status: PARTIAL
raised: 2026-08-28
milestone_dependencies: none
decision_dependencies: unwritten
machine_requirements: none
specific_machine: none
needs_person: no
---
# 188. The IPC fastpath: the gate measures a shape userspace does not use, and three cheaper cuts come before a hand-written path

Partial as of 2026-09-04. Phases 1 to 3 are built and measured; phase 4 is an architect's and is
untouched. The results are in "What phases 1 to 3 measured" below, and they change the
recommendation's arithmetic without changing its conclusion. Minted 2026-08-28, calef, out of the lane that gated the fastpath footprint
on the third architecture (pull request #574). The provisional framing he gave it was *"a
hand-maintained IPC fastpath, so the common case stops paying for the general one."* The title
changed because the scoping work below found that the premise needs checking before the fastpath
does: the gate that says we are over target is measuring an IPC shape that no userspace
program in this tree performs, and the largest single item it reports on aarch64 is a symbol of
which 94% is never fetched.

Phase 4 only; phases 1 to 3 were a lane's own call, needed nobody, and are done.
The decision is [§95 (hand-written IPC)](../decisions/0095-a-proven-ipc-fastpath.md) (a hand-written IPC fastpath, and
whether it can stay proven), and this gate did not cite it until 2026-09-19. It is `DECIDED`, and
what calef decided is *"don't decide yet"*, in two tiers: the eligibility predicate and its proof in
`crates/ipc` are ratified as buildable now, and the fastpath itself stays gated on the one
measurement that would justify it, which is milestone 74 (cycle counters) on milestone 127 (the seL4
machine). So the token stays `DECISION` because the spend is still calef's, and a reader now reaches
the reasoning instead of re-deriving it: phase 4 is a standing verification obligation and a
permanent maintenance cost, and this block recommends against starting it until phases 1 to 3 have
reported and the measurement exists.

## The starting numbers, and where each comes from

`bench/fastpath-{aarch64,riscv64}.txt` on `main` at fc7b04e7, and `bench/fastpath-x86_64.txt` from
pull request #574, unmerged at the time of writing. Written by `script/fastpath-footprint`, which
milestone 132 (the fast path's footprint) owns.

| | `ipc_fastpath` | `syscall_entry` | total |
|---|---|---|---|
| aarch64 | 5,788 | 3,304 | 9,092 (8.88 KiB) |
| riscv64 | 5,106 | 1,870 | 6,976 (6.81 KiB) |
| x86_64 | 6,639 | 1,637 | 8,276 (8.08 KiB) |

The target is in `notes/benchmarks/kernel-footprint-and-caches.md`'s own words ("The target"), and
it is not restated here because that section states it better than a paraphrase would: under 4 KiB
of fastpath instructions and under 1 KiB of data touched per IPC, expressed as a fraction of the
smallest L1i among the machines this project runs on so that it tracks the board list rather than a
number somebody liked. Every architecture is over it on `ipc_fastpath` alone, before entry is
counted. Read that section and the Liedtke argument above it before touching any of this; both
`script/fastpath-footprint`'s header and milestone 132's block carry the same argument in shorter
form.

Two of the figures this milestone was briefed with are stale, and the corrections matter.
`riscv64`'s `syscall_entry` is 1,870 and not 1,866. And milestone 132's "the largest single item is
`syscall::dispatch` at 2,024 bytes", which is where this milestone's whole premise came from, was
true on 2026-08-18 and is not true now. Measured in this lane's own worktree from a release aarch64
kernel built at fc7b04e7:

```
$ llvm-objdump -d --no-show-raw-insn target/aarch64-unknown-none-softfloat/release/kernel
   2020  exception_vectors
   1160  kernel::syscall::dispatch
    124  exception_dispatch
```

`dispatch` is 1,160 bytes, down 864 from 2,024, and aarch64's `syscall_entry` is down by exactly
that same 864. Milestone 156 (extract the rest and ratchet both ways) did that, by moving the rare
administrative arms out of `invoke` into `#[inline(never)]` functions. That is the cheap option this
milestone was asked to price, and it has already been built once and measured.

## Finding 1: the gate measures the SEND/RECEIVE shape, and userspace overwhelmingly uses CALL/reply

`script/fastpath-footprint`'s root list is `ipc_send`, `ipc_receive`, `schedule`, `finish_switch` and
`current_cap`. So `ipc_fastpath` is the closure over a SEND/RECEIVE round trip: two endpoints, a
one-way message each way, four `svc`s from EL0. That is exactly the shape of the `ipc_rtt_el0`
benchmark, whose own doc comment says so (*"a server (RECEIVE request, SEND reply) and a client that
self-times a loop of SEND-then-RECEIVE"*).

It is not the shape the system runs. Counting imports of `crates/user_rt`'s helpers across
`user/src` and `crates`:

| helper | files importing it |
|---|---|
| `call` (`Rendezvous::CALL`) | 25 |
| `receive_cap` (`Rendezvous::RECEIVE_CAP`) | 15 |
| `reply` (`Reply::REPLY`) | 11 |
| both `send` and `receive` in one program | 12 |

24 of `user/`'s 68 `[[bin]]` programs import `call`, plus one crate. The 12 that import both
`send` and `receive` include the sink protocol's streaming shape and the supervisors, which are not
request/response at all. The kernel's
own benchmark suite agrees with userspace and not with the gate: `call_reply`'s doc comment calls it
*"the one-endpoint shape real services use"*, and milestone 23's queue-broker note adds that the
steady state of a capability swap over a stable endpoint *"is [`call_reply`] above, instruction for
instruction"*.

Measuring the shape userspace uses makes the gap worse, not better. Re-running the same
closure with `ipc_call`, `ipc_receive_cap` and `ipc_reply` as roots in place of `ipc_send`/`ipc_receive`,
on the same binary, same cold list, same script logic:

| closure | aarch64 bytes | over the 4 KiB target by |
|---|---|---|
| SEND/RECEIVE roots (what the gate measures) | 5,788 | 45% |
| CALL/reply roots (what a quarter of the programs run) | **7,516** | **88%** |

The difference is not a different code path bolted on; it is that `ipc_receive_cap` (1,828 bytes) and
`ipc_call` (1,692) are each larger than `ipc_receive` (1,320) and `ipc_send` (952), because they carry
the reply-capability mint, the capability-table insert into the server, and the `WaitRole::Reply`
parking that DECISIONS §12 (a one-shot reply capability) requires.

And icount cannot see any of it, which is the Liedtke argument arriving as a measurement rather
than as a quotation. Per-iteration icount ticks from `bench/baseline-*.txt`:

| | `ipc_rtt` | `call_reply` | delta |
|---|---|---|---|
| aarch64 | 1,017.3 | 1,040.1 | +2.2% |
| riscv64 | 170.0 | 174.9 | +2.9% |
| x86_64 | 16,734.6 | 17,150.6 | +2.5% |

The CALL/reply shape costs 2 to 3% more instructions on all three architectures and occupies 30%
more instruction footprint. A tripwire on retired instructions is structurally blind to that gap,
which is precisely the hole milestone 132 was built to close, and it is currently pointed at the
wrong shape.

## Finding 2: aarch64's `syscall_entry` is dominated by a symbol that is 94% never fetched

`exception_vectors` is 2,020 of aarch64's 3,304, and it is not code in the ordinary sense. It is the
hardware's vector table: `kernel/src/arch/aarch64/vectors.s` builds it as sixteen `VECTOR_ENTRY`
macros at `.balign 0x80`, with the comment *"The hardware requires 2048-byte alignment. 16 entries x
128 bytes = 2048."* A syscall from EL0 traps to exactly one of those sixteen entries (index 8, lower
EL AArch64 synchronous) and fetches at most 128 bytes of the table. The other 1,892 are page faults,
IRQs, kernel-mode traps and the AArch32 entries this kernel will never support.

So aarch64's honest entry footprint is about 1,412 bytes (one 128-byte vector entry, plus
`exception_dispatch` at 124, plus `dispatch` at 1,160), not 3,304. Two consequences, and the second
is why this is in the milestone rather than in a bug report:

- The three architectures' `syscall_entry` figures are even less comparable than pull request #574
  already recorded them to be. That note explains why x86_64's 1,637 is small (a `syscall`
  instruction reads `IA32_LSTAR` and consults no IDT entry, so its entry set is four symbols); it
  does not yet say that aarch64's 3,304 is large for the mirror-image reason.
- A lane could "improve" aarch64's number by 1,892 bytes without changing one fetched
  instruction. Counting one vector entry instead of the whole table is a correct accounting fix
  that would look like a 57% win. That is a gate that can be gamed, which is the one property
  AGENTS.md's ranking function says a measurement must not have.

`exception_restore` (92 bytes) runs on the return leg of every syscall and is in neither figure. It
is a small under-count in the other direction and should be added when the table is fixed.

## Finding 3: where the bytes are, per symbol

The whole closure, aarch64, both shapes, so a reader can see what a fastpath would be skipping
rather than guess at it.

| symbol | SEND/RECEIVE closure | CALL/reply closure |
|---|---|---|
| `sched::ipc_receive_cap` | | 1,828 |
| `sched::ipc_call` | | 1,692 |
| `sched::ipc_receive` | 1,320 | |
| `sched::schedule` | 1,244 | 1,244 |
| `sched::ipc_send` | 952 | |
| `sched::finish_switch` | 824 | 824 |
| `sched::ipc_reply` | | 480 |
| `sched::wake` | 452 | 452 |
| `sched::current_cap` | 372 | 372 |
| `kmem::recycle` | 280 | 280 |
| `memcpy` | 272 | 272 |
| `switch_to` | 72 | 72 |
| **total** | **5,788** | **7,516** |

`schedule` + `finish_switch` + `wake` + `kmem::recycle` + `switch_to` is 2,872 bytes, 38% of the
CALL/reply closure, and it is the general scheduler being asked to re-pick a thread that the IPC
already knows the identity of. That, and not `syscall::dispatch`, is the largest structurally
skippable block on this path. `syscall::dispatch` at 1,160 is now about an eighth of an honestly
counted round trip (7,516 of closure plus the ~1,412 of entry that is fetched), and 864 of
its former bulk has already been removed by the cheap method.

## What a fastpath would skip, and what it cannot skip

Skippable. The first item is the only one with a measured size; the rest are read off the source
and a phase-3 lane should price them rather than assume them.

- The general scheduler re-selection, 2,872 bytes. A rendezvous that finds a waiting partner
  knows which thread runs next. seL4's fastpath switches directly to it. Ours calls `schedule()`,
  which re-runs policy, and then `finish_switch`, which carries the corpse-reaping branch that
  milestone 132 already had to classify as cold to keep the number honest.
- `trace::record`, called on every serve and every block in `ipc_call` and `ipc_reply`. It has
  no symbol of its own in the closure, so its bytes are inlined into the callers and are part of
  their 1,692 and 480 rather than a line of their own.
- `kmem::recycle`, 280 bytes, if a successful rendezvous never reaches it. The gate's closure
  includes it because `finish_switch` calls it; whether a switch that came from a completed IPC can
  reach it is a question phase 3 should answer, since a wrong answer here is the same silent
  mis-classification the cold list already risks.
- The error-return plumbing. `take_ipc_aborted()` is called after every rendezvous method in
  `syscall::invoke` and is a second load and branch on a path that has already succeeded.

Not skippable, and this is the half that makes it a design milestone.

- The capability lookup. `sched::current_cap(slot)` at 372 bytes is the bounds check that *is*
  the security mechanism; `syscall::invoke`'s own doc comment says so. A fastpath that caches or
  elides it is not a fastpath, it is a hole.
- The rights check. `SEND` needs `WRITE`, `RECEIVE` needs `READ`, `SEND_CAP` needs `GRANT` on the
  delegated capability and a subset check on the narrowing. `CALL` needs `WRITE`.
- The one-shot Reply mint and its consume-on-use. DECISIONS §12 is the whole reason a CALL is
  answerable exactly once; the fastpath still has to mint the capability, insert it into the
  server's sixteen-slot table, and handle the table being full.
- The `WaitRole::Reply` guard. `ipc_reply` refuses any thread not parked as a reply waiter, and
  its comment records why: a stale reply landing on an ordinary receiver would clobber a mailbox and
  double-enqueue the one intrusive link. That guard is on the fast path by construction.
- Generational thread-id revalidation, which notes/benchmarks.md names as one of the three
  things ours does that seL4's fastpath does not.

## What correctness must not be lost, and how it would be proved

The proofs are on the general structure, which is exactly the structure a fastpath exists to
bypass. `crates/inter_process_communication` carries six `#[kani::proof]` harnesses over `Rendezvous`: that send and receive
and signal each preserve the one-queue invariant, that a send rendezvouses iff a receiver
waited and with exactly that receiver, that a pending signal is drained before a queued sender, and
that a collected sender is forgotten by the rendezvous, which is the rendezvous half of the one-shot
Reply guarantee. `crates/capability` carries twelve more. `script/verify` runs them.

None of them prove anything about `kernel/src/sched.rs`. They prove the pure decision core that
`sched` calls into. A hand-written fastpath has two possible relationships to that, and choosing
between them is most of phase 4's design:

1. The fastpath calls `Rendezvous::send`/`receive` too. The proofs continue to cover it for free.
   It keeps the part of the cost that lives in the queue manipulation, so the win shrinks to
   the scheduler bypass and the skipped error plumbing. This is the option that should be measured
   first, because it may be most of the win for none of the verification cost.
2. The fastpath replicates the decision. Then it needs its own harnesses, plus something the
   tree does not have today: an equivalence proof. The shape is tractable precisely because
   `crates/inter_process_communication` is a pure-logic crate the solver already handles: a harness that seeds a
   nondeterministic `Rendezvous` with `kani::any()`, runs the fastpath's predicate and the general
   `send` over the same state, and asserts the same outcome and the same resulting queue state.
   That harness, not a test, is what would make a second path safe to keep.

The invariants both paths must hold, stated so a phase-4 lane can turn each into an assertion.
A message is delivered to exactly one receiver or to none. A capability is never widened. A Reply is
consumed on first use. A blocked thread appears on at most one queue. A thread's mailbox is written
only while it is parked in a role that expects that write. The first four already have harnesses in
`crates/inter_process_communication` or `crates/capability`; the fifth is `ipc_reply`'s `WaitRole::Reply` guard and is
currently held by a comment and a runtime check.

## How the two paths stay in agreement

This is the honest reason not to do phase 4 lightly, and it is a cost that never ends. seL4's
fastpath is hand-maintained, and hand-maintained means every future change to the IPC surface must
be made twice or must be proved not to reach the fast path.

A differential test that drives every operation through both paths and compares is the obvious
answer and it is not sufficient on its own, for the reason this tree already records about the
`fastpath-footprint` cold list: a wrong entry is silent. A fastpath's whole design is a predicate
that says "this case is simple enough"; a test can only exercise the cases somebody wrote down, and
the failure mode is a case that takes the fast path and should not have. What is sufficient is the
pair: the equivalence proof above for the decision, plus a differential test for the plumbing the
proof does not model (the capability table, the mailbox, the trap frame).

Three mechanisms are available and they are the AGENTS.md ladder in order, strongest first:

1. Make divergence unrepresentable. If the fastpath and the slow path both reach one function
   for the check they share, there is no second copy to drift. This is what option 1 above buys.
2. A gate that fails loudly. The equivalence harness in `script/verify`, plus a differential
   test in the kernel's own suite.
3. A written record at the thing itself. A comment on every rights check saying which path also
   performs it. This is the floor and should not be the plan.

## The alternatives, priced

Per AGENTS.md's six questions, and refusals with reasons, because the refusals are the useful half.

A. Extract cold arms with `#[inline(never)]`. Already built; measured. Milestone 156 did this to
`invoke`'s administrative arms and records its own before and after: `syscall_entry` 4,168 to 3,480
on aarch64 and 2,914 to 2,210 on riscv64, a 16.5% and a 24% cut. Further work since has taken those
to 3,304 and 1,870, so the entry figure is down 20.7% and 35.8% from the pre-156 baseline in total.
Over the same period `ipc_fastpath` went from 5,780 to 5,788, which is eight bytes: the cheap
method has delivered its win on the entry figure and has not been tried at all on the fastpath
closure, where 88% of the overrun now lives. That is phase 3.

B. Reorder the dispatch decode so common opcodes are found first. Refused, and it is worth saying
why loudly. It would not move this number by a byte. `script/fastpath-footprint` sums whole symbol
sizes; reordering a match changes which instructions execute, not which symbol they live in. It is
already effectively done: `SYS_INVOKE` is one of four syscall numbers, `Object::Rendezvous` is
the first arm of `invoke`'s object match, and `SEND`/`RECEIVE`/`SEND_CAP`/`RECEIVE_CAP`/`CALL` are method
numbers 0 through 4. A lane that reached for this would spend a day and report a flat number.

C. Shrink argument validation on the hot path. Refused. The validation on this path is the
capability lookup and the rights checks, which the section above establishes cannot be skipped
without turning the fastpath into a vulnerability. There is nothing else to shrink: DECISIONS §10
(process model: capability-based, microkernel) means no pointer crosses the boundary, so there is no
user-memory validation on this path to begin with.

D. Split dispatch into a hot decoder that tail-calls a cold one. Subsumed by A, which is the
same idea with the extraction done per method rather than per decoder, and which is already built.

E. Fix what is measured before optimizing it. This is the recommendation, and it is phases 1 and
2. It shrinks nothing and it is a precondition for every other option being judgeable.

F. A hand-written fastpath, seL4's shape. Phase 4. Deferred, not refused.

## Phasing

Each phase is a lane that can be picked up without reading the others. Phases 1 and 2 are hours,
phase 3 is a day, phase 4 is a project.

Phase 1: measure the shape the system runs. Add `ipc_call`, `ipc_receive_cap` and `ipc_reply` to
`script/fastpath-footprint`'s roots, or better, report the SEND/RECEIVE and CALL/reply closures as two
named numbers rather than merging them, since they are two shapes and averaging them would hide
both, which is the reasoning the script already applies to `ipc_fastpath` and `syscall_entry`.
Re-record all three baselines with `--save`. Correct notes/benchmarks.md's comparison section,
which says *"our EL0 path issues `SEND`, `RECEIVE`, `SEND`, `RECEIVE`"*: that is true of the `ipc_rtt_el0`
benchmark and false of every real service, which uses `CALL`. The corrected count is three
syscalls to seL4's two (client `CALL`, server `RECEIVE_CAP`, server `REPLY`), not four to two, and
the residual one is the `ReplyRecv` fusion this tree does not have. That is a live open question
below, not a defect.

Phase 2: make the entry figure honest. Count one 128-byte vector entry rather than the whole
2 KiB aarch64 table, add `exception_restore`, and record in the script beside the `ENTRY` table why
each architecture's set is what it is (pull request #574 established that pattern for x86_64; this
extends it to the other two). Re-record. Expect aarch64's `syscall_entry` to drop to roughly 1,412
and expect nothing to have got faster, which is the point: the commit message must say the
number moved because the accounting was wrong.

While in that file, check it against milestone 186 (derive the architecture list). The script's
`arches` default is a hardcoded `"aarch64 riscv64"`, which is the exact defect 186's worklist
exists for, and pull request #574 fixed one instance of it by hand rather than by derivation. Phase
2 is the natural moment to take whichever mechanism 186 lands; taking it here without waiting for
186 would be a second hand-fix.

Phase 3: apply milestone 156's method to the closure. The extraction that took 864 bytes out of
`dispatch` has never been tried on `ipc_call`, `ipc_receive_cap`, `schedule` or `finish_switch`. Each
has cold tails that a successful rendezvous never reaches: the aborted-rendezvous returns, the
capability-table-full path, the corpse reaping in `finish_switch` that milestone 132 already had to
special-case as cold in the closure walk rather than in the code. Measure first, extract second,
and report the number even if it is small, because a small number here is what decides phase 4.

Phase 4: the hand-written fastpath. Do not start this without phase 3's number and an
architect's decision. Build option 1 first (a fast path that still calls the proved `Rendezvous`
methods and skips only the scheduler and the error plumbing), measure it, and only then consider
option 2 and the equivalence proof it requires.

## The recommendation, with the numbers behind it

**Take phases 1 to 3. Do not commit to phase 4 yet.**

The reasoning is not a preference for less work; AGENTS.md's elegance tenet explicitly refuses that
argument, and the test it gives is *would I still choose this if both options were the same amount
of work?* The answer here is still yes, for three reasons that survive the effort question:

- **The premise this milestone was minted on has moved.** `syscall::dispatch` was the obvious target
  when it was 2,024 bytes and the largest single item. At 1,160 it is a sixth of the CALL/reply
  closure, and the largest structurally skippable block is now 2,872 bytes of general scheduling.
  A fastpath aimed at the decoder would be aimed at the thing that already shrank.
- **We do not currently know what a fastpath would be measured against.** The gate reports 5,788 for
  a shape a quarter of the programs do not run, 7,516 for the shape they do, and 1,892 bytes of
  aarch64's entry figure are never fetched. Building a second path against a number in that state
  means the before-and-after cannot be trusted, and a benchmark that cannot be trusted is worse
  here than no benchmark, because it will be quoted.
- **The payoff is unobservable on the hardware this project owns today, and this must be said
  plainly.** `script/fastpath-footprint` bounds a quantity, not a harm: nothing in this tree models
  a cache, and the HVF host's L1i is several times the boards'. `script/bench`'s icount tripwire
  models no cache either, and the 2 to 3% `call_reply` delta above is exactly what it can see of a
  30% footprint difference. A milestone claiming a fastpath speedup today would be claiming a
  number nobody can measure. Milestone 74 (cycle counters) on milestone 127 (the seL4 machine),
  with PMU cache-miss events on a 48 KB L1i, is what turns this from arithmetic into measurement,
  and milestone 25 (cross-OS performance comparison) is where the result would be published.

Phases 1 to 3 are each defensible on their own terms whatever phase 4 turns out to be worth: a gate
pointed at the right shape, an entry figure that cannot be gamed by 1,892 bytes, and the cheap
extraction applied where it has not been tried. If phase 3 gets the CALL/reply closure under 4 KiB,
phase 4 is not worth building and this milestone closes having said so, which is a better outcome
than a heroic one.

## Open questions, and the one that is an architect's

**`ReplyRecv` fusion is a syscall-surface change and is explicitly not proposed here.** seL4's round
trip is two syscalls because it fuses reply-and-wait; ours is three because a server issues `REPLY`
and then `RECEIVE_CAP` separately. Fusing them would be a new method on `Reply` or a new one on
`Rendezvous`, and DECISIONS §10 and §16 govern that surface: it is a boundary every future program
is written against, and AGENTS.md puts anything two programs agree on in the irreversible category.
**It is named here so it is tracked and not so it is planned.** Its cost is one trap per round trip,
which the tree has never measured on its own (notes/benchmarks.md says so in its own words: the
kernel-side `call_reply` bench measures the fused shape, but *"there is no EL0 twin of it, so the
structurally matched comparison to seL4's published pair is not currently measured at all"*).
Measuring it is phase 1 work; deciding it is an architect's.

**Left open, for a lane rather than for calef.** Whether the two closures should be two gated
numbers or one, and if one, which. Whether the fastpath predicate should be a function both paths
call or a duplicated condition with an equivalence proof. Whether the data-footprint half of the
target (under 1 KiB touched per IPC), which milestone 132 records as measured by nothing at all, can
be estimated from the structures this path touches without waiting for a PMU.

## What this cannot deliver

- **No speedup anyone can currently observe.** Everything above is static footprint. The instrument
  that would show the harm does not exist on the machines in this house.
- **No answer for x86_64's `ipc_fastpath` being the largest of the three.** Pull request #574
  established that this is the same portable Rust in a denser ISA rather than a different path, so
  there is no x86_64-specific work hiding here, and milestone 161 (the x86_64 kernel port) owns
  anything that turns out to be.
- **No change to the syscall surface.** A fastpath changes how a call is serviced, not what calls
  exist.
- **Nothing about the data half of Liedtke's pair.** The under-1-KiB target has no instrument and
  this milestone does not build one.

## BUGS

Not started; nothing built yet to carry its own `BUGS` section. Three limitations of the *existing*
gate are recorded above rather than here, because a reader meets them in
`script/fastpath-footprint` and milestone 132 first: the root list measures a shape userspace does
not run, the aarch64 entry figure counts sixteen vector entries where a syscall fetches one, and
`exception_restore` is counted by neither figure.

## Scope note

Phases 1 to 3 are tooling, a baseline re-record and a mechanical extraction: no syscall surface, no
dependency, no wire format, no `DECISIONS` section. Phase 4 is none of those things and would need
its own scope note, its own `DECISIONS` section, and calef.

Names: nothing new is minted here. If phase 1 splits the gated figure in two, the two names are
calef's like every other name in the tree, and a lane should ship provisional ones and say so.

## What phases 1 to 3 measured

*Built 2026-09-04, one lane, pull request #732. Every figure here is from a release kernel in that
lane's worktree; the control runs are named where one was needed.*

**The premise held, and it had drifted a little.** Re-measured at `61e30dcb`, the SEND/RECEIVE closure
is 5,888 on aarch64 (the block said 5,788 at `fc7b04e7`) and the CALL/reply closure is 7,576 (the
block said 7,516). `exception_vectors` is still 2,020 bytes of sixteen entries of which a syscall
fetches one. Both findings the milestone was built on are true.

**Two of the block's supporting counts were low and are worth correcting**, because they were
counted honestly and the tree grew. Files importing `user_rt::call` is 35 rather than 25,
`receive_cap` 23 rather than 15, `reply` 20 rather than 11. The one that moves the argument is `send`,
at 64 files: a bare `SEND` is widely used, and it is a genuinely different shape (one-way
notification) rather than half of a round trip. That is the argument for reporting two numbers
rather than replacing one with the other, and it is what the gate now does.

### Phase 1: the gate reports and checks two shapes

`ipc_send_receive` and `ipc_call_reply`, provisional names. `ipc_fastpath` survives as a derived
figure, the worse of the two rather than the average or the sum, because one round trip is one
shape or the other; it keeps its old name because a dozen places in the tree cite it, and it is
recorded rather than checked, since it is the max of two numbers that are each already checked.

`notes/benchmarks.md`'s "`SEND`, `RECEIVE`, `SEND`, `RECEIVE`" claim is corrected there: three syscalls
to seL4's two, not four.

### Phase 2: aarch64's entry figure is honest, and nothing got faster

One 128-byte vector slot instead of all sixteen, plus `exception_restore` (92 bytes), which runs on
every syscall's return leg and was in neither figure. aarch64 `syscall_entry` 3,304 to 1,508.
riscv64 needs no equivalent (`stvec` is direct-mode, one handler, no table) and x86_64 needs none
(no IDT entry at all), so the asymmetry the milestone predicted is confirmed in both directions and
is now argued in the script beside the `ENTRY` table.

### Phase 3: the cheap method does not transfer, and finding that out is the result

**Milestone 156's `#[inline(never)]` extraction, applied to the closure, made the number bigger.**
Four cold arms went out of line and aarch64's `ipc_send_receive` moved 5,888 to 6,220, because a
closure walk follows the new call and counts the same bytes under a new name. 156's method works on
`syscall_entry` because that half is flat. On a closure, extraction is a no-op unless the walk
is told the callee is cold.

**So the gate now reads `#[cold]` out of the workspace's own source** rather than from a
hand-maintained list inside itself. That is the AGENTS.md ladder moving up a rung: the claim lives
where a reader and the optimizer both already meet it, a lane cannot widen the exclusion by editing
the gate, and outlining a genuinely cold arm is now credited automatically. `strand_callers_of` and
its two siblings dropped out of the regex and are carried by their own attributes.

The four arms: `finish_switch`'s reap, `schedule`'s killed-thread conversion (the *test* stays hot,
two loads and a compare; only the body moved), `schedule`'s self-pop heal, and `set_ipc_aborted`,
whose four call sites are all on these closures.

| | `ipc_send_receive` | `ipc_call_reply` | `syscall_entry` | total |
|---|---|---|---|---|
| aarch64, before | 5,888 | (7,576) | 3,304 | 9,192 |
| aarch64, after | **5,356** (-9.0%) | **7,028** (-7.2%) | **1,508** | **8,536** |
| riscv64, before | 5,122 | (6,390) | 1,828 | 6,950 |
| riscv64, after | **4,632** (-9.6%) | **5,936** (-7.1%) | **1,828** | **7,764** |
| x86_64, before | 6,767 | (8,657) | 1,637 | 8,404 |
| x86_64, after | **6,236** (-7.8%) | **8,122** (-6.2%) | **1,637** | **9,759** |

Parenthesized figures are what the same binary would have reported under phase 1's roots; they were
not measured before this milestone. A control was run to attribute the shrink: the new script
logic against the pre-extraction `sched.rs` reproduces the "before" row exactly on all three ISAs,
so the whole 6 to 10% is the extraction and none of it is the mechanism.

**The cost, measured rather than asserted.** +0.17% retired instructions on `ipc_rtt` and +0.24% on
`call_reply` (aarch64, icount, same tree with and without the extraction). That is the outlined
calls, and `script/bench --check` passes. No cache effect was measured, because none can be:
nothing in this tree models a cache and the HVF host's L1i is several times the boards'.

**One of the block's open questions is closed structurally.** `kmem::recycle` left the closure on
every architecture. It was reachable only through `finish_switch`'s reap arm, so a successful
rendezvous cannot reach it, and that is now a property of the code rather than an assertion in a
script.

## What this says about phase 4, which is the question calef is holding

**Phases 1 to 3 did not close the gap and were never going to.** The shape the system runs is
`ipc_call_reply` at 5,936 to 8,122 bytes against a 4 KiB target: 48% to 103% over, after the
cheap method has been applied and after the accounting was made honest. The block's own closing
condition was *"if phase 3 gets the CALL/reply closure under 4 KiB, phase 4 is not worth building"*.
It did not, by a wide margin, on any architecture.

**But three things changed in phase 4's favor, and one against it.**

For it. The number a fastpath would be measured against can now be trusted, which was the
block's second reason to wait and is discharged: the gate reports the shape services run, aarch64's
entry figure is no longer gameable by 1,892 bytes, and the cold classification is derived from the
code. The scheduler bypass is still the largest skippable block: `schedule` + `finish_switch` +
`wake` + `switch_to` is 2,380 bytes of the 7,028, 34%, and phase 3 shrank it by only 256. And
**milestone 74's riscv64 half landed the same day**, so radon can read real cycles through the SBI
PMU extension, which the block priced as waiting on milestone 127's silicon.

Against it. Radon is not sufficient on its own, and the honest version of that must be said.
`kernel/src/arch/riscv64/pmu.rs` gives cycles, and a cycle count on the U74 with a 32 KB L1i would
show *whether the round trip got faster*. It would not show *why*, and Liedtke's claim is
specifically about the application's working set being evicted, which is a displacement
measurement rather than a latency one. Milestone 134's E4 already measured 0 to 3% application
displacement under IPC load and is the instrument that would have to move. So radon turns "no
observation at all" into "a latency observation with no attribution", which is progress and is not
the experiment.

**The recommendation, unchanged in direction and better supported.** Do not start phase 4 on
footprint arithmetic alone. Start it when radon can show a *latency* difference on a padded build,
which is milestone 134's E3 experiment run on silicon rather than under icount and is a day of work
rather than a project. If E3-on-radon shows nothing, phase 4 is a standing verification obligation
bought for a number nobody can observe, and this milestone should close having said so. If it shows
something, phase 4's option 1 (a fast path that still calls the proved `Rendezvous` methods and
skips only the scheduler and the error plumbing) is aimed at 34% of the closure and is the one to
build first.

### E3 ran on radon, 2026-09-04, and it did not decide this

**The paragraph above named the measurement that would settle phase 4, and the measurement came
back confounded rather than positive or null.** The capture is in
notes/footprint-perturbation.md; six boots, interleaved, on a `single_hart` card where the
placement lottery cannot appear.

| row | unpadded (3 boots) | padded (3 boots) | delta |
|---|---|---|---|
| `ipc_rtt` | 4259 · 4259 · 4259 | 4311 · 4310 · 4310 | +1.21% |
| `call_reply` | 5015 · 5013 · 5013 | 5088 · 5089 · 5088 | **+1.49%** |
| `ipc_rtt_el0` | 128606 · 128626 · 128615 | 124958 · 124391 · 124903 | **-3.01%** |

Every separation is non-overlapping, with a within-condition spread of 0 to 2 units against gaps of
74 and 3,865. On the arithmetic this is the row-1 outcome of the block's own routing table and
phase 4 is justified.

**It is not, because the third row is impossible.** The padded build is 3% *faster* on the round
trip that crosses EL0, and the padding is never executed: it is reached through
`core::hint::black_box` on a branch that is never taken. Resident dead code has no mechanism by
which to make anything faster. What moved is the layout, and the layout effect is 193 ns where
the footprint effect it is sitting on top of is 19 ns.

**So the +1.49% on `call_reply` is a real difference between two binaries and is not evidence about
two footprints.** E3 varies a Cargo feature, and a Cargo feature that inserts a symbol changes the
address of everything after it. This is the Mytkowicz result (*Producing Wrong Data Without Doing
Anything Obviously Wrong*, ASPLOS 2009) reaching our own bench data, and it was equally true of the
2026-08-22 patagonia run; the small cache is what made it visible rather than what caused it.

**Phase 4 therefore stays exactly where it was, and the reason is now sharper than "no evidence".**
It is that the instrument built to produce the evidence cannot separate the effect from an artifact
an order of magnitude larger. A second IPC path is a permanent verification obligation, and 19 ns
under a 193 ns artifact is not what anyone should buy one with.

**What would decide it is a layout control**, and it is cheap: run `fastpath_pad` at several sizes
rather than as a boolean, because footprint predicts a monotonic dose response and layout does not.
Written up as `design/roadmap/370-a-layout-control-for-the-perturbation-experiments.md`.

**One thing the session did settle, in phase 4's disfavor but not against it.** The board can now
resolve these differences at all: `bench: cycles_per_tick 250.00` from milestone 74's riscv64 PMU,
and a boot-to-boot spread of 0 to 2 units on a four-figure count. The methodological complaint
nobody could answer on patagonia, that a small percentage might be a timer artifact, is answered.
The measurement is precise. It is the attribution that is missing.

## Follow-on

- **Recorded.** The closure half of `script/fastpath-footprint` counts bytes no IPC fetches, for the
  same reason its flat half does: whole symbol sizes, so a cold tail inside `ipc_call` or
  `ipc_receive_cap` counts. Phase 3 outlined the arms it could justify; the rest are inside conditional
  branches the script deliberately does not follow. In the script's own `BUGS` section, beside the
  measurement.
- **Recorded.** `--features fastpath_pad` still works on two of three architectures, because
  x86_64 has no `fastpath_pad` module where `kernel/src/arch/aarch64/fastpath_pad.rs` and
  `kernel/src/arch/riscv64/fastpath_pad.rs` exist. In `script/fastpath-footprint`'s `BUGS` section.
  It blocks running milestone 134's E3 on xenon and not on radon, which is the machine that matters
  for the paragraph above.
- **Done.** Run milestone 134's E3 (the footprint-perturbation experiment) on radon with
  milestone 74's cycle counters, which this block named as the measurement that decides phase 4.
  Run 2026-09-04, six interleaved boots; notes/footprint-perturbation.md. It did not decide phase 4,
  for the reason the section above gives: the comparison cannot separate footprint from code layout,
  and the layout artifact is ten times the footprint effect.
- **Milestone 370.** A layout control for the perturbation experiments, which is what E3 now needs
  before any reading of it can be attributed to footprint.
  `design/roadmap/370-a-layout-control-for-the-perturbation-experiments.md`. Until that control is
  built, no E3 number should be quoted as a footprint result, this block's included.
- **Milestone 380.** DECISIONS §144's 16 KiB ceiling was stated over "the sum of `ipc_fastpath` and
  `syscall_entry`", and this milestone changed both terms. The honest subject is now
  `max(ipc_send_receive, ipc_call_reply) + syscall_entry`, which is what the gate prints as `total`,
  and the headroom §144 recorded was measured on the smaller shape (x86_64 is now 60% of the
  ceiling, not 51%). `design/decisions/` is not a lane's to amend, so it was written up as
  `design/roadmap/380-the-ceiling-applies-to-a-number-that-moved.md` for whoever holds §144, and
  they folded it into §144 the same day, which is why that block is BUILT.
- **Outstanding.** Phase 4 itself, the hand-written fastpath. Untouched, gated on calef, and the
  section above says what would decide it. Checked against the tree: `kernel/src/sched.rs` has one
  path through `ipc_call`, `ipc_receive_cap` and `ipc_reply` and no second one.
- **Refused.** `ReplyRecv` fusion, which would take the round trip from three syscalls to two. It is
  a syscall-surface change, DECISIONS §10 and §16 govern it, and the block already says it is named
  so it is tracked and not so it is planned. A lane must not take it.
  This refusal is milestone 467 (design/roadmap/467-replyrecv-fusion.md), which carries it with the
  condition that would change it.

## Index row

Minted from the lane that gated the footprint on x86_64 (#574), whose finding was that `syscall::dispatch` is the largest single item in every architecture's entry figure. Scoping it
found the premise had moved: milestone 156 already cut `dispatch` from 2,024 to 1,160 bytes on
aarch64, `script/fastpath-footprint`'s roots measure a SEND/RECEIVE shape that 25 of 69 userspace
programs replace with `CALL`/reply (7,516 bytes against the gate's 5,788), and 1,892 of aarch64's
3,304-byte entry figure is vector-table padding no syscall fetches. Recommends three cheap phases
(measure the right shape, make the entry figure honest, apply milestone 156's extraction to the
closure) before any hand-written second path, which cannot be justified until milestone 74's PMU
on milestone 127's TX1 can observe the payoff. Gate: DECISION. PARTIAL 2026-09-04: phases 1 to
3 built. Both premises held. The gate now reports `ipc_send_receive` and `ipc_call_reply` separately
(the shape services run is 25 to 29% larger), and aarch64's `syscall_entry` counts one vector
entry instead of sixteen (3,304 to 1,508, accounting, nothing got faster). Phase 3's finding:
milestone 156's extraction does not transfer to a closure, and the gate now reads `#[cold]`
from the source rather than a list inside itself; 6 to 10% off both closures for +0.2%
instructions. The gap is not closed, so phase 4 stays an architect's.
