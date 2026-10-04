---
status: NOT-STARTED
raised: 2026-10-04
promoted_from: graphics-sessions-get-a-builder-of-their-own
milestone_dependencies: 753
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 754. Graphics sessions get a builder of their own, and the login block lets go sooner

Promoted from `design/roadmap/proposals/graphics-sessions-get-a-builder-of-their-own.md` on
2026-10-04 (UTC). The number 754 is provisional until the queue lands it. *(Title and slug are
drafts.)*

Raised by milestone 753 (trace the progenitor's login block peak) as its options B and C. calef
ruled B + C on 2026-10-04 (UTC), on pull request #1608.

## Why

The progenitor's capability table reaches 31 of 32 on a gpu-and-keyboard boot, and the next
capability added at boot halts that boot with no message. Milestone 753's trace found the peak in
two places: the login block and the `graphical_terminal` launch. Seven of the 31 are the gpu's and
the keyboard's boot grants. The spawn service holds them for the life of the boot so it can lend
them to each session's drivers, since milestone 715 (the spawn service holds the display grants,
and the shell holds none). They are the entire difference between a boot with no gpu (24), a gpu boot (28) and a
gpu-and-keyboard boot (31). `notes/capability-peak-trace.md` has the slot table.

## What it would do

### B

Build one small process early in the boot, before the progenitor's first plateau. Hand it
the seven device grants, a construction budget, and the session programs' images as blobs, the way
the progenitor gives `login` its caretaker. It builds a graphical session when a launch request
reaches it: everything `graphical_terminal_grants` through `graphical_terminal_session_children` do
in the progenitor today, about 480 lines. The progenitor stops holding the device grants and stops
building sessions.

### C

In the login block, delete `login`'s seven inputs as soon as `build_child(login)` returns,
build the shell after the block rather than before it, and drop `term_out` sooner.

## What the ruling settled and what it left open

Settled: B and C, not a raise of `CAPABILITY_TABLE_SLOTS`. Left to calef: the new program's name
and its request protocol. The build ships provisional ones and says so. Whether the shell reaches
the builder directly or through the spawn service is part of the protocol question; the lane
proposes one with its reason.

## The test

Milestone 753's replay predicts the progenitor's peak becomes 23 on all three `swish-check` boot
shapes. The build is done when `script/swish-check` reads that on aarch64 and riscv64,
`kernel::cap::CAPABILITY_TABLE_PEAK_MEASURED` records the new number, and the builder's own peak is
measured and recorded beside it. The `graphical_terminal` launch still passes on both boots that
attach a gpu.

## What it costs

A new program, a protocol, and the session code moved out of `crates/system_initializer`. The
builder holds device authority the progenitor no longer does, which is the narrowing milestone 715
started, one process further.

## Index row

The progenitor holds the gpu and keyboard grants and builds graphics sessions, which keeps its capability table at 31 of 32. NOT-STARTED: a builder process of its own takes both, and the login block releases its inputs sooner; replayed, the peak becomes 23 on every boot shape.
