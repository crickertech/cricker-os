---
status: BUILT
raised: 2026-10-03
built: 2026-10-04
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

## Follow-on

- **Done.** An `AGENTS.md` instruction that every commit signs the model that wrote it, including under opencode, which reads that file. #1546 merged 2026-10-03 (UTC) and added it to AGENTS.md's Commits section; `grep -in co-authored AGENTS.md` finds it.
- **Done.** A commit hook that refuses an agent commit with no trailer, 2026-10-04 (UTC). The maintainer ruled on the signal that day, so the fork is closed. `.githooks/commit-msg` requires a non-empty `Co-Authored-By:` trailer only when `CLAUDECODE=1` or `OPENCODE=1` is set. Claude Code sets the first in every shell it runs (verified in the lane's own shell). opencode's CLI startup sets `AGENT=1`, `OPENCODE=1` and `OPENCODE_PID`, read from the installed 1.18.34 binary on 2026-10-04. With neither marker the hook exits 0, so calef's own commits pass untouched, and a merge in progress passes too. Evidence: `helpers/commit-msg-selftest.sh` runs ten cases plus an end-to-end `git commit` through `core.hooksPath`, and `script/lint` runs it and fails if the hook is missing. Watched failing once: with the marker test replaced by `if true`, the selftest exits 1 on `claude, no trailer: wanted refuse, got pass`. Restored, it prints `ok`. The existing `core.hooksPath .githooks` line in `script/setup` wires it, so no new install step exists.
- **Done.** A per-token rate for GLM in the ledger, 2026-10-04 (UTC). calef: the z.ai plan is $80 a month and probably will not renew. The flat price was already a Subscriptions row. `ledger.md` now also carries the derived rate: $80 over the 713,543,216 GLM tokens `script/effort` counts, or $0.1121 per million for both GLM models (source c there). calef supplied the invoice date, 2026-09-29, so the window is 2026-09-29 to 2026-10-28, with the one-month length his belief. Every GLM token so far falls inside it. 2026W40's blended rate in `cost.csv` is now 0.277. The ledger records that the plan is expected to end, so a week with no GLM tokens is expected and not a gap. Filling W40 also needed list prices for `claude-opus-5-5` and `claude-sonnet-5-5` (source d; their cache-write cells are derived, not read). If more GLM tokens are spent first, the rate falls, and the fix is a new dated ledger row.

## Index row

script/effort and the model table counted only Claude Code, so the z.ai week read as idle and unattributed; both now read opencode's database, and 152 commits are attributed by a committed record. The signing instruction (AGENTS.md), the commit-msg hook and the GLM rate are all done.

## BUGS

- The hook is a courtesy, not a boundary. An agent that clears its environment (`env -i git commit`), a harness not on the list (Codex and Gemini CLI set markers it does not read) or `git commit --no-verify` goes through unchecked. That commit lands under `unattributed`. Adding a harness is one line in `.githooks/commit-msg` plus a selftest case, once its marker is verified.
- The opencode marker is read from source, not observed. No run inside an opencode shell has confirmed that its bash tool inherits `OPENCODE=1`, and a later release could rename it. The check for when an opencode session next exists: `echo $OPENCODE` from its bash tool.
- The hook sees the committing process, not the author. A person committing from a shell inside Claude Code's environment is held to the trailer. A commit made through a tool that scrubs the environment is not.
- 2026W39's blended rate is still blank. It has tokens on `open-lane-kimi` (the OpenRouter bake-off model), which has no ledger rate row. That is another vendor's price and not this milestone's work.
- The GLM window's length is calef's belief that the 2026-09-29 invoice covers one month. The invoice is not in the tree.
