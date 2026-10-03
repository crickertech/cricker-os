---
status: PROPOSED
raised: 2026-10-03
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# Machine effort and commit attribution count every vendor

Raised 2026-10-03 (UTC) by the maintainer session writing
[the merge-rate correction](../../../notes/coes/2026-10-03-the-merge-rate.md), from calef's
question on #1513: "Do our metrics in any way assume that Claude is the only LLM vendor?" Name
provisional, this file's alone.

## What assumes one vendor

From 2026-09-28 22:00 to 2026-10-01 05:07 UTC the work ran on z.ai (glm-5.3 and glm-5.3-flash,
under opencode), while Claude's weekly allowance was out. Measured from opencode's database: 4,554
assistant messages, 9.9 million input tokens, 0.96 million output, 701 million cache reads.

| measure | source | what it saw of that z.ai work |
|---|---|---|
| `script/effort`, `effort.csv`, the cost and context-per-turn charts | Claude Code records under `~/.claude/projects` | nothing |
| `script/metrics`, commits and lines by model | the `Co-Authored-By` trailer, a Claude-only table | 154 commits as `unattributed` |
| the ledger, `cash_spend` | `notes/project-metrics/ledger.md` | the $80 subscription, correctly |
| `notes/how-this-is-built.md` | prose | no vendor assumption found |

The open-lane gateway's models (kimi, qwen and others) are counted, because they ran inside Claude
Code. A harness other than Claude Code is what drops out.

## What to build

1. `script/effort` also reads opencode's session database, read only, and writes the same columns
   per model. Tokens there are per message with `providerID` and `modelID`.
2. Commits made outside Claude Code carry a `Co-Authored-By` trailer naming the model, and
   `script/metrics` learns those names, so the model table stops reading them as a gap. The cheap
   half is an instruction in `AGENTS.md`, which opencode reads; the strong half is a commit hook
   that refuses an agent commit with no trailer.
3. Backfill 2026W40 from the database, which still holds it.

## Index row

script/effort and the model table count only Claude Code, so the z.ai week (4,554 messages, 154 commits) reads as idle and unattributed; read opencode's database and sign non-Claude commits.
