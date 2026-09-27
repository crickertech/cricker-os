# Follow-on work, and what happened to it

*Name: provisional (milestone 247 (follow)). It names the thing it tracks, per §75 (provenance)'s noun rule, but calef
names notes as he names everything else in the tree. The six disposition words below are provisional
in the same way.*

A milestone finishes and, on its way out, names work it is not doing. A hazard it noticed, a design
fork it could not settle, a second phase somebody should take. That work is what this project keeps
losing, and this note is the mechanism that stops it, plus the sweep that proved the mechanism was
needed.

## The failure, three times

- Milestone 90 (guard) exists only because calef happened to be at his desk the day a lane report named
  its finding. Nothing else would have caught it.
- Milestone 94 (untracked) swept the tree for exactly this category, and then left its own inventory in a
  pull request body for twelve days. By the time anyone came back, the item-level list was gone and
  had to be re-derived (notes/untracked-work-sweep.md).
- Milestone 244 (largest), on 2026-09-03, named an unvouched-binary hazard in a `BUGS` section and a design
  fork in a handoff paragraph. Both surfaced because calef asked *"any work to follow up on 244?"* by
  hand, which is the mechanism today and is rung zero: somebody has to remember to notice.

AGENTS.md carries the rule. *"Identified work leaves the lane in a tracked form, or the merge
waits."* What it does not carry is anything that fires when it is not followed, and the rule as
written is rung four: a lane report is read once, by one person, on the day it is written.

## The mechanism: a section a finished block has to answer

Every BUILT, REMOVED or PARTIAL milestone block carries a `## Follow-on` section, checked by
`script/roadmap --check` and therefore by `script/lint`. (`PARTIAL` is milestone 252's addition,
below.) The dispositions are tabulated in notes/roadmap.md, which is where a block author
meets them. In short: `None.`, `Milestone N.`, `Done.`, `Recorded.`, `Refused.`, `Decision.`,
`Proposed.`, and `Outstanding.` for a `PARTIAL` block's own remaining scope.

Why it hangs on the status rather than on a marker in prose. The moment a block turns BUILT is
the last time anyone reads it on purpose, so it is exactly the moment the work gets buried. It is
also a state a script can see, which prose intent is not: AGENTS.md priced the greppable alternative
at `git grep -w TODO`'s 82% false-positive rate, and a check that cannot tell an observation from an
intention gets disabled within a week. Nothing here reads a block's prose looking for intent. It
checks that a finished block was asked the question, and that whatever answer its author wrote
resolves to something.

This is the third instance of a shape the tree knows, which is most of the argument for it:
`script/lint` fails a `TODO` that does not name a milestone, `script/citations` checks that a glossed
citation is grounded in the document it names, and this checks that a disposition resolves to a
block, a file, a reason, or a decision.

`None.` and `Recorded.` are deliberately the cheapest things to write. An over-strict version of
this gate is worse than the burial it prevents: if every observation in a `BUGS` section had to
resolve to a milestone, the honest thing to write becomes expensive, people write less, and the tree
gets worse. `Recorded.` points at a `BUGS` entry and never replaces one. The FreeBSD posture is
upstream of this check and is not touched by it.

An explicit refusal is a success. The defect this attacks is silence, not the absence of a
milestone.

## `Proposed.` is the disposition the sweep asked for, and it is the point

Four of the words were designed before the sweep. `Unclaimed.` and `Done.` were not: three lanes
asked for the first and four for the second, independently, on the same afternoon, having hit the
same wall. A block that wrote its follow-on work down honestly, in the place a reader meets it, and
had nobody pick it up, fits none of the original words. `Recorded.` lies about intent and `Refused.`
lies about the decision, so a lane forced to choose writes the more comfortable one, and the lanes
that would not lie left the item out instead. That is the silence this gate exists to stop,
arriving through the gate itself.

Then the constraint that made `Unclaimed.` necessary was removed, the same day. calef ratified
`design/roadmap/proposals/<slug>.md`: a lane may add to the roadmap itself, with no number and no
coordination, because *"on a human team, anybody should be able to add to the roadmap. That's
different than prioritizing that roadmap."* The collision that barred lanes from minting was in the
number, never in the authority, and separating the two makes the burial problem much cheaper to
solve. A mechanism whose cost is "write one file" is one people will use.

So `Unclaimed.` became `Proposed.`, and it now resolves to a file somebody wrote rather than
to a sentence promising that work exists. That is strictly stronger, and it is why the count of
proposals no longer has to be small: the third bucket has somewhere to go.

The economics this changes are the whole reason milestone 247 recurred three times. Before, a
lane that found work wrote it in a report, the maintainer read the report, and an item either became
a milestone or became a chat message. On the day this landed, the maintainer buried three identified
items that way, all by deferring rather than forgetting, and calef caught all three by asking. The
mechanism has to assume the maintainer is a failure point and not only the lanes, and routing the
work into the tree at the moment of discovery is what does that.

## The sweep, 2026-09-03

All 139 finished blocks were read, in twelve parallel lanes, roughly 159,000 words. Not skimmed:
each lane read its blocks in full, including `BUGS` sections and handoff prose, and resolved
successors against the index rather than guessing.

605 dispositions were written. The shape of them is the interesting part:

| Disposition | Count | What it says about the tree |
|---|---|---|
| `Recorded.` | 291 | The `BUGS` convention is working. Most follow-on prose is an honest limitation sitting where a reader meets it |
| `Milestone N.` | 136 | The successor usually exists, and the block usually cited it by number |
| `Refused.` | 105 | Refusals with reasons, which this convention counts as successes |
| `Proposed.` | 38 | Work named, nobody holding it, now a file in `design/roadmap/proposals/` |
| `Decision.` | 24 | Forks written up under `design/decisions/` |
| `None.` | 7 | Almost all of them backfilled history blocks from milestones 1 to 11 |
| `Done.` | 3 | And all three were `Proposed.` first. See below: the word paid for itself within the hour |

Two things the sweep found that were not in its brief.

`Recorded.` bullets citing paths surfaced real rot in the roadmap's own prose: milestone 136 (decision)'s
pin still named `crates/regions` after §113 renamed it to `crates/memory_regions`, 136's free-site
pin still named `kernel/src/untyped.rs` after milestone 135 moved it, and 124 still named
`crates/slots`. Nothing had been checking a path cited in a roadmap block.

And a block written after this convention existed is far cheaper to sweep than one written before
it. Blocks 196, 216 and 234 carried their follow-on work in a legible shape, usually citing
the successor by number. The older blocks name it only by implication, and cost several times the
reading per bullet. That is the case for the gate as well as any argument in this note.

There was an exemption list, for about an hour. The first design let a block that finished before
the sweep sit on a list here instead of carrying a section, with a date cutoff so nothing new could
be parked. The sweep then reached every block, the list closed to zero, and an empty list that
permits parking is exactly the hiding place its own `BUGS` entry had called a foot gun. It is gone
rather than kept at zero.

## What the sweep found, and which of it to promote first

38 pieces of unclaimed work are now proposal files under `design/roadmap/proposals/`, one each,
written by the lanes that understood them. That is the answer the morning's design could not give:
with a proposal costing one file and no coordination, the count does not have to be small, and
nothing has to be argued down to fit a budget.

Three of the 42 were done, and finding that out is the sweep's sharpest result. A lane
told to write a proposal has to go and look at the tree, which nothing had made anyone do before:

- Milestone 54 (network) asked for a roadmap status word meaning "built, then removed". calef minted
  `REMOVED` on 2026-08-30, the same day the code went, and the block's own status line reads it.
- Milestone 57 (partitioning) wanted partitioning on the target. `components/src/disk_partitioner.rs`, commit
  `4db2fd74`, holds a disk endpoint and an entropy endpoint, draws four v4 GUIDs and writes both
  table copies.
- Milestone 57 wanted `mkfs` on the target, and called the `vendor/redoxfs` divergence calef's to
  take. It was taken and documented: `Header::new_with_uuid`, divergence 4 in `vendor/README.md`,
  carried as `patches/redoxfs-no-std-create-uuid.patch`.

Two of those had sat stale for a month inside a `## Follow-on` section, which is this milestone's
own failure one layer in: the section says what happened to the work, and nothing checks that it is
still true. They are `Done.` now, with what carried each. `Done.` was added that morning because four
lanes said the word was missing, and it earned its place before the day was out.

Promotion is a different act from proposal, and it is an architect's. Five of the 42 are worth
an architect's attention first, on a stated bar: the work is concrete and bounded, nothing in the
tree owns any part of it, and leaving it unpromoted costs more than tidiness, because a claim this
project makes rests on it or a record in the tree is now known to be wrong.

1. The three instruments nothing runs. `script/interleaving-check`, `script/crate-probes` and
   `script/rule-violations --check` are referenced by neither CI nor `script/ci-build`. `crate-probes`
   is the instrument behind fatal risk 1, which is recorded GREEN on whatever somebody saw when
   they last ran it by hand. Named by milestone 232 (audit).
2. Replay the kernel falsification records. Six of milestone 202 (every)'s confinement claims have no
   replay mechanism, so they are claims resting on nothing executable. Milestone 212 handed this back
   explicitly and said it stays milestone 210's.
3. Two address spaces drive one UART with nothing arbitrating. The kernel prints its boot tour
   and its fault reports through its own driver while the userspace `console` server drives the same
   device, so output interleaves at byte granularity. It affects every bench session on argon, radon
   and xenon, and milestone 243 (serial)'s `BUGS` tells readers it "has its own home", which did not
   exist until today. Named by milestone 230.
4. Attach the rest of the x86_64 test fixtures. The RedoxFS image, the GPT and blank disks, the
   NIC, the GPU, the keyboard and the RNG. Its measure is the 36 tests taking a "no RedoxFS disk
   attached" arm, and architectural parity is a gate in this tree rather than an aspiration. Named by
   milestones 215 and 176 independently, which is itself evidence.
5. **The records that went stale when the number under them moved.** `design/fatal-risks.md` risk 3
   still reads MEASURED green on 92.4% mutation score against a published 83.4%, and AGENTS.md
   says `kernel/src` measures 40% comments against a measured 45.3%. Both are records this project
   asks strangers to trust.

**The strongest of the rest, so the cut is visible**: bounding what one mutant may allocate. One
mutant goes from 1.4 GB to 15.8 GB in twenty seconds and takes the machine with it, and milestone
238's block prices three shapes for the fix. It is below the line only because nothing the project
claims publicly depends on it.

## The second sweep, 2026-09-03: `PARTIAL`, and why it is the harder half

Milestone 252, the same day. 247 covered `BUILT` and `REMOVED`. **`PARTIAL` was outside it, and it
is the status where the claim is most likely to be false**, for a reason that is structural rather
than about care:

**A finished block's section is a record. A `PARTIAL` block's is a standing claim about the
future.** It gets edited as pieces land, nothing re-reads what it asserts, and its prose is
what a lane reads when deciding what to pick up. So a stale one does not merely misinform; **it
offers work that does not exist.**

Milestone 16 is the case that minted the milestone. It listed three things as remaining and two were
finished: the on-board test-suite exit is the UART marker plus SBI SRST that every board run is
judged by, and the DTB-driven UART IRQ printed `source 32` on all nine boots of the 2026-09-03
series. It said otherwise for weeks, and calef asking what was left is what caught it.

### The result: 22 of 22

Six lanes read all 22 blocks in full, about 8,500 lines, checking each claim against the tree rather
than against the block's own prose. **215 dispositions written; 56 of them record a claim the tree
disproved; every one of the 22 blocks carried at least one.**

That last number is the finding. The expectation going in was a long tail: a few rotten blocks and a
majority that were fine. There is no such majority. Three of the blocks the lanes singled out as
**unusually well maintained** each carried stale items too.

**Three shapes recurred, and none of them is carelessness:**

- A block that accretes lane reports keeps its old questions in the present tense. Milestone 139 (drive)
  is eight rounds appended in order, and its "what is still open" section opens with a sentence its
  own `BUGS` refutes 140 lines later. Every round was honest when written.
- A block written before a decision landed keeps citing the fork. Milestone 142 (text) waited on a font
  and a palette that DECISIONS §104 chose on 2026-08-20, seven days before that block was last
  edited, and it cites §104 nowhere.
- **A block's premise can be deleted out from under it.** Milestone 152's "what was built" section
  cites files the SMB removal deleted on 2026-08-30.

### `Outstanding.`, the eighth word

Seven dispositions say where work *went*. A `PARTIAL` block's commonest honest answer is that it has
not gone anywhere and is still that milestone's own scope, and none of the seven can say it without
lying: `Recorded.` claims a limitation that stays, `Proposed.` claims nobody owns it when this block
does, and `Milestone N.` cannot name the block it is written in. A lane forced to choose writes the
comfortable word or leaves the item out, which is 247's own `Done.` and `Proposed.` lesson arriving a
second time.

**What it buys is the date, not the word.** The gate still cannot tell a stale claim from a live one.
What changed is that somebody wrote "still true, and here is what I checked" beside 82 items.

**`None.` is refused on a `PARTIAL` block**, which is the one thing the gate can prove rather than
enumerate: `PARTIAL` means work remains, so a block answering that nothing is outstanding has
contradicted its own status word. That is the milestone-69 defect, a row and a block disagreeing,
one level in.

**`script/roadmap --outstanding`** prints what is left, per `PARTIAL` milestone, derived from the
blocks. It is the question calef had to ask by hand, answered as a command.

### The wrong sentence is rarely only in the roadmap

Fixing a block repeatedly left a rustdoc, a module header or a note still saying the old thing, where
no gate reads it. Ten of those are listed in
`design/roadmap/333-claims-the-sweep-found-false-outside-the-roadmap.md`: a rustdoc that says
the address-space object does not consult `ENUMERATE` yet, a decision file that quotes a roadmap
block saying nobody is building the decision, six comments citing a type deleted four days earlier.
This is the same rot 247 found through `Recorded.` path citations, one layer out, and nothing
mechanical would have caught any of it.

## BUGS

- A disposition can be wrong and this gate cannot tell. `**Milestone 240 (soak).**` resolves whether or
  not milestone 240 is the work the block meant, which is the same blind spot `script/decisions`
  records for `§N` citations and `script/roadmap` records for its own tree-wide citations. Check by
  content after any renumber.
- `Proposed.` and `Recorded.` are separated by judgment, not by anything checkable, and lanes
  said so. A `BUGS` entry that is a permanent caveat and one that is deferred work read identically
  in prose, and the same text can defensibly take either word. Expect the backlog to be
  under-counted rather than over-counted, because `Recorded.` costs a sentence and `Proposed.` costs
  a file.
- A well-written block generates more unclaimed items than a lazy one. A block that named its
  follow-on work honestly has something for the sweep to find; one that named nothing looks clean.
  Do not read a block's bullet count as a quality signal, in either direction.
- It cannot find work that was never written down. A hazard named only in a chat window or in a
  lane report nobody landed is not in the tree to be found. The count of what has been lost
  that way is unknowable rather than zero.
- A `PARTIAL` block's section fires more than once and is therefore never finished. The gate
  demands it the day the status turns `PARTIAL` and never again, so an item that goes stale the
  following week is stale until somebody re-reads it. 252's sweep is a date, not a mechanism, and
  the next one will find the same thing.
- It fires once, at the moment a block finishes. Follow-on work identified *after* a block turned
  BUILT lands in a block nothing will re-check, because the section exists and passes. The `BUGS` convention and a `TODO(milestone N)` marker both still work there; this gate
  does not add to them.
- Nothing promotes a proposal, and nothing can. A proposal nobody picks up is the same burial in
  a new location. A gate on age ("no proposal older than N days") would be routed around by not
  writing proposals, which costs more than the pile does, so what ships instead is visibility: the
  status line carries its date, `script/roadmap --check` prints the count and the oldest on every
  lint run, and `--proposed` lists them oldest first. That is a number somebody has to choose to
  ignore, which is weaker than a gate and much stronger than a paragraph.
- A well-stocked `proposed/` directory can flatter the project. Forty-two files look like a plan
  and are not one. They are a list of things nobody is doing, and the honest way to read the count is
  as debt.
- **A `Recorded.` bullet cannot quote a path-shaped example.** The path check treats a backticked
  span beginning with a real top-level directory as a citation, so a bullet illustrating
  `script/foo --bar` as a command rather than as a file has to spell it without backticks. That is
  a real constraint on prose in a project whose subject is filenames, and it is the price of
  catching the rot above.
