# A correction of error: the merge rate fell, and the two signals nobody read

*Recorded 2026-10-03 (UTC) by a maintainer session, commissioned by calef the same day, and revised
the same day after his review on #1513. Title provisional. Every figure below was re-derived from the
measurement lane's data or measured again by this session. What is inferred says so.*

## What happened

From 2026-09-28 to 2026-10-02 (UTC) the tree merged 45 pull requests. The ten days before merged 467,
46.7 a day, so the same five days at that rate would have been about 233. That baseline is inflated:
it holds 09-24's 101 merges, and the fifteen days before 09-28 average 39 a day, about 195. Either
way the shortfall is roughly 150 to 190 merges. Read it as deferred, not lost: 10-03 merged 29 once
work resumed.

| day (UTC) | PRs created | PRs merged | queue non-empty, h | Claude Code turns | z.ai turns | long CI job, median min |
|---|---|---|---|---|---|---|
| 09-26 | 75 | 41 | 12.5 | 20,085 | 0 | 29.3 |
| 09-27 | 41 | 64 | 15.0 | 17,950 | 0 | 37.8 |
| 09-28 | 6 | 7 | 2.8 | 1,364 | 4 | 40.6 |
| 09-29 | 17 | 20 | 6.3 | 0 | 1,646 | 42.2 |
| 09-30 | 19 | 11 | 14.1 | 0 | 2,721 | 41.5 |
| 10-01 | 2 | 7 | 1.4 | 0 | 159 | 38.4 |
| 10-02 | 0 | 0 | 0.0 | 1 | 1 | none ran |
| 10-03 | 28 | 29 | 21.5 | 6,096 | 0 | 22.9 |

A turn is one assistant message. The two vendors' turns are not the same unit of work, so the
columns are not summed.

Over the 120 hours from 09-28 to 10-02 the merge queue was empty for 95.4 (79%). Pull requests
created and merged in those five days were 44 and 45. The queue drained what arrived. Little arrived.

Three causes, in order of size. The estimates are judgment over the table, not a model.

| cause | merges, estimated | evidence |
|---|---|---|
| 1. Claude's weekly usage limit, bridged in part by z.ai | 150 to 180 | Claude out 09-28 12:40 to 10-03 00:00; z.ai ran 09-28 22:00 to 10-01 05:07 |
| 2. the long CI job crept into its timeout | 10 to 20 | 29 merge groups cancelled at 45 minutes on 09-30 |
| 3. a manual queue hold, then a disabled drain | about 5 | four pull requests held 7.8 h; drain off 09-30 21:40 to 10-03 |
| flakes | under 5 | two UART splices and one load-sensitive network check, 09-28 to 10-01 |

The maintainer's first hypothesis, before anything was measured, weighted flakes and lane count most
heavily. The measurement overturned it. Flakes cost under five merges, and lane count was near zero
for most of the window because little could run.

### Cause 1: the week's Claude allowance went in two and a half days

The Claude usage limit messages, read from this machine's Claude Code transcripts:

| when (UTC) | message | window |
|---|---|---|
| 09-25 07:46 | weekly limit, resets 5pm Pacific | the previous week, out 16.2 h before its reset |
| 09-26 00:00 | (reset) | new weekly window opens |
| 09-26 18:44 | session limit | |
| 09-28 12:40 | weekly limit, resets Oct 2 at 5pm Pacific | out 2.5 days into a 7-day window |
| 10-03 00:00 | (reset) | |

09-26 and 09-27 are the two heaviest days measured, in turns (20,085 and 17,950) and in output
tokens (8.2 and 6.6 million).

calef bridged the outage with z.ai's coding plan (in the ledger from 09-28 at $80 a month), running
glm-5.3 and glm-5.3-flash under opencode. Its session database on this machine dates that use:

| when (UTC) | what | source |
|---|---|---|
| 09-25 07:46 to 09-26 00:00 | the earlier outage: no z.ai, and only two bot commits | opencode database, git |
| 09-28 12:40 to 22:00 | nothing ran on either vendor, about 9 hours | both |
| 09-28 22:00 | first z.ai messages | opencode database |
| 09-29 02:33 | first z.ai session in a nife worktree; first non-bot commit 02:38 | opencode database, git |
| 09-30 06:00 and 21:57 | z.ai's own five-hour limit, twice | opencode database |
| 10-01 05:07 | last z.ai work, after connection errors from 04:38 | opencode database |
| 10-01 05:07 to 10-03 00:00 | nothing ran, about 43 hours | both |

Measured: 54 z.ai sessions, 26 of them in nife worktrees; 4,554 assistant messages; 154 non-bot
commits authored in the Claude outage, none carrying a `Co-Authored-By` or `Claude-Session`
trailer. Inferred: those commits are the z.ai sessions' work, because no Claude transcript exists
for those hours and opencode does not add the trailers Claude Code does. Inferred, from z.ai's own
reset times landing on the hour of resumed work only when read as UTC+8: z.ai reports reset times
in China Standard Time.

So the 17 and 19 pull requests created on 09-29 and 09-30 were z.ai's, not missing Claude sessions,
which is what this record first guessed. The bridge produced about a third of a normal week's daily
pull requests, and it stopped on 10-01 for 43 hours before Claude reset.

### Cause 2: the long CI job doubled, then crept into its timeout

The merge group's long job (`build + test`, and its successors after the 09-30 split):

| when (UTC) | what changed | long job |
|---|---|---|
| to 09-19 20:44 | | 11 to 13 min, against a 30-minute timeout |
| 09-19 21:13 | #987 merged, milestone 182 (x86_64's own interactive-boot entry point) | 22.3 min, the next run and every run after |
| 09-26 17:11 | #1340's six prompt lines cancelled at 30 min; commit 2556dd307 raised the timeout to 45 | 26 to 35 min |
| 09-26 to 09-30 | swish-check lines 62 to 137 | 25 to 45 min, about 15.6 s per added line (least squares, 421 runs) |
| 09-30 01:44 to 18:30 | 29 merge groups cancelled at 45.0 to 45.6 min | its full-length passes finished at 44.8 to 45.0 |
| 09-30 19:54 | #1468 split `build + test` into one job per check | swish-check 38 min |
| 10-03 05:00 | #1487, milestone 628 (the x86_64 swish-check leg costs what the others do) | x86_64 leg 3m17s under KVM |

From the first timeout at 01:44 to #1468's merge at 19:54 on 09-30, 18.2 hours, the six pull
requests that landed (#1447, #1454, #1456, #1460, #1461, #1462) were notes, proposals and metrics.

The root defect was in the kernel, not in CI. #873 (09-15) made the x86_64 boot thread end in
`arch::halt()`, a `hlt` loop, while it was still on the run queue. #882 (09-15) gave x86_64 a
polling input driver that yields forever, so round robin reached the halting thread constantly and
each visit stopped the core until the next 10 ms tick. #1487 measured 7.7 s per line on that leg
against 0.19 s on aarch64. The job-level slope of 15.6 s is about twice #1487's per-leg figure;
that the x86_64 text and graphical legs both paid it is inferred, not measured.

It was half seen twice. On 09-19, the day the job doubled, commit b3e5459e7 recorded that the poll
"starves the idle loop" and filed it as a CPU cost, now milestone 505 (an x86_64 input driver that
never lets the core idle). Nothing connected it to CI time.

### Why the timeout was raised on 09-26

The record is the lane's own: commit 2556dd307 and #1340's body, under "CI timeout raised (its own
commit)". No maintainer reasoning about it survives in any transcript on this machine. The lane's
reasons, in order:

1. The bound "exists to stop a hung QEMU holding a runner for six hours", so it is "a hang
   detector, not a budget".
2. An honest run was already close to it: main took 26m21s that day, "x86_64's swish-check alone is
   about 17 minutes" under OVMF.
3. The lane's six new prompt lines pushed the job past 30, "cancelled in the last graphical leg with
   every check before it green".
4. 45 "matches the other long jobs here and still catches a hang".

The body added: "This is shared CI config. If a maintainer would rather trim lines, it is one commit
to drop." It merged inside #1340 at 18:14 with no separate decision.

Was it reasonable with what was known then? On the lane's own frame, yes: a timeout read as a hang
detector should sit above an honest run, and the run was honest. But the frame was the error. The
timeout was the only thing in the tree that read the job's wall time, so raising it switched off the
only alarm. The lane had the anomaly in hand, one leg at 17 minutes, and did not ask why it cost what
it did. The doubling a week earlier was in the run history and nobody read it. And the bound was
ours: 45 minutes is the `timeout-minutes` in `ci.yml`, not a GitHub limit, which ends a hosted job
only at six hours. A one-line raise cost nothing to make, so nothing forced the question. That is
the measure-first miss: a remediation chosen before the question was named
([design/tenets/measure-first.md](../../design/tenets/measure-first.md)). The CI note beside the job
then explained the cost as OVMF's framebuffer drawing, an explanation #1487's measurement overturned.

### Cause 3: a hold, then a drain nobody saw stop

At 19:03 on 09-30 four pull requests (#1450, #1457, #1458, #1459) were dequeued by hand around
#1468's split. They were re-enqueued at 02:53 on 10-01, 7.8 hours later. The merge-drain workflow
then sat `disabled_manually` from about 21:40 on 09-30 (an observation; the API keeps no history of
a workflow's state) until it was re-enabled on 10-03, before 08:05. Its `*/5` cron had already been
running at 1.8% of schedule from 09-24 to 09-30, 37 scheduled runs of 2,016. The cost was small only
because little was arriving. The ejection half of this is the class the
[2026-09-30 correction](2026-09-30-lane-follow-through.md) recorded, and its mechanism 5 (the drain
re-arms auto-merge) is still owed.

`briefs/session-start.md` already tells a session to check that the drain reads `active`. No Claude
session started between 09-30 and 10-03, because of cause 1, and the z.ai sessions did not run that
brief. `script/cadence-check` waits 15 days before calling a workflow dead, so at 10-03 it still
reported the drain live, 62 hours after the last run of a five-minute cron.

Other scheduled workflows were failing in the same week. None of them merges anything, so they are
not part of this record; #1511 (the scheduled-workflows milestone, not yet merged) owns them.

## Impact

Roughly 150 to 190 merges deferred over five days, most of them never started rather than blocked.
On 09-30, 29 merge groups ran into the 45-minute timeout, about 22 runner-hours, and only notes
landed for 18.2 hours. And calef spent a morning's attention on the question "why did the merge rate
fall", which a CI budget would have half answered on 09-19.

## Five whys

The root is the merge rate falling. The first answer splits in two, and each branch is followed to
its own fifth why.

1. Why did the merge rate fall? Few pull requests arrived (44 created in five days, against 39 to 47
   a day before), and on 09-30 the ones that did could not clear CI.

Branch A, supply:

2. Why did few arrive? Claude's weekly allowance ran out at 09-28 12:40. The z.ai bridge ran only
   from 09-28 22:00 to 10-01 05:07, hit its own five-hour limit twice, and then nothing ran for 43
   hours.
3. Why was the week's allowance gone in 2.5 days? 09-26 and 09-27 were the heaviest days measured,
   20,085 and 17,950 turns, with 85 and 114 transcripts active.
4. Why did the pace not change before the limit? Claude Code's warnings come within a five-hour
   window and as the weekly limit nears (calef, on #1513). By then the week is spent. The week
   before had also ended early, 16.2 hours before reset, and the pace still did not change.
5. Why is there no earlier signal? The meter is outside the tree, and nothing in the tree compares
   spend to the calendar. `script/effort` reports tokens per week after the fact, from Claude Code
   records only.

Branch B, CI:

2. Why could the 09-30 groups not clear CI? The long job ran into its 45-minute timeout 29 times.
3. Why was the job 45 minutes long? The x86_64 swish-check leg cost 7.7 s a line, because the boot
   thread halted while runnable and the polling input driver kept the scheduler reaching it.
4. Why did the growth go unnoticed for 10.5 days? Nothing reads CI wall time. The timeout was the only
   signal, and when it fired on 09-26 it was raised.
5. Why was raising it the answer? The timeout was framed as a hang detector, not a budget, so
   exceeding it read as a false alarm. It was our own setting and a one-line change, and no budget
   existed for anyone to defend.

## Root cause

Two, and neither is a person forgetting.

1. CI wall time is measured by nothing, and the one bound on it was ours to move. Not
   `script/metrics`, not [notes/project-metrics.md](../project-metrics.md), not `script/bench`, not
   `trunk-health`, not `merge-drain` reads it. The one CI measurement the tree commissioned,
   [ci-queue-remeasure](../../design/roadmap/proposals/ci-queue-remeasure.md) (09-24), measures
   queue waits, though it named splitting the x86_64 legs out as a lever. So a job that doubled in
   one merge stayed doubled, and when it hit its bound, the bound moved instead. Moving it cost
   nothing, because GitHub's own limit is six hours and 45 minutes was a line in `ci.yml`.
2. The token budget's burn-rate signal arrives too late to change a week's pace. Claude Code does
   warn, within a five-hour window and as the weekly limit nears, but both warnings come when most
   of the week is already spent. Nothing reads pace earlier, as used against elapsed. And the
   tree's own effort measure counts one vendor, so even after the fact a bridged week reads as idle.

A wall-time check would have fired at 09-19 21:13, about 10.5 days before the stop, if anything
had read the job's time: the job ran 22.3 minutes, over the 20-minute budget calef ruled on 10-03
(below). Relative rules were measured too. The jump was 1.84 times the trailing median of 12.1, so
1.5 times fires and twice does not. They lost to an absolute number because a reader can check a
number.

## The mechanisms

On CLAUDE.md's ladder, strongest first. Each rung-2 item is filed as a proposal for the maintainer to
number.

1. Rung 1: a boot thread cannot halt while it is runnable. #1487 fixed the x86_64 instance by
   ending the boot thread in `sched::exit()`. That is a corrected call site, not yet rung 1. On
   origin/main at 9d75b4518, aarch64 and riscv64 still end their boot threads in `arch::halt()`
   (`kernel/src/lib.rs`, the riscv64 hand-over and the end of the aarch64 boot), and #1487's
   `BUGS` says their cost is unmeasured. Proposal:
   [the-boot-thread-cannot-halt-while-runnable](../../design/roadmap/proposals/the-boot-thread-cannot-halt-while-runnable.md),
   which makes `arch::halt` take a token only the idle thread and the terminal paths can hold.
2. Rung 2: each merge-group CI job has a 20-minute budget (calef, 2026-10-03). It warns at 15
   minutes and fails above 20, unless a committed ratchet raises that job's budget and states the
   reason. `timeout-minutes` drops to 25, the budget plus margin, so creep stops a job early
   instead of burning 45 minutes a group. Main meets the budget today, measured over the last 25
   green merge-group runs (created 03:25 to 08:00 UTC on 10-03, re-measured by this session):

   | job | median, min | max, min |
   |---|---|---|
   | cpu matrix | 15.0 | 16.6 |
   | test | 10.3 | 11.4 |
   | fuzz | 7.0 | 7.5 |
   | clippy | 4.9 | 5.4 |
   | swish-check | 4.6 | 23.6 (one outlier) |
   | swish-check-x86_64 | 3.9 | 5.5 (17 runs; newer job) |

   cpu matrix has about 3.4 minutes of headroom and will hit the budget first; splitting it is the
   expected remedy.
   Proposal: [a-long-ci-job-checks-its-own-wall-time](../../design/roadmap/proposals/a-long-ci-job-checks-its-own-wall-time.md).
3. Rung 2: swish-check fails any architecture leg whose per-line time is over five times the median
   leg's, under the same accelerator. #1487's figures, 7.7 s against 0.19 s, are about 40 times.
   Proposal:
   [swish-check-fails-a-leg-five-times-the-others](../../design/roadmap/proposals/swish-check-fails-a-leg-five-times-the-others.md).
4. Rung 2: a disabled or silent merge watcher is reported within three of its own cron intervals,
   not fifteen days. This is the part of cause 3 that cost merges. Proposal:
   [a-stopped-merge-watcher-is-reported-at-once](../../design/roadmap/proposals/a-stopped-merge-watcher-is-reported-at-once.md).
5. Rung 2: machine effort and commit attribution count every vendor, so a bridged week does not read
   as idle. Proposal:
   [effort-and-attribution-count-every-vendor](../../design/roadmap/proposals/effort-and-attribution-count-every-vendor.md).
6. Rung 4, an exception and a foot gun: a pace line at session start, the share of the weekly
   allowance used against the share of the week elapsed, with the decision it forces (fewer lanes,
   cheaper models, or the z.ai bridge planned rather than improvised). Step 0 of
   [briefs/session-start.md](../../briefs/session-start.md). It stays because it is the only signal
   early enough to change a week, and Claude Code's warnings are not. No higher rung was found: the
   allowance is shown in `/usage` and on claude.ai, which a script cannot read, and a token count
   from transcripts lacks the denominator (inferred: the limit weighs cache reads and model, not
   output tokens alone). A session that skips the step learns nothing, which is why it is marked
   here.

## How the numbers were made

GitHub's REST API for pull requests, merge-queue events, `ci.yml` merge-group runs and their jobs
(2,553 job files, 09-10 to 10-03), and the logs of every failing merge-group job. Claude Code
transcripts under `~/.claude/projects` for turns, output tokens and limit messages, timestamps and
types only. opencode's session database (`~/.local/share/opencode/opencode.db`) for z.ai sessions,
messages, models and limit errors, read only. Commit trailers and author times from git. The
scripts were session scratch and are not in the tree; the method is this paragraph, and each table
above names its source.

## BUGS

- Cause attribution is estimated, not modelled. The 150 to 180 for cause 1 is the shortfall less
  causes 2 and 3, and it assumes lanes would have produced at the prior rate.
- The z.ai record is this machine's opencode database. Use on another machine would be missed.
- The drain's disable time is an observation, not an API fact. GitHub's audit log would settle it
  and was not read.
- The 15.6 s per line is a whole-job slope; the per-leg split is #1487's, not this measurement's.
- When Claude Code's own warnings fired on 09-27 and 09-28 is not recoverable: they are not written
  to the transcripts.

## Re-derivation, against the lane's summary

- The twice-median signal would not have fired on 09-19: the jump was 1.84 times. That finding fed
  calef's ruling for an absolute budget instead.
- The per-line cost on the job is about 15.6 s by least squares, segments 12 to 18, not 11 to 15.
- Commit 2556dd307 did measure where the time went (x86_64's leg, about 17 minutes); it did not ask
  why. And nothing needing tests is too strong for 09-30: six notes and metrics pull requests
  landed, and the long job's full-length passes cleared the timeout by seconds.
- The zero Claude turns on 09-29 and 09-30 were z.ai's sessions, not missing Claude transcripts.
