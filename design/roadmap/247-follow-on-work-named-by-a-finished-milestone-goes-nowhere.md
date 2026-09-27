---
status: BUILT
raised: 2026-09-03
built: 2026-09-03
---
# 247. Follow-on work named by a finished milestone goes nowhere, and this is the third time

Built on 2026-09-03. Minted the same day by calef, after milestone 244 (largest) named work that
would have been buried had he not asked for it by name. Landed as pull request #692: the
`## Follow-on` section, its dispositions, the sweep of all 139 finished blocks, and
`design/roadmap/proposals/`.

*(Status corrected by milestone 252's lane. It merged still reading IN-PROGRESS on its own branch,
which `script/roadmap --check` refuses, so `main` was red for every lane until somebody moved it.
That is the milestone-69 defect this very family of gates exists to catch, happening to the block
that added the newest member of the family, one merge after it landed.)*

In brief. calef, 2026-09-03: *"we keep burying work identified by completed milestones."*

There are 151 blocks marked BUILT. Each was allowed to finish, and a finished milestone's block
is the last place anyone looks. The work those blocks named on their way out is scattered through
`BUGS` sections, "handoff" paragraphs and lane reports, and nothing in this tree can tell which of
it became a milestone and which of it evaporated.

## This is the same failure three times, which is what makes it a milestone and not a chore

AGENTS.md already carries the rule, in as many words:

> Identified work leaves the lane in a tracked form, or the merge waits.

And it already carries the reason the rule does not hold, which is that the rule is rung four:

> A lane report is read once, by one person, on the day it is written. A pull request body is read
> while the diff is open and never again. Both feel like records while you are writing them, which is
> what makes this the failure that recurs.

The recurrences are on the record and they are not near-misses:

- Milestone 90 (guard) exists only because calef happened to be at his desk the day a report named it.
- Milestone 94 (untracked) swept the tree for exactly this category and then left its own inventory in a
  pull request body for twelve days, by which point the item-level list was gone and had to be
  re-derived (notes/untracked-work-sweep.md).
- Milestone 244, on the day this was minted, named an unvouched-binary hazard in a `BUGS` section
  and a design fork in a handoff paragraph. Both were surfaced because calef asked *"any work to
  follow up on 244?"* by hand. That question is the mechanism today, and it is rung zero: somebody
  has to remember to notice.

## The two halves, and the second is the one that lasts

A sweep, which is the part that is owed now: read the 151 BUILT blocks and their `BUGS`
sections, and for every piece of named follow-on work decide one of three things. It became a
milestone (say which). It is a recorded limitation and stays one (say where). Or it is neither, and
then it gets minted or explicitly refused. An explicit refusal is a success; the defect is
silence, not the absence of a milestone.

And a mechanism, because a sweep that is not made repeatable is milestone 94 again with a new
number. The shape is not specified here, because choosing it *is* the milestone. What the shape has
to survive is stated instead:

- It cannot be a lint that greps prose for intentions. AGENTS.md priced that already: `git grep -w
  TODO` runs an 82% false-positive rate, and a check that cannot tell an observation from an intention
  will be disabled within a week.
- It must not weaken the `BUGS` convention. A recorded limitation is a fact a reader meets beside
  the feature, and it is the FreeBSD posture working as designed. The goal is to route intentions
  *into* the tracked forms, never to drive limitations out of `BUGS`.
- The likely rung is two, and the tree already has the pattern: a marker with a resolvable
  referent. `script/lint` already fails a `TODO` that does not name a milestone, and the citations
  gate already checks that every glossed citation is grounded in the document it names. A block that
  proposes follow-on work in a form a script can find, resolving to a minted block or to a stated
  refusal, is the same shape a third time.

## The proof that this milestone worked

Two things, and the sweep alone is not enough.

1. A written disposition for every piece of follow-on work named by a BUILT block: minted, recorded,
   or refused with a reason.
2. Something that fails when a new one is buried, demonstrated by burying one and watching it go
   red. Without this the sweep has a shelf life and milestone 94 already measured it at twelve days.

## BUGS

- 151 blocks is a lot of prose and the sweep will be imperfect. A sweep that finds most of it and
  ships the mechanism is worth more than an exhaustive one that does not, and a partial sweep should
  say what it did not read rather than implying it read everything.
- This cannot recover what was never written down. Work named only in a chat window or a lane
  report that nobody landed is not in the tree to be found, and the count of what was lost is
  unknowable rather than zero.
- A mechanism that is too strict will be routed around. If every observation in a `BUGS` section
  has to resolve to a milestone, the honest thing to write becomes expensive and people will write
  less, which costs more than the burial does. Whatever ships has to leave "this is a limitation and
  it stays one" cheap to say.

## What was built, 2026-09-03

The mechanism, in two halves. A `## Follow-on` section on every BUILT or REMOVED block, gated by
`script/roadmap --check` and therefore by `script/lint`. Seven dispositions, each resolving to
something a script can check: `None.`, `Milestone N.` (the block must exist), `Done.` (what carried
it), `Recorded.` (any path it cites must exist), `Refused.` (a reason), `Decision.` (a file under
`design/decisions/`), `Proposed.` (a file under `design/roadmap/proposals/`). Tabulated in
design/roadmap/README.md, argued in notes/follow-on-work.md, and put to calef for ratification in
`design/decisions/140-follow-on-disposition-vocabulary.md`.

And the half that makes it cheap enough to use, which is an architect's rather than this lane's
(2026-09-03): `design/roadmap/proposals/<slug>.md`, an unnumbered proposal any lane may write. The
reason lanes could not add to the roadmap was stated as collision, and *"the collision is in the
number, not in the authority"*. Separating them means work reaches the tree at the moment it is
discovered, written by whoever understood it best, with no maintainer in the path. That matters here
more than anywhere: on the day this landed the maintainer buried three identified items by deferring
them into chat messages, and calef caught all three by asking. A burial mechanism has to assume
the maintainer is a failure point and not only the lanes.

It hangs on the status rather than on a marker in prose, which is what keeps it off the rung
this block ruled out. A block turning BUILT is the moment the burial happens and it is a state a
script can see; nothing here greps prose for intent, so `git grep -w TODO`'s 82% false-positive rate
does not apply. Third instance of a shape the tree already has, after the TODO gate and the
citations gate.

**The proof, which is the half this block said the sweep alone could not give.** Four burials were
staged and each went red: a finished block with the section removed, a bullet naming a milestone number
nothing has, beside a `Recorded.` pointing at a file that does not exist and a bare `Refused. Later.`, a block
parked on the (then existing) exemption list after its `Built` date, and a block left on that list
after it grew a section. The gate also found rot nobody staged: three roadmap blocks citing paths
that renames had moved out from under them.

**The sweep.** All 139 finished blocks read in full, in twelve parallel lanes, about 159,000 words.
605 dispositions: 291 `Recorded.`, which is the `BUGS` convention working as designed, and 136
`Milestone N.`, usually the successor the block had already cited by number.

**And 42 pieces of work that were named and never taken, across 32 finished milestones.** That is
the number this block existed to produce and that nothing in the tree could see before. **38 of them
are now proposal files, and 3 turned out to be already built**, which is only possible because of the half above: the morning's
design had them as prose, and would have forced them to be argued down to a handful that a
maintainer could carry. Five are called out in notes/follow-on-work.md as worth promoting first, on
a stated bar: a claim this project makes rests on it, or a record is now known to be wrong.

**Writing a proposal is what caught the three that were already done**, because it made a lane go and
look at the tree, which nothing had required before. Milestone 54's request for a status word meaning
"built, then removed" was answered when calef minted `REMOVED` on 2026-08-30; milestone 57's
partitioning and `mkfs` both shipped in commit `4db2fd74`. Two of the three had sat stale for a month
inside a `## Follow-on` section, which is this milestone's own failure one layer in: the section says
what happened to the work, and nothing checks that it is still true. That is the honest limit of what
this gate buys and it is recorded below.

**Two dispositions were added by the sweep rather than designed before it**, and that is the
finding worth keeping. `Proposed.` (first spelled `Unclaimed.`) and `Done.` were each asked for by
several lanes independently,
on the same afternoon, having hit the same wall: work a block named honestly that nobody took, and
work a block named that two ordinary commits then finished. Neither fits the four words the gate
shipped with. The lanes that would not write a comfortable lie **left those items out**, which is
this milestone's own failure mode arriving through this milestone's own mechanism.

## Follow-on

- **Recorded.** In `notes/follow-on-work.md`: nothing promotes a proposal and nothing can. A gate on
  age would be routed around by not writing proposals, which costs more than the pile does, so what
  ships is visibility: `script/roadmap --check` prints the count and the oldest date on every lint
  run. That is a number somebody has to choose to ignore, which is weaker than a gate and much
  stronger than a paragraph in a finished block.
- **Recorded.** In `notes/follow-on-work.md`: `Proposed.` and `Recorded.` are separated by judgment
  rather than by anything checkable, so the backlog is under-counted rather than over-counted.
  `Recorded.` costs a sentence where `Proposed.` costs a file, and the same text can defensibly take
  either word.
- **Recorded.** In `notes/follow-on-work.md`: the gate fires once, when a block finishes, and it
  never re-reads a disposition. Three of the sweep's own items were already built when it wrote
  them, two of them stale for a month, and only a lane going to look found that out. A `Proposed.`
  or `Recorded.` bullet ages exactly as badly as the prose it replaced; what changed is that it now
  has a shape somebody could sweep.
- **Decision.** The seven disposition words are a lane's and an architect names things. calef minted
  `REMOVED` in the status vocabulary himself, so this one is the same shape one level down; the
  ratification ask is `design/decisions/140-follow-on-disposition-vocabulary.md`, which also points
  at the sweep's five proposed milestones.

## Index row

151 BUILT blocks name work in `BUGS` sections and handoffs, and nothing can tell which of it
became a milestone
