---
status: NOT-STARTED
raised: 2026-08-28
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 183. A physical-range index for capability holders, so revocation stops scanning every thread

Minted 2026-08-28, calef, out of a measured cost finding from milestone
142's own review pass (DECISIONS §102, the CRITICAL 1 fix for reclamation-time capability deletion,
and DECISIONS §132's capability-scoped `REVOKE`). Discussed directly with calef, who asked for this
to be tracked as a milestone rather than only a `BUGS` line.

The data-structure shape (extend an existing structure vs. build a new one, see "The
starting point" below) is a real choice but not one that needs calef's decision before a lane starts:
it is kernel-internal, changes no syscall, no wire format, and no name. A lane picking this up should
investigate and decide, the same latitude this project already gives reversible engineering calls.

## In brief

Every operation that deletes capabilities by physical range (`MemoryRegion::DESTROY`'s reclamation
sweep, §132's capability-scoped `REVOKE`) currently walks every live thread in the system,
checking every slot of every thread's capability table against the range in question
(`kernel/src/sched.rs:2293`, `delete_page_frame_caps_where`). This is correct and was deliberately
not fixed differently: capabilities in this system travel over IPC delegation, not only parent-child
spawn, so a capability overlapping a given physical range could legitimately be sitting in any
thread's table anywhere in the system, and there is no way to know that without either checking
everywhere or tracking where every capability actually went. This milestone is building the second
option: an index from a physical range to the capability-table slots that hold something naming it,
so these operations can look up affected capabilities directly instead of scanning.

## Why it matters, and why it is not urgent

**Measured, not guessed.** Milestone 142's own bench-regression lane bisected the cost precisely:
CRITICAL 1's reclamation sweep added +29,302 ticks to the `spawn_el0` benchmark (riscv64,
`cargo xtask bench --riscv --check`), a real, deterministic instruction-count regression from a real
correctness fix (a use-after-free: a reclaimed region's pages could still be named by a surviving
run capability). A follow-up fix, `CapabilityTable::delete_matching`
(`crates/capability/src/lib.rs:371`, doing the sweep in place instead of `get`-then-`delete` per
slot), roughly halved it, to about +13,895 ticks (~4.4% of baseline), and that remainder was
re-recorded as the honest, accepted baseline (`bench/baseline-riscv64.txt`, `bench/baseline-aarch64.txt`).

That remainder is inside the tripwire's own ±10% margin, and `spawn_el0` is a synthetic stress
benchmark, not a real workload: it calls `DESTROY` in a tight, unbroken 100-iteration loop, which
no real process does. The cost that exists today is real and measured, but not currently costing
anything a real workload would notice. This milestone exists so the option is designed and priced
before it is needed, not because it is needed now.

What sharpened the argument, 2026-08-28: the sweep's cost tracks `MAX_THREADS`, a constant this
tree keeps raising. The thread-table capacity lane (PR #566, `fix/thread-table-capacity`) raised `MAX_THREADS` from
128 to 256 on a measured peak of 130, and `spawn_el0` moved +340,749 ticks on aarch64 (+16.5%), well
outside the tripwire, with every other row flat. Live threads did not change; only the ceiling did.

Two costs live in `delete_page_frame_caps_where` and it is worth keeping them apart, because only
the second is this milestone's:

- Visiting slots nothing occupies. `generational_table::iter_mut` is
  `slots.iter_mut().filter_map(...)`: it yields live entries but walks the whole backing array to
  find them, so doubling the ceiling doubled the walk for 126 slots holding nothing. That is an
  implementation defect independent of this milestone's architecture, and bounding the scan is the
  fix; it is being handled on #566 rather than here.
- Checking every live thread's table. This is the cost this milestone removes, and it is
  unchanged by the above.

So the regression above is **not** evidence that this milestone became urgent, and it should not be
cited as if it were. What it does establish, which nothing did before, is that the whole-table walk
is coupled to a number that is raised roughly whenever a milestone adds a boot service, and
`notes/scheduler.md` records that 130 of 256 puts the same curve on course to arrive again. An index
makes revocation's cost independent of the ceiling; every alternative pays that coupling once per
raise.

## The starting point, not yet a decision

DECISIONS §132 (what `PageFrame::REVOKE` owes an overlapping run) already built the closest thing
to half of this index: `revoke::LogEntry` (`kernel/src/revoke.rs`) now carries a `PageMapSource`
(ratified name, `kernel/src/revoke.rs`) on every recorded page mapping, answering "which capability's
authority backs this mapping" per address space. What it does not answer, and what this milestone is
actually about, is the capability-table side: given a physical range, which capability-table
slots, across every thread, hold an object naming it, independent of whether that object is
currently mapped anywhere.

Two shapes worth pricing against each other, neither decided here:

- Extend the existing mapping-log machinery so a `PageMapSource`-shaped record can also be
  looked up by physical range across threads, not just within one address space's own log. Reuses a
  structure and a set of invariants this tree already built and tested (§132's own regression
  tests), at the cost of that structure's shape being sized for per-space mapping records, not
  necessarily for a global capability-table index.
- **A separate, dedicated index**: physical range to `(ThreadId, slot)` pairs, maintained at every
  mint, derive, and delete site. A cleaner fit for the actual query, at the cost of a new structure
  with its own correctness surface, kept in sync by every one of this kernel's capability-creating
  call sites rather than by the handful that already funnel through `record_mapping`.

**The correctness bar for either shape**: it must never let the sweep miss a capability that
genuinely overlaps (a security regression, silently reopening exactly the bug CRITICAL 1 fixed) in
exchange for going faster. A lane building this should treat "does this index ever fall out of sync
with the capability tables it indexes" as the central question, not an afterthought.

## Prior art

seL4 keeps a mapping database associating each frame mapping with the capability that made it, so
revoking a capability revokes its own mappings and derivatives and nothing else, the same shape
DECISIONS §132 built for the per-space mapping log. This milestone is the same idea one level up:
indexed by physical range, across the whole capability table rather than one address space's
mappings. (Stated from established prior art already cited in §132, not re-derived here.)

## What it does not decide

Whether the cost is ever worth paying at all. `spawn_el0`'s regression is accepted and re-baselined
today; this milestone does not argue that decision should change, only that the option should exist,
priced, for whoever next finds a real (not synthetic) workload where `DESTROY` or capability-scoped
`REVOKE` cost matters.

## BUGS

Not started; nothing built yet to carry its own `BUGS` section.

## Index row

Measured, not guessed: CRITICAL 1's reclamation sweep (DECISIONS §102/§132) added +29,302 ticks to `spawn_el0`, roughly halved by `CapabilityTable::delete_matching` to a ~4.4% remainder now
accepted as baseline. Capabilities travel over IPC delegation, not only parent-child, so the scan
cannot be narrowed to a subtree without an index; not urgent today since the remainder is inside
normal tolerance and no real workload calls `DESTROY` in `spawn_el0`'s tight loop.
