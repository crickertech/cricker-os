---
status: DECIDED
raised: 2026-10-04
decided: 2026-10-04
ratified_by: calef
---

# 247. The split begins with basalt holding nife, and `procps` moves first

*Section number and slug provisional; the maintainer mints the number at merge. Written 2026-10-04
(UTC) by the lane `lane/split-order-proposal`. The options and their costs are in milestone 756
(`procps`, `coreutils` and `util-linux` are the first code to leave this repository); the
measurements are in [notes/the-first-split.md](../../notes/the-first-split.md).*

§151 (the goal of the repository split is independent release) ruled where the split ends and said
its order "should be ruled on its own". This is that ruling. calef answered both questions on
2026-10-04 (UTC), relayed by the maintainer on pull request #1609.

## The rulings

1. **basalt v0 is the first step.** basalt pins `nife` whole at a commit and runs its image build
   and system test on all three architectures; nothing leaves this repository. The ruling includes
   the go-ahead for a lane's first write to `nifeos/basalt`. The manifest's file name and format
   ship provisional. Milestone 755 (basalt v0 pins nife and runs its gate) builds it.
2. **The first code cut is `procps` + `coreutils` + `util-linux`, and only once all six
   conditions hold**:
   - milestone 610 (the interface's stability is measured weekly) carries the per-package co-change
     column §235 (the OS is built and updated from packages) requires before any move;
   - `components` is split so those programs are in crates of their own (milestone 691 (packages
     move out of this repository, one per pull request));
   - the cut builds without a clone of nife, through §235's S2 archive or a dated, recorded
     exception;
   - `abi`, `address_space_map`, `glob` and `machine_statistics_protocol` are measured by 610 and
     settled by its rule for four weeks;
   - basalt takes a component's output as a package by digest (§235's P1, milestone 607 (nife is
     built and updated from packages) item 1);
   - the home and its repository name are ratified, and §201 (one roadmap until a citation has to
     cross)'s citation identity is ruled, since this cut is its revisit event.

## Why, in one paragraph

Of the nine preconditions the records name, only the package format held on 2026-10-04: 157
breaking changes to the contract crates in four weeks, and 56 programs from 18 packages in one
crate. basalt v0 needs none of them and builds the whole-system gate that §151's BUGS calls the real
cost of a split, before any change has to land in two repositories. Among code cuts, `procps` and
its neighbors had the fewest focused cross-package commits (5 of 11) and link the fewest moving
interfaces. `redoxfs` was cheaper to move but more coupled, and preferring it would have been an
effort argument.

## What is not decided here

- The repository name and home for the first cut. Names are calef's under the naming rule.
- The citation identity across repositories, which §201 deferred to the first code cut.
- Whether milestone 691's order puts the SDK last among the leaves, as milestone 756 recommends.

## BUGS

- The coupling figures behind the ruling are a one-off reading at one commit over thirty days.
  Milestone 610's owed column replaces them, and condition 1 makes that column a gate.
