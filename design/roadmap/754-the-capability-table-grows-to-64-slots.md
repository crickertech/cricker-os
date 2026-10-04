---
status: NOT-STARTED
raised: 2026-10-04
milestone_dependencies: 753
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 754. The capability table grows to 64 slots

Raised 2026-10-04 (UTC) by milestone 753 (trace the progenitor's login block peak) as its option A,
which calef chose that day on pull request #1608. The number 754 is provisional until the queue
lands it. *(Title and slug are drafts.)*

## Why

A gpu-and-keyboard boot under QEMU reaches 31 of the progenitor's 32 capability slots, and the next
capability added at boot halts that boot with no message. Milestone 753's trace itemised the 31 and
replayed the alternatives. calef chose to grow the table rather than move holdings into a new
process: the fixed table is the real constraint, and a builder process that exists to move slot
accounting is not worth a new program and protocol. Milestone 753 records that reasoning in full.

## What it would do

- `kernel::cap::CAPABILITY_TABLE_SLOTS` and `abi::CAPABILITY_TABLE_SLOTS` go from 32 to 64.
- The table's free-slot word widens from `u32` to `u64`, and `capability::MAX_SLOTS` follows it.
  Every proof and test that checks the word against the array runs at the new width.
- `abi::fault::FAULT_EP_SLOT` is `CAPABILITY_TABLE_SLOTS - 1`, so it moves from 31 to 63. Every
  supervisor and every child agrees on that number, and they are all rebuilt from this tree.
- The same on aarch64, riscv64 and x86_64, per §19 (architectural parity is a tenet).
- The size assertion beside `CapabilityTable` in `kernel/src/cap.rs` records the new layout, and
  `CAPABILITY_TABLE_SLOTS`'s doc gets the raise and its reason, as each earlier raise did.

## The test

- `script/swish-check`'s gpu-and-keyboard boot reads 31 of 64 at peak, and `swish-check` passes on
  every leg.
- The TCB still fits in its page. `crate::thread`'s page-fit assertion is the check, and it fails the
  build if not. Measured before the change at 1,984, 1,968 and 1,712 of 4,096 bytes used (aarch64,
  x86_64, riscv64); 32 more slots add 1,024 bytes.
- `script/test` passes on all three, including the supervision tests that read the fault slot.

## What it costs

No new memory: the table lives in each thread's TCB page, which is already allocated whole. The IPC
fastpath's footprint (`script/fastpath-footprint`) and the kernel stack temporaries that copy a
table were not measured at 64 and should be, before and after. `FAULT_EP_SLOT` is an ABI number;
nothing outside the tree is built against it today, so moving it now is cheap and moving it later
would not be.

## What it is not

Not growable tables. Whether a process's table should be sized for that process, as seL4's are, is
`design/roadmap/proposals/capability-tables-sized-per-process.md`. Sixty-four slots buy the time to
answer that question with a measurement rather than at the next wall.

## Index row

The progenitor's capability table sits at 31 of 32 on a gpu-and-keyboard boot. NOT-STARTED: the table grows to 64 slots on all three ISAs and the fault slot moves from 31 to 63.
