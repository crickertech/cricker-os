---
status: PROPOSED
raised: 2026-10-04
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# A new confinement test carries a falsification record

Raised 2026-10-04 (UTC) by milestone 742 (every test is falsified as routine). Title and
slug are drafts.

**The gap, measured on 1a145fcaa.** A Kani harness must carry a `Falsification:` block, and
`script/falsifications --check` fails one that does not. A kernel `#[test_case]` need not, by
milestone 305 (the six kernel confinement rows get a falsification a machine can replay)'s deliberate choice: 28 of 323 carry one. A swish-check line need not either: 1 of
about 160 does. Nothing tells the author of a new confinement test that a record is owed, so whether
one is written depends on who remembers, which is rung zero. Milestone 305's own finding is the cost
of that: a RISC-V confinement test that could not fail, green on every gate for four weeks.

**Rate.** 147 `#[test_case]` lines were added in the 30 days before 2026-10-04 (gross, so moves count
twice; the 609 split moved many).

**The change, rung 2.** A ratchet in `script/falsifications --check`, keyed on the diff rather than
the tree: a `#[test_case]` added under `system_tests/src/user/` (where the confinement tests live)
must carry a block, which may be `unfalsified`. Existing tests stay opt-in, so 305's argument against
a wall of `unfalsified` still holds for them. A swish-check line added to `SWISH_CHECK_SCRIPT` waits
on the path question in
[a-swish-check-line-has-a-falsification-path.md](a-swish-check-line-has-a-falsification-path.md).

**What it does not cover, on purpose.** Host `#[test]`s (373) may carry a record since milestone 742, and this would not require
one. Mutation testing (`script/mutation`, and the inflow check in #1582) is the mechanism there,
and it asks a different question: whether some test kills a change to the code, where a falsification
asks whether one named test can fail at all. A host test that cannot fail is invisible to mutation
while a sibling kills the mutants. That limit stays recorded here rather than closed.

**Why not built in 742.** It reverses part of milestone 305's recorded choice, which is a policy
change rather than a missing mechanism, so it is offered rather than shipped. Reversible: it is one
check.
