---
status: BUILT
raised: 2026-10-04
built: 2026-10-04
promoted_from: a-new-confinement-test-carries-a-falsification-record
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 749. A new confinement test carries a falsification record

Promoted from `design/roadmap/proposals/a-new-confinement-test-carries-a-falsification-record.md`
on 2026-10-04 (UTC). Raised by milestone 742 (every test is falsified as routine) and approved by
calef the same day. *(Number provisional until the merge queue lands it; title and slug
are drafts.)*

calef's ruling, 2026-10-04 (UTC): a `#[test_case]` newly added under `system_tests/src/user/` must
carry a `Falsification:` block, which may be `unfalsified`. Existing tests stay opt-in. This partly
reverses milestone 305 (the six kernel confinement rows get a falsification a machine can replay)'s
recorded choice that kernel tests are opt-in, and 305's block carries the correction.

## Built

`script/falsifications --check` runs the ratchet against `git merge-base HEAD origin/main`. Rung 2.

New is decided by function name, not by added lines. A test is new when its name carries
`#[test_case]` in no file of the merge base. Moves add lines without adding claims: 147
`#[test_case]` lines were added in the 30 days to 2026-10-04, mostly the system-tests split. A moved
test keeps its name and passes; a renamed one is new, which is right, because its name is the claim.

`script/falsifications --selftest` holds six fixtures and `script/lint` runs it before `--check`.
Shown failing both ways on 2026-10-04: a test appended to `authority_tests.rs` with no block failed
`--check` with one problem, and passed with `unfalsified`; and a deliberately path-keyed
implementation failed two fixtures (the new test and the moved test).

## BUGS

- A shallow clone has no merge base, so the ratchet prints that it was skipped. CI's lint job has
  history; a person on a shallow clone gets the line, not the check.
- A new test whose name matches any test in the base, anywhere, is not new. Two files may define the
  same function name; the second one escapes.
- Swish-check lines are outside it until §134 (a harness carries a machine-replayable falsification
  record) can name a line: proposal `a-swish-check-line-has-a-falsification-path`.

## Follow-on

- **Recorded.** The shallow-clone skip and the name collision, in this block's BUGS.

## Index row

A new confinement test (`system_tests/src/user/`) must carry a falsification record, `unfalsified`
allowed (calef, 2026-10-04). `script/falsifications --check` decides "new" by test name absent from
the merge base, so a moved test is not new; `--selftest` holds the fixtures.
