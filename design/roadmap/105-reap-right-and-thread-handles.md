---
status: BUILT
raised: 2026-08-04
built: 2026-10-05
---
# 105. The two forks milestone 22 named and left

Raised 2026-08-04. Milestone 22 (trusted init) built the supervision tree and, in
`notes/trusted-init.md`, recorded two questions it deliberately did not answer, both marked
"calef's call, not a thing to slip in". This block stated them precisely enough to be decided and
picked neither. Both are answered now, and the one that needed building is built. §164 (whether
the kernel resolves a tid it already sent), written up 2026-09-19 by milestone 435 (forty-five
milestones are gated on a decision nobody wrote down)'s lane, asked fork two again and is
superseded by §148.

## Fork one: a reap-only right, answered without one

The fact was that `Untyped::DESTROY` and `RETYPE` both need `WRITE`, so a root supervisor able to
restart a dead tier-one server would be one able to build processes.

Answered twice over, and neither answer is a rights bit.

- §32 (a supervisor may collect a corpse without being able to build one)
  ([file](../decisions/32-reap-without-build.md)) was decided 2026-07-29, six days before this
  block was raised. It made reaping a method on the supervision endpoint, `abi::rendezvous::REAP`, authorized by the supervision relationship
  rather than by any right on the region. This block restated a settled question as open, which is
  recorded here rather than quietly fixed.
- §148 (a supervisor restarts by asking, and resolves by asking the kernel)
  ([file](../decisions/148-reap-and-thread-identity.md)), 2026-09-05, closed the rest: a tier-one server that should be
  restartable gets a one-program spawner, as tier two already does, and the root asks it. A bit says
  *may reap*; a one-program spawner says *may only ever produce this*. A supervision endpoint and a
  spawner land together, per server, or neither does.

Nothing was built for fork one, because it is a ruling about a pattern. See Follow-on.

## Fork two: which child died, answered by §148 as amended, and built

The fact was that the death message names the dead thread by tid (§26 (the fault endpoint), item
5), and nothing turned a tid into something a builder holds, so a supervisor with several children
could not say which one died.

calef's ruling, 2026-10-04 (UTC), recorded as §148's amendment: *"R3, unless the IPC benchmarks
move."* The kernel keeps the badge on a child's supervision capability and delivers it, the label
the builder set, with the death message. No `RESOLVE` method, so there is nothing to probe with and
`REAP`'s single-refusal design is untouched.

### What was built (2026-10-05)

- **The kernel keeps the badge.** `START` used to match `Object::Rendezvous(ep, _)` on the reserved
  fault slot and drop the badge. It now records it as `Thread::fault_label` beside `fault_ep`, and
  consumes the slot exactly as before.
- **The label rides beside the mailbox, not in it.** Word 3 carries the fault address and word 4 is
  reserved for §26's resume protocol, so the label is a sixth word, in argument register 5: `x5`,
  `a5`, `r9`. The kernel writes it into the supervisor's saved user frame on the death path only.
  `sched::hand_over_label` does it, through the arch helper `exceptions::set_user_arg` on all three
  ISAs. It is called from both routes a death reaches a supervisor: at the rendezvous
  (`deliver_death`), and when a supervisor collects a parked corpse (`collected_without_serving`). `Thread::mailbox` and
  `fault_msg` stay five words, so ordinary IPC stores nothing new.
- **Userspace declares the register.** `user_mode_runtime`'s syscall primitive is now `invoke6`,
  with register 5 an output that goes in as zero, so a `RECEIVE` cannot clobber a value LLVM thought
  preserved and an unstamped message reads label `0`. `receive_fault` returns the label as its sixth
  word.
- **A supervisor with two children.** `spawner` stamps each child's supervision capability with a
  label the supervisor names in its build request (`rendezvous::BADGE` on its own copy, inserted,
  then deleted). `sub_server_supervisor` supervises two children on one endpoint
  (`supervision_protocol::SUB_SERVER_LABELS`, provisional name), one that crashes and one that
  finishes, and restarts by label alone.

### Proven

- `supervision_tests::a_supervisor_tells_two_dead_children_apart_by_label`: a user-mode supervisor
  reads the tid and the label of two deaths and each tid arrives with its own label, on both
  delivery paths. Falsification replayable: drop the badge at `START` and it fails at its own
  assertion with both labels `0`.
- `supervision_tests::a_child_can_neither_learn_nor_forge_its_label`: after `START` no capability
  the child holds carries its label, and a `SEND` through a capability badged with that very label
  reaches the supervisor with label `0`. Falsification replayable for the learn half (leave the slot
  unconsumed); the forge half is unfalsified by patch, and its record says why.
- `authority_tests::a_dead_sub_server_is_restarted_by_its_supervisor_not_by_init` now also checks
  labels: the real tree, `root_supervisor` down, restarts the crashing child as attempt 1 and runs
  the finishing one exactly once. It is not a test of its own because each tree run leaves about
  1,100 frames parked, and a third put the suite over its frame budget in CI.

These pass on aarch64, riscv64 and x86_64 under QEMU, as do the existing `supervision`,
`authority` and `reap` tests on aarch64.

### The benchmark condition, measured against the base commit

`script/fastpath-footprint` is where `ipc_send_receive` and `ipc_call_reply` are named, and both are
byte-identical to the base commit (`1b4e13ef3`) on all three ISAs: aarch64 4828 and 6084 bytes,
riscv64 4040 and 5048, x86_64 5303 and 6666. The R3 kernel code is off both closures.

`script/bench` (icount, deterministic, base against branch, each run three times with identical
results): `ipc_rtt` and `call_reply` unchanged on aarch64 and x86_64, `call_reply` unchanged on
riscv64. Three rows moved, all inside the 10% band: riscv64 `ipc_rtt` 174 to 176 (+1.15%), riscv64
`ipc_rtt_el0` 373 to 377 (+1.07%), aarch64 `ipc_rtt_el0` 2194 to 2233 (+1.78%). The riscv64
`ipc_rtt` move persists with the base kernel built against this branch's userspace, so it is not the
kernel IPC path. Removing the new `Thread` field changed nothing; removing the zeroing `mov` took
riscv64 `ipc_rtt_el0` back by 2 ticks and aarch64 by none. Those are layout and userspace effects,
reported rather than explained further. Neither named benchmark left its band, so the ruling stands
at R3 and the R2 fallback was not reached.

Re-measured after merging main at `d0ca36c5b` (which moved each thread's capability table behind its
own lock). Both footprint closures are byte-identical to that main on all three ISAs. Against it,
`script/bench` moves only aarch64 `ipc_rtt_el0` (2200 to 2187) and riscv64 `ipc_rtt` (181 to 178),
both downward, which is what layout noise looks like when it is not a cost.

## Follow-on

- **Decision.** §148's amendment decides R3 and leaves the layout to the builder. The built
  semantics belong in `design/decisions/148-reap-and-thread-identity.md`, which a lane may not
  edit: register 5, written on the death path to a plain receive only, zeroed on the way in by the
  receiver, and zero for an unbadged capability. The text is in this lane's report for the
  maintainer, and notes/abi.md carries it meanwhile.
- **Recorded.** Fork one is a pattern, not a built thing: no tier-one server has a spawner today,
  and the first to get a supervision endpoint must get one in the same change. §148's own BUGS
  section records it.
- **Recorded.** The label reaches a plain `RECEIVE` only. A `RECEIVE_CAP` receiver does not get it,
  and neither does a kernel thread receiving in the kernel, which has no user frame. Both are stated
  where a reader meets the layout, in `notes/abi.md`.
- **Recorded.** The `std` overlay's `invoke` (`patches/std-nife/overlay/std/src/sys/pal/nife/rt.rs`)
  does not declare register 5. No `std` program receives a death message, which is the only
  delivery that writes it. The same `invoke` on aarch64 and riscv64 also declares `x1..x4` and
  `a1..a4` as inputs only, where the x86_64 twin makes them outputs. That is a latent clobber for
  any `RECEIVE`-shaped call through it; none exists today.

## Index row

A reap-only right split out of `WRITE`, and a way for a supervisor of several children to say which
one died. The first is answered by §32 and §148 without a right; the second is §148 as amended
(R3), built: the kernel keeps the badge on a child's supervision capability and hands it to the
supervisor beside the death message
