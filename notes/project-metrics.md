# Project metrics: what moved, week by week

*Name: `notes/project-metrics.md` is ratified (calef, 2026-09-02). He proposed it as
`design/project-metrics.md` and moved it here on the argument that `design/` holds arguments
(`fatal-risks.md`, the decisions, the roadmap) while `notes/` holds what was measured
(`benchmarks.md`, `mutation-testing.md`, `unsafe-obligations.md`). `script/metrics` and the
directory `notes/project-metrics/` are **provisional**; naming is calef's, and a lane ships a
provisional name and says so.*

One row per ISO week, computed by `script/metrics` from git history. The data is one CSV per
measure in [`notes/project-metrics/`](project-metrics/), in the tree, versioned with the code it
describes.

**This page is a deck** (calef, 2026-09-24). A chart is a heading, an image, and at most a line or
two saying what it plots and what would otherwise be misread. Everything longer, the definitions,
the arguments behind a measure, the deadlines, the reconciliations and the dated analyses, lives in
[`notes/register-of-measures.md`](register-of-measures.md), whose *The weekly series* table routes
each chart to the appendix that argues it. That register is the half of this pair that does not
change. Keep it that way: a caveat that cannot survive one line is
not a caption, it is a register entry.

<!-- toc: script/metrics writes this -->
- [The nine things that would kill nife](#the-nine-things-that-would-kill-nife)
- [Kani proof harnesses, and what can falsify them](#kani-proof-harnesses-and-what-can-falsify-them)
- [unsafe blocks outside kernel/src/arch/](#unsafe-blocks-outside-kernelsrcarch)
- [The same unsafe blocks, by trust boundary](#the-same-unsafe-blocks-by-trust-boundary)
- [Milestones built each week](#milestones-built-each-week)
- [Pull requests merged each week](#pull-requests-merged-each-week)
- [Which model wrote it](#which-model-wrote-it)
- [Lines touched, by the model that signed them](#lines-touched-by-the-model-that-signed-them)
- [What this project costs](#what-this-project-costs)
- [What a turn costs](#what-a-turn-costs)
- [Architecture decisions by status](#architecture-decisions-by-status)
- [Names by what the tree records about them](#names-by-what-the-tree-records-about-them)
- [Milestones by status](#milestones-by-status)
- [Rust in the tree](#rust-in-the-tree)
- [BUGS sections](#bugs-sections)
- [Coverage](#coverage)
- [The lowest-covered file](#the-lowest-covered-file)
- [The prose budget](#the-prose-budget)
- [The bold backlog](#the-bold-backlog)
- [Benchmark drift since a fixed anchor](#benchmark-drift-since-a-fixed-anchor)
<!-- /toc -->

## Read this before you read a number

- Every row is a restatement, not a report. The series applies *today's* definitions to old
  commits, which is right for a trend and is not what anybody believed at the time.
- Every figure here is derived by an agent and none of it is externally audited. Where a number
  has been checked two different ways, the register says so and names the method.
- A missing bar means the record did not exist, not that the number was zero. No empty cell is
  ever drawn as a nought.
- Weeks are ISO weeks in UTC, spelled `2026W36` everywhere, and the series starts at 2026W29
  because that is where the first commit falls. There is no 2026W28.
- The charts show the ten most recent weeks; the CSV keeps every one. A week is never deleted,
  only no longer drawn.
- A milestone is not a fixed unit and a pull request is not a unit of value. Read the shape, not
  the height.

## The nine things that would kill nife

![Fatal risks](project-metrics/fatal-risks.svg)

From `design/fatal-risks.md`, by "Experiment status": `RUN`, `NOT-RUN` or `CANNOT-RUN`, the field
calef ratified on 2026-09-23. It says whether an experiment happened, never what it found; the
verdicts are prose in that file. Every bar from 2026W36 is nine, the risks in that week's file:
where the field did not exist yet, or held a word with no meaning today, the risk is drawn as
"unclassified" rather than left out. 2026W35 and earlier are zero because the file reached `main`
on 2026-08-31 UTC.

## Kani proof harnesses, and what can falsify them

![Kani proof harnesses](project-metrics/harnesses.svg)

The harness count, split into those carrying a machine-replayable falsification record and those
without. A harness with no record at all is counted as unfalsified, because that is what it is.

## unsafe blocks outside kernel/src/arch/

![unsafe density](project-metrics/unsafe.svg)

Blocks per 10,000 code lines outside `kernel/src/arch/`, which is `script/lint`'s gated census; the
dashed line is the ceiling. The absolute count rose from 171 to 704 over the same period while the
density fell by two thirds, and only the ratio is about soundness.

## The same unsafe blocks, by trust boundary

![unsafe blocks by trust boundary](project-metrics/unsafe-trust.svg)

The same census split by what confines the code: kernel privilege, userspace confinement, both, and
the pre-kernel boot chain. Before 2026W38 the `unclassified` band is crates whose names have since
changed, so read the kernel and userspace bars in those weeks as undercounts of both sides.

## Milestones built each week

![Milestones built each week](project-metrics/velocity.svg)

A flow: milestones whose `Built:` date falls in that week, read from today's tree for every week.
It does not reconcile with the `Built` stock below, and that is the design; the gap is the lag
between finishing a milestone and flipping its row.
The line is the trailing ten-week mean, drawn rather than stored; it is dashed where fewer than ten
weeks exist behind it, which until 2026W47 is the left of the chart.

## Pull requests merged each week

![Pull requests merged each week](project-metrics/pull-requests.svg)

The second flow, counted from `main`'s merge subjects rather than the GitHub API so that it
backfills to the first commit. 2026W29 and 2026W30 are genuine zeros: the practice starts in
2026W31.
The line is the same trailing ten-week mean as the chart above, dashed where the window is short.

## Which model wrote it

![Commits each week, by the model that signed them](project-metrics/models-commits.svg)

From the `Co-Authored-By` trailer. Attributed plus unattributed plus merge is every commit that
week, and a merge is a pull request landing rather than a piece of writing. 2026W29 predates
the convention, so it is absent rather than zero.

The palette has eight hues, so "other models" is Opus 4.8, Fable 5.1 and any trailer the script
does not recognise, drawn as one band; `notes/project-metrics/models.csv` keeps each apart.

## Lines touched, by the model that signed them

![Lines touched each week, by the model that signed them](project-metrics/models-lines.svg)

Added plus removed, not net, and volume rather than effort. The hardest change of 2026-09-23
was a dozen lines and the largest was a mechanical sweep. Merges carry no line count.

## What this project costs

![Machine effort per milestone built](project-metrics/effort.svg)

Millions of lane tokens per milestone built. 2026W29 through 2026W33 are absent, not zero: those
records were never in git and were already gone when the capture started, and the current week is
always understated because its tokens accumulate all week.

## What a turn costs

![What a turn carried, in thousands of tokens of context](project-metrics/context-per-turn.svg)

Mean prompt per request, from session records that are not in git. A turn costs the size
of its context, not the size of its thought: cache reads are about 98% of every token
spent and output about 0.1%. 2026W29 through 2026W33 are absent, not zero.

## Architecture decisions by status

![Architecture decisions by status](project-metrics/decisions.svg)

From `design/decisions/README.md`. The grey band in the first three weeks is decisions that predate
a status existing at all, counted as having none rather than as `DECIDED`.

## Names by what the tree records about them

![Names by what the tree records about them](project-metrics/names.svg)

What each named thing's own provenance block says. The pale band is named things carrying no block,
which before 2026-08-04 is every one of them. A rising `Provisional` band is not debt: nothing
in this tree fails because a name is unratified, and the band worth an alarm is `Unrecorded`.

## Milestones by status

![Milestones by status, and proposals waiting for a number](project-metrics/milestones.svg)

From the milestone blocks in `design/roadmap/`, with unnumbered proposals stacked on top as the
eighth series: work identified and not yet entered into the roadmap, which unlike a provisional name
is closer to debt. The two zero weeks are a restatement artifact, not an empty roadmap; the line to
watch is `NOT-STARTED`, which grows faster than the lanes drain it.

## Rust in the tree

![Rust in the tree, in thousands of lines](project-metrics/lines.svg)

Every tracked `.rs` file outside `vendor/`, with `kernel/src` split from the rest, in thousands of
lines. Lines are volume, not effort. A line with code and a trailing comment counts as code.

## BUGS sections

![BUGS sections](project-metrics/bugs.svg)

Markdown headings plus Rust doc comments. Rising is good here: a `BUGS` section is a limitation
written next to the feature it limits, so a falling line is the alarming one.

## Coverage

![Coverage](project-metrics/coverage.svg)

Line coverage, each week measured by its own tree's `script/coverage` on its own pinned nightly, so
this is the one series on the page that is not a restatement. Weeks 2026W30 to 2026W38 were
backfilled that way on 2026-09-19. 2026W29 is empty because no instrument existed yet; the dashed line is the per-file floor `script/coverage` gates on, drawn for scale.

## The lowest-covered file

![Minimum per-file coverage](project-metrics/coverage-floor.svg)

The minimum per-file coverage, which is the number `script/coverage` actually gates on at 80%.
The aggregate above can hold steady while one file slides, so this is the panel that predicts a
failing build. 2026W31 to 2026W38 were backfilled on 2026-09-24 from each week's own lcov;
2026W30 is empty because the floor did not exist yet. The provenance is in the register's
[proofs and coverage](register-of-measures/code-proofs-and-coverage.md) appendix.

## The prose budget

![Words over the cap](project-metrics/prose-budget.svg)

![Documents over the cap](project-metrics/prose-budget-documents.svg)

calef ratified a 3,000-word cap per document on 2026-09-23, enforced as a ratchet. The first chart
is the debt, the words that would have to move into appendices for the tree to meet its own rule;
the second is how many documents that work sits in.

Weeks before the ratification are derived by `--backfill`, each from its own tree.

## The bold backlog

![Bold spans over the budget](project-metrics/bold-backlog.svg)

![Documents over the bold budget](project-metrics/bold-backlog-documents.svg)

This chart is temporary. calef ruled on 2026-09-26 that bold should be rare: 4 spans per 1,000
words, met by any document a change touches. He chose not to sweep the rest. The chart is
pulled once both panels reach zero. Then `script/metrics` prints "the bold backlog
is zero; retire this chart" and writes it here:

<!-- bold-backlog: script/metrics writes this -->
2026W39: 18465 spans over, in 949 documents.
<!-- /bold-backlog -->

Counted by `helpers/prose_ratchet.py`, from 2026W39.

## Benchmark drift since a fixed anchor

![Benchmark rows more than 5% above their anchor](project-metrics/baseline-drift.svg)

The tripwire compares each save with the last, so steps under 10% add up unseen. This counts rows
past 5% since fixed anchors (calef, 2026-09-26). Every row and reason: [the drift
report](project-metrics/baseline-drift.md).

<!-- baseline-drift: script/metrics writes this -->
2026W39: aarch64 `spawn_reap` +24.79%, riscv64 `spawn_reap` +20.91%, x86_64 `spawn_reap` +8.09%; 15 rows past 5%.
<!-- /baseline-drift -->

## How it stays current

`script/metrics --update` recomputes the current week's row and redraws the charts, and
`.github/workflows/metrics.yml` runs it daily and opens or refreshes a pull request if anything
changed. Coverage is taken on Mondays (UTC) only, because it is the one column that needs a build. The cost
columns cannot be produced by any workflow, because they are read from session records that live on
one laptop and never in git; `script/effort --snapshot` and `script/cadence-check` stand behind
them, and neither invents a number for a week nobody captured. [How the series stays
current](register-of-measures/reading-the-weekly-series.md#how-the-series-stays-current) has the
rest: the snapshot's `launchd` shape, why the file is idempotent, and which commit represents a week.

## BUGS

- `script/metrics --check` is deliberately not in `script/lint`. Any commit changes `HEAD`, and
  the current week's row records the commit it was taken at, so a gate on it would fail every pull
  request that touched anything. The workflow is the mechanism; this is rung two of `AGENTS.md`'s
  ladder declining to be rung one, said out loud rather than left as an omission.
- A newly added column can go blank across history if a CSV merge conflict is resolved with
  `--update` instead of `--backfill`. `--update` only touches the current week and any week
  missing outright; a row the CSV already holds is left exactly as it was, new columns included, so
  a merge that resolves two branches' concurrently-added columns into one header needs a `--backfill`
  afterward or the older rows carry the new columns as empty cells. This happened to
  `unsafe_trust_*` from 2026-09-21 to 2026-09-23 (the register's *The weekly series* has the whole
  case) and nowhere else, checked at the time. Empty, not zero, is the tell: `git diff` on
  `notes/project-metrics/` after any commit that merges two metrics branches is worth a look before
  trusting the row count.
- `milestones_built_this_week` will not equal the week-on-week change in the `Built` stock, in
  any week. It is deliberate and the register argues it, but it reads as an error to anyone who
  differences two rows and expects the flow to fall out, which is what happened on 2026-09-23.
  Nothing gates the two against each other and nothing can: the gap is the lag between finishing a
  milestone and flipping its row, which is a real property of the record rather than a defect in
  either column.
- A milestone dated in a week the series has no row for is dropped, which is milestone 1 (boot
  to Rust on QEMU `virt`, and print to the PL011 UART) and 2026W28. `script/metrics` prints a stderr
  line naming it since 2026-09-23; before that it was silent. A reader summing the chart gets one
  fewer than the tree's dated blocks.
- Two of the three definitions are now shared, and the third is checked instead (milestone 236 (three derivations are
  copied between scripts), 2026-09-03). The `unsafe` census and the comment-and-literal strip the code and comment line split
  is built on live in `helpers/rust_source.py`, which `script/lint` and this script both import, so
  there is one definition and nothing left to drift. The harness count could not be collapsed the
  same way: `script/lint` and `script/falsifications` attribute each harness to a workspace package
  out of `cargo metadata`, and this script reads blobs at revisions nobody has checked out and
  cannot run cargo against them. `script/lint` runs all three derivations and fails on a
  disagreement, which is the weaker answer and is said to be the weaker answer.
- A line inside a multi-line string literal counts as a comment line. Wrong in principle,
  negligible in this tree.
- The charts follow the reader's operating system colour preference, not GitHub's theme toggle.
  GitHub serves an SVG in a markdown page as an `<img>`, so a media query inside it cannot see the
  host page. A reader whose GitHub theme disagrees with their OS gets the wrong background.
- `patches/` is outside the `unsafe` census, inherited from `script/lint` along with its reason.
  That code does run on the machine, so it is a real hole rather than a boundary, and
  `notes/register-of-measures.md` records the blocks it leaves uncounted.
- `names_total` is not the sum of the naming columns, which is the one place a column here
  breaks the pattern the milestone and decision columns set. It is every named thing in that week's
  tree; the four statuses count the ones whose header carries a block that parses. The gap is drawn
  as a band and explained in the register, and it is kept rather than folded into `Unrecorded`
  because silence is not a claim.
- The naming columns count signatures, not names. A name can be ratified and bad, or provisional
  and perfect. Nothing here reads a name, and `script/names`' own `BUGS` is the longer version: it
  cannot check that a recorded reason is still true, that a date is right, or that a `recorded`
  citation leads anywhere.
- Four kinds of named thing, and the tree names more than four kinds. Crates, programs,
  `script/` entry points and Cargo packages carry provenance blocks, so those are what this counts.
  Public function and method names have been calef's call since 2026-08-23 and nothing counts them;
  types, `helpers/` helpers and directory names are outside `script/names`' surfaces too, and
  design/naming.md's `BUGS` carries what that leaves uncovered.
- `proposals_unnumbered` is a net count and cannot see the flow. Five proposals have left the
  directory and 81 remain; a flat line would be consistent with a stalled pile and with one
  draining as fast as it fills. The measurement that would tell them apart is the age of the oldest,
  which `script/roadmap --check` prints on every lint run and this column does not carry.
- `merged_pull_requests` can only see GitHub's default merge subject. A merge made any other way
  is not counted and cannot be distinguished from an ordinary merge commit afterwards. The total
  matches what a maintainer counted by hand on 2026-09-21, which is evidence and not proof.
- A band that goes to zero across the whole chart window changes the colours of the bands after
  it. `series_of` drops an all-zero series and the palette is indexed over what survives, so when
  2026W30 leaves the ten-week window the `before the convention` band disappears and every band
  below it in the by-model legend shifts one hue. The legend is redrawn with it, so nothing is
  mislabelled; a reader comparing two screenshots taken a week apart will still see a colour move.
  It is pre-existing behaviour of every chart here and it is recorded because the by-model panel is
  the first one certain to hit it.
- The cost and context columns carry seven limitations of their own, from a capture that can
  stop silently to a price [ledger](project-metrics/ledger.md) that re-prices history. They are listed where those columns are
  argued: [what this project costs, *Known limitations*](register-of-measures/project-cost.md#known-limitations).
- Four series were restated on 2026-09-24, and an older screenshot will disagree with each.
  Opus 5.5 got its own column (`opus_5_5`), so this week's commits left *other models*, and Opus 4.8
  now draws inside that band because the palette has eight hues. The fatal-risks chart gained
  `fatal_risks_total` and an *unclassified* band, so 2026W36 to 2026W38 read nine rather than five,
  four and five. The coverage floor and the prose budget were backfilled from each week's own tree.
- The prose budget will be restated again when milestone 586 (a prose ratchet in lint) lands,
  because it shares one document scope with `script/lint` and may count documents this one does not.
- Nothing here is audited by anyone outside this project. Stated once at the top and again here,
  because a dashboard is exactly the artifact that makes a reader stop asking.
