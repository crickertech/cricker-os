---
status: PROPOSED
raised: 2026-10-08
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# Comments state the constraint as it is now

Raised for calef, who ruled the shape on 2026-10-07 (UTC) while deciding the forks of
`notes/comment-cost-2026-10-07.md` (PR #1800, fork (c)); this proposal files the sweep that
ruling wants, which nothing yet builds.

## The ruling, as made

calef, 2026-10-07 (UTC), on what a Rust comment is for, in the sharper form he chose:

1. A comment states the constraint as it is now, in a sentence or two.
2. History ("this used to...", "found by milestone N...") goes in the commit message, where
   `git blame` finds it.
3. Semantics, wire layouts and rulings go in `design/decisions/`, cited by `§N`.
4. A cap of about 40 lines per comment block, held by a ratchet that only falls.

The ruling is recorded on #1800 and nowhere else yet; minting its `design/decisions/` section is
part of this work. Its companion ruling (fork (b): code comments cite stable names, never a
`proposals/` path) already has its gate from that same pull request's thread.

Reuse: the block cap copies `helpers/prose_ratchet.py`'s pattern in-tree, as §266 (a Rust source
file stays under 2,000 lines)'s file-length ratchet does. No existing comment-block cap was found
outside the tree (rustfmt and clippy document none as of 2026-10-08), and the sweep itself is
prose movement, not code to take.

## Why a sweep is wanted

`notes/comment-cost-2026-10-07.md` measured the tree's comments on 2026-10-07 (UTC):

- 28,536 comment blocks, median 2 lines, mean 4.8. A uniform sample of 30, judged against
  `AGENTS.md`'s comment rule, kept 24, cut 4 and moved 2: the rule is being followed.
- The cost is the long tail, not the median. A length-weighted draw of 15 found blocks that are a
  legitimate explanation plus history and findings that accumulate in place, which is what
  "correct yourself loudly" produces when the correction is written at the site and never retired.
- Comment is 2.5 million tokens of a 4.3 million token tree.
- Comments went stale and were re-edited in 55 merged pull requests' worth of incidents: 24 where
  a fact moved under the comment, 13 of new content accreting, 8 of record status changes, 6 rename
  sweeps, 4 moved path references.

So the ordinary two-line comment stays. What moves is the block that tells a story: the history of
a removed `Unsupported`, a tutorial whose last sentence is its only constraint, a correction with
its motivation still attached after the code it corrected is gone.

## What the sweep does

Per file, worst first, one milestone's worth at a time:

1. Move history and findings out of comment blocks. The fact goes in a note if it is still a
   finding (`notes/`, cited by name), in the commit message if it is history, or is deleted if it
   restates a record that already exists.
2. Move semantics and rulings to a `§N` citation where a section exists; where the ruling is only
   in the comment, this sweep records it first, exactly as it stands, before deleting the comment.
3. Build the block cap as a lint ratchet that mirrors §212 (a prose budget)'s
   `helpers/prose_ratchet.py`: a committed list of blocks over the cap that only falls, a
   comparison against the merge base, and a selftest. Its shape is the same as §266 (a Rust source
   file stays under 2,000 lines)'s file-length ratchet, milestone 841 (a ratchet on Rust file
   length). The cap is about 40 lines; the exact number is set when the ratchet is built, from the
   measured distribution, not chosen before it.

A comment that cannot shrink to its constraint without losing a fact nobody has recorded is a bug
in where the fact lives, and the sweep records the fact rather than deleting it.

## What it refuses

- No rewrite of the 80% that the sample kept. The rule for ordinary comments does not change.
- No split of files along the way; that is milestone 840 (the scheduler file is split along its
  seams), not this.
- No new prose beyond the notes the findings move into; §213 (writing standards) governs them and
  the prose ratchet holds them from the day they are written.

## Evidence

Everything measured above is in `notes/comment-cost-2026-10-07.md`, including the scripts that
took each number (`notes/comment-cost-2026-10-07/`). The kept/moved judgments of the thirty-block
sample are listed block by block there.
