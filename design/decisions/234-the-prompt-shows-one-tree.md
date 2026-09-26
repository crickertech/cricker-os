---
status: DECIDED
raised: 2026-09-26
decided: 2026-09-26
ratified_by: calef
---

# 234. The prompt shows one tree, and other trees are mounted at names in it

*Section number provisional until the merge queue lands it, for the reason §231 (a swap's warning to
a dependent is advisory) gives. The file name is provisional too.*

Raised 2026-09-26 by the lane for milestone 154 (a process that holds two directory capabilities)
([block](../roadmap/154-multi-directory-namespace.md)), as the last question between a two-tree shell and a
live interactive one. The resolver, the witness and the transport options are in
[`notes/two-trees.md`](https://github.com/crickertech/nife/blob/milestone/154-two-directories/notes/two-trees.md), which lands with #1346 and is not on `main` yet.

## The ruling

calef, 2026-09-26 (UTC): *"One tree with other trees mounted at names in it."* Recorded by the
maintainer at 18:22Z the same day.

- The person at the prompt sees one root. Another tree appears at a mount name inside it, under a
  convention such as `/media/<label>` or wherever its owner binds it.
- Labels are internal. They tell the shell which endpoint a handle belongs to and never appear in a
  path the person types or reads. `pwd` prints the mount path.
- When a mounted tree's capability dies, requests into it get `Gone`, its name disappears from the
  tree, and a shell standing inside it returns home.

## What it amends

§126 (a process holding two directory capabilities gets a real, single, moving `cwd`) resolved an
absolute path by its leading label, so a two-grant shell had no unlabelled root. That part is
replaced for the prompt. §126's cwd stands: one position at a time, and a real `cd` between trees.
What `..` does at a mount name was not part of the question, and this section does not decide it.

## Refused

Labelled roots at the prompt, which is what §126 and the note's resolver built. Every absolute path
would carry a label (`/docs` becomes `/<label>/docs`), which is drive letters by another name.

## Deferred

- How a mount name is chosen automatically when nobody binds one.
- The default policy for who receives a newly attached device.

## The transport, and when it lands

The note's option 1: presence by a named slot, the way `RUN_UNVOUCHED_SLOT` and `NETWORK_SLOT`
are found, with the labels and rights in a shared constant in a crate both programs depend on. It
lands with the first real second filesystem, not with a second subtree of the disk the shell already
holds whole, which would not be a separate tree at all.
