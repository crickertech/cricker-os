---
status: NOT-STARTED
raised: 2026-09-03
milestone_dependencies: none
decision_dependencies: unwritten
machine_requirements: none
specific_machine: none
needs_person: no
---
# 334. Colour and the pager: the spawn protocol's other two thirds

Filed 2026-09-03 as an unnumbered proposal by the milestone 247 sweep,
from milestone 40's block; numbered 2026-09-19 by milestone 433. **Premise re-checked 2026-09-19 and
it holds.** DECISIONS §106 is still the only narrowing that shipped, `components/src/mdr.rs` still
carries "No pager, and the reason is authority rather than effort" in its own module doc, and nothing
in the spawn wiring tells a stage it ends at a real screen. No program in this tree colours its
output.

Both halves widen a protocol two programs agree on, which is the same shape as
DECISIONS §106 itself and the same reason §106 was calef's. A spawn-protocol bit is on a wire, so it
cannot be un-shipped once a program is written against it.

**In brief.** DECISIONS §106 narrowed the spawn protocol for a tail stage's *primary output*, which
is what lets `doc <page>` render at the prompt with no `| wc` in front of it. It built the narrowest
slice that unblocked that one command. Two thirds of the original scope are untaken: a bit telling a
stage that it ends at a real screen, which is the honest replacement for `isatty` and is what colour
needs, and a way to grant one line of *input* without granting the keyboard, which is what a pager
needs. Both want the same wiring bit §106 built one third of.

## Why this matters

The first half is what stops this system growing a dishonest `isatty`. Unix programs decide about
colour by asking the kernel what their file descriptor is attached to, which is ambient authority
answering a question about presentation. A capability system can say it properly, as a bit the
spawner passes, and every program that wants to colour output will need that bit. Until it exists,
the choices are no colour anywhere or a program guessing, and a guess here becomes a convention
before anybody decides it is one.

The second half blocks the pager outright. A pager needs one line of input at a time and must not
hold the keyboard, because holding the keyboard is exactly the authority a confined viewer should
not have. There is no way to express that today, so `mdr` renders and cannot page. Milestone 40's
own refusal of `ratatui` said the tree needs its terminal contract first; this is a piece of that
contract.

Left alone, this is a decision nobody meets until they try to write the sixth program that wants a
screen, and by then there will be a habit rather than a protocol.

## Where it came from

Milestone 40's block: *"The other two thirds of §106's spawn-protocol narrowing: a bit telling a
tail stage it ends at a real screen (colour, the honest `isatty` replacement), and a way to grant
one line of input without granting the keyboard (the pager). Both widen a protocol two programs
agree on, so both are an architect's call."*

`notes/documentation.md`'s "Where this goes next" is the only other record, and it heads its list with
these two: *"DECISIONS §106 took the narrowing for a tail stage's primary output; it did not extend
the same bit to 'tell this stage it ends at a real screen' ... or to granting one line of input
without granting the keyboard. Both still want the wiring bit this entry originally scoped for all
three."*

## Index row

DECISIONS §106 narrowed the spawn protocol for a tail stage's primary output, which is what lets
`doc <page>` render at the prompt with no `| wc` in front of it, and built the narrowest slice that
unblocked that one command. Two thirds of the original scope are untaken: a bit telling a stage that
it ends at a real screen, which is the honest capability-shaped replacement for `isatty` and is what
colour needs, and a way to grant one line of input without granting the keyboard, which is what a
pager needs. The first is what stops this system growing a dishonest `isatty`, since Unix decides
about colour by asking the kernel what a file descriptor is attached to and a capability system can
say it properly as a bit the spawner passes. The second blocks the pager outright: holding the
keyboard is exactly the authority a confined viewer should not have, there is no way to express the
narrower thing, and so `mdr` renders and cannot page. Both widen a protocol two programs agree on,
so both are an architect's.
