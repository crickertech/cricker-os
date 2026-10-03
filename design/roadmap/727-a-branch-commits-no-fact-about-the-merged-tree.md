---
status: IN-PROGRESS
raised: 2026-10-03
branch: lane/no-merged-tree-facts
promoted_from: a-branch-commits-no-fact-about-the-merged-tree
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 727. A branch commits no fact about the merged tree

Promoted 2026-10-03 (UTC) on calef's ruling of the same day on the COE pull request #1564: "Approve decision 4." The number 727 is provisional until the queue lands it. *(Title and slug are drafts.)*

## What it does

Two rules, one principle: a branch commits no fact that only the merged tree can make true.

1. **A branch promotes only a proposal it held.** `script/roadmap --check` (so `script/lint` and CI's `clippy` job, on the pull request, before the queue) fails a numbered block the branch adds that says `promoted_from: X` when `X` is neither in `design/roadmap/proposals/` at the branch's merge base with `origin/main` nor added by the branch's own commits. The message says to wait for the pull request holding the proposal to merge, then merge `origin/main` and `git rm` it. Fixtures in `script/roadmap --selftest`.
2. **The Kani harness count leaves the prose.** The hand-typed `<!--count:kani-harnesses-->` markers in `notes/unsafe-obligations.md` and `notes/verification.md` are gone, so a merge that adds a harness no longer conflicts there or fails the counted-claims check on the merged tree. The generated figure is the chart in `notes/project-metrics.md` (`project-metrics/harnesses.csv`). The `kani-harnesses` derivation stays: `notes/fuzzing.md` keeps a `count-at-least` floor on it.

## Follow-on

- **Recorded.** The prose ratchet's per-file baseline is also a fact about the merged tree; the proposal left it alone on purpose, and word budgets are out of this milestone's scope.
- **Recorded.** `script/roadmap` fails open, with a message, where it cannot compute a merge base (no `origin/main`); CI checks out at `fetch-depth: 0`.

## Index row

A branch may promote only a proposal it held, enforced in `script/roadmap --check`, and the Kani harness count no longer lives in two notes' prose, so neither fails on the merged tree after passing on the branch.
