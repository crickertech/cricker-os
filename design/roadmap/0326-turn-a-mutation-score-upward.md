---
status: PARTIAL
raised: 2026-09-19
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 326. Nobody has been assigned to turn a mutation score upward

Minted 2026-09-19 by calef, from the gap his own fatal-risk-3 ruling named
the same day. *(Number provisional until the merge queue lands it.)* Parts 1 and 2 are done, on
`milestone/326-mutation-survivor-triage`. Part 3's seven named crates are done (2026-09-20, on
`milestone/326-new-crate-backlog`), and the 2026-09-21 census is classified with 89 survivors left
(`## The 2026-09-21 census` below). Part 4 is
untouched and must stay last for the reason its own paragraph gives.

Everything this needs exists: `script/mutation -p <crate>` runs one package,
`.cargo/mutants.toml` is where an exclusion goes with its reason, and milestone 85 (mutation testing over the host crates) already set the
rule a survivor is triaged under. No decision is owed and no hardware is involved.

## Parts 1 and 2, 2026-09-19

169 survivors triaged, and none of the eight crates carries an untriaged one. Measured per crate
with `script/mutation -p <crate>` on the lane's own worktree, before and after. Every kill was
verified by re-running the sweep and watching the mutant die; every equivalence claim is a mutant
the second run still reports. The reasons are in the `## 2026-09-19` sections of
`notes/mutation-testing/regressions-capability-to-dtb.md` and
`notes/mutation-testing/regressions-clock-protocol-swish-filesystem-protocol.md`, crate by crate,
which is where a reader should go to disagree with one.

| crate | before | after | killed by a test | equivalent | excluded | recorded gap |
|---|---|---|---|---|---|---|
| `capability` | 8 | 3 | 5 | 3 | 0 | 0 |
| `memory_regions` | 8 | 0 | 6 | 0 | 2 | 0 |
| `elf` | 6 | 0 | 6 | 0 | 0 | 0 |
| `timetable` | 48 | 0 | 12 | 0 | 36 | 0 |
| `dtb` | 14 | 1 | 13 | 1 | 0 | 0 |
| `clock_protocol` | 6 | 3 | 0 | 3 | 3 | 0 |
| `swish` | 20 | 4 | 16 | 4 | 0 | 0 |
| `filesystem_protocol` | 59 | 40 | 19 | 38 | 0 | 2 |
| total | 169 | 51 | 77 | 49 | 41 | 2 |

`after` is what a re-run reports now: the equivalents plus the two recorded gaps. Excluded mutants are no longer generated, so they leave the column.

What moved the numbers, in order of how much a reader should care.

1. Forty-one of the 169 were never the crate's code. `timetable` keeps its Kani harnesses in
   `src/proofs.rs` rather than in an inline `mod proofs`, and cargo-mutants names a mutant by its
   path *within the parsed file*, so `.cargo/mutants.toml`'s `proofs::` regex matched nothing: 36 of
   that crate's 48 survivors were its own proofs, which is why it read 73.6%. The five `mod
   interleavings` loom models were unexcluded for the same reason one level over. Both are closed
   with globs, and both are the `system_initializer` (milestone 244) and `uefi_loader` (milestone
   280) arithmetic wearing new clothes.
2. Seventy-seven were real, and four of them are places a probe reports nothing.
   `capability::survey_includes`, the predicate deciding which threads a supervision rendezvous may
   see, had no `cargo test` caller at all: a constant `true` and a constant `false` both survived.
   `filesystem_protocol::fixture::twodir` had no distinctness test, so five witness bits could each
   become zero while their boot passed. `memory_regions::has_children` is what the kernel asks before
   it offers to destroy, and every test proved the refusal through a different door.
3. Forty-nine are equivalent and that is a result, not a shortfall. They are argued by group in
   the note so a later pass can re-check a group rather than a mutant, and the two crates whose
   numbers barely moved (`clock_protocol`, `filesystem_protocol`) are the two where that is the
   honest answer.

And the one thing this part was told to look for, which it did not find. The block ordered
`timetable` second because it holds `next_after`, the property `design/fatal-risks/README.md`'s risk 2 calls
its strongest counterfactual. No survivor touched `next_after`, or the phase arithmetic, or the
firing decision; every mutant in them was caught before this lane changed anything. That is evidence
against risk 2 in the place this roadmap thought it most likely, and it is worth as much as a finding
would have been.

One behavior question is recorded rather than answered, in a `BUGS` section on
`timetable::Unbacked` where a reader meets the variants: `Admission::Unbacked` can never carry
`File` or `Directory`, because `admit` hands `grant_plan::plan` the same `dir` bit that `unbacked`
then re-tests, so a designation the scheduler cannot back is refused during planning instead. Two
mutants deleting the `!` in those tests survived on exactly that. Changing which of two true
sentences a reader meets is a behavior change, so it is written down and not made.

Why it exists, and the shape is worth naming. Four milestones touch mutation testing and
three of them are repairs to the instrument: 85 built `script/mutation` and triaged the
baseline's 391 survivors, 238 repaired the workflow's shard indices after it had never once
succeeded, 277 bounded the runaway mutant that took seven of eight shards every week, and 280
explained the two crates whose broken suites made a fall look real. Only 85 ever turned a score, once,
in August. So the tree has an excellent instrument, a working cadence, and nobody pointed at the
number it produces.

That is what holds `design/fatal-risks/README.md`'s risk 3 at AMBER rather than green, on calef's
reading of 2026-09-19. Milestone 85's rule is that every survivor becomes a test, an exclusion with
a reason, or a recorded gap. It held for the baseline's 391. The census of 2026-09-14 produced
771, and none of them has been looked at.

## Part 3's head, 2026-09-20

The seven crates this block names are triaged, and none carries an untriaged survivor. Measured
per crate with `script/mutation -p <crate>` on the lane's own worktree, before and after; every kill
was verified by re-running the sweep and watching the mutant die, and every equivalence claim is a
mutant the second run still reports. The reasons are in `notes/mutation-testing.md`'s
`## 2026-09-20` section, crate by crate, which is where a reader should go to disagree with one.

| crate | before | after | killed by a test | equivalent | excluded | recorded gap |
|---|---|---|---|---|---|---|
| `work_steal_slot` | 0 | 0 | 0 | 0 | 0 | 0 |
| `memory_corruption_canary_gate` | 8 | 2 | 6 | 2 | 1 | 0 |
| `soak_page` | 7 | 0 | 7 | 0 | 0 | 0 |
| `jh7110_entropy` | 23 | 10 | 13 | 6 | 0 | 4 |
| `multicast_dns_protocol` | n/a | n/a | n/a | n/a | n/a | n/a |
| `job_mix` | 10 | 4 | 6 | 3 | 0 | 1 |
| `schedule_store` | 8 | 0 | 8 | 0 | 0 | 0 |
| total | 56 | 16 | 40 | 11 | 1 | 5 |

`before` and `after` count missed plus timeouts, which is what `script/mutation --report` lists as
the survivors themselves.

Two of the seven numbers in the table below were artifacts, and this block predicted one of them.
`work_steal_slot`'s 54.2% was its loom model, which part 1's `interleavings::` entry removes: it now
reports 14 mutants, 13 caught, 1 unviable and nothing owed. `multicast_dns_protocol` (the census's
`mdns_proto`, renamed by milestone 265 (`_proto` is a truncation)) is not in this tree at all:
milestone 298 (retire the multicast DNS responder and its two crates) retired it and its sibling on
2026-09-15, the day after the census measured them,
so its 82 survivors close by deletion. The other flagged crate went the opposite way:
`memory_corruption_canary_gate` was genuinely 50.0% once its loom mutants left the count, worse
than the 66.7% it was flagged at.

Eight of the fifty-six were deadlocks rather than wrong answers. A loop that waits is broken by
making it never accept; a loop that gathers is broken by making it never advance. Neither returns,
so the suite's answer is to hang, and cargo-mutants can only call a suite that did not finish a
timeout. This block's own posture is that such a timeout is the tests noticing rather than missing,
and that stands. What part 3 adds is that noticing by hanging is worth converting into noticing by
*failing* where the crate allows it: `memory_corruption_canary_gate` now runs each test body on a
worker with a deadline (four converted), `job_mix`'s hand-rolled index became a `for` (one removed
at rung one), and `jh7110_entropy`'s four were left because `Pool`'s doctest calls `take` directly,
a hang confirmed by hand-applying the mutant. (Corrected 2026-09-26, UTC: this said a doctest has
nowhere to put a deadline. #1323 disproved it, with hidden `# ` lines that `recv_timeout` a worker.)

Nothing found here is a defect in shipped behavior, which is the answer to the question this
milestone exists to ask. The closest are three places where a plausible-looking mistake was
unguarded: `soak_page`'s offsets could collapse two workers onto one word with every test green,
`schedule_store`'s buffer bound could index one past a caller's array, and `job_mix`'s budget could
size a task nine pages below what `MAP` needs while a compile-time assertion beside it still passed.
All three are now checked. One real limitation was found and recorded rather than fixed:
`job_mix::order(2k)` and `order(2k + 1)` are the same permutation, in a `BUGS` section on the
function, unreachable because the kernel hands out odd seeds.

`.cargo/mutants.toml` gains one entry, `tests::`, and it is measured rather than assumed:
cargo-mutants skips a plain `#[cfg(test)]` module but not the `#[cfg(all(test, not(loom)))]` the
five loom crates have to write, so a helper in one of those is mutated where the identical helper
elsewhere is not.

## The 2026-09-21 census, 2026-09-24

`notes/mutation-testing/census-2026-09-21-triage.md` classifies all 771: 193 were files no host
build runs, 184 were in the four crates taken here (164 killed, 20 equivalent), 305 were already
triaged, and 89 are not.

One wrong-accept was fixed: `portable_executable` bounded a relocation by `memsz`, not `filesz`.
Projected, not measured: corpus 92.4% to 95.9%, like-for-like 96.1% to 96.3%. The 89, and
`device_tree_from_acpi.rs`'s 14 since, are
`design/roadmap/proposals/triage-the-crates-the-2026-09-21-census-measured-first.md`.

## The 2026-10-03 inflow, 2026-10-05

Three lanes (`lane/326-survivors`, `-2`, `-3`) gave all 310 untriaged inflow keys a row in
`notes/project-metrics/mutation-triage.csv`: 101 killed by new tests, 170 already killed, 36
equivalent, 3 recorded gaps. The first batch took the projected count from 414 to about 334.
Detail: `notes/mutation-testing/inflow-2026-10-03.md`.

## What the work is, in priority order, and the order is the argument

1. The seven regressions, because each is a property that used to hold. This is the only part
   that is not ordinary backlog: a crate that never had coverage is a gap, while a crate that scored
   100% and now does not has lost something it had. A census is the first instrument in this project
   able to tell those apart, because a sample cannot distinguish an absent mutant from a killed one.

   | crate | baseline | 2026-09-14 | missed |
   |---|---|---|---|
   | `memory_regions` | 100.0% | 88.9% | 0 to 8 |
   | `elf` | 100.0% | 94.2% | 0 to 6 |
   | `capability` | 97.4% | 88.2% | 1 to 8 |
   | `dtb` | 99.7% | 96.6% | 1 to 14 |
   | `clock_protocol` | 96.8% | 91.0% | 2 to 6 |
   | `swish` | 94.6% | 89.4% | 3 to 20 |
   | `filesystem_protocol` | 93.1% | 90.1% | 37 to 59 |

   `capability` is first inside this list, because it is the crate the capability core's proofs
   are about, and a survivor there is a place the confinement argument could be wrong with every
   test green. `memory_regions` and `elf` are next, as the two that fell from a perfect score.

2. `timetable`'s 48 survivors, and not because of its rate. At 73.6% it is not the tree's worst
   crate, but it holds `next_after`, the property `design/fatal-risks/README.md`'s risk 2 names as its
   strongest counterfactual: the milestone 6 timer drift, proved in this tree over code the timer
   does not call. 48 survivors in a crate carrying a proof is the exact shape risk 2 is about, and
   it is the one place on this list where a survivor might be evidence for a fatal risk rather than
   a missing test.

3. The rest of the new-crate backlog, as backlog. The 1.9-point gap between the like-for-like
   93.6% and the corpus 91.7% is the 26 crates that did not exist at baseline, and the eight worst
   crates in the tree are all of them new (`work_steal_slot` 54.2%, `memory_corruption_canary_gate`
   66.7%, `soak_page` 68.2%, `jh7110_entropy` 76.8%, `mdns_proto` 77.3% with 82 survivors, `job_mix`
   77.8%, `schedule_store` 78.4%). New code arriving less tested than old code is what a month of
   lanes should be expected to produce. This part is a worklist and should be taken as one; a
   milestone that tried to close all of it would be a milestone that never finishes.

   Those seven are done (`## Part 3's head`), and the rest is in `## The 2026-09-21 census`. Re-derive
   any census rate before acting on it: it can be a loom model, proof harnesses, or a deleted crate.

4. Rewrite the baseline once the triage lands. `.cargo/mutants-baseline.txt` is still the
   2026-08-03 run, which is what every weekly report diffs against, so the tree's own comparison
   point predates the census by six weeks. `script/mutation --save-baseline` is the mechanism.
   Do this last, because a baseline written before the survivors are triaged would enshrine the
   regressions as the new normal, which is the one way this milestone could make things worse.

## What "done" means, and it is deliberately not a number

Not a target score. A percentage target is the thing milestone 85's rule exists instead of: it
can be met by excluding awkward crates, and the exclusion is invisible in the rate. Done here is
that parts 1 and 2 have no untriaged survivor left, each one closed as a test, an exclusion in
`.cargo/mutants.toml` carrying its reason, or a recorded gap in `notes/mutation-testing.md`'s ledger.
Whatever the score does as a result is the outcome, not the goal.

A survivor proved equivalent is a success, not a dodge, and milestone 85's ledger already has
that column. What is refused is an exclusion whose reason is that the test would be tedious.

## BUGS

- A triaged crate can regress the same week. Nothing in this milestone stops the next lane
  adding an untested surface, and the weekly report is a report rather than a gate (roadmap 85,
  deliberately). This is a sweep, and sweeps need repeating; whether that should become a gate is a
  question this block does not answer and should not.
- The census is one data point. The weekly workflow succeeded for the first time ever on
  2026-09-14, so "the cadence is alive" rests on a single success. If the second run disagrees with
  the first, part 1's regression table is the thing to re-derive before acting on it, and
  `script/mutation -p <crate>` can settle any single row without waiting for a sweep.
- The survivor identities are not in the tree. `.cargo/mutants-baseline.txt` carries per-crate
  counts and not the mutants themselves, so a lane re-derives a crate's survivors with
  `script/mutation -p <crate>` rather than reading them from the repository. That is cheap per crate
  and is why this block is scoped per crate rather than per survivor.
- This measures the test suite, not the code, which is the standing caveat on every number here,
  and it covers host crates only. The kernel and the arch trees, where risks 5 and 9 live, are
  outside it entirely.
- Memory, not time, is what kills this job. Milestone 277 bounded one mutant's allocation for
  exactly this reason, and AGENTS.md's rule is that a mutation sweep never runs beside lanes. A
  per-crate run is small enough not to be a sweep, and a lane taking part 3 in bulk should read that
  rule first.

## Follow-on

Parts 1 and 2 only. Each of these was checked against the tree on 2026-09-19, on this branch.

- **Done.** *2026-09-20: part 3's seven named crates, 56 survivors to 16,* on
  `milestone/326-new-crate-backlog`. The per-crate table and the argument are in `## Part 3's head`
  above and in `notes/mutation-testing/new-crate-backlog.md`'s `## 2026-09-20` section.
- **Outstanding.** The rest of part 3, which is the other seventeen crates that did not exist at
  the August baseline. Nobody has measured one of them per crate, and the census rates they would be
  picked by are the 2026-09-14 ones. A lane taking this should re-derive before it triages,
  and should first read `notes/mutation-testing.md` and the crate's recent history: three
  crates chosen off the census have now turned out to be already triaged, retired, or measuring
  a loom model, and the ledger said so in every case.

  Two of the nineteen closed on 2026-09-20, and they were the first work this project routed to a
  cheaper model under DECISIONS §202 (mechanical work goes to a cheaper model). Both reproduce on
  the maintainer's own re-run:

  | crate | before | after | killed | equivalent | gap | timeouts |
  |---|---|---|---|---|---|---|
  | `board_console` | 43 survivors, 83.2% | 4, 96.6% | 39 | 1 | 3 | 6 unchanged |
  | `video_terminal` | 79 survivors, 79.0% | 16, 95.8% | 63 | 16 | 0 | 0 |

  `board_console`'s three gaps all need a real tty or a pseudo-terminal pair, which is a dependency
  question above a triage lane's authority. `video_terminal` needed no gap and no exclusion, and the
  crate-by-crate accounts are in `notes/mutation-testing/video-terminal.md` and
  `notes/mutation-testing/board-console.md`.
- **Recorded.** `notes/mutation-testing.md`'s `## Scope and honest caveats` section: a mutant that
  hangs is not a mutant that survived, and this instrument cannot say so. Nine survivors across
  the two 326 lanes were non-terminating rather than wrong, and cargo-mutants 27.1.0's complete set of
  limits is the clock, which milestone 277 (bound what one mutant may allocate) checked rather than
  assumed, so a deadlock and a
  slow test produce the same `TIMEOUT` row and `script/mutation --report` lists both as survivors.
  Every triage so far has had to argue the distinction in prose. What would close it is a rule in
  that report comparing a timeout against the package's own baseline test time.
- **Outstanding.** Part 4, rewriting `.cargo/mutants-baseline.txt`, is deliberately not done. It
  must follow the triage, which is what its own paragraph says, and it should follow a full census
  rather than this lane's eight per-crate runs: `--save-baseline` writes what it is given, so
  feeding it eight crates would replace a 38-crate baseline with eight rows.
- **Milestone 250.** `An unviable mutant is a hole that reads as a pass` already owns the other
  half of what this lane kept meeting. `filesystem_protocol` reports 47 unviable and `elf` 21, and
  no rate here counts them.
- **Milestone 437.** The fixture
  bit-set distinctness tests are hand-maintained lists, and `filesystem_protocol`'s navigation list
  has now gone stale twice, the second time caught by a mutation run rather than a reader. It is
  rung four of AGENTS.md's ladder in a place that has a rung-one answer.
- **Recorded.** `crates/timetable/src/lib.rs`, in a `BUGS` section on `Unbacked`:
  `Admission::Unbacked` can never carry `File` or `Directory`. Two mutants survived on it, and
  whether `admit` should stop pre-consuming the scheduler's `dir` holding is a behavior change
  rather than a test.
- **Recorded.** `notes/mutation-testing/regressions-clock-protocol-swish-filesystem-protocol.md`'s
  `## 2026-09-19` section: `verb`'s compile-time table walk in
  `crates/filesystem_protocol/src/lib.rs` has two mutants no `cargo test` can kill, because its
  checker is `rustc`. The same object as the Kani harnesses, without a module path to exclude by.

- **Done.** *`machine_discovery`, 2026-09-19: 77 survivors to 19, 86.2% to 95.5% of viable,* on
  `milestone/326-machine-discovery-truncation`. It was part 1's category rather than part 3's: a
  crate the August baseline covers, carrying 77 of the 2026-09-19 census's survivors, more than any
  other. Measured before and after with `script/mutation -p machine_discovery`; the before column
  reproduces the census row for row. 58 killed by tests, 11 argued equivalent, 8 recorded gaps,
  and no untriaged survivor is left. The reasons are in `notes/mutation-testing.md`'s
  `## 2026-09-19` section, crate by crate, which is where a reader should go to disagree with one.

  The crate's Kani harnesses were checked before the lane started and are not the miscount that
  inflated `timetable`: they are inline `mod verification` blocks in `acpi.rs`, `framebuffer.rs`,
  `riscv64.rs` and `x86_64.rs`, which the `verification::` exclusion does match. Two mutants are an
  exception the check did not predict, and the finding is the concurrent census-delta lane's rather
  than this one's: a `const` inside such a block gets no module path in its mutant name, so a
  module-path regex cannot reach it. `x86_64.rs:314` is the one in this crate, recorded as a gap;
  that lane measured three tree-wide and filed a proposal for the general fix, whose number is
  minted at merge like every other.

  Two things this lane found that the block's framing had wrong, both worth carrying rather than
  quietly fixing. The first is the timeline: `machine_discovery` did not regress in two days. The 22
  it is compared against is `.cargo/mutants-baseline.txt`'s 2026-08-03 number, the crate grew
  from 212 mutants to 693 over the six weeks since, and the census-delta lane measured PR #927's own
  contribution at 4. The second is the hypothesis this lane was briefed on, that 43 of the 77
  were one truncation defect and one prefix property would take the crate to roughly 96%. The
  property was written alone and measured alone: it killed 12 and reached 88.1%. Forty-one of
  the 77 are bounds or offset shapes, so the reading of the list was close, but only twelve are
  guards a prefix of a valid input can reach, and a prefix loop that asserts only "it returns an
  error" kills half of even those. The note has the argument.
- **Milestone 793.** Milestone 793 (one crate for the right-aligned number writer three programs each copy). `design/roadmap/0793-one-crate-for-the-copied-right-aligned-number-writer.md`: `write_right` is copied into three programs.

## Index row

`design/fatal-risks/README.md`'s risk 3 is AMBER rather than green because 771 survivors from the
2026-09-14 mutation census have never been looked at, and milestone 85's rule is that every one
becomes a test, an exclusion with a reason, or a recorded gap. Four milestones touch mutation
testing and three are repairs to the instrument; only 85 ever turned a score, in August. The order
here is the argument: seven crates that regressed from a score they used to hold, `capability`
first; then `timetable`'s 48 survivors, because it carries the proof risk 2 calls its strongest
counterfactual; then the new-crate backlog as backlog; then the six-week-old baseline every weekly
report still diffs against.
