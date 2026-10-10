---
status: NOT-STARTED
raised: 2026-10-08
promoted_from: schbench-on-nife-and-linux
milestone_dependencies: 225, 812, 835
decision_dependencies: 262, 265
machine_requirements: silicon with 4 or more cores, radon first, then xenon, argon when the TX1 arrives
specific_machine: none
needs_person: yes
---
# 830. schbench on nife and Linux, wakeup-latency tails

*(Minted 2026-10-08 (UTC) by lane/standard-benchmarks from the proposal
`schbench-on-nife-and-linux`, under §262 (nife is measured with the field's standard benchmarks).
The number is provisional until the merge queue lands it; the title and slug are drafts.)*

Written by an agent, 2026-10-08 (UTC), at calef's request. Model: milestone 833 (the same storage
benchmark on nife and Linux, by porting real fio).

**In brief.** Port `schbench`, which reports the distribution of wakeup latency (p50, p99, p99.9)
for message threads waking worker threads under load.

## What it measures

The time from a wakeup being issued to the woken thread running, as percentiles, with a configurable
number of message and worker threads and a simulated compute and sleep per request. It reports tails
where `hackbench` reports a total.

## Which fatal risk it informs

Risk 5 (`5-not-reliable-on-multicore.md`): a wakeup that arrives very late, or never, is the
visible edge of a lost-wakeup or run-queue defect. A p99.9 that is 100x the median is a finding, and
a
stuck worker is a failure the soak (milestone 225 (run the soak on radon, argon and xenon)) would
also catch.

## What nife must supply, as questions

- pthreads, futex-like waits, `nanosleep` and `CLOCK_MONOTONIC` with microsecond resolution. Which of
  these exist, and does the timer fire accurately enough that the measurement is not the timer?
- Does nife's scheduler have per-core queues, so cross-core wakeups happen at all? If it has one
  queue, say so in the row; the comparison is then of designs.
- Does the port expose the kernel's own wakeup path, or a user-level condition variable over IPC?

## The Linux comparison

The same schbench version (commit pinned), same thread counts, runtime, `-C` and `-S` values, same
board and core count, the Linux scheduler (EEVDF or CFS) and `HZ` recorded. Percentiles from three
runs are reported separately, not averaged.

Reuse: schbench is taken and ported.

## What it waits on

Corrected 2026-10-10 (UTC). The C library this waited on exists. calef ruled the question on
2026-10-08 as §265 (a C library started from relibc), and milestone 835 (a C library, stage 1:
files, clock and memory) built its first stage in #1896. The fields now name 835 and §265 in place
of `unwritten`. This program's other needs are below.

Threads. schbench's message and worker tasks are pthreads. Threads are milestone 812
(`std::thread::spawn` runs real threads in one address space), option A of §105
(`std::thread::spawn` stays declined, until a customer needs it), reopened 2026-10-07 and not yet
scheduled.

## How it runs continuously

QEMU TCG wall-clock timings are meaningless, so nothing here gates on them. Where a host-side
harness allows, CI gates on deterministic instruction counts through `script/bench` (a pinned
icount baseline per workload step, `--check` failing on drift). The wall-clock runs happen on
silicon on a schedule, as the runbook does for the soak, and write dated rows to
`notes/benchmarks.md` with the Linux row beside them. A run that cannot reach the board writes
nothing rather than a TCG number.

## Index row

schbench wakeup-latency tails on the boards, the visible edge of a lost wakeup. Waits on a C library
and on threads (milestone 812).
