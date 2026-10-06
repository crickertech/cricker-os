---
status: PROPOSED
raised: 2026-10-06
milestone_dependencies: none
decision_dependencies: unwritten
machine_requirements: none
specific_machine: none
needs_person: no
---
# A program is asked to give memory back before anyone is refused

calef asked for this on 2026-10-06 (UTC), ruling fork 6 of #1777 (a running program acquires more
memory): "Approve Fork 6 A with both follow-ons". It is the "please shrink" notice #1777's fork 4
named. In his words from fork 3 of the same proposal: "Most of my professional work has involved
large programs with spikes in memory that would need to be returned." A writing-only lane wrote
it and built nothing.

It depends on #1777's `memory_broker`, its accounts, and reserve and commit (forks 1, 4 and 5 there,
ruled). Every name here is provisional: the signal, its levels, the runtime hook and the fixture.

## Why, and its honest rank

Under #1777, a program's memory comes back two ways. The allocator trims its band and frees large
regions on its own. Or the owner revokes, which kills. Neither reaches memory a program holds on
purpose: a garbage-collected heap sized for throughput, a database's buffer pool, a cache. That
memory is the bulk of a large program's footprint, and it is the part the program could give back
if asked. Without a signal, the next program's commit is refused while the memory it needed sits in
someone's cache.

On principle 1, this is behind #1777's first slice and milestones 801 (packages over the internet)
and 802 (the trivial install). It matters to the
first customer that runs a runtime with a collector or a cache, and to calef's own workloads. It
moves no fatal-risk verdict.

## What the tree has, read on 2026-10-06 (UTC)

- Notification objects: a word a sender ORs bits into and a waiter collects, bindable so a thread
  waiting on an endpoint also wakes for it (§101 (notification objects), `abi::notification`).
- #1777's broker counts committed memory per account, refuses a commit past the account's ceiling
  or the per-identity cap or when the pool is empty, and gives the owner a revoke.
- No runtime here has a way to be told anything about memory.

Reuse: notification objects are the tree's. The levels borrow from cgroups and Android (below).

## The forks

Four, in the order calef rules on them.

### 1. When the signal fires

| option | what it is | verdict |
|---|---|---|
| A | Two triggers: an account's soft limit (`high`) below its ceiling, and the pool's free pages falling under a watermark | recommended |
| B | The pool watermark only | refused |
| C | Only when a commit is about to be refused | refused |

A mirrors cgroups (from memory): `memory.high` throttles and reclaims a group before `memory.max`
refuses it. The account trigger keeps one program honest against its own ceiling. The pool trigger
asks everyone when the machine is short. B cannot tell a program it is near its own limit. C is
too late: by then a commit is waiting, and a program that frees memory in response returns it after
the refusal it could have prevented.

The soft limit defaults to 80% of an account's ceiling. The pool watermark defaults to 10% of the
pool. Both are guesses to measure, not results.

### 2. How it is delivered

| option | what it is | verdict |
|---|---|---|
| A | A notification object per account, given to the program when the account opens; the broker signals a level bit | recommended |
| B | A message on an endpoint the program serves | refused |
| C | An upcall that interrupts the program | refused |

A costs a program nothing until it looks, never blocks the broker, and coalesces: two signals
before the program wakes read as one. A program binds it to its main endpoint if it serves one,
or polls it from its allocator. B blocks the broker on a slow program. C needs a kernel mechanism
the tree does not have, and runs arbitrary code at an arbitrary point.

Two levels, set as bits in the word: moderate (give back what is cheap) and critical (give back
what you can). Android's `onTrimMemory` levels and Linux PSI's some and full are the precedent,
from memory.

### 3. Who may send it

| option | verdict |
|---|---|
| A. The broker only. The owner asks through the broker. | recommended |
| B. Any holder of the account | refused |

The notification capability the broker keeps is the only one with `SIGNAL`. A program holds it
with `WAIT` only. A peer cannot make another program shrink.

### 4. What a program does with it

| option | what it is | verdict |
|---|---|---|
| A | The runtime acts first: the allocator trims its band with no hysteresis and frees cached free increments. Then it calls hooks the program registered | recommended |
| B | Hooks only | refused |

A gives every program something for free, because #1777's band trimming keeps one free increment
in hand as hysteresis, and pressure is when to drop it. Hooks are for the memory the runtime cannot
see: a collector's heap target, a buffer pool. `user_mode_runtime` gains a hook registry, and
`std` gains one under `std::os::nife`, since Rust's `std` has no portable API for this. A
collector registers a hook that lowers its heap target and collects.

## What it never does

Ruled by #1777's fork 4 already, and restated so nobody builds it: the signal is advisory. It never
kills, never revokes, and never delays a refusal past the point fork 4 sets. A program that ignores
it loses nothing it would not lose anyway. Killing stays the owner's revoke, a person's decision.

## How others do it

From memory, unchecked; the building lane owes a read of each.

- Linux: cgroup `memory.high` reclaims and throttles below `memory.max`. PSI reports the share of
  time tasks stall on memory, and a process can poll a PSI trigger.
- Android: `onTrimMemory` delivers levels to apps, and the low-memory killer acts after.
- iOS: `didReceiveMemoryWarning` asks apps to shed caches, and jetsam kills after.
- Windows: memory resource notifications report low and high memory, and a process waits on them.
- Fuchsia: memory-pressure signals to components, and a component-level killer after.
- Every one of these has a killer behind its signal. nife keeps the signal and leaves the kill to a
  person.

## The first slice

1. The broker gains a per-account soft limit and a pool watermark, and a notification per account.
2. `user_mode_runtime` and `std` read the notification, trim at once, and run registered hooks.
3. A fixture, `cache_holder`, keeps a cache it can drop and registers a hook that drops it.

### Exit criteria a stranger could check

One `cargo xtask` gate that `script/test` runs, exiting 0 under QEMU on aarch64, riscv64 and
x86_64:

1. Account pressure. `cache_holder` fills a 32 MiB cache with its soft limit at 24 MiB. It gets the
   moderate bit, drops the cache, and its committed total, read from the broker, falls below 8 MiB.
2. Pool pressure. With the pool near its watermark, a second program's commit succeeds after
   `cache_holder` gives back, where it was refused with the signal disabled.
3. Advisory. A program that ignores the signal keeps running with its memory intact. Nothing is
   revoked.
4. Only the broker signals. A program's attempt to `SIGNAL` its own or another's notification is
   refused.
5. Measured, not gated: the time from signal to pages back in the pool.
6. A `design/decisions/` section records the levels and the delivery, minted by the integrator.

## BUGS

- The default soft limit and watermark are guesses until the first slice measures them.
