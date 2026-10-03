---
status: PROPOSED
raised: 2026-10-03
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# A gate that types SCHEDULE on the real boot

Raised 2026-10-03 (UTC) by the record-hygiene pass over milestone 152 (durable delegation). The
durable session, the schedule store and the start-up re-derivation are all proven by the kernel
suite's `login_tests` on three ISAs, but the real boot's `login` is exercised by nothing that
sends it `SCHEDULE`: `script/swish-check` drives the prompt and has no `login` client.

What a lane would do: give `swish-check` (or the boot check) a client that logs in, registers a
one-line schedule, and asserts the timetable ran it on the real boot's grant sizes, so a change to
`login`'s budget (`login_protocol::durable::BUDGET_PAGES`) that the kernel suite's harness hides
fails a gate. A cheaper first step is to measure whether the harness and the real boot differ in
any way that matters; if they do not, refuse this with that measurement.
