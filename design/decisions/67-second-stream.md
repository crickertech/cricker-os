---
status: DECIDED
raised: 2026-08-03
decided: 2026-08-03
ratified_by: calef
---

# 67. A program's second stream is a declaration, not a number

<!-- writing-standards: exception. Marked 2026-10-05 (UTC) by lane/bold-backlog-12 (calef's ruling
of 2026-10-05 UTC: "Bold-backlog lanes can edit all of design/"). Reason: the opening line below keeps its
bold because script/decisions' restated-key check anchors on a line that starts "Decided <date>" and
sees it only through the markup; unbolded, the document fails that check, and bold-free it would need a
rewording that this lane may not make. It is a foot gun: the bold is a workaround, and the fix is to
reword the opening in a later pass, after which this exception should be removed. -->

**Decided 2026-08-03 (calef), from notes/pipes.md's open fork.** `2>` gets built on option (c):
a program that has diagnostics declares a second output in its manifest (`OutputSpec` grows
the position), the shell plans a second endpoint only for programs that declare one, and `2>`
binds to the declared output. Aimed at a program that declares none, it is a truthful refusal, the
same statement `caps` already makes: the command line can only name what the manifest offers.

The alternatives, refused with reasons. A numbered-slot convention (Unix's fd 2 transplanted)
imports the ambient-agreement disease the note diagnoses: nothing here is ambient, and a number
everyone must agree on forever is the mechanism this system exists to not need. A second opcode
on the one endpoint (a diagnostic tag in the sink frames) is cheap but preserves "one channel,
two kinds of thing" as a tag, sends diagnostics down a pipe into `wc` by default, and dissolves
nothing.

The consequence the choice buys: separation arrives per program, as each declares (`date` first,
whose clockless complaint into a redirected file is the motivating loss). The consequence it
costs: `2>` works only on declaring programs, which is this shell's grammar being itself.
