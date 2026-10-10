---
status: IN-PROGRESS
raised: 2026-10-10
branch: milestone/866-the-comment-block-sweep-closes
promoted_from: the-comment-block-sweep-closes
milestone_dependencies: 865
decision_dependencies: 267
machine_requirements: none
specific_machine: none
needs_person: no
---
# 866. The comment-block sweep closes, worth six

Raised 2026-10-10 (UTC) by the lane that built milestone 865 (the comment-block sweep continues a
third time, worth five), which found the worst rows to be constraint floors; minted from the
proposal of the same name. §267 (a comment
states the constraint as it is now) caps a comment block at 40 lines, with history in the commit
message and git and no narrative transplanted into documents.

This worth takes the next three rows: `crates/login_protocol/src/lib.rs:2` (169),
`crates/jh7110_entropy/src/lib.rs:2` (169), `kernel/src/arch/x86_64/iommu.rs:1` (164). It expects
floors. Three worths before it took the history. What remains should be constraint.

If the worth confirms that, it closes the sweep. The ratchet and the baseline stay as the
tripwire against new over-cap blocks. The rows that stay over cap are the recorded cost of the
constraints they carry, led by the gated `no_run` doctest in `crates/system_initializer/src/lib.rs:6`.

Reuse: the ratchet, the lexer and the baseline already exist (milestone 860 (comments state the constraint as it is now) built them). This
worth only runs the sweep, banks and closes, and takes no new code.

## Index row

Worth six, the closing worth of the comment-block sweep: the last three over-cap rows checked, expected floors, and the sweep closed with the ratchet left standing as the tripwire.
