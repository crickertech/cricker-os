---
status: BUILT
raised: 2026-09-05
built: 2026-09-19
---
# 262. The naming rule is written twice, so changing it means changing two files and remembering to

The note half was done by this block's lane; the `AGENTS.md` half was
done by DECISIONS §155 on 2026-09-18, which went further than this block's diff (the naming section
is 26 lines, a pointer and the authority, and the conventions live only in `design/naming.md`); the
duplicate heading was renamed on calef's ruling of 2026-09-19. It was PARTIAL from 2026-09-05 with
the note half done and the `AGENTS.md` half proposed as a diff, which is the split this block's own
BUGS section predicted. Minted 2026-09-05 by calef, on finding that a rule change had to land in
`AGENTS.md` and `design/naming.md` together or the note would have contradicted the constitution the
moment it merged. *(Number provisional until the merge queue lands it.)*

## The measurement

`AGENTS.md` is 1,043 lines under a hard ceiling with zero headroom by design (milestone 118), and
132 of them, 12%, are the naming section. `design/naming.md` is 760 lines on the same subject.

Both stated the same rule. `AGENTS.md` said standard terms *"should not be touched"* with a
list; `design/naming.md:154` said *"Standard terms are already right and must not be touched"* with
the same list. On 2026-09-05 that rule changed, and it had to be edited in both places in one commit.
Nothing would have caught it if it had not been: no gate compares the two, and the note is the file a
lane reads *second*, so the contradiction would have been discovered by somebody acting on the wrong
one.

That is this tree's recurring shape, named by milestone 259 (sweep)'s sweep the same day: *a note gets
corrected where its subject lives and not where its framing does*. Here there are two subjects and
one framing, which is the same defect wearing a second hat.

## The split to make, and it is not rule versus detail

The constitution keeps the tests a lane applies. Who names things, the `snake_case`/hyphen
domain table, nouns rather than verbs, and now the acronym test. Short, imperative, and read every
session, because `AGENTS.md` is the file an agent has in front of it and
`design/naming.md` is a file it visits only when something sends it there. Moving a rule to a note
demotes it to rung four of this file's own ladder, and naming is the most frequently applied rule
in the project: every lane ships names.

The note keeps the argument and the history. Most of those 132 lines are already that rather than
rule: the two-tier convention calef rejected and why, the `wc` example that killed it, the evidence
that a rule was needed at all, the three crates settled the day it was written, the count that was
wrong because a grep matched only single-line includes. Every one of those is worth keeping and none
of it is something a lane applies.

Expect to reclaim roughly half the section, which is real budget in a file that has none, and the
number should be reported rather than estimated when the work is done.

Measured: 60 lines, from 137 to 77, which is 44% of the section and 5.7% of the whole file.
`AGENTS.md` goes from 1,048 lines to 988, and the `agents-md-lines` ceiling in
[118's block](0118-constitution-budget.md) drops to 988 with it, in the same commit that applies the
diff. The section boundary measured is lines 770 to 906, the heading through the `NAME_LEN`
paragraph; the block's "132" counted the prose and not the blank lines between paragraphs.

Most of the reclaim was already-duplicated argument rather than argument being moved. The
two-tier rejection, the `wc` example, the "0 of 57" measurement, the standard-terms guard rail and
the `NAME_LEN` history were each written out in full in **both** files before this milestone, which
is the same defect the block was minted for, found four more times while fixing it once. Those
paragraphs were deleted from `AGENTS.md` and not moved, because the note's versions are longer and
better. What genuinely moved is the `dwarden`/`conx`/`cseam` evidence, the shared-module history,
the "three modules" miscount, the crates-are-in-scope justification, the milestone 160 extension, the
three crates the noun rule settled, and the domain table's two arguments.

## What good looks like

- One statement per rule, in `AGENTS.md`, with the note carrying its case.
- The note says it is the case rather than the rule, so a reader who finds a fuller treatment
  there does not think they have found a second authority.
- **Nothing is deleted.** Every argument moved, none dropped, because the history is why the rules
  survive contact with someone who disagrees.

## BUGS

- Nothing will stop it happening again. No gate compares a rule's statement in `AGENTS.md`
  against `design/naming.md`, and none plausibly could, since the two are meant to say different
  things about the same subject. This milestone reduces the surface rather than closing it.
- `AGENTS.md` is an architect's file. A lane may not edit it (milestone 118 (CLAUDE.md has a
  budget) records that a lane cannot even carry the budget marker there), so the note half is a
  lane's and the constitution half is calef's, and the milestone is not done until both have
  happened.
- The naming section is not the only duplicated one. Nobody has checked whether the merge queue,
  the lane roles or the dependency rule are stated twice the same way; this block is scoped to the
  one that was caught.
- "Roughly half" was a guess, from reading the section rather than from counting. It came out at
  44%, and the guess was closer than it deserved to be: the counting method was not decided until
  the work was done, and a section boundary drawn one paragraph differently moves the number by
  several points.
- No gate compares the two files, and this milestone did not add one. `script/lint` reads
  `AGENTS.md` only for its line count (`agents-md-lines`) and never parses the naming section, so
  the domain table and the prose around it moved without touching a check. That is convenient here
  and is the same fact as the first bug: nothing was watching, and nothing is watching now.
- The note still states some rules in its own words, which is unavoidable and is why the framing
  paragraph at the top of `design/naming.md` says which file wins. The standard-terms rule in
  particular reads "must not be touched" there and "not a licence to rename everything" in the
  constitution; those agree, and a future edit to one of them is exactly the failure this block
  describes.
- **`design/naming.md` carries two sections both titled `## Crates`**, one about the rule and one
  about what the 39 crate directories actually contain. Found while working and left alone, because
  a heading is a name and renaming one is a naming decision with extra steps.

## Follow-on

- **Done.** On 2026-09-19. The `AGENTS.md` half: DECISIONS §155 (2026-09-18) moved the naming
  conventions out of `AGENTS.md` entirely, superseding this block's diff. The `agents-md-lines`
  ceiling in 118's block is now 922, the file's exact count after §155, where this item had asked
  for 988 against the smaller diff.
- **Done.** On 2026-09-19, `design/naming.md`'s second `## Crates` became
  `` ## What `crates/` holds ``, calef's ruling that day. Its audience lists now name the crates as
  they are called today; its census, taken on 2026-08-01, keeps that day's names and now says so.
- **Recorded.** The merge queue and the lane roles are stated twice in the milder shape, and the
  dependency rule is not; the reading is in the pull request body and the lane report.

## Index row

AGENTS.md's naming section is 132 lines, 12% of a file with a zero-headroom ceiling, and
design/naming.md is 760 more on the same subject. Both stated the rule that changed on 2026-09-05.
The constitution should keep the tests a lane applies; the note should keep the argument. The note
half is done: it opens by saying it is the case rather than a second authority, and it received
the `dwarden`/`conx` evidence, the shared-module history, the milestone 160 extension, the noun
rule's three settled crates and the domain table's two arguments. The `AGENTS.md` half is a **60-line** reclaim (137 lines to 77, 1,048 to 988) proposed as a diff, because a lane cannot edit
that file. **Most of the reclaim was argument already written out in full in both files**, not
argument being moved: the two-tier rejection, the `wc` example, "0 of 57", the standard-terms rail
and the `NAME_LEN` history were each duplicated before this milestone, which is the same defect
found four more times while fixing it once
