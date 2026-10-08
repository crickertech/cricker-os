---
status: NOT-STARTED
raised: 2026-10-08
promoted_from: hackbench-on-nife-and-linux
milestone_dependencies: 225, 812
decision_dependencies: 262, unwritten
machine_requirements: silicon with 4 or more cores, radon first, then xenon, argon when the TX1 arrives
specific_machine: none
needs_person: yes
---
# 827. hackbench on nife and Linux, as a scheduler benchmark and a multicore soak

*(Minted 2026-10-08 (UTC) by lane/standard-benchmarks from the proposal
`hackbench-on-nife-and-linux`, under §262 (nife is measured with the field's standard benchmarks).
The number is provisional until the merge queue lands it; the title and slug are drafts.)*

Written by an agent, 2026-10-08 (UTC), at calef's request. Model: milestone 833 (the same storage
benchmark on nife and Linux, by porting real fio).

**In brief.** Port `hackbench` (rt-tests; about 700 lines of C) and run it on the boards, once as a
benchmark and again as a long soak. Groups of sender and receiver tasks pass small messages over
sockets or pipes, so every message is a cross-task, often cross-core, handoff.

## What it measures

Wall time for G groups of 20 senders and 20 receivers to exchange N messages. It stresses the
scheduler, wakeups, IPC and cross-core migration together, with no useful compute.

## Which fatal risks it informs

Risk 4 (the per-crossing cost): total time over message count is a cost per handoff that is
comparable across OSes. Risk 5 (`5-not-reliable-on-multicore.md`): the load is thousands of tasks
with constant cross-core wakeups, which is the shape that exposes lost wakeups and races. Run for
hours it is a second soak workload beside milestone 225 (run the soak on radon, argon and xenon).
A hang or corrupted message count is a defect found, not a number.

## What nife must supply, as questions

- Processes or threads? hackbench has both modes. Which maps onto nife's spawn cost and capability
  grants, and does a group of 40 fit the capability-slot budget?
- Sockets or pipes? nife has neither as a Unix object. Is the port over IPC endpoints (a changed
  program) or over a pipe layer (a changed kernel path being measured)? Record which, and why.
- Is `fork` needed? If so the port has to substitute `spawn`, which Linux then must be asked to
  match, or the comparison is not like for like (see the spawn caveat in `notes/benchmarks.md`).

## The Linux comparison

The same hackbench revision, pinned by rt-tests tag, same group count, loops, message size and
mode, same board, with the Linux kernel version and scheduler (EEVDF or CFS) recorded. Three runs,
median and spread; core count fixed with `--smp` or `taskset` identically on both.

Reuse: hackbench is taken, ported and not rewritten.

## What it waits on

Every program in this family is POSIX C, and nife has no C library that runs one unmodified. §31
(the foreign-language seam) lets C make no syscalls, and full POSIX is milestone 478 (tier three:
full POSIX behind the foreign-language seam), refused until a component needs it. Whether §262 makes
these programs that component is calef's call, and nobody has written that question up, so this
block carries `decision_dependencies: unwritten`.

Threads or fork. hackbench's thread mode needs threads, and its process mode forks. Threads are
milestone 812 (`std::thread::spawn` runs real threads in one address space), option A of §105
(`std::thread::spawn` stays declined, until a customer needs it), reopened 2026-10-07 and not yet
scheduled. nife has no `fork` by design (§10 (process model: capability-based, microkernel)), so a
path that needs it never unblocks. The port runs the program's no-fork mode where one exists, and a
row that cannot run is recorded with its reason, as `notes/benchmarks.md` already does for spawn.

## How it runs continuously

QEMU TCG wall-clock timings are meaningless, so nothing here gates on them. Where a host-side
harness allows, CI gates on deterministic instruction counts through `script/bench` (a pinned
icount baseline per workload step, `--check` failing on drift). The wall-clock runs happen on
silicon on a schedule, as the runbook does for the soak, and write dated rows to
`notes/benchmarks.md` with the Linux row beside them. A run that cannot reach the board writes
nothing rather than a TCG number.

## Index row

hackbench on the boards, as a cost per cross-task handoff and as a second multicore soak. Waits on a
C library and on threads (milestone 812), since its other mode forks.
