# The progenitor's stack has no measured headroom

**Status: PROPOSED 2026-09-26.** Raised by lane `milestone/600-userspace-graphical-stack` (milestone
600 (provisional), the graphical terminal stack is built in userspace), when its first gate on top
of #1340 overflowed the progenitor's stack.

**Gate: NONE.** A measurement, then a constant or a gate.

## The finding

The progenitor runs on eight stack pages (`kernel::user::INIT_STACK_PAGES`, whose doc calls 32 KiB
"generous"). `system_initializer::boot`'s own frame is 12,768 bytes in the debug build `swish-check`
boots (`sub sp` in its prologue, aarch64), and it lives for the whole boot because the spawn service
runs inside it.

This lane's first version grew that frame by 560 bytes. `swish-check` on aarch64 then died at
`package install greeting` with a data abort 24 bytes below the stack's last page (`far
0x4f8fe8`, `sp 0x4f8fe0`). Main passed the same script. So main's deepest path reaches within
roughly 540 bytes of the guard, inferred from those two numbers rather than measured. The lane cut
its own growth to 16 bytes and went green, which fixes its change and not the margin.

Nothing measures this stack. `script/stack-depth-check` walks kernel thread stacks only, and
`script/stack-frame-check` gates single frames.

## What to find out, then build

1. Measure the progenitor's high-water mark on `swish-check` (a watermark like milestone 84 (stack high-water: measure kernel stack depth), or a
   call-graph walk from `_start`).
2. Then either raise `INIT_STACK_PAGES` with the number beside it, or gate the depth, so the next
   lane that adds a local to `boot` fails loudly instead of at a prompt.
