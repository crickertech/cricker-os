---
status: DECIDED
raised: 2026-10-04
decided: 2026-10-04
ratified_by: calef
---

# 246. A plain `RECEIVE` never takes a capability, whichever side reaches the rendezvous first

*Section number PROVISIONAL: the maintainer mints it at merge. Written 2026-10-04 (UTC) by the lane
on PR #1611 as the maintainer's delegate. The finding, the seven questions and the measured cost are
in the block of
[milestone 752 (a seeded syscall driver with a shadow model)](../roadmap/752-a-seeded-syscall-driver-with-a-shadow-model.md),
section "A real finding".*

calef, 2026-10-04 (UTC), ruling option A of that section.

## The ruling

A plain `RECEIVE` (`abi::rendezvous::RECEIVE`) never installs a capability in the receiver's table.
The outcome no longer depends on arrival order:

- **A `SEND_CAP` reaching a plain `RECEIVE`** delivers its data word (`x0`), `0` in `x1` and `x2`,
  and the sender's badge in `x3`. The capability copy is dropped; the sender keeps its own, since
  `SEND_CAP` narrows a copy and never moves the source. The sender's `SEND_CAP` returns `0`.
- **A `CALL` reaching a plain `RECEIVE`** delivers its two words (`x0`, `x1`) and the badge, and the
  caller's `CALL` returns `Gone`. The one-shot Reply lives in the receiver's capability table
  (§12 (call/reply IPC)), and a plain `RECEIVE` names no slot it wants filled, so nobody could ever
  answer it.

`RECEIVE_CAP` is unchanged. No new method, no new syscall number, no new error: this narrows what
an existing method returns, within §10 (the capability-based microkernel process model).

## How the kernel knows

Each receive records at its park which kind it is (`Thread::receiving_cap`, name provisional):
`ipc_receive_cap` sets it beside its existing `cap_delivered` reset, `ipc_receive` clears it. The
two receiver-first arms read it: `ipc_send_cap` installs only for a `RECEIVE_CAP`, and
`ipc_call_badged` hands a plain receiver to an out-of-line `call_meets_plain_receive`. The
sender-first order already dropped a `SEND_CAP`'s copy (milestone 633 (an outside agent attacks the
confinement claim)); its `CALL` arm now releases the caller with `Gone` (`answer_caller_gone`)
instead of leaving it parked on a Reply that existed nowhere. All of it is shared scheduler code,
so one change covers aarch64, riscv64 and x86_64.

## Why

Before this, a receiver that parked first had the capability installed and its slot returned in
`x1`; one that arrived second did not. Every plain-`RECEIVE` server in the tree (swish draining a
child's output, `wc`, `mdr`, the terminal sink) ignores `x1`, so each delegation that found one
parked cost it one of its 32 slots for good. A std program, including an unvouched one, can retype
its own region into a frame carrying `GRANT` and `SEND_CAP` it to its output, so a confined program
could fill the shell's table. Fatal risk 7.

seL4 has the receiver opt in: a capability is transferred only into the slot the receiver names,
and the message arrives regardless (Reference Manual, `manual/parts/ipc.tex` at seL4 commit
`c6ce4d2a0c`, "Capability Transfer"). Its Reply lives outside the capability space, so a plain
`seL4_Recv` can answer a `Call`; nife's lives in the table, so `Gone` is the honest answer.

## What else was considered

From milestone 752's block, in one line each. (B) a plain `RECEIVE` always takes the capability:
refused, a receiver that did not ask gains authority. (C) refuse a `SEND_CAP` at a plain receiver:
refused, the sender-first order has already parked and cannot be refused, so the orders would still
differ. (D) seL4's sender-side gate, `GRANT` on the endpoint capability itself: a later rung, not a
substitute. (E) leave it recorded: refused, it was exploitable.

## What it costs, measured

`script/fastpath-footprint` against the parent commit, same machine: `ipc_call_reply` aarch64
7,044 to 7,104 B (+0.9%), riscv64 6,106 to 6,138 (+0.5%), x86_64 8,227 to 8,291 (+0.8%);
`ipc_send_receive` aarch64 5,692 to 5,768 (+1.3%), riscv64 4,988 to 5,084 (+1.9%), x86_64 6,588 to
6,636 (+0.7%). All inside the 5% band. The block's throwaway measurement of the receiver-first half
gave `call_reply` +0.02% under icount and `ipc_rtt` +1.4%.

## Tests

`system_tests::user::receive_cap_attack_tests::a_send_cap_to_a_plain_receive_installs_nothing_whichever_side_parks_first`
and `..::a_call_to_a_plain_receive_is_answered_gone_whichever_side_parks_first`, each with a
replayable falsification; and the seeded driver's model (`syscall_fuzzer_tests`,
`Model::plain_receive`).

## What is still open

- (D), the sender-side gate, is not built. Today any holder of `WRITE` on an endpoint and `GRANT` on
  anything may `SEND_CAP` there; what changed is only that a plain receiver no longer keeps it.
- §245 (a `CALL` server tells a Reply from a delegation)'s "still open" note about a plain-`RECEIVE`
  `CALL` server seeing the tag on one order only is answered by this section: neither order delivers
  a Reply now.
