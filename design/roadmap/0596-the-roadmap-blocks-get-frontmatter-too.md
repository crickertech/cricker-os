---
status: BUILT
raised: 2026-09-24
built: 2026-09-26
promoted_from: the-roadmap-blocks-get-frontmatter-too
---
# 596. The roadmap blocks get frontmatter too

Promoted from the proposal
`the-roadmap-blocks-get-frontmatter-too` on calef's ruling of 2026-09-26 (UTC): *"Roadmaps should
shift from bolder sections to front matter. We did a trial and it seems to be working. So let's
roll it out."* The trial is milestone 582 (a decision's status becomes a field, and the index
becomes generated). The number is provisional; 595 was the highest claimed when this lane cut its
branch, and the integrator mints it at merge. It was the last block written in the old form, and
the switch it built converted it with the rest.

## What the proposal said, kept

`design/roadmap/` carries the defect `design/decisions/` carried until 582. A block's status is a
sentence, and `script/roadmap` recovers the status, the Built date, the gate, the branch and the
promoted-from slug with regexes over paragraphs. The migration is 582's four steps: a schema, a
mechanical rewrite, scripts that read fields, and a rule that prose may not restate a field. The
proposal gated itself on 582 holding as a pilot. calef's ruling says it has.

## Phase 1: what the migration faces

Measured 2026-09-26 over `main` at `716ae77ef`: 583 numbered blocks, one lettered addendum (20a),
and 18 proposals.

| status | blocks | Built date | gate line | other companion |
|---|---|---|---|---|
| `BUILT` | 258 | 258 of 258 | none | |
| `NOT-STARTED` | 217 | none | 217 | |
| `PARTIAL` | 44 | none | 44 | |
| `REFUSED` | 43 | none | none | 43 of 43 carry `## Revisit` (40 Condition, 4 Unstated bullets) |
| `SUPERSEDED` | 10 | none | 10 | prose names 2 to 5 milestones each |
| `RECORDED` | 6 | none | 6 | |
| `REMOVED` | 3 | 2 of 3 | none | |
| `OPTIONAL` | 2 | none | 2 | |
| `IN-PROGRESS` | 0 | | | none to check for a branch |

The vocabulary is nine tokens, not the proposal's eight: it left out `RECORDED`. Proposals add a
tenth, `PROPOSED`, legal only under `proposals/`.

The gate has already done most of the archaeology 582 had to do. No `BUILT` block lacks its date,
no `REFUSED` block lacks its `## Revisit`, and no `IN-PROGRESS` block lacks a branch, because the
gate fails all three. The expensive field is the one 582 also found expensive, a raise date.

Gates, 283 lines holding 291 tokens: `NONE` 168, bare `DECISION` 41, `DECISION §N` 29, `HARDWARE`
34, `MILESTONE N` 19. Combinations: 8 `HARDWARE` or `DECISION` pairs and 5 with a milestone.

Other facts a parser reads from prose:

- Promoted-from slug: 99 status paragraphs parse through `helpers/roadmap_proposals.py`, and 5 more
  mention a promotion in a shape the parser does not accept.
- The milestones a `REFUSED` status paragraph names, which `--revisit` excludes as self-citations:
  43 blocks, 42 of them naming two or more.
- 88 status tokens carry words inside the bold. 79 are `BUILT <date>` whose date equals the Built
  line, and 9 carry prose that has to move out of the bold verbatim. 46 status paragraphs are only
  the marker and vanish.
- 282 status paragraphs still carry a "number provisional" note from before they landed. Out of
  scope here; it is prose.
- None of §207's five dependency fields appears in any block. §207 (the roadmap is a graph, and the
  block says so in fields a script can walk) is `DECIDED` with no milestone building it.

Raise dates. 295 blocks state a mint or raise date in their first three paragraphs, unverified as to
which event it dates. From git, 87 blocks were first added by milestone 76 (split the roadmap) in
`6192cd358`, or by the backfill that followed it. Their origin is in `design/roadmap.md` history,
the same tracing 582 did through `DECISIONS.md`. 181 blocks were renamed after they were added and
need `--follow`.

Parsers of this prose: `script/roadmap`, `helpers/roadmap_proposals.py`, `script/journeys` (working
tree), and three that read old revisions, `script/catch-up`, `script/citations --moved` and
`script/metrics`. `script/fatal-risks` and `script/audits` read `script/roadmap --index` and need
nothing. The three history readers must keep their prose parser permanently, because the blobs they
read predate the flip. `script/catch-up` already does exactly this for decisions since 582.

## The schema, as proposed and then ratified

Flat `key: value` lines, as 582's parser requires: no nesting, and a list is one comma-separated
value. Keys snake_case, values uppercase where they are a vocabulary, dates UTC.

| key | values | required when | from today's | blocks |
|---|---|---|---|---|
| `status` | the nine tokens; `PROPOSED` under `proposals/` | always | `**Status: X` | 601 |
| `raised` | `YYYY-MM-DD` | always | prose, else git's first add, earlier wins | 601 |
| `built` | `YYYY-MM-DD` | `BUILT`; optional on `REMOVED`; else forbidden | `**Built:**` line | 260 |
| `branch` | a branch name | `IN-PROGRESS` | backticked branch in the status paragraph | 0 |
| `promoted_from` | a proposal slug | optional | status-paragraph regex | 99 + 5 to read |
| `superseded_by` | milestone numbers, or `§N` | `SUPERSEDED` | prose, chosen by reading | 10 |
| `refused_by` | milestone numbers, or `none` | `REFUSED` | every milestone the status paragraph names | 43 |
| `milestone_dependencies` | numbers, or `none` | where a gate is required today | `MILESTONE N` | 279 |
| `decision_dependencies` | section numbers, `unwritten`, or `none` | the same | `DECISION §N`, bare `DECISION` | 279 |
| `machine_requirements` | capabilities, or `none` | the same | `HARDWARE`, read by hand | 279 |
| `specific_machine` | host and reason, or `none` | the same | `HARDWARE`, read by hand | 279 |
| `needs_person` | `yes` or `no` | the same | `HARDWARE`, read by hand | 279 |

Where it mirrors 582: `status`, `raised` and `superseded_by` are 582's keys with 582's rules. No
`number` or `title` key, since the filename and H1 hold both. The prose may not restate a key:
`**Status:`, `**Built:**` and `**Gate:` at the start of a line outside a fence fail the gate, the
shape of `script/decisions`' `RESTATED`.

Where it does not: 582 refused a `branch` key because a branch is transient. Here `IN-PROGRESS` is
defined as a branch, and the check that it has not merged is the reason the token exists. There is
no `decided` or `ratified_by`: nothing in a milestone is ratified the way a section is.

What stays prose: the H1, the `## Index row` summary (a paragraph, and 582 kept prose out of the
frontmatter), the `## Follow-on` and `## Revisit` bullets, and the dates a block was refused,
superseded or removed, which no script reads. After `built` moves out, `## Index row` holds only the
summary, under a heading named for a table that no longer exists. Renaming it is a naming call and
not this milestone's.

The last five rows are §207's fields, which is the fork below. The smaller alternative is one `gate`
key holding today's tokens verbatim.

## Phase 2 onwards: the rollout, so it does not wreck lanes in flight

On 2026-09-26, 6 of 15 open pull requests touch `design/roadmap/`: #1324, #1321, #1318, #1311,
#1301 and #1289, the last touching 38 blocks. Each edits the top of its own block, which is where
the migration writes, so each would conflict with a hand-made flip.

1. Readers first. One shared parser that reads frontmatter and falls back to prose, used by every
   parser above. It lands like any pull request and breaks nothing, because both forms are legal.
2. A migrator in the tree (`script/roadmap --migrate [FILE...]`, provisional), deterministic and
   idempotent: same input, same output, and a migrated file is left alone.
3. The flip is a script run, not an edit. It is regenerated from current `main` rather than rebased,
   so its own conflicts cost nothing. In the same commit, `--check` starts rejecting the old form,
   with a message naming the migrator. It is enqueued when the queue is drained.
4. A lane rebasing across the flip takes its own side of any conflicted block and reruns the
   migrator on it. That is a new case in `briefs/rebase-onto-main.md`. A lane that rebases clean but
   wrote a new block in the old form fails lint, and the failure names the command.

## The bold touch rule (#1311), measured

The migration does not shrink the bold backlog, because #1311 already stopped counting bold a script
parses. On #1311's branch (`32afdfa2d`) the roadmap blocks carry 8,263 spans over budget in 494
documents. Migrated, they carry 8,270 in 494: seven more, because the removed marker words shrink
each document's allowance. The migration takes 1,126 parsed spans (status, Built and gate markers)
out of the working tree, and #1311 was not counting them. Without #1311's exemption the roadmap
figure would be 10,846. Most of what remains exempt is the 1,535 `## Follow-on` and `## Revisit`
tags, which this schema leaves as prose.

## The decisions, as calef ruled them

Seven were owed. calef answered the first on 2026-09-26 (UTC), *"Do the dependency fields now"*:
the block carries the five dependency fields of §207 (the roadmap is a graph) as keys, and `Gate:`
retires with no interim key. He answered the other six the same day, *"Yes to all"*:

- the key names in the schema table, snake_case forms of §207's labels included;
- `decision_dependencies: unwritten` for the 41 blocks whose gate named no section;
- `raised` on every block, from the block's own text, else git;
- a `branch` key for `IN-PROGRESS`, though 582 refused one;
- one format for proposals, with `status: PROPOSED`;
- `superseded_by` and `refused_by` as lists that may name a `§N`.

The last one corrected phase 1's recommendation that `superseded_by` be one number, as 582's is.
Reading the ten showed four naming several milestones, and milestone 350 (the comment ratio
AGENTS.md quotes is wrong) answered by a decision, §177 (whether AGENTS.md quotes measured numbers at
all). Key names are not in `script/names`' scope, so the ratification is recorded here and in
`helpers/roadmap_block.py`, where they are spelled.

## Steps 1 and 2: built

Every parser reads both forms through `helpers/roadmap_block.py`. The three that walk history
(`script/metrics`, `script/catch-up`, `script/citations --moved`) keep the prose reader for good. In
the frontmatter form, `script/roadmap` checks what each key requires and forbids, and refuses a prose
line that restates a field. It computes blocked-ness from each dependency's own status, so a landing
no longer obliges an edit in another block. `--unmodelled` lists the blocks with no dependency
fields.

`script/roadmap --migrate [FILE...]` is the migrator (`helpers/roadmap_migrate.py`). It is
deterministic, and a second run changes nothing. Migrating one file gives the same bytes the
whole-tree run does, which is what a lane rebasing across the switch relies on.

The 34 `HARDWARE` gates were read by hand into the migrator's table. 33 need a person. Nine name a
specific machine with the reason one machine is the point: argon twice, radon four times, xenon
three times. Milestone 143 (silicon IOMMU) needs a machine nobody has, not a person.

Proved on a throwaway worktree migrated whole, 602 files:

- `script/roadmap --index` and `--ready` are byte-identical before and after, and `--check` passes.
- `script/catch-up` and `script/citations --moved` across the switch report no status change, and
  `script/metrics`' milestone and velocity series are unchanged.
- `script/citations --ratchet`, `helpers/prose_ratchet.py --check` and the whole of `script/lint`
  pass on the migrated tree.

## The prose ratchet re-measures a field token as syntax

A first trial migration failed the prose ratchet on 125 blocks, none of whose prose got worse. It
counted `**Status: BUILT.**` and `**Gate: NONE.**` as two-word sentences, so removing them raised 118
medians. The maintainer ruled on 2026-09-26 (UTC) that the ratchet reads every field token
`script/roadmap` parses as syntax, the principle #1311 applies to bold, and re-banks the rows that
moves once, in the same change.

So `helpers/roadmap_block.py` now holds the tokens (status, gate, `**Built:**`, and the Follow-on and
Revisit bullet tags) and `without_fields`, the one rewrite the migrator performs. The prose ratchet
measures a roadmap document through it, the migrator writes its output, and `script/citations
--ratchet` treats a line that only lost a token as unchanged. The three agree by construction, so the
migrator writes no glosses any more; the first trial's 160 were an artifact of the two disagreeing.

`MEASURE` in the ratchet is now 2 and the baseline records it. A change that bumps it may raise the
rows it moved, once, through `--remeasure`, and no other change may. That re-measured 158 roadmap
rows, every one a median, and raised no other column. The word count a prose-budget exception is
held to now leaves frontmatter out, as the main-body count already did. No exception in the tree
moves today, because none sits on a file with frontmatter; after the switch, milestone 139 (drive
down unsafe) and milestone 47 (navigation and naming) are the two it keeps under their grants.

## Step 3: the switch

One commit ran `script/roadmap --migrate` over 584 blocks and 18 proposals and turned on the refusal
of the old form, with `notes/roadmap.md`, both READMEs and case 8 of `briefs/rebase-onto-main.md`
rewritten to match. It was enqueued when nothing ahead of it in the queue touched
`design/roadmap/`. The bold touch rule #1311 added to the prose ratchet would otherwise have held
every one of those blocks to 4 bold per 1,000 words, though nobody edited their prose. So a block
whose only change is the switch is not touched, by the same `without_fields` comparison the rest of
this uses. That was this lane's call, and it is reversible.

## Follow-on

- **Recorded.** `script/roadmap --unmodelled` lists the finished blocks with no dependency fields.
  Their gates were cleared as their dependencies landed, and nothing in the block can recover them;
  §207 left that backfill lazy, and this milestone did not do it.
- **Recorded.** Bare `DECISION` became `decision_dependencies: unwritten` on 41 blocks, each a fork
  nobody has written up. The limitation is recorded in `notes/roadmap.md`.
- **Recorded.** `helpers/roadmap_migrate.py` holds the hand-read tables, and a block that gains a
  `HARDWARE` gate or a `SUPERSEDED` status on a branch after 2026-09-26 is refused by name. Its
  `BUGS` section says so, and it goes away once no branch in flight predates the switch.

## Index row

A milestone's status, dates and dependencies stop being sentences a regex guesses at and become
fields, the migration milestone 582 piloted on `design/decisions/`, extended to the 583 blocks every
lane edits.
