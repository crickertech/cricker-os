---
status: NOT-STARTED
raised: 2026-09-04
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 366. A block site that writes `Blocked` by hand opts out of teardown, silently

Filed as a proposal on 2026-09-04 by the milestone 133 lane, from that
milestone's block; promoted by milestone 433 on 2026-09-19. Read against the tree that day and
nothing has moved: `crates/thread_wake_handshake/src/lib.rs` still declares `pub state: RunState`
and `pub wait_on: Option<W>` as two fields that must agree, `park` is still the only thing that
writes them together, and `sched::finish_blocked_resident` still opens with the `debug_assert!` that
pairs `Blocked` with a recorded wait, which is nothing at all on a release board. No `script/lint`
check names `RunState::Blocked`, and the grep the rung-two option asks for comes back clean today,
so the check would pass on the tree as it stands.

No decision is owed. It is a lint or a type change in
`crates/thread_wake_handshake` plus its callers in `kernel/src/sched.rs`, and it wants a lane rather
than a hotfix because the interesting half is deciding which rung of the ladder it can reach.

In brief. `Handshake::park` writes `state = Blocked` and `wait_on = Some(..)` in one statement,
and the pair is what every teardown path reads to find the queue a thread is linked on. The fields
are `pub`, because the kernel has legitimate out-of-protocol writers, so a future block site can
write `state = Blocked` directly and leave `wait_on` holding whatever the last wait left there.
`thread_wake_handshake`'s own BUGS says nothing prevents it. Make it impossible, or make it fail
loudly at the write rather than at the read.

## Why this matters more since milestone 133

Before that milestone the consequence of a stale `wait_on` was a diagnostic one: a hang dump saying
the wrong thing. `sched::finish_blocked_resident` now *acts* on it. It resolves the recorded
rendezvous name and unlinks the thread's TCB from that rendezvous's queues by pointer, and then the
region's reclaim frees the page the TCB sits on. If the name is stale, the unlink runs against the
wrong rendezvous, finds nothing, and the reclaim leaves a freed page still linked into a live wait
queue, which the next `receive` on that rendezvous follows. That is a use-after-free in the IPC path,
and it is the sharpest failure mode the research (notes/blocked-thread-teardown.md) named for every
one of its four proposals.

Milestone 133 (ending a permanently blocked thread, and deciding who) bought what a caller can buy on its own and no more. It asks both queues rather
than trusting the recorded `WaitRole`, which it must, because a `CALL` caller that met no server is
recorded `Reply` and is genuinely on the sender queue; and it carries a `debug_assert!` pairing
`Blocked` with a recorded wait. Neither defends the *name*, and a `debug_assert` is rung two on a
release board.

## What the options look like, so a lane starts from the fork rather than at it

- **Rung one, and it is the reason to look:** make the pair unrepresentable. `state: RunState`
  and `wait_on: Option<W>` are two fields that must agree; one field carrying `Blocked(W)` cannot
  disagree with itself. The cost is that `RunState` is loom-searched and matched on in many places,
  and the out-of-protocol writers the fields are public *for* would all have to be looked at, which
  is the work.
- **Rung two:** a `script/lint` check that no file outside `thread_wake_handshake` assigns
  `RunState::Blocked`. Cheap, greppable, and it fires without being remembered; it is also exactly
  the shape AGENTS.md prices at `git grep -w TODO`'s false-positive rate, so it wants checking
  against the real writes before it is written.
- **Rung two, narrower:** promote the `debug_assert!` in `finish_blocked_resident` to a real refusal,
  so a resident whose wait is unrecorded is refused rather than unlinked from a guess. That is the
  smallest honest change and could ship on its own, ahead of either of the above.

## Where it came from

Milestone 133's Follow-on, and before that `crates/thread_wake_handshake`'s own BUGS section and
notes/blocked-thread-teardown.md's, which names it as the failure mode proposals A, B and C all
inherit.

## Index row

`Handshake::park` writes `state = Blocked` and `wait_on = Some(..)` in one statement, and that pair
is what every teardown path reads to find the queue a thread is linked on. The fields are public
because the kernel has legitimate out-of-protocol writers, so a future block site can write
`Blocked` by hand and leave `wait_on` holding whatever the last wait left there. Since milestone 133
that is no longer a diagnostic problem: `sched::finish_blocked_resident` acts on the recorded name,
and a stale one unlinks the thread from the wrong rendezvous and leaves a freed page linked into a
live wait queue, which the next `receive` follows. The interesting half of the lane is deciding which
rung it can reach: one field carrying `Blocked(W)` cannot disagree with itself, but `RunState` is
loom-searched and matched on widely; a lint is cheap and is the shape AGENTS.md prices at a high
false-positive rate; and promoting the `debug_assert!` to a real refusal is the smallest honest
change and could ship ahead of either.
