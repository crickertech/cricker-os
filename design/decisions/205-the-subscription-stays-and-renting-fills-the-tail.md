---
status: DECIDED
raised: 2026-09-22
decided: 2026-09-22
ratified_by: calef
---

# 205. The subscription stays, and rented models fill the mechanical tail

calef, 2026-09-22. *(Number provisional until the merge queue lands it.)*
He had proposed downgrading Max 20x to Max 5x, freeing $100 a month for open-weight models, and
cancelled that after the numbers came in. Max 20x renews.

## What was actually proposed, and why it failed

The plan was symmetric and wrong in one term: drop 75% of Claude capacity, replace it with
rented open-weight capacity at a fraction of the price. §202 (mechanical work goes to a cheaper
model) had ruled the routing and §203 (capacity is rented rather than bought) had ruled the
sourcing, so this looked like execution rather than a new decision.

calef found the flaw before the maintainer did, in one sentence: *"30% offload doesn't seem like
enough if we're bumping up against 20X and downgrading to 5X. It seems like we need to offload
75%."*

## The measurements that settled it, all taken 2026-09-22

The infrastructure works and is not the constraint. An Anthropic-format gateway runs on cordoba
(notes/open-model-lanes.md), and three real lanes completed on open-weight models with every gate
green. What decided the question was price per *kind* of work:

| lane | model | cost |
|---|---|---|
| renumber two milestone files, pure mechanics | Qwen3-Coder | **$0.048** |
| the same, for comparison | Kimi K3, uncached | $0.393 |
| the same, preference-pinned so the cache warms | Kimi K3 | $0.258 |
| **milestone 570 (the install offer should say what is already on the disk), a real implementation** | Qwen3-Coder | **$0.98** |

The twentyfold gap between the first and last rows is the finding. A mechanical lane is five
cents; a real implementation lane is about a dollar. Twenty implementation lanes a day is $600 a
month, three times the subscription it would replace, and Max 20x supports roughly 100 to 140
lanes a month, which is $1.50 to $2 a lane. So renting is *modestly* cheaper than the
subscription for implementation and dramatically cheaper only for the mechanical tail.

Against that, the offloadable share measured about 30% of lanes, and those are also the *cheap*
lanes in tokens: the promotion repair took 125k and the falsification refresh 169k, against 549k for
the installer and 344k for rung 2b. So the tail is roughly 18% of a day's tokens.

18% offloaded cannot fund a 75% cut. That is the whole arithmetic.

## What is decided

1. Max 20x renews. The binding constraint is a rate limit rather than a bill (§203), and
   downgrading tightens exactly the thing that hurts.
2. The mechanical tail moves off Claude anyway, because it is nearly free: ~180 lanes a month at
   $0.048 is about $9, a 4.5% spend increase that returns roughly 18% of Claude capacity,
   or three to four more judgement lanes a day.
3. Maintainer work is the next candidate and is unmeasured. Queue nannying, CI-log triage and
   rebase conflict resolution are mechanical, are done in the most expensive context available, and
   plausibly exceed the 18% above. The first delegated rebase ran on 2026-09-22.

## BUGS

- The redo rate is unmeasured, and it is the number most likely to invert this. A lane that
  passes every gate and must be redone on Claude costs *more* than never having offloaded. Three
  open-model lanes have passed their gates; two carried editorial defects a gate cannot see (one
  dropped a load-bearing sentence, one conflated "unpartitioned" with "unreadable" in a prompt whose
  purpose is informed consent). Neither required a redo, but neither was clean.
- **The maintainer's own consumption is still not instrumented.** Every share above is a count of
  lanes or of subagent-reported tokens; the session's own use is invisible, which is what milestone
  553 (what a lane spent on its milestone) exists to fix and has not.
- All prices are one day's observation on one provider, with one model per row and no repeats.
  A second run of the same lane could move any of them.

## The one-time reset was spent on 2026-09-23, and the insurance is gone

calef pulled the manual limit reset that night, restoring capacity until Friday's weekly roll. He
had one available before 2026-10-22 and now has none.

He spent it on the criterion this section's own conversation set, which is worth recording
because it is the difference between a decision and a reaction: the maintainer recommended holding
it until the routing work landed, and named the single case for spending early as *hitting the
weekly wall mid-build, where the alternative is idling until the window rolls*. That was the case.
The projection was running out by the following morning.

**What it lifted, and what it did not**, because these are easy to conflate and only the first is
about money:

- Tokens: lifted, to Friday.
- Memory: unchanged. A Kani harness still reaches about 3.5 GB against this machine's 16 GB, so
  one solver at a time holds whatever the budget says.
- CI runners: unchanged, and binding that night. Two pull requests carrying 44 and 21 new
  falsification records sat behind 35 queued runs org-wide. More tokens buys more lanes that queue
  behind the same runners.

So the reset bought authoring capacity, not verification capacity, and the night's work was
stacked on the second.

**The routing work is now more urgent, not less.** This section priced an open-model tail and the
maintainer proposed batch variants at roughly half price and an `effort` lever. Neither landed.
`notes/effort-levels.md` measured `effort` as a null result on the task it tested: prompt wording
explained all of the correctness and the level explained none. Batch pricing was raised three times
and never applied to a single lane.

There is no second reset behind the next wall. That is the whole of what changed.
