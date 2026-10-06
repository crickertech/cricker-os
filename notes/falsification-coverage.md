# Which tests can be shown to fail, by kind, and what replays the evidence

Milestone 742 (every test is falsified as routine), for fatal risk 3 (the tests do not
test anything). The note's name is provisional. Measured on `main` at 1a145fcaa, 2026-10-04 (UTC).
[falsification.md](falsification.md) has the convention and DECISIONS §134 (a harness carries a machine-replayable falsification record) the rule; this note is
the census and what the milestone changed.

## The census

| Kind | Count | Carry a record | Replayable | Replayed per pull request | Replayed on a schedule |
|---|---|---|---|---|---|
| Kani harnesses | 216 | 216 (required) | 160 | yes, those a diff can reach; gating since 742 | weekly |
| Kernel QEMU tests (`#[test_case]`) | 323 | 28 (opt-in) | 27 | no | weekly |
| swish-check lines | about 160 | 1 | 1, replayable since 742 | no | weekly, since 742 |
| Host unit tests (`#[test]`) | 373 | 4 since #1596 (opt-in) | 4 | yes, since 742 | weekly, since 742 |
| Lint checks (`script/lint`) | 69 sections | 22 fixture selftests | 22 | every lint run | every lint run |
| Confinement claims (notes/confinement-claims.md) | 31 rows | 29 falsified | rows 1 to 19 by Kani, 20 to 30 by kernel tests, 31 by swish-check | rows 1 to 19 only | all replayable rows |

Stale or vacuous now: none. The sweep of this branch's base (run 37167409725, 2026-10-04) replayed
187 records (160 Kani, 27 kernel tests) with 0 survivors, 0 stale and 0 without a verdict. The last
scheduled sweep, 2026-09-28,
swept 165 records and found 0 survivors and 1 stale (`port_holder_transmits_then_a_non_holder_faults`,
fixed 2026-10-03 by #1547's distinct-test count).

## What the census found

Every replay existed; what was missing was a consequence. Milestone 194 (the falsification record,
its lint, and the sweep that replays it) built both halves of the mechanism. Both reported into
nothing:

- The per-pull-request replay (`re-falsify the harnesses this change can reach`, verify.yml) was not
  a required check. Of the last 60 merged pull requests, 4 merged with it red: #1532, which dropped
  `timetable`'s `mod proofs;`, and #1535, #1538 and #1551, red because `main` was. All four were that
  one real defect; none was a flake.
- The weekly sweep ran with `continue-on-error`, so its job reported success whatever it found. The
  2026-09-28 run's stale record sat in a green run.

So the escapes that motivated this milestone split cleanly. The timetable one was a gate that fired
and was not required. The other two were kernel tests: the x86 port one (#1551), and the RISC-V one
found by milestone 305 (the six kernel confinement rows get a falsification a machine can replay).
Only a weekly sweep replays a kernel record; nothing does so per pull request.

## What 742 changed, and which rung each sits on

1. The per-pull-request replay is required (rung 2), folded into `verify (Kani proofs)` the way
   milestones 587 and 589 folded the kernel proof rows, so the ruleset did not change. Cost, over 10
   merge-group runs: it finished after both proof shards once, by about four minutes, and before them
   nine times.
2. The weekly sweep's run goes red on a finding (rung 2 for visibility, still not a gate): a
   survivor, a stale record or no verdict now fails the job after the summary publishes.
3. A swish-check record is swept (rung 2). `script/falsifications` reads a block above a function
   in `xtask/src/swish_check.rs`, boots `script/swish-check --arch <a>` with the patch applied, and
   calls it red only when swish-check's FAILED report names the patch's `Line:` (provisional). Row 31
   moves from replayed by hand to replayed weekly.
4. A host `#[test]` with a record is swept (rung 2) by `cargo test ... --exact`, red only when
   exactly one test ran and failed. It costs seconds, so it also rides the per-pull-request replay.
   #1596's four `paging` records were replayed this way, all red, 3.6 s for the four.

The branch's own sweep (run 37171273807) replayed 188 records with 0 survivors, 0 stale and 0
without a verdict. Row 31's swish-check record went red on aarch64 in 35.3 s, on the line it names:
`installed/unvouched` answered `domain: REACHED` and `slots held: 0 1 2 7`.

## How this differs from mutation testing

Mutation testing asks whether any test notices when the code changes. A falsification asks whether
one named test can fail at all, so a test that cannot fail hides from mutation whenever a sibling
kills the mutant.

The proof-side twin is the risk-2 reach study (`lane/kani-reach`, #1587), which mutates the code
under each Kani harness. Its seam with this note: that lane measures how many defects a proof
catches, and 742 only makes the one recorded defect per proof a gate. No proof mechanism was built
here.

## BUGS

- **A kernel or swish-check record is replayed weekly and by nothing else.** A pull request can
  break one and merge green; the next Monday run goes red. `script/falsifications`' header records
  why (a boot per record would put about eight minutes on every pull request touching `kernel/`).
- **Only one swish-check record can exist** until §134's path spells a line, which is an
  architect's call:
  [proposal](../design/roadmap/775-a-swish-check-line-has-a-falsification-path.md).
- Existing kernel tests owe no record. A new one under `system_tests/src/user/` does, since
  milestone 749 (a new confinement test carries a falsification record), ruled by calef 2026-10-04
  ([block](../design/roadmap/749-a-new-confinement-test-carries-a-falsification-record.md)).
- A record that cannot reach a verdict now blocks a merge. That is intended, and the 15-minute
  per-record limit keeps one hung record from costing the whole 45-minute job; a slow record is a
  finding about that record. No flake was seen in the 60 pull requests measured.
