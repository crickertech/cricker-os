---
status: PARTIAL
raised: 2026-08-04
milestone_dependencies: none
decision_dependencies: 162
machine_requirements: none
specific_machine: none
needs_person: no
---
# 95. An unmap primitive, and the mappings init never lets go

Raised 2026-08-04 from milestone 22's closing lane, which named it as the
largest residual left standing after the interactive boot gave away its authority.

An unmap method is a syscall-surface addition, which the block calls a design
fork for calef before it is a task: whether unmap belongs on the address space or on the frame,
what it does to a mapping another holder also has, and whether restructuring the loader to map one
page at a time avoids the new method entirely. **It is
§162 (whether a holder can give up a mapping)**, decided as A, `AddressSpace::UNMAP(va)`, on 2026-10-05; written
up 2026-09-19 by milestone 435's lane, which found this gate naming no decision though the block had
called it a fork since 2026-08-04.

**The finding.** `build_child` maps each page it lays down for a child into init's own address
space to write it, and never unmaps it, because nothing in the ABI can: there is no unmap.
Reaping a job hides the problem for jobs, since §13's revoke takes mappings with the region, but
the boot servers are never reclaimed, so init keeps a writable window onto every page of
every server it built, for the life of the machine.

That is a real hole in the story milestone 22 otherwise tells. Init drops its construction budget,
its device capability and its interrupt, and the kernel itself confirms `RETYPE` now answers
`NoSuchSlot`; and then init can still write into the filesystem server's text. The lane recorded
it rather than shrinking a budget to make the number look better, which is the right call and
leaves the work here.

What this milestone is. A way for a holder to give up a mapping it made: an unmap method on
the address-space object, symmetric with the map that created the window. That is a syscall
surface addition, so it is a design fork for calef before it is a task (CLAUDE.md: a new
method is fine within the model, but its semantics are recorded in `design/decisions/`, and a brand-new
syscall number is a fork). The questions the fork has to answer: whether unmap is a method on the
address space or on the frame capability; what it does to a mapping some other holder also has
(§13 already decides revoke's answer, and this must not contradict it); and whether unmapping is
enough or the scratch window wants a narrower shape, mapping one page at a time and releasing it
before the next, which needs no new method at all if the loader is restructured instead.

That last possibility is why this is not obviously a syscall: the cheapest fix may be a loader
that never holds more than one page, and the measurement that decides it is how much slower a
one-page-at-a-time loader boots.

## Built, 2026-10-05 (UTC)

`AddressSpace::UNMAP(va)` is built on all three architectures (`abi::address_space::UNMAP`,
`kernel/src/syscall.rs`'s `address_space_unmap`), proven by `system_tests/src/user/unmap_tests.rs`
with a replayable falsification for each of its four tests. The two semantics §162 left owed are
built provisionally and argued in `notes/unmap.md`: a `va` with nothing mapped answers
`BadPointer`, and no capability is consumed. They are asked of calef on pull request #1678.

**It does not close this block's hole yet**, and the reason is a finding, not a gap in the build.
No capability names a running address space: `CONFIGURE` retires the name, and the kernel grants
none at boot. The progenitor's scratch window is in its own running space, so it has nothing to
invoke `UNMAP` on. That is a fork on the syscall surface, written up as a proposal (below) and not
built.

## Scope note

The proof, when it happens, is the shape milestone 22 already used: init writes to a boot server's
page and faults, as a negative control, rather than an inventory of what init holds. Until then
the residual is recorded where a reader meets it, in notes/trusted-init.md's BUGS.

## Follow-on

Checked against the tree on 2026-10-05 (UTC), on `lane/95-unmap`.

- **Proposed.** How a running process names its own address space, without which `UNMAP` cannot
  reach the window this block names: `design/roadmap/proposals/a-running-process-can-name-its-own-address-space.md`.
- **Outstanding.** The progenitor giving up each scratch page after a boot server is built, and the
  negative control this block's scope note names (the progenitor writes to a boot server's page and
  faults). Both wait on the proposal above; `supervision_protocol::map_scratch` still never unmaps.
- **Outstanding.** The amendment to §162 (whether a holder can give up a mapping) recording the two
  semantics, once calef rules on them. A lane may not edit `design/decisions/`.
- **Recorded.** No test shows a remote core losing a translation through `UNMAP`, one page per call,
  and no `icount` row: `notes/unmap.md`'s `BUGS`.

## Index row

Milestone 22's largest residual: `build_child` maps every page it writes for a child and nothing
in the ABI can unmap it, so init keeps a writable window onto every boot server for the life of
the machine. A design fork, decided as A by §162 on 2026-10-05; the cheapest fix may be a one-page loader
rather than a new syscall
