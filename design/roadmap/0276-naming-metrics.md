---
status: BUILT
raised: 2026-09-11
built: 2026-09-11
---
# 276. The dashboard counts milestones and decisions by status, and names not at all

Built 2026-09-11. Minted the same day by calef, from asking whether the metrics
dashboard carried naming data and finding it does not. *(Number provisional until the merge queue
lands it; milestone 275 is in flight ahead of it.)*

It gated on nothing while it ran, which was the block's own reading of it: not a design fork,
because `script/names` already computed the counts and `script/metrics` already had the machinery to
read a historical tree in bulk. Both held. The only thing that had to be built rather than reused
was the enumeration.

## The gap, stated plainly

`notes/project-metrics/weekly.csv` carries 34 columns. Milestones get seven, broken out by their
full status vocabulary (`built`, `partial`, `in_progress`, `removed`, `not_started`, `optional`,
`recorded`). Decisions get six the same way (`decided`, `amended`, `superseded`, `proposed`,
`unrecorded`). Names get none, despite having an exactly parallel four-word vocabulary that
`script/names` already computes and reports: `ratified`, `recorded`, `provisional`, `unrecorded`.

Today that reads 101 unratified of 205: 0 unrecorded, 37 recorded (a ruling only), 64
provisional. Nothing shows whether those numbers are climbing or falling week over week, which is
the one thing the other status columns exist to make visible.

## Scope added 2026-09-11, after the block was minted

A sixth column, `proposals_unnumbered`. calef, the same day, from the same question one record
over: `design/roadmap/proposals/` holds 81 unnumbered proposals and nothing on the dashboard counts
them. That is not an omission anyone made. `milestones()` matches index rows in
`design/roadmap/README.md` and keys on a milestone NUMBER, and a proposal is defined by not having
one (milestone 247, follow-on work named by a finished milestone goes nowhere, and this is the third
time), so the pile is invisible to that count by construction. The three `proposals/` strings in that
README are links inside other blocks' prose, not rows.

It belongs here rather than in a milestone of its own for the reason the naming columns do:
`script/roadmap --proposed` already computes it, and the work is a column and a parse to share.

The honesty clause applies differently to it, and copying the naming one across would have been
wrong. That is recorded in `BUGS` below and argued on the page.

## What to build

1. Columns mirroring the existing pattern: `names_ratified`, `names_recorded`,
   `names_provisional`, `names_unrecorded`, `names_total`. Follow `milestones()` and `decisions()` in
   `script/metrics` rather than inventing a third shape.
2. A chart, if it earns one. The existing page renders series that move. If this one turns out to
   be a near-flat line plus a slowly-growing provisional count, a table row may serve better than a
   plot; that is the builder's call from the data.
3. **The caveat on the page, not just in this block.** See below. It is the part most likely to be
   dropped as "obvious" and it is the part that keeps the number honest.

## Two things that make this harder than the existing columns

**The data is spread across every named thing, not in one index file.** `milestones()` and
`decisions()` each parse a single README. Naming provenance lives *at the name* (milestone 115's
whole design: the crate header that would carry a refusal is what a proposer actually reads), so the
computation has to read every program, crate, package manifest and `script/` entry point in that
week's tree. `script/metrics` already streams a whole tree's blobs in one `git cat-file --batch`, so
the machinery exists; the cost is the enumeration, not the reading.

The early weeks will read as zero, and that is an artifact rather than a fact. The series starts
2026W29 (2026-07-15). Provenance blocks did not exist until milestone 115, 2026-08-04
(2026W32). So three weeks of this series would show no ratified names, not because nothing was
ratified but because the convention that records ratification had not been invented. `script/metrics`'
own header already warns that *"every row is a RESTATEMENT, not a report... regenerated from git
history by applying TODAY's definitions to old commits"*; this series is the sharpest instance of that
hazard in the file, and the page should say so where those weeks are rendered rather than leave a
reader to infer a trend from a convention's birth.

## BUGS

- **A rising provisional count is not necessarily debt, and a dashboard makes it look like one.**
  `AGENTS.md` is explicit that `script/names --unratified` is *"a worklist rather than a wall,
  precisely so that an unratified name never blocks anyone's build."* A provisional name that works
  costs nothing while it sits. The page must say what a rising line does and does not mean, or this
  column manufactures pressure the project deliberately decided not to apply. Discharged on
  `notes/project-metrics.md`, in a subsection of its own rather than a sentence, and left standing
  here because the next person to change the chart inherits the hazard rather than the fix.
- **It measures the record, not the names.** A name can be ratified and bad, or provisional and
  perfect. This counts signatures.
- **`script/metrics`' own name is provisional** (its header, line 45: *"no architect has ratified it"*),
  so this milestone adds a naming metric computed by a tool that appears in its own unratified
  column. It does: `metrics` is one of the 63 `provisional` names in the 2026W36 bar, and so is
  `script/names`, which supplies the vocabulary.
- **A rising unnumbered-proposal count is closer to debt than a rising provisional name is, and the
  page says so rather than reassuring.** A provisional name costs nothing while it sits, because
  nothing waits on it. A parked finding is something this tree was found to be missing that nobody
  has scheduled. But the count cannot diagnose a stall: it is net, five have already left, and
  the tell is the age of the oldest, which `script/roadmap --check` prints on every lint run and
  this column does not carry. The page names that number as the one to watch, and does not pretend
  this one is it.
- It counts four kinds of named thing, and the naming rule covers more. Public function and
  method names have been calef's since 2026-08-23 and nothing counts them; types, `helpers/` helpers
  and directory names are outside `script/names`' surfaces, so they are outside this series too.
  design/naming.md's `BUGS` carries what that leaves uncovered.

## What was built

Five columns and a chart, in `notes/project-metrics/weekly.csv` and `names.svg`. At 2026W36:
**204 names, 104 `ratified`, 37 `recorded`, 63 `provisional`, 0 `unrecorded`**, which is
`script/names`' own table to the digit from a different walk of the tree.

The parse is shared rather than copied (`helpers/name_provenance.py`, a provisional name), which
is milestone 236's (three derivations are copied between scripts, and nothing notices when they
drift) rule applied the moment the dashboard became a second reader of these blocks. `script/names`
keeps its file walk and `script/metrics` keeps its `git ls-tree`, because a gate is asked about the
tree in front of it and a report is asked about eight revisions nobody has checked out; that is the
same line `helpers/rust_source.py` already draws. `script/names`' output is byte-identical on every
mode, checked against the pre-refactor run on stdout and stderr both.

A chart, and it earned one. The series was expected to be a near-flat total plus a slowly-moving
provisional count, which a table row would have served better. It is not: the bar goes 16, 40, 119,
135, 152, 175, 188, 204, `Unrecorded` collapses from 60 to zero in one week, and `Provisional` more
than triples in the same week. A stacked bar shows all of that at once and a table row would have
shown a number climbing.

`names_total` is every named thing, not the sum of the four columns, which is the one place this
deviates from `milestones()` and `decisions()` and it is deliberate. The four words are what a
`Name:` block *says*; a crate with no block has said nothing, and counting that silence as
`unrecorded` would put a claim in the record nobody made. The chart draws the difference as a band,
so the three pre-115 weeks show a tree with 119 named things and no provenance rather than three
empty bars, which this page's own rule would otherwise read as "no data".

The band found something the block did not predict. It survives two weeks past milestone 115 and
is exactly seven names, all of them Cargo packages: milestone 115 covered three surfaces and a
package was not one, the `package` kind closed the hole on 2026-08-18, and this series reproduces the
hole and its closing without being told about either.

And a bug, found by running the thing. `script/metrics --backfill` dropped the `coverage_lines_pct`
cell, which the script's own comment calls the only copy of that measurement there will ever be:
adding a column meant a rewrite, and a rewrite reset every row. Fixed in the same change, and the
evidence the columns were *added* rather than the series altered is that all 34 existing columns are
unchanged in all eight weeks and the seven existing SVGs are byte-identical.

The sixth column, and what it found. `proposals_unnumbered` is 74 at 2026W36 and zero in every
week before, because the directory was created on 2026-09-04. One bar is a number rather than a
series, so it gets no chart and a section of prose instead; the column exists so the series
accumulates. Its parse is shared the same way (`helpers/roadmap_proposals.py`, also provisional), and
`script/roadmap`'s output is byte-identical on every mode.

The premise that it is a one-way queue is false, and the checking is the useful part. Five
proposals have left the directory, so 86 have been written and 81 remain. One was promoted to a
number the ordinary way (milestone 256, x86_64 places PCI BARs in a hardcoded window, and on xenon
that window is RAM); the rest were done, by a lane that picked the file up and fixed the thing,
twice filing a narrower proposal in its place. So the column is a NET count over a gross flow, and a
flat line on it would be consistent with a stalled pile and equally consistent with one draining as
fast as it fills.

The caveat is on the page, in its own subsection, because the `BUGS` entry below was right that
it is the part most likely to be dropped. `notes/project-metrics.md` carries "A rising `Provisional`
band is not debt" with `AGENTS.md`'s worklist-not-a-wall quote under it, the reason a provisional
name is the mechanism working rather than failing, and the note that the band worth an alarm is
`Unrecorded`, which this chart has at zero.

## Follow-on

- **Done.** The current week's row is deliberately not written here. A lane's `HEAD` is not the
  first-parent trunk commit the file says stands for a week, so the CSV keeps the eight weeks it
  had, and `script/metrics --update` on trunk or `.github/workflows/metrics.yml` on Monday writes
  2026W37. Nothing is owed; this is recorded so the next reader does not file the missing row as a
  bug.
- **Recorded.** `helpers/name_provenance.py` is a provisional name and an architect names modules.
  It carries no `Name:` block because `script/names` puts `helpers/` out of its own scope, which is
  the same hole the `package` kind closed one surface over; its header paragraph is the record
  instead.
- **Recorded.** design/naming.md's `BUGS` said `kernel`, `xtask`, `redoxfs_server` and
  `tools/redoxfs_host` were uncovered surfaces. They have carried blocks since the `package` kind
  landed on 2026-08-18, and this milestone's own series is what showed the sentence was stale. The
  entry now says what closed it and when.

## Index row

calef, 2026-09-11. 101 of 205 names unratified and nothing shows whether that is climbing or
falling, where milestones get seven status columns and decisions six. Five naming columns and a
chart, plus proposals_unnumbered (scope added the same day). The series found seven packages
carrying no provenance for two weeks, and a --backfill that dropped the coverage cell.
