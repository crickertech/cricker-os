---
status: NOT-STARTED
raised: 2026-09-04
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 371. A reply capability names a thread, so only a sweep keeps it from answering the wrong call

Filed as a proposal on 2026-09-04 by the milestone 254 lane, out of the
hazard that milestone shipped a mitigation for rather than a fix; promoted by milestone 433 on
2026-09-19. Read against the tree that day and unchanged: `kernel/src/cap.rs` still declares
`Reply(crate::thread::ThreadId)` and `reply_cap` still mints it from a thread name alone, so
`ipc_reply`'s guard can still only ask whether the thread is parked awaiting some reply. The
property continues to be held by `strand_reply_caller`'s sweep, which is what this file says: the
hazard is closed, and it is closed at rung two.

No decision is owed. It changes `Object::Reply`'s payload, which is kernel-internal
(the capability is minted by the kernel, never forged and never delegated, so no wire format and no
user program agrees on its shape), and it wants a lane that can also state the new invariant as a
Kani property.

**In brief.** `cap::reply_cap` mints `Object::Reply(tid)`: a generational thread name and
nothing else. No call sequence number, no rendezvous, no nonce. So `sched::ipc_reply`'s guard can
only ask *is this thread parked awaiting some reply*, not *is this thread awaiting this reply*,
and the rendezvous in `wait_on` is discarded with a `_`. Give the payload a call identity, and the
guard can ask the second question.

## Why it is worth doing, given that the hazard is closed

It is closed by a sweep, which is rung two of AGENTS.md's ladder, and a call identity is rung
one. Milestone 254 deletes every `Object::Reply(caller)` in the machine before it frees a stranded
caller, which is seL4's non-MCS answer (`cteDeleteOne(callerCap)`, reached from `cancelIPC`). That
is correct today because the sweep runs at all four places a server can stop being able to answer.
The property it maintains is *"no unconsumed reply capability names a thread that has left its
park"*, and nothing in the type system says so: a fifth way for a server to stop answering, added by
somebody who has not read `strand_reply_caller`, reopens the hazard silently.

With a call identity there is nothing to maintain. A stale capability names a call that is over, the
guard compares and refuses, and no sweep has to have been remembered.

**The hazard is not hypothetical and two systems document it.** L4Re warns that a partner may still
hold the temporary reply capability after a finite receive timeout and *"may use this capability to
reply to the caller at a later, unexpected time specifying an arbitrary IPC label"*. Zircon says the
same about `zx_channel_call` timing out: a later reply *"could match another outbound request"*.
Both quotes and their sources are in notes/blocked-thread-teardown.md.

And it removes a cost. The sweep is 128 x 16 comparisons per caller freed, on a teardown path
today. A deadline on `CALL` (milestone 106) would move that cost onto a timer, where it is a
different question.

## Sketch

`Object::Reply(ThreadId)` becomes `Object::Reply(ThreadId, CallId)`, where the call identity is a
counter on the caller's own `Thread`, bumped by `ipc_call` at each park. `cap::reply_cap` reads it;
`ipc_reply` compares it against the caller's current one and drops the reply if they differ, which is
the same disposition it already gives a caller that is not reply-parked at all. `strand_reply_caller`
then needs no sweep, and can bump instead.

**What it does not decide.** Whether the sweep should be kept anyway, on the argument that a
capability naming a finished call is garbage worth collecting even when it is harmless. That is a
judgment about capability-table pressure, and it is cheap either way.

## What is blocked until it is answered

Nothing. Milestone 254 shipped the mitigation, and the property holds today.

## Index row

`cap::reply_cap` mints `Object::Reply(tid)`, a generational thread name and nothing else, so
`sched::ipc_reply`'s guard can ask whether a thread is parked awaiting some reply and not whether it
is awaiting this one. Milestone 254 closed the hazard with a sweep that deletes every
`Object::Reply(caller)` in the machine before freeing a stranded caller, which is seL4's non-MCS
answer and is correct today because the sweep runs at all four places a server can stop being able
to answer. Nothing in the type system says so, and a fifth way to stop answering, added by somebody
who has not read `strand_reply_caller`, reopens it silently. A call identity in the payload makes a
stale capability name a call that is over, so the guard compares and refuses and no sweep has to be
remembered. L4Re and Zircon both document the hazard in their own words. It also removes 128 by 16
comparisons per caller freed from a teardown path. Nothing is blocked until it is answered.
