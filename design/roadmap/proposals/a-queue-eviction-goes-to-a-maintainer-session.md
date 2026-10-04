---
status: PROPOSED
raised: 2026-10-03
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# A queue eviction goes to a maintainer session

Raised by the lane that wrote the
[2026-10-03 queue correction](../../../notes/coes/2026-10-03-the-queue-judged-one-pull-request-at-a-time.md),
after calef's review on #1564: *"I don't want the job of watching the queue."* On 2026-10-03 he was
the queue's only working detector. This proposal strips the automation that acted on his behalf
and builds the one piece that would have handed that job to a maintainer session.

## Remove

1. **The drain's arming.** Lanes arm their own pull requests in the same command as `gh pr ready`.
   On 10-03 the calef account made 87 of 91 first arms, against 37 of 112 left to the bot from 09-26
   to 09-28.
2. **The drain's re-arm and re-queue**, including `stranded_numbers` and milestone 630 (a merge-queue
   ejection is caught before the queue, and recovered after it)'s re-arm of a cancelled head. Since
   09-24 it re-armed 30 times after an ejection and 5 of those merged at that head. It re-queued over
   calef-account dequeues 6 to 7 times, and it gave the already-merged #1555 a stale entry. This
   reverses the re-arm that the 2026-09-30 correction asked for.
3. **The drain's stall comments.** 478 since 08-26. In the hour after a bot comment, 55% of pull
   requests saw some action, against 50% in the hour before. calef and the maintainer session say
   they do not read them.

## Keep

- `architect-label.yml`, `coe-architect-label.yml` and `architect hold`. The re-labelling fights
  stopped once `architect-ruled` existed (the last was at 09-27 20:30), and removing the bots would
  hand the labelling back to calef.
- `ready status`, with its `merge_group` leg reading no API. A group is never a draft, and its one
  group failure was #1557's HTTP 503.

## Build: the detector

This is a scheduled job as `nife-smelter[bot]`, on the drain's old cadence. It changes no queue
state. It labels `needs-maintainer` (provisional) on four kinds of pull request: one with a current
non-merged, non-manual queue removal; one that is ready and `DIRTY`; one that is ready, green and
unarmed for 30 minutes; and one whose queue entry outlives its merge. The label comes off when the
head moves or the pull request is back in the queue. A maintainer session reads
`gh pr list --label needs-maintainer` at session start, and while it is running it blocks on the
same list changing. The fix goes to the lane, not to calef.

## The fork, answered

1. **Alternatives.** *Keep the drain and fix its fights*: that keeps a bot that guesses for calef.
   *Strip everything*: that hands detection back to calef, which fails the criterion. *Comments
   instead of a label*: they were measured above.
2. **In the tree.** `queue-ejected` (milestone 630) is already a label the drain sets on the same
   events. This widens it and drops the re-arm.
3. **Prior art, recalled.** A manual merge queue usually has one owner who watches it. Here that
   owner is a session.
4. **Premise.** Today's evictions were acted on 0 to 319 minutes later, all by the calef account.
5. **Cost.** It removes most of `helpers/merge-drain.sh`'s 982 lines. The detector is about 100 lines
   with fixtures.
6. **Reversible.** Yes.
7. **Same cost?** Yes. It is smaller.

## BUGS

- With no session running, a labelled pull request waits for the next one.
- A lane that forgets to arm gets caught after 30 minutes, not after 5.
