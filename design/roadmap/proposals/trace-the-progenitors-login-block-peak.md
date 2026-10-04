---
status: PROPOSED
raised: 2026-10-03
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# Trace the progenitor's login block peak

Raised by the lane for milestone 715 (the spawn service holds the display grants, and the shell
holds none). *(Title and slug are drafts.)*

## Why

Milestone 715 moved the gpu's four and the keyboard's three grants from the shell into the spawn
service for the life of the boot. They now sit on the progenitor's login block, which is its
capability table's peak: a gpu and keyboard boot reads 31 of 32, and
`kernel::cap::CAPABILITY_TABLE_PEAK_MEASURED` was moved from 30 to 31 to record it. That constant's
own doc says the next capability held across the peak "buys a slot back or raises
`CAPABILITY_TABLE_SLOTS`", and that which grants sit on the peak "is not traced".

A slot cannot be bought back from a peak nobody has itemised. Raising the table is the other answer,
and it costs every thread's table, so it should be chosen against a measured alternative rather than
by default.

## What it would do

Name every capability the progenitor holds at its login block's high-water mark, on a boot with no
gpu (24) and on a gpu and keyboard boot (31), and say for each whether it must be held there. The
output is a table in `notes/`, and either a slot released earlier or a recorded reason none can be.

## What it is not

Not a raise of `CAPABILITY_TABLE_SLOTS`. That is an architect's call and this is the data it wants.
