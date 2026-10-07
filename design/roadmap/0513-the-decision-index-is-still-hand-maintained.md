---
status: NOT-STARTED
raised: 2026-09-19
promoted_from: the-decision-index-is-still-hand-maintained
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 513. The decision index is still hand-maintained

*(Number provisional until the merge queue lands it.)* Promoted from the
proposal `the-decision-index-is-still-hand-maintained`, filed 2026-09-19, on calef's instruction of
2026-09-20 to give every proposal on `main` a number. The text below is the proposal's own, unedited
except for this paragraph: the argument is its author's and promotion is not the moment to improve
it. Filed by milestone 443 (lanes wait on each other for three)'s lane, which took the gates off the generated roadmap index and found
the same hotspot one directory over, untouched.

It is milestone 294's shape applied to `design/decisions/`, over files already in
the tree.

## The shape, because it is the one already solved next door

`design/roadmap/README.md` used to be the tree's worst merge conflict, structurally rather than by
bad luck: every lane edits its own milestone's block, every milestone also needs a row in one sorted
table, so every lane in flight collided with every other lane in flight, always, in that one file.
Eight conflicts in a single session on 2026-09-14. Milestone 294 derived the row from the block and
the conflicts stopped.

`design/decisions/README.md` is 194 rows of exactly that table and nothing derives it. Every lane
that lands a decision edits the index by hand, and §194 names it in passing as one of the two
"generated indexes" that conflict on every interleave, which is half true: the roadmap one is
generated and this one is not.

## What it would take

Each decision file already carries, on line 1 and in its Status line, three of the four columns the
row holds: the number, the title and the status. The fourth is the filename. `script/decisions`
already parses all of them, and already checks that the row and the file agree, which is the check
that becomes unnecessary the moment one is derived from the other.

So the work is the same three pieces 294 had, minus its hard part: there is no hand-written
summary column to migrate, which was 294's whole expense (288 blocks, a median similarity of 0.07
against the opening paragraph, so the column had to move verbatim rather than be derived).

1. `script/decisions --write` renders the table between markers, `--index` prints it.
2. The row checks become checks on the file, as they did for the roadmap.
3. The staleness of the committed table is **reported and does not fail**, for 294's reason: failing
   would make every lane edit the file it was just taken out of.

## What it does not fix

**It does not remove the number collision**, which is the thing two sessions actually fight over and
which §194 rules stays. It removes the *conflict in the table* and leaves the duplicate, which is the
right division: a duplicate number is a real defect and should be a merge conflict somebody reads.

And it inherits the open question next door. A generated index that no lane may edit is only as
current as whoever regenerates it, and nothing does;
`design/roadmap/0510-nothing-regenerates-the-roadmap-index.md` is that question and this would
be its second customer rather than a second instance of it.

## Index row

`design/roadmap/README.md` used to be the tree's worst merge conflict, structurally rather than by
bad luck: every lane edits its own milestone's block, every milestone also needs a row in one sorted
table, so every lane in flight...
