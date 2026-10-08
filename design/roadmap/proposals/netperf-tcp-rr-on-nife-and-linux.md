---
status: PROPOSED
raised: 2026-10-08
milestone_dependencies: 494, 53
decision_dependencies: none
machine_requirements: silicon with a NIC nife drives and a peer on the same wire, xenon or radon
specific_machine: none
needs_person: yes
---
# netperf TCP_RR on nife and Linux, request and response latency

Written by an agent, 2026-10-08 (UTC), at calef's request. Model: the fio proposal. Pairs with the
iperf3 proposal (`iperf3-on-nife-and-linux`); throughput hides per-packet cost and this exposes it.

**In brief.** Port `netperf` and `netserver` and run TCP_RR (and UDP_RR): a one-byte request, a
one-byte response, in a loop. The result is transactions per second, so its inverse is round-trip
latency.

## What it measures

One round trip through the whole path: client, stack, driver, wire, peer and back. With
a one-byte payload there is nothing to amortize, so the number is dominated by fixed cost per crossing.

## Which fatal risks it informs

Risk 4 (the per-crossing cost): a request and response cross driver, stack and client boundaries
each way; the count is known from the design, so the measured time divides into a cost per
crossing that can be set beside `ipc_rtt_el0`. Risk 6: a confined driver that adds a fixed delay
shows up here at once.

## What nife must supply, as questions

- The same socket surface questions as iperf3. Does netserver's fork-per-connection model need
  `fork`, or can it run with `-D` and one process?
- `TCP_NODELAY`, `SO_RCVBUF` and `getsockopt`: which does the stack implement? Nagle behavior
  changes the answer by an order of magnitude, so it must match on both sides.
- Interrupt or poll in the driver? Record it; it moves the number more than any other setting.

## The Linux comparison

The same netperf version and parameters (`-t TCP_RR -l 30 -- -r 1,1`), same peer and cable,
Linux and nife in turn on the same board, with `netserver` on the peer and CPU affinity recorded.
Linux's interrupt coalescing is set to the same mode as nife's driver, or both modes are reported.

Reuse: netperf is taken and ported.

## How it runs continuously

QEMU TCG wall-clock timings are meaningless, so nothing here gates on them. Where a host-side
harness allows, CI gates on deterministic instruction counts through `script/bench` (a pinned
icount baseline per workload step, `--check` failing on drift). The wall-clock runs happen on
silicon on a schedule, as the runbook does for the soak, and write dated rows to
`notes/benchmarks.md` with the Linux row beside them. A run that cannot reach the board writes
nothing rather than a TCG number.
