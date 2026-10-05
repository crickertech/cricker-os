---
status: DECIDED
raised: 2026-09-04
decided: 2026-09-04
ratified_by: calef
---

# 140. The words a finished milestone may use to say what happened to the work it named

*Amended 2026-09-26: rule text says "an architect" where it said calef, per §217 (every architect
holds the whole role). Records and quotations keep his name.*

Ratified 2026-09-03 by calef, who took the vocabulary as it stands: *"Ratify proposed."* The seven words are the gate's, and `Proposed.` is the one he was asked about. *(Number provisional. Section numbers are the integrator's at merge, per
AGENTS.md, and this one was written by the milestone 247 lane.)*

## What is being decided

Milestone 247 built a gate: every `BUILT` or `REMOVED` roadmap block now carries a `## Follow-on`
section saying what became of the work it named on its way out, and `script/roadmap --check` fails a
block that does not. **The mechanism is not in question here.** What is in question is the seven
words a bullet may open with, because a vocabulary is a thing readers learn and a thing every future
block is written against, and this tree treats status vocabulary as an architect's: calef minted
`REMOVED` himself on 2026-08-30 when the six words then available could only lie about milestone 54.

The words, as shipped:

| Word | Means | Gate requires |
|---|---|---|
| `None.` | Nothing was identified | It stands alone |
| `Milestone N.` | It became milestone N | Block N exists, and is not this one |
| `Done.` | Done, and not as a milestone | Prose saying what carried it |
| `Recorded.` | A limitation, and it stays one | Prose; any path it cites must exist |
| `Refused.` | Considered, deliberately not taken | Prose giving the reason |
| `Decision.` | An architect's call, written up as one | A file under `design/decisions/` |
| `Proposed.` | Named, nobody took it, so it is a proposal now | A file under `design/roadmap/proposals/` |

## What else was considered

Four words, which is what shipped first thing that morning: `None.`, `Milestone N.`, `Recorded.`,
`Refused.`. That set is what the roadmap block's own reasoning implies, and it survived contact for
about four hours.

`Decision.` was added before the sweep, on the first block read. Milestone 55 names a vocabulary
gap that is an architect's call, and forcing it into `Refused.` would have been a lie in the one
place this gate exists to stop lying. It is not a new tracked form: AGENTS.md already says open
decisions live in a file rather than in a conversation.

**`Proposed.` and `Done.` were added by the sweep, not designed.** Twelve lanes read all 139
finished blocks. Three reported, independently and within the same hour, that there is no word for
work a block named honestly that nobody has taken; four reported that there is no word for work a
block named that two ordinary commits then finished. The evidence that this is the right call rather
than a convenience is what those lanes did while the words were missing: they left the items out.
That is the exact silence milestone 247 exists to stop, arriving through milestone 247's own
mechanism, and it is recorded in notes/follow-on-work.md.

`Proposed.` was spelled `Unclaimed.` for about six hours, and took only prose, because a lane
could not mint a milestone number and so had nothing to point at. calef removed that constraint the
same day with `design/roadmap/proposals/<slug>.md`: the collision that barred lanes from minting was
in the number, never in the authority. The disposition now resolves to a file somebody wrote, which
is strictly stronger than a sentence promising work exists, and it is the reason the sweep's third
bucket did not have to be argued down to a small number.

## The recommendation, and it is a ratification rather than a fork

Take the seven as they stand, and the argument is that they were derived from the corpus rather
than guessed at: four were designed, three were forced by 139 blocks of real prose, and the sweep
that forced them is the largest read of this roadmap anyone has done.

What is genuinely open, and is smaller than the vocabulary:

1. The words themselves. `Proposed.` is the one worth arguing about, because it now collides in
   the reader's ear with `**Status: PROPOSED**` on a decision file and on a proposal file. That may
   be a feature (all three mean "written down, not yet taken up") or it may be the kind of overload
   that costs a newcomer an hour. `Open.`, `Orphaned.` and `Unowned.` were available and name the
   absence rather than the artifact.
2. Whether a proposal should age. Nothing distinguishes one written today from one that has sat a
   year except the date on its own status line, and nothing escalates either. That is deliberate
   (a backlog, not a queue, and a gate on age would be routed around by not writing proposals), and
   it is the first thing that will look wrong in six months.

## What is blocked until this is answered

Nothing. The gate is live, all 139 blocks are swept, and the words work. A rename is a mechanical
edit across the sections plus one regex in `script/roadmap`, which is why this is `PROPOSED` rather
than held: the *move fast on what can be undone* tenet puts a vocabulary on the expensive side, but
this one is early enough that nobody outside the tree has learned it yet.

The cost of saying nothing is that the words harden by use. Every block written from here is
written against them, and the reversibility above decays with each one.

## The sweep's five proposed milestones, which are a separate ask

All 42 pieces of unclaimed work are now proposal files under `design/roadmap/proposals/`, so none of
them needs a decision to exist. Five are called out in notes/follow-on-work.md as worth promoting
first, on a stated bar: a claim this project makes rests on it, or a record in the tree is now known
to be wrong. Promotion is prioritising, which is an architect's, and it wants numbers minted at merge rather
than an answer here.
