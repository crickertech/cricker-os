---
status: IN-PROGRESS
raised: 2026-10-03
branch: milestone/630-queue-ejection
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 630. A merge-queue ejection is caught before the queue, and recovered after it

Raised 2026-10-03 (UTC) by calef, after #1473 (milestone 624) was ejected from the merge queue at
01:45 UTC. The number is provisional until the merge queue lands it; the title and slug are drafts.

## Index row

A ready pull request whose own block still reads IN-PROGRESS fails in seconds on `pull_request`
rather than 20 to 38 minutes into a merge group, and an ejection is detected, labelled, explained
on the pull request and re-armed once its head moves and goes green.
