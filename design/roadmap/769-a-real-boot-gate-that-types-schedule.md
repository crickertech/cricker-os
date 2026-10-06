---
status: NOT-STARTED
promoted_from: a-real-boot-gate-that-types-schedule
raised: 2026-10-03
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 769. A gate that types SCHEDULE on the real boot

<!-- writing-standards: exception. Granted 2026-10-06 (UTC) by the maintainer minting this milestone, not ratified by an architect. Reason: this block was promoted unedited from design/roadmap/proposals/, which the prose scope excludes, so it meets the sentence and bold limits only after an edit that promotion does not make. Trimming it is a separate pass, and the exception goes when it is done. -->

Raised 2026-10-03 (UTC) by the record-hygiene pass over milestone 152 (durable delegation). The
durable session, the schedule store and the start-up re-derivation are all proven by the kernel
suite's `login_tests` on three ISAs, but the real boot's `login` is exercised by nothing that
sends it `SCHEDULE`: `script/swish-check` drives the prompt and has no `login` client.

What a lane would do: give `swish-check` (or the boot check) a client that logs in, registers a
one-line schedule, and asserts the timetable ran it on the real boot's grant sizes, so a change to
`login`'s budget (`login_protocol::durable::BUDGET_PAGES`) that the kernel suite's harness hides
fails a gate. A cheaper first step is to measure whether the harness and the real boot differ in
any way that matters; if they do not, refuse this with that measurement.

## Index row

The durable session, schedule store and start-up re-derivation are proven only by the kernel suite's harness; nothing types `SCHEDULE` at the real boot's `login`. A swish-check client would make a change to `login`'s real grant sizes fail a gate, or a measurement would show the harness and the real boot do not differ and the work can be refused.
