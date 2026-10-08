---
status: PROPOSED
raised: 2026-10-07
milestone_dependencies: 261
decision_dependencies: none
machine_requirements: x86_64 silicon with VT-d and an NVMe drive
specific_machine: xenon (the Micron 2450 the first comparison was taken on)
needs_person: yes
---
# The same storage benchmark on nife and Linux

Written by an agent, 2026-10-07 (UTC), at calef's request, from fatal risk 6's GREEN verdict
(`design/fatal-risks/6-a-confined-driver-is-too-slow.md`) and milestone 261 (the NVMe driver leaves
the kernel).

**In brief.** Risk 6 went GREEN on one pair of numbers: nife's confined driver at 4 KiB, queue depth
1, against a Linux `fio` run photographed on the same evening. The nife read was 2.16x Linux's, which
nobody can explain, and the figures were hand-transcribed. This milestone replaces that comparison with
one job file run on both systems and read by one script.

## What it builds

- **One reference job file.** `randread`, `randwrite`, `read` and `write`; 4 KiB and 128 KiB; queue
  depth 1, 4 and 32; `direct`; a fixed offset and size inside the window the confined driver owns.
- **Linux side.** `fio` with `io_uring`, polled and interrupt-driven. Also SPDK's `spdk_nvme_perf`,
  the like-for-like design: it is Linux's polled userspace NVMe driver, which is what nife's server is.
- **nife side, first.** A small program that reads the subset of the job file nife's server can
  serve and emits fio's JSON output, so one script compares both systems.
- **nife side, later.** A port of real `fio`. It would also be evidence for risk 1 (only software
  written for nife runs on nife).
- **A per-I/O latency histogram on each OS.** This is the part that explains the read gap. A mean
  and a median cannot say whether nife's reads are fast or Linux's are slow in a tail.

## What it must answer

Why the 2026-10-04 read was 2.16x Linux's (272 MB/s against 126 MB/s) when polled Linux reads were no
faster than interrupt-driven ones. Either nife's number is wrong, which the histogram will show, or
the gap is real and has a cause worth knowing. Queue depths 4 and 32 are where the comparison stops
being one-command-in-flight and "real speed" can be claimed beyond depth 1.

Reuse: `fio` and SPDK's `spdk_nvme_perf` are taken as they are for the Linux side. The nife-side
emitter is new because fio's engines need a POSIX I/O layer nife does not have; the port of fio is
the later step that removes that gap.
