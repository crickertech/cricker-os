---
status: DECIDED
raised: 2026-09-20
decided: 2026-09-20
ratified_by: calef
---

# 200. Two crates the documentation system refused, and why each loses on its own terms

Milestone 40 (documentation as a system service), 2026-08-26, minted as a
section on 2026-09-20 by the integrator, on the finding by milestone 448 (a refusal gets a number, a status, and a condition) that two
dependency refusals were living as prose in a BUILT block where §46 (thin primitives or whole subsystems) is the record that
judges them. *(Section number provisional until the merge queue lands it.)*

Nothing here is new. Both refusals were made when the documentation system was built, by the lane
that built it, and neither has been revisited. What changed is where they live: DECISIONS §46 says
taking a dependency is a decision rather than a convenience, and a decision belongs where the next
person reaching for that crate will look, rather than in the follow-on section of a milestone that
finished a month ago.

## `comrak`, for GFM tables, strikethrough and footnotes

Refused. It carries more dependencies than the job needs, and nothing in the corpus has wanted
a GFM table. That second clause is the load-bearing one and it is a measurement rather than a
preference: the refusal is not that the crate is bad, it is that the feature has no user.

This sits exactly where §46 draws its line. `comrak` is neither a thin architectural primitive nor a
whole subsystem nobody here would write; it is a richer parser than the renderer currently needs,
which is the "in between" §46 says to write rather than take.

Revisit when a page in this tree needs a table that the current renderer cannot show. At that
point the question is a real comparison, `comrak`'s graph against extending what we have, and it
should be asked again rather than inherited from here.

## `ratatui`, for the pager

Refused, and for a sharper reason than dependency weight. It needs a backend written against
this tree's own terminal contract before it can render anything at all. Taking it would buy a
widget library and leave the actual work undone, which inverts what a dependency is for: the point
of taking one is that somebody else did the part you did not want to do.

It is also the case §46's shape predicts. A terminal backend is ours by construction, because the
contract it must speak is ours, so the work does not leave the tree no matter which crate renders on
top of it.

Revisit if a backend against this tree's terminal contract is written for another reason. Then
the calculation changes completely, because the part that made this a bad trade is already paid for,
and a widget library on top of a working backend is the ordinary kind of dependency question.
