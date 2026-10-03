---
status: PROPOSED
raised: 2026-09-26
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# The confinement table lists the unvouched child

Raised by milestone 198 (a package manager) rung 3a's D2 lane,
which built and falsified the claim milestone 202 (every confinement test is a ritual until
somebody breaks the confinement) was owed and could not add its row. Name provisional.

An editing pass on one note, reversible, and a maintainer's to run.

The claim: an unvouched child holds no capability its caller did not delegate, beyond the clock
and configuration pages (§219 (how the shell names an installed program to the spawner), gate D2).
It is tested by `script/swish-check`'s `installed/unvouched` line and was falsified three times by
hand on aarch64, once per authority. The account is notes/packages/running-unvouched.md.

It belongs as row 31 of notes/confinement-claims.md's table. Touching that note costs three things
the lane did not take on:

- The bold touch rule. The note carries 99 bold spans in about 9,000 words; a touched document
  must end at 4 per 1,000, so about 65 go.
- The citation gate glosses every citation on a changed line, and stripping bold changes about
  seventeen cited lines.
- The note is over §212 (a prose budget)'s word cap, so the row and the glosses have to be paid for by cuts.

Done means the row is in the table, the note passes `script/lint`, and milestone 202's Follow-on
entry for this proposal is marked done.
