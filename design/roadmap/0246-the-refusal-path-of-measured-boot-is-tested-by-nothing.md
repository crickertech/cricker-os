---
status: BUILT
raised: 2026-09-03
built: 2026-09-03
---
# 246. Measured boot's refusal path is tested by nothing, and one mutant turns it off

Minted 2026-09-03 by calef, from milestone 244's (the largest crate in the tree is
proved by nothing a mutation can reach) recorded limitation. Built 2026-09-03 by the lane on
`milestone/246-measured-refusal`.

## What was built, and the falsification

The decision moved to `measured_boot::verdict`, beside `verify_in_manifest`, whose rule it
applies. `system_initializer::measured` is now one line: an archive read and a call. Nothing about
the boot path changed, and `script/swish-check` still proves that the boot makes the call.

**`Lookup` did not have to move, which is the `BUGS` clause below not firing.** The signature takes
`Option<&[u8]>` rather than a `nifefs::Fs`, so the crate gained no archive dependency and the caller
passes `fs.read(name)` straight through; and it returns the parsed image rather than the bytes, so
the caller is left with no struct literal of its own to get wrong. The one dependency added is `elf`,
which has none of its own and is already in every build that links `measured_boot`. The public
surface gained one struct (`Verdict`, the old private `Lookup` with its two fields) and one function.

The mutant, and it was run. `unvouched: true` changed to `false` in `verdict`'s refusal arm, by
hand, before the test was committed: `substituted_bytes_are_refused_and_reported_as_unvouched` fails
with *"a substituted program must be reported unvouched"*, and nothing else in the tree goes red.
Reverted.

Then the hand-run mutant was made a standing one, which is the ladder's rung two rather than
rung four. `cargo mutants`' only operator on a function returning a struct is to replace the body
with `Default::default()`; without a `Default` impl that does not compile, so the tool scored the
mutant unviable and reported nothing at all about this function. `Verdict` now derives `Default`,
which is the fail-safe value (no image means nothing runs) and is exactly the dangerous wrong answer
here, an absence where there was a refusal. `cargo mutants -p measured_boot --in-diff` over this
lane's diff: 1 mutant, 1 caught. Before the derive: 1 unviable, 0 tested.

Three tests, one per answer, because the three are not interchangeable and that distinction is
the interesting part of this function: substituted bytes (and a name the table never mentions, and
an empty table) are a refusal; an absent entry is not; and bytes the table vouches for that are not
an ELF report `unvouched: false`, because the measurement did its job and the packaging did not.
Putting a security word on a build defect would be the wrong answer there.

In brief. `system_initializer::measured` decides whether a component the system is about to start
was vouched for by the measured-boot manifest. It is the enforcement point of DECISIONS §104's
measurement, and no test in this tree ever takes its refusing branch.

```rust
if measured_boot::verify_in_manifest(table, name, bytes).is_err() {
    return Lookup { elf: None, unvouched: true };
}
```

Change that `true` to `false` and an unvouched binary starts. Nothing goes red. Milestone 244
measured that and recorded it beside the code: the function is *"proved only by `script/swish-check`
booting a system whose table happens to be right"*, which exercises the accept path and never the
refuse path.

## Why this is its own milestone rather than part of 244

244 measured the whole crate and correctly declined to lift it: 33 of 196 mutants are in pure
leaf functions, about sixty lines of 2,632, and moving them would put the boot path behind a feature
flag for a flattering score. That is the right answer for the aggregate.

It is the wrong lens for this one function, and separating them is the point of this block. The
other pure leaves are `hex_password`, `sentence`, `archive_name` and `opt_cap`: a mutant in any of
them produces a wrong string or a wrong option. A mutant here produces a system that runs code
nobody vouched for, which is the property `measured_boot` exists to provide. Counting those
mutants as interchangeable is what makes an aggregate fraction the wrong instrument.

## What makes it cheap, which is why it is worth doing now

`measured` takes a `nifefs::Fs`, a `&str` table and a `&str` name. All three of its dependencies
already compile for the host (`nifefs`, `measured_boot`, `elf`), and every one of them is already
mutated and covered. The function sits in an excluded crate because of its neighbors, not because of
anything it does.

## The proof that this milestone worked

A host test that fails when the refusal is removed. Concretely: a manifest and an archive that
disagree, asserting `unvouched` is set and no `Elf` is returned; and the mutant `unvouched: false`
turning it red. State the mutant that was run and that it was caught, the way milestone 193's block
states its falsification.

Not a test that only takes the accept path, which `script/swish-check` already does better.

## Follow-on

- **Milestone 250.** An unviable mutant is a hole in the measurement that reads as a pass. `cargo
  mutants` never mutated this milestone's own function: its only operator on a function returning a
  struct is `Default::default()`, `Verdict` had no `Default`, so the mutant did not compile and
  scored unviable rather than missed. `measured_boot` had nine such. calef minted milestone 250 from
  this handoff the same day. The proposal file this bullet used to name was promoted by milestone
  433 as `design/roadmap/354-the-mutants-nobody-counts.md` and superseded by 250 in the same act,
  because the two are one piece of work.
- **Recorded.** `components/src/login.rs` spells the same load-or-refuse decision itself and folds all three
  outcomes into `None`, so it cannot distinguish an absent program from a refused one. `verdict`'s
  signature already fits it; it was not switched because that is a boot path this milestone did not
  gate. Beside the code, in `components/src/login.rs`.
- **Recorded.** `system_initializer::measured` still exists and is still host-unreachable, and
  `cargo mutants` generates nothing for it. "No mutants generated" is a property of the tool rather
  than a proof. Beside the code, in `crates/system_initializer/src/lib.rs`.

## BUGS

- **This does not make `system_initializer` measurable**, and should not be quoted as if it did. 163
  of its 196 mutants stay unreachable by construction; milestone 244's block carries the split.
- **A refusal proved on the host is not a refusal proved on the boot path.** The host test asserts
  the function's contract; that the boot *calls* it, on both ISAs, with the table the kernel measured,
  stays `script/swish-check`'s claim and nothing here strengthens it.
- **`Lookup` did not have to move as a `nifefs`-shaped thing, but a struct did become public.**
  `measured_boot::Verdict` is the old private `Lookup`, and `measured_boot` gained a dependency on
  `elf`. That is the smallest version of the cost this clause anticipated, not none of it.
- `system_initializer::measured` still exists and is still unreachable by any host test. It is a
  delegation with no branch and no literal, so `cargo mutants` generates nothing for it, but "no
  mutants generated" is a property of the tool rather than a proof. What is proved is the decision it
  delegates to.
- The other refusal path in this tree was not touched. `components/src/login.rs` runs the identical
  `verify_in_manifest` over the caretaker blob it was handed and folds all three outcomes into
  `None`; it takes bytes rather than an archive, so `verdict`'s signature already fits it, and
  nothing here changed it. See the handoff in this lane's report.

## Index row

The decision is `measured_boot::verdict` now, with three host tests. Falsified by hand
(`unvouched: false` turns one red) and then made a standing mutant rather than a one-off check
