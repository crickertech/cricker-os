# 601. The region table prints its peak, and a refused split's leak is recorded

**Status: BUILT.** 2026-09-26. *(Number provisional: 596 to 600 are held by open pull requests,
so the lane took the next free; the integrator confirms it at merge. Title a draft.)* Promoted by
the maintainer from two findings in the lane of milestone 152 (durable delegation), both about the
kernel's region table (`memory_region::MAX_REGIONS`, 256).

One PROPOSED question for calef is below, and nothing waits on it.

## Why

On aarch64 `main` the suite reached `timetable_tests` with 252 of 256 regions live. The lane of
milestone 152 measured that with a temporary print. The cause was `login_test_client` leaking a
four-page scratch region per run, which #1347 fixes. One more login test made it 253, and the
timetable's `--mem` split failed. It failed as a 60-second hang, in a test unrelated to logins.
The thread table had the same failure shape before its raise to 256 (`sched::MAX_THREADS`). The
answer there was a `threads:` line on every run. Nothing printed the region peak.

## What is built

- A `regions:` line in every suite's closing summary, beside `threads:`, on all three
  architectures. The runner is shared, so one change covers all three. It prints the peak, the
  ceiling, the spare, and the test during which the peak was last raised.
  `memory_region::PEAK_REGIONS` is kept under the `REGIONS` lock on the two paths that grow the
  table. `RegionTable::len` is the reading, and its name is provisional.
- The ledger beside `MAX_REGIONS`: per-module residue at the peak. With #1347 the peaks are
  aarch64 225, riscv64 224 and `x86_64` 125, all set in `timetable_tests`. Without it, this lane's
  CI printed 256 of 256 on aarch64 and 255 on riscv64, and the suite still passed.
- A frame low-water beside it. The maintainer widened the milestone to cover it after the lane of
  milestone 198 (a package manager) hit the same failure shape on frames. `memory::FREE_LOW_WATER`
  sees every allocation. The summary also prints the shortest longest-free-run at any test
  boundary, and any refused allocation with the test it happened in. The ledger is at
  `memory::FREE_LOW_WATER`.
- No gate on headroom, deliberately. The thread peak's reason holds. A second reason is written at
  `MAX_REGIONS`: nothing yet says which regions are meant to be permanent.
- A `# BUGS` entry at `RegionTable::split`. A split refused for a full table keeps the parent's
  bumped child count, so the parent can never be reclaimed, and the failure ratchets. Its
  reproduction is a doctest, which fails the day the rule changes.
- A sibling leak, fixed rather than recorded. `syscall::memory_region_split` minted the child,
  then returned `OutOfMemory` when the caller's capability table was full. The orphaned child had
  the same permanent consequence. It now destroys the child.
  `split_refused_for_a_full_capability_table_orphans_no_child` proves the parent comes back
  reclaimable with its budget whole, and it passes on all three architectures.

## PROPOSED: does the bump-only rule survive a full table

This is calef's call, in `notes/region-split-on-a-full-table.md`. The options are A (keep), B
(decide before mutating, recommended), C (mutate then roll back) and D (raise the ceiling instead).
Both reasons the rule was written for have lapsed. The two lock holds it protected became one
borrow in milestone 135 (the region claim, under loom). The "practically unreachable" full table
was measured at 256 of 256. It is cheap to reverse either way, and nothing is blocked on it.

## Follow-on

- **Proposed.** `design/roadmap/proposals/the-ntp-and-login-tests-give-their-regions-back.md`:
  those two modules hold a third of the region residue, and only the login service is on the held
  list.
- **Recorded.** A split refused for a full table leaks its parent: the `# BUGS` entry on
  `RegionTable::split` in `crates/memory_regions/src/table.rs`. Whether the bump-only rule changes
  is calef's, proposed in `notes/region-split-on-a-full-table.md`.

## Index row

**Built:** 2026-09-26

The suite now prints how close the boot came to the region table's ceiling and to running out of
page frames. It names the test that pushed it there, so the next full table is read off a
transcript instead of found by a hang in an unrelated test. `MAX_REGIONS` carries the ledger of
what holds regions at the peak. A refused split that leaks its parent is recorded where a reader
meets it, and a second road to the same leak through `MemoryRegion::SPLIT` is closed.
