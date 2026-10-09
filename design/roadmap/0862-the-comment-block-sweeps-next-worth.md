---
status: IN-PROGRESS
raised: 2026-10-09
branch: milestone/862-the-comment-block-sweeps-next-worth
promoted_from: the-comment-block-sweeps-next-worth
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 862. The comment-block sweep's next worth

*(Minted 2026-10-09 (UTC) by lane `milestone/862-the-comment-block-sweeps-next-worth` from the proposal `the-comment-block-sweeps-next-worth`. The number is provisional until the merge queue lands it; the title and slug are drafts.)*


Raised 2026-10-09 (UTC) by the lane building milestone 860 (comments state the constraint as it
is now). It filed this at the end of its worth. §267 (a comment states the constraint as it is
now) caps a comment block at 40 lines. `helpers/comment_block_ratchet.py` holds the cap against
`design/comment-block-baseline.tsv` (both names provisional). The baseline held 338 rows when the
gate landed and holds 337 after milestone 860's worth. The sweep works through them per file,
worst first, one milestone's worth at a time.

The next worst after that worth, with their rows: `kernel/src/testing.rs:240` (468 lines),
`crates/system_initializer/src/lib.rs:6` (406), `components/src/timetable.rs:1` (224),
`crates/jh7110_entropy/src/lib.rs:2` (209), then `kernel/src/arch/x86_64/iommu.rs:1` and
`crates/job_mix/src/lib.rs:1` (197 each). A worth takes the worst handful. Each block's history
moves to a commit message, its findings to a note, its semantics to a `§N` citation. The shrinks
are banked (`python3 helpers/comment_block_ratchet.py --bank`). The method, the refused
alternatives and the evidence are milestone 860's block and `notes/comment-cost-2026-10-07.md`.
This worth claims no new decision, only the next slice of a ruled sweep.

Reuse: the ratchet, the lexer and the baseline already exist (milestone 860 built them). This
worth only runs the sweep and banks, and takes no new code.

## Index row

The comment-block sweep's next worth: the 337 over-cap blocks the baseline holds, swept a milestone's worth at a time, worst first. Every shrink is banked, so the baseline only falls. History moves to commit messages, findings to notes, rulings to sections.
