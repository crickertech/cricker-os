---
status: REFUSED
raised: 2026-09-19
promoted_from: nothing-regenerates-the-roadmap-index
refused_by: 443
---
# 510. Regenerating the index is nobody's job

Refused 2026-09-21. *(Number provisional until the merge queue lands it.)* Retired
unbuilt: calef deleted the artifact this milestone existed to keep current, ruling that the index
table was no longer worth building. The staleness this block describes was real, and it is now
unrepresentable rather than fixed, which is the strongest way for a problem to end. REFUSED
rather than REMOVED, because nothing here was ever built to remove. The argument below stands as
written and is kept because it was correct; what changed is the world it described. Promoted from the
proposal `nothing-regenerates-the-roadmap-index`, filed 2026-09-19, on calef's instruction of
2026-09-20 to give every proposal on `main` a number. The text below is the proposal's own, unedited
except for this paragraph: the argument is its author's and promotion is not the moment to improve
it. Filed by milestone 443 (lanes wait on each other for three)'s lane, which took every gate off the generated index and found the last
thing holding it together is a sentence.

## What is left after 443

`design/roadmap/README.md` is generated from the per-milestone blocks (milestone 294 (design/roadmap/README.md's index is generated, not hand-maintained)) and no longer
feeds any gate: `script/fatal-risks` and `script/audits` read `script/roadmap --index` instead, and
`script/lint` fails if a new reader of the committed file appears. What the file still is, and this
is the whole reason it stays committed, is the roadmap a person browses on GitHub and the only
record `script/metrics` and `script/catch-up` can read at a past revision.

So the file is now purely a rendering, and nothing makes it current. `script/roadmap --check`
reports its staleness and deliberately does not fail, because failing would put every lane back in
the tree's worst merge hotspot. The regeneration is the integrator's at merge, written down in
`AGENTS.md` and in `script/roadmap`'s own header, which is rung four of the ladder: a note.

**It is already not happening.** On 2026-09-19 `main` carried three stale rows, and one of them was
not cosmetic. Milestone 177 turned BUILT on 2026-09-19 and its row still read `PARTIAL` with no
Built date, which is the exact state that hid a true `script/fatal-risks` finding about risk 9 until
somebody happened to regenerate. 443 fixed the reading; it did not fix the lag.

## What is proposed, and what it costs

A workflow on `main`, after each merge, that runs `script/roadmap --write` and commits the result
if it changed. Nothing else in the diff, ever: one file, between two markers, from a generator that
refuses to run on blocks that do not validate.

**This is a bot writing to `main`, which this tree has never done**, and that is the decision rather
than the YAML. Three things have to be answered before it exists:

1. Whose identity. Every commit and pull request here is authored under calef's account by the
   `gh` token, which `AGENTS.md` already calls out as the reason every agent artifact opens by
   saying it is an agent's. A commit on `main` that no person wrote is the strongest form of that
   problem, and milestone 128's App is the mechanism that answers it. Until then the bot's commits
   would be indistinguishable from calef's own.
2. **What else such a commit may touch, stated as a rule and not as a habit.** The narrow answer is
   the only safe one: this one file, between the two markers, and a refusal to push anything else.
3. What happens when it collides. A merge lands while the regeneration is in flight, the push is
   rejected, and the honest behaviour is to give up and let the next merge do it, because the state
   is idempotent and one stale rendering is what today already looks like.

The alternative that needs no bot, and why it is not recommended. Delete the file and print the
index on demand. It costs a reader the browsable roadmap on GitHub, which is what the file is for,
and it costs `script/metrics` its milestone-count series and `script/catch-up` its transitions from
the day it lands forward, because both read blobs at revisions where no block-derived answer exists
and none can be manufactured. That is a permanent loss of a record to avoid a recurring chore.

And the cheap thing that is not this. A CI check on a pull request that *fails* when the index is
stale would work and is the wrong shape: it makes the lane regenerate, which makes the lane edit the
one file milestone 294 removed it from, which is the hotspot coming straight back. That trade was
already made and this proposal does not reopen it.

## BUGS

- Nothing here bounds how stale is too stale. Three rows was tolerable and invisible; the failure
  mode is that it is still tolerable at thirty and still invisible. A staleness number is printed on
  every `script/roadmap` run and read by whoever happens to be looking, which is rung zero.

## Revisit

- **Condition.** Nothing, and that is the point rather than an omission. This milestone's work was
  keeping a generated table current; the table is gone, so the work cannot recur. The bell would be
  the reintroduction of any generated artifact that a human is expected to regenerate by hand, at
  which point this block's argument applies again to that artifact rather than to this one.

## Index row

`design/roadmap/README.md` is generated from the per-milestone blocks (milestone 294) and no longer
feeds any gate: `script/fatal-risks` and `script/audits` read `script/roadmap --index` instead, and
`script/lint` fails if a new reader...
