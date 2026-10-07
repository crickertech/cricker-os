---
status: DECIDED
raised: 2026-09-23
decided: 2026-09-23
ratified_by: calef
---

# 208. Installing a package is granting it, and the activation set is versioned

Ruled by calef on 2026-09-23, reading milestone 507 (installing a package: mutate shared directories, compose a view, or only widen what can be spawned)'s three options with the maintainer: A3, with
rollback. *(Section number provisional until the merge queue lands it.)*

## The ruling

Installing records that a package exists. Its digest and manifest become spawnable; its data is
a read-only directory a session binds by name. Installing changes only what may be granted. Nothing
is written into shared space, and uninstalling is removing the entry.

And the table of entries is versioned, so an activation set can be selected and rolled back as a
whole rather than one package at a time. That second clause is the half A3 does not get for free,
and it is the half calef asked for by name.

## How he got there, because each step eliminated something

First he asked whether any of the three supports checking that an install succeeded,
*"specifically thinking of packages that can live load to replace an older version and then roll
back if it breaks"*.

That eliminated A1. Mutating shared directories overwrites files in place, so there is no trial
state to compare against and nothing to go back to. Milestone 507's own table records that A1's undo
is a per-package file list with "no atomicity across files unless RedoxFS provides it", and that
RedoxFS atomicity is **not checked**. An option whose undo story rests on an unverified property of
a vendored filesystem cannot answer the question he asked.

Then the second step, which is the one worth keeping: *"I think we want to make a choice with
packaging that supports replacing an already running process other than the kernel. Live loading the
kernel is deferrable and we should assume live loading other components since so much runs in
userspace buys us a long runway on live loading the kernel."*

This is the microkernel thesis paying off, and it should be read as such. In a monolithic system
most updates are kernel updates, so live-patching the kernel is the first thing an upgrade story has
to solve. Here the drivers, the filesystem and the network stack are userspace processes, so
live-loading userspace components covers most of the ground an upgrade needs to cover, and
milestone 509 (live-patching the kernel) can wait behind it. The kernel stays the one fixed thing,
which is what milestone 23 (a capability-routed component OS with live replacement) claimed the
architecture would buy and this is the first time the claim has decided anything.

## Why A2 lost, and it did not lose on merit

A2 stores each package whole and read-only and puts a composer in front of them, presenting the
union of the activated set. It is Haiku's packagefs and Nix's profiles, both read rather than
recalled by milestone 507's lane and cited there with sources. It is a good design and it is the one
with the strongest prior art in the table.

It lost on fit. Three reasons, none of them about its quality:

- The union server would be the first place this tree composes a namespace in a server rather than
  in a client. §50 (namespace composition) chose `bind` over stored paths and milestone 47 (navigation and naming) put the composition in the
  client. A2 moves it, and moving it is a change to how every session reaches a file, not an
  addition beside it.
- It buys nothing on authority. `design/haiku-bfs-and-packages.md` already records Haiku's own
  limit: activation "gives atomicity and rollback, not confinement." Under A2 a program's authority
  is still its grant, and the composed view is a convenience over the top.
- The atomicity it does buy is what "with rollback" adds to A3 directly. Versioning a table of
  entries is a smaller object than a union server, and it is the part of A2 calef actually wanted.

## The non-effort case for A3, which is what decided it

Milestone 507's own §92 (a caretaker is supervised by the client it serves) test says that if A3 is preferred because it is cheaper to build than
A2, that is effort and must be said in those words. It is not the reason here, and the reason it is
not is a fact about the tree rather than about the work.

The tree already has capability-routed live replacement, and three quarters of it is built.
Milestone 23 is `PARTIAL` with three of its four parts done: the component manifest
(`crates/component_plan`), the hung-component case, and dependency-aware orchestration, where a
supervisor asks the dependency graph who must be warned before a swap instead of hard-coding it per
system. `kernel/src/user/live_swap_tests.rs` exercises a real protocol against a real swap, with
`QUIESCED`, `DRAINED`, `PROBE_SURVIVED`, `REFUSED` and `DEATH` as reported steps.

Under A3 an activation is a grant, and rolling one back is revoking one. That is §16 (object revocation)'s
machinery, already built and already proven, pointed at a new kind of object. The supervisor that
performs a live swap and the thing that decides which version is active are the same authority
holding the same kind of capability.

Under A2 a server owns the view, so the supervisor coordinates with a composer sitting between
it and the component it supervises. Every swap becomes a three-party agreement: the supervisor, the
incumbent, and the server that decides which bytes either of them sees.

**So this is about fewer moving parts, not about less work.** Would we still choose A3 if both cost
the same to build? Yes, and for that reason: one authority instead of two, and rollback falling out
of a mechanism the kernel already has rather than being a second mechanism beside it.

## What this obligates, which is the cost of the ruling

Three things, stated plainly so nobody reads A3 as free:

- The program namespace is sealed at boot, and A3 widens it. The shell resolves a name through a
  closed `grant_plan::Prog` enum with `PROG_COUNT` of 13, sends its integer id over `spawnproto`,
  and the progenitor indexes a table it filled once at boot from the measured archive. Milestone 150 (adding a program should not need eight hand-maintained lists)
  generates that table from one declaration and removes the hand-maintenance; the seal survives it.
  Under A3 the set is no longer fixed when the image is built, and naming a program that was not
  there at boot is a change two programs agree on.
- The spawner cannot read anything installed after boot. The progenitor gives the file service
  away as soon as the shell holds it, so a package written to RedoxFS is bytes that nothing which
  builds processes can reach. The tree has solved this exact shape once: `login` needs an image
  the progenitor cannot fetch, so the progenitor hands over a copy of the one program it needs as a
  blob (milestone 233 (login dies on every boot)). That precedent is a hand-wired special case for one program, and A3
  needs it generalized.
- **Versioning the activation set is the piece A3 does not get for free.** A1 and A2 were each
  weighed with an undo story attached; A3's is an addition, and it is what "with rollback" buys. It
  is also the smallest of the three, which is a fact about this option rather than an argument for
  it.

None of this touches §195 (a reviewed recipe vouches for a package), which asks a different question: whether the progenitor will run
something the build-time measurement table does not name. A3 decides what installing *does*; §195
decides what the machine will vouch for.

## What it reopens, and it is one thing

§116 (live component state handoff is declined, for want of a customer) declined state handoff on 2026-08-23 for want of a customer, on the ground that no
component with meaningful live state existed or was being built. A package upgrade that replaces a
running component is that customer. The whole point of the second half of calef's argument is that
the filesystem server and the network stack get upgraded live, and those are precisely the
components whose open handles and connection tables cannot be dropped on the floor.

§116 recorded a transport shape as guidance without committing to it: an opaque blob over a shared
page, with capabilities moved by `GRANT`. That guidance is now owed a decision rather than a note.

So §116 needs revisiting, and this ruling is what supplies the premise it was missing. It is
named here and not edited: a section is revisited deliberately, by whoever takes that work, and a
lane recording a different ruling is not that.

## What is unblocked

Installing onto a running system, which milestone 507 listed as blocked until this was answered, and
with it rung 3 of milestone 198 (a package manager, and the trivial install that makes a second customer possible) under §157 (a trivial install is a web page, a USB drive, and packages over the internet), whose
definition of a trivial install is growing the system by installing packages over the internet.
