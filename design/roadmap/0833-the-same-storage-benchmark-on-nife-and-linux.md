---
status: NOT-STARTED
raised: 2026-10-07
promoted_from: the-same-storage-benchmark-on-nife-and-linux
milestone_dependencies: 261, 812
decision_dependencies: 262, unwritten
machine_requirements: x86_64 silicon with VT-d and an NVMe drive
specific_machine: xenon (the Micron 2450 the first comparison was taken on)
needs_person: yes
---
# 833. The same storage benchmark on nife and Linux, by porting real fio

*(Minted 2026-10-08 (UTC) by lane/standard-benchmarks from the proposal
`the-same-storage-benchmark-on-nife-and-linux`, under §262 (nife is measured with the field's
standard benchmarks). The number is provisional until the merge queue lands it; the title and slug
are drafts.)*

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

Why the 2026-10-04 read was 2.16x Linux's (272 MB/s against 126 MB/s) when polled Linux reads were
no
faster than interrupt-driven ones. Either nife's number is wrong, which the histogram will show, or
the gap is real and has a cause worth knowing. Queue depths 4 and 32 are where the comparison stops
being one command in flight and "real speed" can be claimed beyond depth 1.

Reuse: `fio` itself is taken, ported rather than rewritten, and `spdk_nvme_perf` is taken as is for
the Linux side. No nife-side benchmark program is written, so there is no shim to keep.

## What it waits on

Every program in this family is POSIX C, and nife has no C library that runs one unmodified. §31
(the foreign-language seam) lets C make no syscalls, and full POSIX is milestone 478 (tier three:
full POSIX behind the foreign-language seam), refused until a component needs it. Whether §262 makes
these programs that component is calef's call, and nobody has written that question up, so this
block carries `decision_dependencies: unwritten`.

Threads. fio times its jobs with its own threads even when jobs are processes. Threads are milestone
812 (`std::thread::spawn` runs real threads in one address space), option A of §105
(`std::thread::spawn` stays declined, until a customer needs it), reopened 2026-10-07 and not yet
scheduled. Until then, milestone 834 (ioping) answers risk 6's read gap at queue depth 1 without
threads.

## Index row

Real fio, one job file on nife and Linux, queue depths 1, 4 and 32 with per-I/O histograms, so risk
6's read gap is explained or corrected. Waits on a C library and threads.
