---
status: PROPOSED
raised: 2026-10-07
milestone_dependencies: 261
decision_dependencies: none
machine_requirements: x86_64 silicon with VT-d and an NVMe drive
specific_machine: xenon (the Micron 2450 the first comparison was taken on)
needs_person: yes
---
# The same storage benchmark on nife and Linux, by porting real fio

Written by an agent, 2026-10-07 (UTC), at calef's request, from fatal risk 6's GREEN verdict
(`design/fatal-risks/6-a-confined-driver-is-too-slow.md`) and milestone 261 (the NVMe driver leaves
the kernel). calef, 2026-10-07: *"Of course we should port fio. That makes perfect sense."*

**In brief.** Port real `fio` to nife and run the same binary and the same job file on both systems.
Risk 6 went GREEN on one pair of numbers: nife's confined driver at 4 KiB, queue depth 1, against a
Linux `fio` run photographed the same evening. The nife read was 2.16x Linux's, which nobody can
explain, and the figures were hand-transcribed. The port serves two risks at once: risk 6 (a
comparison nobody has to take on trust) and risk 1 (only software written for nife runs on nife),
because a real third-party program would then run on nife.

## What it builds

- One reference job file. `randread`, `randwrite`, `read` and `write`; 4 KiB and 128 KiB; queue
  depth 1, 4 and 32; `direct`; a fixed offset and size inside the window the confined driver owns.
- Linux side. `fio` with `io_uring`, polled and interrupt-driven. Also SPDK's `spdk_nvme_perf`,
  the like-for-like design: it is Linux's polled userspace NVMe driver, which is what nife's server is.
- nife side. The same `fio` source, built for nife and run against the confined NVMe server, reading
  the same job file and emitting the same JSON.
- A per-I/O latency histogram on each OS. This is the part that explains the read gap. A mean
  and a median cannot say whether nife's reads are fast or Linux's are slow in a tail.

## What the port needs from nife, to be discovered

These are questions the port answers, not assumptions. Each is a dependency found by trying.

- Threads. `fio` runs jobs as processes or threads and times them with its own clock threads.
  Whether nife's thread and timer surface covers what fio uses is not known.
- Which `ioengine` maps to the confined driver. `fio` has no engine for a block server reached over
  an endpoint. The candidates are `psync` over a file-like layer on `blk`, or a new engine written for
  nife's block contract. Which one measures the driver rather than the layer above it is the open question.
- Direct I/O and a block-device view. `direct=1` and an offset window have to mean the same on
  both sides, or the two numbers are not comparable.
- What std and libc fio's build wants that nife lacks, found by the first compile.

## What it must answer

Why the 2026-10-04 read was 2.16x Linux's (272 MB/s against 126 MB/s) when polled Linux reads were no
faster than interrupt-driven ones. Either nife's number is wrong, which the histogram will show, or
the gap is real and has a cause worth knowing. Queue depths 4 and 32 are where the comparison stops
being one command in flight and "real speed" can be claimed beyond depth 1.

Reuse: `fio` itself is taken, ported rather than rewritten, and `spdk_nvme_perf` is taken as is for
the Linux side. No nife-side benchmark program is written, so there is no shim to keep.
