---
status: PROPOSED
raised: 2026-10-09
milestone_dependencies: 862
decision_dependencies: 267
machine_requirements: none
specific_machine: none
needs_person: no
---
# The comment-block sweep continues, worth three

Raised 2026-10-09 (UTC) by the lane that built milestone 862 (the comment-block sweep's next
worth), at the end of its worth. §267 (a comment states the constraint as it is now) caps a
comment block at 40 lines. The baseline held 337 rows when milestone 860 (comments state the
constraint as it is now) landed and holds 336
now. One row left entirely: the frame-budget ledger, gone to `notes/frame-budget.md`. Five
banked down (406, 224, 209, 197 and 197 lines, to 301, 201, 171, 164 and 141).

The worst still on it, with their rows: `crates/system_initializer/src/lib.rs:6` (301, held by
its `no_run` doctest, which is a gate and stays), `components/src/timetable.rs:1` (201), then
`crates/glob/src/lib.rs:1` (190), `crates/board_console/src/lib.rs:1` (186), and
`crates/non_volatile_memory_express/src/lib.rs:2` and `crates/component_plan/src/lib.rs:1` (185
each). A worth takes the worst handful that is not a gated example. Each block's history goes to
a commit message, its findings to a note, its naming record to `design/naming/`, and every shrink
is banked. The method, the refused alternatives and the evidence are milestone 860's block and
`notes/comment-cost-2026-10-07.md`.

Reuse: the ratchet, the lexer and the baseline already exist (milestone 860 built them). This
worth only runs the sweep and banks, and takes no new code.
