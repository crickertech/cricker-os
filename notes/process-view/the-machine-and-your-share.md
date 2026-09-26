# The machine and your share: how `free`, `vmstat` and `slabtop` were built

An appendix to [the process view](../process-view.md), written 2026-09-26 by the lane
`milestone/126-free` for milestone 126 (the `procps` package). It records how DECISIONS §225 (`free`
sees the machine and your share) was built, and the wire facts the ruling left for the building lane
to propose. The stem `the-machine-and-your-share` is a provisional name, minted with this file.

## What was built

Two mechanisms, one per line of `free`'s output.

`MemoryRegion::USAGE` is a new method on an existing object, gated by `ENUMERATE`, in the shape
`pmap` gave the address-space object under §114 (`ENUMERATE` extends to the address-space object).
It answers one figure per call, chosen by a record selector, the way `SURVEY` does. A view narrowed
to `ENUMERATE` learns a region's size, what it has spent, and what the spending went to; it is
refused every method that spends, splits or destroys. The root region capability now carries
`ENUMERATE`, so the right flows down every `SPLIT`.

The machine statistics page is one kernel frame holding the machine's counters: total and free
frames, and per core the busy and idle ticks, context switches, interrupts and run queue. The
counters live in the page itself, so there is no copy and no refresh. Each per-core word has one
writer, its own core, and each core has its own cache line. The kernel mints one capability to it,
for the progenitor, which hands it to the boot prompt's shell and keeps no copy. The shell sends it
back with the spawn request of a program whose manifest declares `machine`, and the progenitor maps
it read-only into that child.

The page travels with the session for a measured reason. The progenitor's capability table peaks
during the login block at 23 of its 24 slots, and the first build, which kept the page in the
progenitor for the life of the boot, took it to 24 of 24. The kernel's own record of that peak says
the next permanent capability should buy a slot back rather than spend the last one. Handing the
page to the session before the login block buys it back, and it is also the shape §225's words
describe: what a program sees is decided by what its session holds.

Four programs read them:

| program | holds | prints |
|---|---|---|
| `free` | the page and a view of the job budget | `Mem:` from the page, `Yours:` from the budget |
| `vmstat` | the page | run queue, memory, interrupts and switches per second, busy and idle |
| `slabtop` | a view of the job budget | where the budget's pages went, by kind of kernel object |
| `top` | `ps`'s three slots and the page | a machine line under its summary, which is what became of `tload` |

`slabtop` needed one thing the ruling did not name. A job budget's own pages are almost all carved
into job regions, and the objects live in those, so a count of the budget alone would say its
threads cost nothing. The object counts therefore sum over the region's whole live subtree, which
the region table already records through each region's parent.

## Proposed, and calef's to ratify

Every number and name here was minted by this lane and ships provisional.

| what | proposed | why this and not another |
|---|---|---|
| the method | `abi::memory_region::USAGE = 5` | the next number on the object, after `DESTROY` |
| its records | `abi::usage`: `SIZE` 0, `COMMITTED` 1, `FRAMES` 2, `RENDEZVOUS` 3, `ADDRESS_SPACES` 4, `THREADS` 5, `CHILDREN` 6 | one figure per call, `SURVEY`'s selector shape, so a new figure is a value and not a new return register |
| the page's name | the machine statistics page, crate `machine_statistics_protocol` | §225 said "machine memory page"; the page carries the scheduler's counters too, because a second page would be a second grant for one question |
| the page's layout | a 64-byte header line (magic `MACHSTA1`, frame bytes, tick rate, total and free frames), then one line per possible core | one writer per line, so no two cores share a written cache line |
| where a child sees it | `0x005F_F000`, read-only | the last page of the 2 MiB block a child's program and stack already use, so a spawn pays no new page-table frame. The first choice, `0x00f0_0000`, cost one, and on `x86_64` that took `top` over its job region |
| the progenitor's slot | 17 | past the entropy slot, the highest fixed boot slot before it |
| the session's slot | `spawnproto::MACHINE_PAGE_SLOT` 21 | one under the run-unvouched slot, for that slot's reasons |
| the spawn wire | `spawnproto::MACHINE_BIT`, bit 42 of word 2; the page is the last delegated capability | the next free bit, and last so every existing request keeps its order |
| the child's slots | `MACHINE_SLOT` 11, `SHARE_SLOT` 12 | named slots past `NETWORK_SLOT`, for the reasons `DOMAIN_SLOT` gives |
| the manifest fields | `machine`, `share` | `clock`'s family: nothing a command line designates |
| the owner's switch | `system_initializer::GRANT_MACHINE_PAGE`, default `true` | see below |

## The owner's switch is a boot-time constant, and that is an exception

§225 says owner policy grants the page to every login by default and can withhold it. The switch
built here is one constant in the progenitor, `GRANT_MACHINE_PAGE`, beside the owner's other
boot-time policy (the run-unvouched capability). It decides whether the boot prompt's session holds
the page, and a program spawned from a session without it says so on its second stream rather than
printing a machine of zero bytes.

The mechanism is already per session, since the page travels with each request. A per-login policy
is `login` handing each session the page or not, the way it hands on the run-unvouched capability.
Nothing hands the page to `login` yet, so the constant decides for the boot prompt alone, and it is
marked as provisional where a reader meets it.

One exposure comes with holding `GRANT`, which delegation needs. `PageFrame::REVOKE` needs `GRANT`
too, so a session could revoke the page from every holder, the kernel's mappings aside. The shell
never calls it; the same exposure applies to every frame a session holds with `GRANT`.

## What the counters cost

The context switch now does one load, one index and one add on its own core's line of the page,
before `switch_to`. The tick does two stores and two adds. The frame allocator stores two words
under the lock it already holds. None of it takes a lock that was not already held, and the IPC
fast path's footprint and the switch's instruction count are measured by `script/bench` in CI,
which is where this change will be judged.

## Swap

`free` prints no `Swap:` line and `vmstat` no `swpd`, `si` or `so` columns. calef ruled on
2026-09-26 that nife refuses paging out for now (the refusal in pull request #1356), and a row of
zeroes would say swap exists and is empty. Each program's `BUGS` section cites the refusal where the
rows would have been.
