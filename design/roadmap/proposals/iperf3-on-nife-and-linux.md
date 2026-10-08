---
status: PROPOSED
raised: 2026-10-08
milestone_dependencies: 494, 53
decision_dependencies: none
machine_requirements: silicon with a NIC nife drives and a peer on the same wire, xenon or radon
specific_machine: none
needs_person: yes
---
# iperf3 on nife and Linux, throughput for the network drivers

Written by an agent, 2026-10-08 (UTC), at calef's request. Model: the fio proposal. Starts after a
NIC works on silicon, so it waits on milestones 494 (a driver for the network card a PC actually
has) and 53 (the board's own peripherals).

**In brief.** Run iperf3 as the server on nife and as the client on a peer, TCP and UDP, in both
directions, and do the same with Linux on the board.

## What it measures

Sustained TCP and UDP goodput, retransmits and CPU use over a wired link, single stream and
four parallel streams. It is the network twin of fio for risk 6.

## Which fatal risk it informs

Risk 6 (`6-a-confined-driver-is-too-slow.md`): the NIC driver and the stack are separate servers on
nife. Line rate, or a measured gap to Linux, is the NIC half of the confined-driver verdict that
fio gives for NVMe. A shortfall points at copies across the driver-stack-client boundary, which is
risk 4's currency.

## What nife must supply, as questions

- Does the stack offer a BSD socket surface (milestone 649, every client of a network stack shares its socket numbers) that iperf3's
  C code can use, or does the port need a shim? If a shim, it is part of what is measured.
- `select` or `poll`, threads (iperf3 3.16+ is multithreaded) and a timer fine enough for its interval reports?
- Does the stack have window scaling and enough buffer for a gigabit link, or does it cap lower?
- Is the peer a fixed Linux box? Its kernel is recorded too.

## The Linux comparison

The same iperf3 version, same peer, cable, switch, MTU and parameters (`-t 30 -P 1/4`, `-R`, `-u -b`),
Linux and nife booted on the same board in turn. Link speed is checked first so a 100 Mb
fallback is not read as slowness. Linux's NIC driver and offloads are recorded.

Reuse: iperf3 is taken and ported; no nife-side traffic generator is written.

## How it runs continuously

QEMU TCG wall-clock timings are meaningless, so nothing here gates on them. Where a host-side
harness allows, CI gates on deterministic instruction counts through `script/bench` (a pinned
icount baseline per workload step, `--check` failing on drift). The wall-clock runs happen on
silicon on a schedule, as the runbook does for the soak, and write dated rows to
`notes/benchmarks.md` with the Linux row beside them. A run that cannot reach the board writes
nothing rather than a TCG number.
