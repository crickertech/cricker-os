---
status: BUILT
raised: 2026-08-03
built: 2026-08-03
---
# 79. Miri over the host crates

Raised 2026-08-03, from a survey of what analysis the tree runs against what
it could. Milestones 79 to 85 all come from that survey.

This project's method is pure logic in host-testable crates, and Miri interprets exactly those tests
while checking the rules nothing else here checks: aliasing, pointer provenance, uninitialized reads.
Kani proves the properties it is asked about; fuzzing sees crashes; clippy sees shapes. None of them
sees a `&mut` that aliases, and in a tree with 224 `unsafe` occurrences under `crates/` that class is
live. The pinned nightly already ships Miri as a rustup component, so the toolchain cost is one line
in `script/bootstrap`.

The work: a `script/undefined-behavior-check` front door delegating to `cargo xtask undefined-behavior-check`, which runs `cargo miri test`
over the host-testable crates. The first full run is most of the milestone: triage every finding,
fix what is real, and record what is not in the note this milestone writes.

## Scope note

Miri is an interpreter, roughly two orders of magnitude slower than native. The exhaustive suites
(`ntp_proto` runs its entire 10^9-value domain, `gpt` does 460,000 table validations) cannot run
under it as-is; the honest treatment is to exclude or sample them and say so, because "Miri-clean"
then means "the sampled paths are clean". Cadence is a weekly scheduled workflow plus on-demand,
not per-PR. `-Zmiri-strict-provenance` is a later ratchet to consider once the default run is clean.

## Follow-on

- **Recorded.** `notes/undefined-behavior.md` names each substitution in a table: "Miri-clean" means
  the sampled paths, because the exhaustive suites cannot run under an interpreter, so `ntp_proto`
  and `glob` run strided samples under a Miri configuration and one `glob` harness is skipped
  outright, since a sample that misses its argmax fails against correct code.
- **Recorded.** `notes/undefined-behavior.md` BUGS: strict provenance is not on, and for the user
  heap it never can be, because the allocator mints pointers into separately donated regions.
  Turning it on anywhere needs a per-crate carve-out.
- **Recorded.** `notes/undefined-behavior.md` BUGS: the Miri-only samples are hand-maintained twins
  of the native domains, and most carry no completeness pin, so a sample could shrink silently while
  the run stayed green.
- **Recorded.** `notes/undefined-behavior.md`: the board console crate is excluded from the run
  entirely, so the memory rules are unchecked there.
- **Refused.** Running Miri per pull request. It is an interpreter roughly two orders of magnitude
  slower than native, so the cadence is a weekly scheduled workflow plus on demand; paying that on
  every change would buy a check that already runs against the same tests once a week.

## Index row

The method is pure logic in host-testable crates, and Miri checks exactly those tests for the
undefined behavior Kani is not asked about and fuzzing cannot see. The pinned nightly already
ships it. Weekly, not per-PR, because the cost is runtime
