---
status: REMOVED
raised: 2026-09-19
built: 2026-09-19
---
# 434. Retire the `Proposed.` disposition and the machinery that read its directory

Removed 2026-09-19, the same evening it was built. The work was sound and the premise
was not: the maintainer read calef's *"we want to drive proposals to zero"* as a ruling to abolish
`design/roadmap/proposals/`, and his follow-up question, *"shouldn't a proposal become a milestone as
NOT-STARTED?"*, as a ruling that lanes should skip the proposal step. He meant neither. He was
saying that proposals should be promoted into milestones rather than left to accumulate, which is
what milestone 433 did to all 106 of them.

calef, 2026-09-19, on reading what had been done: *"There will be future proposals. Deleting
support for them would be short sighted. We just don't want anything to change for future proposals.
The mechanism was working fine. We just needed to see them into milestones."*

So every cut this block made is reverted: `**Proposed.**` is back in the vocabulary,
`design/roadmap/proposals/` is back, and so are `--proposed`, `--unclaimed`, `PROPOSED_DIR`,
`script/fatal-risks`' check 6b and the `script/metrics` column. The block is kept rather than deleted
because the reverted work was careful and its reasoning is worth reading, and because a `REMOVED`
block is the record a deleted file would not be, which is calef's own argument for promote-then-close.

Two costs the maintainer failed to price, and they are why he was wrong, recorded here so the
next person weighing this fork starts from them. **A number is permanent and never reused**: of the
106 promoted that evening, 12 landed `BUILT` or `SUPERSEDED`, so twelve numbers are spent forever on
work that was already done or subsumed, where a deleted proposal costs nothing. And **the
low-ceremony door matters most to the person who cannot mint**: the 2026-09-03 ruling's headline is
*"anybody may add to this roadmap"*, and asking an outside contributor for a global permanent number
is more than asking for a file with a slug.

*(Number provisional until the merge queue lands it.)*

It carried `Gate: NONE` while it was open, because the decision was already made and written
down in two places and what was left was the code that had not caught up. The line is gone because a
finished block's gate can only be stale.

## The contradiction this closes

calef ruled on 2026-09-19 that `design/roadmap/proposals/` should not exist, milestone 433 drained
all 106 files out of it, and two records were rewritten to say so:
`design/decisions/0140-follow-on-disposition-vocabulary.md` retired `Proposed.` and left six words,
and the roadmap's own prose, now `notes/roadmap.md`, says a lane writes the numbered block itself.

The scripts said otherwise. `script/roadmap` still accepted `Proposed.` in a follow-on bullet,
still required that bullet to name a file under a directory that no longer exists, still carried a
`--proposed` mode that listed an empty pile and a `--unclaimed` mode whose only input was the
`Proposed.` branch, and still spent about sixty lines of header and section prose explaining a
queue nobody is in. `script/fatal-risks` still carried a check keyed on the same directory.
`script/metrics` still counted it.

A tree where the prose and the gate disagree teaches the gate, because the gate is what fails a
build. This block is the other half of the same afternoon's work.

## What was cut, and what was kept

The reasoning was kept and the remedy was replaced, which is the distinction that decided every
paragraph here. `Proposed.` was not a mistake: it named a real hole in the vocabulary, and the
account of that hole is still true. Three lanes on milestone 247's sweep found work a block had
named honestly and nobody had taken, found that `Recorded.` lies about intent and `Refused.` lies
about the decision, and left the item out, which is the burial arriving through the gate that
exists to stop it. What changed is only where the work is written down: the word needed a file to
point at, the file needed a number, and the number is what a lane could not mint. The 2026-09-19
ruling removed that constraint instead of routing around it.

So every cut below either carries its argument forward to the thing that superseded it or says
plainly that the argument went with the directory.

### `script/roadmap`

- `Proposed` is out of the `DISPOSITION` regex, out of the `elif kind ==` chain, and out of the six
  error messages that spelled the vocabulary. The header's account of why the word existed is kept,
  compressed, under a heading that says it is retired and names what replaced it.
- `--proposed` and `--unclaimed` are gone. `--unclaimed`'s only source was the `Proposed.` branch,
  so it could only ever print nothing; the `--check` line that reported its count is gone with it.
- `PROPOSED_DIR` is gone, and with it the `os.listdir` of the pile, the five per-file diagnostics,
  and the import of `helpers/roadmap_proposals.py`.
- **The directory can no longer come back in silence, and that is a stronger guard than the one
  removed.** The file loop used to skip `proposals` by name. It no longer does, so a
  `design/roadmap/proposals/` that reappeared would fail the check that already says nothing but
  milestone files lives here. That is rung one wearing an existing check's clothes: the wrong state
  is not gated, it is unrepresentable in a passing tree. Proved by making the directory and reading
  the red.

### The promoted-proposal check, which is deleted rather than kept as a guard

Added 2026-09-18, it failed a numbered block claiming promotion from a proposal that was still
sitting in the pile. **It is deleted**, for this script's own recorded reason rather than for
tidiness: its whole input was `os.listdir` of a directory that cannot exist, so it could not come
back red on any tree. `script/roadmap`'s header already says what that costs, about its own
merged-branch check, which *"spent a day passing in CI while being unable to fail"*: a check that
cannot fail is worse than one that is absent, because its green is read as an answer.

Keeping it as a guard against the directory returning would have been the softer call and the worse
one, because the file loop above now guards that outright, and two checks answering one question is
the shape where the second stops being updated.

### `helpers/roadmap_proposals.py` survives, cut to its parse half

Deleting it was the expected answer and it is the wrong one, for a reason that is specific to how
`script/metrics` works. That dashboard is a **restatement**: `--backfill` rewrites every week's row
by applying today's definitions to old commits, reading blobs at revisions nobody has checked out.
The `proposals_unnumbered` column is 74 at 2026W36 and 93 at 2026W38, and those two numbers are
true about the tree at those revisions. Delete the parse and the next backfill writes **zero** into
both, which is a dashboard silently lying about a fortnight that happened. The rise and fall of
that directory is the measurement §140 and 433 both rest on, so it is the last thing to erase.

What went is `promoted_from` and the promotion commentary around it, whose only caller was the
check above. What stays is `DIRECTORY`, `filename_problem` and `classify`, with a header that says
the form they parse is historical and says why one caller is enough to keep it.

Milestone 236's rule (a derivation two scripts copy will drift while both look authoritative) no
longer applies, since there is one caller now. The module stays a module anyway: it is the place a
reader meets the explanation of why a column on a live dashboard is permanently zero going forward,
and folding fifty lines of a dead record form into the middle of the dashboard would bury that.

### The roadmap's disposition table, one word

The disposition table still introduced itself as "one of eight dispositions" after its own row for
`Proposed.` was deleted. Seven now, six of which any block may use and `Outstanding.` only a
`PARTIAL` one, which is the count the table has actually had since the ruling.

### `script/fatal-risks`

Check 6b asked whether a proposal cited by a risk entry had changed after the entry was last dated,
using git rather than a recorded date because a proposal had no completion to record. `design/fatal-risks/README.md`
cites no proposal today and can never cite one again, so 6b and its two fixtures
(`proposal-moved`, and the `no-history` refusal that existed to keep 6b from passing vacuously in a
shallow clone) are gone, along with the `last_touched` argument that fed only them. The
`missing-path` fixture kept its check and lost its proposal-shaped path.

**The question 6b asked is still good and is deliberately not generalized here.** "A cited artifact
that would answer this risk changed after the entry was written" applies to numbered blocks too,
and check 6a only reaches the ones that turned `BUILT`. Widening it to every cited path would trade
the one-path noise surface that script's header deliberately chose for a tree-wide one, which is a
judgment about what a gate should accept rather than a consequence of this ruling. So it is
recorded where a reader meets the check, in that script's own header beside check 6, rather than
decided here.

## BUGS

**Nothing re-reads a script header.** The prose cut here had been wrong since the moment 433's last
lane merged, and no gate in this tree can tell a stale paragraph from a current one. What caught it
was a person reading the file to change it, which is rung four, and this block does not fix that.

About forty citations of `design/roadmap/proposals/<slug>.md` still dangle, in milestone blocks,
notes, two audit reports and one shell script. They are untouched here and are milestone 436:
433 chose deliberately to keep the slug on promotion so that a reader can resolve one with a single
`ls design/roadmap/ | grep <slug>`, which makes them readable but does not make them true.

`script/metrics --check` is red on this branch and was red on its base. The CSV is regenerated
by `.github/workflows/metrics.yml` on a schedule, this week's `proposals_unnumbered` will fall to 0
when it next runs, and nothing here touches the generated file.

## Follow-on

- **Milestone 436.** About forty citations of `design/roadmap/proposals/<slug>.md` still dangle,
  across roughly twenty-seven files, and a handful of finished blocks still describe the directory
  in the present tense. Found while verifying that nothing referenced what this block was cutting,
  deliberately not fixed here, and minted as its own block under the convention this one finishes.
  Number provisional.
- **Recorded.** *The question check 6b asked is still open and this block did not answer it.* A risk
  entry in `design/fatal-risks/README.md` that cites a `NOT-STARTED` or `PARTIAL` milestone which changed
  under it is invisible, because check 6a reads only the roadmap's recorded Built date and 6b, which
  read git, went with the directory it was scoped to. Recorded in `script/fatal-risks`' own header
  beside check 6, where the next person to touch that gate is already reading, with the reason it is
  not taken here: widening the git half to every cited path trades a one-path noise surface for a
  tree-wide one, which is a judgment about what the gate should accept.
- **Recorded.** *Nothing re-reads a script header.* The prose cut here had been wrong since the hour
  milestone 433's last lane merged, and no gate in this tree can tell a stale paragraph from a
  current one. Recorded in this block's `BUGS` above. It is the same blind spot `script/roadmap`
  records for well-formed-but-wrong citations, one layer out, and a gate that tried to read prose
  for intent is what AGENTS.md priced at `git grep -w TODO`'s 82% false-positive rate.
- **Done.** *`script/metrics --check` is red and stays red until the scheduled run.* Nothing here
  touches `notes/project-metrics/weekly.csv`, which `.github/workflows/metrics.yml` regenerates with
  `--update`; it was already red on this block's base commit `d6db414` for reasons of its own. The
  branch that carries this block leaves the generated file alone on purpose, because two maintainer
  branches were open against that CSV when this landed.
- **Refused.** *Deleting `helpers/roadmap_proposals.py`.* It was the expected answer and it is
  wrong: `script/metrics --backfill` restates every week from git history, so removing the parse
  would rewrite two true weeks (74 at 2026W36, 93 at 2026W38) to zero, which is the dashboard lying
  about a fortnight that happened. The module keeps its parse half and lost `promoted_from`.

## Index row

The tree contradicted itself for an afternoon. calef retired the `Proposed.` disposition and the
`design/roadmap/proposals/` directory on 2026-09-19, milestone 433 drained all 106 files out of it,
and the records were rewritten to say so, while `script/roadmap` still accepted the token, still
required it to name a file under a directory that no longer existed, still carried `--proposed` and
a `--unclaimed` mode that could only print nothing, and still spent sixty lines of header prose on
a queue nobody is in. `script/fatal-risks` and `script/metrics` each kept a piece of the same
machinery. This block cuts the token, the two modes and the two checks that can no longer fail,
keeps the argument for why the word existed beside what superseded it, and turns the file loop's
existing "nothing else lives here" check into the guard that the directory cannot come back in
silence. `helpers/roadmap_proposals.py` survives on one caller, because `script/metrics --backfill`
restates every week from git history and deleting the parse would rewrite two true weeks to zero.
It is also the first use of the convention it finishes: this block was minted by the lane that
wrote it, with a provisional number.
