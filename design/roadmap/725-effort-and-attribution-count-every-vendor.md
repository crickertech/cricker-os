---
status: PARTIAL
raised: 2026-10-03
promoted_from: effort-and-attribution-count-every-vendor
milestone_dependencies: 519
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 725. Machine effort and commit attribution count every vendor

Promoted from `design/roadmap/proposals/effort-and-attribution-count-every-vendor.md` on 2026-10-03 (UTC). The number 725 is provisional until the queue lands it. *(Title and slug are drafts.)*

Raised 2026-10-03 by the maintainer session that wrote the merge-rate correction, from calef's question on #1513: do our metrics assume Claude is the only vendor? They did. From 2026-09-29 02:33 to 2026-10-01 05:07 UTC the work ran on z.ai (GLM models, under opencode). `script/effort` read none of it, and `script/metrics` filed its 154 commits under `unattributed`.

The work: `script/effort` reads opencode's session database for this project's sessions and writes the same per-model rows. Commits it can prove an opencode session made go in a committed `commit-models.csv`. `script/metrics` reads that file, so a GitHub runner with no database still attributes them. What cannot be proved stays unattributed and is counted.

## What is built, and what is left

Built: `script/effort` reads opencode's database (`--selftest` pins the shape), `--attribute` writes `notes/project-metrics/commit-models.csv`, `script/metrics` reads it, and 2026W40 is backfilled. The z.ai window now reads 3,874 `glm-5.3` and 632 `glm-5.3-flash` requests, and 152 commits are attributed to them. What stays unattributed is stated by `--attribute` on every run, by week.

Left, and why it is not built here:

- **An `AGENTS.md` instruction that every commit signs the model that wrote it**, including under opencode, which reads that file. A developer lane does not edit `AGENTS.md`, so the maintainer owns this line.
- **A commit hook that refuses an agent commit with no trailer.** A hook cannot tell an agent from calef, so it needs a signal that this commit is an agent's. That is a design fork, not a build.
- **The cost columns for GLM.** The ledger carries a flat z.ai subscription and no per-token rate, so a week with GLM tokens has a blank blended rate. That is the honest reading until a rate is recorded.

## Index row

script/effort and the model table counted only Claude Code, so the z.ai week read as idle and unattributed; both now read opencode's database, and 152 commits are attributed by a committed record. Still open: a signing instruction in AGENTS.md and a hook, both of which need an architect or the maintainer.
