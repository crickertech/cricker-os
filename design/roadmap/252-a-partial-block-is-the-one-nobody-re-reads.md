---
status: BUILT
raised: 2026-09-03
built: 2026-09-03
---
# 252. A `PARTIAL` block claims work is remaining and nobody re-reads it

Built on 2026-09-03. Minted the same evening milestone 247 (follow) shipped, by calef, after a
`PARTIAL` block was found describing as remaining two things the tree had finished. Built the same
day: all 22 `PARTIAL` blocks swept and answered, `Outstanding.` added to the vocabulary, and
`script/roadmap --check` extended to refuse a `PARTIAL` block with no section.
*(Number provisional until the merge queue lands it.)*

In brief. Milestone 247 made every `BUILT` and `REMOVED` block carry a `## Follow-on` section
saying what became of the work it named, and `script/roadmap --check` fails a block that does not.
`PARTIAL` is not in that set, and it is the status where this failure is most likely.

Milestone 16 (real hardware + IOMMU-backed driver isolation, RISC-V first) is the worked example and
the reason for this block. It listed three things as remaining. Two of them were done: the
on-board test-suite exit is the UART marker plus SBI SRST that every board run is judged by, and the
DTB-driven UART IRQ printed `source 32` on all nine boots of the 2026-09-03 series. The block said
otherwise for weeks, and it was caught by calef asking what was left rather than by anything in the
tree.

Why `PARTIAL` is the worse case, not the lesser one. A finished block is written once and closed.
A `PARTIAL` block is a *standing claim about the future* that gets edited as pieces land, and nothing
re-reads what it still asserts. Its prose is also the thing lanes read when deciding what to pick up,
so a stale one does not merely misinform: it offers work that does not exist.

22 blocks are `PARTIAL` today. Milestone 16 is one. Nobody knows how many of the other 21 are in
the same state, and that number is the first thing this milestone should produce.

## What "follow-on" means for an unfinished block, which is the design question

For a `BUILT` block the question is *what happened to the work this named*. For a `PARTIAL` block it
is what is still outstanding, and is that still true, which is a different sentence and may want a
different section or different words. Milestone 247's seven dispositions may fit as they are: `Done.`
already exists, and it is exactly what milestone 16's first two items needed.

Resolve that before sweeping, and say which way it went. Reusing `## Follow-on` unchanged is the
cheaper answer and probably the right one, since a second section name is a second thing to remember;
but if the words do not fit, forcing them would produce dispositions that lie in a new way.

What the gate can and cannot do is unchanged from 247 and should not be overclaimed: it can check
that the claims are enumerated and that each resolves to something that exists. It cannot check that
a claim is still true. What actually catches staleness is a person writing the dispositions out, which
is what 247's sweep demonstrated when it found three items already built, two of them stale inside a
`## Follow-on` section for a month.

## The proof that this milestone worked

All 22 `PARTIAL` blocks carry dispositions for their outstanding work, and the gate refuses a
`PARTIAL` block that does not, demonstrated by removing one and watching it go red. Four ways,
each with its own message: the section missing, `None.` on a `PARTIAL` block, `Outstanding.` on a
finished one, and a bare `Outstanding.` with no prose saying how it was checked.

The count that makes it worth having done: 56 of the 215 dispositions record a claim the tree
disproved, and every one of the 22 blocks carried at least one. That last part is the
result. Not a long tail of a few rotten blocks: *all of them*, including the three the sweep singled
out as unusually well maintained. Milestone 16, which is why this block exists, was not the outlier
it looked like; it was the one calef happened to ask about.

## What the sweep found, 2026-09-03

Six lanes read all 22 blocks in full, roughly 8,500 lines, and checked every claim against the
tree rather than against the block's own prose, which is the method that found milestone 16's two
finished items. 215 dispositions written. 56 of them record a claim the tree disproved, and every
one of the 22 blocks carried at least one.

| Disposition | Count | What it says |
|---|---|---|
| `Outstanding.` | 82 | Genuinely still this milestone's own scope, re-checked against the tree |
| `Recorded.` | 46 | A limitation that stays one, already beside the feature |
| `Done.` | 37 | Finished, and mostly finished without the block noticing |
| `Milestone N.` | 34 | Belongs to another block, often one this block never cited |
| `Refused.` | 8 | Considered and deliberately not taken |
| `Decision.` | 7 | Already written up under `design/decisions/` |
| `Proposed.` | 1 | Named, unowned, now a file |

The 56 is a lane's judgment, not a column of that table, and the distinction is worth keeping.
Not every `Done.` was a stale claim (a few were closed by this sweep itself) and not every
`Milestone N.` was one (some blocks always meant to hand work on). The 56 are the items where a lane
could name the file, commit or boot transcript that disproved what the block said.

`Done.` is the number to read, plus the 22 `Milestone N.` bullets naming a successor the block
had not noticed. Together they are the 56.

**Why every block, and not a few bad ones.** Three shapes recurred, and none of them is
carelessness:

- A block that accretes lane reports keeps its old questions in the present tense. Milestone
  139 is eight rounds appended in order, and its "what is still open" section opens with a sentence
  its own `BUGS` refutes 140 lines further down. The block is honest at every point in time and
  wrong when read as a whole.
- A block written before a decision landed keeps citing the fork. Milestone 142 (text) waited on a font
  and a palette that §104 chose on 2026-08-20, seven days before that block was last edited, and it
  never cites §104 anywhere.
- **A block's premise can be deleted out from under it.** Milestone 152's "what was built" section
  cites files the SMB removal deleted on 2026-08-30, and milestone 53's ranking argument rests on
  milestone 55, which is `REMOVED`.

**And the wrong sentence is rarely only in the roadmap.** Fixing a block repeatedly left a rustdoc,
a module header or a note still saying the old thing, where no gate reads it. Ten of those are in
`design/roadmap/333-claims-the-sweep-found-false-outside-the-roadmap.md`.

**One block should probably not be `PARTIAL` at all.** Milestone 207's four Community Standards
items are all delivered and GitHub scores the repository at 100%; what keeps it `PARTIAL` is that
nothing verifies the issue forms render. Its index row still says `CODE_OF_CONDUCT` is missing.
That call is an architect's, so the block says so rather than moving.

## BUGS

- It cannot tell a stale claim from a live one, which is 247's limitation inherited whole. The
  value is that the claims become enumerable and that somebody reads them once.
- `IN-PROGRESS` and the rest of the status vocabulary are still outside this. Whether they want
  the same treatment is not answered here, and a status that means "a lane is on it right now" has a
  much shorter staleness window than `PARTIAL`, which can sit for months.
- **A `PARTIAL` block's outstanding work is often the milestone itself**, not a follow-on, so there
  is a real risk of ceremony: writing `**Milestone 74.**` under a heading when the block's own prose
  already said it. If that is what the sweep finds, the honest outcome is to say the section buys
  little for `PARTIAL` blocks and to record that rather than shipping a gate nobody gains from.

## Follow-on

- **Milestone 357.** `IN-PROGRESS`, `NOT-STARTED`, `OPTIONAL` and `RECORDED` are still outside
  this, as the `BUGS` above says, and the sweep found a worked example of a false one: milestone 75
  reads NOT-STARTED on a `DECISION` gate for a decision made 2026-09-02 and built twice that week,
  and three blocks cite it as a live blocker.
  `design/roadmap/357-the-statuses-the-follow-on-gate-does-not-cover.md`.
- **Milestone 333.** The ten stale claims the sweep found in code comments, rustdocs and notes,
  where no gate reads them. Re-checked on 2026-09-19 at promotion: eight still stand, one is closed
  and one wants a re-read.
- **Milestone 358.** Milestone 139's log-page walk, which round 8 asked to have minted and nobody
  did, is `design/roadmap/358-the-unsafe-log-page-walk-in-revoke.md`.
- **Done.** Two of milestone 117's own handoffs had rotted a second time and were fixed here by
  deleting the duplicated fact rather than re-copying it: `CONTRIBUTING.md` no longer lists
  `script/gates`' stages and `script/setup` no longer repeats the toolchain date.
- **Done.** `script/roadmap`'s header described a `--unswept` mode that never shipped, left as half
  a sentence spliced onto the next when milestone 247 removed its exemption list an hour after
  writing it. Corrected here, by the lane that was reading the header in order to add to it.
- **Decision.** Whether milestone 207 (the four things GitHub asks for) should be `BUILT` rather
  than `PARTIAL` is an architect's, since a status word is a claim about the tree and this lane does
  not move another milestone's. The evidence is in 207's own `## Follow-on` and in
  `design/decisions/` nowhere: it is one line in a review, not a decision file.
- **Recorded.** `Outstanding.` is a provisional name, like the seven words it joins. It is recorded
  as provisional in `script/roadmap`'s own comment and in `notes/roadmap.md`, which is where that
  vocabulary moved when the roadmap index was retired on 2026-09-21.

## Index row

All 22 `PARTIAL` blocks swept: 215 dispositions written, **56 of them recording a claim the tree
disproved, and every one of the 22 blocks carried at least one**
