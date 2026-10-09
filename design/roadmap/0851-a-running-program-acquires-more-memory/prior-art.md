# How others hand a running program more memory

An appendix of [milestone 851 (a running program acquires more memory as it needs
it)](../0851-a-running-program-acquires-more-memory.md). All from memory and unchecked; the
building lane owes a read of each.

- seL4: the kernel allocates nothing, and a memory server hands out untyped. Fork 1 A's shape.
- Genode: a child out of RAM quota asks its parent, which may upgrade it. Fork 1 B.
- Linux: `brk` and `mmap` backed on fault, overcommit, an OOM killer, cgroup `memory.max`. Forks 2
  C and 4 C.
- Windows: `VirtualAlloc` reserves, then commits against a system commit limit. No overcommit and no
  OOM killer: a commit past the limit fails. Fork 4's reserve and commit.
- glibc: chunks of `M_MMAP_THRESHOLD` (128 KiB, rising dynamically) or more get their own `mmap`.
  The heap top is trimmed, and `malloc_trim` `madvise`s free pages inside. Fork 3, plus `madvise`.
- jemalloc and mimalloc: free pages inside the heap go back by `madvise` after a decay delay. nife
  cannot do that below a region, which is why fork 3 separates large allocations at all.

Also from memory:

- KeyKOS, EROS and Coyotos: space banks, hierarchical capabilities to allocate storage with limits.
  Destroying a bank reclaims all allocated from it. The closest ancestor of forks 1 and 5.
- Fuchsia Zircon: VMARs reserve and VMOs back, like reserve and commit. Jobs form a tree with
  memory limits, yet it added pressure signals and a component-level killer.
- seL4 with CAmkES: each component's memory is fixed at build time, like fork 6's boot servers.
- Linux cgroups: `memory.high` applies pressure and reclaim below `memory.max`.
- Pressure notices: Android `onTrimMemory`, iOS `didReceiveMemoryWarning`, Windows memory resource
  notifications, Linux PSI.
