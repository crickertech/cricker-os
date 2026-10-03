# A correction of error: the queue's helpers judged one pull request at a time

*Recorded 2026-10-03 (UTC) by a design lane, commissioned by calef the same day ("Write up the COE
proposal"). Title provisional. Every figure was measured from GitHub or on patagonia by this lane;
the commands are at the end. Anything recalled rather than read is marked.*

This is a new record, not an extension. The [2026-09-30 correction](2026-09-30-lane-follow-through.md)
is about follow-through living in memory, and #1461 extended it with the ejection that cancels
auto-merge. Failure 1 below is a defect in the re-arm that extension asked for, but failures 2 and 3
have nothing to do with memory. The [merge-rate correction](2026-10-03-the-merge-rate.md) blames its
ejections on a CI job that had doubled. That job is fixed, and the queue still threw away 47 group
builds today. The cause is different, so the record is separate.

## What happened

Three failures on 2026-10-03, each in a mechanism that reads one pull request alone.

1. **The drain re-queued pull requests a person had just taken out.** `nife-smelter[bot]` put a pull
   request back in the queue, or re-armed it, 7 times on 5 pull requests (#1525, #1530 twice, #1535,
   #1537 twice, #1538). Each time, the person had dequeued it by hand and had not pushed since. The
   drain was back between 2 seconds and 2 minutes later. On #1530 the maintainer dequeued it four
   times between 20:47 and 21:05. On #1537 they gave up and converted it to a draft at 19:38.
2. **The pre-push hook outlasted the agents' tool calls.** On a warm target directory, the hook's
   `script/lint` took 70.3, 110.7 and 119.4 s on patagonia, with load averages from 11.6 to 19.3.
   On a cold one it took 201.9 s. The agent harness's Bash tool gives up after 120 s by default (read
   from this session's tool description). On #1556 the claim push sat in the hook for the whole lane,
   #1560's first push timed out, #1559's hung, and several lanes pushed with `--no-verify` after a
   clean lint on the same tree.
3. **Pull requests that passed on their own failed together.** The queue ran 142 CI group builds
   today and 47 failed. 39 of those failures were in lint, and every one of the 39 was a fact about
   the merged tree that no single branch got wrong. Separately, 13 merges of `main` into pull
   request branches hit textual conflicts, across 9 pull requests.

Two things happened that this record does not blame on any of the three. #1557 was ejected because
its `ready status` group job got an HTTP 503 from GitHub at 21:53:20; it was re-queued at 21:57 and
merged. And #1555 was given a queue entry by the drain at 22:23:57, the same second it merged, and
that entry sat there until a person removed it at 22:38:59.

## Timeline

| when (UTC) | what | source |
|---|---|---|
| 01:11 to 01:28 | five groups fail on an IN-PROGRESS block for milestone 624 (the x86_64 paint path stops repainting the world), a class milestone 630 now catches | CI |
| 10:30 to 10:33 | #1520 and #1521 fail as a group on the prose ratchet. #1520's own CI was green at the same head | CI |
| 17:10:13 | #1525 dequeued by hand; the drain re-arms it at 17:10:49 | timeline |
| from 18:35 | milestones 721 to 724 armed. Each block, like 720's, says `promoted_from` a proposal that exists only on #1513's unmerged branch | git, timeline |
| 18:53:49 | the first group failure: #1530's proposal "is still here" | CI |
| 19:14:26 | #1513 merges, and its four proposals land on `main` beside the four blocks that promoted them | timeline |
| 19:18:14 | #1526 merges. Nothing else merges until #1532 at 20:17:14, and after that nothing until 20:54:38 | timeline |
| 19:28 to 20:01 | the maintainer dequeues #1535, #1537 and #1538 eight times; the drain re-adds four of them, and each re-add starts a group that fails | timeline, CI |
| 19:46 | four commits titled "remove the promoted proposal now that #1513 has merged" | git |
| 20:17:19 | the last of 31 group failures on the promotion check | CI |
| 20:47 to 21:05 | #1530 dequeued four times, re-added twice by the drain, while its `ci.yml` conflicts with #1543's rename | timeline, git |
| 21:24 to 21:26 | #1534 resolves the Kani harness count by hand: "213 + 1 (main) + 2 (this lane) = 216" | git |
| 22:18 to 22:37 | #1547 dequeued twice and merges `main` three times, conflicting each time | timeline, git |
| 22:38 | #1548 and #1552 conflict on `main` | git |

## Impact

| kind | failed groups | group minutes | ejections | hand interventions |
|---|---|---|---|---|
| failure 1: drain re-adds after a dequeue | 4 | about 96 | counted under failure 3 | 7 re-dequeues, 1 draft |
| failure 2: the hook | 0 | 0 | 0 | at least 3 stuck pushes, `--no-verify` habit |
| failure 3, union facts: promotion, prose budget, counts | 33 | 652 | 7 | 8 dequeues, 4 fix commits, 3 count merges |
| failure 3, appends at one spot | 0 | 0 | 0 | 3 conflicted merges, 2 dequeues |
| failure 3, ratchet churn (bold, glosses) | 0 | 0 | 0 | 2 hunks inside other kinds' merges |
| failure 3, real overlap | 0 | 0 | 2 `merge_conflict`, one traced | 6 conflicted merges, 4 dequeues of #1530 |

A group minute is the wall time of one failed CI group run. The runs overlap, so this is not runner
time. Failure 1's four groups are also among failure 3's 33. Over the whole day: 122 queue entries, 39 of them leaving without a merge (32%, over calef's
20% line), and 96 minutes from 19:18 with a single merge in them.

## Five whys

1. Why did the queue lose most of an evening? Three mechanisms each did what they were built to do
   for one pull request, and the cost was between pull requests.

Branch 1, the drain:

2. Why did the drain put #1530 back? It arms every eligible pull request on every pass
   (`helpers/merge-drain.sh`, `pass()`). Eligible means open, not a draft, based on `main`, from
   this repository, and without a hold label.
3. Why didn't the dequeue count? `helpers/queue-ejected.jq` skips `manual` removals on purpose,
   because reporting a deliberate act back to the person who did it is noise. The report was
   skipped. The arming went ahead anyway.
4. Why not check `autoMergeRequest`? That is not a premise that holds. The field is null for a pull
   request sitting in the queue, for one that was just ejected, and for one that was never armed.
   Arming those is the drain's job. Today the drain enabled auto-merge 10 times and added to the
   queue 22 times.
5. Why could a person not hold it? The only levers are `needs-architect` and `held-for-red-trunk`,
   and both mean something else, or a draft, which re-runs CI. The drain runs every five minutes
   and after every CI run finishes, so a person will not get there first.

Branch 2, the hook:

2. Why did pushes hang? The hook runs all of `script/lint`, at 70 to 120 s warm and 202 s cold.
3. Why so slow? No one check is: of 84, the largest is spelling at 3.7 s. It is the count. 33 cargo
   invocations take 37.0 s, and 51 other checks take 33.3 s.
4. Why run all of it? Milestone 630 (a merge-queue ejection is caught before the queue, and
   recovered after it) widened the hook on the same day, after measuring 56.5 s with
   two lanes beside it. The tree had since grown, and the machine had more lanes on it.
5. Why didn't it pay for itself? It lints the branch alone. All 39 lint failures in the queue today
   were in the union, so the hook could not have caught any of them. On pull requests, lint failed
   in 2 of 312 CI runs. The clippy job takes 2 to 5.5 minutes there, and the drain will not arm a
   red pull request.

Branch 3, the union:

2. Why did groups fail when every branch passed? Three lint checks judge the merged tree: the rule
   that promoting a proposal deletes it, the prose ratchet's per-file word baseline, and the
   counted-claims marker on the Kani harness count.
3. Why did promotion break 31 groups? Five lanes (milestones 720 to 724) wrote blocks `promoted_from` proposals that were
   on #1513's branch but not on theirs. Neither branch could have deleted the file. The union
   could, and the union failed.
4. Why was that possible? `design/roadmap/proposals/README.md` says promotion is a `git mv`, and
   `script/roadmap` checks only that the proposal is gone, not that this branch ever had it.
5. Why are counts typed by hand? Milestone 125 (a number in the prose is a claim) gates them, and
   nothing generates them. `notes/verification.md` has listed the collision in its BUGS since
   2026-10-02, and 31 commits since 09-01 fix a count.

## Root cause

Every mechanism here reads one pull request against `main` as it was. The drain reads a pull
request's state and not the person's act on it. The hook reads the branch and not the union. A
branch is asked to commit facts that only the merged tree can make true. The queue is the only
thing in the pipeline that reads the union, and today it ended up doing most of the checking.

The kinds of conflict, measured:

- Facts about the merged tree (counts, promotions, word budgets) cost 33 of the 47 failed
  groups. This is the cause.
- Appends at one spot (#1547's risk-7 entry against #1558, #1529's BUGS bullet against #1533,
  #1547's bullets against #1544): three hand merges.
- Ratchet churn (the bold `**yes**` in #1534; a §19 (architectural parity is a tenet) gloss moved between rows in #1547): it rode
  along with other conflicts and caused none on its own.
- Real overlap (#1530 against #1543, #1552 against #1530, #1548 against #1541, #1534 against
  #1526, #1377, #1517): six hand merges. These are what conflicts are for.

## Action items

Highest rung first. The three proposals hold the options and the measured cost of each.

- **Proposed.** `design/roadmap/proposals/a-branch-commits-no-fact-about-the-merged-tree.md` (rung
  2, plus one rung-1 deletion). `script/roadmap` fails a pull request whose block is `promoted_from`
  a proposal its branch never held, and the hand-typed Kani harness count leaves the prose. That
  covers 31 of today's 47 failed groups and 3 of its 13 conflicted merges.
- **Proposed.** `design/roadmap/proposals/the-drain-honours-a-dequeue-by-hand.md` (rung 2). A
  person's dequeue at an unchanged head holds the pull request until it gets a new head or the
  person re-queues it. That covers today's 7 re-adds and 4 of the failed groups.
- **Proposed.** `design/roadmap/proposals/the-pre-push-hook-runs-what-fits-in-seconds.md` (rung 2,
  narrowed). The hook runs `script/fmt --check` and `--ready-branch`, and full lint is left to CI.
  That takes 70 to 202 s off every push, and today it would have let through no lint failure that
  reached the queue.

## How the numbers were made

```
# timelines: queue, auto-merge, head events, with actors (helpers/merge_queue_share.py's paging)
gh api graphql -f query='...pullRequests(first:40,orderBy:{field:UPDATED_AT,direction:DESC})
  {nodes{number timelineItems(last:100,itemTypes:[ADDED_TO_MERGE_QUEUE_EVENT,
  REMOVED_FROM_MERGE_QUEUE_EVENT,AUTO_MERGE_ENABLED_EVENT,AUTO_MERGE_DISABLED_EVENT,
  HEAD_REF_FORCE_PUSHED_EVENT,PULL_REQUEST_COMMIT]){...actor{login} reason}}}'
# group runs, and the failing check in each
gh api --paginate "repos/nifeos/nife/actions/runs?event=merge_group&created=2026-10-03"
gh run view <id> --log-failed | grep -B4 'lint ends'
# conflicts: every first-parent merge on a pull request's head that is not on main
git show --remerge-diff <merge> | grep '^remerge CONFLICT'
# the hook, per check
script/lint 2>&1 | perl -MTime::HiRes=time -ne 'printf "%.2f %s", time-$s, $_'
```

## BUGS

- A pull request that rebased instead of merging hides its conflicts from the remerge count, so
  13 is a floor.
- That a re-add caused a failed group is inferred from timing: the group started within 20 s of the
  re-add, at the same head.
- The hook's timings come from a loaded laptop, which is the machine lanes push from. An idle
  machine would be faster, and this one is rarely idle.
- #1494's `merge_conflict` ejection was not traced to the pull request that caused it.
