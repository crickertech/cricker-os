---
status: NOT-STARTED
raised: 2026-10-08
milestone_dependencies: 261
decision_dependencies: 262, unwritten
machine_requirements: x86_64 silicon with VT-d and an NVMe drive
specific_machine: xenon (the Micron 2450 that risk 6's first comparison was taken on)
needs_person: yes
---
# 834. ioping on nife and Linux, storage latency one request at a time

*(Minted 2026-10-08 (UTC) by lane/standard-benchmarks under §262 (nife is measured with the field's
standard benchmarks). The number is provisional until the merge queue lands it; the title and slug
are drafts.)*

Written by an agent, 2026-10-08 (UTC), from calef's lean the same day: *"I'm leaning ioping now ...
ioping beats SQLite to move our risks, but SQLite is way more useful."*

In brief. Port unmodified `ioping` and run it on nife and on Linux on xenon, against the same files,
with real logs, repeated passes and a latency for every request. It is the lightest standard storage
benchmark there is: one process, one thread, one request in flight. So it can run before threads
exist, which milestone 833 (real fio) cannot.

## What it measures

ioping issues one I/O at a time, like `ping`, and prints the latency of each one, then the minimum,
mean, maximum and deviation. With direct I/O it times the driver and the device rather than a
cache. Request size, count and read or write are flags, so a run can match risk 6's first
comparison: 4 KiB, queue depth 1, a fixed window on the Micron 2450.

## Which fatal risks it informs

Risk 6 (a confined driver at real speed). calef put it back to amber on 2026-10-08 because nife's
read was 2.16x Linux's and nobody can explain it. Those figures came from one pass per boot,
transcribed from photographs. A latency per request, from logs on both systems, shows whether
nife's reads are fast or Linux's are slow, and in which part of the distribution.

Risk 1 (only software written for nife runs on nife): a third-party C program running unmodified.

## What nife must supply, as questions

- What the target is. ioping takes a file, a directory or a device. "The same files" means the same
  size at the same offset inside the window the confined driver owns. Is that a file on nife's
  filesystem, or the block window itself? Linux gets the same choice, and both sides make the same
  one.
- What direct I/O means on nife. The block server keeps no cache, but the filesystem server may. If
  the file path goes through a cache, the run uses the block window instead, and the row says so.
- A clock with microsecond resolution or better, reachable from C.

## The Linux comparison

The same ioping release, pinned by tag. The same flags on both sides: direct I/O, 4 KiB, a fixed
count, reads and then writes. The same disk and window as `bench/xenon-2026-10-04/`. Linux runs from
the Fedora live USB used on 2026-10-04 or a pinned successor, and its kernel version, I/O scheduler
and polling mode are recorded. Each system boots at least three times and runs at least five passes
per boot. Every pass is reported, not only a median.

The logs are committed whole under `bench/xenon-<date>/`, as the program printed them. Nothing is
transcribed by hand.

## What it waits on

ioping is POSIX C, and nife has no C library that runs one unmodified. §31 (the foreign-language
seam) lets C make no syscalls, and full POSIX is milestone 478 (tier three: full POSIX behind the
foreign-language seam), refused until a component needs it. Whether §262 makes these programs that
component is calef's call, and nobody has written that question up, so this block carries
`decision_dependencies: unwritten`.

It needs nothing else that is missing. From memory, to be checked against the pinned source, ioping
uses `open`, `pread` and `pwrite`, `clock_gettime` and `getopt`, with no threads and no `fork`. Of
the benchmarks §262 lists, it asks the least of nife.

Reuse: ioping is taken unmodified; no nife-side benchmark program is written.

## How it runs continuously

QEMU TCG wall-clock timings are meaningless, so nothing here gates on them. Where a host-side
harness allows, CI gates on deterministic instruction counts through `script/bench` (a pinned
icount baseline per workload step, `--check` failing on drift). The wall-clock runs happen on
silicon, and write dated rows to `notes/benchmarks.md` with the Linux row beside them. A run that
cannot reach the board writes nothing rather than a TCG number.

## BUGS

- ioping is a common operator's tool, but papers cite it far less often than fio. It is here
  because it moves risk 6 soonest, not because it carries the most weight. Milestone 833 is the
  comparison with weight behind it.
- Queue depth 1 only. "Real speed" beyond one request in flight is milestone 833's question.

## Index row

The lightest standard storage benchmark, unmodified on nife and Linux on xenon with real logs and a
latency per request, so risk 6's unexplained 2.16x read is explained or corrected before fio can
run. Waits only on a C library.
