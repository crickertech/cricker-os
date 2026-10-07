---
status: BUILT
raised: 2026-09-19
built: 2026-09-23
promoted_from: the-census-blamed-a-pull-request-that-added-four
---
# 512. The census blamed one pull request for 55 survivors it did not write

*(Number provisional until the merge queue lands it.)* Promoted from
the proposal `the-census-blamed-a-pull-request-that-added-four`, filed 2026-09-19, on calef's
instruction of 2026-09-20 to give every proposal on `main` a number. The paragraph below this one
is the proposal's own, unedited: the argument is its author's and promotion was not the moment to
improve it. Found by milestone 438 (would a diff-scoped mutation check have caught), whose first
measurement was a replay of the pull request `design/fatal-risks/README.md` names, and which found four
survivors where that record accounts for 55.

What this lane found on re-checking the proposal against the merged tree. The premise held
for the arithmetic but not for the target: between 2026-09-19 (when the proposal was filed) and
2026-09-23 (when it was built), risk 3 was rewritten twice more (milestones 517 and 518), and the
specific `machine_discovery` paragraph quoted below is no longer in `design/fatal-risks/README.md`; it
was folded into a broader correction about the two censuses' timeout and rename conventions,
which named the same shape of error ("a comparison across two records that were never made
comparable") without the crate, the numbers, or the arithmetic that closes it. So the work was not
a rewrite of a paragraph that no longer exists; it was restoring the specific case (with the exact
numbers) to the paragraph that replaced it, and doing the part nobody had done yet: recording the
trap where `script/mutation --report`'s own output and `notes/mutation-testing.md`'s scope section
meet a reader, so the next person reading that column does not have to rediscover this. See
`design/fatal-risks/README.md`'s risk 3, the paragraph dated 2026-09-23; `script/mutation`'s `report()`
comments; and `notes/mutation-testing.md`'s `Scope and honest caveats` section.

What the record says. `design/fatal-risks/README.md`'s risk 3, AMBER as of 2026-09-19: *"One crate
accounts for the fall and it was not one of the eight. `machine_discovery` went from 22 survivors to
77, at 86.2% ... It is the crate milestone 319 proved on 2026-09-17: the proofs landed, the
parsing around them did not get tests, and two days later the census found it."*

What is measured. A full sweep of the crate at `aa6a50b^1`, main immediately before milestone
319's pull request merged, finds 73 survivors already there:

```console
$ script/mutation -p machine_discovery
686 mutants tested in 7m: 73 missed, 533 caught, 71 unviable, 9 timeouts
```

The pull request added **seven** mutants to the crate and four survivors, and 73 plus 4 is the
census's 77 exactly. It did not write them.

Where the 22 comes from, as a reading rather than a measurement. The `(baseline missed)` column
`script/mutation --report` prints is `.cargo/mutants-baseline.txt`, where `machine_discovery` reads
`147 22 3 40`. That file is the **2026-08-03** baseline, so a rise from 22 to 77 is six weeks of a
crate growing from 212 mutants to 693, not two days of one pull request. The per-crate numbers from
the 2026-09-14 census are not in the tree, only its aggregates, which is what made the wrong column
the available one.

Why the correction matters more than the arithmetic. Risk 3's verdict is *"the tree adds untested
code faster than triage removes it"*, and the evidence offered for the rate is this attribution. Take
the attribution away and the same numbers support a different reading: the code accumulated over six
weeks and the instrument looked twice, once on 2026-09-14 and once on 2026-09-19. That is a
cadence problem, and the record already carries the fact that supports it (*"the workflow has
succeeded exactly once, so a cadence is claimed by one data point"*) in a different paragraph from
the one that draws the conclusion.

Risk 3 stays amber either way. What changes is what would turn it green, and milestone 438 refused a
diff-scoped gate partly on this: a mechanism aimed at a rate cannot be judged against a case that was
not a rate.

The work. Rewrite risk 3's `machine_discovery` paragraph against the measurement, say plainly
which census each number comes from, and record the trap: **`--report`'s baseline column is
2026-08-03 and is not the previous census.** The trap is the durable half. Every future reading of
that column by anyone will make the same mistake, and the fix is either a second column or a header
that says the date out loud.

## Follow-on

- **Recorded.** This lane did not build "a second column, or a header that says the date out
  loud", the mechanical fix the proposal named as the alternative to a comment. It instead
  recorded the trap where a reader already stands when they run `--report`: in `script/mutation`'s
  own `report()` comments, beside the `(baseline missed)` header, and in
  `notes/mutation-testing.md`'s `Scope and honest caveats` section (this file's `BUGS`-equivalent;
  milestone 326 (nobody has been assigned to turn a mutation score upward) already cites it as
  such). Rung three of AGENTS.md's ladder rather than rung two.
  If the column keeps getting misread despite the comment, the mechanical fix is still there to
  build.

## Index row

`design/fatal-risks/README.md` blamed milestone 319 (the crate that parses firmware)'s pull request for
55 of `machine_discovery`'s survivors it did not write: 73 already existed, from `script/mutation
--report`'s `(baseline missed)` column reading as a two-day delta when it is always a fixed diff
against the 2026-08-03 baseline. Risk 3's paragraph now carries the crate, the arithmetic and the
date; `script/mutation` and `notes/mutation-testing.md` now name the trap where a reader of that
column meets it.
