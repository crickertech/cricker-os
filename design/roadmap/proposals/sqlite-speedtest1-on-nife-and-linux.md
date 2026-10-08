---
status: PROPOSED
raised: 2026-10-08
milestone_dependencies: none
decision_dependencies: none
machine_requirements: silicon for wall-clock rows, radon first, then xenon
specific_machine: none
needs_person: yes
---
# SQLite's speedtest1 on nife and Linux, a real database workload

Written by an agent, 2026-10-08 (UTC), at calef's request ("Yes" to one proposal per benchmark,
2026-10-07/08). Model: the fio proposal (`the-same-storage-benchmark-on-nife-and-linux`).

**In brief.** Port SQLite's `speedtest1` (one C file beside the amalgamation) and run it on nife and
Linux with identical parameters. It is the cheapest real database workload there is.

## What it measures

About 100 numbered SQL tests (inserts, indexed and unindexed selects, updates, joins, `VACUUM`),
each timed. It stresses the C library, the allocator, `mmap` or file I/O and the clock far more
than the CPU. A separate in-memory run (`:memory:`) isolates compute and allocator from storage.

## Which fatal risk it informs

Risk 1 (`design/fatal-risks/1-only-software-written-for-nife.md`): SQLite is the canonical C you
cannot rewrite (milestone 36, a foreign-language component, seam first, says so). A passing `speedtest1` is a third-party
program running unmodified on nife. Risk 4 (the per-crossing cost): each test's file I/O count is
known, so the time per crossing falls out.

## What nife must supply, as questions

- Does nife's libc surface (or the C toolchain path of milestone 36) build the amalgamation at all?
- Which VFS does SQLite get: unix over a file capability, or a small custom VFS over the fs server?
  Locking and `fsync` semantics decide whether the durable tests are comparable.
- Is there a monotonic clock fine enough for sub-millisecond tests, and `mmap` or a substitute?
- How much heap can a C program draw (some tests allocate tens of MB)?

## The Linux comparison

The same `speedtest1.c` and SQLite version, pinned by tag and amalgamation hash, same `--size`,
`--journal`, `--pagesize` and `--cachesize`, run on the same board against the same storage
(ext4 on the NVMe Linux sees, nife's filesystem on the same drive). Linux kernel, compiler and flags
are recorded beside nife's numbers. A durable run (`synchronous=FULL`) and a non-durable one are
reported separately, because nife's write path commits per request without a device flush unless
asked (see `notes/benchmarks.md`, "What is not apples to apples").

Reuse: SQLite itself is taken unmodified; no benchmark program is written.

## How it runs continuously

QEMU TCG wall-clock timings are meaningless, so nothing here gates on them. Where a host-side
harness allows, CI gates on deterministic instruction counts through `script/bench` (a pinned
icount baseline per workload step, `--check` failing on drift). The wall-clock runs happen on
silicon on a schedule, as the runbook does for the soak, and write dated rows to
`notes/benchmarks.md` with the Linux row beside them. A run that cannot reach the board writes
nothing rather than a TCG number.
