---
status: NOT-STARTED
raised: 2026-09-19
promoted_from: a-filesystem-server-that-knows-the-time
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 497. A filesystem server that knows what time it is

*(Number provisional until the merge queue lands it.)* Promoted from the
proposal `a-filesystem-server-that-knows-the-time`, filed 2026-09-19, on calef's instruction of
2026-09-20 to give every proposal on `main` a number. The text below is the proposal's own, unedited
except for this paragraph: the argument is its author's and promotion is not the moment to improve
it. Written by the milestone 64 (enough std to run somebody else's crate) lane (`milestone/64-std-filesystem-shim`), from milestone 64's block
and `notes/touch.md`'s `BUGS`.

The mechanism a reader needs already exists and already has a decision behind it:
DECISIONS §43 made reading the wall clock a broadly grantable, read-only authority (a `Frame`
capability naming the clock page), and every `std` program already holds one at slot 5. What this
needs is a lane, and one judgment a lane can make: which spawn sites give the FS server the page.

In brief. `redoxfs_server` stamps every mutation with its own counter (`Server::clock`, starting at
1 on each mount and incremented per write, create, truncate, attribute change and bare `touch`),
because it holds no clock. The file contract says `GETMTIME` reports Unix seconds. Give the server a
read-only mapping of the clock page and stamp with the wall-clock second it reads, the same way
`SystemTime::now()` does in `patches/std-nife/overlay/std/src/sys/time/nife.rs`.

## Why this matters

Until 2026-09-19 the only reader of an on-device mtime was the shell's `touch`, and its `BUGS` section
already said the stamp was a counter. **Milestone 64 bound `std::fs::Metadata::modified`**, so now
every std program that asks gets that counter as a `SystemTime`, and it is wrong in three ways a
program can act on:

- A file written on nife reads as a moment in early 1970. `SystemTime::now()` in the same
  program reads 2026. Anything that computes an age (`now - modified`) gets fifty-six years.
- It orders wrongly against the image. Files the host tool put there carry real seconds
  (`tools/redoxfs_host`'s `put` stamps `SystemTime::now()`), so a file written on nife is always
  *older* than one the host made, whatever the true order. Every make-style staleness check that
  compares a source against an output made on the other side gets the wrong answer.
- **A nife write to a host-made file does not change its time at all.** The engine only moves an
  mtime forward (`vendor/redoxfs`'s `write_node`), and a counter in the tens is never ahead of a
  real second, so the host's stamp survives every write this system makes. A staleness check sees
  a file it just rewrote as unchanged.
- **It does not survive a reboot.** The counter restarts at 1 on each mount, so a file written in
  the second boot can be stamped earlier than one written in the first.

`std_exerciser` is written around these on purpose: its `write moves mtime ok` line asserts a write
moving a file *it made* forward, and runs before its `set_times` rather than after, because a write
after a `set_times` into the future does not move the time. When this lands, that line should
tighten to "inside the wall-clock window", a write after `set_times` should move the time again,
and a write to `motd` should move the host's stamp; those three are the test of done.

## What it would take, measured against the tree

- The authority exists. §43's clock page, a `Frame` with `READ`, is what `std_service` grants a
  std program (slot 5, mapped at `0x1200_0000`), and `crates/clock_protocol` is the reader and its
  seqlock, which the time PAL already uses (generated into `sys/pal/nife/clockproto.rs`).
- Spawn sites. `redoxfs_server` is spawned from `kernel/src/user/fs_service.rs` and from
  about eight test harnesses, and by `crates/system_initializer` on a booted system. Each would pass
  the page, or the server keeps its counter as a fallback and says so (a server started with no
  clock must not invent one; §42).
- The fallback question is the one judgment in it: when no clock is granted, keep today's counter
  (monotonic within a mount, and honest about being a counter only in `BUGS`) or refuse to stamp.
  Recommendation: keep the counter, because refusing a write for want of a timestamp is worse for
  every caller than a wrong timestamp documented as one.

Reversibility: fully reversible, and nothing on the wire changes; the contract already promises
Unix seconds. Nobody has acted on the counter values except the tests that were written knowing
they are a counter.

What is blocked: nothing hard. Milestone 121's `rg --sort modified` would order nife-written files
wrongly against host-made ones until this lands.

## Index row

`redoxfs_server` stamps every mutation with its own counter (`Server::clock`, starting at 1 on each
mount and incremented per write, create, truncate, attribute change and bare `touch`), because it
holds no clock.
