# Citations that name what they cite

A citation in this tree points into one of two numbered records: a decision (`§N`, in
`design/decisions/`) or a milestone (`milestone N`, in `design/roadmap/`). There are about 2,150 of
the first and 2,860 of the second, roughly a fifth of them in code comments. They are load-bearing:
the documentation is part of this project's deliverable, so a footnote that lands in the wrong place
is a defect in the product, not an untidiness.

Two gates already check that a citation **resolves**. `script/decisions --check` proves a cited `§N`
has an index row; `script/roadmap --check` proves a cited `milestone N` has a block under
`design/roadmap/` (it checked for an index row until milestone 294 generated that index from the
blocks). Neither can prove
it resolves to the thing its author meant, and both say so in their own headers.

`script/citations` is the third gate, and it reads the target.

## Why this needed a gate at all

Twenty-eight comments across `crates/`, `kernel/`, `notes/` and `user/` credited the two-tier `^C`
design to "milestone 24". Milestone 24 is *A second aarch64 board: Virtualization.framework*. The
`^C` design is DECISIONS §24.

It was never a renumbering. The first of them was written on 2026-07-28 and the roadmap already said
Virtualization.framework that day, so it was wrong at birth and spread by copy-paste. Both existing
gates reported clean the entire time, because both numbers exist.

The tell, once you know to look for it, is that several sites read `milestone 24, DECISIONS §24`:
one thing cited twice under two schemes. `milestone N` and `§N` are two numbering schemes over the
same integers, and they agree by accident for the early numbers (milestone 12 and §12 really are
both Call/Reply IPC) and diverge after. That is exactly the trap. Reading them as one scheme is
right often enough to survive.

## The convention

A citation may carry a gloss, and a gloss is checked.

```
milestone 24 (a second aarch64 board)
DECISIONS §24 (interrupting the foreground process)
```

Both are self-checking: the parenthetical can be compared against the record's own title, which is
the comparison neither existing gate can make. Write one and it binds.

Until 2026-09-19 the other half of that sentence was "write the citation without a gloss and nothing
new happens", and for everything already in the tree it still is. On a line a commit adds, it is
no longer true: see the ratchet below.

### The tolerance rule

A title is long (*"Interrupting the foreground process: two-tier, shell-held, no new kernel
surface"*) and a citation should be able to name a distinctive fragment rather than the whole thing.
A gloss passes on either of two tiers.

Tier 1, the title. Every content word of the gloss appears in the record's H1, order-free and
ignoring a short stopword list. A gloss abbreviates and reorders, so `§31 (the foreign-language
seam)` matches *"The foreign-language seam: C holds no capabilities and makes no policy"*, and
`milestone 55 (Time Machine over SMB3 with Apple's extensions)` matches a title that differs from it
by one preposition. 76 of the tree's 106 glosses pass here.

Tier 2, the quotation. The gloss appears as a contiguous phrase in the record's body, matched on
normalized whitespace. This is a different act rather than a weaker tier 1: `§4 ("a driver never
reaches into a kernel global")` quotes what a decision *says* rather than what it is *called*, and
no title check can accept it. Contiguous is deliberate. "Every word appears somewhere in a
5,000-word document" is not a test, it is a coincidence generator.

Three shapes are not glosses and are recognized rather than judged: a repo path (`milestone 15
(design/roadmap/15-asids.md)`), a cross-reference to the other record (`milestone 12 (§12)`), and a
bare date. A path is held to a *harder* standard than a gloss, because it is exact: it must exist,
and a path into a numbered record must carry the number that cited it.

### The diagnostic that matters

When a gloss fails, the script asks whether it matches the same number under the other scheme,
and says so when it does. That turns the symptom into the fix, and it is the defect this whole thing
was built for:

```
citations: notes/credentials.md:346: §49's gloss (users, login, and attribution) is not
    grounded in 49-removal-and-recursion.md
      §49 is: Removal is a directory operation, and `-r` widens the grant rather than...
      but it matches milestone 49: Users, login, and attribution: what identity is for...
      Two schemes, one number. This is the milestone-24 defect.
```

## The ratchet: a citation a commit adds says what it cites

`script/citations --ratchet`, in `script/lint` since 2026-09-19, on calef's ruling that day: *"add
the gloss ratchet."* It reads the **diff** against the merge base rather than the tree, so nothing
existing is touched and no lane is asked about text it did not write.

**The rule is per number per file, not per occurrence.** A `milestone N` or `§N` on an added line
passes if that number, under that scheme, is glossed *somewhere* in the same file: by title, by
quotation, by a path into the record, or by a `GLOSS_ALLOW` entry. A paragraph that names §151 four
times explains it once, and a gate asking for four glosses would be answered by deleting three
mentions.

A bare date and a bare cross-reference do not count as that one gloss, though `--check` recognizes
both and is right to. They are well-formed citations that do not say what the target *is*, which is
the only property this ratchet is about.

### Why a ratchet rather than the sweep this page refused

The BUGS entry below priced the sweep at 2,911 sites and refused it, on the ground that every one has
to be **read**: a pattern-applied gloss would make a wrong citation look verified, which is the
milestone-24 defect with better spelling. That argument is about a sweep and says nothing against a
rule on new text. The ratchet reads nothing retroactively and writes nothing mechanically.

What it buys is DECISIONS §151 (the goal of the repository split is independent release). A number is
one namespace over one tree, and it means nothing in a repository that does not hold the block; a
gloss is a string a reader or a `grep` resolves wherever the block ends up.

### What it cannot catch

- A citation that arrives in a merge commit. The diff is `base..HEAD`, and a merge brings in
  lines this branch never wrote. Deliberate: the alternative fires on a lane for another lane's text.
- A grounded gloss that is still the wrong citation, which is this page's standing BUGS entry and
  is not made smaller by the ratchet. It is made more *visible*: a wrong gloss is something a reader
  can see, and a bare number is not.
- A file that has already glossed the number keeps the escape forever, including for a later
  mention that means something else. That is the price of the per-file rule, and it is the right
  price: the reader has the name in front of them.
- A parenthetical that opens with the citation (`(milestone 298, notes/mdns.md)`) is not a gloss
  in this scheme and never was, because `CITE` looks for `milestone N (`. The ratchet asks such a
  site to be rephrased (`milestone 298 (retire the multicast DNS responder), notes/mdns.md`).
- Captured logs and the generated roadmap index are out of the corpus, with `vendor/`,
  `patches/` and lockfiles. Nobody wrote those lines as prose, and a fixture is the one thing a lane
  must not edit to satisfy a gate.
- **It reads `HEAD`, not the working tree**, because the line numbers in a `base..HEAD` diff are
  HEAD's and reporting them against an edited file would point at the wrong lines. So a fix has to be
  committed before this goes quiet. Since 2026-09-24 the tool enforces that: an uncommitted edit that
  adds or removes a citing line fails the run with "Commit first" instead of reporting on the old
  text (see the BUGS entry below).

### What it costs, measured before it was turned on

Over the 42 pull request merges from #953 to #1002, replaying each lane's own diff against its merge
base: a median of 16 asks per pull request, each answered by one parenthetical. The distribution
is skewed rather than flat: 8 of the 42 would have been silent, and the one outlier is #970 at 893,
which landed 24 proposal files at once. That number is the cost of writing 24 new documents that
cite the roadmap heavily, which is exactly the case the gate is for.

## The census: the population nothing had ever counted

`script/citations --census` counts the citations that carry no gloss at all, which `--check`
cannot see, because `--check` reads only the parentheticals that exist. The first run was 2026-09-19
and it corrected the framing of milestone 444 (a citation says what it cites)'s own brief:

```
$ script/citations --census
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

Read the last two columns first. 18,517 citations, in 1,447 files, and 505 of the 9,483
(file, scheme, number) pairs are named: 5.3%. The roadmap-after-the-split proposal reported the
tree as "83% split-proof by habit", and that number is true of the 560 citations that carry a gloss.
Over the whole population it is 5%. Both numbers are honest and they are about different sets; the
second is the one that prices the backfill, and nobody had it before.

The zero is the other finding, and it was re-checked before being believed. Not one cited number
in the tree fails to resolve to a block or a section. That is `script/roadmap --check` and
`script/decisions --check` doing exactly their job, for 18,517 citations. The check on the zero
matters because a `git grep` for citations does turn up six-digit numbers that resolve to nothing.
They are segment timestamps in the filenames under `bench/radon-2026-09-04/`, binary captures, and
they arrive through the trap `script/decisions` records, where `git grep` prints *"Binary file ...
matches"* into the same stream as the matches. The
census skips a file containing a NUL byte and excludes `*.log`, so the zero survived the re-check.

**The unit is the pair, not the occurrence**, because that is the unit of work: one gloss in a file
answers every mention of that number in it. `--census --list` prints the worklist, one line per pair,
which is how the backfill below chose what to do.

## The backfill, and the rule that chose it

Milestone 444 glossed 25 pairs across three files: `README.md`, `CONTRIBUTING.md` and
`SECURITY.md`. That is 0.3% of the backlog, chosen rather than sampled, and the rule is one
question.

Gloss where the citation is the reader's only route to what is meant. `SECURITY.md` is the
worked case and it is why those three files were picked: a person reporting a vulnerability met
*"(DECISIONS §20, §23, §30; `crates/dma_validator`, notes/iommu.md)"* and had no way to know what
those three sections claim without cloning the repository and opening three files. They now read
*"§20 (IOMMU-backed DMA isolation), §23 (multi-queue DMA confinement), §30 (the DMA boundary is
proved for descriptors)"*. These are the documents a stranger meets before they have a checkout,
which is principle 3 (a newcomer must be able to succeed without asking anyone) applied to a
footnote.

**Do not gloss where the surrounding text already says what the target is.** Two cases were looked
at and deliberately left:

- `notes/scripts.md`, 50 pairs. Its citations sit in a table whose row already describes the
  script. The `script/icount` row says it is the instruction-count instrument and what it boots, and
  the parenthetical records only which milestone built it. That number is provenance, not a pointer,
  and a gloss would add a clause to a dense table row saying what the next clause already says.
- `notes/README.md`, 128 pairs. The notes index, same shape: each row links the note and
  describes it, and the milestone number records which milestone wrote it.

**And do not gloss a dated account, a quotation, or a block's own history**, which is
`design/naming.md`'s rule that a dated record keeps its words. A gloss inserted into a sentence
written in August rewrites what that sentence said.

The remainder, 8,953 pairs, is recorded as work in milestone 444's block rather than carried as an
intention: `script/citations --census --list` regenerates the worklist at any time, so nothing about
it has to be kept in a note that will go stale.

## --moved: when a status flips, who was citing that milestone

Milestone 385 (when a milestone's status flips, tell the lane which notes cite it) is the fourth
mode and the only one that cannot fail. The other three say something is wrong. This one says
something might be, and it cannot tell which, so it prints a worklist and returns zero.

The failure it answers has a measured shape. Six "is there a milestone for X" questions in one
evening turned up four things wrong on `main` rather than four things missing, all the same: prose
that was true when written and went silently false when a milestone landed.
`notes/why-not-general-purpose.md` told newcomers there was no networking, no writable filesystem,
no display and no SMP, five of six rows false. Milestone 66 (Vaultwarden: somebody else's real
application, running here)'s gap table said TCP listen and accept were absent from the contract a
month after milestone 107 (the socket contract learns to accept) shipped them. Every one stayed
green through every gate.

```
$ script/citations --moved 5f850c367~1..5f850c367
citations: milestone 525 moved NEW -> BUILT: A bad upgrade cannot brick the machine: two boot slots, tries an
citations:   design/decisions/207-the-roadmap-is-a-graph-and-says-so.md:50
citations:   design/roadmap/554-a-good-upgrade-sticks.md:10
```

Two files, and the second is the real one: milestone 554 (a good upgrade sticks: what marks a trial
boot successful)'s block still described 525 as the proposal it had been.

### Why it tells rather than obliges

DECISIONS §207 (the roadmap is a graph, and the block says so in fields a script can walk) ruled on
this exact coupling from the other direction, and cites milestone 385 by number as its evidence.
§207 retired `Gate: MILESTONE N` because the rule obliged an edit in every dependent block when
a dependency landed, made by somebody in another lane who was not looking, so a milestone being
satisfied turned unrelated branches red on a line they never touched.

What §207 removed is an obligation to *edit* on a status flip. `--moved` is a duty to *tell*
somebody on a status flip, addressed to the one lane already in the file. It asks for no edit, it
is read and dismissed in a sentence, and no other branch goes red. A version of this that demanded
edits would reintroduce what §207 refused three weeks after it was decided, which is why it reports.

### The four measurements that decided the design

1. **A worklist is three files, not thirty.** Across `notes/`, 194 milestones are cited by at least
   one note, median 3 and mean 4.4. Three files is a thing a lane reads.
2. **It fires per branch, not over the tree.** Sixty-eight rows flipped to `BUILT` in fourteen days.
   Tree-wide that is fifteen note-reads a day owned by nobody, which is how a check becomes noise.
3. It keys on the bare `milestone N`. Only 4% of milestone citations in `notes/` carry a gloss,
   so reading the glossed form `--check` reads would find one citation in twenty-five. It uses the
   same regex `--census` uses, for the same reason.
4. It reads the block file and nothing else. 385's own `BUGS` feared a status moving without the
   pull request touching the index, and asked the implementation to read both. There is no longer a
   both: `design/roadmap/README.md` was generated from the blocks by
   milestone 294 (`design/roadmap/README.md`'s index is generated, not hand-maintained) and retired
   on 2026-09-21, so the `Status:` line in the block is the single record.

### What it costs, measured over history

Over the 150 most recent merges into `main` on 2026-09-23, **50 moved at least one status and the
other 100 printed a single line saying nothing moved. A lane that edits its own block without
changing its status, which `script/lint` 4b makes every lane do, sees that line and nothing else.

Of the 50, the median is 4 citing files. Those 50 merges hold 262 individual flips, and per
flip the median is **1** citing file and the mean 3.4.

The tail is real and is worth naming rather than averaging away, because it is where a reader would
decide this is noise:

| merge | flips | citing files |
|---|---|---|
| #970, milestone 433 (drain the proposal pile to zero, and keep it there) | 113 | 363 |
| #1018, milestone 448 (a refusal gets a number, a status, and a condition that would change it) | 43 | 103 |
| #992, milestone 161 (the x86_64 kernel port: bring up the HAL's third architecture) | 1 | 48 |

The first two are sweeps that added or restatused dozens of blocks at once, which is the same
outlier the ratchet's own measurement found in #970. The third is one flip on a heavily cited
number. The worst single milestone in the window is 433 at 108 citing files, for the same reason:
a milestone that reorganized the roadmap is cited by the roadmap.

## Attributed quotations

The second half. A block quote may name the file it came from, and the passage must still be there:

```
> Kernel memory is never demand-paged. Kernel pages are mapped eagerly.
>
> -- design/decisions/09-irq-safe-locking.md
```

Matching is on normalized whitespace, which is the only reason this is affordable. A note wraps
at 100 columns and a block quote of it wraps at 96 after the `> ` prefix, so byte equality would
fail on every correct quote in the tree and the check would be thrown away inside a week. An
ellipsis (`...`) splits the quote into segments that must appear in order, so a quote may skip the
middle of a paragraph.

### Why it is worth the false-positive cost, argued rather than assumed

The obvious objection is reflow: rewrap the source and every quote of it breaks. That objection is
answered by the normalization, and what remains is the case where the source's **words** changed,
which is precisely what the check is for.

The defect it answers is real and recent. `design/roadmap/74-cycle-counters.md` carried a block
quote attributed to `notes/benchmarks.md` (*"At ~3.2 GHz, 705 ns is ~2,200 cycles round trip... we
are 4 to 7 times heavier"*). Milestone 101 re-measured and retracted that arithmetic; the paragraph
no longer exists in the note. The roadmap was quoting a retraction as the current record, and
nothing could see it, because a prose block quote attributed to another file is a citation that no
gate resolves.

So: yes, verify it. The cost is bounded by the normalization and the payoff is a class of rot that
is otherwise invisible.

## EXAMPLES

Report every citation, classified:

```
$ script/citations
73 decisions, 115 milestones

GLOSSED CITATIONS (106)
  cross-reference  6
  date             1
  exempt           1
  path             13
  quotation        9
  title            76

ATTRIBUTED QUOTATIONS (1)
```

Gate it, which is what `script/lint` runs:

```
$ script/citations --check
citations: 106 glossed citations, all grounded (76 by title, 9 by quotation, 13 by path,
    6 cross-references, 1 dates, 1 exempt)
citations: 1 attributed quotations, all still present in their sources
```

The breakdown sums to the total on purpose. A summary line whose parts do not add up is a number
nobody can check, which is the defect `script/lint`'s shellcheck line was fixed for.

Ask what this branch adds, which is the second thing `script/lint` runs:

```
$ script/citations --ratchet
citations: every citation on the 121 lines this branch adds says what it cites
```

The proof that it fires, run on 2026-09-19 by planting one line in this very file and committing
it. The planted line cited three records, and the interesting part is which two were reported:

```
$ script/citations --ratchet
citations: notes/citations.md:296: milestone 444 is cited with no gloss anywhere in this file
      milestone 444 is: A citation says what it cites, so a number stops being the identity
citations: notes/citations.md:296: §46 is cited with no gloss anywhere in this file
      §46 is: Thin primitives or whole subsystems; we write everything in between
```

The third, milestone 55, was not reported, because this page already carries
`milestone 55 (Time Machine over SMB3 with Apple's extensions)` in its tolerance-rule section. That
is the per-number-per-file rule working, and it is the whole difference between a ratchet and a tax.
The line was then removed and the same command went quiet.

Judge a range that is not this checkout's HEAD, which is how the cost above was measured:

```
$ script/citations --ratchet "$(git merge-base $pr^1 $pr^2)..$pr^2"
```

Citing the decision rather than the milestone, in a code comment:

```rust
/// Granted a per-job interrupt channel so `^C` can reach it (DECISIONS §24). A
```

Quoting a decision's own words, which tier 1 cannot accept and tier 2 can:

```markdown
DECISIONS §19 (parity is a gate, not an aspiration) applied to the toolchain.
```

Note that §19's *title* says parity is a **tenet**. The body says it is a gate. Two sites in the
tree glossed it as "architectural parity is a gate", which is neither, and both now quote the body:

> **Parity is a gate, not an aspiration.** A kernel capability ships on every supported
>
> -- design/decisions/19-architectural-parity.md

That block is not an illustration. It is the tree's one live attributed quotation, and `script/lint`
re-resolves it against `19-architectural-parity.md` on every run; edit that sentence in the decision
and this page fails the build until it is brought back into agreement.

## BUGS

The scanner sees across one line break, and only one. Until milestone 583 (`script/citations`
could not see a citation a line break split, or a lettered milestone) the gap between a number and
its `(` had to be a single literal space, so reflowing a paragraph until the wrap fell between
a citation of milestone 326 (nobody has been assigned to turn a mutation score upward) and its `(` removed the citation from the gate's view without failing anything. Three
lanes hit that in one week. A closing emphasis marker is part of the gap too, for the same
reason. This tree bolds the number and leaves the gloss plain:

```
**milestone 41** (dead code: triage the suppressions)
```

so the `` sat where nothing was allowed to be, hiding thirteen sites across ten files. A citation split across *two* line breaks is
still invisible, because
the one-newline cap is what stops a stray `(` on line 40 pairing with a `)` on line 900, and nothing
in the tree wraps that way today. `script/citations --selftest` pins both the shapes that must be
seen and the near-misses that must stay quiet, and it runs in `script/lint` ahead of `--check`: a
green check looks identical whether the scanner works or has quietly stopped seeing a shape.

**A lettered citation is glossed against its parent unless the letter has a file. milestone 20a (name the seams)
is read against `design/roadmap/20a-name-the-seams.md`, which is a block of its own. The letters
with no file (7a, 9a, 16a, 16b, and milestone 19 (run a real workload)'s 19a through 19f) are
read against their parent block, which is
where those sub-parts are actually described. The ratchet's per-file key drops the letter either
way, so a file that glosses milestone 19 has answered for milestone 19d as well; that matches
the per-number rule the ratchet is built on, and it means a sub-part can ride on its parent's gloss.

The candidate pre-filter is deliberately looser than the scanner. It accepts a line that *ends*
in a citation, because that is what a wrapped one looks like to a line-based `git grep`, which
widens the file list from 649 to 853 and costs about a second. Tightening it for speed is how the
gate goes blind again, and the selftest is what would notice.

**A possessive gloss is not recognized, although the tree is full of them.** The gate accepts
`milestone 128 (the automation gets its own identity, and the agents get their own voice)` and
rejects `milestone 128's (the automation gets its own identity, ...)`, reporting the glossed line
itself as "cited with no gloss anywhere in this file". The possessive form is ordinary English and
already appears in the tree: milestone 218 (every boot of the VisionFive 2 needs a human typing four
commands into U-Boot) is cited in exactly that possessive shape in `design/roadmap/`, among others,
and those files pass only because they predate the ratchet. This very entry tripped the gate while
being written, on the line above, which is as good a demonstration as the entry could ask for. Found 2026-09-23 by the `maintainer/what-the-machinery-did` lane, which spent three commits
on it: the failure names the right line and gives no hint that the apostrophe is what broke it, so
the obvious response is to reword the gloss rather than to move it. The workaround is to rewrite
the sentence so the number is followed directly by its parenthesis, which is what that lane did.
Either the matcher should allow `'s` before the gloss, or the message should say which form it
wants.

**`--moved` cannot tell staleness from correctness, and it never will.** A note saying that
milestone 30 (the network stack as a confined component) built the net stack is right forever, and
it is listed every time 30's block is touched. The output is a prompt to look. If looking is
usually wasted the mode will be skipped, and that, rather than a false negative, is the failure mode
to watch for.

`--moved` misses the worse half of the class it was raised for, and the miss is inherent. A page
saying "there is no networking" cites nothing, because a negative claim has no anchor to hang a
check on. That is the shape of the worst of the four instances that prompted milestone 385 (when a
milestone's status flips, tell the lane which notes cite it), and it is not detectable without
telling an assertion from an observation in prose, which this tree has already priced: `git grep -w
TODO` ran at an 82% false-positive rate. The other half wants a sweep rather than a gate, and
milestone 259 (sweep `notes/` for claims that stopped being true) is that sweep.

**`--moved` sees a citation in prose and not one in code.** It walks `notes/` and `design/` only. A
`milestone N` in a Rust comment is usually provenance for the code beneath it rather than a claim
about what the tree can do today, so including `*.rs` would multiply the worklist by the population
`--census` counts without adding a case of the defect. Untested rather than proved: nobody has
measured how often a code comment makes a present-tense claim about a milestone.

**A block deleted or renumbered is not reported by `--moved`.** The mode reads the status at the tip
and skips a file that is gone, so a number retired out from under its citers prints nothing here.
That case is already a hard failure elsewhere: `script/roadmap --check` fails a `milestone N` that
resolves to no block at all.

An untracked file is invisible to this check, and a green run says nothing about it. The walk is
`git ls-files "*.md"`, so a file that has been written but not yet `git add`ed is not in the corpus:
the check reads the tracked tree, reports honestly about it, and exits 0. Found on 2026-09-19, the
expensive way. A maintainer wrote a new roadmap block containing a block quote that ended
mid-sentence, ran `script/citations --check`, read exit 0, committed and pushed. The quote had never
been read. The same run after the commit failed on it, and a lane that had branched from the pushed
commit reported the gate red on its own base.

It is the pattern this tree keeps meeting, which is an absent failure signal read as a pass, and
the same shape as `script/decisions` silently skipping a binary file and `script/roadmap`'s
merged-branch check spending a day unable to fail. Nothing here is wrong: a check over the tracked
tree is the right corpus, since that is what a reader clones. What is wrong is reading its exit code
as a statement about the working directory.

The habit that fixes it costs nothing: stage before you check. `git add -A` and then run the
gate, or run it again after committing and before pushing. `script/lint` does not have this hole for
the same files because it is invoked on a committed tree in CI, which is why the defect survives
locally and not on a pull request.

A gloss is optional, so an unglossed citation is checked by nothing here. This is the honest
limit and it was a deliberate choice, measured rather than assumed. Requiring a gloss on the first
mention of each number in each file means 2,911 sites, every one of which has to be read to know
what its author meant (that is the whole premise; a pattern-applied gloss would be a confident
falsehood next to every wrong number). That is not a sweep this project can do correctly in one
pass, and a mechanically-inserted gloss would make the 28 milestone-24 defects *look* verified. So
the rule is "what you write is checked", not "you must write it". The number is recorded here so
that whoever revisits it is arguing against a measurement.

That measurement was itself too small, and the census above replaced it on 2026-09-19. 2,911 was
first mentions counted one way; the census counts 9,483 (file, scheme, number) pairs, of which
8,978 carry no gloss, over 18,517 citations. The conclusion does not move, it gets stronger: a
sweep of that size cannot be done correctly in one pass, and it is why milestone 444 shipped a
ratchet plus 25 hand-read glosses rather than a rewrite.

**The ratchet does not close this entry, it stops it growing.** An unglossed citation already in the
tree is still checked by nothing, and the escape in the per-file rule means a file that glosses a
number once satisfies the ratchet for every other mention in it forever.

A `--ratchet` replay of an old commit judges it against today's records. The decision and
milestone tables are loaded from the working tree, not from the revision under test, so a range
replayed over history can report a number that had no block at the time, or ground a gloss in a title
that has since been rewritten. That is harmless for the live check, where the tree and HEAD are the
same thing, and it is worth knowing before quoting a replayed number as history.

A wrong citation whose gloss is also wrong in the same direction passes. The first of these
grounds in nothing and fails; the second grounds fine and is still in the wrong place if it sits
next to interrupt code:

```
milestone 24 (the ^C work)
milestone 24 (a second aarch64 board)
```

The check proves the gloss and the number agree, never that either matches the surrounding prose.
(Writing this paragraph is what proved the fence exemption above is load-bearing: the first line
failed the gate until it went inside a fence.)

Tier 2 is only as strong as the target document is short. A long decision file contains many
phrases, so a wrong-but-plausible quotation of one has more room to land in another. Nothing in the
tree does this today; it is the direction the rule is weakest in.

A gloss may span at most one line break. Long glosses in Rust doc comments wrap, and the
scanner joins one continuation. Two is not read, and a three-line gloss is invisible. The cap exists
because an unbounded one lets a stray `(` on line 40 pair with a `)` on line 900 and swallow the
file. This is a real limit, not a theoretical one: the line-based first draft of this script read
**zero** wrapped glosses and missed one of the milestone-24 defects for exactly that reason.

Fenced code blocks in markdown are skipped entirely. A page documenting this convention has to
be able to show a wrong citation, and this page does. The cost is that a genuine citation inside a
fence is unchecked. `script/lint`'s TODO gate exempts *all* of markdown for the same reason, so this
is the narrower version of a carve-out the tree already makes.

Lettered milestones resolve to their base number. `milestone 19e` is checked against milestone
19's file, because there is no `19e` file. A gloss describing what 19e specifically did will not be
found in 19's text and has to be phrased as commentary instead.

**A file whose only glossed citations are lettered is not scanned at all**, which is a stronger
statement than the entry above and was found by accident on 2026-08-16. The `git grep` that picks
candidate files matches `[Mm]ilestone [0-9]+,? \(`, with no `[a-z]?` where the whole-file regex has
one, so a lettered citation never puts a file on the list:

```
Milestone 16b (DECISIONS §20, notes/iommu.md)
```

Three files are invisible to the gate for this reason today (`design/decisions/26-fault-endpoint.md`
and both `iommu.rs` files), and every citation in them, lettered or not, goes unchecked.
`notes/trusted-init.md` was the fourth until milestone 94's blessing lane added an ordinary citation
to it, which pulled the file onto the list and surfaced a lettered gloss that had never been read:
the gate reported a defect on line 31 in response to an edit on line 167, which is how the blind
spot was noticed at all. That one was fixed in the same lane; the other three were left, because
fixing the pattern is one character and then four sites need rewording or an allowlist entry, which
wants a lane rather than a drive-by. The general shape is worth more than the bug: when a
checker chooses its inputs with one pattern and reads them with another, the gap between the two is
unreachable by any test that only runs the checker.

Nothing checks `design/decisions/`'s index rows' own titles against the files they link to,
which is `script/decisions`' stated non-goal (a row may abbreviate). A row that abbreviates
*wrongly* is still invisible. The roadmap left this class on 2026-09-14: milestone 294 generates its
index from the blocks, so a roadmap row's title is its block's H1 by construction, and the migration
found five rows that had drifted.

## What it found on the first run

- **28** comments crediting the `^C` work to milestone 24 rather than §24.
- **31** glosses that could not be grounded, of which 6 were the wrong record (two `§49` for
  milestone 49, one `§51` for milestone 51, one `milestone 6` for `§6`, one `milestone 8` that is
  not a filesystem, and one roadmap index row repeating the `§51` error), and the rest were
  paraphrases or commentary that read as a gloss.
- One stale `design/roadmap.md` path, which has not existed since milestone 76 split the roadmap
  into a directory.
- Two claims a decision does not make: `§32` glossed as `Endpoint::REAP`, a method name §32 never
  spells (it reaches reaping through §16's `Untyped::DESTROY`), in two places.

## The same shape in another medium

Worth naming, because it says what the general defect is and this gate only covers part of it.

Milestone 112 (the SAFETY comments that bind nobody) found a `// SAFETY:` comment in
`components/src/net_transport.rs` whose paragraph described `invoke` and capability validation, sitting
above a `write_volatile` into a DMA page. It described a different operation entirely, and it passed
`clippy::undocumented_unsafe_blocks` for as long as the file existed, because that lint asks whether
a comment is *present*, never whether it is *about the thing underneath it*.

That is the same failure as `milestone 24` resolving to a board: a reference that is well-formed and
wrong, sitting under a check that only asks whether it resolves at all. **A comment that names its
subject can be compared against it; one that does not is unverifiable by construction.** Every gate
in this file is an instance of that one idea.

SAFETY comments are milestone 112's territory and not this script's, deliberately. The point of
recording it here is that the citation gate is one member of a family, and whoever adds the next
member should recognize the shape rather than rediscover it.

**The ratchet reads the committed tip, not the working tree, and it now refuses to pretend
otherwise.** `--ratchet` diffs against the branch's base and reads each file with `git show`, so a
gloss you have just typed and not committed is invisible to it. Found on 2026-09-20 by a maintainer
who fixed three unglossed citations, re-ran the ratchet, and read the same three failures back; a
one-line "Commit first." note was added, and on 2026-09-24 it was missed beside a failure, the
uncommitted fix was right, and CI failed on the committed line. So since 2026-09-24 a file whose
uncommitted edits add or remove a line citing a real `milestone N` or `§N` (or an untracked file
containing one) fails the run, names the file, and checks nothing else: any report would be about
text you are no longer looking at. Commit, then run it. Dirty files whose edits cite nothing
still get the one-line note and the verdict stands, which is what keeps `script/lint` usable on a
work-in-progress tree. The scan is looser than the ratchet (it reads raw diff lines, fences and all),
so it can ask for a commit that would not have changed the verdict; that is the cheap direction to be
wrong in. Checking the working tree instead was refused because it would make a local pass mean
something different from a CI pass, which only ever sees commits.

A gloss must sit on the same line as the number it explains. The parenthetical is matched near
its citation, so a citation at the end of a line whose gloss wraps onto the next one is read as
unglossed, and the fix is to rewrap rather than to reword. This is the price of matching by
proximity rather than by parsing prose, it is cheap to pay once you know, and knowing is what this
entry is for. It bites hardest in a long table row and in markdown wrapped at 100 columns, which is
most of this tree.

A tool that reads only stdout can report this gate as clean while it is failing. `script/citations`
writes some findings to stderr, so a script that captures `stdout` alone and looks for
`cited with no gloss` sees nothing and concludes the tree is clean. A maintainer's gloss-fixing loop
did exactly that on 2026-09-20: it printed `clean`, and `script/lint` failed on the same tree
seconds later. This is the same family as the entries above and the fix is one flag: capture
both streams, or check the exit code rather than the text. The exit code is right; it was the
transcript that lied.

## See also

- [Naming things](../design/naming.md): why `§N` and `milestone N` are different numbers over the same
  integers.
- [The `script/` entry points](scripts.md): where this sits among the gates.
- `script/decisions` and `script/roadmap`: the two gates that check a citation resolves, and whose
  headers name this blind spot.
