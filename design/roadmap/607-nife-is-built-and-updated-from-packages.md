---
status: NOT-STARTED
raised: 2026-09-26
milestone_dependencies: none
decision_dependencies: 235
machine_requirements: none
specific_machine: none
needs_person: no
---
# 607. nife is built and updated from packages, and the repository divides along what releases together

The number is provisional: the integrator mints it at merge. calef asked for it on 2026-09-26
(UTC). A design lane (`milestone/607-packages-and-divisions`) wrote the forks and built nothing.

Four records touch the question and none joins the others. Milestone 39 (repository structure for a loosely-coupled OS) recorded a monorepo now and
a split later. §151 (the goal of the repository split is independent release) made the split's goal independent release and third-party programs. Milestone
198 installs programs and does not build the OS. Milestones 525 and 554 swap a whole image between
two boot slots. This block asks how a package becomes part of the OS, and where the repository's
seams are.

## What the design lane found

The forks are in [§235 (the OS is built and updated from packages)](../decisions/235-packages-build-the-os-and-the-tree-divides-by-release.md)
(provisional number), status PROPOSED. The measurements and the prior art are in
[notes/packages-and-divisions.md](../../notes/packages-and-divisions.md). Three findings shape it:

- The kernel's compiled-in trust root pins every base program, so the kernel relinks whenever
  userspace changes. It cannot release on its own while that holds.
- The kernel crate is also the integrator and the system test suite: `kernel/src/user/` is 28,441
  of its 89,885 lines, and it links nine service crates and three fixture crates.
- Contracts still change with their consumers: 76% of contract commits in 60 days touched another
  division. §151's precondition for ruling the split's order is not met.

## What is built

Nothing. The block is NOT-STARTED until a proposal below has a lane.

## What is left

The proposals, none minted, in the recommended order. §235 has the reasoning.

1. The image is assembled from base packages by digest (host side; waits on nothing).
2. The kernel crate stops carrying the userspace bring-up and the system tests (waits on nothing;
   the test-wiring hotspot, so one lane alone).
3. A contract never links the runtime: `supervision_protocol` and `swap_protocol` (waits on nothing).
4. A third party builds `greeting` with no clone of this repository (waits on §235's Fork 4).
5. An OS update is a set of packages that lands through a slot (waits on Fork 1 and item 1).
6. The kernel stops pinning userspace (waits on Fork 2 and item 2).
7. Each division is a workspace with a version (waits on Fork 3 and items 2 and 3).

## BUGS

- Division names are this lane's and provisional; calef names them.
- The image size quoted in §235 is a stale local build, and item 1's lane should measure a fresh one.

## Index row

how a package becomes part of the OS, and where the repository divides
