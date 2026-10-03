---
status: NOT-STARTED
raised: 2026-09-26
promoted_from: w-shows-who-is-logged-in
milestone_dependencies: none
decision_dependencies: 164
machine_requirements: none
specific_machine: none
needs_person: no
---
# 681. `w`: who is logged in, and what they are running

Promoted from `design/roadmap/proposals/w-shows-who-is-logged-in.md` on 2026-10-03 (UTC). The number 681 was minted by the maintainer in a batch promotion of the proposal pile and is provisional until the queue lands it. *(Title and slug are drafts.)*

Raised by milestone 126 (the `procps` package)'s lane
`milestone/126-free`, which moved `w` out of 126 so that milestone could close on what it built.

A tid has no name, and `w`'s `WHAT` column is a name.

## What is owed

Upstream `w` prints who is logged in and what each of them is running, with idle and CPU time per
session. Measured against the tree on 2026-09-26, in section 2 of
`notes/process-view/what-is-left.md`:

- What they are running needs a name for a tid, which is DECISIONS §164 (whether the kernel resolves
  a tid it already sent), still proposed. §164's own "what is blocked" does not yet list `w`'s
  `WHAT` column or `ps`'s missing `CMD` column.
- Who is logged in: `components/src/login.rs` runs one session at a time on one terminal, so a `w`
  today would always print one row.
- CPU time exists since milestone 282 (a thread's CPU time).

A `w` built before both would demonstrate nothing a reader could not see by looking at the terminal.

## Index row

`w` prints who is logged in and what they are running, but a tid has no name and login serves one session at a time. Proposed: build `w` once DECISIONS §164 (whether the kernel resolves a tid it already sent) is ruled.
