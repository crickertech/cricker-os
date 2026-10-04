---
status: IN-PROGRESS
raised: 2026-10-03
branch: lane/fast-pre-push-hook
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 727. The pre-push hook runs what fits in seconds

Raised 2026-10-03 (UTC). The number is provisional until the merge queue lands it; the title and
slug are drafts, and every new name below is provisional.

## Why

Milestone 630 (a merge-queue ejection is caught before the queue, and recovered after it) made the
pre-push hook all of `script/lint`. On 2026-10-03 that cost 70 to 120 s warm and 202 s cold on a
loaded machine, a claim push sat in it for a whole lane, and it refused five claim pushes.
The roadmap block they lacked cannot exist yet. Lanes took to `--no-verify`. calef approved the narrower
decision on #1564 (2026-10-03 UTC): "Approve the revised decision 3."

## Built

(Filled in when the lane finishes.)

## Index row

The pre-push hook runs `script/fmt --check`, then every lint check that does not invoke cargo
(`script/lint --no-cargo`), then the ready-status check, and skips all of it for a push that changes
no files.
