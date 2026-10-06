# Returning memory while running: the measurements behind forks 3 and 7

An appendix of [a running program acquires more memory](../a-running-program-acquires-more-memory.md).
It holds the arithmetic, so the main text can state the decisions. Written 2026-10-06 (UTC).

## The region table's peak, per architecture

Milestone 601 (the region table prints its peak) made every suite run print a `regions:` line: the
most regions live at once, of `MAX_REGIONS`, and the test during which the peak was set. The 225 in
`kernel/src/memory_region.rs`'s ledger came from that line on 2026-09-26, on `main` at `484f3ebe`.

Read again on 2026-10-06 (UTC) from the `test` job of three merge-queue CI runs (37512763962,
37512623317 and 37512004040). All three agree:

| suite | peak | spare | set during |
|---|---|---|---|
| aarch64 | 242 | 14 | `timetable_tests::a_calendar_entry_keeps_time_by_its_granted_clock_and_a_step_moves_it_by_s3` |
| riscv64 | 241 | 15 | the same test |
| x86_64 | 136 | 120 | the same test |
| x86_64, a second machine configuration | 112 | 144 | the same test |

Each architecture also runs a kernel-only pass that peaks at 8. The suite's peak is mostly residue:
regions earlier tests left behind, which the ledger itemizes. The peak rose 17 on aarch64 in ten
days with nothing in this proposal built.

An interactive boot's own count was not measured. It is far lower than the suite's, so the suite
is the binding case for `script/test` and the ceiling a stranger meets first.

## Slots fork 3 adds

Per growing program, at the account caps of fork 3:

| what | regions |
|---|---|
| band increments, doubling from 256 pages to an 8 MiB cap | 10 for a 64 MiB small-object heap, 16 at the cap |
| large allocations of 1 MiB or more, live at once | up to 16 |
| total per account | up to 32 |

One such program in the suite would take aarch64 from 242 to 274, past 256. A typical program
uses far fewer. `rg` over this tree, at 3.0 MiB on macOS, would take two band increments and
likely no large regions. But a cap a single program can exceed is not a cap.

The per-process capability table is the second ceiling. It has 64 slots
(`kernel/src/cap.rs`, `CAPABILITY_TABLE_SLOTS`), a `std` program uses its 9 fixed slots and some
working ones, and every region it holds takes one more. 32 regions fit; 128 KiB large allocations
would not.

## The threshold

glibc's 128 KiB default (from memory, with its dynamic rise toward 32 MiB on 64-bit) is priced for
`mmap`, whose cost is a per-process VMA. Here a large allocation costs:

- a broker round trip: a `CALL`, then the grant over the result endpoint, since a reply carries no
  capability (`abi::reply`);
- a `SPLIT` in the broker and a `DESTROY` at free;
- one region slot machine-wide and one capability slot in the process;
- one `MAP` per page, the same as in the band.

At 1 MiB, the 256 `MAP` calls dominate, and the fixed part is a few percent at most. That is an
estimate; the first slice's measured criterion counts it. At 128 KiB the fixed part is a third of
the cost or more, and slot use is eight times higher for the same bytes.

## What raising `MAX_REGIONS` costs

The table is `RegionTable<MAX_REGIONS>`, a `generational_table::Table` of `Option<Region>` and a
`u32` generation per slot. `Region` holds four `u64` page counts, a parent name, a pin flag, a
`u32` child count and a `Spent` record of eight `u64`s. That is about 112 bytes, so about 116 per
slot with the generation. This is estimated from the fields, not measured with `size_of`:

| `MAX_REGIONS` | table size |
|---|---|
| 256 | about 29 KiB |
| 512 | about 58 KiB |
| 1,024 | about 116 KiB |

Names do not limit it. A name is a generation in the high 32 bits and a slot in the low 32
(`generational_table`), and `Table::new` asserts only `N <= u32::MAX`.

The proofs do not depend on it. `crates/generational_table`'s Kani proofs use `Table<u8, 2>`.
`crates/memory_regions`' four (`split_stays_within_budget_and_progresses`,
`a_retyped_run_is_the_pages_asked_for_and_zero_is_one`, `destroy_never_frees_a_child_to_the_allocator`,
`coalescing_never_reclaims_a_held_page`) are over scalar watermarks. The loom test uses
`RegionTable<2>`. A raise re-proves nothing.

The walks do. `USAGE`'s subtree records and the reclaim scans walk every live region, each one's
parent chain, under the region lock with interrupts masked. The table's sweep stops at one past the
highest live slot, so cost tracks what is held. The 2026-10-03 security audit measured 42 µs for a
root over the deepest chain 256 regions allow, and 62 µs for its leaf. Both grow with chain depth
times table occupancy. At 1,024 that could be 16 times worse, about 1 ms masked, though no real
workload splits a chain that deep. That figure is an extrapolation, which is why fork 7 A's exit
criterion re-measures it, and why 512 is the fallback.

## How the allocator sees a free increment

The allocator keeps one record per band increment: its region's capability slot, its start and its
length, in the order granted. `user_mode_heap` gains two pure calls, host-tested like the rest of
it: the highest free block, and withdrawing a range from the free list.

After a `dealloc` that coalesces into the highest free block, the allocator compares that block's
start with the newest increment's start. If the block reaches down past it, the increment is wholly
free. If the increment below is wholly free as well, the newest is withdrawn from the free list and
released to the broker, which `DESTROY`s it and uncharges the account. The cursor drops by its
length. The broker does the `DESTROY` so the account's books and the pages move in one place.

The check is cheap when the free list knows its tail, which it can keep beside its head. Without
that it is a walk of the free list on every `dealloc` that touches the top.

The job's own build region is never trimmed. It is the baseline the account returns to.
