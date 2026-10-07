---
status: AMENDED
raised: 2026-08-30
decided: 2026-08-30
ratified_by: calef
---

# 134. A harness carries a machine-replayable falsification record, or it is not evidence

calef, 2026-08-30, in three rulings; the patch path amended 2026-08-31 when
milestone 194's lane found the ratified spelling could not name eighteen of `paging`'s harnesses,
clarified 2026-09-01 to say package rather than crate, and amended twice more from milestone 323's
review: `Expected to fail:` ratified 2026-09-18 and "harness" widened to cover a kernel `#[test_case]`
2026-09-19. Both of those late amendments ratified what the tree already did.
Originally: The direction: *"It sounds like [option C] is
where we want to land if we want to state that nife is proven."* Then the format, after the options
were costed: *"Go with the diff, weekly plus per-PR for touched harnesses."* Then the spellings, ratified the
same day and recorded at the bottom. Nothing here is open.

## What is being decided

Whether every Kani harness in this tree must carry evidence that it can fail, in a form a machine
can replay, and what that form is.

## Why now

`notes/verification.md` already states the rule, and states it well:

> Falsify a property before believing it. Break the code the harness guards and confirm the harness
> fails. Every milestone 35 property was falsified this way, and one falsification corrected a claim in
> the code (the load-bearing guard was not the one the comment pointed at). A harness that cannot be
> made to fail is not evidence.
>
> -- notes/verification.md

It is rung four, honored by whoever remembers, and milestone 191 measured what that produces:
145 harnesses, 23 `kani::cover!` sites across 4 of 24 harness crates, and no harness anywhere
recording what was done to falsify it. The reverse pass found the predictable result:
`capability::subset_is_reflexive` proves `a & !a == 0`, a tautology no plausible implementation error
breaks, and twelve of the 26 `paging` harnesses restate six properties once per ISA.

The forcing question is calef's, and it is about a claim rather than about code: if the project is
going to say proven, the evidence that the proofs are load-bearing cannot itself be a written
claim that they are load-bearing. That is one level of indirection away from evidence, and it is the
same shape as the nine misrecorded roadmap statuses and the fabricated block quote that survived
twelve days of gates.

## Two failures, and they are not the same problem

Conflating them produces a convention that half works.

Vacuity is mechanical. Assumptions contradict, the input set is empty, and Kani reports
`SUCCESSFUL`. `kani::cover!` is the check that catches it, which `notes/verification.md` already
says.

**Tautology is not.** A property true of every plausible implementation passes every cover check
ever written. Only falsification catches it: if nobody can describe a change that turns the harness
red, the harness is decorative.

## Prior art, and it is a field rather than an idea

Read rather than recalled, because this tree carried a fabricated block quote for twelve days.

The model-checking literature has done this since roughly 2004, under two names that belong
together. Vacuity detection and coverage metrics are the standard sanity checks, and the
survey work makes the unifying observation explicit: both work by *repeating the verification on a
mutated input*, with vacuity mutating the specification and coverage mutating the system.

What this section proposes has a name there: mutation coverage for model checking, where an
element of the system is considered covered by the specification if changing that element falsifies
the specification. That is "break the code, confirm the harness fails", stated formally, with two
decades behind it.

§39's protected class applies: a term a reader already knows from outside costs a newcomer
nothing, so this convention should be spelled in the field's vocabulary rather than in ours.

There is also a cheaper family, Inductive Validity Cores, which compute the minimal set of model
elements a proof actually needed and so give coverage without re-running anything. *(Since checked: Kani exposes none, as `script/falsifications` records.)*

Sources: [Coverage Metrics for Formal
Verification](https://link.springer.com/chapter/10.1007/978-3-540-39724-3_11), [Sanity Checks in
Formal Verification](https://www.cs.huji.ac.il/~ornak/publications/concur06b.pdf), [Mutation Coverage
Estimation for Model Checking](https://link.springer.com/chapter/10.1007/978-3-540-30476-0_29).

## The options

| | what | catches | cost | prior art |
|---|---|---|---|---|
| **A** | a lint requiring `kani::cover!` in every harness | vacuity only | cheap, per pull request | vacuity detection, standard practice |
| **B** | a **prose** falsification record, presence lint-enforced | tautology, by human review only | cheap, and it is a claim nobody re-runs | none |
| **C** | a **machine-replayable** falsification record, replayed on a schedule, each harness required to go red | both, mechanically | about one `script/verify` run per sweep | mutation coverage for model checking |
| **D** | Inductive Validity Cores | coverage, with no re-run | unknown, may not exist in Kani | IVC literature |

## The recommendation: A and C, with B refused on the record

A, because vacuity is mechanical and a cheap gate catches it. It is also already the tree's
stated practice, so the lint is making an existing rule enforceable rather than adding one.

C, because it is the only option that lets the word carry weight. The price is smaller than it
looks, and the arithmetic is the reason this is recommended rather than admired: a falsification
runs one harness, not all of them. `script/verify` costs about 42 minutes for 140 harnesses, so a
full sweep of one recorded falsification per harness is the same order of magnitude as a single
verify run, not 145 times it.

Its posture is `script/mutation`'s, deliberately copied: a scheduled report with a baseline, not
a per-commit gate, because the run costs too much to block a commit on and a regression is a worklist
entry rather than a defect in whatever landed that day. A survivor here is a harness whose own
recorded falsification failed to make it fail, which is exactly the signal nothing in this tree can
currently detect.

B is refused rather than skipped, because it is the tempting middle and it is the precise failure
this section exists to end: a written claim standing in for evidence. It would also pass its own lint
forever while meaning nothing.

Machine-generated mutation is refused too, and the reason is cost rather than preference.
`cargo-mutants` produced 5,551 mutants over the host crates. At test-run cost that is hours; at
proof-run cost it is not a job. So the affordable form is one recorded falsification per harness,
written by whoever writes the harness.

## What this does not claim, which is the half that matters

**Parity with seL4 on the proof is not available and should never be implied.** seL4 proves
functional correctness by refinement to an abstract specification, plus integrity and information
flow. That is a different category and a decade of work. nife does bounded model checking of selected
properties over selected crates, and after milestone 193 (put `kernel/src` within reach of the
prover) it will do that over some of the kernel too.

Parity on the assumption discipline is available now, and it is most of what makes seL4 trusted.
The seL4 Foundation publishes *What the Proofs Assume* as a first-class page with eight numbered
assumptions: assembly code (about 340 lines, assumed correct), hardware functionality, cache and TLB
management, boot code (about 1,200 lines, outside the proof), virtual memory (where they say plainly
that the proof is not from first principles and there is potential for human error), DMA devices,
information side-channels, and the compiler except where binary verification covers it. They also
keep test suites specifically for the code left as an assumption.

And they name this section's exact problem as their hardest assumption: **one can never be sure that
a formal specification means what we think it should mean.**

So the claim this section buys is not "nife is proven". It is the sentence this tree already writes
about benchmarks, applied to proofs: **these specific properties are proved, each is falsifiable and
the falsification is replayed on a schedule, and here is what is not covered.** That sentence is
honest at any size. It gets larger from milestone 193, not from this decision.

## The format, decided

### The carrier: at the harness, derived by a script

This tree has solved this exact problem once and the answer transfers. Milestone 115 needed a
fact enumerable across the tree without a central registry, and `script/names`' own header records
why the registry lost:

> provenance lives at the name, in the header of the crate, program or script it belongs to,
>
> -- script/names

The full argument there is worth reading and does not quote cleanly (a multi-line quote out of a
shell comment carries its own `#` markers into `script/citations`' normalized text, which is a
limitation of that check worth knowing before writing one). In summary: milestone 115's first draft
proposed a single ratified-names table in `design/naming.md`, calef rejected it on 2026-08-04 for
scaling the way the original `DECISIONS.md` and `design/roadmap.md` scaled, and the decisive half was
collisions rather than size, since every lane adding a name would edit one file and that is exactly
what produced three section-number collisions in a day.

Same argument, same shape: a `Falsification:` block immediately above the harness, beside `Name:`
in the same family, and a script derives the report. Same family as `script/names`, `script/roadmap` and `script/decisions`, and for the
same reason: a computed report over the tree cannot drift from it.

### Three states, and the unknown one is first-class

The naming convention's second lesson, taken whole. `script/names` has three states because two
questions were wearing one word, and it insists that *"`unrecorded` is a first-class answer and must
stay one"*, since inventing a ratification to fill a row puts a false claim in the one record whose
job is saying who claimed what. Exactly the same is true here:

| state | means | what the sweep does |
|---|---|---|
| **`replayable <path>`** | a patch exists that a script applies to turn this harness red | applies it, runs that one harness, **requires red**, reverts |
| **`attested <date>`** | a person broke the code and watched it fail; nothing can re-check it | counts it, and it is a worklist entry |
| **`unfalsifiable <why>`** | examined, and no defect in the code under proof turns it red; reason written | counts it apart |
| **`unfalsified`** | nobody has | counts it, and this is the claim's honest denominator |

This is what makes the convention shippable against 145 existing harnesses, which the first draft
of this section named as its largest cost. They land at `never` on day one, the lint passes
immediately, and the worklist derives itself instead of being written.

It also states the refusal precisely. **Prose is refused as the destination, not as a way-station.**
`attested` is honest and is how a falsification exists between being performed and being made
replayable; what is refused is a convention whose *endpoint* is a claim nobody re-runs. The word
carries that: an attestation is an assertion, and an assertion is not evidence.

### The mutation is a unified diff

calef, 2026-08-30, choosing between three spellings that differ in what they cost later rather
than now.

- A unified diff in a sibling file. Replay is `git apply`, run the harness, require failure,
  revert. No new tooling at all. Chosen.
- A structured operator record (file, symbol, operator). Survives refactors, and needs a mutation
  engine that can drive `cargo kani`, which `cargo-mutants` was not built for and nobody has tried.
- A `cfg`-gated defect in the source. Cannot rot, because the compiler checks it, and it puts 145
  blocks of deliberately wrong code into shipping source, each a path no normal build exercises.

Why the diff wins, and it is the elegance tenet rather than the convenience one. It has the
fewest moving parts, needs nothing that does not exist, and leaves the operator route open if a
sweep ever proves cheap enough to want it.

Its rot is a feature, and this is the load-bearing claim. A patch that no longer applies means
the covered code moved, which is exactly when a falsification should be redone rather than trusted.
A record that survives a refactor of the thing it falsifies is asserting something nobody checked.

### Cadence: weekly, plus per-PR for touched harnesses

calef, 2026-08-30. The full sweep is `script/mutation`'s posture, deliberately copied: a
scheduled report against a baseline, not a per-commit gate, because a full run costs about one
`script/verify` and a regression is a worklist entry rather than a defect in whatever landed that
day.

**The per-PR half is what the mutation sweep does not have**, and it is why this is not simply a copy.
A lane that edits a harness or the code it covers re-falsifies that harness only, which is
seconds rather than an hour, and it closes the window in which a refactor silently invalidates a
record. `script/verify --affected-since` already computes the "can this change reach the proofs"
question from `cargo metadata`, so the machinery to decide which harnesses a diff touches exists.

## The spellings, ratified

calef, 2026-08-30, with the refusals kept because they are the half a future proposer needs.

`Falsification:` is the keyword, beside `Name:` in the same block. `Name:` labels the block with
the noun for the thing it records, and the thing here is a falsification; in `Name: ratified
2026-08-30` the noun is the label and the participle is the state, which gives `Falsification:
replayable <path>`. Refused `Falsified:`, which reads more naturally at the site and breaks that
parallel; refused `Evidence:` as too broad, since every block in this tree is evidence of something;
refused `Coverage:`, which collides with what `script/coverage` already owns.

`replayable` / `attested` / `unfalsified` are the three states. `attested` is the load-bearing
one: it says a person asserts this, and an assertion is not evidence, which is the distinction this
whole section exists to draw. `unfalsified` mirrors what `unrecorded` does in `script/names`, the
honest negative, greppable, first-class rather than a gap. Refused `witnessed`, reluctantly, because
DECISIONS §31 (the foreign-language seam) already spends "witness" on the confinement pages and a
second sense would cost a reader the recognition; refused `manual` (says how, not what), `never` (a
bare adverb reads as a verdict on the harness rather than a state of the record), and `by hand` (two
tokens, worse to grep and worse to align).

`script/falsifications` is the sweep. The `script/` family splits in a way nobody had written
down: what *does* something is a verb (`verify`, `test`, `fuzz`, `bench`) and what *reports* is a
noun (`names`, `citations`, `decisions`, `roadmap`, `coverage`, `mutation`). This section decided a
report with a baseline rather than a gate, so it belongs in the noun half, and the name matches
`citations` in shape. Refused `script/falsify`, whose verb form would promise the gate this section
deliberately did not build.

`<package>/falsifications/<module.path>.<harness_fn_name>.patch` is where a patch lives,
for example `crates/paging/falsifications/sv39.index_is_always_in_bounds.patch` and
`crates/capability/falsifications/verification.subset_is_reflexive.patch`. Per-crate rather than
central, which is the carrier argument this section already rests on: at the thing, so two lanes
touching two crates cannot collide. Naming each file for its harness makes the link mechanical, and
lets the lint check both directions, that every `replayable` path resolves and that every patch has a
harness. Refused a repository-root `falsifications/`, for the reason milestone 115 refused a central
names table on 2026-08-04.

**Clarified 2026-09-01: the path is beside the *package*, not under `crates/`.** The ratified
wording said `crates/<crate>/`, which was true of every case that existed when it was written and is
not the rule. `kernel/` and `user/` are packages at the repository root, and within a day of the
mechanism landing both had falsification patches: milestone 202 (every confinement test is a ritual until somebody breaks the confinement)
created `kernel/falsifications/` and milestone 197 (`user/` and `xtask` are out of reach of the
prover) created `user/falsifications/`. Both follow this section's stated reason, which is per-package
rather than central so that two lanes touching two packages cannot collide; both were outside its
words. A rule three of whose own instances violate it teaches the next reader to guess whether that
was sloppiness or intent, so the words now say what the reason always meant.

And one of those two paths has since moved, which is worth leaving visible rather than
correcting in place. Milestone 175 split `user/` into `components/` and `fixtures/` on
2026-09-13, so 197's patch lives at
`components/falsifications/proofs.push_never_writes_past_the_buffer_it_was_given.patch` today and
there is no `user/` package. The clarification above was accurate when it was written; the citation
went stale twelve days later, which is the ordinary cost of naming an instance to illustrate a rule
and is cheaper than not illustrating it.

Amended 2026-08-31, and the first spelling was wrong. It was
`falsifications/<harness_fn_name>.patch`, which assumes a harness function name is unique within its
crate. **In `paging` it is not**: six properties are stated once per ISA across `aarch64.rs`,
`sv39.rs` and `x86_64.rs`, so eighteen harnesses share six names, three would collide on one file,
and `cargo kani --harness index_is_always_in_bounds` cannot separate them either. Milestone 191 (did
the proofs catch the bugs?) had already reported that duplication and the maintainer ratified the
spelling without checking it against the tree. Found by milestone 194's lane on first contact.

The module path is always included, with no branch. The reason is the one this section used to
choose a unified diff: fewest moving parts and fewest places to be wrong. It is unique by
construction, it is stable (a harness added elsewhere cannot retroactively invalidate an existing
path), and it is already the string the tooling needs, since Kani's `--exact` takes the fully
qualified harness name, so the path is the sweep's own filter with the separators changed.

Refused, and this one matters more than the others: **unqualified when unique, module-qualified when
not.** That is a branch keyed on an *unstable* property, since adding a harness elsewhere would
retroactively invalidate an existing path, and it is the same shape as the two-tier program-naming
rule calef rejected on 2026-08-01 one domain over. Also refused a subdirectory per module, which
turns `filesystem_proto`'s three-deep nesting into a tree of near-empty directories, and renaming the
eighteen harnesses to make a filename work, which would put the ISA in a function name the module
already states and is a naming decision driven by a path.

## `Expected to fail:`, ratified 2026-09-18

calef, 2026-09-18, ratifying a convention rather than minting one, which is why this is four
paragraphs and not four options.

A falsification record names the assertion it expects to fail, on a line beginning
`Expected to fail:` in the patch's prose head, beside `Falsifies`. `script/falsifications` may then
compare it against the transcript and fail a record whose red arrived somewhere else.

**The naming question had already been answered by use, and the proposal that raised it did not
know.** Milestone 323's part 4 was filed as a decision about what to call a new field, on the
observation that *"four patches in the tree already solve this in prose"*. That was a sample. Counted
on 2026-09-18: 66 falsification records, all 66 carrying the line, 65 spelling it exactly
`Expected to fail` and one `Expected red`
(`crates/nifefs/falsifications/verification.the_validation_implies_reads_slice_is_in_bounds.patch`,
corrected in the same change). So the decision was not what to name a field; it was whether to ratify
the name 65 records already used, or rewrite 65 files to no reader's benefit.

**What it buys, and what it does not.** It closes the wrong-reason red: milestone 202's break of
[§31](0031-foreign-language-seam.md) surfaced as a 234-second watchdog timeout reading *"a livelock, not a lost
wakeup"*, which is the right answer with a diagnostic containing no word about confinement, and
milestone 305 hit the same edge and swapped the patch rather than record a red for the wrong reason.
It buys nothing against an unreachable assertion, which is milestone 307's subject and a
different hole: there the patch's prose and the transcript simply agree on some other line. Saying so
here is the point, because a gate that looks like it covers both would be worse than one that covers
one.

## "Harness" covers a kernel test too, widened 2026-09-19

calef, 2026-09-19, closing a gap `script/falsifications` named against itself and handed back.
Its header says, in the paragraph on milestone 305, that the only word which strains is `harness`,
that it strains in prose rather than in the convention, and that widening the wording is a decisions
edit rather than that script's.

Read every "harness" in this section as "a claim the sweep can replay". Today that is a
`#[kani::proof]`, replayed with `cargo kani --harness <name> --exact`, or a kernel `#[test_case]`,
replayed with milestone 210's `cargo xtask test --arch <a> --test <name>`. The `Falsification:`
block, the three states, the patch path and the cadence are the same for both, and were the same for
both before this paragraph existed: milestone 305 built the second replay verb on 2026-09-16 without
needing a word here changed, and milestone 202's patch obeyed the ratified path spelling with no
special case on 2026-08-31, sixteen days before that. This widens the words to what the convention already does,
which is the same job the 2026-09-01 clarification did for the path.

**The one deliberate asymmetry is not a wording question and is not widened.** A `#[kani::proof]`
that carries no block fails `script/falsifications --check`, because the denominator is the point and
on 2026-09-19 the sweep counted 170 harnesses in 26 packages. A kernel `#[test_case]` is opt-in,
because the tree carries 427 of them and almost none is a confinement claim, so requiring a block
on all of them would buy a wall of `unfalsified` and teach nobody anything; 14 carry one today and
13 of those are replayable. A kernel test is counted when it carries a block. The both-directions patch
check is what keeps that honest: a patch under a package's `falsifications/` must be claimed by a
proof or by a test, and an unclaimed one is reported as rot.

The title stays as written, because it is cited by path elsewhere in the tree and the sentence it
makes is still true. The vocabulary is fixed here rather than by a rename, which is the cheaper of
two irreversible-looking edits and the one that leaves the citations working.

## `unfalsifiable`, a fourth state, added 2026-10-06

calef, 2026-10-06 UTC (#1730): *"unfalsifiable ratified."* and *"amend §134"*. Its meaning is in the
table above. `script/falsifications --check` fails one with under 200 characters of reason, so one
word cannot empty the `unfalsified` count. The name's refusals are in `helpers/rust_source.py`.

## BUGS

- This adds friction to writing a harness, at the moment harness-writing is about to increase
  sharply if milestone 193 lands. That is a real cost and this section does not pretend the
  convention is free.
- A recorded falsification proves the harness catches *that* defect, not that it catches the
  class. It is a floor, and a low one.
- A diff rots against refactors, and the section above argues that is correct rather than
  defending it as harmless. It is still churn, and a heavily refactored crate will re-falsify often.
- The states make the convention shippable and also make it easy to stall. Every harness
  may sit at `never` forever while the lint stays green, so the number that matters is the ratio, and
  nothing forces it upward.
- The `kani::cover!` lint can be satisfied vacuously too, by covering something trivially
  reachable. A gate that counts `cover!` sites is weaker than a human asking what the cover is for.
- `kernel/src/arch/` stays out of reach under every option here, so the architecture layer, where
  the VisionFive 2's undelivered-wake defect actually lived, gains nothing from this section.
