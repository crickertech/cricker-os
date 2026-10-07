---
status: BUILT
raised: 2026-09-27
built: 2026-09-27
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: yes
---
# 611. Every program and crate belongs to a package, and every package has a home

Built 2026-09-27 (UTC). *(Number provisional until the merge queue lands it. Title a draft.)*
calef ruled on 2026-09-27, on pull request #1389 (the OS is built and updated from packages; its decision is not yet on main).
The tree stays one repository for now, "with package boundaries drawn and enforced inside it by
lint check". The end state is everything leaving it: "I want to force decisions on homes versus
there being a default of sticking around." Packages borrow the composition of Linux distributions.
Later the same day he widened it: "Everything in the tree may not be in a package, but it may be in
a repo."

Enforce first, move nothing. The table, the formats and every name here are provisional and
calef's to ratify. [notes/package-boundaries.md](../../notes/package-boundaries.md) is the table
and the reasons.

## What is built

- Map. 26 packages over every crate, program and the std overlay in the eight workspaces: 16
  base, 5 optional, 4 sdk, 1 test. A kind says where a package ends up (calef ratified the rule
  and the four kinds at 2026-09-27T07:23Z). Where Linux has the tool the distros' grouping is taken
  (`procps`, `coreutils`, `util-linux`); where it has only a role, nife groups by role. calef
  renamed the three `process-tools`, `core-tools` and `disk-tools` on 2026-10-06 (UTC), moving
  `uuid` into `core-tools`: the grouping is borrowed, the names are not.
- Declare. `packages/<name>.package.toml` per package, with `name`, `kind`, `home` and at least
  one member required. `packages/homes.toml` gives a home to every tracked path in no package. Both
  are TOML, as the recipes are since calef's ruling of 2026-09-27. `home` is required, and
  `undecided` needs a reason.
- Gate. `script/lint` runs `helpers/packages.py`: a selftest planting fourteen violations, then
  the tree. Every tracked path has one home, and every crate and program is in one package. A link
  across a boundary goes to an interface or a declared dependency, or through a dated exception
  that fails once its link is gone. Proved on the tree by a planted `use timetable` in `rm`, which failed.
- Metric. `script/metrics` counts packages by where their home stands and whether they have
  left, and tracked files the same way: two panels under "Homes" on the weekly page, from 2026W39.

At build: one package home undecided (`redoxfs`), 25 provisional. Of 2,637 tracked paths, 1,693
had an undecided home, 944 a provisional one, none unclaimed. Thirteen exceptions were recorded; nine remain after pull request
#1392 (the system tests leave the kernel).

## How the moves will be done

Not here. One package per pull request at a quiet moment in the queue, as unchanged file moves in
their own commit, leaves first. The plan is a proposal, linked below.

## BUGS

- Dev-dependencies are not checked, and the per-program check reads source text. The note's BUGS
  section has both, with what each leaves open.
- A `depends` line admits every crate of the package it names. Review has to ask whether it is true.

## Follow-on

- **Milestone 691.** Milestone 691 (packages move out of this repository, one per pull request). `design/roadmap/691-packages-move-out-one-per-pull-request.md`: the moves,
  one package per pull request, once a home is ratified.
- **Milestone 689.** Milestone 689 (contracts leave the implementation crates they live in). `design/roadmap/689-contracts-leave-implementation-crates.md`: four kernel
  exceptions where a contract lives in a tool's or a driver's crate.
- **Milestone 684.** Milestone 684 (a documentation site, and documentation that ships in packages). `design/roadmap/684-a-documentation-site.md`: calef's documentation site, and
  why every note's home is undecided until it is settled.
- **Recorded.** The kernel links three fixtures, `board_console` links one, and
  `system_initializer` builds the whole image from inside `init`. Each is an exception in its
  package file and a line in notes/package-boundaries.md's BUGS.
- **Done.** Pull request #1392 (the system tests leave the kernel) made four kernel links
  dev-dependencies; the gate failed their exceptions as stale and they were deleted.
- **Milestone 686.** Milestone 686 (a package's license is derived from what it links, and a lint checks it). `design/roadmap/686-a-package-licence-derived-from-what-it-links.md`: a
  per-program SPDX license from linked crates, a lint against it, owed by §135 (running GPL
  software is aggregation)'s amendment (calef, 2026-09-27T15:11Z). Not built here.

## Index row

Every crate, program and tracked path now has a package or a home, declared in `packages/` and
gated by `script/lint`. There are 26 packages borrowing the distros' grouping and 9 dated
exceptions. The weekly metrics page counts undecided homes, which calef's end state takes to zero.
