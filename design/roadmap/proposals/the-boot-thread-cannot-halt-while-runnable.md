---
status: PROPOSED
raised: 2026-10-03
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# The boot thread cannot halt while it is runnable

Raised 2026-10-03 (UTC) by the maintainer session writing
[the merge-rate correction](../../../notes/coes/2026-10-03-the-merge-rate.md). Name provisional,
this file's alone.

## The defect class

A thread that halts the core while it is still on the run queue costs a timer tick every time round
robin reaches it. On x86_64 the boot thread ended in `arch::halt()` after the hand-over, and the
polling input driver kept the rotation turning. The swish-check leg paid 7.7 s a line against
aarch64's 0.19 s, the long CI job doubled on 2026-09-19, and on 2026-09-30 it ran into its timeout
29 times. #1487, milestone 628 (the x86_64 swish-check leg costs what the others do), fixed that
call site by ending the thread in `sched::exit()`.

The other two architectures still park the same way. On origin/main at 9d75b4518 the riscv64
hand-over and the end of the aarch64 boot in `kernel/src/lib.rs` both call `arch::halt()` on the
boot thread, and #1487's `BUGS` (in `notes/benchmarks/swish-check-x86-leg.md`) says their cost is
unmeasured. Their legs cost 0.2 s a line, so the cost is small today (inferred: their input is
interrupt-driven, so the rotation rarely reaches the parked thread).

## What to build

1. Measure aarch64 and riscv64 first: the swish-check leg per line with the boot thread ending in
   `sched::exit()` against `arch::halt()`, one change at a time, as #1487 did.
2. End both boot threads in `sched::exit()`, so all three architectures leave the scheduler the
   same way. A parity difference here is the bug, per DECISIONS §19 (architectural parity is a
   tenet).
3. Make the wrong state unrepresentable. `arch::halt()` takes a zero-sized token (name provisional)
   that only the idle thread and the terminal paths (panic, a test build's exit, the bench and soak
   boots) can construct. A scheduled thread holding no token cannot call it; the compiler is the
   gate.

## Index row

The boot thread on aarch64 and riscv64 still halts while runnable, as x86_64's did until #1487; measure it, end it in `sched::exit()`, and give `arch::halt` a token only the idle thread and terminal paths hold.
