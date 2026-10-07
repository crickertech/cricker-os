---
status: BUILT
raised: 2026-09-20
built: 2026-09-20
---
# 518. A census that cannot be attributed is a number nobody can act on

*(Number provisional until the merge queue lands it.)* The record
exists, it is backfilled to the first census this project ever completed, and the first question it
was asked returned a finding: **the fall that `design/fatal-risks/README.md`'s risk 3 stands on did not
happen.** It is an artifact of two rows computed two different ways.

Built from artifacts GitHub still holds and from `.cargo/mutants-baseline.txt`; no mutation run was
needed and none was made, which is the point.

## Why

`design/fatal-risks/README.md`'s risk 3 (the tests do not test anything, and the quality is illusory) went
AMBER on 2026-09-19 on the strength of a rate that fell between two whole-corpus censuses, and had
to say in the same breath what it could not say:

> Which crates caused it is unknown, because the 2026-09-14 census's per-crate numbers were
> never written into the tree: the only per-crate record here is the August baseline, which is why
> the mistake above was available to make at all.

The mistake it refers to is milestone 512 (the census blamed one pull request for 55 survivors it did not write): `script/mutation --report`'s `(baseline missed)`
column is `.cargo/mutants-baseline.txt`, whose header reads *"Run of 2026-08-03"*, and a delta read
out of it is six weeks of growth wearing two days' clothes. Both failures are the same failure.
`script/mutation --report` prints a per-crate table from a finished run and then the run output goes
away, so the tree has kept one per-crate record in its life, and every comparison anybody wanted
to make had to be made against it whether it was the right comparand or not.

## What was built

- `script/mutation-census`, a new entry point. `--add-run <id>` captures a finished GitHub
  Actions census in one command, `--add-baseline` ingests `.cargo/mutants-baseline.txt`, `--list`
  and `--show` read the record back, and `--compare A B` answers the question nobody could answer on
  2026-09-19. **Name provisional**, per AGENTS.md; naming is an architect's.
- `notes/project-metrics/mutation-census.csv`, one row per crate per census:
  `census,run,crate,tested,caught,missed,timeout,unviable`. `tested` is the other four added up, so
  a truncated row fails its own arithmetic instead of looking plausible. The crate is the name the
  run recorded, never rewritten; resolution happens at comparison time.
- `notes/project-metrics/mutation-census-renames.csv`, derived by `--renames` from
  `git log --diff-filter=R` over `crates/*/src/lib.rs`, committed so a wrong pairing can be
  corrected by hand and stay corrected.
- [`notes/mutation-census.md`](../../notes/mutation-census.md), the page a reader meets: the
  series, its provenance, what is unrecoverable, and the arithmetic below.

**The comparison is an attribution, not a ranking.** `--compare` prints each crate's exact
contribution to the corpus-level move, `((k_b - k_a) - rate_a * (v_b - v_a)) / v_b` in points, and
the column sums to the move. So a crate that dragged the rate down by growing while holding its own
score is visible as such, which is the case `machine_discovery` turned out to be.

## What the record was backfilled with, and what is gone

Four censuses, and there have never been more than four:

| census | run | crates | mutants | viable | survivors | killed |
|---|---|---|---|---|---|---|
| 2026-08-03 | `.cargo/mutants-baseline.txt` | 38 | 5,551 | 5,141 | 391 | 92.4% |
| 2026-09-14 | [34833498873](https://github.com/nifeos/nife/actions/runs/34833498873) | 64 | 10,012 | 9,277 | 771 | 91.7% |
| 2026-09-16 | [35163453633](https://github.com/nifeos/nife/actions/runs/35163453633) | 62 | 9,626 | 8,903 | 687 | 92.3% |
| 2026-09-19 | [35421192143](https://github.com/nifeos/nife/actions/runs/35421192143) | 62 | 9,656 | 8,925 | 563 | 93.7% |

The baseline and the 2026-09-14 rows reproduce `notes/mutation-testing.md`'s own published totals to
the unit, which is the check that the ingest derives what a person derived by hand.

Nothing is unrecoverable, and that was luck rather than design. All eight shard artifacts of all
three green runs are still on GitHub and unexpired; the default retention is 90 days, so the same
backfill attempted in December would have recovered nothing. **Everything absent is absent because
it never existed**: the scheduled runs from 2026-08-10 to 2026-08-31 produced no result at all
(`mutation.yml`'s own `BUGS` has why), and 2026-09-03 produced one shard of eight, which is a sample
and is deliberately not a row, because a sample cannot tell an absent mutant from a killed one and a
per-crate row from one reads as a crate that got worse.

## The finding: the score did not fall

Risk 3's two headline rows were computed under two different definitions of `killed`, and the
second one also dropped a crate to a rename. Reproduced exactly from the artifacts:

| | crates | viable | as risk 3 reports it | recomputed |
|---|---|---|---|---|
| like-for-like, 2026-09-14 | 38 | 6,552 | 93.6% | 93.6% |
| like-for-like, 2026-09-19 | 37 | 6,472 | 92.6% | **94.7%**, over 38 crates and 6,604 viable |
| whole corpus, 2026-09-14 | 64 | 9,277 | 91.7% | 91.7% |
| whole corpus, 2026-09-19 | 62 | 8,925 | 91.4% | **93.7%** |

Two things separate the two columns, and each is worth about a point:

- A timeout is a kill, or it is not, and the two rows disagree. `notes/mutation-testing.md`
  defines it: *"`killed%` counts a timeout as a kill, because every one of the 96 was checked by
  hand and every one is a detected hang"*. The 2026-08-03 and 2026-09-14 figures follow that; the
  2026-09-19 figures are `caught / viable`, with the 205 timeouts scored as survivors. 8,157 of
  8,925 is 91.4%, which is the published number, and (8,157 + 205) of 8,925 is 93.7%.
- `cred` became `credentialer`, and the join dropped it. Reproducing 37 crates and 6,472 viable
  requires resolving `dtb` and `ipc` by hand (both renamed 2026-09-18) and missing `credentialer`,
  which is 103 viable mutants at 100%. That is the same trap one level down from milestone 512's:
  a comparison whose comparand is not what it claims.

The direction reverses under either definition read consistently. Whole corpus with timeouts as
kills: 91.7% to 93.7%. Whole corpus with timeouts as survivors: 89.5% to 91.4%. Survivors fell 771
to 563 in the same window. There is no reading of these artifacts in which the score went down.

This does not decide whether risk 3 is green, and this lane does not touch
`design/fatal-risks/README.md`. The entry's own better argument survives the arithmetic intact: milestone 85 (mutation testing over the host crates)'s rule
is that every survivor is triaged into a test, an exclusion with a reason or a recorded gap, and 563
of them are not. A verdict on a corrected rate is an architect's.

**And the honest caveat on the definition this block prefers.** The hand-check that justifies
counting a timeout as a kill was performed on the baseline's 96. There are 205 now and nobody has
looked at them. That is a reason to check them, not a reason to switch definitions between two rows
of one table.

## Where this leaves the instrument

`script/mutation --report`'s baseline column prints `new` for `credentialer` today, because
`.cargo/mutants-baseline.txt` was swept by the rename commits that renamed the crates and `cred` was
missed, the rename being to a different word rather than a longer spelling of the same one. This
lane does not rewrite that file: that is milestone 326 (turn a mutation score upward)'s part 4, held until its survivors are
triaged, because a baseline written first would enshrine the regressions as the new normal. The
record here makes that rewrite safer by giving it something to be checked against.

## BUGS

- Nothing gates capture. A census can still be run and thrown away, exactly as four of them
  were; this only makes it one command not to. A gate would have to know a run happened, and the
  thing that knows is the workflow, which does not write to the tree. See `## What wants a lane`.
- The baseline row's counts are derived, not observed. Its own file says so: the 2026-08-03 run
  was resumed twice, `caught` is total-minus-the-rest, and three of 5,551 mutants could not be
  matched to any pass. A comparison against it compares against a reconstruction.
- A deleted crate and a crate renamed to something git could not follow look identical, and both
  print as "only in A". `multicast_dns_protocol` and `multicast_dns_config` are that case between
  2026-09-14 and 2026-09-19.

## Follow-on

- **Milestone 532.** milestone 532 (the mutation census should write its own row),
  `design/roadmap/0532-the-census-writes-its-own-row.md`: have `mutation.yml`
  write the record rather than a person remembering to. It needs write permission on a workflow that
  has `contents: read` today, which is a decision rather than a patch.
- **Recorded.** The capture is rung three of AGENTS.md's ladder and nothing gates it, beside the
  feature in `script/mutation-census`' own `BUGS` section and in `notes/mutation-census.md`.
- **Recorded.** `script/mutation --report` prints `new` in the baseline column for `credentialer`,
  because `.cargo/mutants-baseline.txt` still spells it `cred`. Recorded in
  `notes/mutation-census.md`'s `BUGS`; the fix is milestone 326's part 4, held on purpose.
- **Milestone 512.** The correction this lane's finding owes `design/fatal-risks/README.md` belongs with
  the correction already proposed there, and both are an architect's to make; this lane edits
  neither.

## What wants a lane

- Have the workflow write the row. `mutation.yml`'s report job already aggregates every shard;
  a step that runs `script/mutation-census --add` and opens a pull request would make the record a
  by-product of the census rather than a thing somebody remembers. It needs write permission on a
  workflow that currently has `contents: read`, which is a decision rather than a patch.
- Triage the 205 timeouts, or stop counting them as kills. The definition this tree uses rests
  on a hand-check of 96 of them performed six weeks ago.

## Index row

The tree kept one per-crate mutation record in its life, so risk 3 could report that its score fell
and not say which crates caused it. `script/mutation-census` is that record: one row per crate per
census in `notes/project-metrics/mutation-census.csv`, backfilled to all four censuses that have
ever completed, with a `--compare` that attributes the corpus-level move to crates exactly. The
first question it was asked found that **the fall never happened**: risk 3's two rows count a
timeout two different ways and the second drops `credentialer` to a rename, and read consistently
the like-for-like rate went 93.6% to 94.7% while survivors fell 771 to 563.
