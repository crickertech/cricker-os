---
status: NOT-STARTED
raised: 2026-09-30
promoted_from: a-screen-session-can-be-interrupted-and-torn-down-like-any-job
milestone_dependencies: 632
decision_dependencies: 24
machine_requirements: none
specific_machine: none
needs_person: no
---
# 700. A graphical terminal session can be interrupted and torn down like any job

Promoted from `design/roadmap/proposals/a-screen-session-can-be-interrupted-and-torn-down-like-any-job.md` on 2026-10-03 (UTC). The number 700 was minted by the maintainer in a batch promotion of the proposal pile and is provisional until the queue lands it. *(Title and slug are drafts.)*

<!-- writing-standards: exception. Granted 2026-10-03 (UTC) by the maintainer minting this milestone, not ratified by an architect. Reason: this block was promoted unedited from design/roadmap/proposals/, which the prose scope excludes, so it meets the sentence and bold limits only after an edit that promotion does not make. Trimming it is a separate pass, and the exception goes when it is done. -->

Raised 2026-09-30 by the milestone 632 (graphics on demand: `graphical_terminal`, launched from the swish prompt) lane (graphics on demand), which built the `graphical_terminal` session
as a plain spawn and recorded the cost in its own block: a session ends on its own (`^C` through
its terminal, or a `quit` line) and is not born under DECISIONS §24 (interrupting the foreground process, two-tier and shell-held)'s supervised-job shape, so a
second `^C` cannot escalate to a teardown and a hung session holds its 464-page region until
something reboots the machine.

The work: give the session's spawn the job untyped and job frame (the interruptible wiring the
shell already knows how to watch), so `graphical_terminal` becomes interruptible like any long-running job,
with the drivers swept by the same reap that already collects the region. The manifest's
`interruptible: false` is a first cut, not a design; flipping it is most of the milestone.

## Index row

A `graphical_terminal` session is a plain spawn, so a second `^C` cannot escalate to a teardown and a hung session holds its 464-page region until a reboot. Proposed: spawn it under the supervised-job shape of DECISIONS §24 (interrupting the foreground process, two-tier and shell-held).
