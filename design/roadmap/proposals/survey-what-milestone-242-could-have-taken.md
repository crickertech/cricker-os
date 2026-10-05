---
status: PROPOSED
raised: 2026-10-04
milestone_dependencies: 242
decision_dependencies: 46
machine_requirements: none
specific_machine: none
needs_person: no
---
# Survey what milestone 242 could have taken

Raised on 2026-10-04 (UTC) by `lane/s46-reuse-default`, the lane that wrote the 2026-10-04
amendment to §46 (thin primitives or whole subsystems). Title and slug are drafts.

**In brief.** Milestone 242 (USB host and HID), in pull request #1629, wrote its xHCI and USB logic
here, as host crates `usb` and `extensible_host_controller_interface` (names provisional) with six
Kani harnesses. It was built before §46 made taking the default outside the kernel and the crates
Kani proves. This survey answers what taking would have bought. It is a measurement, not a rewrite.

**Reuse:** rust-osdev's `xhci` (0.9.2 on crates.io, MIT or Apache-2.0: register, context and ring
definitions) and Redox's `xhcid` (MIT, a whole userspace xHCI driver). These are the candidates to
measure; nothing is taken by this proposal.

## What it measures

1. For each of the two crates 242 wrote, the lines a candidate would have replaced and the lines it
   would not have.
2. Whether each candidate builds `no_std` for the three targets, and what it would need patched.
3. Which of 242's six Kani harnesses prove a claim the system rests on (§46's verification path)
   and which only prove code that could have been taken.

## What it decides

Nothing by itself. If the numbers say a candidate covers most of a crate and builds `no_std`, the
follow-up is a proposed milestone to adopt it, ruled by an architect like any dependency.
