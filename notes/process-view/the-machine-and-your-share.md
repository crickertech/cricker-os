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

## Ratified, and what moved at rebase

calef ratified this table on 2026-09-27 (UTC). Three numbers moved when the branch was rebased onto
a `main` that had taken them in the meantime. Numbers stay provisional until the queue lands them,
so a move at rebase needs no second ratification. Each is recorded where it lives.

| what | ratified | landed | why |
|---|---|---|---|
| the method | `abi::memory_region::USAGE = 5` | 5 | nothing collided |
| its records | `abi::usage` 0 to 6 | 0 to 6 | nothing collided |
| the page's name and layout | `machine_statistics_protocol`, magic `MACHSTA1`, a 64-byte header, one line per core | the same | nothing collided |
| where a child sees it | `0x005F_F000` | the same, now a pair page | the address-space map of milestone 206 (a program image has under 896 KiB) moved images and stacks away from it, so the page-table argument for it no longer holds; nothing collides with it |
| the progenitor's boot slot | 17 | 23 | the GPU and keyboard of milestone 600 (the graphical terminal stack is built in userspace) took 17 to 22; calef raised the table from 24 slots to 32 so a slot free on every boot exists |
| the session's slot | 21 | 20 | `grant_plan::SHELL_CONFIG_SLOT` took 21 (milestone 47 (navigation and naming)) |
| the spawn wire | `MACHINE_BIT`, bit 42 | bit 44 | `ARGS_BIT` took 42 (milestone 205 (how a foreign program is told what to do)) and `NAMESET_BIT` takes 43 (#1402) |
| the child's slots | `MACHINE_SLOT` 11, `SHARE_SLOT` 12 | the same | nothing collided |
| the owner's switch | `GRANT_MACHINE_PAGE`, default `true` | the same | |

The manifest fields `machine` and `share` were not in the table and stay provisional. They also
needed two bytes of the ELF manifest note's descriptor (`crates/manifest_note`), whose layout calef
ratified after this table was drawn; that amendment is raised on #1360.

## The table raise, measured

The one Kani harness family that ranges over the table size is `crates/component_plan`'s, whose
`any_slot` assumes `s < abi::CAPABILITY_TABLE_SLOTS`. The five harnesses' verification times on
patagonia (2026-09-27, one sample each, seconds):

| harness | 24 slots | 32 slots |
|---|---|---|
| `too_many_live_instances_never_silently_truncates` | 0.03 | 0.03 |
| `dependents_finds_exactly_the_non_target_instances_that_declared_it` | 2.89 | 2.47 |
| `the_device_split_partitions_the_mappings` | 3.43 | 4.02 |
| `a_missing_route_refuses_rather_than_falling_through_to_a_slot` | 3.87 | 3.28 |
| `a_plan_never_grants_a_right_the_declaration_did_not_ask_for` | 3.09 | 1.98 |

No change the noise can tell apart; a first 32-slot run read 8.5 s on the second harness and the
next read 2.5, which is the size of that noise. `crates/capability`'s harnesses fix their own small
table sizes (2, 3, 4, 16, 64) and do not read the constant, so the raise does not reach them.

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
