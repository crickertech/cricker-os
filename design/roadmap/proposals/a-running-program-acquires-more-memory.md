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
it needs it. A writing-only lane wrote it and built nothing.

It is the program half of #1769 (concurrent login sessions), whose fork 4 calef ruled "D" the same
day: a `memory_broker` holding the pool left after the boot's carve, handing out geometric
increments, with a revoke capability for the owner. He asked that programs be able to ask it too.
So the two proposals share one broker and one request format, and this one defers to #1769 there.

Every name here is provisional: `memory_broker` (#1769's), the account, the verbs, the `std` slot
and the fixture.

## Why, and its honest rank

Against principle 1, the customer path is vacant (milestone 530 (name a customer)) and blocked on
milestones 801 (packages over the internet) and 802 (the trivial install). Nothing measured says
the package client needs more than one region. So this ranks below 801 and 802 unless the package
client is shown to hit the ceiling. Nobody has measured that.

Against the fatal risks, it guards one verdict that is already GREEN. Risk 1 (only software written
for nife runs on nife) went GREEN on milestone 121 (`ripgrep` on nife). But `notes/ripgrep-on-nife.md`
records the gap: "`ripgrep` never allocated much, because it stopped before searching", and the heap
"is untested against a real workload". The next real run of 121, its walk benchmark against Linux,
meets the ceiling. That makes this the work that keeps risk 1 GREEN once anyone searches.

### What the ceiling is, measured on 2026-10-06 (UTC)

A `std` program is built in one region of `STD_REGION_PAGES` (384 pages, 1.5 MiB), plus its image
for a file run by path (`image_region_pages`). Its address space, image, stack and page tables come
out of that region first. Its heap is what is left, about 256 pages (1 MiB), the number
`grant_plan`'s own comment gives.

What real programs want, on calef's Mac (macOS `peak memory footprint` from `/usr/bin/time -l`,
which counts dirty memory and not the clean image):

| workload | peak footprint | multiple of a `std` heap |
|---|---|---|
| `rg --threads 1 --no-mmap -c fn kernel crates` on this tree | 3.0 MiB | 3x |
| `rg --threads 1 --no-mmap` over this tree's `target/` | 52.8 MiB | 50x |
| `sqlite3 :memory:`, 100,000 rows and one index | 28.5 MiB | 28x |
| `rustc -O` on hello world | 24.0 MiB | 24x |

Not apples to apples: macOS footprint includes its `malloc` slack, `dyld` and the stack. The
direction is clear. A compiler, a database, `rg` on a large tree and milestone 99 (`git` on nife)
need tens of MiB.

What fails today, read from the code and not run: `MAP` returns `OutOfMemory`, the allocator returns
null, and `std` aborts through `handle_alloc_error`. Nothing grows.

A web server is unmeasurable today: no `std` program holds the network from the prompt yet
(milestone 595 (the shell runs a `std` program)'s BUGS).

## What the tree has, read on 2026-10-06 (UTC)

The allocator already grows lazily. `user_mode_runtime::heap` and `std`'s restatement
(`patches/std-nife/overlay/std/src/sys/alloc/nife/mod.rs`) map pages one `MAP` at a time,
geometrically, into a reserved virtual band. The band is `address_space_map::HEAP`, 256 MiB at
`0x4000_0000`. So the virtual reservation exists. What is missing is a second source of pages once
the first region is spent.

`memory_region` has `MAP`, `RETYPE`, `RETYPE_OBJ`, `SPLIT`, `DESTROY` and `USAGE`. Nothing grows or
shrinks a region (`crates/abi`). A process can hold several regions, and `MAP` takes any of them.

The kernel has no demand paging. A fault goes to the fault endpoint as a death (§26 (the fault
endpoint)), which reserves a word for a fault-reply protocol and builds none.
`design/open-design-ideas.md` lists "a userspace pager" as a trigger for `Tcb::SUSPEND`.

Regions are carved bump-only. Since the 2026-10-03 amendment to §16 (object revocation), a hole
returns once nothing above it is held, and not before.

`MAX_REGIONS` is 256, machine-wide. The suite's peak is 225 on aarch64, 224 on riscv64 and 125 on
x86_64 (milestone 601 (the region table prints its peak)'s ledger), so aarch64 has 31 spare.

A reply carries two words and no capability (`abi::reply`). `login_protocol` sends its capabilities
over a private result endpoint for that reason, and a broker grant must do the same.

There is no pool for a broker to hold yet. The kernel hands the progenitor one 12,288-page region
(48 MiB of QEMU's 256). The progenitor carves its own budget, the shell's, the job pool and the image
pool, then calls `cap_delete(ut)` on the rest, unmeasured (`crates/system_initializer`). The rest of
RAM stays with the kernel's frame allocator. #1769's fork 4 assumes a pool "left after the boot's
carve". Today that remainder is thrown away, and its size is the first slice's step 0.

Milestone 205 (how a foreign program is told what to do) carries `--mem`, a size a person names at
the prompt. That is complementary: it sizes the first region. It cannot help a program whose need
is only known while it runs.

Reuse: the broker is #1769's. The policy is `user_mode_runtime::heap`'s. Genode's quota upgrade
lends a shape and no code.

## The forks

Six, in the order calef rules on them. Forks 1 to 4 shape the first slice.

### 1. Who a program asks

| option | what it is | verdict |
|---|---|---|
| A | The session's `memory_broker`, on an account its spawner opened for it | recommended |
| B | Its spawner, which asks the broker on its behalf (Genode's quota upgrade) | refused |
| C | A kernel method that grows a region in place | refused |
| D | Every program holds the session's own broker capability | refused |

Under A, the spawner opens a child account under its own when it builds the program. The account is
a badged endpoint, with a ceiling no larger than what the spawner's account has left. The program
asks the broker directly, and the spawner closes the account at reap. The broker sees one request
format whether a session or a program is asking, so #1769's session account is simply the root of
the tree.

B is Genode's shape. It puts a second hop in every grow, and every spawner (the shell,
`job_undertaker`, a future `make`) must serve memory requests beside its own job. A gives the parent
the same control, the ceiling and the close, without being on the path.

C is a syscall-surface fork, refused on the model. A region is a fixed run of pages with a bump
watermark. Growing one in place needs the pages after it free, which its siblings' carves make
rare, or a non-contiguous region, which changes every caller and every Kani proof in
`crates/memory_regions`. It also hands the kernel a policy choice the untyped model keeps out.
A process can already hold a second region, so C buys nothing A lacks.

D loses accounting and reclaim: a dead program's increments stay charged to its session, and one
runaway can spend the session's share.

A is about elegance, not effort: B is about the same amount of work.

### 2. The allocator's shape

| option | what it is | verdict |
|---|---|---|
| A | One contiguous heap band, backed by successive regions: when `MAP` refuses, ask for an increment and keep mapping at the same cursor | recommended |
| B | Separate arenas, one per region, each with its own free list | refused |
| C | Reserve virtual space and back it on fault | refused for now |

A is a small change to code that exists. Both allocators already hold a cursor, `base + committed`.
On refusal they ask the broker, receive a region in a new slot, and keep mapping from that slot at
the same address. The free list sees one band, so adjacent free blocks coalesce across an
increment's edge. The allocator keeps a short array of region slots in the order it got them. The
256 MiB band bounds the whole heap, which is eight doublings of today's 1 MiB.

B fragments the free list at every region edge, and a large `Vec` cannot span two arenas.

C needs a pager: fault, message, map, resume. The kernel has none, and §26 chose dead-until-reaped.
Lazy backing would save only pages mapped and never touched. Geometric growth bounds those at about
half the heap, and today's allocator already pays that. A pager has stronger triggers (swap,
`mmap` of a file for milestone 121's `memmap2`). It should be designed for those, not for this.

One policy change rides along. `std` restates the growth policy by hand because it cannot depend on
`user_mode_runtime`. `user_mode_heap` is already generated verbatim into `std` by `cargo xtask
std-src`. So the grow decision (how much to ask for, when to give up) moves into `user_mode_heap` as
pure, host-tested logic, and the two copies become one.

Increment size: the first request asks for as much as the heap already has, doubling, never less
than the allocation that failed. That is the rule `grow` uses today within one region. Reaching 64
MiB from 1 MiB takes six increments and six region slots.

### 3. Giving memory back

| option | what it is | verdict |
|---|---|---|
| A | A program keeps its increments until it exits; the spawner's close returns them all | recommended, first slice |
| B | The allocator also returns its newest increment when the whole of it is free, with hysteresis | follow-on, after A is measured |
| C | The broker reclaims an increment from a live program under pressure | refused |

A region is the smallest unit the kernel returns, so there is no `madvise`. A program can give back
only its newest whole increment, since its heap is one band. Rust programs rarely shrink, and exit
is the moment that matters.

B is cheap in the allocator, but a program freeing and regrowing near an edge would thrash.
Measure first.

C is fork 4's revoke, and revoke kills (below). It cannot be a quiet reclaim.

The pool's own fragmentation is #1769's concern too. Two programs growing in turn interleave their
increments in the pool. When one exits, its holes return only when nothing above them is held
(§16). Geometric increments keep the holes few and large. The first slice measures the stranded
pages, the same number #1769's exit criterion 9 measures.

The 256 slots are the tighter limit. Each increment is a live region. Six increments per large
program, and a handful of large programs, is tens of slots against aarch64's 31 spare in the suite.
The broker should cap increments per account at 8, and say so in its refusal. Raising `MAX_REGIONS`
is cheap if the ledger shows the need: it is a kernel constant with a measured peak beside it.

### 4. What failure means

| option | what it is | verdict |
|---|---|---|
| A | A broker refusal is an allocation failure: null, so `try_reserve` sees `Err` and plain `alloc` aborts | recommended |
| B | The request blocks until memory frees | refused |
| C | The broker picks a victim and kills it (an OOM killer) | refused |

Under A, a program that handles failure can, and one that does not dies the way it does today, at
a larger size. The broker's refusal names one of three causes: the account's ceiling, the
per-identity cap, or an empty pool. That avoids the collapse `memory_region`'s BUGS records for
`OutOfMemory`. Each refusal is one attributed record in the system log (milestone 613 (a system log
service)), so the owner can see who is asking.

B deadlocks a program holding memory another needs, invisibly.

C needs authority to kill that the broker should not hold, and it puts policy (whom to kill) in a
server that only counts pages. Linux's OOM killer exists because overcommit promises memory that is
not there. Nothing here overcommits: a page is mapped only when it is paid for.

What the owner's revoke does, stated plainly: the broker `DESTROY`s the account's increments. Their
pages are revoked from the program's address space, its next touch faults, and the fault endpoint
delivers its death to its supervisor. So in this system revoke is a kill, decided by a person and
logged. A gentler "please shrink" notice would need a channel the program reads. That is a
follow-on that wants a customer first.

### 5. How this meets #1769's caps and reserve

Recommended, and nothing here is new beyond #1769's fork 4:

- Accounts form a tree. The root is the broker's pool. Under it sits one account per session, each
  capped by the owner's per-identity file when there is one. Under a session sit its programs'
  accounts. A grant is charged to every account above it, so a program cannot exceed its session's
  cap.
- The owner's reserve is outside the pool by construction. The boot shell's budget, the job pool and
  the image pool are carved before the broker is handed anything. That is rung 1 of the ladder: no
  grant can reach them, because the broker never held them.
- The boot shell's jobs get accounts too, and are the broker's only clients until #1769's
  sessions exist.

The open question is the default ceiling of a program's account. Recommended: no ceiling below its
session's, matching calef's "default none" for the per-identity cap. A spawner may set one, as
`--mem` would.

### 6. Whether native programs get it

| option | verdict |
|---|---|
| A. Yes, opt-in: `MemoryRegionHeap::init` takes an optional broker account; boot servers keep fixed budgets | recommended |
| B. `std` programs only | refused |
| C. Every program, boot servers included | refused |

A comes nearly free once the policy is in `user_mode_heap`. C is wrong for servers like
`credentialer` and `net_stack`, whose fixed budget is part of their confinement claim. A server that
grows from a session's pool would let one person's traffic spend another's memory.

## How others do it

All from memory and unchecked; the building lane owes a read of each.

- seL4: the kernel allocates nothing, and a memory server hands out untyped. Fork 1 A's shape.
- Genode: a child out of RAM quota asks its parent, which may upgrade it. Fork 1 B.
- Fuchsia: VMOs mapped into VMARs under job policy; memory pressure signals, then kills. Fork 2 C.
- Linux: `brk` and `mmap` backed on fault, overcommit, an OOM killer, cgroup `memory.max`. Forks 2
  C and 4 C.

## The first slice

One milestone, shippable without #1769's sessions. Whichever lands first builds the broker, and the
other adds to it.

0. Measure on all three architectures what the progenitor drops at `cap_delete(ut)` with `USAGE`. If
   it is too small to show growth past 16 MiB, raise the progenitor's 12,288 pages in the same
   slice and record why.
1. The progenitor hands that remainder to `memory_broker`. The boot shell holds the owner's revoke.
2. `memory_broker`: accounts, geometric grants delivered over a result endpoint, three named
   refusals, close and revoke.
3. The grow policy moves into `user_mode_heap`. Both allocators ask the broker when `MAP` refuses.
4. `std_runtime_protocol` gains slot 9 for the broker account. Empty means "not given", and the
   allocator behaves as it does today.
5. The progenitor opens an account per `std` job and closes it at reap, on `job_undertaker`'s path.

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
6. Slots. The suite's printed region peak stays under 256 on all three.
7. Host tests in `user_mode_heap` cover the grow policy: doubling, the floor, the band's end, the
   increment cap.
8. Measured, not gated: `MAP` calls per MiB, and the stranded pages after two growers interleave.
9. A `design/decisions/` section records the broker protocol, minted by the integrator.

## What is blocked until the ruling

Forks 1 to 4 block the first slice. The broker's request and reply are a wire format two programs
agree on, and they are shared with #1769. That makes the protocol the expensive part, and calef's
call. Forks 5 and 6 can ride on the recommendations unless he says otherwise.

## Follow-ons, proposed, unnumbered

- Returning the newest increment while running, fork 3 B, after the first slice's measurement.
- A "please shrink" notice before a revoke, when a program wants one.
- Milestone 121's walk benchmark under a grown heap, which this unblocks.

## BUGS

- `realloc` always copies (`std`'s nife allocator). A `Vec` growing to N briefly holds about 1.5 N.
  Growth makes that affordable without fixing it.
- The host measurements above are macOS figures. Nothing here measured a program on nife past its
  ceiling, because nothing can grow yet.
