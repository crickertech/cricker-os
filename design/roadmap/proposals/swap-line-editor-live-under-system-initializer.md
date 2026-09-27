---
status: PROPOSED
raised: 2026-09-26
milestone_dependencies: none
decision_dependencies: unwritten
machine_requirements: none
specific_machine: none
needs_person: no
---
# Swap `line_editor` live under `system_initializer`

Raised by the lane for milestone 23 (a capability-routed component
OS with live replacement), which built state handoff and then checked why the interactive stack is
still not swapped. The full account is notes/interactive-stack-swap.md.

Two changes to the terminal contract, which eleven programs speak, so both are
wire-format calls for an architect: an additive quiesce opcode in `line_editor::proto`, and what a
reader parked in `OP_READLINE` is told when a swap begins. The recommendation for each, with the
options that lost, is in the note.

## What to build, once the two forks are answered

- A `component_plan` declaration for `line_editor`, after the ELF-note manifest lane (PR #1338)
  settles where a manifest lives, so the declaration is written once in its final home.
- A handoff page count in `component_plan::Handoff`: `line_editor`'s state with history is a little
  over one page. Or drop history across a swap and record it as a limit.
- The swap itself in `system_initializer`, which already holds every object `line_editor` was built
  from: `swapper`'s `ROLE_HANDOFF` sequence (drain, absorb or roll back, retire) against a real
  component, with `swish` parked at the prompt across it.
- A guest test with `swish`'s own view as a witness: the person's half-typed line survives, and no
  keystroke is lost.

## What it unblocks

The last `Outstanding` line of milestone 23's block, for the one interactive component a userspace
supervisor can reach today.
