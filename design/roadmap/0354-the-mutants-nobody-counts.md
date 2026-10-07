---
status: SUPERSEDED
raised: 2026-09-03
superseded_by: 250
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 354. An unviable mutant is a hole in the measurement that reads as a pass

Superseded 2026-09-19, by milestone 250, which carries the same title and more of the
argument. Filed as a proposal on 2026-09-03 by the milestone 246 lane. Checked on 2026-09-19:
milestone 250 was minted by calef on the day this file was written, before
`design/roadmap/proposals/` existed, and its block holds every part of this one (the `Verdict`
worked example, the nine unviable mutants in `measured_boot`, the ledger that counts missed,
equivalent and hang and not unviable, the anti-gaming test) plus two things this file never had: the
`uefi_loader::image::parse` disposition of 2026-09-04, and the reverse defect of 154 mutants scored
MISSED in "0s build + 0s test" behind `required-features`. This file's own last paragraph said so.
Milestone 326, which triages the 2026-09-14 census's 771 survivors, is a different question and does
not cover this one: a survivor is a mutant that ran, and the subject here is a mutant that never
built.

`cargo mutants --list` already reports what this needs, and no run is required to
get the count.

What the work is. `cargo mutants` scores a mutant it cannot build as unviable, and nothing in
`notes/mutation-testing.md`'s ledger accounts for that category. The ledger has missed, equivalent and
hang. So a published rate cannot distinguish *"no test could kill this"* from *"the tool could not
build it"*, and only the first is a fact about the code under test.

Milestone 246 found the sharp end of it: the tool's only operator on a function returning a struct is
to replace the body with `Default::default()`. `measured_boot::verdict` returned a `Verdict` with no
`Default`, so the mutant did not compile, and the tool reported nothing at all about the one
decision in that crate saying whether unmeasured code may run. Deriving `Default` turned it from *1
unviable, 0 tested* into **1 caught**. `measured_boot` alone had nine unviable mutants.

Why it is worse than a missed mutant. A missed mutant is visible: it appears in the report, it
lowers the score, somebody argues about it. An unviable one leaves no trace in the number, so a
crate's score is silently computed over a smaller function than the reader believes.

The cheap version is most of it: `cargo mutants --list` per crate counting unviable (one command,
no run), then reading them to see whether a small honest change to the code makes the mutant both
buildable and meaningful, then putting the count in the ledger.

The hazard to name in whatever does this: making a mutant viable can be gaming. Deriving `Default`
where the default is meaningless adds a mutant any test kills and proves nothing. The test is whether
the default is a value the code could plausibly be wrong with; for `Verdict` it is both the fail-safe
value and exactly the dangerous wrong answer, an absence where there was a refusal.

**Superseded in part:** milestone 250 was minted from this on the day it was written, before the
`proposals/` directory existed. This file is the record of where it came from.

## Index row

Filed 2026-09-03 by the milestone 246 lane and superseded on arrival: calef minted milestone 250
from it the same day, under the same title, before the proposals directory existed. The work lives
there, with two worked examples this file never carried. Promoted and disposed of in one act by
milestone 433, because a proposal cannot be retired in place and a deleted one is not a record.
