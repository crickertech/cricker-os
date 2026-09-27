---
status: PROPOSED
raised: 2026-09-26
milestone_dependencies: none
decision_dependencies: unwritten
machine_requirements: none
specific_machine: none
needs_person: no
---
# A second filesystem mounts in the boot shell

Raised by milestone 154 (a process that holds two directory
capabilities), which built the mechanism and was closed BUILT without wiring a second tree into the
interactive shell, on calef's instruction. Name provisional: this file's stem is a lane's coinage.

Two choices calef deferred on 2026-09-26 sit in front of this: the automatic
mount name a new filesystem gets, and the default policy for who receives a new device. The trigger
is not a date but an event: the first real second filesystem, a second disk or a pulled-in drive.
Until one exists there is nothing honest to mount, which is why the boot shell passes none today.

## What calef ruled, and what it leaves to build

The presentation is "one tree with other trees mounted at names in it" (calef, 2026-09-26). The
shell shows one root; a second tree appears at a mount point, a `/media/<label>` convention or
wherever the owner binds it, and `pwd` prints the mount path. `cd ..` from a mount point goes to its
parent, pending calef (what-dot-dot-does-at-a-mount-point.md). Milestone 154 built all of that and proved it on the real wire (notes/two-trees.md).

The transport is option 1 of that note's proposal: init puts the second tree's endpoint at a named
capability slot, the shell probes it at `_start`, and a constant in a crate both depend on holds
the mount path and the rights. Built with the first real second filesystem, not before.

What lands with it:

- Wire `crates/system_initializer::boot`'s `second_dir` to the named slot, replacing its positional
  slot 5, and have the interactive shell probe it and call `Nav::rooted_twice`.
- When the mounted tree's capability dies, a request into it answers Gone rather than hanging, the
  mount name disappears, and a shell standing inside it returns home with a message (calef,
  2026-09-26).
- The spawn protocol says which tree a directory grant is in, so `rm` under a mount point is built
  rather than refused at delivery.
- Verification under `script/swish-check`, which is the only thing that runs a real init.

## Exit criterion

A boot with a second filesystem attached shows it at its mount point at the prompt, `ls`, `cd`,
`<` and `rm` work under it, and pulling it leaves the shell at `/` with a message and no hang.

## Index row

A second filesystem appears in the boot shell at a mount point, per calef's one-tree ruling, when
the first real one exists; the automatic mount name and device policy are deferred decisions.
