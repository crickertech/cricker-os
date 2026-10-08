---
status: PROPOSED
raised: 2026-10-08
milestone_dependencies: none
decision_dependencies: none
machine_requirements: silicon, radon, xenon, later argon
specific_machine: none
needs_person: yes
---
# STREAM on nife and Linux, the memory-bandwidth baseline

Written by an agent, 2026-10-08 (UTC), at calef's request. Model: the fio proposal. It pairs with
`coremark`, the compute control in `notes/benchmarks.md` that must never move.

**In brief.** Port STREAM (one C file) for the Copy, Scale, Add and Triad bandwidth numbers. If nife
and Linux disagree on a memory-bound loop that makes no syscalls, the cause is in page mappings,
page size or cache attributes, and nothing a scheduler or IPC change can explain.

## What it measures

Sustained memory bandwidth in MB/s over arrays far larger than cache, single-threaded and, once
threads exist, with one thread per core.

## Which fatal risk it informs

Risk 4 (the per-crossing cost) as a control: any cost per crossing is only believable when the
workload with no crossings matches Linux. A mismatch here would be a mapping or cache-attribute
defect (memory type, huge-page use) that contaminates every other benchmark on the page. It is a
baseline rather than a risk test.

## What nife must supply, as questions

- Does a program get memory with normal cacheable attributes, and in what page size? Linux uses
  transparent huge pages when enabled; record both and run Linux with THP on and off.
- OpenMP is optional; the single-threaded build needs only `malloc` or a static array and a clock.
- A timer of `wtime` quality (microseconds or better). Is one reachable from std on nife?

## The Linux comparison

The same STREAM revision, `STREAM_ARRAY_SIZE` at least 4x the last-level cache, same compiler and
flags (`-O3`, no auto-parallel unless both sides have it), same board, best of ten iterations as the
program reports. Frequency scaling is pinned on Linux, and the clock and governor recorded.

Reuse: stream.c is taken unmodified except the timer shim.

## How it runs continuously

QEMU TCG wall-clock timings are meaningless, so nothing here gates on them. Where a host-side
harness allows, CI gates on deterministic instruction counts through `script/bench` (a pinned
icount baseline per workload step, `--check` failing on drift). The wall-clock runs happen on
silicon on a schedule, as the runbook does for the soak, and write dated rows to
`notes/benchmarks.md` with the Linux row beside them. A run that cannot reach the board writes
nothing rather than a TCG number.
