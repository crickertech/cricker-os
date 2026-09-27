---
status: NOT-STARTED
raised: 2026-09-03
milestone_dependencies: none
decision_dependencies: unwritten
machine_requirements: none
specific_machine: none
needs_person: no
---
# 356. Retention declares the thread capability and says nothing about the region

Filed as a proposal on 2026-09-03 by the
`maintainer/spawn-retention-field` lane, while building DECISIONS §142's answer; promoted by
milestone 433 on 2026-09-19. Premise checked against the tree that day and unchanged:
`crates/supervision_protocol/src/lib.rs` still declares `Retention` with exactly two variants
(`Nothing` and `ThreadControlBlock { reason }`), the region is named only in prose there ("after
`START` the only authority over this child is the region it was built from"), and §142 has been
DECIDED since 2026-09-03 with nothing added about the region. The three spawn paths this file names
as disagreeing still disagree.

What a spawner keeps of a child's *region* is a convention with the same shape
as §142's, and §40 (there is no reaper of last resort) already records the open question this would
answer. It is an architect's call whether the declaration widens, and it should not widen on a
lane's initiative: the field is read by every spawn path in the tree.

**In brief.** §142 gave `ChildEndowment` a `retention` field, so one struct literal now states both
what a child is given and what its spawner keeps. What it states is the disposal of one capability,
the child's `ThreadControlBlock`, which the tree's own audit showed is **inert** after `START`: all
three of its methods refuse anything but an `Embryo`. The capability that is *not* inert, and that
actually decides whether a child can be ended, is the region it was built from, and the endowment
says nothing about that one.

## Why this is not tidiness

The declaration's whole claim is that reading a `ChildEndowment` tells you the authority in play in
both directions. It does not, yet, and the half it omits is the powerful half. `MemoryRegion::DESTROY`
on the region a child was built from is the only thing in this system that can end a live thread
(DECISIONS §16's amendment). Whether a spawner keeps that capability is decided today exactly the
way TCB retention was decided before §142: one call site at a time, by whether it happens to call
`cap_delete(region)` afterwards.

The sites disagree, and the disagreement is real rather than accidental. `components/src/spawner.rs`,
`fixtures/src/c_confiner.rs` and `components/src/timetable.rs` drop the region as soon as the child runs, so
they hold nothing that reaches a live instance's memory and say so in comments. `crates/system_initializer`'s
job path keeps its region across the child's life and reclaims it on death. Both are correct for
what they do. Neither is stated anywhere a reader of the endowment can see.

## What §40 already says about it

§40's first recorded caveat is this question in different words: the ownership chain that makes a
reap work "holds only if children are built from the supervisor's own region," and whether building
a child from a *delegated* region is forbidden or merely discouraged "is not yet decided, and it
should be before someone writes a supervisor that does it." A declared field is the natural place
for that answer to live, because the declaration is per-child and the rule is per-child.

## The fork, so it is not decided by accident

There are at least three shapes and they are not equivalent, which is why this is `DECISION` rather
than a lane:

- **Widen `Retention`** into an enum over both objects, so one field says what is kept of each. One
  noun to learn, and the two are genuinely one question about one child.
- **A second field beside it**, since the region is passed to `build_child` as an argument
  (`build_ut`) rather than named in the endowment, and the two capabilities have different
  lifetimes: the TCB is disposed of at `START` and the region at reap, which may be much later.
- **Leave it out and say so** in `Retention`'s own docs, on the ground that the region's disposal is
  a property of the *builder's* bookkeeping rather than of the child, and that a field which cannot
  be acted on at one moment is a declaration the code cannot enforce.

The third is a real answer and may be the right one. What is not an answer is the present state,
where the endowment reads as a complete account of the authority in play and is silent about the
capability that carries most of it.

## Index row

DECISIONS §142 gave `ChildEndowment` a `retention` field so one struct literal states both what a
child is given and what its spawner keeps. What it states is the disposal of the child's
`ThreadControlBlock`, which the tree's own audit showed is inert after `START`. The capability that
actually decides whether a child can be ended is the region it was built from, and the endowment
says nothing about it: `components/src/spawner.rs`, `fixtures/src/c_confiner.rs` and
`components/src/timetable.rs` drop the region when the child runs while `crates/system_initializer`
keeps its across the child's life, both correct for what they do and neither stated where a reader
of the endowment can see it. §40's first recorded caveat is the same question in other words.
Whether the declaration widens is an architect's, which is why the gate is DECISION: three shapes
are written out here and the third, leaving it out and saying so in `Retention`'s own docs, is a
real answer.
