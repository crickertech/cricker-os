---
status: PROPOSED
raised: 2026-10-04
milestone_dependencies: 753
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: yes
---
# Graphics sessions get a builder of their own

Raised 2026-10-04 (UTC) by milestone 753 (trace the progenitor's login block peak), as its
recommended option B. Title and slug are drafts, and the program this would add needs a name from
an architect, which is why `needs_person` is yes.

## Why

The progenitor's capability table reaches 31 of 32 on a gpu-and-keyboard boot, at two places: the
login block and the `graphical_terminal` launch. Seven of the 31 are the gpu's and the keyboard's
boot grants, held for the life of the boot so the spawn service can lend them to each session's
drivers (milestone 715 (the spawn service holds the display grants, and the shell holds none)). They are the entire difference between a boot with no gpu (24), a gpu boot
(28) and a gpu-and-keyboard boot (31). `notes/capability-peak-trace.md` has the slot table.

## What it would do

Build one small process early in the boot, before the progenitor's first plateau. Hand it the seven
device grants, a construction budget, and the session programs' images as blobs. It builds a
graphical session when the shell's launch request reaches it, which is everything
`graphical_terminal_grants` through `graphical_terminal_session_children` do in the progenitor
today. The progenitor stops holding the device grants and stops building sessions.

Replayed on milestone 753's traces, the progenitor's peak becomes 24 on all three boot shapes.

## What it costs

- A program name and a request protocol, both an architect's.
- Whether the shell reaches it directly or through the spawn service is a design choice the
  proposal leaves open.
- About 480 lines move out of `crates/system_initializer`.
- The new process's own peak has to be measured once it exists. The launch plateau suggests the low
  twenties.

## What it is not

Not a raise of `CAPABILITY_TABLE_SLOTS`, and not a change to which grants the kernel makes at boot.
