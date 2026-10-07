---
status: SUPERSEDED
raised: 2026-09-20
superseded_by: 806
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 481. Real terminal multiplexing

Superseded 2026-10-06 (UTC), by milestone 806 (concurrent login sessions). Its revisit condition
came true that day, when calef asked for more than one person on the machine at once. He ruled the
six forks of the proposal 806 was promoted from the same day, and §253 (concurrent login sessions)
records them. The refusal is kept below because it
is the argument 806 answers. 806 took its own number rather than reviving this one, because it is
wider than multiplexing a terminal: a greeter per console, authority granted at login, memory from
a broker and the owner's login all ride with it.

Refused by milestone 49 (design/roadmap/49-users-and-attribution.md), and
recorded there on 2026-09-03. Backfilled here on 2026-09-20 by
milestone 448 (design/roadmap/448-a-refusal-gets-a-number.md), which gave a refusal that names work a number, a
status and a condition that would change it. *(Number provisional until the merge queue lands it.)*

**The date is when the refusal was written down, not necessarily when it was made.** Most of this
tree's `- **Refused.**` bullets were written in one sweep on 2026-09-03, so the decision is usually
older than the bullet and its reasoning sits in the block's own prose above it.

## The refusal, in its own words

From '49. Users, login, and attribution: what identity is for once it stops being authority', under
`## Follow-on`:

> Real terminal multiplexing. `login` hands the terminal to the first successful caller and
> refuses the rest with a dedicated code until `LOGOUT`, deliberately, because the narrow shape
> commits to nothing the wider one would later have to unwind.
>
> -- design/roadmap/49-users-and-attribution.md

## Why it is here rather than only there

`login` hands the terminal to the first successful caller and refuses the rest with a dedicated code
until `LOGOUT`. That is a narrow shape chosen on purpose: it commits to nothing that a wider one
would later have to unwind, and the refusal it returns is explicit rather than a hang or a race.

## The condition that reopened it

The condition was met on 2026-10-06 (UTC). It read:

- **Condition.** Somebody needing two sessions at once. The refusal's argument is about commitment
  rather than difficulty: the narrow shape can grow into the wide one, and the wide one cannot be
  un-shipped, which is this tree's standing test for how much deliberation a decision deserves.

## Index row

One terminal, one session, and an explicit refusal for the second caller. The refusal is about what
a wider shape would commit to rather than about the work, which makes the condition a user rather
than a design.
