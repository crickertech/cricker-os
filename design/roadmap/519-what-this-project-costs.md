---
status: BUILT
raised: 2026-09-21
built: 2026-09-21
---
# 519. What this project costs, tracked where it cannot rot

Built 2026-09-21. Minted 2026-09-21 by calef, from a conversation about what nife would
have to be to change anyone else's behaviour. Every honest answer began with a cost number this tree
cannot state. *(Number provisional until the merge queue lands it.)*

Built the same day it was minted, and the sequencing was the deadline's: the capture ran and was
committed before any of the machinery around it was written, because this week's session records
existed on 2026-09-21 and were not promised to exist on 2026-09-28. What the first capture found, and
what it found already gone, is in the record below.

## Why this is a milestone and not a note

`AGENTS.md`'s second principle says the method is a result. A method's result is a cost, and this
tree measures only scale: milestones, lines, harnesses, proofs, and since 2026-09-20 a velocity
chart. None of it answers what an outsider asks, which is what a trustworthy system costs to build
this way.

The literature this project is compared against is denominated in exactly that unit. seL4 reports
about eleven person-years plus nine more at a 20:1 proof-to-code ratio; Atmosphere reports 1.5
person-years on verification alone at 3.32:1. Those numbers move institutions in a way a feature
list does not.

And the inputs are known today and unrecoverable tomorrow, which is what makes this urgent rather
than tidy. See the deadline below.

## What is known, as of 2026-09-21

| input | figure | source |
|---|---|---|
| Human effort | **about ten person-weeks** | calef works on nife full time, so one calendar week is one person-week. His own statement, 2026-09-21 |
| Inference | **$200 a month since 2026-07-12**, about $470 to date | calef, 2026-09-21 |
| Hardware | **$89.99** for argon, the Jetson TX1 | milestone 127 (the seL4 machine: a Jetson TX1)'s own record |
| Other hardware | **$181.82, and the line above is wrong** | xenon *was* bought for this project: milestone 87 (the x86_64 bare-metal machine) records the OptiPlex, its serial module and the RS-232 chain, bought 2026-08-15 for the third ISA target §19 (architectural parity is a tenet; the targets are aarch64, riscv64, and x86_64) names. Corrected by this lane against the tree; the original line was written from memory |
| Still unpriced | no purchase record in this tree | radon, the UART adapters and the smart plugs. `notes/riscv-port.md`'s "~$70" is a sentence about the market, not a receipt, and the ledger leaves the row out rather than guess |
| Machine effort | **22.3 billion tokens over six weeks**, captured 2026-09-21 | the harness's session records on patagonia, which this tree now keeps a committed aggregate of in `notes/project-metrics/effort.csv` |

The shape those numbers make is the finding, and it should be stated before anyone optimises the
wrong term. Cash to date is under a thousand dollars all in, against about ten person-weeks of one
experienced engineer's full attention. At any plausible rate for that person's time, the human cost
is on the order of 99% of the economic cost and the machines are a rounding error. The claim this
project can honestly make is not that software became cheap; it is that the scarce input is still a
person, and what changed is how much one person's attention can be made to carry.

## The deadline, which decides the sequencing

Every other column in `notes/project-metrics/weekly.csv` is computed from a git revision, so history
backfills by reading the tree at a past commit: that is how the mutation census and the velocity
column were both filled in retroactively.

**Token and wall-clock records are not in git.** They live in session task records outside the
repository and they are discarded. The ten weeks behind us are already unrecoverable, and every week
that passes without capture is another one lost. The small version now beats the complete version
later, because this week's numbers exist today and will not exist next week.

## What to build

1. Columns in the weekly series, in `script/metrics`, because the deck exists and its page already
   has the conventions for being honest about its own numbers:

   | column | source | note |
   |---|---|---|
   | `human_person_weeks` | 1.0 per calendar week, by calef's standing datum | corrected by hand when it stops being true |
   | `lane_tokens` | task records, summed per week | the physical quantity |
   | `lane_wall_clock_hours` | task records | conflates queueing; say so |
   | `price_per_mtok_at_date` | recorded by hand when it changes | what makes dollars re-derivable |
   | `cash_spend` | subscription plus dated purchases | what was actually paid |

2. **A ledger for what is not weekly**: hardware purchases with dates and prices, and the
   subscription's start date and rate. One committed file, appended to, never regenerated.

3. One chart, and it is a ratio rather than a total. Machine effort per milestone built, beside
   the velocity chart. A total says the project is busy; a ratio says whether the method is getting
   cheaper, which is the claim.

4. The mechanism that keeps it current, which is the half that usually fails. Weekly capture must
   not depend on anybody remembering: the metrics workflow already runs on a schedule, and the
   question a lane has to answer is what it can read without a human present. If the honest answer
   is that some input needs a person once a week, say so and make the gap visible rather than
   pretending otherwise (`script/cadence-check` is this tree's existing shape for a thing that is
   due and has not happened).

## What was built, 2026-09-21

`script/effort` (provisional name) reads the harness's session records on this machine and writes
`notes/project-metrics/effort.csv`, one row per ISO week and model. It splits tokens four ways
because the API prices them between 0.1x and 2x of each other, and splits by model because this
tree's mix is roughly half `claude-opus-5` and half `claude-sonnet-5` by token. A single blended
number would have had no derivation behind it, which is what the dollar refusal below is about.

Five columns in `notes/project-metrics/weekly.csv`, exactly as the table above specifies, filled
from two committed records rather than from any revision: `effort.csv` and a new
`notes/project-metrics/ledger.md`. Both are read from the working tree for every week, which is
`built_by_week`'s precedent and is there for its reason. A sixth column, `merged_pull_requests`,
arrived on calef's request the same day and is derived from git's own first-parent log: 996 merges,
backfilled to the first commit, with no dependence on an API that retains ninety days.

One chart, `effort.svg`, and it is a ratio: millions of tokens per milestone built, beside the
velocity chart. It fell from 307 in 2026W34 to 73 and 68 in 2026W36 and 2026W38.

The ledger carries hardware with dates and prices, the subscription's start and rate, and a dated
table of vendor list rates per model, which is what makes the shadow price re-derivable rather than
asserted.

The mechanism. A GitHub runner cannot see the session records, so the weekly workflow cannot
capture them and no workflow ever will. `script/effort --snapshot` writes a machine-local cache
outside the repository, safe to run unattended from `launchd`, so a week's numbers outlive the
transcripts; `script/cadence-check` gained its one non-workflow row and reports when the committed
file does not hold the current week, through the watcher patagonia already runs. `--update` refuses
to lower a figure a past week already carries, because the way this measurement dies is a pruned
transcript and a later run writing the smaller number over the larger one.

## What the first capture found, and what was already gone

Read on 2026-09-21 from 477 record streams:

| weeks | state |
|---|---|
| 2026W29 - 2026W33 | **gone.** No session record survives anywhere on this machine. Five weeks, from the first commit on 2026-07-12 |
| 2026W34 - 2026W39 | captured. 22.3 billion tokens, 523 machine-hours, six weeks |

They are recorded as **absent, not zero**, in the CSV and on the chart, which now draws a marker for
a week nobody measured. Coverage's own 2026W29 gap got the same marker; it had been a sentence in a
subtitle.

## What this must not become

**Not time tracking for calef.** A person required to log hours to satisfy a metric stops
volunteering the honest ones, and the metric is worth less than what it displaces. His standing datum
is accurate enough for every comparison named above; the only thing ever asked of him is a correction
when it stops being true.

**Not a single headline number.** Three audiences want three units: person-weeks for the comparison
against the literature, tokens with a dated price for the argument that the method is getting
cheaper, and cash for calef's own budgeting.

**Not a dollar figure that hides which dollar it means.** This project pays a fixed subscription, so
its marginal cost per lane is zero. A retail-API figure is a shadow price: what somebody else
would pay to reproduce the work. Both are legitimate, and quoting one while implying the other is the
dishonest version.

## BUGS

- 2026W37 is captured and should be treated as suspect. 12.2 machine-hours against 110 to 133 on
  either side, in a week whose roadmap says 13 milestones were built. Either it was quiet or its
  records were rotated away before the first capture. Nothing can tell those apart after the fact,
  which is the whole argument for the snapshot.
- The subscription's first $200 is outside the series. calef dates it 2026-07-12, a Sunday, which
  is 2026W28; the first commit is 2026-07-13 UTC and the series starts at 2026W29. A week has a row
  only when a commit fell in it. `script/metrics` prints the discrepancy on every run rather than
  folding the money into a neighbouring week.
- **`lane_tokens` counts the whole project session, not its lanes.** A maintainer answering a
  question, a review, and this block being written are all in it. It is the cost of the project
  rather than the cost of the code, and the column name is narrower than the thing.
- **The ten person-weeks is a statement, not a measurement**, and nothing will ever make it one
  without time tracking, which this milestone refuses. It is accurate to the bucket the comparisons
  need and no better.
- Machine effort cannot be attributed to a milestone yet, and the raw material for it is on disk.
  Every session record carries a `gitBranch`, and a lane's branch is named for its milestone, so the
  join is *available*. It is deliberately not built: a branch is not a milestone (maintainer
  branches, `main`, rebases), and a wrong attribution is worse than none. Until somebody decides what
  a correct one looks like, per-component cost is not derivable and only per-week totals are. That is
  the one piece of this milestone that wants a lane of its own.

## Follow-on

- **Milestone 574.** The join from machine effort to a component is milestone 574 (what a lane
  spent on its milestone, joined from the branch it worked on),
  `design/roadmap/574-what-a-lane-spent-on-its-milestone.md`. Every session
  record carries a `gitBranch` and a lane's branch is named for its milestone, so the join from
  machine effort to milestone is on disk and was deliberately not built here: a branch is not a
  milestone (maintainer branches, `main`, rebases), and a wrong per-component cost is worse than
  none. It is the difference between "this project cost 22 billion tokens" and "a verified component
  costs this much", which is the number the literature comparison actually wants.
- **Recorded.** The suspect 2026W37 capture, the $200 of subscription outside the series, the
  restatement of old weeks at the newest ledger rate, and `lane_tokens` counting the whole session
  rather than its lanes all live in `BUGS` sections beside the thing a reader meets: this block's
  own, `notes/project-metrics.md`'s, `notes/project-metrics/ledger.md`'s, and `script/effort`'s.
- **Recorded.** `script/cadence-check`'s new row is checked by its output rather than by a run, so a
  capture that ran and wrote nothing because the records were pruned reads as live. That hole is in
  `script/effort`'s `BUGS`, where the reader who could act on it is.
- **Done.** The correction to this block's own hardware line, which said xenon was not bought for
  this project. Milestone 87 says otherwise and the ledger follows the tree.
- **Refused.** A single headline number, time tracking for calef, and a dollar figure that does not
  say which dollar it means. All three are calef's, settled at minting, and are recorded in `What
  this must not become` above and carried into the page's prose where a reader meets the figures.

## Index row

The method is half this project's claim, and a method's result is a cost; this tree measures only
scale, and the inputs that would answer it are discarded weekly.
