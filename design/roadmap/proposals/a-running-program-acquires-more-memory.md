---
status: PROPOSED
raised: 2026-10-06
milestone_dependencies: none
decision_dependencies: unwritten
machine_requirements: none
specific_machine: none
needs_person: no
---
# A running program acquires more memory as it needs it

calef asked for this on 2026-10-06 (UTC): a running program should be able to get more memory when
it needs it. A writing-only lane wrote it and built nothing. calef ruled every fork on #1777 the same day.

It is the program half of #1769 (concurrent login sessions), whose fork 4 calef ruled "D" the same
day: a `memory_broker` holding the pool left after the boot's carve, handing out geometric
increments, with a revoke capability for the owner. He asked that programs be able to ask it too.
So the two proposals share one broker and one request format, and this one defers to #1769 there.

Every name here is provisional: `memory_broker` (#1769's), the account, the verbs, the `std` slot
and the fixture.

## Why, and its honest rank

Against principle 1, the customer path is vacant (milestone 530 (name a customer)) and blocked on
milestones 801 (packages over the internet) and 802 (the trivial install). Nothing shows their
package client needs more than one region, so this ranks below them.

Against the fatal risks, it guards a GREEN verdict. Risk 1 (only software written for nife runs on
nife) went GREEN on milestone 121 (`ripgrep` on nife). Yet `notes/ripgrep-on-nife.md` says
"`ripgrep` never allocated much, because it stopped before searching". 121's walk benchmark meets
the ceiling, so this keeps risk 1 GREEN once anyone searches.

### What the ceiling is, measured on 2026-10-06 (UTC)

A `std` program is built in one region of `STD_REGION_PAGES` (384 pages, 1.5 MiB), plus its image
for a file run by path. Its heap is what the build leaves, about 256 pages (1 MiB).

What real programs want, on calef's Mac (macOS `peak memory footprint` from `/usr/bin/time -l`,
which counts dirty memory and not the clean image):

| workload | peak footprint | multiple of a `std` heap |
|---|---|---|
| `rg --threads 1 --no-mmap -c fn kernel crates` on this tree | 3.0 MiB | 3x |
| `rg --threads 1 --no-mmap` over this tree's `target/` | 52.8 MiB | 50x |
| `sqlite3 :memory:`, 100,000 rows and one index | 28.5 MiB | 28x |
| `rustc -O` on hello world | 24.0 MiB | 24x |

Not apples to apples (macOS counts its `malloc` slack, `dyld` and the stack), but a compiler, a
database and `rg` on a large tree need tens of MiB.

What fails today, read from the code: `MAP` refuses, the allocator returns null, and `std` aborts.

## What the tree has, read on 2026-10-06 (UTC)

The allocator already grows lazily. `user_mode_runtime::heap` and `std`'s restatement map pages one
`MAP` at a time, geometrically, into `address_space_map::HEAP`, a 256 MiB band at `0x4000_0000`.
What is missing is a second source of pages once the first region is spent.

`memory_region` has `MAP`, `RETYPE`, `RETYPE_OBJ`, `SPLIT`, `DESTROY` and `USAGE`. Nothing grows or
shrinks a region (`crates/abi`). A process can hold several regions, and `MAP` takes any of them.

The kernel has no demand paging: a fault is a death on the fault endpoint (§26 (the fault
endpoint)).

Regions are carved bump-only. Since the 2026-10-03 amendment to §16 (object revocation), a hole
returns once nothing above it is held, and not before.

`MAX_REGIONS` is 256, machine-wide; the suite peaks at 242 on aarch64 (fork 7). A process's
capability table has 64 slots.

A reply carries two words and no capability (`abi::reply`). `login_protocol` sends its capabilities
over a private result endpoint for that reason, and a broker grant must do the same.

There is no pool for a broker yet. The kernel hands the progenitor 12,288 pages (48 MiB of QEMU's
256). After its carves, the progenitor calls `cap_delete(ut)` on an unmeasured rest
(`crates/system_initializer`). #1769's fork 4 assumes that leftover pool; measuring it is step 0.

Milestone 205 (how a foreign program is told what to do) carries `--mem`, which sizes only the
first region.

Reuse: the broker is #1769's, the policy `user_mode_runtime::heap`'s.

## The forks

Seven, in ruling order; fork 7 came out of fork 3.

### 1. Who a program asks

Ruled A, calef, 2026-10-06 (UTC): "Yes on Fork 1, broker accounts". A program asks the shared
`memory_broker` on an account its spawner opens with a ceiling and closes at reap.

Refused: a parent relaying requests (Genode), a kernel method growing a region in place, and
programs sharing the session's broker capability.

### 2. The allocator's shape

Ruled A, calef, 2026-10-06 (UTC): "Yes on Fork 2, one band with user_mode_heap". The existing 256
MiB band is backed by successive regions, mapped at one cursor. The grow policy moves into
`user_mode_heap`, which `std` already copies, so `std` stops restating it.

Refused: one arena per region. Refused for now: backing on fault, which needs a pager.

### 3. Giving memory back while running

Ruled, calef, 2026-10-06 (UTC): "Approve Fork 3, but we need to be more like linux and make the
limit [large] enough that nobody notices it." He had sent the first draft back because large programs
spike, and memory they cannot return looks like a leak. Fork 7 takes the second half of the ruling.
The [appendix](a-running-program-acquires-more-memory/returning-memory.md) has the arithmetic.

A region is the smallest unit the kernel takes back. nife cannot `madvise` a page in the middle of
a region, so whatever is returned must be a whole region.

Part 1, an allocation of 1 MiB (256 pages) or more gets its own region, above 2 GiB where the map
leaves room, returned at `free`. glibc's 128 KiB (from memory) was refused because a region costs
a machine-wide slot; 4 MiB, because it leaves 2 MiB spikes pinned in the band. The appendix prices
both.

Part 2, the band is trimmed from the top. When the newest band increment is wholly free and the
one below it is too, the newest goes back to the account and the cursor drops. Keeping one free
increment in hand is the hysteresis that stops thrashing at an edge. `user_mode_heap` is first-fit
over an address-sorted list, so the top tends to empty first. The appendix says how the allocator
detects a free increment.

Doubling keeps regions few, but one live object pins the newest increment. So the two kinds get
different policies:

| region kind | size | returned |
|---|---|---|
| band increment | doubling from 256 pages, capped at 2,048 (8 MiB) | from the top, keeping one free increment in hand |
| large allocation | the allocation's own size, page-rounded | at `free` |

The 8 MiB cap bounds what a pinned increment holds back; a 64 MiB small-object heap takes about 10.

Every region a program gets is at least 1 MiB, so its region count is at most its account's bytes
over 1 MiB. Fork 7 builds the limit on that. The broker never reclaims from a live
program on its own; that is fork 4's revoke, and revoke kills.

### 7. Limits nobody notices

Ruled A, calef, 2026-10-06 (UTC): "Approve Fork 7." The standard is Linux's `vm.max_map_count`
(65,530, from memory): a limit no ordinary program meets.
The [appendix](a-running-program-acquires-more-memory/returning-memory.md) has the measurements.

- The region table grows to 16,384 slots (1.8 MiB, measured). Child links and a free-slot list
  make every walk visit only what it concerns, where a walk over a chain is quadratic today.
- No per-program cap. Every region is at least 1 MiB, so a program's regions are bounded by its
  account's ceiling. The broker's slot budget is the table less a system reserve of 1,024, so no
  program exhausts slots before memory.
- A program holds a region's capability only while mapping it, so 64 capability slots suffice.
- Refused for now: a table-free design, seL4-style. Sizing the table from RAM comes first.

### 4. What failure means

Ruled A, calef, 2026-10-06 (UTC): "Approve Fork 4 A with reserve/commit."

| option | what it is | verdict |
|---|---|---|
| A | A broker refusal is an allocation failure: null, so `try_reserve` sees `Err` and plain `alloc` aborts | ruled, with reserve and commit |
| B | The request blocks until memory frees, which can deadlock unseen | refused |
| C | The broker picks a victim and kills it (an OOM killer) | refused |

Under A, a program that handles failure can, and one that does not dies as it does today. A
refusal names one of three causes (the account's ceiling, the per-identity cap, an empty pool) and is one attributed record in the system log (milestone 613 (a system log service)).

C puts kill authority and victim policy in a server that only counts pages. Linux needs an OOM
killer because overcommit promises memory that is not there; nothing here overcommits.

Stated plainly, the owner's revoke is a kill. The broker `DESTROY`s the account's regions, the
program's next touch faults, and its supervisor gets the death. A "please shrink" notice is a
follow-on that wants a customer.

#### Reserve and commit

calef chose Windows' model (`VirtualAlloc`'s `MEM_RESERVE` and `MEM_COMMIT`) so a runtime like the
JVM or Go can reserve far more than it uses, with no overcommit. Only commit can be refused.

| verb | what it does | charged |
|---|---|---|
| reserve | claims a range of the program's own address space | nothing |
| commit | backs part of a reserved range with a broker region, mapped there | the region, and its page tables |
| decommit | unmaps that region and returns it to the account | returned |
| release | gives the range back | nothing |

They live in the runtime, `user_mode_runtime` and `std`'s PAL, because commit and decommit make
calls. The bookkeeping, which ranges are reserved and which committed, is pure and goes in
`user_mode_heap` with the grow policy. Commit works in whole regions of at least 1 MiB, aligned,
which keeps fork 7's bound.

The two existing users become instances. The heap band is a 256 MiB reservation, committed upward
and decommitted from the top. A large allocation is its own reservation above 2 GiB, committed
whole and released at `free`.

Reserving costs nothing and the kernel never learns of it, so it needs no bound beyond the
address space (256 GiB on riscv64's Sv39, the smallest). Page tables are paid at commit, from one
long-lived table region per program, because `DESTROY` cuts the tables a region paid for and
everything beneath them (`abi::page_frame::MAP`). That binds today's band trimming too. The
appendix has the cost and the bound.

### 5. How this meets #1769's caps and reserve

Ruled as written, calef, 2026-10-06 (UTC): "Approve Fork 5". Under fork 4's reserve and commit,
every account counts committed memory only.

- Accounts form a tree: the pool, then one account per session (capped by the owner's per-identity
  file, if any), then its programs. A grant is charged to every account above it.
- The owner's reserve is outside the pool by construction, rung 1 of the ladder. The boot shell's
  budget, the job pool and the image pool are carved before the broker is handed anything.
- The boot shell's jobs get accounts too, and are the broker's only clients until #1769's
  sessions exist.

A program's default ceiling is its session's (fork 1); a spawner may set a lower one.

### 6. Whether native programs get it

Ruled A, calef, 2026-10-06 (UTC): "Approve Fork 6 A with both follow-ons". The follow-ons are
below.

| option | verdict |
|---|---|
| A. Yes, opt-in: `MemoryRegionHeap::init` takes an optional broker account; boot servers keep fixed budgets | ruled |
| B. `std` programs only | refused |
| C. Every program, boot servers included | refused |

C is wrong for servers like `net_stack`, whose fixed budget is part of their confinement claim.

## How others do it

All from memory and unchecked; the building lane owes a read of each.

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

From memory, unchecked; the building lane owes a read:

- KeyKOS, EROS and Coyotos: space banks, hierarchical capabilities to allocate storage with limits.
  Destroying a bank reclaims all allocated from it. The closest ancestor of forks 1 and 5.
- Fuchsia Zircon: VMARs reserve and VMOs back, like reserve and commit. Jobs form a tree with
  memory limits, yet it added pressure signals and a component-level killer.
- seL4 with CAmkES: each component's memory is fixed at build time, like fork 6's boot servers.
- Linux cgroups: `memory.high` applies pressure and reclaim below `memory.max`.
- Pressure notices: Android `onTrimMemory`, iOS `didReceiveMemoryWarning`, Windows memory resource
  notifications, Linux PSI.

## The first slice

One milestone, shippable without #1769's sessions. Whichever lands first builds the broker, and the
other adds to it.

0. Measure on all three architectures what the progenitor drops at `cap_delete(ut)` with `USAGE`. If
   it is too small to show growth past 16 MiB, raise the progenitor's 12,288 pages in the same
   slice and record why.
1. The progenitor hands that remainder to `memory_broker`. The boot shell holds the owner's revoke.
2. `memory_broker`: accounts, grants over a result endpoint, release, four named refusals, the
   slot budget, close and revoke.
3. The grow policy moves into `user_mode_heap`. Both allocators ask the broker when `MAP` refuses,
   give large allocations their own regions, and trim the band from the top.
4. `std_runtime_protocol` gains slot 9 for the broker account. Empty means "not given", and the
   allocator behaves as it does today.
5. The progenitor opens an account per `std` job and closes it at reap, on `job_undertaker`'s path.
6. The region table as fork 7 rules: 16,384 slots, child links, a free-slot list. Programs hold a
   region's capability only while mapping it.
7. Reserve, commit, decommit and release in the runtime, with page tables from one table region
   per program.

### Exit criteria a stranger could check

One `cargo xtask` gate that `script/test` runs, exiting 0 under QEMU on aarch64 (`virt`), riscv64
(`virt`) and x86_64 (`q35`):

1. Growth. A `std` fixture, `heap_grower`, allocates and writes 16 MiB in 1 MiB pieces, then one
   `Vec` of 8 MiB. Each piece's last byte reads back. It prints its committed bytes, and the gate
   checks they exceed 16 MiB.
2. Refusal. Under an account ceiling of 4 MiB, `try_reserve` returns `Err`, and the broker's log
   record names the ceiling.
3. The reserve holds. One grower spends the pool until refused. Then the boot shell spawns a native
   job, which runs, and runs `revoke` on the grower's account. The job pool's and the shell's
   `USAGE` figures are unchanged before and after.
4. Revoke kills and returns. The revoked grower dies through its fault endpoint. The broker's free
   count returns to its starting value.
5. Death returns everything. A grower killed mid-growth leaves the broker's free count and the
   region table's count where they began.
6. A spike returns. `heap_grower` holds a baseline, then allocates 64 MiB in 4 MiB buffers and 32
   MiB of 4 KiB objects, frees them all and keeps running. The account's held total, read from the
   broker, falls to within 8 MiB of the baseline before the program exits.
7. Limits nobody notices. A grower keeps 1 MiB buffers live until its account ceiling refuses
   it. The refusal names the ceiling, not slots. Its capability table stays within 2 slots of its
   baseline, and the region table keeps at least 1,024 free.
8. Host tests in `user_mode_heap` cover the grow policy, the threshold, the highest free block,
   withdrawing a range and the hysteresis.
9. Host tests count the records each walk visits: a `USAGE` walk visits its subtree plus one, and
   a return visits the parent's children. A chain of 4,096 under a full table stays under 1 ms.
10. Reserve and commit. On all three, a fixture reserves 32 GiB, commits 1 MiB at 16 places across
    it, then decommits half. After each step the account's held total equals the committed
    regions plus their table pages, and never moves on reserve or release.
11. Measured, not gated: `MAP` calls per MiB, and the stranded pages after two growers interleave.
12. A `design/decisions/` section records the broker protocol, minted by the integrator.

## What is blocked until the ruling

Every fork is ruled. The broker's request and reply, shared with #1769, are a wire format for
the `design/decisions/` section at merge.

## Follow-ons, proposed, unnumbered

Fork 6's two, each a proposal on #1786:

- `a-client-pays-for-its-session-in-a-server`: client-paid server memory, after Genode.
- `a-program-is-asked-to-give-memory-back`: the advisory "please shrink" signal fork 4 named.

Others:

- Milestone 676 (the NTP and login tests give their regions back), shrinking the suite's residue.
- Growing a large allocation in place by mapping the pages after it, as `mremap` does.
- Milestone 121's walk benchmark under a grown heap, which this unblocks.

## BUGS

- `realloc` always copies (`std`'s nife allocator). A `Vec` growing to N briefly holds about 1.5 N.
  Growth makes that affordable without fixing it.
- Page tables built for a commit stay until the program exits, charged to its account.
- The host measurements above are macOS figures. Nothing here measured a program on nife past its
  ceiling, because nothing can grow yet.
