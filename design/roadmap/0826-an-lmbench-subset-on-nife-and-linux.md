---
status: NOT-STARTED
raised: 2026-10-08
promoted_from: an-lmbench-subset-on-nife-and-linux
milestone_dependencies: 25
decision_dependencies: 262, unwritten
machine_requirements: any of radon, xenon or (later) argon silicon; PMU not required
specific_machine: none
needs_person: yes
---
# 826. An lmbench subset on nife and Linux, the real programs, not the same idea

*(Minted 2026-10-08 (UTC) by lane/standard-benchmarks from the proposal
`an-lmbench-subset-on-nife-and-linux`, under §262 (nife is measured with the field's standard
benchmarks). The number is provisional until the merge queue lands it; the title and slug are
drafts.)*

Written by an agent, 2026-10-08 (UTC), at calef's request. Model: milestone 833 (the same storage
benchmark on nife and Linux, by porting real fio). Milestone 25
(cross-OS performance comparison) already measures primitives "the lmbench way" with its own EL0
programs and compares against lmbench numbers taken on Linux and macOS. This milestone is the
step it left out: run the lmbench source itself on nife, not an imitation of it. It extends
milestone 25 and does not replace it.

**In brief.** Port only the lmbench programs that map onto nife's capability model, and write down
those that do not and why. The imitation measures what the author thought lmbench measured; the real
program measures what everyone else's published number measures.

## Candidates

| lmbench | measures | maps to nife? |
|---|---|---|
| `lat_syscall null` | a trap that does nothing | yes: `null_syscall` already exists |
| `lat_ctx` | context switch over pipes | partly: needs pipes or a pipe layer; today derived by subtraction |
| `lat_pipe` | byte round trip over a pipe | a question, below |
| `lat_proc fork+exit` | process creation | no: nife has `spawn`, no `fork`; record it |
| `lat_proc exec` | spawn a program | yes, as `spawn` plus run |
| `bw_mem`, `lat_mem_rd` | bandwidth and cache latency | yes, compute only |
| `lat_sig`, `lat_unix`, `lat_mmap` | signals, sockets, mmap | likely no; each gets a written reason |

## Which fatal risk it informs

Risk 4 (the per-crossing cost): a null trap, a switch and an IPC are the unit cost of crossing a
boundary. Using unmodified lmbench is what lets a stranger compare nife's number to any published
one.

## What nife must supply, as questions

- Does `fork` need a shim, or is its absence the finding? Faking it measures the shim.
- Are pipes an endpoint pair in the port? If so `lat_pipe` is an IPC benchmark under another name;
  record that in the row, as `notes/benchmarks.md` already does for `ctx_switch`.
- lmbench's timing harness (`gettimeofday`, `benchmp` with fork) needs a clock and a process
  layer. The harness may need a rewrite, in which case it is no longer "unmodified".

## The Linux comparison

lmbench 3.0-a9 or the pinned fork in use, same flags (`-P 1 -W 1 -N 5`), same board, single core
pinned identically, Linux version and governor recorded. The apples-to-oranges list in
`notes/benchmarks.md` grows by whichever rows needed a shim.

Reuse: lmbench is taken; the harness is the likeliest place a patch is needed.

## What it waits on

Every program in this family is POSIX C, and nife has no C library that runs one unmodified. §31
(the foreign-language seam) lets C make no syscalls, and full POSIX is milestone 478 (tier three:
full POSIX behind the foreign-language seam), refused until a component needs it. Whether §262 makes
these programs that component is calef's call, and nobody has written that question up, so this
block carries `decision_dependencies: unwritten`.

Fork. nife has no `fork` by design (§10 (process model: capability-based, microkernel)), so a path
that needs it never unblocks. The port runs the program's no-fork mode where one exists, and a row
that cannot run is recorded with its reason, as `notes/benchmarks.md` already does for spawn. Here
that is `lat_proc fork`, and, from memory and to be checked against the source, lmbench's own timing
harness (`benchmp`), which forks its measured children. If the harness needs a rewrite, the program
is no longer unmodified, and the row says so.

Milestone 25 (cross-OS performance comparison) is the base: its EL0 programs measure the same
primitives the lmbench way. This block runs the real source beside them. 25 also holds `sel4bench`,
the seL4 column, which waits on a PMU on silicon.

## How it runs continuously

QEMU TCG wall-clock timings are meaningless, so nothing here gates on them. Where a host-side
harness allows, CI gates on deterministic instruction counts through `script/bench` (a pinned
icount baseline per workload step, `--check` failing on drift). The wall-clock runs happen on
silicon on a schedule, as the runbook does for the soak, and write dated rows to
`notes/benchmarks.md` with the Linux row beside them. A run that cannot reach the board writes
nothing rather than a TCG number.

## Index row

The real lmbench, not an imitation, on nife and Linux: the primitives every OS paper reports, so a
stranger can set nife's crossing cost beside any published number. Waits on a C library, and its
fork rows never run here.
