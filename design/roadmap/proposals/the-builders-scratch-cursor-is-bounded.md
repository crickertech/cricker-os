---
status: PROPOSED
raised: 2026-09-26
milestone_dependencies: none
decision_dependencies: unwritten
machine_requirements: none
specific_machine: none
needs_person: no
---
# The builder's scratch cursor is bounded

Raised by the lane for milestone 206 (a program image has under 896 KiB)
while siting every window on the user address-space map.

The fix needs a way for a userspace builder to take back a mapping in its own
address space, which is a question about the capability surface rather than about this loader.

## The finding

`supervision_protocol::SCRATCH_NEXT` is where the tree's one userspace ELF loader maps each frame of a
child it is building, so it can fill it. The cursor starts at `0x1000_0000`, advances one page per
page built, and never comes back, because the builder has no unmap.

In the progenitor, the kernel's initrd window sits at `0x2000_0000`, 256 MiB above the cursor's start.
A `ripgrep`-sized program is 2.6 MiB of pages to build, so after about a hundred such spawns in one
boot the cursor reaches the archive's window, the kernel refuses the mapping as already mapped, and
every later build fails as "could not spawn". Before milestone 206 raised the image ceiling no
program that large could be spawned, so the arithmetic was never reachable.

## The fix

Either the builder returns its scratch mapping when a child is built (an unmap of one page in its own
space, against a capability it already holds), or the cursor is a fixed window reused per build.
The second is what the loader did once, and it was removed because, without an unmap, a reused
address collides with the previous child's mapping (the history is beside the cursor). Both routes
need the unmap.

## Until then

Recorded as `BUGS` beside the cursor in `crates/supervision_protocol/src/lib.rs`, and in the
address-space map's own `BUGS`.
