---
status: DECIDED
raised: 2026-09-25
decided: 2026-09-25
ratified_by: calef
---

# 216. Fatal-risk facts are correctable, and verdicts are the architect's

calef ruled on 2026-09-25 (UTC). *(Section number provisional until the merge queue lands it;
§215 (the second RISC-V machine is a rented Scaleway Elastic Metal RV1) took the number before.)*

## The ruling

The maintainer may correct a factual error in `design/fatal-risks.md`, or in an appendix under
`design/fatal-risks/`, without asking the architect first. A factual error is a wrong date, a wrong
instrument, or a claim the machine disproves. Each correction is dated and cites the source that
disproves the old text.

Three things stay the architect's, and a correction does not touch them:

| stays the architect's | where it lives |
|---|---|
| the Experiment status word | each entry's status line, from the set §211 (what a fatal-risk verdict says, and what the chart can plot as a result) closed |
| the color: GREEN, AMBER or red | the prose after the status word, and the running order's cells |
| the running order | the table at the foot of the file |

When the corrected facts argue that a verdict should move, the maintainer says so to the architect and
leaves the verdict as it stands.

## Why

Milestone 536 (two records still say the prover cannot see `kernel/src`) is the case that forced it.
Risk 2 said `cargo kani` never compiles the kernel for three weeks after milestone 193 (put
`kernel/src` within reach of the prover) made it compile. The correction sat behind a `DECISION`
gate because the file was treated as wholly calef's, so a sentence the machine had already disproved
waited on his attention. That is the wrong side of AGENTS.md's reversibility line: a dated
correction citing its source is cheap and reversible, while a verdict is a published claim a
stranger may quote.

## What else was considered

- Every edit waits on calef. The state before this ruling. It spends his attention on lookups,
  and it is why risk 2 stayed wrong for three weeks.
- Any lane may correct. A developer lane is briefed on one milestone and does not see the file's
  whole argument. The lane for milestone 64 (enough `std` to run somebody else's crate) did edit
  risk 1, and said in the text that it does not normally do so. The ruling names the maintainer
  role, not every agent.

## How it is enforced

Rung three of AGENTS.md's ladder: the rule is written in `design/fatal-risks.md` under *Who may change
an entry*, where an editor meets it. `script/fatal-risks --check` already fails a status word that
contradicts the record it names, which covers half of the verdict side mechanically. Nothing gates
the line between a fact and a verdict, and no check could tell them apart in prose.
