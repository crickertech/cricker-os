# 3. The tests do not test anything, and the quality is illusory

*Risk 3 of [the nine](README.md). The status vocabulary, the rule an entry meets and the running order are there.*

The claim: AGENTS.md's principle 2 says the method works because of the gates, the proofs and the
review discipline. If the suite would not notice the code being wrong, that sentence is decoration.

**The experiment:** milestone 85 (mutation testing over the host crates), read as a census and
re-read against the baseline.

**Experiment status: RUN, 2026-09-19, re-read 2026-09-24.** MEASURED
rather than merely observed, and AMBER. calef ruled amber on the 2026-09-14 numbers; the fall behind
it did not happen. On 2026-09-21 the 38 baseline crates read 96.1% against
92.4% in August. The corpus reads 92.4%, or 93.6% without 132 mutants no host build
compiles ([`notes/mutation-testing.md`](../../notes/mutation-testing.md)).

It stays amber on the standard this entry holds: milestone 85's rule that every survivor becomes a
test, an exclusion carrying its reason, or a recorded gap. The 2026-09-21 census counted 771 missed
survivors; after #1277's 164 kills and triage, 414 are projected, measured at the next census
([triage](../../notes/mutation-testing/census-2026-09-21-triage.md)), and
milestone 326 (nobody has been assigned to turn a mutation score upward) owns the repair. Green is a ruled
condition rather than a number (calef, 2026-09-20): inflow, meaning the survivors a merged pull
request adds on its own lines are triaged.

Two caveats. This verdict speaks for the host-testable corpus and not for the kernel, where a census
is roughly 500 runner-hours against 52 minutes today. And one convention is load-bearing and
unchecked: whether a timeout counts as a kill moves this entry two points. That rule rests on a
hand-check of 96 timeouts in August; 206 stood on 2026-09-21.

Fact, 2026-10-04: milestone 745 (count the error paths no test reaches), a provisional number, found that no test executes 586 of the host crates' 1,150 Result-family error paths (51%). 490 of them are a `?` whose error side never ran. A further 1,159 are in the kernel and services, where no coverage run reaches ([untested error paths](../../notes/untested-error-paths.md)).

Fact, 2026-10-03: scheduled-workflow run
[37108924347](https://github.com/nifeos/nife/actions/runs/37108924347) (a dispatch, milestone 636 (the scheduled workflows are failing, and nothing says so)) is the
first complete census since 2026-09-21: 85 crates, 14,853 mutants, 13,734 viable, 1,004 missed, 255
timeouts, 92.7% killed against 92.4% on 2026-09-21.
[Appendix](the-mutation-verdict.md).

Fact, 2026-10-03: milestone 517 (what fraction of survivor growth arrives on lines a pull request touched)'s inflow measurement ran once and is not a weekly report; between the 2026-09-21 and 2026-10-03 censuses 600 new survivors sit on lines 58 merged pull requests wrote, 364 in crates with a triage ledger section and 236 in crates with none ([inflow](../../notes/mutation-testing/inflow-2026-10-03.md)).

Fact, 2026-10-04: the inflow check exists (milestone 740 (the survivors a merged pull request adds are
checked against a triage record), provisional). The weekly mutation workflow diffs each census
against the previous one, blames new survivors to the merged pull request that wrote the line, and
fails listing any with no row in `notes/project-metrics/mutation-triage.csv`. Against the 2026-09-21
census, 472 survivor keys are blamed to merged pull requests since: 162 have a triage row and 310
do not.

Fact, 2026-10-04: of the last 60 merged pull requests, 4 merged with the per-pull-request falsification replay red, because it was not a required check; milestone 742 (every test is falsified as routine) made it part of `verify (Kani proofs)` ([coverage](../../notes/falsification-coverage.md)).
