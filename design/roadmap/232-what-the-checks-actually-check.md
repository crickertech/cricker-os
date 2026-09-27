---
status: BUILT
raised: 2026-09-02
built: 2026-09-03
---
# 232. Audit every check against two questions: does anything run it, and does it block

Minted 2026-09-02 by calef, after four independent findings in one
day, each a different way for this tree's verification to report something that was not true.
*(Number provisional until the merge queue lands it.)*

The inventory is `notes/check-inventory.md`, measured 2026-09-03 against `a0059022`: every
workflow job, every `script/` entry point that renders a verdict, and the two checks that are
neither. Nineteen check names reach a pull request; eleven block. What it found, ranked by what a
wrong answer costs:

1. The mutation workflow has never once succeeded. Four scheduled runs since it was written
   (2026-08-10, 08-17, 08-24, 08-31), every one red. `design/fatal-risks.md`'s third risk stands at
   MEASURED green on a 2026-08-03 number and closes by saying *"the weekly workflow already publishes
   the report."* It has published nothing, through 2,529 commits.
2. Miri has been red for three weeks on a missing environment variable, not on undefined
   behaviour. `crates/manual/tests/render.rs` reads `CARGO_MANIFEST_DIR` at run time; Miri does not
   forward the environment. The same test passes under `cargo test`. A check that cries wolf on a
   schedule is worse than one nobody runs, because the only available response is to stop reading it.
3. `re-falsify the harnesses this change can reach` does not block, and PR #663 merged through the
   hole on 2026-09-03 with that check red and all eleven required checks green.
4. Three instruments run nowhere, and the two cheap ones were measured rather than assumed:
   `script/interleaving-check` at 12.4 seconds (loom, 26 harnesses, all green) and
   `script/crate-probes` at about 3 minutes (43 of 50, which is exactly the split
   `notes/crates-io-on-nife.md` records, so fatal risk 1's instrument is the one place the audit
   found the record already true). `script/rule-violations --check` is the third.
5. `image permissions` and `architect hold` are already-recorded open asks, both waiting on the
   same repository setting, both saying so in their own files.

The recommendation, which is an architect's to take: require `re-falsify the harnesses this
change can reach`, `image permissions`, and `architect hold`. Refuse `verify scope`, the `prove`
shards and `draft gate`, which are either aggregated by a required check already or are not
verdicts. The note argues the `architect hold` case against this block's own sentence, since the
workflow was built to be required and says so three times.

Risk 3's status was not touched, per this block. The two facts it needs are in the note: the
number is a month old, and the sentence that made a stale number acceptable is false.

It was minted with no gate, on the grounds that it is an audit of what already exists. That held,
with one qualification worth recording: two of the five findings are visible only in GitHub's run
history and in nothing that lives in this tree, so the audit needed the API as much as the files.

In brief. AGENTS.md's second principle says the method works because of the gates, the proofs and
the review discipline. `design/fatal-risks.md` risk 3 (the tests do not test anything, and the
quality is illusory) stands at MEASURED, green, on 92.4% of viable mutants killed.

Four findings on 2026-09-02 say that number is measuring somewhere the failures are not, and no
two of them are the same defect:

- Milestone 214 (a test that prints "skipping" and returns is counted as passed) found 80 sites
  across 18 files that printed a skip line and returned, so the harness counted them as passes.
  Twenty-five tests left the pass column on x86_64 once they told the truth. A mutation sweep cannot
  see this: a test that returns early still runs and still kills nothing.
- Milestone 222 (the one command a person runs before pushing has a leg that fails instead of
  skipping) found `script/test --hvf` failing rather than skipping, telling a contributor something
  was broken without saying whether it was theirs.
- Milestone 230 (`script/shell-check` is red on `main`, on both architectures, and nothing says
  so) found a check red for five days that ran in neither `script/test` nor CI. Its cause was a
  constant restored on two true observations by a lane that was right about everything it checked and
  shipped a system that could not boot, because no suite in `script/test` boots the real init.
- And then that same check, once running, found `login` dead on every boot on both
  architectures while the boot printed `init: login ready`, because init measured the identity
  provisioning rather than login's survival.

Four shapes: a test that lies, a gate that fails instead of skipping, a check nothing runs, and a
check that passes while what it names is dead.

## What the audit is

Two questions asked of every check in this repository, and the answers written down:

1. Does anything run it? On 2026-09-02 `script/repeat-under-load` and `script/crate-probes` were
   referenced by neither CI nor `script/gates`. `crate-probes` builds fifty crates.io crates
   against the nife `std`, which is the instrument behind fatal risk 1 (only software written for
   nife runs on nife), and that risk is recorded **GREEN**. Its last result is whatever somebody saw
   when they last ran it by hand.
2. **Does it block?** The `main` ruleset requires eleven checks. CI runs more. On 2026-09-02
   `image permissions` (milestone 208's W^X gate, added that day), `re-falsify the harnesses this
   change can reach`, and `architect hold` all ran without being required. The third is correct, since
   a label gate should not block. The first two are worth deciding rather than inheriting: a linker
   script could reintroduce a writable-executable segment, turn that job red, and still merge.

**Then a third question, which is the one with teeth: what does a green result actually assert?**
`swish-check` was green while `login` was dead. That is not a check that failed to run or failed to
block; it is a check whose passing meant less than its name.

## What it must not become

**Not a sweep that adds checks.** `script/lint` has had three checks deleted for the "only ever
rejects legitimate work" signature, and this milestone would be a poor way to acquire the fourth,
fifth and sixth. The deliverable is an inventory and the decisions it forces, not a pile of new
gates.

**Not a rewrite of risk 3's status on one day's evidence.** Whether the mutation number should be
re-read is `design/fatal-risks.md`'s question and it is an architect's. This milestone gives them
the inventory to answer it with.

## BUGS

- An audit is a snapshot, and the four findings above all arrived within one day of each other,
  which suggests the rate matters more than the count. Nothing here creates a mechanism that keeps
  the answer current.
- It cannot find the fourth shape by inspection. A check whose passing means less than its name
  is only discovered by someone asking what it proves, which is what found `login`. An inventory can
  list checks; it cannot audit their meaning without reading each one.
- **Two of the unrun checks are expensive** (`crate-probes` builds fifty crates, `repeat-under-load`
  boots QEMU repeatedly), so "run it in CI" is not automatically the answer and the audit should
  price rather than assume.

## Follow-on

- **Milestone 238.** The two scheduled workflows that have never once produced a result: the
  mutation sweep, four red runs and zero reports while `design/fatal-risks.md` risk 3 stands green
  on a number it says the workflow is refreshing, and the Miri check, three weeks red on a missing
  `CARGO_MANIFEST_DIR` rather than on undefined behaviour. 238 was minted the same day from these
  two findings.
- **Recorded.** `notes/check-inventory.md` carries the required-list recommendation and the two
  already-recorded open asks. Three checks would join the ruleset (`re-falsify the harnesses this
  change can reach`, which PR #663 merged straight through on 2026-09-03; `image permissions`, whose
  own `BUGS` paragraph in `ci.yml` says the same thing; and `architect hold`, which the note argues
  against this block's own sentence). It is a repository setting and calef's, and the note prices
  each one rather than assuming.
- **Recorded.** `design/roadmap/232-what-the-checks-actually-check.md`'s BUGS: an audit is a
  snapshot, all four findings arrived within one day of each other, and nothing built here keeps the
  answer current.
- **Recorded.** `design/roadmap/232-what-the-checks-actually-check.md`'s BUGS: the fourth shape, a
  check whose passing means less than its
  name, cannot be found by inspection. An inventory lists checks; only someone asking what a check
  proves finds the next `login`.
- **Recorded.** `design/roadmap/232-what-the-checks-actually-check.md`'s BUGS, on the three
  instruments nothing runs: two of them are
  expensive (`crate-probes` builds fifty crates, `repeat-under-load` boots QEMU repeatedly), so "run
  it in CI" is not automatically the answer and the audit prices rather than assumes.
- **Milestone 341.** Give the three instruments
  nothing runs a caller: `script/interleaving-check` (26 loom harnesses, 12.4 seconds, green),
  `script/crate-probes` (about 3 minutes, 43 of 50) and `script/rule-violations --check`. Deciding
  which joins `script/gates`, which joins CI and which gets a cadence is unowned, and fatal risk 1
  stands GREEN on a hand-run instrument meanwhile.

## Index row

`notes/check-inventory.md`: 19 check names reach a pull request and 11 block; the mutation
workflow behind fatal risk 3 has never once succeeded, Miri is red on a missing environment
variable, and three instruments run nowhere
