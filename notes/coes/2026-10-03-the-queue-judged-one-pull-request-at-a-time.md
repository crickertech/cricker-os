# A correction of error: the queue's automation fought itself, and calef watched the queue

*Recorded 2026-10-03 (UTC) by a design lane, commissioned by calef the same day ("Write up the COE
proposal") and revised after his review on #1564. Title provisional. Every figure was measured from
GitHub or on patagonia by this lane; the commands are at the end. Anything recalled is marked.*

## The success criterion

calef, 2026-10-03: *"I don't want the job of watching the queue."* That day he did it. He spotted
each conflicting or failing pull request, removed it from the queue, and told the maintainer
session. Every decision below is judged against one test: once it lands, does any of that work
come back to him? Removing automation is right only when nothing it did lands on calef.

His framing comes first too. The queue has a mix of automation and manual steps, the automation
fights itself and those steps, and perhaps it should be stripped back and rebuilt on purpose. So
the first decision is keep or remove, piece by piece. Fixes come after, and only for what is kept.

This is a new record, not an extension. The
[2026-09-30 correction](2026-09-30-lane-follow-through.md) is about follow-through living in memory,
and the [merge-rate correction](2026-10-03-the-merge-rate.md) blames ejections on a CI job that had
doubled. That job is fixed, and 47 group builds still failed.

## What the record can say about who acted

Every manual step in this data is credited to the `calef` account. That means calef, or a session
using his token, and GitHub records no difference. calef believes he is the only human here. From
his account of the day: calef acted in the UI on #1530 once, #1534, #1547 and #1557. The maintainer
session called `dequeuePullRequest` with his token on #1530 twice and on #1555. This record does not
guess about any other step. So the data cannot say whether a "manual" step was human, and milestone
642 (the record should say whether a person or the machinery took a step) is the action item that
removes the limit. It is NOT-STARTED.

The drain is separable. Since milestone 128 (the automation gets its own identity, and the agents
get their own voice) it acts as `nife-smelter[bot]`, from 2026-09-24 01:21. Before that it used
calef's token, so its earlier steps are mixed into his.

## What happened

1. **The drain re-queued pull requests the calef account had just taken out.** That was 6 times
   with no commit in between (#1525, #1530 twice, #1535, #1537, #1538), and 7 counting a second
   #1537 re-add with a commit of uncertain push time. Each came 2 to 123 seconds after the dequeue.
   All of them were on 2026-10-03, out of 60 calef-account dequeues since 09-24. #1537 was converted
   to a draft at 19:38, the only way left to stop the drain.
2. **The pre-push hook outlasted the agents' tool calls.** A warm `script/lint` took 70.3, 110.7 and
   119.4 s, and a cold one took 201.9 s. The agent harness's Bash tool gives up at 120 s by default
   (read from its tool description). On #1556 the claim push sat in the hook for the whole lane,
   #1560's first push timed out, and #1559's hung.
3. **Pull requests that passed alone failed together.** The queue ran 142 CI group builds and 47
   failed. 33 failed on lint checks that judge the merged tree, which every branch had passed on its
   own. 13 merges of `main` into pull request branches conflicted, across 9 pull requests.

## Each eviction today: who noticed, how long it sat, and who would after

Here "acted" means the first commit or calef-account step after the eviction.

| kind | count | acted after | today | once the action items land |
|---|---|---|---|---|
| conflict ejected (`merge_conflict`) | 2 (#1494, #1517) | 35, 22 min | the calef account | the detector labels it; a maintainer session hands it to the lane |
| conflict found before the queue (`DIRTY`) | 6 PRs | dequeued in 0 to 11 min | calef or his token | the same detector |
| failed group check | 14 | median about 25 min; #1520 and #1521 sat 5 h | the calef account, and the drain re-armed #1542 | the detector; 31 of the day's 47 failed groups would not happen (action item 3) |
| external HTTP 503 in `ready status` | 1 (#1557) | 3 min; re-queued 47 s later | calef, in the UI | the detector; the group leg stops calling the API |
| stale entry for a merged PR | 1 (#1555) | 15 min | a session with calef's token | none needed, because the drain's racing enqueue goes |
| head moved while queued, or already merged | 5 | none needed | nobody | nobody |

The gap is the detector. Today nothing but calef noticed an eviction in time. The drain's stall
comments came out, and nobody acted on them (below). The one piece of automation worth rebuilding
is detection that hands the pull request to a maintainer session and does not re-queue it. With no
session running, the hand-off waits until one starts. That is slower, and it is not calef's job.

## Keep or remove, piece by piece

| piece | evidence | verdict |
|---|---|---|
| drain: first arming | Of first arms since 09-24, the bot made 51 of 372. It made 4 of 127 on 09-24 and 09-25, 37 of 112 from 09-26 to 09-28 12:40, 6 of 42 on z.ai to 10-02, and 4 of 91 on 10-03. 47 of the 51 came more than N = 5 min after ready | remove; the lane arms in the same command as `gh pr ready` |
| drain: re-arm and re-queue | 30 re-arms after an ejection since 09-24, of which 5 merged at that head. 6 to 7 re-adds over calef-account dequeues. #1555's stale entry | remove |
| drain: stall comments | 478 since 08-26. Hour after a bot comment: 55% saw an action; hour before: 50%. The 308 posted with calef's token before 09-24 notified nobody, since GitHub does not notify an account of its own comments (recalled) | remove |
| architect-label bots and `architect hold` | 48 labels. 11 re-labels after a calef-account removal, the last at 09-27 20:30, and none since `architect-ruled` | keep |
| `ready status` check | 5 failures in 259 pull request runs. 1 failure in 131 group runs, which was #1557's 503 | keep, with no API call in the group leg |
| pre-push hook | PR lint failure rate by the hook in the pushed tree: 5.7% (29 of 511) and then 3.6% (9 of 250) with fmt only, 1.8% (2 of 111) with all of lint. On 10-03 it caught 5 findings and wrongly refused 6 pushes, 5 of them claims | narrow it to fmt plus lint's text checks, and skip claims |

**Why N is 5 minutes.** Of 321 calef-account first arms since 09-24, 224 came within 1 minute of
ready, which is the same command as `gh pr ready`. Only 11 more came by 5 minutes. Five minutes is
also the drain's cron interval.

**Newer models.** 10-03's lanes left the drain 4 of 91 arms, against 37 of 112 from 09-26 to 09-28.
That fits calef's guess, but 09-26 is also when lanes stopped watching CI and began ending with
`WAITING` (this is in CLAUDE.md, and the date is recalled from the maintainer's memory index). So
model and brief cannot be told apart, and before 09-24 they cannot be told from the drain.

## Impact

| kind | failed groups | group minutes | ejections | calef-account interventions |
|---|---|---|---|---|
| drain re-adds over a dequeue | 4, also counted below | about 96 | | 7 re-dequeues, 1 draft |
| hook | 0 | 0 | 0 | 3 stuck pushes, 5 refused claims |
| union facts: promotion, prose budget, counts | 33 | 652 | 7 | 8 dequeues, 4 fix commits, 3 count merges |
| appends at one spot | 0 | 0 | 0 | 3 conflicted merges, 2 dequeues |
| ratchet churn (bold, glosses) | 0 | 0 | 0 | rode along with other conflicts |
| real overlap | 0 | 0 | 2 | 6 conflicted merges, 4 dequeues of #1530 |

A group minute is the wall time of one failed CI group run, and the runs overlap. Over the day
there were 122 queue entries, and 39 left without a merge (32%). In the 96 minutes from 19:18, one
pull request merged.

## Five whys

1. Why did calef end up watching the queue? Its automation acted on one pull request at a time and
   reported where nobody looked. The costs were between pull requests, and between the bot and the
   calef account.

Branch 1, the drain:

2. Why did it put #1530 back? It arms every eligible pull request on every pass. Eligible means
   open, not a draft, on `main`, and with no hold label.
3. Why didn't the dequeue count? `helpers/queue-ejected.jq` skips `manual` removals, so the report
   was skipped. Arming went ahead anyway.
4. Why not check `autoMergeRequest`? It is null for a queued pull request, an ejected one, and one
   never armed, so it cannot tell a deliberate dequeue apart.
5. Why does it arm at all? On 2026-08-04 lanes left green pull requests unarmed. On 10-03 they armed
   87 of 91 themselves.

Branch 2, the hook:

2. Why did pushes hang? Full lint takes 70 to 120 s warm and 202 s cold.
3. Why? It is 84 checks, and the largest takes 3.7 s.
4. Why all of them? Milestone 630 (a merge-queue ejection is caught before the queue, and recovered
   after it) widened the hook that morning.
5. Why keep any of it? It does catch real findings, about 4 per 100 pushes against the fmt-only
   weeks, and 36 of the 38 fmt-only failures came from checks that need no compiler. It
   cannot catch union failures, and it refuses the claim push.

Branch 3, the union:

2. Why did groups fail when every branch passed? Lint checks the merged tree for three things:
   promotion deletes its proposal, the prose ratchet's word baseline, and the Kani harness count.
3. Why did promotion break 31 groups? Milestones 720 to 724 were promoted from proposals that
   existed only on #1513's unmerged branch.
4. Why was that possible? `script/roadmap` checks that the proposal is gone, not that the branch
   ever had it.
5. Why are counts typed by hand? Milestone 125 (a number in the prose is a claim, and nothing
   re-derives it) gates them and nothing generates them. That has caused 31 fix commits since 09-01.

## Root cause

Queue automation was built to act, not to hand off. It re-armed, re-queued and commented, and each
was a guess on calef's behalf about one pull request. None of them told a maintainer session that
something needed its attention, so calef became the detector. Two smaller causes widened the gap.
The hook checks the branch where the queue checks the union, and branches commit facts that only
the union can make true.

## Action items

Highest rung first.

- **Proposed.** `design/roadmap/proposals/a-queue-eviction-goes-to-a-maintainer-session.md` (rung
  2). Remove the drain's arming, re-queueing and stall comments. Keep the architect-label bots and
  `ready status`, without an API call in its group leg. Build one detector that labels an evicted,
  `DIRTY` or ready-but-unarmed pull request for a maintainer session.
- **Proposed.** `design/roadmap/proposals/the-pre-push-hook-runs-what-fits-in-seconds.md` (rung 2,
  narrowed). The hook runs `script/fmt --check`, lint's text checks and the ready-branch question,
  in about 33 s (estimated), and skips a claim. Clippy stays in CI.
- **Proposed.** `design/roadmap/proposals/a-branch-commits-no-fact-about-the-merged-tree.md` (rung
  2, plus a rung-1 deletion). It covers 31 of the 47 failed groups.
- **Milestone 642.** Milestone 642 (the record should say whether a person or the machinery took
  a step). The data cannot say whether a calef-account step was calef or a session, so it
  cannot say whether "manual" means human.

`design/roadmap/proposals/the-drain-honours-a-dequeue-by-hand.md` was deleted in this revision.
Its premise was a drain that re-arms, and the first action item removes that.

## How the numbers were made

```
# every pull request since 2026-08-10 with queue, auto-merge, ready, label, comment and head events
gh api graphql -f query='...pullRequests(first:20,orderBy:{field:CREATED_AT,direction:DESC})
  {nodes{number timelineItems(first:250,itemTypes:[ADDED_TO_MERGE_QUEUE_EVENT,
  REMOVED_FROM_MERGE_QUEUE_EVENT,AUTO_MERGE_ENABLED_EVENT,READY_FOR_REVIEW_EVENT,ISSUE_COMMENT,
  LABELED_EVENT,PULL_REQUEST_COMMIT,HEAD_REF_FORCE_PUSHED_EVENT]){...actor{login}}}}'
gh api --paginate "repos/nifeos/nife/actions/runs?event=merge_group&created=2026-10-03"
gh run view <id> --log-failed | grep -B4 'lint ends'
git show --remerge-diff <merge> | grep '^remerge CONFLICT'
```

A stall comment is one carrying a `<!-- merge-drain:... -->` marker. An action is any commit, label,
ready, draft, arm, queue, close or merge step that is not the bot's or the queue's.

## BUGS

- Commits carry commit dates, not push dates, so "no commit in between" is approximate.
- An action after a stall comment is not proof the comment caused it. The before-and-after rates
  are the only test this data supports.
- A rebased branch hides its conflicts, so 13 is a floor.
- The hook's timings come from a loaded laptop, which is where lanes push from.
