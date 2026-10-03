---
status: PROPOSED
raised: 2026-09-30
milestone_dependencies: 632
decision_dependencies: 24
machine_requirements: none
specific_machine: none
needs_person: no
---
# A screen session can be interrupted and torn down like any job

Raised 2026-09-30 by the milestone 632 (graphics on demand: screen, launched from the swish prompt) lane (graphics on demand), which built the `screen` session
as a plain spawn and recorded the cost in its own block: a session ends on its own (`^C` through
its terminal, or a `quit` line) and is not born under DECISIONS §24 (interrupting the foreground process, two-tier and shell-held)'s supervised-job shape, so a
second `^C` cannot escalate to a teardown and a hung session holds its 464-page region until
something reboots the machine.

The work: give the session's spawn the job untyped and job frame (the interruptible wiring the
shell already knows how to watch), so `screen` becomes interruptible like any long-running job,
with the drivers swept by the same reap that already collects the region. The manifest's
`interruptible: false` is a first cut, not a design; flipping it is most of the milestone.
