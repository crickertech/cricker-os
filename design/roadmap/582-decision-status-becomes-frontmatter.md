---
status: BUILT
raised: 2026-09-23
built: 2026-09-23
---
# 582. A decision's status becomes a field, and the index becomes generated

Raised by calef on 2026-09-23: *"I wonder if the key values should be front
matter instead."* He ratified the schema the same day. *(Number provisional until the merge queue
lands it. The brief said 580, which was taken on `main` at this lane's base commit `bfc91c8d2` by
`580-nobody-reads-branches.md`. This lane then took 581 and collided with another lane that could
not see it, which is the collision `AGENTS.md` gives the integrator to resolve at merge; pull
request #1193 keeps 581 and this block moved to 582.)*

## Why a prose field was the defect

`**Status: DECIDED.**` is a data field written as a sentence, and `script/decisions` read it with
`re.match(r"\*\*Status: (.+?)\.\*\*", first)`. Two failures on record come from exactly that shape.

A file said `**Status: NOT YET`, the regex captured `NOT`, dropped `YET`, and the report printed a
word nobody had written. That is §211 (what a fatal-risk verdict says, and what the chart can plot as a result)'s finding.

`design/fatal-risks/README.md`'s risk 7 carried two status lines eighteen lines apart. The script takes the
first match per section, so `AUDITED` was invisible to every consumer for weeks.

A schema removes that class. A tighter regex would have fixed the two instances. That is rung one of
`AGENTS.md`'s ladder against rung two, and the ladder says to take the higher rung when it fits.

## The second payoff, which is the larger one

`design/decisions/README.md` repeated every section's number, status and title, and part of what
`script/decisions` did was check that a file and its row agreed. Generated from the files, that
index stops being hand-maintained. `briefs/rebase-onto-main.md` names it as the worked example of an
additive-index conflict, case 4, because every lane minting a section edited it. That conflict class
is now gone, and the brief's example is stale. Naming it is this block's job; editing that brief is
not, since it sits on another branch in flight.

## The schema, ratified by calef on 2026-09-23

```yaml
---
status: DECIDED
raised: 2026-09-23
decided: 2026-09-23
ratified_by: calef
---
```

| key | values | required when |
|---|---|---|
| `status` | `PROPOSED`, `DECIDED`, `AMENDED`, `SUPERSEDED` | always |
| `raised` | `YYYY-MM-DD`, UTC | always |
| `decided` | `YYYY-MM-DD`, UTC | `status` is `DECIDED` or `AMENDED` |
| `ratified_by` | GitHub username | `status` is `DECIDED` or `AMENDED` |
| `superseded_by` | a section number | `status` is `SUPERSEDED` |

Values are uppercase, matching the vocabulary already in readers' heads. Keys are snake_case.
`superseded_by` splits today's compound `SUPERSEDED BY 42` token into a value and a pointer, so a
gate can check the target exists, which it could not before. There is no `number` key and no `title`
key, because both already live in the filename and the H1, and a third copy is a third thing to
disagree. There is no `branch` key, because a branch is transient and the pull request records it.

The rule that ships with the schema: frontmatter is authoritative, and the prose may not restate the
status. Two copies of one fact in one file is the defect this removes.

## What the gate can and cannot say about a restated field

The gate refuses a `**Status: ...**` token anywhere outside a code fence, and it refuses an opening
sentence of the field-shaped forms `Ratified by <user> on <date>` and `Decided <date>`. It does not
refuse an attributed sentence like `calef, 2026-09-18: <ruling>`, which 130 files use and which is
voice rather than a duplicated field. A gate that stripped those would make 130 files worse to read
in exchange for removing no ambiguity, since the frontmatter is the field either way. That limit is
recorded in the script's own `BUGS` section, where a reader meets it.

## The provenance the tree never wrote down, and where it was found

The schema was ratified before anyone had counted, and the corpus predating it states its own
provenance thinly. Of 211 files, 59 state a raise date and 106 a decision date.

The rest came from the repository, on calef's ruling of 2026-09-24 that a decision's date is the
date its commit went to decided, and that a raise date is the date it was minted, and on his
sharpening of it the same day: *"We write things down when we raise them. There is no gap between
the commit and when it was raised."* So a date read from git is the date, not a bound on it.

`raised` is the first commit that wrote the decision down, wherever it then lived. For the 73
sections that predate milestone 114 (split `DECISIONS.md`, and give a decision a status), that is
the first commit whose `DECISIONS.md` (or, for the six it absorbed, `design/open-decisions.md`) carries the
section's heading, matched by title as well as number so a renumbered section keeps its origin. The
split commit's own date, 2026-08-04, is when the files were cut, not when anything was raised, and
it dates none of them. After the split, `raised` is the file's first commit found with `--follow`,
merged with a plain walk because `--follow` truncates at some renames; that is what dates §197 (a
package is one archive file) to the proposal file it was promoted from, and §103 (what a `Frame`
names) to its first number.

`decided` is the first commit whose text reads DECIDED or AMENDED, in either spelling. A section
filed into `DECISIONS.md` was filed decided, since open questions lived elsewhere until milestone 114
absorbed them, so for those 42 files it is the raise commit. Author dates, UTC.

| key | from the prose | from git | total |
|---|---|---|---|
| `status` | 211 | 0 | 211 |
| `raised` | 59 | 152 | 211 |
| `decided` | 106 | 62 | 168 |
| `ratified_by` | 168 by ruling | 0 | 168 |
| `superseded_by` | 4 | 0 | 4 |

Where both sources speak, the earlier wins, and that is now a rule about the prose rather than about
git. The prose dates written before 2026-09-13 are calef's local day, not UTC, so a commit made on a
Pacific evening lands one UTC day after the date the prose gives it. Fifteen files came out raised
one day after they were decided, and `raised` is clamped to `decided` there. Twelve are that
evening rollover. Three are not, and they are the exceptions to calef's "no gap": §28 (SMP
placement), §34 (RedoxFS is the primary filesystem) and §46 (thin primitives or whole subsystems)
each say they were decided the day before the commit that wrote them into `DECISIONS.md`, in local
time too. One goes the other way: §100 (the terminal font)'s prose
names its amendment date, and git's earlier decided date is the original ruling.

§212 (a prose budget) and §213 (writing standards) landed on `main` in the old `**Status:**` form
while this was in review, and were converted when it rebased. Both state their dates in their own
prose, 2026-09-23 in UTC, which is earlier than their first commits (04:43 UTC on 2026-09-24), so
the prose wins and they are not counted in the table above.

A first pass of this migration dated the pre-split files from the split commit, 2026-08-04, and
called every git date "no later than" to cover it. That was a tracing error presented as a caveat;
the history before the split is in the repository, and following it moved 46 raise dates and 42
decision dates back by up to three weeks.

## Why `ratified_by` came out and went back

This block records the round trip because the value is identical and its provenance is not, and
because the difference is the one a later reader cannot reconstruct.

The migration first wrote `ratified_by: calef` on all 168 decided sections, reasoning that this tree
has one architect, that every decision is his by the constitution, and that no other name appears as
a ratifier anywhere in the directory. A reviewer refused it, correctly on the evidence available:
this lane had just declined to infer 213 dates on the grounds that the field whose job is not lying
must not hold a reconstruction, and a lane that then infers 89 ratifiers has a preference rather
than a standard. The key was blanked on the 89 files whose prose does not name him.

calef then ruled, on 2026-09-24: *"ratified_by: calef on all 168. [He is] the only ratifier to
date."* That is the architect stating a fact about his own role, which is not the act the refusal was
aimed at. The key is back on all 168, and nobody should read it as the inference that was refused.

## The gap ledger, and why there is none

There was one, `design/decisions/PROVENANCE-GAPS.md`, holding 302 entries and enforced as a ratchet:
a gap named there passed, a gap that was not failed. It was the right answer while the tree held
facts nobody had written down, and it is the shape of `script/names --unratified`, a worklist rather
than a wall.

Deriving the dates emptied it. An allowlist with nothing on it is weaker than no allowlist, because
it offers an escape a lane can reach for instead of doing two `git log` calls, so it is gone and the
required-when rules are hard failures. That is rung one where the ledger was rung two.

## Index row

A decision's status stops being a sentence a regex has to guess at and becomes a field, and
`design/decisions/README.md` stops being hand-maintained, which deletes the additive-index conflict
every lane minting a section used to pay.

## Follow-on

- **Milestone 596.** Milestone 596 (the roadmap blocks get frontmatter too) is the same
  migration for `design/roadmap/`, promoted from its proposal on calef's ruling of 2026-09-26 that this pilot
  has held. Its status vocabulary is nine tokens rather than four, and `script/roadmap` reads far
  more out of prose than `script/decisions` did.
- **Recorded.** The gate cannot refuse an attributed sentence that repeats a date the frontmatter
  holds, and the reason is in `script/decisions`' `BUGS` section.
