---
status: PROPOSED
raised: 2026-10-09
milestone_dependencies: 863
decision_dependencies: 267
machine_requirements: none
specific_machine: none
needs_person: no
---
# The comment-block sweep continues again, worth four

Raised 2026-10-09 (UTC) by the lane that built milestone 863 (the comment-block sweep continues,
worth three), at the end of its worth. §267 (a comment states the constraint as it is now) caps a
comment block at 40 lines. Its 2026-10-09 amendment rules the method: the constraint stays,
history goes to the commit message and git, and no refusal narrative is transplanted into a
document.

The baseline holds 336 rows. The worst after worth three:
`crates/jh7110_clock_and_reset/src/lib.rs:2` (184), `crates/current_cpu_protocol/src/lib.rs:1`
(183), `components/src/login.rs:1` (178), then `crates/jh7110_entropy/src/lib.rs:2` and
`crates/board_console/src/lib.rs:1` (176 each). A worth takes the worst handful that is not a
gated example, and every shrink is banked. The method, the
refused alternatives and the evidence are milestone 860 (comments state the constraint as it
is now)'s block and
`notes/comment-cost-2026-10-07.md`.

Reuse: the ratchet, the lexer and the baseline already exist (milestone 860 built them). This
worth only runs the sweep and banks, and takes no new code.

## Index row

Worth four of the comment-block sweep: the worst five over-cap blocks shrunk to their constraints and banked, history to commit messages and git, no transplant documents.
