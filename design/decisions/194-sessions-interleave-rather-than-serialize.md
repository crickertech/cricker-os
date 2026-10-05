---
status: DECIDED
raised: 2026-09-19
decided: 2026-09-19
ratified_by: calef
---

# 194. Sessions interleave rather than serialize, and a renumber is the price

calef, 2026-09-19: "Keep interleaving. We are modeling multiple contributors
in the code base and interleaving seems like how we would handle that." Two sessions landing work
into the same global namespaces at the same time is the normal condition, not an incident, and the
ordering is not to be serialized to avoid it.

*(Number provisional until the merge queue lands it, which on this evening's evidence is not a
formality.)*

## What was asked, and what the alternative was

A 333-file branch (milestone 433's drain, pull request #970) took four merge conflicts and four
renumbers of `design/decisions/` in two and a half hours, because a second session was minting
sections from the same range and landing every ten minutes. It minted §156, then §157, then §158,
then §159; each landing displaced the first branch's whole run by one, from §156-§189 to
§160-§194.

The maintainer put two options to calef and recommended the first:

1. Land the big branch next, ahead of the other session's queue. One ten-minute pause, and the
   range stops moving.
2. Keep interleaving, and re-merge each time.

He chose 2, and the reason is the one a recommendation from inside a single branch could not see.
This project's second demonstrator claim is that a system of this size can be built this way at
all (AGENTS.md), by many agents in parallel lanes. A contributor who is told to wait so that
somebody else's branch does not have to rebase is not a contributor; serializing on demand would
make the collisions disappear by removing the condition the method exists to prove it can handle.

## What it costs, stated rather than waved at

The conflicts are cheap and the renumbers are not. The generated indexes
(`design/roadmap/README.md`, `design/decisions/README.md`) conflict on every interleave and are
regenerated, never hand-merged, which is mechanical. A renumber is not, and its hazard is specific:
a citation rewritten by number can be silently wrong and still pass every gate in the tree,
because the section it now names exists. `script/decisions --check` verifies that §174 resolves; it
cannot know the sentence meant the section that used to be §173.

So the price of this ruling is paid in a mechanism rather than in care. A renumber is keyed on the
filename, which is unique across a collision, and the bare `§` sigils that no filename
disambiguates are printed for a person to read rather than rewritten by a regex. That is how the
four renumbers of 2026-09-19 were done, the last two by a script rather than by hand.

## What this does not decide

**It does not raise the lane count.** AGENTS.md bounds lanes by the collision surface, by memory and
by disk, and none of those moved. This is about whether two sessions that are already running are
asked to take turns at the merge queue. They are not.

**It does not make a provisional name less provisional.** The opposite: it is the ruling that makes
that rule load-bearing rather than theoretical. Anything global to the tree, a section number, a
milestone number, a name, is provisional until the queue lands it, and on this evening that clause
fired four times in two and a half hours.

And it does not settle the missing tool. The renumber script exists and lives in a session
scratchpad, which is rung four. Moving it into the tree beside `script/decisions` is proposed as
`a-renumber-that-cannot-cite-the-wrong-section.md`.
