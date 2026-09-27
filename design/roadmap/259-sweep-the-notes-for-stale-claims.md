---
status: BUILT
raised: 2026-09-05
built: 2026-09-05
---
# 259. Sweep `notes/` for claims that stopped being true, because the gate cannot see them

Built (2026-09-05). Minted 2026-09-05 by calef, as the other half of milestone 385,
`design/roadmap/385-a-note-that-cites-a-milestone-that-moved.md`, which catches the notes that
cite a milestone and says plainly that the notes which cite nothing are the worse half.
*(Number provisional until the merge queue lands it.)*

## Why a sweep rather than a check

**A page that says "there is no networking" cites nothing, because there is nothing to cite.** That
is the shape of the worst instance found so far, and it is inherent: a negative claim has no anchor,
so no citation gate can hang off it. Detecting negative claims in prose is the `git grep -w TODO`
experiment again, which AGENTS.md prices at an **82% false-positive rate**.

**But a signal useless as a gate is fine as a worklist**, and that is the whole reason this is a
milestone and not a lint. 82% false positives fails rung two and is perfectly serviceable for
ranking 175 files a person is going to read. The heuristic the check must refuse is the heuristic
this sweep should use.

**The precedent is milestone 247** (follow-on work named by a finished milestone goes nowhere),
which swept 139 finished blocks for buried work and produced the `## Follow-on` convention. This is
that exercise aimed at `notes/`.

## What went wrong, so the sweep knows what it is looking for

`notes/why-not-general-purpose.md`, rewritten 2026-09-05, is the worked example and every property
of it is worth carrying:

- **Five of six gap rows were false.** It claimed no networking, no writable filesystem, no display
  and single-core, and that *"you cannot drop in existing software; every program is hand-written
  against our ABI"*. Milestone 121 ran unmodified `ripgrep` with zero patches on 2026-08-31.
- **It cited none of the milestones that falsified it.** Milestone 7 and §4, §5, §10, §11; not 30,
  33, 57 or 121. So the citation check would not have caught it.
- **It was built on the retired framing.** It grounded itself by quoting the project's goal as
  *"understanding how operating systems work, not running applications"*, which AGENTS.md names
  specifically and says to treat as stale wherever found.
- **It asked to be updated and was not.** Its closing line said to add to it *"when a subsystem that
  would move the needle (a writable FS, a POSIX shim, a net stack) actually gets built."* Two of the
  three were built.

## The distinction that scopes this, and it is not age

**Measured 2026-09-05: 175 notes, 62,993 lines. Median 12 days since last touch, 44 untouched for
21 days or more, 26 for 30 or more.** An age-ranked sweep would start with `notes/mmu.md`,
`registers.md`, `uart.md`, `page-tables.md` and `higher-half.md`, all 54 to 55 days old.

**Every one of those is fine, and reading them first would waste the sweep.** They explain how an
MMU works, what a system register is, how a page table is walked. That machinery does not change
when a milestone lands.

**The split is what a note makes claims about, not when it was written:**

- **Concept notes** explain hardware, an algorithm, or a specification. They go stale only when the
  world does, which is rarely and visibly.
- **State notes** describe what nife currently is, has, lacks, or measures. **They go stale by
  construction**, because the system changes daily, and they are where every instance found so far
  has lived.

A note can be both, which is the interesting case: a concept note with a paragraph saying "we do not
do this yet" carries a state claim inside an otherwise durable page, and that sentence is exactly
what nobody rereads.

## How to run it

**A classification pass first, then deep reads.** Read every note's title and opening paragraphs,
which is cheap over 175 files, and sort them into concept, state, and both. Then read the state ones
against the tree. Do not read 63,000 lines in order.

**Rank the deep reads by how load-bearing the claims are**, not by age. A note a newcomer meets
early, or one that tells a reader what the system cannot do, outranks a note about a subsystem's
internals, because AGENTS.md's third principle is that a stranger must reach a correct mental model
without asking anyone.

**Check claims against the tree, not against memory.** Every stale claim found so far was found by
somebody looking at the code or the roadmap, and this session put three wrong citations into one
decision file by trusting recall.

**Fix in place, and keep what was wrong visible where it is instructive.** The
`why-not-general-purpose.md` rewrite kept the old claims as a "Then" column, because how fast they
went stale is itself worth seeing. That is a judgment per note rather than a rule.

## The proof that this milestone worked

**A written count**: notes read, notes found stale, claims corrected, and the ones deliberately left
alone with the reason. A sweep that reports only what it fixed hides its own coverage, which is the
number a future reader needs to know whether to run it again.

**And at least one recurring shape named.** 247's value was not the 139 blocks it read, it was the
`## Follow-on` convention that came out of them. If this sweep finds that stale claims cluster in one
kind of note or one kind of sentence, that finding is worth more than the corrections.

## What the sweep read, and what it found

**175 notes, 62,993 lines.** Classified by reading every title and opening, per the plan above.

| | count | |
|---|---|---|
| **concept** | 46 | explain hardware, an algorithm or a spec, so they go stale only when the world does |
| **state** | 129 | describe what nife is, has, lacks or measures |
| deep-read against the tree | 71 | every state note the ranking reached, plus 22 concept notes that turned out to carry a state paragraph |
| **found stale** | 66 | 64 notes plus `fuzz/seeds/README.md` and one deleted test file |
| left alone, deliberately | 109 | correct as written, or held by another lane, or historical by design |

**The corrections, by kind.** 262 dead citations (a crate, a file or a Rust path renamed away); 8
capability claims falsified by a milestone (single core, revocation, `|`, real hardware, the std
gap tables); 6 rows of a ranked gap list that a milestone had closed; 4 conditions under a security
audit; 3 first-light cells; 1 restored test.

**Held by another lane and reported rather than edited:** `notes/visionfive2.md`,
`notes/board-console.md` and `notes/scripts.md` (milestone 257). `notes/why-not-general-purpose.md`
was held by PR #750, which landed mid-lane; its rewrite is the worked example at the top of this
block, and re-reading it after the merge found nothing further.

**Left alone and worth naming as coverage rather than as a gap.** The five oldest notes,
`mmu.md`, `registers.md`, `uart.md`, `page-tables.md` and `higher-half.md`, were read and are
correct, exactly as this block predicted. So is `net.md`, which is the model answer: its
"not proven by the gate" paragraph is dated *"when this was written"* and followed immediately by
**"Milestone 107 built exactly that, and the paragraph above is kept as the record of the gap rather
than edited away."* That is the shape every other stale paragraph should have had.

## The recurring shape: a note is corrected where its subject lives and not where its framing does

This is the finding, and it is one sentence: **the correction lands in the section that owns the
subject, and the opening line, the index entry and the closing aside are left saying the opposite.**
Every non-mechanical instance had it.

- `capability-lifecycle.md`'s "Revocation (milestone 13)" section has said **Built** since 13
  landed. Its own first paragraph promised to explain *"why it cannot yet be revoked"*, and
  `notes/README.md`'s entry repeated that two bullets above the entry for `object-revocation.md`.
- `target-hardware.md` carries a **Recast (2026-07-27)** banner saying first silicon is a
  VisionFive 2. Its "The plan" section, four screens down, still opened *"Raspberry Pi 4 is the
  next port."*
- `crates-io-on-nife.md`'s ranked gap list had five rows reading `Unsupported` with a **verb
  exists** annotation beside them. The PAL's own header records all five under *"Also bound since
  milestone 64"*. The note that says a gap is one binding away is the note nobody rereads when the
  binding lands.
- `security.md`'s findings were maintained; its closing "shape of the result" still said
  single-core, no IOMMU, no quotas, no delegation, and *"learning kernel"*.

**Why it recurs is structural rather than careless.** A person fixing a fact opens the file at the
fact. A framing sentence is upstream of it, an index entry is in a different file, and a closing
aside is past where anybody scrolls. §76 named the same defect in the roadmap ("a status fixed where
somebody happened to look and left wrong where they did not"); this is that one directory over, and
the practical rule that falls out is small: **when you correct a fact in a note, re-read that note's
first paragraph, its last paragraph, and its `notes/README.md` entry.** Three places, all cheap.

## The second finding: 262 of the corrections were a gate nobody had written

More than half of everything found was one mechanical shape, a backticked in-tree path that no
longer resolves, and it is the half a machine should own. It is milestone 383,
`design/roadmap/383-a-backticked-path-that-does-not-resolve.md`.

Its trap is worth carrying here too, because this lane fell into it: **a crate is named three ways**
(`crates/fs_proto`, `fs_proto::PAGE`, and a bare `` `fs_proto` ``), the first pass matched one of
them, and reported itself clean with 167 instances left.

## What this found that was not prose

`crates/elf/tests/fuzz_seed.rs` was deleted by `acc2338a` (2026-08-31), a commit about adding
`Segment::p_paddr` whose message never mentions the 62-line file it removes. `notes/fuzzing.md` and
`fuzz/seeds/README.md` both went on describing the guard in the present tense: a seed that stops
parsing is not a seed, and a fuzz run over rejected inputs reports the same "no crashes" a working
one does. Both tests pass unchanged and are restored.

**That is the argument for reading a note against the tree rather than for internal consistency.**
The note was not wrong about what should exist. Following its citation is what found that it did
not.

## Follow-on

- **Milestone 383.** A gate for an in-tree path in backticks that does not resolve, with the
  false-positive classes enumerated from this sweep's own four and an allow-list carrying a reason
  per entry. Written up the same day as an unnumbered proposal and numbered on 2026-09-19 by
  milestone 433's drain of the pile.
- **Recorded.** In this block's BUGS: this sweep is a snapshot and will go stale. It read 175 notes
  on one day against one tree.
- **Recorded.** In this block's BUGS: three notes were not read for staleness because milestone
  257's lane held them, and `notes/why-not-general-purpose.md` carries a title question that is
  calef's.
- **Recorded.** In `notes/falsification.md`: its BUGS said a kernel `#[test_case]` falsification
  sweep waits on milestone 210, which landed 2026-08-31. The sweep is now affordable and nobody has
  run it; the entry says so rather than still reading as blocked.
- **Recorded.** In `notes/security.md`: `sched::spawn_with_quota` has no caller, so the resource
  exhaustion the milestone-11 audit named is still reachable, for a different reason than the audit
  gave. That is a mechanism nobody wired rather than a mechanism nobody built.
- **Refused.** Deleting `notes/session-handoff.md`, which its own first paragraph authorises
  ("delete or overwrite once its contents are stale"). It carries the only narration several
  2026-07-29 decisions have. It gets a banner and a corrected index entry instead, and whether it
  should exist at all is an architect's.

## BUGS

- **Three notes were not read.** `notes/visionfive2.md`, `notes/board-console.md` and
  `notes/scripts.md` were held by milestone 257's lane throughout. They are the three this sweep
  can say nothing about, and two of them are large.
- **`notes/why-not-general-purpose.md`'s title overstates its own content**, and a rename is
  calef's. PR #750 landed during this lane and its rewrite records the same thing at the page
  itself, so the page was read here and needed nothing further; the title question stays open and
  stays an architect's.
- **A sweep is a snapshot and this one will go stale too**, on a tree where 68 milestone rows flipped
  to `BUILT` in fourteen days. The proposal it comes from is the recurring half; this is the
  one-time half, and neither substitutes for the other.
- **`notes/` is not the whole surface.** Milestone 66's stale gap table and milestone 99's compressor
  claim are both in `design/roadmap/`, and 99's is still there. This milestone is scoped to `notes/`
  because that is where the newcomer-facing prose lives, and the roadmap has the same disease.
- **"Load-bearing" is a judgment and this block does not define it.** That is deliberate, since a
  definition tight enough to check would be tight enough to be wrong, but it means two lanes would
  rank differently.
- **The 82% figure is borrowed.** It was measured for `git grep -w TODO` against a different
  question, and nobody has measured the false-positive rate of a negative-claim grep over `notes/`.
  The sweep can now say roughly what it was in practice: the negative-claim grep returned 141 files,
  and the ranking that came out of it produced findings in about a fifth of the ones read. Useless
  as a gate, exactly as this block predicted, and it did rank the reading.
- **"Deep-read" is softer than the table makes it look.** A note of 2,754 lines
  (`benchmarks.md`) and one of 87 (`asids.md`) both count as one, and the long ones were read for
  their claims rather than end to end. A second lane on `benchmarks.md`, `soak.md`,
  `load-sensitive-assertions.md` and `x86-port.md` alone would probably find more.
- **The classification is a judgment and two lanes would split it differently**, which this block
  already says about "load-bearing". 46 concept and 129 state is one reading; `nifefs.md` and
  `gpt.md` are formats (concept) that also describe what this tree implements (state), and both
  were counted as concept.

## Index row

the other half of the stale-notes proposal: a page saying "there is no networking" cites nothing,
so no check can catch it. 175 notes read and classified 46 concept / 129 state, 66 found stale,
109 left alone. **The recurring shape is that a note gets corrected where its subject lives and
not where its framing does**: `capability-lifecycle.md`'s revocation section said BUILT while its
own opening paragraph said "cannot yet be revoked", and `target-hardware.md` carried a recast
banner over a plan section still naming a Pi 4 as the next port. So the rule is three cheap
places, the first paragraph, the last paragraph and the `notes/README.md` entry. **262 of the
corrections were one mechanical shape** a gate should own, a backticked in-tree path that no
longer resolves, which every check here passes because it is neither a markdown link nor a `§N`;
proposed, with the trap that a crate is named three ways and a one-spelling check reports a clean
tree. It also found a deleted test: `crates/elf/tests/fuzz_seed.rs` went away in a commit about `p_paddr` that never mentions it, and two documents described the guard in the present tense for
five days
