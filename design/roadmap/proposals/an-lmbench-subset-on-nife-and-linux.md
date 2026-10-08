---
status: PROPOSED
raised: 2026-10-08
milestone_dependencies: 25
decision_dependencies: none
machine_requirements: any of radon, xenon or (later) argon silicon; PMU not required
specific_machine: none
needs_person: yes
---
# An lmbench subset on nife and Linux, the real programs, not the same idea

Written by an agent, 2026-10-08 (UTC), at calef's request. Model: the fio proposal. Milestone 25
(cross-OS performance comparison) already measures primitives "the lmbench way" with its own EL0
programs and compares against lmbench numbers taken on Linux and macOS. This proposal is the
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
boundary. Using unmodified lmbench is what lets a stranger compare nife's number to any published one.

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

## How it runs continuously

QEMU TCG wall-clock timings are meaningless, so nothing here gates on them. Where a host-side
harness allows, CI gates on deterministic instruction counts through `script/bench` (a pinned
icount baseline per workload step, `--check` failing on drift). The wall-clock runs happen on
silicon on a schedule, as the runbook does for the soak, and write dated rows to
`notes/benchmarks.md` with the Linux row beside them. A run that cannot reach the board writes
nothing rather than a TCG number.
