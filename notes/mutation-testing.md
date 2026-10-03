# Mutation testing: the baseline and the triage rule

Milestone 85 (mutation testing over the host crates). `script/mutation` runs
[cargo-mutants](https://mutants.rs/) over the host crates. It rewrites one function at a time
(replace a return value, delete a match arm, flip an operator), reruns the mutated package's tests,
and records whether anything noticed. Coverage asks whether a line ran under a test. A mutation run
asks whether any test would notice if the line were wrong, which is the property a test suite exists
for.

It is a report, not a gate. A new survivor fails nothing until the weekly numbers are stable enough
to deserve it.

This page is enough to run the tool, read its numbers and triage a survivor. The per-crate history,
and how each number got to where it is, lives in the [appendices](#appendices).

## Running it

```console
$ script/mutation -p calendar             # one package; minutes
$ script/mutation --list                  # enumerate mutants, run nothing
$ script/mutation --report [DIR...]       # per-crate table from finished run output
$ script/mutation-census --add-run <id>   # record a green weekly run in the tree
$ script/mutation-census --compare 2026-09-19 2026-09-21
```

A full local run takes hours. The weekly `mutation testing` workflow (`.github/workflows/mutation.yml`)
runs the same command eight-way sharded with `--sharding round-robin`, so each shard is a uniform
sample of every crate.

The tool version is pinned in `.cargo-mutants-version`, today 27.1.0. The pin matters more here than
for most tools: cargo-mutants changes which mutants it generates between versions, so an unpinned
tool moves the numbers with nothing in the tree changed. Exclusions live in `.cargo/mutants.toml`,
each with its reason. That file is config, not a code dependency, per §46 (thin primitives or whole
subsystems).

To ask where a crate's mutants sit before restructuring it for testability, run
`cargo mutants --list -p <crate>`. It attributes every mutant to its enclosing function.

## The triage rule

Every survivor becomes exactly one of three things, and nothing stays untriaged:

1. **A test worth writing.** The mutant found a property no test asserts; assert it.
2. **A recorded exclusion.** The function cannot be tested on the host, or the mutant is
   semantically equivalent to the original. It goes in `.cargo/mutants.toml` with the reason next
   to it. An exclusion without a reason is a hole, not a decision.
3. **An honest deferral, recorded here.** A real gap whose test is not worth its cost yet. It is
   named in this note, so the weekly number has a ledger behind it.

A kill counts only when the named test was seen to fail under the mutation. An equivalence claim
counts only when a second run still reports the mutant, or when it is an algebraic identity.
The baseline triage learned why: a verdict reached by reading is wrong about ten percent of the
time; a verdict reached by running is not ([baseline-ledger](mutation-testing/baseline-ledger.md)).

## How to read the numbers

`killed%` is caught plus timeouts over viable mutants.

**`killed%` counts a timeout as a kill**, because every one of the 96 was checked by hand and every
one is a detected hang. That check was made on the 2026-08-03 baseline. A timeout is almost always
a loop that never ends, such as `+=` becoming `-=` on a walker's cursor.

It is over viable mutants, so `unviable` is excluded from the denominator: an unviable mutant does
not compile, which says nothing about the tests. It is still a hole in the measurement. A function
whose only mutant is unviable was never tested at all, and scores as though it passed
([measured-boot](mutation-testing/measured-boot.md)).

"Survivors" means missed plus timeouts. That is what `script/mutation --report` lists as "the
survivors themselves".

A census is a whole-corpus run. The weekly run's eight shards together make one; a single shard is
a sample. A sample cannot tell an absent mutant from a killed one, so a per-crate rate from one
reads as a crate that got worse. A whole run is different: it is a census rather than a sample.

## The current numbers

The latest census is the scheduled run of 2026-09-21,
[35589550926](https://github.com/nifeos/nife/actions/runs/35589550926), recorded in the tree
on 2026-09-24:

| census | crates | mutants | viable | survivors | killed |
|---|---|---|---|---|---|
| 2026-08-03, the baseline | 38 | 5,551 | 5,141 | 391 | 92.4% |
| 2026-09-14 | 64 | 10,012 | 9,277 | 771 | 91.7% |
| 2026-09-16 | 62 | 9,626 | 8,903 | 687 | 92.3% |
| 2026-09-19 | 62 | 9,656 | 8,925 | 563 | 93.7% |
| **2026-09-21** | **66** | **10,988** | **10,178** | **771** | **92.4%** |

Here "survivors" is the missed count; 2026-09-21 also has 206 timeouts. The per-crate rows are in
`notes/project-metrics/mutation-census.csv`, and [`notes/mutation-census.md`](mutation-census.md)
explains them.

Like-for-like, the score has risen since the baseline. Read consistently, the 38 baseline crates
were 94.7% on 2026-09-19 against 92.4% in August. Over the 62 crates shared with 2026-09-19, the
2026-09-21 rate rose again, 93.7% to 93.8%.

The corpus number fell on 2026-09-21 because of what arrived, not what decayed. Four crates were
measured for the first time and one grew eightfold:

| crate | viable | killed | missed | since 2026-09-19 |
|---|---|---|---|---|
| `uefi_loader` | 356 | 48.9% | 182 | 34 viable at 100%; 132 missed were unbuilt files |
| `stick_maker` | 288 | 64.9% | 101 | new |
| `portable_executable` | 277 | 72.6% | 76 | new |
| `firmware_configuration` | 32 | 84.4% | 5 | new |
| `sealed_pair` | 63 | 95.2% | 3 | new |

That worklist is done (2026-09-24, [census-2026-09-21-triage](mutation-testing/census-2026-09-21-triage.md)).
The same appendix sorts all 771 by kind. 193 were files no host build compiles or can run, 286 are
equivalents or recorded gaps, and 292 were missing tests, of which 203 are now written and **89 are
untriaged**. `stick_maker` read 64.9% chiefly because its host half was mutated at all; it is now
excluded, as `script/coverage` already excluded it. Projected onto the census's counts, the corpus
reads **95.9%** with 414 missed and like-for-like **96.3%**; the next census is the measurement.

Read `uefi_loader`'s row with care: most of its drop was the
[2026-09-04 trap](mutation-testing/uefi-loader.md) again, confirmed on 2026-09-24. The exclusion
covered `uefi_loader/src/main.rs` but not the modules it declares, `src/arch/` and `src/chooser.rs`,
which no host build compiles either. The census artifacts put 132 of the crate's 182 missed in
`src/arch/`. Without them the crate reads 77.7% (174 of 224 viable), with 50 real survivors in
`device_tree_patch.rs`, `handoff.rs` and `image.rs`, and the corpus reads 93.6%. Both files are now
excluded, and `script/lint` walks a gated target's whole module tree, so the next one fails the gate.

## How it got here

The baseline was one local run on 2026-08-03, and it is the fixed point
`.cargo/mutants-baseline.txt` records ([baseline-2026-08-03](mutation-testing/baseline-2026-08-03.md)).
Every one of its 391 missed and 96 timeouts was triaged, crate by crate, and none was deferred
([abi to ipc](mutation-testing/baseline-survivors-abi-to-ipc.md),
[grant_plan to swish](mutation-testing/baseline-survivors-grant-plan-to-swish.md),
[the ledger](mutation-testing/baseline-ledger.md)).

The weekly workflow then failed every scheduled run until 2026-09-14, and nobody noticed until
milestone 238 (two scheduled checks have never once succeeded) looked. One shard of eight finished
on 2026-09-03 and read as a fall from 92.4% to 85.3%. The first full census, on 2026-09-14, showed
the fall was an artifact ([the-first-weekly-censuses](mutation-testing/the-first-weekly-censuses.md)).
Three crates had been scored against suites that could not reach them, and a single mutant could
exhaust a runner's memory:

- `system_initializer` could not compile on the host at all; milestone 244 (the largest crate in
  the tree is proved by nothing a mutation can reach) excluded it.
- `uefi_loader`'s binary half is a file no host build compiles
  ([uefi-loader](mutation-testing/uefi-loader.md)).
- `documentation` had a third of its tests behind a default-off feature
  ([documentation](mutation-testing/documentation.md)).
- Milestone 277 (bound what one mutant may allocate) gave each mutant a memory limit, which is what
  let a whole run finish ([a-memory-bound-per-mutant](mutation-testing/a-memory-bound-per-mutant.md)).

Milestone 326 (turn a mutation score upward) then triaged the 2026-09-14 census. Parts 1 and 2 took
the crates that regressed from the baseline
([capability to dtb](mutation-testing/regressions-capability-to-dtb.md),
[clock_protocol, swish, filesystem_protocol](mutation-testing/regressions-clock-protocol-swish-filesystem-protocol.md),
[machine_discovery](mutation-testing/machine-discovery.md)). Part 3 took the worst of the crates
new since the baseline ([new-crate-backlog](mutation-testing/new-crate-backlog.md) and the four
appendices after it). Most survivors were missing tests, not bugs. The exceptions are real
wrong-accepts, and `gpt`'s at the baseline is the one to read first: it accepted a table whose last
usable block sat inside the backup array.

## The ledger

Every triaged survivor lands in one column: killed (a named test fails under it), equivalent,
hang (a timeout confirmed to be a loop that never ends), excluded (with its reason in
`.cargo/mutants.toml`), or recorded gap. The baseline's 487 survivors split 225 killed, 170
equivalent, 95 hangs, and none deferred ([baseline-ledger](mutation-testing/baseline-ledger.md)).
Milestone 326's crates each carry the same split at the head of their own section, in the
appendices from 2026-09-19 on. A recorded gap is written beside the crate's accounting, with what
would close it.

The ledger has no column for unviable mutants, and it should. Milestone 250 (an unviable mutant is
a hole in the measurement that reads as a pass) is that work, not started.

## What the tool has taught

These recur across the appendices, so each is stated once here.

- An exhaustive sweep proves *rejection*, not *rejection for the right reason*. A mutant that
  weakens check B survives any input that also trips check A. `gpt`'s real bug hid this way
  ([baseline-2026-08-03](mutation-testing/baseline-2026-08-03.md)).
- A fixture that never meets the boundary cannot tell `>` from `>=`. Test at the exact limit.
- A wire format written as shifted constants needs one test pinning the exact values.
  `1 << 0` becoming `1 >> 0` is the one equivalent case.
- A release-mode guard behind a `debug_assert!` of the same condition is unobservable under
  `cargo test`. Those mutants are recorded as equivalent under the harness, not excluded, so they
  stay visible.
- Memory-ordering mutants change nothing a single-threaded test can see. Concurrency claims are
  argued in comments or proved. Loom models are excluded from the count (`interleavings::` in
  `.cargo/mutants.toml`), because a mutant there tests the model, not the crate.
- A mutant that breaks a loop that waits produces a hang, not a wrong answer. Where the crate
  allows it, run the test body on a worker with a deadline, and the hang becomes a named assertion
  failure. Better still, delete the loop: a `for` over a slice has no increment to lose
  ([new-crate-backlog](mutation-testing/new-crate-backlog.md)).
- A score can measure code no test build compiles. A missed mutant in "0s build + 0s test" is a
  file nothing rebuilt, not a gap in the tests.

## Scope and honest caveats

This is the note's `BUGS` section.

- **Scope is the main workspace's host crates.** The exclusions and their reasons are in
  `.cargo/mutants.toml`. The bare-metal crates cannot compile for the host. `supervision_protocol`,
  `swap_protocol` and `virtio` compile but cannot run a line without a kernel. `xtask` is the build
  system, whose tests are the gates it runs.
- `redoxfs_server` and `tools/redoxfs_host` are not mutated. Each is its own workspace, kept apart
  so upstream RedoxFS never meets our clippy and fmt gates, and cargo-mutants works one workspace
  at a time. `redoxfs_server`'s pure logic is small and host-tested. The heavy half of its suite
  runs under QEMU (`script/test`'s redoxfs leg), so a host-only score would overstate the gap.
  Deferred, on the record.
- A survivor count is not a quality score across crates. Crates differ in how much of their surface
  a host test can assert. Compare a crate to its own last census, not to its neighbours.
- `script/mutation --report`'s `(baseline missed)` column is not "last week". It is
  `.cargo/mutants-baseline.txt`, one fixed run from 2026-08-03. Milestone 512 (the census blamed one
  pull request for 55 survivors it did not write) recorded this on 2026-09-23, and the trap is also
  named in `script/mutation` beside the column. Read as a recent delta, it turns six weeks of growth
  into whatever days sit between the reader and the crate's last change. That happened once.
  `design/fatal-risks/README.md`'s risk 3 blamed milestone 319 (the crate that parses firmware)'s pull
  request for `machine_discovery` going from 22 survivors to 77. Replaying `cargo mutants --in-diff`
  against that pull request found 4; the crate already carried 73 before it merged. Compare
  censuses with `script/mutation-census --compare` instead, which milestone 518 (a census that
  cannot be attributed) built for this.
- Timeouts are auto-derived by cargo-mutants from each package's own build and test time. A mutant
  that makes a loop spin forever is recorded as `timeout`. The baseline's timeouts were checked and
  are detected hangs, mostly cursor arithmetic in walkers. A timeout on a mutant that could not
  hang would be triaged as a survivor.
- A mutant that hangs is not a mutant that survived, and this instrument cannot say so. The only
  limits cargo-mutants 27.1.0 offers are clocks: `--timeout`, `--build-timeout`, their two
  multipliers and `--minimum-test-timeout`, as milestone 277 checked. So a deadlocked suite and a
  slow one produce the same `TIMEOUT` row, and `--report` lists both among the survivors. Nine
  survivors across milestone 326's two lanes were non-terminating rather than wrong, and each
  had to be argued in prose. What would close it is a rule in `--report` comparing a timeout with
  the package's baseline test time: orders of magnitude over is a deadlock, twice over is a slow
  test. Until then, read every `timeout` row as unclassified, and expect the triage to say which.
- A deadlock in one test hides an assertion failure in another. The classification is per run: if
  any test in the binary hangs, the mutant is a `TIMEOUT` however loudly the others failed.
  Milestone 326 met this twice. In `memory_corruption_canary_gate`, deleting `ArmGuard`'s `Drop`
  fails a named assertion and hangs a sibling test. In `jh7110_entropy`, `Pool`'s own doctest calls
  `take` directly, so no change to the test module can move the classification. Hand-applying the
  mutant left `cargo test --doc` at "has been running for over 60 seconds". Bounding one blocking
  call proves the property but does not move the number. Bounding all of them works only where no
  doctest blocks.
- Nothing gates capturing a census. The 2026-09-21 run went green and sat unrecorded for three days
  until this page needed its number. GitHub keeps the artifacts for 90 days.
- The 205 timeouts of 2026-09-19 and the 206 of 2026-09-21 have not been hand-checked the way the
  baseline's 96 were. `killed%` still counts them as kills, on that older evidence.

### Closed on 2026-09-24: the one survivor nobody had triaged

`compositor`'s `replace * with + in Rect::area` was the single genuine decay on a line nobody
edited. Milestone 517 (what fraction of survivor growth arrives on lines a pull request touched)
found it among 142 candidates; the other 141 were already survivors in August. That lane could not
hold this file, so the survivor was recorded here as pending. Every fixture rectangle was square,
and `2 * 2 == 2 + 2`. The test `area_multiplies_width_by_height` now uses a 3 by 5 rectangle, and
fails under the hand-applied mutant.

## Appendices

Each holds the dated entries named in its row, so a citation of "notes/mutation-testing.md, the
2026-09-20 section" resolves here. The appendix stems are provisional names; naming is an
architect's.

| appendix | what it verifies | holds |
|---|---|---|
| [baseline-2026-08-03](mutation-testing/baseline-2026-08-03.md) | the fixed point, and how its counts were derived | the baseline; calibration against the exhaustive crates; recurring patterns; `gpt`'s bug |
| [baseline-survivors-abi-to-ipc](mutation-testing/baseline-survivors-abi-to-ipc.md) | that every baseline survivor was triaged | the hand-triaged crates, first half |
| [baseline-survivors-grant-plan-to-swish](mutation-testing/baseline-survivors-grant-plan-to-swish.md) | the same | the hand-triaged crates, second half |
| [baseline-ledger](mutation-testing/baseline-ledger.md) | 487 survivors: 225 killed, 170 equivalent, 95 hangs, none deferred | the ledger; the alarming survivors |
| [the-first-weekly-censuses](mutation-testing/the-first-weekly-censuses.md) | that the fall to 85.3% was an artifact | 2026-09-03 (the weekly report had never run); 2026-09-14 (the first census) |
| [measured-boot](mutation-testing/measured-boot.md) | five equivalences, proved | 2026-09-03 (`measured_boot` re-run) |
| [uefi-loader](mutation-testing/uefi-loader.md) | a file nothing compiles scores 0% | 2026-09-04 |
| [a-memory-bound-per-mutant](mutation-testing/a-memory-bound-per-mutant.md) | why a whole run can finish | 2026-09-12 |
| [documentation](mutation-testing/documentation.md) | the feature-gated third of a suite | 2026-09-13 |
| [regressions-capability-to-dtb](mutation-testing/regressions-capability-to-dtb.md) | milestone 326 parts 1 and 2 | 2026-09-19: `capability`, `memory_regions`, `elf`, `timetable`, `dtb` |
| [regressions-clock-protocol-swish-filesystem-protocol](mutation-testing/regressions-clock-protocol-swish-filesystem-protocol.md) | the same | 2026-09-19: `clock_protocol`, `swish`, `filesystem_protocol` |
| [machine-discovery](mutation-testing/machine-discovery.md) | 77 survivors: 58 killed, 11 equivalent, 8 gaps | 2026-09-19 `### machine_discovery`; 2026-09-20 re-derivation |
| [new-crate-backlog](mutation-testing/new-crate-backlog.md) | milestone 326 part 3, and a loop that waits | 2026-09-20: `work_steal_slot`, `memory_corruption_canary_gate`, `soak_page`, `schedule_store` |
| [job-mix-and-jh7110-entropy](mutation-testing/job-mix-and-jh7110-entropy.md) | the same | 2026-09-20: `job_mix`, `jh7110_entropy` |
| [board-console](mutation-testing/board-console.md) | the same | 2026-09-20: `board_console` |
| [video-terminal](mutation-testing/video-terminal.md) | the same | 2026-09-20: `video_terminal` |
| [clock-and-reset-nvme-screen-console](mutation-testing/clock-and-reset-nvme-screen-console.md) | the same, and what part 3 left | 2026-09-21: `jh7110_clock_and_reset`, `non_volatile_memory_express`, `screen_console` |
| [census-2026-09-21-triage](mutation-testing/census-2026-09-21-triage.md) | the 2026-09-21 census's 771, classified; 164 killed | 2026-09-24: `portable_executable`, `stick_maker`, `uefi_loader`, `paging` |
