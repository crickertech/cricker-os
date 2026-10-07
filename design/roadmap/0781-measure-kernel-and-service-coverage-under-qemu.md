---
status: NOT-STARTED
promoted_from: measure-kernel-and-service-coverage-under-qemu
raised: 2026-10-04
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 781. Measure kernel and service coverage under QEMU

Raised 2026-10-04 (UTC) by the lane for milestone 745 (count the error paths no test reaches),
provisional. *(Title and slug are drafts.)*

## Why

`script/coverage` measures host crates only. The kernel, the services and the syscall-wrapping
crates hold 1,159 Result-family error paths and 602 Option-family ones that no coverage run sees,
about half of the tree's total ([`notes/untested-error-paths.md`](../../notes/untested-error-paths.md)).
Every cleanup path the count found is among them.

## Two ways, neither built

1. A QEMU TCG plugin that records executed blocks, mapped to source lines through DWARF. No kernel
   change, all three ISAs. Homebrew's QEMU 11.1.1 accepts `-plugin` but ships no plugin libraries,
   so one is built from QEMU's `contrib/plugins`. Blocks give lines, not regions, so a `?`'s error
   edge has to be found by address, and user programs that share virtual addresses have to be told
   apart by address space.
2. `-C instrument-coverage` on the test kernel with a `no_std` profiler runtime, dumping counters
   through semihosting at exit. The same regions as the host report, so `helpers/error_paths.py`
   reads it unchanged. It is a dependency, under §46 (thin primitives or whole subsystems), and each service would need its own dump.

The first is cheaper to try. The second is the better instrument. Choosing is an architect's call
because the second adds a dependency.

## Index row

`script/coverage` sees only host crates, so about half the tree's 1,761 error paths, including every cleanup path the count found, are invisible to it. The block prices two ways to measure kernel and service coverage under QEMU and chooses neither.
