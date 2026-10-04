# A workflow records the run it was watched failing in

*Name provisional, as are `helpers/observed_failing.py`, the `Observed failing:` spelling and the
cutoff date. Milestone 640 (a gate is not evidence until somebody has watched it fail), number
provisional until the queue lands it.*

A new gate ships against a tree where its defect is absent, so its first result is green. Green is
also what it reports if it cannot fire at all, and from outside the two look the same. This tree has
paid for that twice. `falsifications.yml` replayed zero patches for three weeks and reported
success. `coe-architect-label.yml` applied no label on the first correction it saw and reported
pass. Neither had been run once on purpose before it was trusted. The account is
[the sweep that swept nothing](corrections/2026-09-23-the-sweep-that-swept-nothing.md).

DECISIONS §134 (a harness carries a machine-replayable falsification record, or it is not
evidence) already asks this of a Kani harness. This note asks it of a workflow, in a weaker form on
purpose: a cited run rather than a replayable patch. A workflow's inputs are pull requests, labels,
schedules and the state of GitHub, not a source tree, so a replayable version would cost more than it
buys.

## The rule

Every file in `.github/workflows/` carries a column-zero comment saying where somebody watched it
fail. `script/lint` holds it, through `helpers/observed_failing.py`.

```
# Observed failing: 2026-10-04, run 37226530740: red on #1617 because `needs-architect` was on it,
#   which is the hold this check exists to make.
```

"Failing" means the arm that carries the workflow's claim went off. For a gate that is a red job.
For a workflow that is green by design, it is the thing it does instead: a labeller's label, a
sweep's reported finding, the drain's `needs-maintainer`. The record says which, in its own words.

Two other shapes are accepted:

- `<date>, by script: ...` is for a workflow whose real failure cannot be staged. It names what was
  broken on purpose and what ran against it, and says why a real run was not possible.
- `never. <reason>` says nobody has watched it fail. It is accepted only for a workflow the tree
  already had on 2026-10-04, when the rule landed. The helper reads that age from git, following
  renames, rather than from the record. A new workflow cannot say `never`, and a clone too shallow
  to answer is refused.

The record lives in the workflow's own header, beside the reasoning already there. That is rung three
of the ladder: the next person to touch the file is already reading it. The gate that requires it is
rung two. Nothing keeps a list of which workflows have been watched; `--list` derives it.

## How to add a workflow

1. Write it and merge nothing yet.
2. Make it fail for the reason it exists. Dispatch it on a branch where the defect is present: a
   label put on, a record broken, a pin pointed at nothing. A scheduled workflow runs only from
   `main`, but `workflow_dispatch` runs the branch's copy.
3. Read the log, not the tick. Check the red step is the one carrying the claim, and that the reason
   it gives is the defect you staged. A red run that died on infrastructure is not the observation.
4. Put the run's id and date (UTC) in the header with one sentence on what it showed, then revert
   the staged defect.

## EXAMPLES

```console
$ helpers/observed_failing.py
observed-failing: 20 workflows, 15 watched failing, 5 never (each older than 2026-10-04)

$ helpers/observed_failing.py --list | head -3
architect-hold.yml               2026-10-04 run 37226530740
architect-label.yml              2026-10-04 run 37226530701
audit-cadence.yml                2026-09-28 run 36450379599

$ helpers/observed_failing.py --verify-runs | head -2
ok  architect-hold.yml               run 37226530740 2026-10-04 pull_request failure (.github/workflows/architect-hold.yml)
ok  architect-label.yml              run 37226530701 2026-10-04 pull_request success (.github/workflows/architect-label.yml)
```

`--verify-runs` needs `gh` and the network, so `script/lint` does not run it. It checks that each
cited run exists, belongs to the file citing it, and started on the date the record gives.

## The backfill, 2026-10-04

Each existing workflow's history was read with `gh run list --status failure`, then the failing
step's log for the newest candidate. A run counted only when the log showed the workflow's own reason.
For the workflows green by design, the label events and the sweep summaries were read instead.

Fifteen of twenty had such a run. Five had none, and say `never` with what would stage them:
`ci-failing.yml`, `metrics.yml`, `stick-maker-hosts.yml`, `stranger-cadence.yml` and
`vendor-watch.yml`. Two of those five have been red, for the wrong reason. `metrics.yml` went red
three days running on an unknown `--flags` argument. `ci-failing.yml`'s only run died on
`jq: Argument list too long` before judging anything; it landed the same day, and its record states
the defect.

None of the five was staged here. The milestone does not apply the rule backwards; it asks for the
honest record, and a staged run of each is somebody's lane.

## This gate's own observation

<!-- filled in by the lane once its CI run is read -->

## How this sits beside milestone 641

Milestone 641 (a mechanism that reports clean says over how many units, and zero is loud) came out
of the same correction and the same fifth why. This milestone's block says it may belong inside 641.
That is an architect's call and is not made here. The two touch different things. This one asks
whether a workflow has ever been seen to fire. 641 asks how many units each run examined. A gate can
pass either and fail the other.

## BUGS

- A cited run is checked only by `--verify-runs`, which nothing runs on a schedule. A mistyped run id
  passes `script/lint`.
- The record is per file, not per job or per arm. `ci.yml`'s record is about its `clippy` job, and
  `verify.yml`'s about its `re-falsify` leg. A failure arm added to an existing step inherits its
  file's record, though the milestone asks for a fresh observation then. Nothing here can tell a new
  arm from an edited line.
- The cutoff is a date. A workflow added on a branch cut before it and merged after can still say
  `never`, until those branches drain.
- Whether a red run was red for the right reason is a reading, stated in the record's prose. No check
  can tell a gate catching its defect from a runner falling over.
