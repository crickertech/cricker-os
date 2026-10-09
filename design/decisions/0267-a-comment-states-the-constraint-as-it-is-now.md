---
status: DECIDED
raised: 2026-10-09
decided: 2026-10-07
ratified_by: calef
---

# 267. A comment states the constraint as it is now

Raised 2026-10-09 (UTC) by the lane building milestone 860 (comments state the constraint as it is
now). Decided two days earlier: calef ruled the forks of `notes/comment-cost-2026-10-07.md` on
[PR #1800](https://github.com/nifeos/nife/pull/1800) (2026-10-07, UTC) and ratified fork (c) in the
sharper form, *"ratify #1800 Fork (c) in the sharper form"*. The ruling is recorded on that pull
request; this section is the tree's copy of it, written by lane
`milestone/860-comments-state-the-constraint-as-it-is-now`. *(Section number provisional until the
merge queue lands it. Every script and file name below is provisional too.)*

The build is [milestone 860 (comments state the constraint as it is
now)](../roadmap/0860-comments-state-the-constraint-as-it-is-now.md): the gate, and the first
milestone's worth of the sweep.

## The ruling, as made

calef, 2026-10-07 (UTC), on what a Rust comment is for, in the sharper form he chose:

1. A comment states the constraint as it is now, in a sentence or two.
2. History ("this used to...", "found by milestone N...") goes in the commit message, where
   `git blame` finds it.
3. Semantics, wire layouts and rulings go in `design/decisions/`, cited by `§N`.
4. A cap of about 40 lines per comment block, held by a ratchet that only falls.

Points 1 through 3 say where each kind of sentence lives. The comment keeps the constraint; the
commit message keeps the history; a section keeps the semantics and the rulings, and the comment
cites it. A comment that cannot shrink to its constraint without losing a fact nobody has recorded
is a bug in where the fact lives. The sweep records the fact, in a note, a `BUGS` entry or a
section, rather than deleting it.

The rule the tree already had (AGENTS.md, "Comments": a comment explains a constraint the code
can't show; never restate the next line) is unchanged. The measurement that asked the question
found the rule is being followed: a uniform sample of 30 blocks kept 24, cut 4 and moved 2. What
the ruling adds is the cap on the long tail, and the placement rule for the sentences that
accumulate in place.

## The cap: 40 lines

"A cap of about 40 lines" is calef's wording. The exact number is set from the measured
distribution, as the milestone's block promised, not chosen before it. Measured 2026-10-09 (UTC)
at `2d59ddde1`, with the same lexer and block definition the note measured with
(`notes/comment-cost-2026-10-07.md` section 4): 28,950 blocks, median 2 lines, mean 4.77. Blocks
over 40 lines hold 19.8% of comment lines (the note, one day earlier, measured 20%); over 30 holds
24.5%, over 50 holds 17.3%. Forty is where the long tail's bulk sits, and it is the number calef
named. A block of 40 lines is allowed; 41 is over.

The block is the note's unit: a run of adjacent comment lines of one kind, doc (`///`, `//!`) or
plain (`//`, `/* */`). Comment density itself is not capped and not the target; this tree comments
on purpose. The cost the note measured is the long tail of essays, findings and corrections that
accrete in place, not the median two-line explanation.

## The gate: a ratchet in script/lint

`helpers/comment_block_ratchet.py` holds the cap, and `design/comment-block-baseline.tsv` is its
list. Both copy §266 (a Rust source file stays under 2,000 lines)'s file-length ratchet, which
copies §212 (a prose budget)'s prose ratchet:

- No block outside the list may exceed 40 lines. A new file's blocks meet the cap outright.
- The blocks over the cap when the gate lands go on the list, one row each, with the size of that
  day. A listed block may not grow against the merge base and may not exceed its row.
- A row that no longer has an over-cap block behind it fails, so the list cannot go stale.
  `--bank` lowers rows to the tree and never raises one.
- A block that must stay over the cap needs this section amended, by an architect.

Why a ceiling that only falls rather than exact match: §266 section 3's measurement (164 of 664
merges in a fortnight touching a listed file) argues it there, and holds one level down. The sweep
banks its shrinks rather than editing numbers by hand.

## The sweep

The list held 338 rows when the gate landed (2026-10-09, UTC). The sweep works through them per
file, worst first, one milestone's worth at a time. Each block over the cap either shrinks to its
constraint, with history to the commit message, findings to a note and semantics to a `§N`
citation, or stays listed until a later milestone's worth reaches it. Milestone 860 is the first
worth: the five blocks the note's samples judged "move" or tutorial.

## BUGS

- "About 40" is calef's number read off one measurement; 30 and 50 were not put to him, and either
  could be argued from the same distribution.
- The block count treats a trailing comment on a code line as a one-line block that can begin a
  run, which is the note's definition as its scripts took it. A definition that ignored trailing
  comments would count slightly fewer blocks.
- The lexer is the note's hand-written one, adapted. It decides comment spans well enough for a
  gate, and a file it mis-lexes would be mis-measured; none has been found, and the note's own
  caveats apply.
