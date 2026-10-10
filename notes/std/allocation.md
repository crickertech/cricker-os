# The `std` heap under threads: a cache per thread in front of one lock

Milestone 561 (a per-CPU allocator is what the current-CPU page was for) asked for a free list per
CPU id. This appendix records why it builds a cache per thread instead, what was surveyed before
writing anything, and what was measured. Written 2026-10-10 (UTC) by the milestone's lane.

## The problem, measured by milestone 812

`std`'s allocator (`patches/std-nife/overlay/std/src/sys/alloc/nife/mod.rs`) is
`crates/user_mode_heap` behind one spinlock. Milestone 812 (`std::thread::spawn` runs real threads
in one address space) made `std::thread::spawn` real and measured it with `std_heap_contention`: the
same 200,000 small allocations take 2.1 to 3.2 times longer split across four threads than done by
one (`notes/std/threads.md` on #1892). A thread preempted holding the lock makes the others spin out
their slices.

## Why per thread and not per CPU

The current-CPU page was built for a per-CPU allocator, and the block named the hard part: the id
can be stale the instruction after it is read, and this tree has no restartable sequences (`rseq`).

Staleness is the smaller half. A thread can be preempted between two instructions of a pop, and the
next thread on that core reads the same, fresh, id and pops the same list. User mode cannot turn
preemption off, so without `rseq` a per-CPU list is shared by every thread that may run on that core,
and each operation needs an atomic read-modify-write. The block's three candidates, checked:

| candidate | correct without `rseq`? | fast-path cost |
|---|---|---|
| per-list lock, slow path only | no: preemption mid-pop corrupts the list | none, which is why it fails |
| per-list lock on every operation | yes: a stale id locks another core's list, which is contention, not corruption | one uncontended acquire and release |
| compare-and-swap pop tolerating the wrong list | only with ABA protection: a double-width CAS `riscv64` lacks, or a tag in the pointer | one CAS, plus the tag scheme |
| owner check on free | not needed here | (blocks are fungible, see below) |
| **cache per thread** | yes: only its thread touches it, on any core | none |

An owner check is for lists that own address ranges. These blocks are fungible: any block of a size
class serves any request for it. So a free goes to the freeing thread's cache, as glibc's `tcache`
does (`malloc/malloc.c`, `_int_free` calling `tcache_put`; `tcache_thread_shutdown` returns a dying
thread's entries to the arena).

Cost of reaching each, after milestone 812, where each thread's current-CPU page is found through its
thread-pointer block (`sys/thread_local/key/nife.rs`):

- per thread: read the thread pointer, load the cache from the block.
- per CPU: read the thread pointer, load the page's address, load the CPU id, index the arenas, then
  the lock.

Per CPU is a superset of per thread. It wins on memory only when threads far outnumber cores (many
threads, few caches), and a process here has at most sixteen threads. Prior art agrees: TCMalloc runs
per-CPU only with `rseq` and "if RSEQ is unavailable ... reverts to" per-thread caches
([TCMalloc overview](https://github.com/google/tcmalloc/blob/master/docs/overview.md), read
2026-10-10); jemalloc's `percpu_arena` is the per-list-lock row above, an arena with its lock per
CPU ([jemalloc manual, `opt.percpu_arena`](https://jemalloc.net/jemalloc.3.html)).

Nothing here needs `rseq`, a new syscall or a new method. The current-CPU page loses the consumer it
was built for; what it still serves is `available_parallelism`'s allowance (milestone 812).

## Reuse: what was surveyed, 2026-10-10 (UTC)

Read from crates.io's API and each crate's source or README. The question for each: does it give
`std` a heap whose allocations stop taking one lock, on three bare targets, with no C and no `mmap`?

| candidate | license, latest release | what it is | why not |
|---|---|---|---|
| `talc` 5.1.1 | MIT, 2026-09-09 | `no_std` heap, `TalcLock` over `lock_api` | one heap, one lock; its README sends concurrent users to jemalloc or mimalloc |
| `linked_list_allocator` 0.10.6 | MIT/Apache-2.0, 2026-04-14 | first-fit list, spinlocked `LockedHeap` | `user_mode_heap`'s shape, same lock |
| `buddy_system_allocator` 0.13.0 | MIT, 2026-03-30 | buddy heap, spinlocked `LockedHeap` | same lock |
| `dlmalloc` 0.2.14 | MIT/Apache-2.0, 2026-05-16 | already in `std`'s own `Cargo.lock` (xous, wasm, SGX); an `Allocator` trait for system memory | one global lock (`std`'s `sys/alloc/xous.rs` wraps it in one); would replace `user_mode_heap`, which [§31 (the foreign-language seam)](../../design/decisions/0031-foreign-language-seam.md) rule 4 and `notes/std.md` keep as the one heap algorithm |
| `rlsf` 0.2.3 | MIT/Apache-2.0, 2026-07-27 | TLSF | "does not support concurrent access"; its README recommends a thread-caching allocator |
| `frusa` 0.1.3 (Motor OS) | MIT/Apache-2.0, 2025-11-22 | size-class allocator over a back end | its own benchmark falls from 16.8 to 8.6 operations per microsecond going from one thread to four; needs nightly `feature(test)`; tested on x86_64 only |
| `rsbmalloc` | (not checked further) | thread-cached in its `std` build | its `no_std` build drops the caches for spinlocks, and it needs `mmap` |
| `mimalloc` 0.1.52 | MIT, 2026-05-22 | C; per-thread heaps, cross-thread frees to a separate atomic list | a C toolchain inside `std`'s build on three targets, an OS layer over `mmap` that §31 forbids C to call, a TLS slot model this tree lacks, and it would replace `user_mode_heap` |
| `snmalloc-rs` 0.7.6 | MIT, 2026-10-02 | C++; per-thread allocators, remote frees by message passing | a C++ toolchain and a platform layer to port, for the same replacement |

Verdict: nothing to take. Every pure-Rust `no_std` allocator is one heap behind one lock, which is
what we have; the ones with per-thread caches are C or C++, or need `libc`. The layer that is missing
is small and sits in front of the heap we keep: `crates/allocation_cache` (name provisional),
about 150 lines. Its shape is glibc's `tcache`: bounded bins of 16, frees to the freeing thread, a
flush at the thread's end. It adds refills and spills batched under one lock acquisition. That
adapts a design and takes no dependency, so no ruling is owed.

## BUGS

- The `std` overlay does not use the cache yet: it needs milestone 812's thread-pointer block, which
  is on #1892 and not on `main` as of this writing. Until then this page is the design and the host
  tests, and the measurement table is owed.
