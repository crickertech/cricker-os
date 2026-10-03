---
status: PROPOSED
raised: 2026-10-03
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# The drain honours a dequeue by hand

Raised by the lane that wrote the
[2026-10-03 queue correction](../../../notes/coes/2026-10-03-the-queue-judged-one-pull-request-at-a-time.md),
failure 1. On that day `nife-smelter[bot]` re-queued or re-armed 7 pull requests a person had just
dequeued at an unchanged head (#1525, #1530 twice, #1535, #1537 twice, #1538). It took between 2 s
and 2 minutes each time. Four of the re-adds each started a group that failed, about 96 group
minutes in all.

## The change

`helpers/queue-ejected.jq` gains one case. Suppose the last queue removal is `manual`, its actor is
not the drain's App, and nothing has been pushed or enqueued since. Or suppose the last
`AutoMergeDisabledEvent` has `reasonCode: manually_disabled` under the same conditions. Then the
action is `dequeued`: the drain does not arm it, and it posts no comment or label, because the
person already knows. The hold ends when the head moves or the person enqueues again, and both of
those are acts the drain can read. `helpers/queue-ejected-selftest.sh` gets three fixtures: the
hold, a push, and a re-enqueue. Rung 2.

## The fork, answered

1. **Alternatives.** *Skip pull requests whose auto-merge is off*: refused, because the premise is
   false. `autoMergeRequest` is null for a pull request in the queue, one just ejected, and one never
   armed. Arming those is the drain's whole job: today it enabled auto-merge 10 times and enqueued
   22 times. *A hold label a person adds*: refused, because it is rung 4. The dequeue is already the
   signal, and a label is one more thing to remember to take off. *Convert to draft*: the lever
   people use today (#1537 at 19:38), which re-runs CI and lies about the pull request's state.
2. **In the tree.** Milestone 630 (a merge-queue ejection is caught before the queue, and recovered
   after it) already holds an ejected head until it moves. This uses the same release, keyed on a
   person's act rather than a failure.
3. **Prior art, recalled and not re-read.** bors-ng's `r-` cancels approval until a new `r+`. Prow's
   tide drops a pull request when its `lgtm` label goes. Both honour the person's last act.
4. **Premise.** The drain reads `autoMergeRequest` only for its `ARMED` log baseline
   (`helpers/merge-drain.sh`). Its eligibility test is in `helpers/queue-eligible.jq`, and that test
   has no idea of intent.
5. **Cost.** About 10 lines of jq and three fixtures. One more field on the query the drain already
   makes, `actor { login }`. The API calls per pass stay the same.
6. **Reversible.** Yes. It is a script that one workflow runs.
7. **Same cost?** Yes. It is the cheaper option and also the right one.

## BUGS

- If a person dequeues and then forgets, the pull request stays out of the queue. That is the sixth
  class in the 2026-09-30 correction, now caused by the person on purpose. The drain's summary line
  should list holds by number so they are visible.
- The maintainer's queue jump (dequeue, then enqueue) releases the hold straight away, which is the
  intent.
