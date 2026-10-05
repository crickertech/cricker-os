---
status: BUILT
raised: 2026-09-19
built: 2026-09-19
---
# 444. A citation says what it cites, so a number stops being the identity

Minted 2026-09-19 by the maintainer, on calef's ruling the same day:
*"add the gloss ratchet and launch a lane to address the 17% that fail today."* Built the same day.
*(Number provisional until the merge queue lands it.)*

## What calef ruled, and one correction to the framing

Ruled: a `script/lint` ratchet may require a gloss on a `milestone N` or `§N` citation that
appears on a line a commit **adds**. Existing citations are not touched by the ratchet itself.

The correction, because the number in the ask means something else. `script/citations` resolves
560 **glossed** citations: 465 by the target's own title, 29 by quotation, 24 by path, 12 as
cross-references, 29 by date, 1 exempt. The 95 that do not resolve by title are not failures; a path
citation and a dated one are honest and checkable. **The real backlog is citations with no gloss at
all**, which the checker cannot verify in either direction, and the population is much larger than
560: `git grep` counts 3,697 `milestone N` and 1,953 `§N` occurrences in `*.rs` alone.
Measure that properly before editing anything.

## Why it matters, and it is the split rather than tidiness

DECISIONS §151 (the goal of the repository split is independent release) rules that the repository
becomes several. A number is one global namespace and stops meaning anything outside its
repository; a glossed citation resolves by searching for the title, wherever the block ends up. The
roadmap-after-the-split proposal measured that the tree is already
83% split-proof by habit (465 of 560) and that nobody decided to make it so. A gloss is
therefore the cheap half of an expensive decision that is still open: what a citation's identity
becomes after the split.

## What was built

1. The ratchet. `script/citations --ratchet`, run by `script/lint` as *"new citations say what
they cite"*. It reads the diff against `git merge-base HEAD origin/main`, never the tree, so a lane
is asked only about text it wrote. The rule is **per number per file, not per occurrence**: a
citation on an added line passes if that number, under that scheme, is glossed somewhere in the same
file, by title, by quotation, by a path into the record, or by an allowlist entry. A paragraph that
names §151 four times explains it once.

*Proved both directions before it was wired in.* Silent on the clean lane; planted one line citing
three records and committed it, and the run named exactly two of them, staying quiet about the
third because the planted file already glossed that one somewhere else. That transcript is in
notes/citations.md's EXAMPLES, and the planted line was removed in the next commit.

*Priced before it was turned on.* Replaying each lane's own diff for the 42 pull request merges from
#953 to #1002: a median of 16 asks per pull request, 8 of the 42 silent, one outlier of 893
(#970, which landed 24 proposal files at once).

2. The measurement, `script/citations --census`, so the number is re-derivable rather than prose
that rots:

```
                    files  citations   named  unnamed  no record
roadmap proposals      23        196       5      101          0
roadmap blocks        439       5224     212     2351          0
decisions             188       1944      66      984          0
design/ (other)        25        444      38      224          0
notes/                175       3408     131     1660          0
Rust                  395       5867      33     2738          0
scripts                64        716      12      399          0
manifests              55        370       7      260          0
CI workflows           13         64       0       44          0
everything else        70        284       1      217          0
total                1447      18517     505     8978          0
```

18,517 citations in 1,447 files, and 505 of the 9,483 (file, number) pairs carry a gloss: 5.3%.
The "83% split-proof" figure above is true of the 560 citations that have a gloss; over the whole
population it is 5%. Both are honest and they are about different sets, and the second is the one
that prices the backfill. Nobody had it before: this is the first count of the citations that carry
nothing.

The zero in the last column is the other finding, and it was re-checked rather than believed.
Not one cited number in the tree fails to resolve, which is `script/roadmap --check` and
`script/decisions --check` doing their job across 18,517 sites. The re-check mattered because a
`git grep` for citations turns up six-digit numbers: they are inside `bench/radon-2026-09-04/*.log`,
binary captures, and they are the *"Binary file ... matches"* trap `script/decisions` records.

3. A backfill of 25 pairs, in `README.md`, `CONTRIBUTING.md` and `SECURITY.md`: 0.3% of the
backlog, chosen rather than sampled. The rule, recorded in notes/citations.md, is *gloss where the
citation is the reader's only route to what is meant*. `SECURITY.md` is the case that earned it: a
person reporting a DMA escape met a parenthetical naming three decision numbers, a crate and a
note, with no way to learn what those three sections claim without cloning the repository. They now
name the IOMMU-backed isolation, the multi-queue confinement and the descriptor proof in the
sentence itself.

Left deliberately: `notes/scripts.md` (50) and `notes/README.md` (128), where each row already
describes the target and the number is provenance rather than a pointer, and every dated account,
where `design/naming.md`'s rule that a dated record keeps its words applies.

## BUGS

- A grounded gloss can still be the wrong citation. The check proves the words match the target,
  not that the target is the right one to cite; only a reader catches that.
- The ratchet bites the honest case too: a lane adding a line to a dated account has to gloss a
  citation that the surrounding passage already explains. Whether that is an exemption or an
  accepted cost is the first thing to answer if it turns out to fire often. The median of 16 is the
  number to argue against.
- A citation in a merge commit is invisible to it, deliberately, since the alternative fires on
  a lane for another lane's text.
- The per-file escape is permanent. One gloss in a file satisfies the ratchet for every other
  mention of that number in it, forever, including a later one that means something else.
- Quoting another file's bare citation counts as adding one, because the ratchet reads text
  rather than intent. Reproducing a table row that names a milestone in parentheses is an added
  citation with no gloss, and the ask lands on the lane doing the quoting. Describing the quote
  instead of reproducing it is the cheap answer, and both this block and notes/citations.md took
  it rather than editing a quotation.
- **The 8,953 unglossed pairs that remain are not scheduled.** They are re-derivable at any time
  with `script/citations --census --list`, which is why no list of them is written down anywhere.

## Follow-on

- **Recorded.** *The 8,953 unglossed pairs the backfill did not touch*, in `notes/citations.md`'s
  BUGS section beside the convention they belong to, with `script/citations --census --list` as the
  worklist so no copy of it can go stale. A second slice has to be chosen by a reader rather than by
  a count: a mechanically inserted title would make a wrong citation look verified, which is the
  defect this whole family of gates exists for.
- **Recorded.** *What a citation's identity becomes after the split*, in the roadmap-after-the-split
  proposal, which holds the options and the measurement. The ratchet does not prejudge it; it makes
  the cheap half true in advance, at the rate the tree changes.
- **Recorded.** *The ratchet's cost on an honest dated account*, in `notes/citations.md` under what
  it cannot catch. The median of 16 asks per pull request is the number to argue against if it turns
  out to want an exemption.

## Index row

calef ruled on 2026-09-19 that a citation added by a commit must say what it cites. The ratchet is
`script/citations --ratchet` in `script/lint`, per number per file, read from the diff: a median of
16 asks per pull request over the 42 merges before it. Its census took the count nobody had, 18,517
citations of which 5.3% carry a gloss, and 25 of them were backfilled where a stranger meets them,
in `README.md`, `CONTRIBUTING.md` and `SECURITY.md`. A number stops being an identity when the
repository becomes several (§151); a gloss survives the move, and this does not prejudge which
identity wins.
