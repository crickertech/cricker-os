# The suite's frame budget

*Name provisional. Until 2026-10-09 (UTC) this account lived in `kernel/src/testing.rs`'s doc
comment on `SUITE_PAGE_FRAME_BUDGET`, accumulating a dated entry per raise. Milestone 862
(comments state the constraint as it is now) moved it here. The constraint and the conventions
stay at the constant; the findings stay here; the measurement narratives went to that move's
commit message, where `git blame` finds each entry's own commit too.*

`SUITE_PAGE_FRAME_BUDGET` is what the whole system-test suite may leave unreturned, in frames,
checked once at the end of the run by `report_frame_ledger`. The gate's other half,
`SUITE_MIN_FREE_RUN`, is documented at that constant. The objects themselves are
[notes/frames.md](frames.md)'s subject; this note prices the ones the suite keeps.

## What the suite legitimately keeps

Measured conclusions, undated; each row's number is the reading that established it.

- `root_supervisor`'s construction budget: `SPLIT` from a parent with live children, refused at
  reclaim by construction.
- One page per kernel endpoint, never freed by design.
- One current-CPU page per live `AddressSpace` (about 103 spaces at the 2026-09-21 reading): freed
  by `Drop`, and `current_cpu_tests::the_page_is_returned_when_the_space_is_dropped` proves it.
- The memoized `login`: its construction budget is never returned on purpose, plus a 32-page
  channel untyped. The start-up re-derive test's second `login` carries a durable budget: 832
  pages plus two store caretakers and the timetable's staging buffer.
- Two `credential_service` instances (1,552 and 1,606 frames): nothing in this tree tears one down.
- The file servers' client staging windows (112 frames) and working-set heaps, the latter seen as
  §13 (capability revocation and untyped reclamation) mapping records.
- Terminals, compositors and raw-mode scaffolding the tests leave running the way a session leaves
  its own: five compositors about 1,900 frames, three terminals about 267 after `Cell` grew to 16
  bytes, four scratch pages per spawned client role.
- The `e1000e` DMA region: 18 pages and its domain's four page tables, held for the boot and
  reused by every later wiring.

## The conventions every change to the number follows

- Measure the merged tree's real run. Never sum two branches' numbers; per-test frame cost is not
  additive across unrelated changes.
- The tighter architecture of the pair carries the headroom.
- `+32` headroom, set 2026-08-27 and kept since, covers a measured two-frame local-against-CI
  divergence and the `caretaker_teardown` local flake's run-to-run spread.
- Raising or lowering it is a decision, not a formality: read the `[that test kept N frames]` lines
  the run prints, find who grew or shrank, and be able to say why.

## Every number it has been

| date (UTC) | measured | set to | cause |
|---|---|---|---|
| 2026-08-16 | 14,031 / 13,787 | baseline pair | first measured pair, milestones 54 and 55 merged |
| 2026-08-22 | 15,624 | | milestone 49 (users, login, and attribution): the memoized `login`, 640-frame budget |
| 2026-08-22 | | 16,968 | `login` capability-table fix, `CONSTRUCTION_PAGES` 640 to 1,408 |
| 2026-08-23 | | 18,627 | milestone 155 (a provisioning tool): a second credential service, 1,659 |
| 2026-08-25 | 18,621 to 18,626 (CI) | 18,632 | `MappedWindow` pulls `core::fmt` in, +54 KiB text on three programs |
| 2026-08-26 | 19,054 | 19,060 | milestone 49 channel-per-client: +320 budget, +104 client scratch |
| 2026-08-26 | 19,142 | 19,157 | milestone 47 (navigation and naming) `printenv`, measured with the above together |
| 2026-08-26 | 26,943 | 26,958 | milestone 142 (a text display good enough that people use it): scanout 128x64 to 1280x720 |
| 2026-08-27 | 22,075 / 21,853 | 22,090 | scanout retarget to 924x344: the first lowering |
| 2026-08-27 | 19,158 | 19,190 | milestone 49 terminal test scaffolding, margin doubled to +32 (branch) |
| 2026-08-27 | 22,107 / 21,868 | 22,139 | the two above reconciled on the merge |
| 2026-08-28 | 22,217 / 21,941 | 22,249 | milestone 169 (`kilo`, the smallest real text editor): raw-keystroke tests on a merged trunk |
| 2026-09-21 | 22,352 | 22,384 | one frame per live address space, the current-CPU page |
| 2026-09-26 | 22,482 | 22,514 | milestone 599 (a frame per filesystem client channel): seven staging windows, 112 frames |
| 2026-09-26 | 22,640 | 22,672 | `Cell` 8 to 16 bytes, three terminals outlive their tests (branch) |
| 2026-09-26 | 22,754 / 22,613 | 22,786 | the two above reconciled on the merge |
| 2026-09-26 | +14 (CI) | 22,800 | milestone 606 (a directory walk costs what it does on Linux): FS working set |
| 2026-09-27 | 23,450 | 23,482 | milestone 152 (durable delegation) fork 7: a second `login` proving start-up re-derive |
| 2026-09-27 | | 23,626 | fork 8: the durable budget +144, arithmetic |
| 2026-09-30 | | 24,829 | first full measured durable run, re-derive alone 5,426 |
| 2026-10-02 | 24,715 | 24,861 | timetable staging buffer 64 to 128 KiB |
| 2026-10-03 | 26,636 / 26,598 | 26,668 | milestone 719 (compositor confinement claim 25): five tests, each keeps its own |
| 2026-10-04 | 26,673 / 26,648 | 26,705 | overflow checks in the release builds |
| 2026-10-04 | 26,713 | 26,745 | milestone 494 (a driver for the network card a PC actually has): the `e1000e` DMA region |
| 2026-10-06 | 23,732 / 23,674 | 23,764 | lowered: tests stopped keeping endpoints and scanouts |
