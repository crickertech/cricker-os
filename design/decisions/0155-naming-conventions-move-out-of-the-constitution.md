---
status: DECIDED
raised: 2026-09-18
decided: 2026-09-18
ratified_by: calef
---

# 155. The naming conventions move out of the constitution, and the note becomes the rule

*Amended 2026-09-26: rule text says "an architect" where it said calef, per §217 (every architect
holds the whole role). Records and quotations keep his name.*

calef, 2026-09-18: `AGENTS.md` keeps the naming authority and nothing else;
`design/naming.md` becomes the authority for the conventions. The file moves from `notes/` in the
same ruling, because a document that is normative is not a note.

What that means concretely, so no lane has to infer it:

1. `AGENTS.md` keeps four things: names are an architect's call, ship a **provisional** name and say so,
   never rename on your own initiative, and where the conventions live. Twenty-four lines where there
   were eighty-two.
2. `design/naming.md` keeps everything else: the six-domain spelling table, §154's acronym test,
   nouns over verbs, the three failure modes, the crate-and-program pairing, `NAME_LEN`, how to
   perform a ratified rename, and the refusals behind all of it.
3. The tiebreak inverts, and both files say so. Where they disagree, `design/naming.md` is the
   rule for conventions and `AGENTS.md` is the bug. Before today it was the other way round.
4. `script/names` prints the pointer, because whoever is about to rule on a name is already
   running that command and would otherwise be reading the wrong file.

## Why the split moved

The constitution is on a budget and naming was 8% of it. Milestone 118 caps `AGENTS.md` at 1,009
lines and `script/lint` enforces it. Recording §154 on 2026-09-18 blew that ceiling by fourteen lines
and took four passes to squeeze back under, and every line cut was argument rather than rule.
That is the budget working: it made the misplacement visible. `design/naming.md` was already 1,349
lines, a third larger than the constitution it was serving.

This finishes milestone 262 rather than reversing it. On 2026-09-05 the rules were written out
twice, in full, in both files, and 262 shrank that to one statement per rule up top with the case for
it below. The residue was still duplication, and it bit within two weeks: §154 changed the acronym
test and it had to be edited in both places in one commit or the note would have contradicted the
constitution on merge. Nothing compares the two and nothing plausibly could. §155 removes the
duplication instead of shrinking it.

**What makes it safe is the provisional name, and that is the load-bearing argument.** Moving a rule
out of the always-loaded file is a drop down AGENTS.md's own ladder, from a rule an agent has in
front of it to one it must choose to open, and this tree has been bitten by exactly that shape
(milestone 115 exists because naming decisions lived in a table cell nobody could find). It is
acceptable here because the audience for the conventions is not the audience under time pressure.
A lane inventing a name ships it provisional, marked as expected to change, and the maintainer
surfaces it. The conventions are needed at ratification and at rename, which are unhurried
moments with an architect in the loop. So the four lines that stay are exactly the ones whose violation is
expensive, and the ones that moved are the ones whose violation is already absorbed.

## Why the file left `notes/`

`notes/` is for concepts and findings; `design/` is for what is normative. A file that is the rule
for naming does not belong in a directory of explanations, and the move makes one `script/lint`
exclusion redundant on its own: the "daemon" rejected-vocabulary check exempted `design/` and
`notes/naming.md` separately, and now needs only `design/`. An exemption that disappears because a
file moved is evidence the file is finally in the right place.

Sixty-five files cited the old path and all were rewritten; none of the citations sat inside a
quotation, which was checked before the move rather than assumed.

## The near miss, recorded because it is the argument for the rule it nearly broke

Two rules existed only in `AGENTS.md` and were not in the note at all: the six-domain spelling
table (`SCREAMING_SNAKE_CASE` appeared zero times in the note) and "a crate and a program may share a
name, and it says something when they do". Trimming the constitution without checking would have
deleted both, and the note's own header would still have claimed it carried the case for every
rule above it.

They were found by grepping the note for each rule before committing, not by review. That is the same
failure family as the blind `sed` that rewrote the row recording a name's refusal: a cheap edit
destroying an expensive record. The procedure that caught it is the one this decision asks of the
next person who moves normative text: enumerate what you are deleting, and prove each piece landed.

## What this does not change

**Not the authority.** Names remain an architect's, at every level including public functions.

Not `script/lint`'s checks. What it could and could not verify before, it verifies now; the
conventions it cannot check were already prose and are prose in a different file.

Not the numbering schemes. `§N` and `milestone N` stay two schemes over one tree, explained where
they always were.
