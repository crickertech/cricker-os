---
status: BUILT
raised: 2026-09-26
built: 2026-09-27
promoted_from: the-progenitor-stack-has-no-measured-headroom
---
# 677. The progenitor's stack has no measured headroom

Promoted from `design/roadmap/proposals/the-progenitor-stack-has-no-measured-headroom.md` on 2026-10-03 (UTC). The number 677 was minted by the maintainer in a batch promotion of the proposal pile and is provisional until the queue lands it. Status BUILT 2026-09-27: built on PR #1409. *(Title and slug are drafts.)*

<!-- writing-standards: exception. Granted 2026-10-03 (UTC) by the maintainer minting this milestone, not ratified by an architect. Reason: this block was promoted unedited from design/roadmap/proposals/, which the prose scope excludes, so it meets the sentence and bold limits only after an edit that promotion does not make. Trimming it is a separate pass, and the exception goes when it is done. -->

Raised by lane `milestone/600-userspace-graphical-stack` (milestone
600 (provisional), the graphical terminal stack is built in userspace), when its first gate on top
of #1340 overflowed the progenitor's stack.

A measurement, then a constant or a gate.

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

## Built (lane `milestone/progenitor-stack`, 2026-09-27)

*Promotion and a number are the integrator's at merge. Names below are provisional.*

**Measured.** A kernel gauge, `kernel/src/progenitor_stack.rs`, paints the progenitor's stack pages
as `boot_progenitor` maps them and prints the high-water mark from the idle loop (the yield syscall
on `x86_64`) each time it settles at a new peak. On `script/swish-check`, out of 32,768 bytes:

| | aarch64 | riscv64 | x86_64 |
|---|---|---|---|
| debug, at the prompt | 19,000 | 18,976 | 18,184 |
| debug, `package install` | **32,440** | **32,184** | **31,304** |
| release, `package install` | 16,432 | 16,544 | not run |

So main had 328 bytes to spare on aarch64 debug, which is what three lanes in a row hit. The finding
above had inferred about 540. `boot`'s frame is 12,848 bytes of it in debug and stays live under the
spawn service.

**Gated.** `kernel::progenitor_stack::HEADROOM_FLOOR` is two pages. A boot that leaves less prints
`BELOW`, and `script/swish-check` fails on it, naming the command; it also fails if the gauge never
prints. `swish-check` strips the gauge's lines out of the transcript as they arrive, since they land
between a prompt and the next echo.

Fixed with the numbers. `INIT_STACK_PAGES` went from 8 to 12: the debug peak is now 66%, and the
gate fires 8.5 KB above it. The more elegant fix is to stop the spawn service standing on `boot`'s
frame, and that is proposed separately
([696-the-spawn-service-runs-outside-boots-frame.md](696-the-spawn-service-runs-outside-boots-frame.md))
because four open lanes were editing those functions. By the elegance test the raise wins on effort
and collision, not on elegance, and the note says so.

Also: `script/swish-check --release` (aarch64 and riscv64 tested), which the release column needed,
and `cargo xtask initrd-riscv` now packs the profile it builds.

notes/stack/progenitor-stack.md has the frames, the decision and the `BUGS`.

## Follow-on

- **Milestone 696.** Milestone 696 (the spawn service runs outside `boot`'s frame). The more elegant fix the raise to twelve pages did not take, running the spawn service outside `boot`'s frame: `design/roadmap/696-the-spawn-service-runs-outside-boots-frame.md`.

## Index row

The progenitor runs on eight stack pages and one lane's 560-byte frame growth overflowed it. Proposed: measure the headroom, then set a constant or add a gate.
