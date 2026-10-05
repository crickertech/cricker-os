---
status: PARTIAL
raised: 2026-10-04
promoted_from: capability-lookup-off-the-global-lock
milestone_dependencies: none
decision_dependencies: none
machine_requirements: riscv64 silicon with four or more harts
specific_machine: radon (the success test is the 2026-10-04 radon evening's, repeated on the same board)
needs_person: yes
---
# 761. Capability lookup off the global lock

Promoted from `design/roadmap/proposals/capability-lookup-off-the-global-lock.md` on 2026-10-04
(UTC) by the lane `lane/capability-lock-per-thread`. The number 761 is provisional until the queue
lands it. *(Title and slug are drafts.)* `needs_person` is yes only because the result is taken at
radon's bench.

## Index row

The cheapest syscall stops taking the kernel's one global lock: each thread's capability table gets
its own lock, so a lookup contends with nobody. Fatal risk 4's open finding is the null syscall
costing 147 ticks a job at four busy cores on radon against 99 at one. 41% of its lookups found
`IPC_TABLES` held. This is the change that finding pointed at, built and waiting on radon.

## Why

Every syscall that names a capability starts in `sched::current_cap`, which took `IPC_TABLES`, the
kernel's one global lock, to read the calling thread's own table. On radon at four busy tasks 41%
of those lookups found the lock held, and the null syscall cost 147 ticks a job against 99 at one
task ([`notes/job-mix/null-syscall-under-load.md`](../../notes/job-mix/null-syscall-under-load.md)).
A capability table has exactly one owner thread; it shared the global lock only because it was a
field of the thread table's entries. This is fatal risk 4's open finding.

## What was built

- Each thread's capability table has its own lock, an `IrqSafeMutex` at the new rank 57
  (`sync::rank::CAPABILITY_TABLE`), stored in the free space of the thread's TCB page past its FP
  register file (`thread::capability_table_of`), no longer a field of `Thread`. Outside the struct
  is the soundness argument, written at the type: another core holding `IPC_TABLES` may hold a
  `&mut Thread` for the running thread, and a `&mut` covers every byte of the struct.
- The running thread reaches its table through a per-core pointer (`CURRENT_CAPABILITIES`), set
  only by `set_current` beside `PerCpu::current`. `current_cap`, `grant`, `grant_at` and
  `delete_current_cap` take that table's lock and nothing else. On x86_64 a deleted `PortRange`
  still takes `IPC_TABLES`, to clear the cached grant under it.
- The order is written at the rank and enforced by the ranking check. Anything touching another
  thread's table holds `IPC_TABLES` and takes the table under it (60 then 57). A table holder takes
  nothing, and two tables are never held at once.
- Why the pointer cannot dangle: a running thread's page is recycled only by `Threads::remove`,
  reached from the successor's reap and from region teardown, which refuses `Running` threads and
  threads a core stands on. The argument is written at `CURRENT_CAPABILITIES`. No new loom model:
  the pointer adds no protocol, it rides the switch-out handshake loom already searches.
- `sync::lock_found` takes a lock found through per-core state with one interrupt mask, and
  `sched::deliver_capability` is one out-of-line copy of the IPC delivery insert. Both exist to keep
  the fast paths inside their bands (below).

No syscall semantics, ABI or wire format changed. Names `CAPABILITY_TABLE` (rank),
`CapabilityTableLock`, `capability_table_of`, `init_capability_table`, `CURRENT_CAPABILITIES`,
`set_current`, `current_capabilities`, `lock_found`, `deliver_capability`,
`get_mut_with_capabilities`, `iter_mut_with_capabilities` and `capabilities` are provisional.

## What QEMU said

riscv64 TCG, `script/job-mix --arch riscv64 --smp 4 --release --lock-wait`, two runs each side,
interleaved on the same host:

| At 4 tasks | before | after |
|---|---|---|
| `current_cap` calls that found their lock held | 92,286 and 99,195 of 480,000 (19 to 21%) | 17 and 25 |
| `IPC_TABLES` contended acquisitions | 361,138 and 392,402 | 259,777 and 269,260 (down 29%) |
| rank 57 (`capability_table`) contended | (no such lock) | 30 and 42 |

The counts are the result TCG can give. Its ticks cannot: the host carried other lanes' builds, and
`null_syscall` at four tasks read 1,066 to 1,861 across six runs a side with no separation, which is
the null-syscall note's own caveat. The size of the win is radon's to say.

## Gates

- Kernel suite, riscv64, local: 135 kernel and 284 system tests passed (4 skipped). CI runs all three.
- `script/fastpath-footprint`: within band on all three ISAs (`ipc_call_reply` +1.7% aarch64,
  +1.5% riscv64, +0.05% x86_64). The first cut was over on two, and `current_cap` had grown from 256
  to 584 bytes on riscv64 (a double interrupt mask, and a result LLVM copied byte by byte); the
  second commit is that fix.
- `script/stack-frame-check`, `script/icount` (riscv64) and `script/bench --check` (aarch64, riscv64)
  pass. Against the pre-change tree on riscv64 icount, `yield_switch` is +3.5% and `call_reply` +4.1%
  (the per-switch pointer store, in the bench's debug kernel), `ipc_rtt_el0` -0.6%.
- Five falsification patches re-cut against the moved code and replayed red as predicted.

## Outstanding: the radon evening

Two payloads are built from `74bbb19a1`, the same shapes as the diagnosis evening. The procedure and
the thresholds, written before any boot, are in the null-syscall note's section on this milestone.

## BUGS

- A capability read and a capability granted are two critical sections, so a revocation sweep
  can fall between them. Take a syscall that derives: it reads a capability with `current_cap`,
  then `grant`s a narrowed copy. A sweep in between deletes the source and the copy is still filed. Unverified, found by reading while proving this milestone's linearizability argument, and
  not new: with one global lock the two steps were already two acquisitions. This milestone neither
  widens nor narrows the window. It lives here until somebody drives a sweep into that gap.

- `current_capabilities` returns a guard typed `'static` to a table that lives as long as its
  thread. Sound because no caller blocks or switches while holding it, and private to `sched`; a
  caller that held it across `schedule()` would be the bug.
- The `site=current_cap` counter now measures the thread's own table lock. Its meaning is the same
  (how often the cheapest syscall waited), and its rank in the per-rank lines moved from 60 to 57.

## Follow-on

- **Outstanding.** The radon evening: five boots of the two payloads, read against the thresholds in
  `notes/job-mix/null-syscall-under-load.md`. Checked 2026-10-04 (UTC): both payloads exist under
  the lane's worktree, built from `74bbb19a1`; no boot has run.
- **Milestone 17.** The IPC and scheduler rows (`schedule`, `CALL`, `REPLY`, `finish_switch`) stay on
  `IPC_TABLES`; splitting the thread table is milestone 17 (multikernel-leaning scheduler)'s question.
- **Recorded.** `PERCPU` straddling cache lines, in `notes/job-mix/null-syscall-under-load.md`'s BUGS,
  unchanged here so the radon comparison moves one thing.
- **Recorded.** The derive-then-grant window and the `'static` guard, in this block's BUGS.
