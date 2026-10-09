---
status: PROPOSED
raised: 2026-10-09
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# The comment-block sweep's next worth

Raised 2026-10-09 (UTC) by the lane building milestone 860 (comments state the constraint as it
is now), at the end of its worth. §267 (a comment states the constraint as it is now) caps a
comment block at 40 lines, held by `helpers/comment_block_ratchet.py` against
`design/comment-block-baseline.tsv` (both names provisional). The baseline held 338 rows when the
gate landed and holds 337 after milestone 860's worth; the sweep works through them per file,
worst first, one milestone's worth at a time.

The next worst after that worth, with their rows: `kernel/src/testing.rs:240` (468 lines),
`crates/system_initializer/src/lib.rs:6` (406), `components/src/timetable.rs:1` (224),
`crates/jh7110_entropy/src/lib.rs:2` (209), then `kernel/src/arch/x86_64/iommu.rs:1` and
`crates/job_mix/src/lib.rs:1` (197 each). A worth takes the worst handful, moves each block's
history to a commit message, its findings to a note, and its semantics to a `§N` citation, and
banks the shrinks (`python3 helpers/comment_block_ratchet.py --bank`). The method, the refused
alternatives and the evidence are milestone 860's block and `notes/comment-cost-2026-10-07.md`;
this proposal claims no new decision, only the next slice of a ruled sweep.

Reuse: the ratchet, the lexer and the baseline already exist (milestone 860 built them); this
worth only runs the sweep and banks, and takes no new code.
