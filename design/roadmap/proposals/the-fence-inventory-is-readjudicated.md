---
status: PROPOSED
raised: 2026-09-29
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# The fence inventory is re-adjudicated

From the 2026-09-29 documentation audit, finding 7
([names and numbers](../../audit-reports/2026-09-29-names-and-numbers.md)). `notes/memory-ordering.md`
says thirteen fences at its line 28, "all fourteen" at line 83 over a fifteen-row table, and
"Twelve sites today" at line 227, while `script/lint` counts 19. Behind the stale numbers, the
fences this window's milestones added are adjudicated nowhere a reader would meet them, and an
inventory whose table no longer names its population is worse than no inventory.

The milestone re-derives the count, adjudicates every site the window added (the clock writer,
`drain_input`'s two acquires, and whatever else the nineteen includes), and corrects the three
stale numbers. The audit refused to fold this in because a correction without the adjudication
would misrepresent the table as complete.

## Done means

The note's table names the whole population `script/lint` counts, each row says why its fence is
there, and the note and lint agree by construction: either the note cites lint as the counter, or
a check refuses the next drift. A reader meeting any fence in the source finds its adjudication
in the inventory.

Name provisional.
