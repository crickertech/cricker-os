# The roadmap

*Name: provisional (a lane ships names provisionally; an architect names things). This file was
`notes/roadmap-index.md` until 2026-09-21, when the index it was named for was retired and its
vocabularies moved here.*

`design/roadmap/` is one file per milestone and nothing else. The directory listing is the
index. calef, 2026-09-21: *"I don't think the milestone index table is worth building any more.
It has too many rows to be digestible as a table."*

So there is no `design/roadmap/README.md`. What was in it that is not a row is here: the status
vocabulary, the frontmatter every block opens with, the follow-on dispositions, the rule that anybody may add a
proposal, and the history of the table itself. The rows are not here and are not anywhere, because
every one of them was generated from its own block's `## Index row` section and still lives there.

The queries are the other half of the index, and they are what a reader wants more often than a
list of 522 rows:

```console
$ script/roadmap --ready          # nothing open that it depends on, NOT-STARTED or PARTIAL: startable today
$ script/roadmap --outstanding    # what a PARTIAL milestone still says is left
$ script/roadmap --proposed       # unnumbered proposals, oldest first
$ script/roadmap --unclaimed      # follow-on work a finished milestone named that nobody took
$ script/roadmap --revisit        # a REFUSED milestone whose stated condition has come true
$ script/roadmap --check          # what the gate runs
$ script/roadmap --unmodelled     # the blocks that carry no dependency fields
$ script/roadmap --migrate FILE   # convert a block written in the old prose form (see below)
$ script/roadmap                  # every milestone that is not BUILT, grouped by what it waits on
```

## The task: add a milestone to the roadmap

You are a lane, you have been given milestone `N`, and you want it on the roadmap.

1. Write `design/roadmap/N-<slug>.md`. It opens with frontmatter, then `# N. <title>`:

   ```markdown
   ---
   status: NOT-STARTED
   raised: 2026-09-26
   milestone_dependencies: 527, 543
   decision_dependencies: none
   machine_requirements: riscv64 silicon; PMU cycle counter
   specific_machine: none
   needs_person: yes
   ---
   # N. <title>
   ```

   The keys, and what each status requires, are under *The frontmatter* below. The prose may not
   restate them: no `**Status:` line, no `**Gate:` line, no `**Built:**` line.
2. Give it a `## Index row` section, conventionally last, beside `## Follow-on` and `## BUGS`,
   holding one paragraph saying why this milestone matters. The Built date is the `built` key.

   The heading keeps its name although the index is gone (`script/audits` and
   `script/fatal-risks` still read it); renaming it is an architect's call.
3. Name what you considered taking, in a `Reuse:` line or a `## Reuse` section, or write
   `none exists` and where you searched. §46 (thin primitives or whole subsystems) makes taking the
   default outside the kernel and the crates Kani proves; `--check` requires this from 2026-10-05.
4. That is the whole procedure. There is no table to update and no regeneration to wait for.

### EXAMPLES

```console
$ script/roadmap --check
roadmap: 585 milestones, 586 files, status vocabulary clean, every '## Index row' parses, ...
roadmap: 280 of them classified by gate, 154 ready to start (script/roadmap --ready)
```

```console
$ script/roadmap --index | head -3     # the five columns, for the two scripts that read them
| #  | Status | Milestone | Why it matters (§14) | Built |
|----|--------|-----------|----------------------|------------|
| 1 | BUILT | [First boot][01-first-boot.md] | ... | 2026-07-12 |
```

## The vocabularies

### The frontmatter

Every block and every proposal opens with it, since milestone 596 (the roadmap blocks get
frontmatter too). The schema was ratified by calef on 2026-09-26 (UTC), *"Yes to all"*, after the
same move for `design/decisions/` in milestone 582 (a decision's status becomes a field, and the
index becomes generated). The format is 582's: flat `key: value` lines between two `---`, no
nesting, a list written as one comma-separated value, dates UTC.

| key | values | required when |
|---|---|---|
| `status` | the tokens below; `PROPOSED` for a proposal | always |
| `raised` | `YYYY-MM-DD` | always |
| `built` | `YYYY-MM-DD` | `BUILT`; optional on `REMOVED`; otherwise refused |
| `branch` | a branch name | `IN-PROGRESS`; otherwise refused |
| `promoted_from` | a proposal slug | optional |
| `superseded_by` | milestone numbers or `§N`, comma-separated | `SUPERSEDED` |
| `refused_by` | milestone numbers or `§N`, or `none` | `REFUSED` |
| `milestone_dependencies` | milestone numbers, or `none` | every status but `BUILT`, `REMOVED`, `REFUSED` |
| `decision_dependencies` | section numbers, `unwritten`, or `none` | the same |
| `machine_requirements` | capabilities, or `none` | the same |
| `specific_machine` | `<host> (<reason>)`, or `none` | the same |
| `needs_person` | `yes` or `no` | the same |

The five dependency fields travel together. They are §207 (the roadmap is a graph, and the block
says so in fields a script can walk), and they replaced the `**Gate:**` line: see *Dependency
fields* below. The schema is spelled once, in `helpers/roadmap_block.py`, and a lane does not extend
it.

A block written in the old form. Until 2026-09-26 each of these facts was a bold line in the
prose (`**Status: BUILT.**`, `**Gate: NONE.**`, `**Built:** 2026-09-14`), read by regex.
`script/roadmap --check` now refuses that form and names the command that converts it,
`script/roadmap --migrate <file>`. `briefs/rebase-onto-main.md` has the case for a lane that wrote
its block before the switch.

Status vocabulary. The `status` key in a milestone's own file is the *one* canonical place its
state lives, and it uses exactly these tokens. This exists because status used to be prose scattered through the detail
blocks, phrased a dozen different ways ("Built 2026-07-28", "DONE", "Phase 1 built", "Largely done"),
which is unreadable at a glance and, worse, unparseable: a status sweep on 2026-07-30 mis-reported eight
milestones. `script/roadmap` validates this column and fails on anything outside the vocabulary.

| Token | Means |
|---|---|
| `BUILT` | Complete, and proven by the gate on every supported ISA. |
| `PARTIAL` | Some phases shipped and more remains, with nobody currently on it. The block says which phases. |
| `IN-PROGRESS` | Active work on a branch right now. **The block carries a `branch` key**, and `script/roadmap --check` fails if that branch has already merged. |
| `REMOVED` | Was built, in whole or in part, and the code was deliberately deleted. The block must record when it went and why. The row keeps its `Built` date if it ever had one. |
| `NOT-STARTED` | Specified, nothing built. |
| `OPTIONAL` | Deliberately off the thesis path; not a backlog item. |
| `RECORDED` | Analysis captured and the decision deliberately *not* taken. |
| `REFUSED` | Work considered and deliberately not taken. **The block must carry a `## Revisit` section** saying what would change it, and `script/roadmap --check` fails without one. Not a backlog item: it is excluded from `--ready`, from the gate classification and from every count that reads as outstanding work. |

A detail block may narrate its state in prose (that is where the evidence belongs), but the
`status` key is what answers "where do we stand", and since 2026-09-21 it is the only record of
it there is. The rule here used to read *"if the two disagree, the column is wrong and the block is
right"*, which was the correct answer to a question that should not have existed: two
hand-maintained records, both looking authoritative, and a reader with no way to tell which was
lying. Milestone 69 (split `kernel/src/user.rs` by service) had a row saying `NOT-STARTED` while its
block said `BUILT`. Milestone 294 (the index is generated, not hand-maintained) dissolved the
question by deriving the column; retiring the column finished the job.

`REMOVED` was minted 2026-08-30 (calef), when milestone 54 (A network file service a Mac can actually mount)'s code was deleted and the six words
then available could only lie about it. `NOT-STARTED` denies it was ever built, which erases the one
demonstration this project has that a real Mac mounted a share served by this kernel. `OPTIONAL`
reads as available to pick up. `RECORDED` means the work was deliberately *not* taken, and here it
was taken and then reversed. Leaving it `BUILT` makes the column claim a network file service that
does not exist, and this table's own rule says that when column and block disagree the column is
wrong.

It carries the same extra obligation `IN-PROGRESS` does, and for the same reason: the block must
say when the code went and why. A status that records a deletion is the one nobody will
remember to explain, and a year later the difference between "removed because the customer left" and
"removed because it never worked" is the entire content of the row.

One word, not two. Built-then-deleted and specified-then-abandoned are genuinely different, and
splitting them was refused: the case has occurred once, and this vocabulary's own history is that a
word earns its keep by being used correctly. Milestones 54 and 55 both take `REMOVED` and their
blocks say which kind they were. The cost of that choice showed up immediately and is why the
date is optional: 54 was `BUILT` and has one, 55 was `PARTIAL` and has none, and the gate rejected
the first draft of this rule within a minute of it being written.

`REFUSED` was minted 2026-09-20 (calef), with the same complaint behind it that minted `REMOVED`:
the existing words could only lie. This tree carried 140 `- **Refused.**` bullets across 98 milestone
blocks, and exactly two of them named a milestone, so a refusal that named real work lived in the
last section of a finished block and was read once. `RECORDED` was the nearest word and it says
something else: it means the analysis was captured and the work deliberately not taken, and it
carries no condition and nothing asks it for one. The whole value here is the condition, so a status
that did not demand one would have reproduced the bullet in a bigger font. `NOT-STARTED` says pending
where this says declined, and `OPTIONAL` reads as available to pick up.

It carries an extra obligation nothing else does: a `## Revisit` section, whose bullets open
`**Condition.**` (what would make this worth reopening), `**Nothing.**` (refused permanently, on a
principle) or `**Unstated.**` (the original names no condition and none can honestly be inferred).
Silence is the one answer that is not allowed, because silence and "we thought about it and nothing
would change our minds" look identical to the next reader and mean opposite things. The condition is
not a promise to build, which is exactly what §71 (A limitation is promoted when it stops being a fact and becomes a plan) refuses to let a `BUGS` entry become; it is the
difference between a dead end and a door with a bell on it, and `script/roadmap --revisit` is the
bell. See milestone 448 (a refusal gets a number, a status, and a condition that would change it).

`IN-PROGRESS` earned its extra rule by being wrong every single time it was used (2026-08-17). A
sweep checked all six rows carrying it and found six false: milestones 58, 80, 112 and 115 had
merged, 115 thirteen days earlier, and 47 and 117 named no branch because nobody held them. The
same sweep audited nine `NOT-STARTED` blocks against the artifacts they named and found all nine
honest, so this is not general rot. It is one token, and the reason it rots is structural rather than
careless: it is a manually-maintained cache of a fact that lives somewhere else and expires on its
own. Merging a lane deletes its branch, which is exactly the moment the token becomes false, and
nothing was watching. Naming the branch is what makes the claim falsifiable at all, which is why it is
required rather than suggested.

The stronger fix is to retire the token, and that is an architect's call rather than a lane's. §90 (the claim is a draft pull request; the status flip is a gate) made a
lane's first act a draft pull request, so `gh pr list --draft` already answers "who is on this right
now" and cannot go stale, because merging removes the row. A status token duplicating that fact is the
lower rung by construction. Until that is decided, the check above is the tripwire.

The `Built` column is the date a milestone turned `BUILT`, and it is empty for every other status but `REMOVED`, which keeps it: the milestone did turn `BUILT` on that date and nothing later makes that untrue. Dates in this tree are UTC (calef, 2026-08-04), because they arrive from three
sources that disagree: a git author date is local to whoever committed, a lane deriving a date reads
whatever `git log` gives it, and a maintainer typing "today" means their own midnight. An evening
commit in California is already tomorrow in UTC, which is how milestone 22 (trusted init: verify it, and shrink what a broken one can do)'s row came to read a day
ahead of the commit that flipped it and looked like a defect. One zone, stated here, and the
disagreement stops being a puzzle.

### Dependency fields

The status says where we stand; it does not say what stops a lane from starting. That question
was answered in conversation until 2026-08-04, then by a `Gate:` line, and since 2026-09-26 by five
fields, which are §207's:

| field | says |
|---|---|
| `milestone_dependencies` | Work this roadmap tracks that this needs first, by number. |
| `decision_dependencies` | Sections in `design/decisions/` this waits on, or `unwritten` for a fork nobody has written up yet. |
| `machine_requirements` | A capability an emulator cannot provide ("riscv64 silicon with an ASID-tagged TLB"). Any host with it will do. |
| `specific_machine` | One machine, only where that machine is the point, with the reason: `argon (the Jetson TX1 under seL4's published numbers)`. |
| `needs_person` | `yes` when somebody has to be at a machine or an account: flashing, a serial console, a firmware setting. |

`unwritten` is a recorded limitation. 41 blocks carried a bare `DECISION` gate at the switch,
naming no section, so each is waiting on a fork nobody has written up. The honest value is the
word, not an invented section; writing one up and replacing the word is how it closes, and the full
report lists them under `DECISION`.

A dependency is never cleared. It stays true after its target lands, and whether it still
blocks is computed from the target's own status. The `Gate: MILESTONE N` line this replaced failed
the build the day N turned `BUILT`, which obliged an edit in a block that nobody in the landing
lane was reading, and turned unrelated branches red (§207's argument). A decision works the same
way: once its section is ruled, it no longer blocks, and nothing is edited to make that so.

`HARDWARE` was four things, and the three machine fields are why. The old token covered a board
not yet here, a board here that needs somebody at it, one specific machine for a like-for-like
comparison, and a rented account. The one that never discharges by waiting is the person, and a
field named for hardware hid it from every count: 33 of the 34 `HARDWARE` gates turned out to need a
person. The migration read all 34 by hand.

`script/roadmap --ready` offers a milestone when its status is `NOT-STARTED` or `PARTIAL`, every
dependency is resolved, no machine is required and nobody needs to be present. The full report groups
what is not ready by what it waits on.

Follow-on vocabulary: the `## Follow-on` section every finished block answers (milestone 247 (follow-on work named by a finished milestone goes nowhere, and this is the third time),
2026-09-03). The dependency fields say what stops a milestone starting; this says what happened to the
work the milestone named on its way out, and it exists because that work is the thing this project
keeps losing. Three times on the record: milestone 90 (A guard page under the per-CPU secondary stacks) exists only because calef happened to be at
his desk the day a report named its finding, milestone 94 (the untracked-work sweep, and the convention that ends the category) swept the tree for this exact category and
left its own inventory in a pull request body for twelve days, and milestone 244 (the largest crate in the tree is proved by nothing a mutation can reach) named an
unvouched-binary hazard and a design fork that surfaced only because calef asked for them by hand.

A block whose status is `BUILT`, `REMOVED` or `PARTIAL` carries the section, and
`script/roadmap --check` fails on one that does not. It is a bullet list, and each bullet opens with
one of eight dispositions:

| Bullet opens | Means | Must resolve to |
|---|---|---|
| `**None.**` | Nothing was identified. | Nothing; it stands alone as the whole answer |
| `**Milestone N.**` | It became milestone N. | A block under `design/roadmap/`, not this one |
| `**Done.**` | It was done, and not as a milestone. | What carried it: a pull request, a branch, a file |
| `**Recorded.**` | It is a limitation and it stays one. | Prose, and any path it cites must exist |
| `**Refused.**` | Considered and deliberately not taken. | A reason, in prose |
| `**Decision.**` | It is an architect's call, written up as one. | A file under `design/decisions/` |
| `**Proposed.**` | Named, nobody took it, so it is now a proposal. | A file under `design/roadmap/proposals/` |
| `**Outstanding.**` | Still this milestone's own remaining scope, checked against the tree and still true. `PARTIAL` blocks only. | What is left, and how you checked |

`PARTIAL` was added by milestone 252 (A `PARTIAL` block claims work is remaining and nobody re-reads it), and it is the harder half. A finished block is written
once and closed, so its section is a record. A `PARTIAL` block's is a standing claim about the
future, edited as pieces land, and nothing re-reads what it still asserts. Its prose is also what a
lane reads when deciding what to pick up, so a stale one does not merely misinform: it offers work
that does not exist. Milestone 16 (real hardware and IOMMU-backed driver isolation) listed three things as remaining, two of them were finished, and
the block said otherwise for weeks.

`Outstanding.` is the word a `PARTIAL` block needs that a finished one does not. The other seven
say where work *went*; a `PARTIAL` block's commonest honest answer is that it has not gone anywhere
and is still this milestone's own scope. None of the seven can say that without lying, and a lane
forced to choose would write the comfortable word or leave the item out, which is the silence this
gate exists to stop arriving through the gate. It is refused on a `BUILT` or `REMOVED` block, where
`Recorded.`, `Proposed.` and `Milestone N.` already cover the ground and a softer synonym would only
give them somewhere to hide.

`None.` is refused on a `PARTIAL` block, which is the one thing this gate can prove rather than
merely enumerate. `PARTIAL` means work remains, so a block answering that nothing is outstanding has
contradicted its own status word: either name what is left, or the status is stale and the row wants
`BUILT` with a date. That is the milestone-69 defect (row and block disagreeing) one level in.

What the gate can and cannot do is unchanged. It checks that the claims are enumerated and that
each resolves to something that exists. It cannot check that a claim is still true. What catches
staleness is a person writing the dispositions out, which is what both sweeps demonstrated: 247's
found three items already built, and 252's found more.

An explicit refusal is a success here. The defect being attacked is silence, not the absence of
a milestone, and `**None.**` is meant to be the cheapest sentence in the roadmap.

It does not weaken the `BUGS` convention and cannot. `**Recorded.**` points at a limitation
recorded beside the feature; it never replaces one. An over-strict version of this rule, where every
observation had to resolve to a milestone, would make the honest thing expensive to write and thin
the `BUGS` sections out, which costs more than the burial does. Nothing here reads prose looking for
intent: AGENTS.md priced that at `git grep -w TODO`'s 82% false-positive rate.

`Proposed.` is the one that carries this milestone's whole subject, and it went through two
shapes in a day. A block that wrote its follow-on work down honestly and had nobody pick it up fits
none of the other words: `Recorded.` lies about intent and `Refused.` lies about the decision, so a
lane forced to choose writes the more comfortable one. Three lanes that would not lie left the item
out instead, which is the burial arriving through the gate. It was first spelled `Unclaimed.` and
took only prose, because a lane could not mint a milestone number and had nothing to point at. That
constraint went the same day, and the disposition now resolves to a file somebody wrote.

`Done.` came from the same afternoon and four lanes, for work a block named that two ordinary
commits then finished. Each of those lanes resolved it by leaving the item out, which is the silence
this gate exists to stop, arriving through the gate itself.

The whole vocabulary is provisional until calef ratifies it. `REMOVED` was minted by him and
these six words are a lane's, offered with the sweep that produced them.

## Anybody may add to the roadmap: `design/roadmap/proposals/`

Ratified by calef on 2026-09-03, and it dissolves a rule this tree had been treating as one thing.
A lane writes a proposal itself, as `design/roadmap/proposals/<slug>.md`, with no number in the
name. No coordination, no maintainer in the path, no waiting.

*"On a human team, anybody should be able to add to the roadmap. That's different than prioritizing
that roadmap."* Lanes were barred from minting because concurrent lanes cannot see each other and
two reaching for the same number collide. The collision is in the number, not in the authority,
and conflating them meant a lane that found work had to route it through a report, through the
maintainer, into a decision that might be deferred. Every hop is a chance to lose it, and on the day
this landed the maintainer had buried three items by deferring them into chat messages. Prioritizing
the roadmap is still calef's, and so is every number and every name.

Why a slug and not a GUID, which was considered and rejected. Milestones are cited in prose
constantly and a GUID cannot be said out loud. This tree already found bare numbers too opaque, which
is why it cites a milestone by number *and* name; a GUID moves further along the axis that already
needed correcting. The slug is the readable half of a filename this directory already uses, so
nothing new has to be learned, and two lanes collide only by choosing the same words, which is
visible rather than silent. Numbers stay for promoted blocks, where they also carry recency for free.

A proposal carries the same frontmatter a numbered block does, with `status: PROPOSED` and the
date it was written as `raised`, so promotion changes almost nothing:

```markdown
---
status: PROPOSED
raised: 2026-09-03
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# <Title, the way a numbered block is titled>

Written by <who or what>, from milestone <N>'s block.

**In brief.** <what the work is>

Reuse: <what existing code was considered, and why it was or was not taken>
```

Promotion is the integrator's, at merge, like every other global name: give the file its number,
`git mv` it up a directory, set its `status`, and add `promoted_from` with the slug. `script/roadmap --check` validates the rest, and nothing else has to
happen, because there is no table for it to appear in.

The graveyard question, asked here rather than discovered later. A proposal nobody promotes is
the same burial in a new location. Nothing can force a promotion, and a gate that tried ("no proposal
older than N days") would be routed around by not writing proposals, which is worse than the pile.
What the gate does instead is make the pile impossible to miss: `raised` carries the date it was
written, `script/roadmap --check` prints the count and the oldest date on every lint run, and
`--proposed` lists them oldest first. An unread number is still rung two, where a paragraph in a
finished block was rung four.

The dependency fields say what stops a milestone's headline deliverable, and several milestones
have a startable piece behind a blocked headline: milestone 88 (nife on rented silicon)'s stage 1 boots UEFI locally with no cloud account. The prose says so where that is true,
and no gate checks it.

The converse is a rule. A dependency on part of a milestone means that milestone is too coarse:
split it, so the field names the part (calef, 2026-10-06).

## Effort

Effort, calibrated from git history (2026-07-30), not guessed. The milestone files give effort in
lanes: one lane is one agent session end to end. Measured across the fourteen milestone branches
merged so far, a lane is 31 to 57 minutes of wall clock, 1 to 9 commits, 7 to 30 files, and 694 to
4,351 inserted lines: a much narrower band than the work's apparent ambition suggests, since
proving the DMA boundary, building a compositor, and confining a C component all cost about the same.
Milestones that took more than one lane took them as *phases that landed separately* (27 and 30 took
three each, 22, 29, 31 and 35 two each), not as one long push.

This replaces an S/M/L scale that was written before any of it was built and was systematically
pessimistic where it can now be checked: 27, 29, 30 and 32 were each labeled "Effort L", and each
came in at roughly a lane per phase. Anything still labeled by feel rather than by history says so.
Re-derive with `git log --first-parent` over the merge commits rather than trusting these numbers as
they age.

## Why this is markdown in the tree and not GitHub Issues

Decided 2026-07-30 by calef. Issues would buy PR
linkage, a home for discussion, and a board view. They would cost the things this table is actually for:
the roadmap stops being version-controlled alongside the code, so a status change is no longer a diff in
the commit that caused it; `script/roadmap --check` has nothing to validate, and that gate is what caught
milestone 34 (gPU acceleration via virtio-gpu 3D (the display ladder's rung four)) having no row; and the cross-references to `DECISIONS §N` and `notes/*.md` decay from one
grep into URLs. The deciding argument is that a second place where status lives is a second source of
truth, which is the exact failure this project spent 2026-07-30 cleaning up: `bench.rs` contradicting
notes/benchmarks.md about what `fs_read` measures, status prose phrased a dozen ways that made a sweep
mis-report eight milestones, and §27 (the filesystem service: a capability-shaped contract)
corrected four times. The linkage only starts paying when more than
one person files work, so revisit if that changes: with external contributors, the shape would be
markdown canonical and issues generated one-way from it, never synced back.

Note also that GitHub's own *Milestones* feature is a name collision with this list and a poor fit
besides, being built for dated release grouping; these are capability-shaped and deliberately undated.

## The index, and why it is gone

The table was hand-maintained until 2026-09-14, generated from the blocks until 2026-09-21, and is
now not built at all. The two changes had different reasons and the second did not undo the first:
generating it fixed a merge problem, and retiring it answered a readability one. Five hundred rows of a
five-column layout with paragraph-length cells is not a document anybody reads, and a derived
document nobody reads is work the tree was doing for itself.

Nothing was lost in the retirement, and that is only true because of the migration. Milestone
294 had already moved the two hand-written columns into the blocks, so by the time the table went,
every cell in it was a rendering of text that lives in a milestone's own file. Had the order been
reversed, deleting the table would have deleted 288 hand-written summaries and 288 dates.

What survives is the rendering, not the file. `script/roadmap --index` still assembles the five
columns to stdout, because `script/audits` counts BUILT rows for its cadence trigger and
`script/fatal-risks` reads Built dates for its "as of" check. Both had been moved off the committed
file onto that feed by milestone 443 (lanes wait on each other for three reasons, and none of them
is the work), which is why retiring the file needed no change in either of them.

The rest of this section is the record of the generation, kept because the measurements in it are
the evidence that the retirement cost nothing.

### Why it was generated first

Measured on 2026-09-14, in one session: eight merge conflicts, every one of them in
`design/roadmap/README.md`'s index table and zero anywhere else. Pull requests #836, #843, #847,
#849, #850, #851, #854 and #855, several more than once, because `main` moving even once
re-conflicted every open lane.

It was structural. `script/lint` check 4b requires every lane to touch its own milestone's block;
every milestone also needed a row in one sorted table; so every milestone in flight collided with
every other milestone in flight, always, in that one file. `AGENTS.md` names
`system_tests/src/user/tests.rs` as the hotspot to fear, measured on 2026-08-16, and that is the wrong
file: only some lanes wire a test, and every lane adds a row.

The strongest argument is that `script/roadmap` exists because of this table. Its header records
that milestone status used to be prose phrased a dozen ways, that a sweep mis-reported eight
milestones, and that *"milestone 69's row said NOT-STARTED while its block said BUILT"*. One of its
checks was that the row and the block must agree. Generating the row makes that check vacuously
true, which is rung one of `AGENTS.md`'s ladder where the gate was rung two.

### The two columns that had to move, and the measurement that decided it

Three of the five columns were already derivable: the number is the filename's, the status is the
`**Status:` line the script already read, the title is the H1 it already read. The other two were
not, and both options for avoiding a migration were measured before they were refused.

Deriving the summary from the block's opening paragraph. Refused on evidence. Across 288
milestones the row's summary scored a median similarity of 0.07 against the block's first body
paragraph; three scored above 0.4 and none above 0.6. An opening paragraph is written to open a
document rather than to summarize one. Taking it would have rewritten what the index said about 288
milestones in a single commit, and those cells are the tree's own account of its history.

Reading the Built date out of the status sentence. Refused on evidence. 129 of 288 blocks
did not state the date anywhere a parser could find it, and the ones that did used at least eight
spellings:

```
**Status: BUILT.**                                    (the date only in the index)
**Status: BUILT, 2026-08-26.**
**Status: BUILT (2026-08-01).**
**Status: BUILT** 2026-09-14.
**Status: BUILT** (2026-08-03).
**Status: BUILT** on 2026-08-18 (PR #320).
**Status: BUILT**, 2026-08-27, as `rmle` rather than ...
**Status: REMOVED 2026-08-30.** Built 2026-08-17, and ...
```

Canonicalising 129 status lines is rewriting prose in 129 blocks to serve a parser, which is the
wrong direction and is not a thing a lane may do.

Generating only the derivable columns and leaving the summary in the table was the third
option, and it solves nothing: a lane would still edit one shared file to add its summary, so the
whole conflict class survives and the milestone would not have paid for itself.

#### Would we still choose this if both options cost the same?

Yes, and it is worth answering out loud because the chosen option is much the most expensive
of the three: it migrated 288 files where deriving from the opening paragraph would have migrated
none. The reason is not effort in either direction. Deriving would have changed what the index says
about 288 milestones, silently, in the direction of text that was never written to be a summary.
Moving text is safe; regenerating it is not.

### The method, and why the proof is reconstruction

The migration moved each cell's text into its block, and the proof was that the table regenerated:
271 of 288 rows byte for byte, and all 17 differences pre-existing drift in the hand-maintained
table. Milestone 294 (the index is generated, not hand-maintained) has the itemized diff. Verify by
reconstruction, not by inspection, is the transferable part, and milestone 596 (the roadmap blocks
get frontmatter too) proved its migration the same way.

## What the gate does now, and the one thing it deliberately stopped doing

## BUGS

- `## Index row` is named after a thing that no longer exists. The section is still where a
  block states its Built date and its one-paragraph precis, and both are still read, so the name is
  wrong rather than the section. It appears in every numbered block in `design/roadmap/`, and a
  rename is an architect's (`design/naming.md`). Until then, read it as "the two facts about this
  milestone that are not in its prose".
- `## Index row` was provisional when it was minted, too. `## Summary` is the generic word this
  tree's naming tenet warns against, and `## Why it matters` was unavailable: 13 blocks already
  carry one, holding a different kind of text.
- The section is one paragraph, and a gate enforces that. It was one paragraph because a table
  cell is; the table is gone and the constraint stayed, because relaxing it is a change to every
  block for a block that has not wanted two paragraphs yet.
- `script/metrics` and `script/catch-up` read the committed table at historical revisions, and
  those revisions still have it. For revisions after 2026-09-21 both fall back to counting the
  blocks. The two answers are not identical for the weeks in between: a week whose revision carried
  a stale table counts what the table said, which is what that week's reader would have seen.
- Two lanes minting the same milestone number still collide. The shape is two files claiming
  one number, which `script/roadmap` has always called fatal and which `script/decisions` records
  as the reason a directory beats a single file.
- Nothing publishes any of this. calef's sentence retiring the table named a website as where
  the project plan eventually goes; there is no publishing story today, and the gap is written up
  in `design/roadmap/565-a-website-for-the-project-plan.md`.
