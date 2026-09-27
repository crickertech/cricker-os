---
status: BUILT
raised: 2026-09-03
built: 2026-09-13
---
# 280. `uefi_loader` at 15% and `documentation` at 52% are unexplained holes in the published score

Written by the milestone 247 sweep as a proposal on 2026-09-03, from
milestone 238's block. **Promoted out of the proposal queue on 2026-09-13 by calef**, who asked what
would progress fatal risk 3: this is step three of four, and the only one that can be worked before a
clean sweep exists. Built the same day on `milestone/280-unexplained-mutation-scores`. *(Number
provisional until the merge queue lands it; 278 is in flight ahead of it.)*

## The answer: half a measurement bug, half a real gap, and two features that reached nobody

The block below is kept as it was written. This section is what the measurement found.

**`documentation`'s 52% was measuring a crate with a third of its test suite compiled away**, which
is the `system_initializer` result (milestone 244) and the `uefi_loader` result (2026-09-04) a third
time, at a **Cargo feature** rather than at a crate or a target. `cargo-mutants` tests one package at
a time (`--test-workspace` is off by default, and turning it on would run the whole workspace suite
per mutant), so every number ever published for this crate came from `cargo test -p documentation`
with its default features. The `builder` feature is off by default so the guest program links no
allocator, and it gates the index writer: 92 of 1,043 mutants that nothing could have killed.

**The cost was not the writer, though, and that is the part nobody had noticed.** `builder` also
gates **six of the crate's twelve index tests**, because a test that reads an index needs `build` to
make one. So the run compiled away half the suite and then scored the *reader* against what was
left. The crate's own `Cargo.toml` described the first half of this as a caveat worth knowing
(*"`cargo test -p documentation` therefore skips the builder's tests"*) and left it at rung four.

Whole-crate runs, all three at the pinned tool version on the same machine:

| | mutants | caught | missed | timeout | unviable | killed |
|---|---|---|---|---|---|---|
| as the sweep measures it, before | 1,043 | 499 | 455 | 48 | 41 | **54.6%** |
| with `builder` compiled, before any new test | 1,043 | 733 | 211 | 58 | 41 | **78.9%** |
| **after this milestone** | 1,056 | 908 | 47 | 60 | 41 | **95.4%** |

The published 52% came from a one-eighth round-robin sample (56 caught, 60 missed); the whole-crate
number in that same configuration is 54.6%, so the sample was honest about a crate being measured
wrongly.

**The fix is a dev-dependency on itself**, which Cargo allows and which resolver 2 unifies across
the test build: `cargo test -p documentation` now compiles the builder and runs all twelve index
tests, `cargo build -p documentation` still does not, and no consumer changed, because a dev
dependency is not transitive. `script/lint` gained the third gate in this family beside the two the
earlier two findings left, deriving from `cargo metadata` that every feature of every mutated crate
is on when that crate's own tests run; it was verified in both directions.

**`--all-features` was the obvious alternative and it is measured rather than assumed.**
`cargo check -p uefi_loader --all-features` fails on this tree: that crate's build script panics
without `NIFE_UEFI_KERNEL`, by design, because the UEFI application embeds the kernel it boots. One
global switch cannot be right for a workspace whose only three featured packages want three
different answers.

### The two features the sweep found that no test could have

Both are the same shape and it is a shape worth naming: **a value that is computed correctly and
read by nobody.** No test can fail on one, because there is no output to be wrong. A mutation run
finds them immediately, and finds them as a *cluster* of survivors in one function.

- **Table column alignment.** `read_align` parsed `:---`, `---:` and `:---:` off the delimiter row
  into a per-column array from the first day; `flush_table` padded every cell on the right whatever
  that array held. All three rendered identically to `---`. Sixteen mutants survived there, one of
  them replacing the entire function with `()`. It is honoured now, and resets with `delimited`
  because both are properties of one delimiter row.
- **Tab indentation.** `indent_of` returns a byte offset and a column count, counting a tab as four
  columns. Every caller took the offset and dropped the count, including the one that uses it as a
  margin, so a tab indented by one column. Invisible on this corpus, which is why only a sweep could
  have found it: every tab-indented line in this repository's markdown is inside a fence, where the
  margin is `CODE` and that code does not run.

### What is left, and it is listed rather than implied

The residue is in `notes/mutation-testing.md`'s ledger, per function, split into proved equivalents
and honest deferrals, with the timeouts accounted for separately as the loop-control hangs they are.
`crates/documentation`'s own `BUGS` section carries the pointer, so a reader who meets a number for
this crate in a published report can find what it is made of. **That is the half of this milestone
that milestone 244's block is the template for**: an accepted score with a written reason beats a
chased one.

**Half of this was already answered, and the promotion did not notice.** `uefi_loader` was diagnosed
on **2026-09-04**, the day after this proposal was written, and the record is
`notes/mutation-testing/uefi-loader.md`'s dated section plus the exclusion and its derived gate in
`.cargo/mutants.toml`. The 15% was arithmetic rather than a finding: `src/main.rs` carries
`required-features = ["uefi"]`, so `cargo test` never puts it in the build graph, and all 154 of its
mutants came back MISSED in *"0s build + 0s test"* because nothing rebuilt. The pure half the design
had lifted out to be host-testable was at **94.1%** the whole time. The result of milestone 244 (the
largest crate in the tree is proved by nothing) one level down, at a target rather than a crate.

**So what is left of this milestone is `documentation` at 52%**, and one residue that is already
tracked elsewhere: excluding `src/main.rs` made the number honest, not the file proved, and whether
its 790 firmware lines get lifted the way `handoff` and `image` were is milestone 381,
`design/roadmap/381-the-uefi-loaders-firmware-half-is-proved-by-one-boot.md`.

**Where it sits on that path.** Milestone 277 built the memory bound so a sweep can survive a runaway
mutant; the first green scheduled run is what proves it. This milestone is what stops the number that
run produces from having two unexplained holes in it, and
`design/roadmap/proposals/fatal-risk-3-against-the-new-number.md` is the reading calef then does. It
can start now: both crates can be measured against the existing 83.4% shard without waiting for a
full run.

**In brief.** The first mutation report the sweep ever published put the tree at 83.4%, down from
92.4%. Three crates carry nearly all of that fall. Milestone 244 took `system_initializer`, measured
it, and closed `RECORDED` on the honest reason that the pure fraction a mutation can reach in that
crate is small. The other two, **`uefi_loader` at 15% and `documentation` at 52%**, have nobody on them.
(That crate was measured under its then-name `manual`; calef ratified `documentation`
on 2026-09-13. The quoted `Follow-on` below keeps the old name, as a quotation must.)
The work is to measure each properly and give each an answer: tests that kill the surviving mutants,
or a `BUGS` entry that accepts the score and says why.

## Why this matters

The published number has two holes in it and nothing anywhere admits to them. No `BUGS` entry
accepts either score, so a reader who finds 83.4% has no way to learn that two crates account for
most of the gap or that nobody has looked at them. That is the failure mode this tree's `BUGS`
convention exists to prevent, and it currently applies to a figure that is about to be quoted in a
fatal risk's verdict.

15% is low enough to mean something specific rather than to be noise. Either `uefi_loader` is barely
tested, or almost all of its mutants are unreachable by a test that cannot boot firmware, and those
two conclusions have opposite consequences. Only measuring tells them apart. `documentation` at 52% is the
milder case and the more likely to be a real testing gap, since it is ordinary host-testable Rust.

Milestone 244 is also the template for the acceptable outcome, which is worth saying because it
keeps this from being open-ended. It did not chase the score; it measured, found the reachable
fraction small, and recorded that. Either of these two crates may end the same way, and a recorded
reason is a complete answer.

## Where it came from

Milestone 238's `## Follow-on`: *"Measure and answer for `uefi_loader` at 15% and `manual` at 52%,
the two crates this block names beside `system_initializer` as carrying nearly all of the fall from
92.4% to 83.4%. Milestone 244 took `system_initializer` alone. Nothing tracks these two, and no
`BUGS` entry anywhere accepts their scores, so the published number has two unexplained holes in
it."*

## BUGS

- **The score is a whole-crate run on one machine, not the weekly sweep's number.** The weekly
  report shards eight ways and publishes a sample; these three runs are `script/mutation -p
  documentation` end to end. They are comparable to each other and to the published sample only in
  the way a census is comparable to a survey.
- **The residue is not zero and the ledger is an argument in places.** `notes/mutation-testing.md`'s
  own scope section says a verdict reached by reading is wrong about ten percent of the time; the
  equivalence groups there were argued from the code, with one applied and re-run. The deferrals are
  named as gaps rather than dressed as equivalents.
- **The new `script/lint` gate checks that a feature is COMPILED, not that anything tests it.** A
  crate could satisfy it with a dev-dependency and no test behind the feature at all, and the
  mutation report would then honestly say so. It also reads `required-features` as an exemption, so
  a feature that gated both a `[[bin]]` and a `#[cfg]` block in the library would pass and should
  not. Nothing in the tree does that today.
- **Table alignment is honoured in the first chunk of a table only**, on the same terms as the
  header emphasis beside it, and both are recorded in `crates/documentation`'s own `BUGS`.
- **This does not re-read `design/fatal-risks.md`'s third risk**, which is an architect's, and does
  not produce the clean full sweep that risk wants. It removes one of the two named holes from the
  number that sweep will publish.

## Follow-on

- **Done.** The measurement, the `builder` fix, `script/lint`'s third mutation gate and 34 tests, on
  `milestone/280-unexplained-mutation-scores`; the whole story is in `notes/mutation-testing.md`'s
  2026-09-13 section, which carries the three whole-crate runs and the per-function ledger.
- **Done.** `uefi_loader`'s half was answered on 2026-09-04, the day after this was written as a
  proposal, by the exclusion and derived gate in `.cargo/mutants.toml` and the dated section in
  `notes/mutation-testing.md`. The promotion did not notice; the correction is at the head of this
  block.
- **Recorded.** The residue this crate's tests do not reach, per function, split into proved
  equivalents and honest deferrals, in `notes/mutation-testing.md`'s ledger, with the pointer a
  reader meets beside the feature in `crates/documentation/src/lib.rs`'s own `BUGS`.
- **Recorded.** A table that spills past `TABLE_ROWS` loses its header emphasis and its column
  alignment in the second chunk, both being properties of a delimiter row that is not carried across
  the flush. In `crates/documentation/src/lib.rs`'s `BUGS`, where a reader meets the table support.
- **Milestone 381.** Whether `uefi_loader/src/main.rs`'s 790 firmware lines get lifted the way
  `handoff` and `image` were is the residue excluding the file made honest rather than proved. It
  was written up the same day as an unnumbered proposal and numbered on 2026-09-19 by milestone
  433's drain of the pile.
- **Done.** The reading calef then does. The clean full sweep this bullet wanted arrived on
  2026-09-14 once milestone 277's memory bound landed, and he read it on 2026-09-19: risk 3 is
  **MEASURED, AMBER**. This milestone's two crates are why the fall was not real, and they are
  unrecognisable in the census: `uefi_loader` 100%, `documentation` 95.4%.

## Index row

Written by the milestone 247 sweep 2026-09-03, promoted 2026-09-13 by calef while working out what
progresses fatal risk 3. `uefi_loader` was answered on 2026-09-04 and the promotion did not
notice, so what was left was `documentation`. Its 52% was the third instance of one failure:
cargo-mutants tests one package at a time, so a non-default feature is mutated and never compiled,
and `builder` gates the index writer AND six of the twelve index tests, so the reader was scored
against half a suite. 54.6% whole-crate before, 78.9% with the feature on, 95.4% after. Fixed by a
dev-dependency on itself plus a third script/lint gate; the sweep also found table alignment and
tab indentation computed correctly and read by nobody.
