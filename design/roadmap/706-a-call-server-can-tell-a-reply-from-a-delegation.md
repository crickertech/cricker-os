---
status: NOT-STARTED
raised: 2026-10-03
promoted_from: a-call-server-can-tell-a-reply-from-a-delegation
milestone_dependencies: none
decision_dependencies: unwritten
machine_requirements: none
specific_machine: none
needs_person: yes
---
# 706. A `CALL` server can tell a Reply from a delegation

Promoted from `design/roadmap/proposals/a-call-server-can-tell-a-reply-from-a-delegation.md` on 2026-10-03 (UTC). The number 706 was minted by the maintainer in a batch promotion of the proposal pile and is provisional until the queue lands it. *(Title and slug are drafts.)*

<!-- writing-standards: exception. Granted 2026-10-03 (UTC) by the maintainer minting this milestone, not ratified by an architect. Reason: this block was promoted unedited from design/roadmap/proposals/, which the prose scope excludes, so it meets the sentence and bold limits only after an edit that promotion does not make. Trimming it is a separate pass, and the exception goes when it is done. -->

Raised by the 2026-10-03 security audit's follow-up (item (b) of
`design/audit-reports/2026-10-03-eight-constants-and-thirteen-components.md`'s reconciliation).
It is on the syscall surface, so it is calef's call; this file is the options with their costs.
It is also an input to fatal risk 7 (the confinement claim), and this proposal does not touch
`design/fatal-risks/`.

## The gap milestone 634 left, by its own account

Milestone 634 (a plain SEND received by RECV_CAP never hands the receiver a sender-chosen slot)
made `x1` on `RECV_CAP` `NO_CAP` unless the kernel installed a capability. Its block records what
it did not close: no `RECV_CAP` consumer in the tree checks the kind of object in the slot it
received, and the ABI has no call that would let one. So the guard every `CALL` server has is
`x1 != NO_CAP`, and a *real* capability passes it.

A client holding `WRITE` on a server's endpoint (any client) and `GRANT` on any capability of
its own (a rendezvous it retyped from its budget is enough) can `SEND_CAP` that capability to the
server. `ipc_recv_cap` installs it and returns its slot in `x1`. The server, written for `CALL`,
does its work and invokes `reply(x1, r0, r1)`, which is method `0` on whatever object sits there.

What method `0` is, per object type (`crates/abi`): `SEND` on a rendezvous, `SIGNAL` on a
notification, `ARM` on a timer, `MAP` on a memory region or page frame, `MAP_INTO` on an address
space, `CONFIGURE` on a thread control block, `WAIT` on an interrupt. Two of those block. A
rendezvous the attacker never receives on parks the server in `SEND` for the life of the machine;
every client of that server then hangs. The servers reachable this way are the ones every
program talks to: `net_stack` (`components/src/net_stack.rs`, which guards `NO_CAP` and nothing
else), the compositor (`components/src/compositor.rs`, `reply(reply_slot, ...)` with no guard) and
the file service.

The second cost is quieter. A Reply is one-shot and the kernel deletes it on use; every other
object stays in the server's table after the `reply` fails or returns, so each delivery that is
not a Reply leaks a slot, and the table is 32 slots. Twenty-odd messages and the server can no
longer receive a capability at all, which is the same denial by a slower road.

Severity: medium. Not demonstrated under QEMU; read from `ipc_recv_cap` (`kernel/src/sched.rs`),
the `SEND_CAP` arm's rights checks (`kernel/src/syscall.rs`) and the servers' reply sites. The
demonstration is the first step of whichever option is taken, as a red test in
`system_tests/src/user/recv_cap_attack_tests.rs`, which already holds the two 634 reduced to.

## The options, with what each costs

1. **A kernel-written kind tag in `x4`.** `RECV_CAP` already uses `x4` for one kernel-only
   word, `notification::BOUND`. Add `rendezvous::REPLY_DELIVERED`, written by `ipc_call_badged`'s
   delivery path only, and every `CALL` server checks it before `reply`. Cost: one constant, two
   stores in `sched.rs`, and a guard in each server (five or six sites). Reversible: a server that
   does not check behaves as today. The wart: the check is the server's to remember, which is rung
   three.
2. **An endpoint that refuses `SEND_CAP`.** A rendezvous attribute the server sets at creation
   (or a right the spawner withholds) so the kernel refuses a delegation into a `CALL`-only
   endpoint at the `SEND_CAP` arm, the shape the interrupt endpoints of milestone 603 (an
   interrupt's endpoint refuses every send) took for `SEND`.
   Cost: a new attribute on the object or a new rights bit, which is a wire-format change, and
   every server's creation site. Strongest: the wrong state is unrepresentable for a server that
   opts in. Irreversible in the rights word.
3. **`reply` refuses a non-Reply.** `abi::reply::REPLY` stops sharing number `0` with every other
   type's first method, or the `Reply` arm becomes the only arm that answers that number. Cost: a
   renumbering of one method, which every program is written against, and it only closes the
   blocking half; the slot leak stays. Refused on its own: the fix is at the receive, not the
   reply.

Recommendation: option 1 now, because it is one patch for all three ISAs in shared scheduler
code, it is the shape the tree already took for `BOUND`, and it is reversible; option 2 as the
rung-one answer when a server asks for it. What option 1 costs that option 2 does not is the
guard in each server, and the audit lane that lands it should add the guard to every `reply`
site it can find in the same change.

## What is blocked until it lands

Nothing builds on this. What is open is a denial of service on any `CALL` server by any of its
clients, recorded in `crates/abi`'s `RECV_CAP` BUGS where the next server author meets it.

## Index row

After milestone 634 no `RECV_CAP` consumer checks the kind of object in the slot it received, and the ABI has no call that would let one. Proposed: a way for a `CALL` server to tell a Reply from a delegation; a syscall surface change, options with costs.
