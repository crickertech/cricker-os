---
status: BUILT
raised: 2026-09-24
built: 2026-09-24
promoted_from: a-ruling-updates-the-gate-it-answers
---
# 591. A ruling should make the gate it answers fail until someone updates it

The number is provisional; the integrator mints it at merge, and 590 was the
highest claimed when this lane cut its branch. The file stem is a lane's coinage and is provisional
too. Promoted from the proposal `a-ruling-updates-the-gate-it-answers`, filed 2026-09-24 by the
`maintainer/198-gate-is-stale` lane. That lane found milestone 198 (a package manager, and the
trivial install that makes a second customer possible) still gated `DECISION`. Its last fork had
been ruled a day earlier, by §208 (installing a package is granting it, and the activation set is
versioned).
`design/fatal-risks/README.md` risk 8 repeated the stale gate as the reason nobody can be asked to run
nife. Built the same day as option 2 below. The proposal's analysis is kept, lightly cut; its gate
line is gone because a BUILT block may not carry one.

## What went wrong, in the tree's own mechanisms

A `MILESTONE N` gate fails `script/roadmap --check` the day milestone N is `BUILT`, so a gate still
pointing at finished work fails the build instead of going stale. A `DECISION` gate had no such
partner. It named no object, so nothing could tell when the decision it waited on had been made.
198's activation fork was a roadmap block, milestone 507 (installing a package: mutate, compose, or
widen what can be spawned). Ruling it minted §208 and set 507's own gate to `NONE`, but touched
nothing that cites 507. Rung 3 of AGENTS.md's ladder (a written record at the thing) was present in
507 and absent at 198.

## Measured, 2026-09-24, over `design/roadmap/*.md`

- 70 blocks carry a `DECISION` gate. 39 cite at least one `§N` in the gate paragraph, and 31 cite
  none.
- 12 cite only sections that are already ruled (9 `DECIDED`, 3 `AMENDED`). Some are stale. Others
  cite a ruled section as context while waiting on something else: milestone 39 (repository
  structure for a loosely-coupled OS) cites §151 (the goal of the repository split is independent
  release), which ruled the goal and left the order open.
- So a prose citation cannot be read as "waits on", which rules out inferring the gate from prose.

## Options

| | Mechanism | Rung | Cost | Why it wins or loses |
|---|---|---|---|---|
| 1. Infer from prose | `--check` warns when every `§N` in a `DECISION` gate paragraph is ruled | 2, as a warning | An afternoon | Loses as a gate: 12 hits today, and 39 is a false positive the checker cannot tell apart |
| 2. `DECISION §N` in the gate vocabulary | A `DECISION` token may name its section; `--check` fails once that section is ruled | 2, failing | The grammar, plus 39 blocks read by hand | Built. It is the `MILESTONE N` rule applied to the other kind of wait. Bare `DECISION` stays legal for a fork not yet written up |
| 3. `DECISION MILESTONE N` | As 2, for a fork held as a roadmap block (507's shape) | 2 | Small, on top of 2 | Not built; see BUGS |
| 4. At ruling time | `script/decisions --check` lists blocks whose gate cites a section being ruled | 3, a report | Small | Loses alone: a list printed once to whoever rules is rung 4 in practice |

Would we still choose 2 if 1 cost the same? Yes. 1 cannot fail without false positives, and 2 is
the rule the tree already trusts for milestones.

## What was built

`script/roadmap` accepts `DECISION §N` beside bare `DECISION`, one token per section, so a gate on
two forks names two tokens and loses one when one is ruled. `helpers/roadmap_proposals.py` accepts
the token in a proposal's gate line. `--check` fails when the section does not exist, and when its
frontmatter says anything but `PROPOSED`. `DECIDED` and `AMENDED` answered it; `SUPERSEDED` means
another section did. The status is read off `status:`, which milestone 582 (a decision's status
becomes a field) made a field.

27 of the 39 citing blocks were converted, each by reading the gate paragraph. In each, exactly one
cited section is `PROPOSED` and the paragraph names it as the wait. The ratchet in
`script/citations` then wanted each converted block to gloss its section in plain text, so the
section's first markdown link became a glossed citation.

| Blocks converted |
|---|
| 52, 66, 95, 102, 105, 131, 142, 147, 178 |
| 180, 205, 206, 258, 394, 395, 397, 398, 403 |
| 404, 406, 407, 408, 413, 415, 421, 423, 440 |

The other sections those paragraphs cite are context they already call ruled. The subshells block
says the conversation calef asked for is its gate; its section is where that answer lands, so it
converts too.

The 12 that cite only ruled sections stay bare, since `DECISION §N` would fail the day it was
written. They are 39, 48, 137, 172, 188, 334, 340, 356, 376, 388, 391 and 417. They go to a
proposal rather than a guess here.

## BUGS

### A bare `DECISION` is still checked for nothing

It names nothing, so nothing can tell when it has been answered. That is deliberate, since a fork
not yet written up has no section to name. It means the 31 blocks that cite no section, and the 12
above, can go stale exactly as 198 did.

### A named section can be the wrong one

The check proves the section is still open, never that it is what the block waits on. A gate naming
a sibling fork stays green after the real one is ruled.

### Option 3 is not built

`DECISION MILESTONE N` is not in the vocabulary. Nothing gates on a fork held as a roadmap block
today; the first block that does should add it rather than fall back to bare `DECISION`.

### A proposal's `DECISION §N` is parsed, not checked

A proposal can be written in the form it will be promoted in, but only numbered blocks are checked
for staleness.

## Follow-on

- **Milestone 652.** Milestone 652 (twelve `DECISION` gates cite only sections that are already ruled). `design/roadmap/0652-twelve-decision-gates-cite-only-ruled-sections.md`: read
  the 12 bare gates and, for each, set the gate to what still stops a start or say why the ruled
  section is context.
- **Refused.** Option 1 as a gate. It would fire today on the 12, and 39 is a false positive the
  checker cannot tell apart.

## Index row

A `DECISION` gate named nothing, so nothing could tell when it had been answered, and milestone 198
sat behind a ruled fork for a day. `DECISION §N` now names the section and fails `--check` once
that section is ruled, the rule `MILESTONE N` already follows. 27 gates converted; 12 cite only
ruled sections and stay bare.
