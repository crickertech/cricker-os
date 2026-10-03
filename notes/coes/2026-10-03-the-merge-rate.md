# A correction of error: the merge rate fell, and the two signals nobody read

*Recorded 2026-10-03 (UTC) by a maintainer session, commissioned by calef the same day, and revised
twice the same day after his reviews on #1513. Title provisional. Figures were measured by this
session or re-derived from the measurement lane's data; the commands are at the end.*

## What happened

The merge pipeline degraded for nine days before the work stopped arriving, and the two halves
compound.

From 2026-09-20 each merge cost more. The long CI job had doubled on 09-19, and the queue began
ejecting what it was given. Pull requests kept arriving and kept merging (53 a day from 09-22 to
09-27), but each took longer and more runs to land:

| days (UTC) | opened a day | merged a day | queue entries | ejected | share | median hours, opened to merged | group runs per merge |
|---|---|---|---|---|---|---|---|
| 09-15 to 09-19 | 26.6 | 24.2 | 134 | 11 | 8% | 0.9 | 1.05 |
| 09-20 to 09-21 | 41.0 | 37.0 | 100 | 24 | 24% | 1.0 | 1.36 |
| 09-22 to 09-27 | 57.3 | 53.2 | 551 | 235 | 43% | 2.4 | 1.80 |
| 09-28 to 10-02 | 8.8 | 9.0 | 67 | 22 | 33% | 4.8 | 2.09 |

Then, from 2026-09-28 to 10-02, little arrived. The tree merged 45 pull requests in those five days,
against 39 to 47 a day before. Read the shortfall, roughly 150 to 190 merges, as deferred rather
than lost: 10-03 merged 41 by 10:40 once Claude was back.

| day (UTC) | opened | merged | ejected | queue non-empty, h | Claude Code turns | z.ai turns | long CI job, median min |
|---|---|---|---|---|---|---|---|
| 09-26 | 81 | 41 | 33 | 12.5 | 20,085 | 0 | 29.3 |
| 09-27 | 41 | 64 | 15 | 15.0 | 17,950 | 0 | 37.8 |
| 09-28 | 6 | 7 | 1 | 2.8 | 1,364 | 4 | 40.6 |
| 09-29 | 17 | 20 | 2 | 6.3 | 0 | 1,646 | 42.2 |
| 09-30 | 19 | 11 | 17 | 14.1 | 0 | 2,721 | 41.5 |
| 10-01 | 2 | 7 | 2 | 1.4 | 0 | 159 | 38.4 |
| 10-02 | 0 | 0 | 0 | 0.0 | 1 | 1 | none ran |
| 10-03 | 47 | 41 | 8 | 21.5 | 6,096 | 0 | 22.9 |

10-03 runs to 10:40 for GitHub and about 08:00 for the rest. A turn is one assistant message, and
the vendors' turns are not summed. "Merged" counts the merge queue's merge events.

Three causes, in order of size. The estimates are judgment over the tables, not a model.

| cause | merges, estimated | evidence |
|---|---|---|
| 1. Claude's weekly usage limit, bridged in part by z.ai | 150 to 180 | Claude out 09-28 12:40 to 10-03 00:00 |
| 2. the long CI job doubled, the queue ejected 43%, and the job crept into its timeout | 10 to 20 in the window, more before it in turns and hours | 29 merge groups cancelled at 45 minutes on 09-30 |
| 3. a manual queue hold, then a disabled drain | about 5 | four pull requests held 7.8 h; drain off 09-30 21:40 to 10-03 |
| flakes | under 5 | two UART splices and one load-sensitive network check, 09-28 to 10-01 |

Cause 2 cost few merges before 09-28 because Claude was paying for it. Measured: each merge took 2.7
times as long and 1.7 times as many group runs. Inferred, not measured: the rebases, re-runs and
triage that bought those merges drew on the allowance cause 1 then ran out of.

## Timeline

| when (UTC) | what | source |
|---|---|---|
| 09-15 | #873 ends the x86_64 boot thread in `arch::halt()` while it is still runnable; #882 adds a polling input driver that yields forever | git |
| 09-19 21:13 | #987 merges, milestone 182 (x86_64's own interactive-boot entry point); the long CI job goes from 11 to 13 minutes to 22.3, every run after | CI jobs |
| 09-19 | commit b3e5459e7 records that the poll "starves the idle loop" and files it as a CPU cost, now milestone 505 (an x86_64 input driver that never lets the core idle) | git |
| 09-20 to 09-21 | the queue ejects 24% of its entries, from 8% the five days before | queue events |
| 09-22 | 81 of 93 queue entries ejected, 40 on merge conflicts and 37 on failed checks; 15 merges | queue events |
| 09-25 07:46 | Claude's weekly limit, in the previous window, 16.2 h before its reset | transcripts |
| 09-26 00:00 | a new weekly window opens | |
| 09-26 17:11 | #1340's six prompt lines are cancelled at 30 minutes; commit 2556dd307 raises the timeout to 45 | git, CI |
| 09-26 and 09-27 | the heaviest days measured: 20,085 and 17,950 turns, 8.2 and 6.6 million output tokens; median opened-to-merged reaches 5.4 h on 09-27 | transcripts, queue |
| 09-28 12:40 | Claude's weekly limit, resetting 10-03 00:00, 2.5 days into a 7-day window | transcripts |
| 09-28 12:40 to 22:00 | nothing runs on either vendor | both |
| 09-29 02:33 | first z.ai session in nife ("My claude session ran out of tokens"); first non-bot commit 02:38 | opencode, git |
| 09-29 20:57 | a lane is launched to chase the swish-check flake that failed #1442 and #1444 | opencode |
| 09-30 01:44 | the first of 29 merge groups cancelled at 45.0 to 45.6 minutes | CI |
| 09-30 05:08 | calef: "Has the last three days of merges been more painful than usual? It feel like our progress has ground to a crawl." | opencode |
| 09-30 06:00 | z.ai's own five-hour limit | opencode |
| 09-30 15:51 | calef: "We're on day four of compromised throughput." | opencode |
| 09-30 18:30 | the last timeout cancellation | CI |
| 09-30 19:03 | calef clears the merge queue; #1450, #1457, #1458 and #1459 dequeued around #1468's split | queue events, opencode |
| 09-30 19:54 | #1468 splits `build + test` into one job per check; swish-check 38 minutes | CI |
| 09-30 20:42 | calef sets the priority: "figuring out what's expensive in x86_64 and optimizing it" | opencode |
| 09-30 about 21:40 | the merge-drain workflow is `disabled_manually` (observed; the API keeps no history) | GitHub |
| 09-30 21:57 | z.ai's five-hour limit, a second time | opencode |
| 10-01 02:53 | the four held pull requests re-enqueued, 7.8 h later | queue events |
| 10-01 05:07 to 10-03 00:00 | the last z.ai work, after connection errors; then nothing runs for about 43 hours | opencode |
| 10-03 05:00 | #1487, milestone 628 (the x86_64 swish-check leg costs what the others do): that leg 3m17s under KVM | CI |
| 10-03 before 08:05 | merge-drain re-enabled | GitHub |

In the 18.2 hours from the first timeout to #1468, only six notes, proposals and metrics pull
requests landed.

### The defect under cause 2

It was in the kernel, not in CI. The polling driver never stopped yielding, so round robin reached
the halting x86_64 boot thread constantly, and each visit stopped the core until the next 10 ms
tick. The fix, #1487, measured 7.7 s per line on that leg against 0.19 s on aarch64.

### Why the timeout was raised on 09-26

Commit 2556dd307 and #1340's body give the lane's reasons: the bound is "a hang detector, not a
budget"; main already took 26m21s, with "x86_64's swish-check alone about 17 minutes"; the six new
lines pushed it past 30; and 45 "still catches a hang". It merged inside #1340 with no separate
decision.

On that frame it was reasonable. The frame was the error. The timeout was the only thing that read
the job's wall time, so raising it switched off the only alarm, and the lane did not ask why one leg
cost 17 minutes. The 45 is our own `timeout-minutes` (a hosted job may run six hours), so the raise
cost one line and nothing forced the question. That is the measure-first miss
([design/tenets/measure-first.md](../../design/tenets/measure-first.md)).

### The drain nobody saw stop

Its `*/5` cron ran 37 of 2,016 scheduled times from 09-24 to 09-30. No Claude session started between
09-30 and 10-03 to run `briefs/session-start.md`'s check, and the z.ai sessions did not run it.
`script/cadence-check` still called the drain live on 10-03, 62 hours after its last run.

## What the z.ai bridge spent, and what it delivered

calef bridged the outage with z.ai's coding plan ($80 a month, in the ledger from 09-28), running
glm-5.3 and glm-5.3-flash under opencode: 54 sessions, 4,554 assistant messages (3,903 of them
glm-5.3), 962,000 output tokens and 701 million cache-read tokens. The outage's 154 non-bot commits
carry no Claude trailer and no Claude transcript covers those hours, so they are inferred to be
these sessions' work. By what each lane session, and each of calef's 172 prompts to the maintainer
session, was about:

| spent on | messages | share | output tokens | share |
|---|---|---|---|---|
| the merge-rate problem: CI time, flakes, red checks of unknown cause | 1,967 | 43% | 396,685 | 41% |
| landing pull requests opened before the outage: rebases, conflicts, red checks on their own subject | 1,029 | 23% | 188,562 | 20% |
| new work: audits, proposals, package versions, week notes, fatal-risk colours, graphics, a Kani review | 1,408 | 31% | 343,001 | 36% |
| running the session itself | 121 | 3% | 29,997 | 3% |
| not nife (homelab) | 29 | 1% | 3,714 | 0% |

The largest merge-rate items were #1377's inbound failure (370 messages over four sessions), the
x86_64 paint path (354) and the swish-check flake (270). The #1377 failure turned out to be the
branch's own: 12 of 12 boots failed on it and 12 of 12 passed on main. Counted as landing work
instead, the merge-rate share is 35%. Either way, more than a third of the bridge went to diagnosing
the problem this record is about.

What landed. From 09-28 12:40 to 10-03 00:00, 40 pull requests merged:

| kind | merged | pull requests |
|---|---|---|
| merge-rate fixes and measurements | 12 | #1444, #1446, #1447, #1454, #1457, #1458, #1461, #1465, #1466, #1467, #1468, #1470 |
| landing older work | 7 | #1360, #1369, #1374, #1381, #1413, #1424, #1434 |
| new work | 14 | #1440, #1441, #1442, #1445, #1448, #1449, #1450, #1451, #1452, #1453, #1455, #1456, #1459, #1460 |
| scheduled bots | 7 | #1435, #1436, #1437, #1438, #1439, #1462, #1463 |

Milestones 126, 141 and 623 turned BUILT, nine new blocks were numbered, and eleven proposals were
filed. After Claude returned, more z.ai work merged: #1473 is milestone 624 (the x86_64 paint path
stops repainting the world), and #1472, #1443, #1377, #1370 and #1286 followed. Three closed unmerged:
#1469, #1471 and #1464. The claim-only #1460 merged with nothing in it; its work was rebuilt as milestone
625 (fatal risk colors, counted by week) in #1485, and the empty-diff check now refuses that shape.

## Impact

Roughly 150 to 190 merges deferred over five days, most never started. Before that, nine days in
which each merge cost 2.7 times the wall time and 1.7 times the group runs. On 09-30, 29 merge groups ran into the 45-minute timeout, about 22
runner-hours. Between 35 and 43% of the z.ai bridge went to diagnosis rather than to new work.

## Five whys

The first answer splits in two, and each branch reaches its own fifth why.

1. Why did the merge rate fall? From 09-20 each merge cost more, and from 09-28 few pull requests
   arrived (44 in five days, against 57 a day the week before).

Branch A, supply:

2. Why did few arrive? Claude's weekly allowance ran out at 09-28 12:40. The z.ai bridge ran from
   09-29 02:33 to 10-01 05:07, hit its own five-hour limit twice, and spent 35 to 43% of
   itself diagnosing branch B.
3. Why was the week's allowance gone in 2.5 days? 09-26 and 09-27 were the heaviest days measured.
   Inferred: part of that was branch B's friction, merges that took 1.8 group runs and 2.4 hours
   each, with the rebases and triage between.
4. Why did the pace not change before the limit? Claude Code's warnings come within a five-hour
   window and as the weekly limit nears (calef, on #1513). By then the week is spent. The week
   before had also ended early, and the pace still did not change.
5. Why is there no earlier signal? The meter is outside the tree, and nothing in the tree compares
   spend to the calendar. `script/effort` reports tokens after the fact, from one vendor.

Branch B, friction:

2. Why did each merge cost more from 09-20? The queue ejected 43% of its entries from 09-22 to 09-27,
   against 8% the week before: 90 on failed checks, 85 on merge conflicts, 51 by hand. On 09-30 the
   long job ran into its 45-minute timeout 29 times.
3. Why did it eject more? The long job doubled on 09-19, so each group held the queue twice as long
   while `main` moved under it. The reasons are measured; that the doubling drove the conflicts is
   inferred from the timing.
4. Why was the job slow, and getting slower? The x86_64 swish-check leg cost 7.7 s a line, because
   the boot thread halted while runnable and the polling driver kept the scheduler reaching it.
5. Why did it go unnoticed for 10.5 days? Nothing reads CI wall time or the queue's ejection share.
   The timeout was the only signal, and when it fired it was read as a false alarm and raised.

## Root cause

Two, and neither is a person forgetting.

1. CI wall time and the queue's ejection share are measured by nothing, and the one bound on either
   was ours to move. No script or watcher reads them;
   milestone 648 (re-measure CI queue waits by runner label) measures waits only. So a job that doubled in one merge stayed doubled, an ejection share that went from one in
   twelve to nearly half read as a busy week, and when the job hit its bound, the bound moved.
2. The token budget's burn-rate signal arrives too late to change a week's pace, and the tree's own
   effort measure counts one vendor, so even after the fact a bridged week reads as idle.

A wall-time check would have fired at 09-19 21:13, 10.5 days before the stop: the job ran 22.3
minutes, over the 20-minute budget calef ruled on 10-03. A twice-the-median rule would not have
fired (the jump was 1.84 times), which is why the budget is absolute.

## Action items

Highest rung first. `script/roadmap --check` fails a bullet that resolves to nothing, and
`script/metrics` charts how many are open each week.

- **Proposed.** `design/roadmap/proposals/the-boot-thread-cannot-halt-while-runnable.md` (rung 1).
  #1487 fixed the x86_64 call site; aarch64 and riscv64 still end their boot threads in
  `arch::halt()`.
- **Milestone 628.** The x86_64 swish-check leg costs what the others do, built 10-03 by #1487.
- **Milestone 505.** Seen on 09-19 and still open.
- **Proposed.** `design/roadmap/proposals/a-long-ci-job-checks-its-own-wall-time.md` (rung 2), the
  20-minute budget per merge-group job calef ruled on 10-03, with `timeout-minutes` at 25.
- **Proposed.** `design/roadmap/proposals/the-queue-reports-its-ejection-share.md` (rung 2), with
  a report for any day over 20%.
- **Proposed.** `design/roadmap/proposals/swish-check-fails-a-leg-five-times-the-others.md` (rung
  2); #1487's legs differed about 40 times.
- **Proposed.** `design/roadmap/proposals/a-stopped-merge-watcher-is-reported-at-once.md` (rung 2),
  within three of its own cron intervals.
- **Proposed.** `design/roadmap/proposals/effort-and-attribution-count-every-vendor.md` (rung 2),
  so a bridged week does not read as idle.
- **Milestone 630.** Built 10-03 as milestone 630 (a merge-queue ejection is caught before the
  queue, and recovered after it), the re-arm the
  [2026-09-30 correction](2026-09-30-lane-follow-through.md) left owed.
- **Done.** A pace line at session start, the weekly allowance used against the week elapsed, in
  step 0 of `briefs/session-start.md`. A rung-4 exception and a foot gun: only `/usage` and
  claude.ai show the allowance, and no script can read either.
- **Done.** Corrections' action items are counted, carried by #1513: `helpers/coe_actions.py`,
  `script/roadmap --check` and `--coe-actions`, and the `coe-actions` measure in `script/metrics`.

## How the numbers were made

```
# merge-queue events and pull requests, newest first, back to 09-06
gh api graphql -f query='query($endCursor:String){repository(owner:"nifeos",name:"nife"){
  pullRequests(first:50,after:$endCursor,orderBy:{field:CREATED_AT,direction:DESC}){
  pageInfo{hasNextPage endCursor} nodes{number title createdAt mergedAt state
  timelineItems(first:100,itemTypes:[ADDED_TO_MERGE_QUEUE_EVENT,REMOVED_FROM_MERGE_QUEUE_EVENT]){
  nodes{__typename ...on AddedToMergeQueueEvent{createdAt}
  ...on RemovedFromMergeQueueEvent{createdAt reason}}}}}}}'
# merge-group runs per day
gh api --paginate "repos/nifeos/nife/actions/workflows/ci.yml/runs?event=merge_group&created=2026-09-15..2026-10-03&per_page=100"
# z.ai: sessions, assistant messages and tokens, read only
sqlite3 "file:$HOME/.local/share/opencode/opencode.db?mode=ro" "select s.title, count(*),
  sum(json_extract(m.data,'$.tokens.output')) from message m join session s on s.id=m.session_id
  where s.time_created >= strftime('%s','2026-09-28')*1000
  and json_extract(m.data,'$.role')='assistant' group by s.id"
```

An ejection is a queue removal whose reason is not `merged` or `already_merged`. A lane session
is classed by its title and first prompt; in the maintainer session each message goes with the
prompt it answered, and the prompts were classed by hand. Claude figures come from the
`~/.claude/projects` transcripts, which hold no z.ai sessions, and CI job times from 2,553 job files.

## BUGS

- Cause attribution is estimated, not modelled, and assumes lanes would have produced at the prior
  rate.
- How much of 09-26 and 09-27's Claude spend went to friction is not measured. Two keyword
  classifiers over the transcripts' first prompts put it at 1% and at 36%, and a 15-prompt spot check
  found the looser one matching boilerplate, so neither is reported.
- The z.ai classification is by hand, and its boundary is a judgment: a red check on a pull request's
  own subject is landing work, one of unknown cause is the merge-rate problem.
- The z.ai record is this machine's opencode database. Use on another machine would be missed.
- The drain's disable time is observed, not read from an API.
- When Claude Code's own warnings fired on 09-27 and 09-28 is not recoverable: they are not written
  to the transcripts.
