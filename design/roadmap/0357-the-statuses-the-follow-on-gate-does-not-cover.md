---
status: NOT-STARTED
raised: 2026-09-03
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 357. `NOT-STARTED` is the third status that can be false, and milestone 75 is the proof

Filed as a proposal on 2026-09-03 by the milestone 252 sweep; promoted by
milestone 433 on 2026-09-19. Both halves checked against the tree that day and both still hold.
`design/roadmap/0075-cycle-counter-authority.md` still reads `NOT-STARTED` with `Gate: DECISION`,
though DECISIONS §139 answered that decision on 2026-09-02 and milestone 74's own gate line now
opens *"Both halves of this line are under correction"* and says in full why 75's status is false.
`script/roadmap`'s `ANSWERS` tuple is still `BUILT`, `REMOVED` and `PARTIAL`. One count in the body
below has moved: `SUPERSEDED` was minted on 2026-09-15, so the uncovered statuses are now five
rather than the three the title claims (`NOT-STARTED`, `IN-PROGRESS`, `OPTIONAL`, `RECORDED`,
`SUPERSEDED`), and `IN-PROGRESS` keeps the one mechanical check it was given on 2026-08-17.

Fixing milestone 75's status is a paragraph and a word, and the design question
behind it is answerable by reading `script/roadmap` and the four uncovered statuses.

In brief. Milestone 247 gated `BUILT` and `REMOVED`; milestone 252 gated `PARTIAL`. Three
statuses are left uncovered (`NOT-STARTED`, `IN-PROGRESS`, `OPTIONAL`, `RECORDED`), and at least one
of them rots the same way. **`design/roadmap/0075-cycle-counter-authority.md` reads `NOT-STARTED` with
`Gate: DECISION` for a decision calef made on 2026-09-02 (§139) and two milestones built the same
week** (229 on 2026-09-02, 237 on 2026-09-03). Three separate blocks cite 75 as a live blocker, and
one of them, milestone 74, carries `MILESTONE 75` in its own gate line, so the staleness propagates
into the readiness report a maintainer uses to brief lanes.

## What is worth deciding

- Fix 75 first, which is a status word and a paragraph, and re-read the gate lines that name it.
  That is worth doing whether or not anything below happens.
- Then the question this proposal exists for: whether a `NOT-STARTED` block can be checked at
  all. A `PARTIAL` block enumerates work, so the gate can demand the enumeration. A `NOT-STARTED`
  block asserts one thing, that nothing has been done, and the tree cannot see that.
- The one mechanical handle that does exist: a `DECISION` gate on a block whose decision file is
  `DECIDED`, and a `MILESTONE N` gate where N is BUILT. `script/roadmap` already refuses the second
  case for a gate line; it does not connect a `DECISION` gate to any file, because a gate line does
  not name one. Making it name one would be a real check and a real cost.
- `IN-PROGRESS` is already the one status with a mechanical check (its branch must not have merged),
  minted for the same reason on 2026-08-17 after all six were found false, and it is evidence that
  this family of defect is worth gating where it can be.

## BUGS

- **The gate for `NOT-STARTED` may not exist.** If the honest answer is that only a person can tell,
  this proposal ends with 75 fixed and a sentence in `notes/roadmap.md` saying so, and that
  is a legitimate outcome rather than a failure. Milestone 252's own block reserves the same right.
- `OPTIONAL` and `RECORDED` are deliberately off the work list, so a stale one costs less than a
  stale `NOT-STARTED`, which is offered to lanes as ready work.

## Index row

Milestone 247 gated `BUILT` and `REMOVED`, milestone 252 gated `PARTIAL`, and the rest of the status
vocabulary rots with nothing watching it. `design/roadmap/0075-cycle-counter-authority.md` is the
proof: it reads `NOT-STARTED` with `Gate: DECISION` for a decision calef made on 2026-09-02 and a
mechanism two milestones built the same week, three blocks cite it as a live blocker, and milestone
74 carries `MILESTONE 75` in its own gate line, so the staleness propagates into the readiness report
a maintainer briefs lanes from. Fixing 75 is a word and a paragraph and is worth doing on its own.
The question behind it is whether a `NOT-STARTED` block can be checked at all: a `PARTIAL` block
enumerates work so a gate can demand the enumeration, while a `NOT-STARTED` block asserts that
nothing has been done and the tree cannot see that. The one mechanical handle that exists is a
`DECISION` gate whose decision file reads `DECIDED`, and making a gate line name that file is a real
check with a real cost.
