# PageFrame capabilities: shared memory a process owns

*Renamed 2026-08-24 (DECISIONS §113, milestone 158): the kernel object this file describes was
called `Frame` until this rename. It is now `PageFrame`, the standard OS term for a physical page
in a virtual memory system, chosen because bare `Frame` collides with `crates/compositor`'s own use
of "frame" for a rendered screen update, an unrelated concept. The section below headed "A note
about stack frames" is historical and left as it read before the rename: `frame` there always meant
a CPU call frame (compiler stack-size accounting), never this object, and renaming it would create
the exact collision this rename exists to remove.*

DECISIONS §10 (process model) has a one-line rule for the data path: IPC carries control, shared memory carries
data. The endpoint moves the small stuff (a length, a request code) and the bulk bytes live in a
page both parties can see, so the kernel never copies them. For a long time nife honored that
rule only by accident of setup: the kernel allocated the shared page and mapped it into both the
console client and server at spawn, and both sides found it at a fixed virtual address they had
agreed on in advance. The sharing was real but frozen. Two processes could share memory only if the
kernel decided, at the moment it created them, that they should.

A `PageFrame` capability makes shared memory a thing processes *do* instead of a thing the kernel
*pre-arranges*. This note is that object.

## What a PageFrame is

A capability whose object is a single physical page. Its address is its identity:
`Object::PageFrame(pa)` names the page at `pa`, and a process can never forge one, because the only
ways to hold a `PageFrame` are to retype it out of your own untyped or be handed it by someone who
has it, and both keep the object intact. Its rights say what you may do with the page: `READ` to map
it read-only, `WRITE` to map it read/write, `GRANT` to pass it on.

## Retype, then map: two operations, not one

seL4 splits "get a page" from "put it in your address space," and so do we, because the split is what
makes a page a first-class, delegatable object rather than something that only exists mapped.

- `Untyped::RETYPE` carves one page out of the caller's untyped and mints a `PageFrame` capability for
  it, full rights, into the caller's capability table. Nothing is mapped. The caller now *holds a page* and
  can map it, or delegate it, or delegate it and never map it.
- `PageFrame::MAP(va, writable, untyped_slot)` maps the page at `va`. A read/write mapping needs `WRITE`
  on the page frame; a read-only one needs `READ`. The page tables to reach `va` are drawn from the
  untyped named by `untyped_slot`, so like everything a process spends, mapping a page frame comes out of
  its own budget and the kernel allocates nothing.

Contrast `Untyped::MAP`, which does both at once (retype a page and map it writable). That is the
convenient path for a process's private memory. `RETYPE` + `MAP` is the path when the page is going
to be shared, because between the two steps is where the delegation happens.

## Sharing is delegation applied to memory

Because a `PageFrame` is an ordinary capability, it travels over an endpoint with `SEND_CAP`, and the
rights narrow on the way exactly as they do for any delegation (see [delegation.md](delegation.md)).
So the whole sharing protocol is:

1. Producer `RETYPE`s a page frame, `MAP`s it read/write, writes into it.
2. Producer `SEND_CAP`s the page frame to the consumer, narrowed to `READ` (dropping `WRITE` and `GRANT`).
3. Consumer `RECEIVE_CAP`s it, `MAP`s the *same physical page* read-only, and reads what the producer
   wrote.

The kernel copied nothing and was never told these two processes would share memory. They built the
sharing themselves out of a capability, and the read-only narrowing means the consumer can look and
not touch. A peer handed `READ` alone gets `NotPermitted` if it asks to map the page writable, which
the test checks by trying.

## The lifetime question, and why there is no double-free

A page shared into two address spaces cannot be owned by either, or the first one to die frees memory
the other is still using. nife sidesteps this cleanly because of how teardown already works: an
`AddressSpace` frees only the page frames it recorded at spawn (`self.frames`) plus its page tables and
root. A page mapped at *runtime*, by `Untyped::MAP` or `PageFrame::MAP`, is never in that list, so
teardown does not free it. A page frame's page (and the page tables that map it) belong to the untyped
region they came from, and are reclaimed only when that region is destroyed, wholesale, the way
untyped memory always is. So when the producer exits, its mapping of the shared page goes away
with its address space; the physical page persists, and the consumer's mapping is still good. No
refcount, no double-free, because address spaces borrow page frames and never own them.

The honest limit: individual page frames are not reclaimed on their own, only with their whole untyped
region. That is the same bounded, deliberate gap untyped memory already has, and closing it is the
same parked problem: capability revocation.

## The synchronization edge is the IPC rendezvous

On ARM's weak memory model, the producer's write is not automatically visible to the consumer just
because it happened first in time. What makes it visible is that the delegation is a *rendezvous*:
the producer's `SEND_CAP` releases the scheduler lock and the consumer's `RECEIVE_CAP` acquires it, and
that release/acquire pair is the happens-before edge. The write lands before the send, the send
synchronizes with the receive, the read comes after. So the same IPC that carries the capability also
orders the memory, which is a tidy demonstration of why "control travels by IPC" and "data travels by
shared memory" fit together rather than being two unrelated rules.

## What the test proves

`a_page_frame_capability_shares_a_page_and_a_read_only_view_cannot_write_it` runs the protocol above
with two user processes and checks two things: the consumer reads the producer's sentinel through its
own mapping (the page is genuinely shared), and a writable mapping of the read-only view is refused
(the rights confine it). The sharing half is self-verifying in a nice way: `RETYPE` hands back a
*zeroed* page, so if the consumer had somehow mapped a different page instead of the shared one, it
would read zero, not the sentinel. Reading the sentinel can only mean it mapped the producer's page.
And verified it can fail: stub the `WRITE` check in `PageFrame::MAP` and the read-only view becomes
writable, so the confinement assertion trips.

## Three ways a page gets into an address space, and one of them is invisible

This is the finding milestone 108 turned up, and it is the reason that milestone existed. There
were, and are, three routes:

| Route | Who calls it | In the mapping database? |
|---|---|---|
| `PageFrame::MAP` | a process, for a page frame it holds | **yes** |
| `AddressSpace::MAP_INTO` | a userspace loader, into a space it is building | **yes** (`user_address_space_map`) |
| `Spawn::maps` | the kernel, before the process's first instruction | **no** |

The first two go through `revoke::record_mapping`, and an unrecordable mapping is refused rather
than made, at the mapper's own expense. The third is `AddressSpace::map_physical`, which maps and
returns; there is nothing to record it against, because the process does not exist yet.

So a page delivered by `Spawn::maps` cannot be revoked. `PageFrame::REVOKE` deletes every
capability naming the page (there is none) and unmaps it from every space that recorded it (this one
did not), and the holder's mapping survives untouched. That is not a bug in `revoke`; it is the
honest consequence of a mapping that no capability ever stood behind. A spawn-time mapping is
permanent by construction.

It cannot be narrowed by anyone downstream, because the kernel picked the permissions at spawn
and there is no object to attenuate, and it cannot be handed on, because there is nothing to hand.

## The migration (milestone 108)

The disk and display paths now hold their pages as `PageFrame` capabilities and map them themselves.
Each migrated program gained two things in its capability table: the page frames, and an untyped to draw the
page tables from, because `PageFrame::MAP` retypes intermediate tables out of a region the caller names
and the kernel allocates nothing.

Migrated: `disk_surveyor` (the block-shared page and the roster), the roster probe, `disk_partitioner`,
`mkfs`, the virtio-gpu driver (its whole DMA region), `painter` (the surface), and `display_terminal`
in its whole-screen mode (the surface and the application's output page).

The stack is the floor. It is still a `Spawn::maps` entry and has to be: a program cannot map its
own stack, because it needs a stack before it can make the syscall that would map one. Everything
else a process touches can be a capability; that one page cannot.

### EXAMPLES

The wiring side, from `kernel/src/user/disk_service.rs`. The roster goes in with `READ` and nothing
else, and the program is handed a budget to reach it with:

```rust
crate::sched::grant_at(SURVEY_SLOT_BUDGET, untyped_cap(budget))?;
crate::sched::grant_at(SURVEY_SLOT_ROSTER, page_frame_cap(roster_phys, Rights::READ))?;
run(surveyor_image, Spawn { arg0: ROLE_SURVEY, grants: &[], maps: &stack, .. })
```

The program side, from `components/src/disk_surveyor.rs`. It picks its own address, because it owns the
page now and the kernel has no opinion:

```rust
if !user_mode_runtime::map_page_frame(ROSTER_PAGE_FRAME, ROSTER_VA, /* writable */ false, BUDGET) {
    user_mode_runtime::exit()
}
```

And the negative control, which gained a rung it could not have had before. Under the old wiring the
only boundary was the page permission, so the only thing the probe could do wrong was write. Now it
cannot even obtain a writable window:

```rust
// Rung one: refused by the rights on the capability, before a page-table entry is written.
let rw_refused = !user_mode_runtime::map_page_frame(ROSTER_PAGE_FRAME, ROSTER_VA, true, BUDGET);
// Rung two: the read-only mapping we are entitled to, and a write through it. This faults.
user_mode_runtime::map_page_frame(ROSTER_PAGE_FRAME, ROSTER_VA, false, BUDGET);
unsafe { core::ptr::write_volatile(ROSTER_VA as *mut u64, 0) };
```

### What the migration proves that the object alone did not

`disk_tests::the_roster_can_be_revoked_out_from_under_its_holder`. A program maps the roster page
frame, reads its first word and reports it, and parks. The kernel checks that word against its own
read of the same physical page through the direct map (so the mapping was real, and was of *that*
page), revokes the page frame, and lets the program go. The second read faults, at the address it
faults at.

Verified it can fail, which is the point of writing it: put the roster back as a `Spawn::maps`
entry and the test trips its own assertion, "a program read a page frame that had been revoked out
from under it, at 0x50010000, and was NOT stopped: the mapping outlived the capability". That was the
state of the world for every driver in the tree the day before.

## The page frame budget: what a test boot spends, and what it gets back

This section is the receipt for the failure that has misled three milestones, and it is written as a
budget rather than as a story because the number is the point.

### The symptom, and why it always accused the wrong test

The aarch64 test boot failed as `Unmappable(OutOfPageFrames)` about one run in three (measured
2026-08-16). It failed in whatever test happened to spawn last, which was never the test that spent
the memory. It failed disguised: milestone 107 (socket) met it as `time_tests` reporting *"no swish
program in the initrd archive, or no memory to wire one"*, which reads like a packaging bug and is
not one. `notes/net/memory-and-reclamation.md` had recorded it eight separate times as "`virtio::MAX_DEVICES` has asked for
reclamation again".

Two numbers, not one, and the second is the one that fails a boot. Free page frames and the longest
*run* of free page frames are different questions, and `alloc_contiguous` asks the second. Milestone 107
measured 137 page frames free and no run of 128 at the failing allocation and read it as exhaustion.
Both readings now exist: `memory::free_page_frames()` and `memory::largest_free_run()`
(`PageFrameAllocator::largest_free_run`, host-tested in `crates/page_frames/tests/allocator.rs`).

### The instrument: the page frame ledger

`kernel/src/testing.rs` reads free page frames once per test, at the top, so the readings *partition*
the run: what a test is charged is the drop between its own reading and the next one. That shape is
not fussiness. Reading before and after the test body instead charges only what the test spent while
it was running, and a test that spawns a service and returns the moment it has its report leaves that
service still mapping its heap: on the first measured boot, before-and-after attribution left
17362 of 29091 page frames landing nowhere at all.

What a run prints:

```text
test kernel::user::dir_capability_tests::a_full_directory_capability_... ... ok
    [that test kept 2284 frames]
...
frames: 29280 free before the first test, 15249 after the last (14031 never returned); longest free run 14080
```

A charge under `PAGE_FRAME_REPORT_MIN` (16 page frames) is silent, so the transcript names the
services and not the arithmetic.

Two ceilings fail the run, and the second is the one that names the bug. `SUITE_PAGE_FRAME_BUDGET`
catches the total residue growing past what is accounted for below, which is how a
new service-shaped test that forgets to hand its memory back is caught in the act. `SUITE_MIN_FREE_RUN`
requires the boot to end with a free run of at least 1024 page frames, and that is the real gate: loading a
program calls `alloc_contiguous`, so what a boot runs out of is contiguity rather than memory. A suite
can pass a residue ceiling and still be fragmented into uselessness, which is exactly the state this
one was in.

### What the boot spent, measured

| | free before the first test | free after the last | never returned | longest free run |
|---|---|---|---|---|
| before | 29307 | **216** | 29091 | **117** |
| after reclaiming init | 29306 | 12504 | 16802 | 12394 |
| after reclaiming init and the net services | 29306 | **15307** | 13999 | **14080** |
| the same, remeasured on the merged tree | 29280 | **15249** | 14031 | **14080** |

216 free page frames and no run longer than 117 is the whole failure. There was no headroom left for
anything, so which test died was decided by scheduling.

The fourth row is the same boot after merging `main` on 2026-08-16, and the 32 extra page frames it keeps
are milestone 54's second act: the SMB test now runs a seeding client through the FS service before
it wires the adapter, and that client is one more process. The number that decides whether a boot
lives is unchanged at 14080, because a client that runs and exits fragments nothing. Read the pair as
the honest shape of this ledger: the residue moves a little every time a test grows a process, and
the free run is what the gate is about. riscv64 came out at 13787 kept and 13733 longest, from
29692 free.

The largest single causes, before:

| page frames | who | what it was |
|---|---|---|
| 12289 | the six `spawn_progenitor` tests | 2048 page frames of building budget each, reserved and never returned |
| 2759 | the ten net tests | a `net_stack`'s 128-page-frame budget, its stack, and its client's, ten times |
| 2284 | the FS service | one block server and one FS server with an 8 MiB heap budget |
| 2146 | the two `authority_tests` | `root_supervisor`'s 1024-page-frame budget, twice |
| 1656 | the credential store | a 6 MiB budget sized by Argon2id's scratch |
| 1362 | the crash-recovery FS test | two more FS servers, deliberately killed and never reclaimed |

### The fix, which is wiring and not mechanism

Everything used here is DECISIONS §16 object revocation, already built and already proved. What was
missing was a handle and an ordering.

- `user::holding::Holding` (name provisional) remembers what the kernel handed a service: its
  threads, the regions to reclaim while those threads still exist, and the regions that may only be
  reclaimed once they are provably gone. `release` spends `sched::kill_thread` and
  `sched::reclaim_region` on them in that order, retrying because the first `reclaim_region` is what
  *arms* §16's kill on a resident. `release_or_fail` is the form tests use, because an instrument that
  reports success either way is not one.

- The region's endpoints are now swept before the refusal, not after (`sched::reap_region_objects`).
  This is the load-bearing half. A blocked thread never reaches `schedule()`, so it never spends the
  armed kill, so a region holding a server parked in `RECEIVE` was refused forever:
  `userspace_init_brings_up_the_console_server` builds exactly such a server out of the progenitor's budget, and
  its 2048 page frames were unreclaimable by construction. Sweeping first fixes it because the wake was
  already there: removing an endpoint drains its wait queues, aborts each waiter's IPC and wakes it,
  which is precisely the transition the doomed resident needs. A refused reclaim was already
  destructive (it arms kills; see `reclaim_region`'s BUGS), so this is the same commitment one object
  over.

- `spawn_hello` carves `hello`'s building budget outside the spawned thread and hands the caller a
  holding over it. The region is unchanged; who can name it is not, and that is the whole difference
  between 8 MiB spent and 8 MiB lent.

- A service's endpoints come out of a region of its own (`net_stack`'s do), because that is the
  handle the sweep above needs. From the kernel's shared endpoint chunks there is no such handle and
  no way to end the process short of rebooting.

- Stack pages are retyped from a region instead of allocated, and that region is reclaimed only
  after every thread is gone. A `Spawn::maps` page is not a recorded mapping (see "Three ways a page
  gets into an address space" above), so §13's revocation cannot pull it, and freeing a running
  thread's stack is a use-after-free rather than a fault.

### What is still held at the end of a boot, and why

14031 page frames on the merged tree (13999 before that merge; the table above says where the 32
went), and the difference is accounted rather than shrugged at:

- The FS service, ~2284. Wired once (`fs_service::ensure`) and used by every later filesystem
  test. It is a boot service, not a leak.
- The credential store, 1656. Same shape: wired once behind a `DONE` flag and shared.
- `root_supervisor`'s two trees, 2146, and this one is a real limit rather than a choice.
  `root_supervisor` `SPLIT`s the spawner's budget out of its own, and `reclaim_region` refuses a
  region with live children (freeing its whole run would double-free the child's pages). The child is
  destroyed only by *its* owner, which is a process that has been torn down, so the parent can never
  become childless. Reclaiming a split parent whose children's owners are gone is what a capability
  derivation tree buys and this kernel deliberately does not have (notes/object-revocation.md). It
  wants its own lane.
- The crash-recovery FS servers, 1362, and the disk, sink, `c_seam` and shell services below them.
  All are the same shape as the two above: reclaimable in principle by giving their spawn helper a
  holding, and each is a small separate change rather than part of this one.
- One DMA page and one virtio shadow page frame per net service, ~20 page frames. Deliberately
  *not* reclaimed. The NIC keeps whatever receive buffers the dead driver posted, and returning those
  pages to the allocator would let a live device write into memory handed to somebody else. Ending
  that safely means resetting the device at teardown, which is a change to the transport seam and its
  own piece of work. Twenty page frames is not worth the hazard.
- A page per kernel endpoint, carved into chunks by `sched::create_endpoint` and never freed by
  design.
- The login service, ~640 (2026-08-22, milestone 49 (users, login,)). Same shape as the credential store above:
  wired once behind a `DONE` flag (`system_tests/src/user/login_tests.rs`) and shared by every login test.
  `crate::untyped::create` reserves the whole 640-page-frame construction budget the instant the
  service is spawned; splitting pieces of it into a caretaker or a client budget afterwards costs the
  ledger nothing further; only the initial reservation does. See notes/login.md and
  `components/src/login.rs`'s own BUGS on why nothing gives it back: the service serves logins for the life
  of the boot and this slice builds no teardown path.
- A second credential service instance, ~1659 (2026-08-23, milestone 155 (provisioning tool)). The provisioning
  suite (`system_tests/src/user/identity_provisioning_tests.rs`) needs a store *before* anyone has sealed
  it, which the tree's one shared fixture (`credential_tests::provisioned()`) cannot offer: that
  instance is sealed by the time it returns. So this suite wires its own, same shape as the shared
  one and as permanent for the same reason (`credential_service::start`'s own 1552-page-frame
  reservation: `CRED_BUDGET_PAGES` 1536 plus `CRED_STACK_PAGES` 16), plus the small cost of the two
  `identity_provisioner` invocations this suite runs against it and one `fs_subtree_caretaker` its
  headline test builds to prove the created subtree is real. This suite's own tests report their
  charge directly (`[that test kept N frames]`), which is where the 1659 comes from rather than a
  re-derivation here.
- `MappedWindow`'s formatted panic, ~5 (2026-08-25, milestone 139 (drive unsafe) round 4). Found by the
  `toolchain/nightly-bump` PR going red on a plain toolchain bump with no code change of its own;
  bisected against CI's own historical `build + test (host + QEMU)` logs (five independent runs at
  18621 before `202831a3`/`c94f5d21`, two independent runs at 18626 immediately after, both
  populations internally exact). `MappedWindow::check` (`crates/user_mode_runtime/src/mapped_window.rs`,
  milestone 139) panics with a *formatted* message, and that alone is enough to pull `core::fmt`'s
  panic-with-arguments machinery into any binary that calls it, even once. Measured directly
  (`llvm-size`, dev profile): `painter`, `window` and `display` each grew their linked text by
  roughly 54 KiB (11–15 KiB to 65–70 KiB) adopting it; `display_terminal`, which already pulled the
  formatter in elsewhere, grew by only ~5 KiB. `display` is the standing candidate for where this
  becomes permanent rather than transient: `system_tests/src/user/display_tests.rs` says outright that
  its driver instance "is a long-lived server and never exits" for the rest of that test's boot,
  and a process's `AddressSpace` is sized from its own ELF segment page count
  (`kernel/src/user.rs::load`), so a permanently bigger binary should cost permanently more frames
  by construction. This is the fact the account does not have: the per-program page math for
  `display` alone (roughly +14 pages between the same two commits) does not cleanly reduce to the
  suite's measured +5, and that specific test's own reported charge moved by only +1 across the
  same comparison. `report_frame_ledger`'s attribution is by whichever test happened to be running
  when a shared or lazily-built resource was first touched, not by the code that spent it, and
  `#[test_case]` registration order can shift when unrelated code size changes elsewhere in the
  binary, so a clean per-test diff was not trustworthy evidence here; only the repeated, internally
  consistent *suite-wide* total is. Recorded as +5, attributed to this migration with confidence;
  not attributed to a specific one of the four migrated programs. See `kernel/src/testing.rs`'s
  `SUITE_PAGE_FRAME_BUDGET` doc comment for the rest of the account, including the separate ~1-2 frame
  cross-environment variance this raise had to make room for (18621 → 18626 → 18627 local →
  18628 the one CI run that failed), which this investigation ruled a `swish.rs` change
  and a QEMU version mismatch out of and could not otherwise pin down.
- `printenv`'s four spawns, ~85 (2026-08-26, milestone 47 (navigation naming), DECISIONS §111 (inert configuration)). `date_tests.rs`'s
  own shape one program over: `system_tests/src/user/printenv_tests.rs` spawns a real `printenv` ELF four
  times (`spawn_printenv`, `date_tests::spawn_date`'s own helper), and neither the child processes
  nor, in three of the four cases, a by-hand-allocated config-page frame (`assembled_page`/
  `blank_page`) are ever reclaimed, exactly as `date_tests.rs`'s own five spawns and one `blank`
  clock page are not. Measured directly, twice, zero variance: 18717 both times, +85 over the
  previous budget. None of the four tests prints its own `[that test kept N frames]` line, for the
  same reason none of `date_tests.rs`'s five spawns do either: a handful of pages per spawn does
  not cross whatever threshold makes a line worth printing, but the frames still count toward the
  suite total. `printenv`'s own linked text is 15,381 bytes (`llvm-size`, dev profile), roughly four
  pages, which times four spawns plus a stack page and construction overhead each is a plausible
  order-of-magnitude match to +85; this is not a page-by-page reconciliation, for `report_frame_ledger`'s
  own attribution limitation the entry above already names. See `kernel/src/testing.rs`'s
  `SUITE_PAGE_FRAME_BUDGET` doc comment for the rest of the account (18632 + 85 + 15 headroom =
  18732).

### BUGS in the ledger itself

- A charge is attributed to the test that was running, not to the code that spent it. A boot
  service wired lazily by whichever test asks first is charged entirely to that test, which is why
  `a_full_directory_capability_does_everything_inside_and_nothing_outside` appears as the biggest
  spender in the tree while spending almost nothing of its own. Read a large charge as "the service
  this test was first to need", not as an accusation.
- The two ceilings are set from one measurement each, so they are as good as that boot was
  representative. A run where the host resolver does not answer the non-gating DNS check spends a
  little less; that variance is inside the headroom both numbers carry, but neither is a tight bound
  and neither should be read as one.
- **The ledger cannot see memory that never reaches the page frame allocator.** A region reserved and
  unspent costs exactly as much as one filled to its watermark, which is correct for this failure
  (an untyped is a reservation) and means the ledger says nothing about waste *inside* a budget.

## BUGS

- **A capability naming a run outlives nothing it should not, but the sweep that guarantees that is
  not the one `PageFrame::REVOKE` uses.** Found by milestone 142's adversarial review, 2026-08-27,
  and fixed: `MemoryRegion::DESTROY` used to delete capabilities by *exact object*, one mapped page
  at a time, so `PageFrame(base, 311)` matched nothing and survived the reclamation of every page it
  named. A holder could then re-map pages the allocator had already handed out as a page table or
  another process's stack, which is §13's use-after-free with the widening's name on it. Reclamation
  now sweeps by overlap over the whole range (`sched::delete_page_frame_caps_overlapping`),
  which closes an older hole the same way: a capability to a page nobody had *mapped* was never
  a candidate before, because the sweep was driven by the mapping log and the log had no record of
  it. `PageFrame::REVOKE` deliberately keeps the exact-object sweep; see the next entry.

- **A `REVOKE` on one run used to reach mappings it did not authorize.** `revoke::unmap_everywhere`
  is space-blind: it pulls a physical page out of every address space that maps it, regardless of
  which capability produced that mapping. That was equivalent to "capability-scoped" while a
  `PageFrame` named exactly one page and could not overlap another; §102 made overlap possible and
  the tree's display wiring uses it (the gpu driver holds `PageFrame(dma, 312)` while the painter
  and terminal hold `PageFrame(dma + 4096, 311)` over the same memory, deliberately, because the
  driver's capability must span the DMA window it registered with the IOMMU). So revoking the
  terminal's run unmapped those pages out of the driver's space too, under a capability nobody had
  revoked.

  **Fixed 2026-08-27** (design/decisions/0132-overlapping-page-frame-runs.md, option C, decided
  rather than deferred to the first `GRANT`). Each mapping record now carries the base address of
  the object it was made under, which names the capability *and every capability derived from it*
  because `derive` never changes the object, and `PageFrame::REVOKE`'s unmap half matches on it
  (`revoke::unmap_under_object`). `revoke::LogEntry` grew from 16 bytes to 24 and `LOG_ENTRIES`
  fell from 255 to 170, so a space pays about 1.5x the log pages it did; the existing
  `assert!(size_of::<LogPage>() == FRAME_SIZE)` is what holds that arithmetic to account, and it
  was checked by breaking it. Reclamation and the device take-back stay object-blind on purpose,
  because neither is asking a capability's question.

- **An overlapping capability still survives a `REVOKE`, and that is the decision rather than the
  gap.** Revoking `PageFrame(dma + 4096, 311)` leaves the driver's `PageFrame(dma, 312)` in place,
  naming pages the revoke has unmapped out of the revoked capability's holders. §102 contemplates
  two capabilities coexisting over sub-ranges of one region, and the alternative (sweep capabilities
  by overlap, as reclamation does) would let a one-page holder delete a 312-page capability by
  naming any page inside it. §132 refused it for that reason.

- **A run and a longer run sharing a base are one object to the log.** `PageFrame(p, 401)` and
  `PageFrame(p, 74)` are different objects to `Cap` and identical to `revoke::LogEntry`, so revoking
  the shorter unmaps the longer's mappings of the pages they share. Carrying the length would take
  `LOG_ENTRIES` from 170 to 127 to separate a pair nothing in the tree mints. Bounded in the safe
  direction (the over-broad unmap cannot reach outside the revoked run) and named in `LogEntry`'s
  own `BUGS`.

- **Revocation still says nothing to a device**, and §132 deliberately did not answer this half. A
  driver's IOMMU/DMA window is registered as a byte range (`virtio::register(.., dma,
  DMA_PAGE_FRAMES * FRAME_SIZE, ..)`) and is not derived from any capability, so revoking a
  `PageFrame` narrows what the *CPU* may reach and leaves device-initiated DMA into the same
  physical pages exactly as it was. Even a capability-perfect revocation of a surface leaves the
  device able to write those pages until the driver's virtio registration is itself torn down.
  Whether `REVOKE` should narrow a DMA window couples two objects that are independent today; it is
  the last open section of the decision above and remains an architect's call.

- **A failed multi-page `MAP` gives the page tables back to nobody.** Both mapping paths
  (`page_frame::MAP` and `address_space::MAP_INTO`) are all-or-nothing across a run since
  2026-08-27: a failure partway through unmaps the prefix it had already mapped and tombstones its
  revocation records, so a caller that gets an error never has to wonder how much of its run landed.
  The budget is not rolled back with it. Whatever L3s (and parents) the prefix needed were
  retyped out of the caller's region, and a region is spend-only, so those pages stay spent. The
  mapping is undone; the memory is not returned. A caller that retries a large run after an
  `OutOfMemory` should expect to have less budget than it started with, not the same.

- **A run capability names more than its object, by up to one page.** `SURFACE_PAGE_FRAMES` is
  `SURFACE_BYTES.div_ceil(4096)`, and at 924x344 the surface is 1,271,424 bytes in 311 frames, so
  the capability names 2,432 bytes past the end of the surface. Those bytes are inside the same
  contiguous allocation (the driver's DMA region), not another object's memory, and nothing
  addresses them: every consumer bounds-checks against `SURFACE_BYTES` rather than the frame count
  (`user_mode_runtime::mapped_window::MappedWindow`). It is recorded because a capability that names more
  than the thing it stands for is exactly the kind of fact this convention exists to write down,
  and because the `SURFACE_BYTES % 4096 == 0` assertion that used to hide the question by
  construction was deleted this milestone (`crates/graphics_protocol/src/lib.rs` has the search that
  justified deleting it).

- **RESOLVED 2026-08-26 (DECISIONS §102, built by milestone 142's terminal-size lane).** `Object::
  PageFrame` now carries a page count (`PageFrame(phys, count)`), `page_frame::MAP` and
  `page_frame::REVOKE` operate on the whole run, and `cap::page_frame_run_cap` mints one capability
  for a multi-page region instead of one per page. The display driver's DMA region, the painting
  client's and display terminal's surface grant, and the compositor's screen (plus a capture
  client's read-only mirror of it) all moved to this, which is what makes the grown scanout (900
  page frames at the 1280x720 this was first built and measured against; 311 frames at the 924x344
  the scanout was retargeted to on 2026-08-27, `crates/graphics_protocol/src/lib.rs`'s `WIDTH` has the
  reasoning) fit a sixteen-slot capability table at all: at one capability per page neither size
  would have. `display_service::DRIVER_SLOT_DMA`'s `const` assertion (below, and the error it
  used to produce) is retired along with the pressure it guarded against, and replaced rather
  than deleted: the reformulated assertion beside those slot constants makes the same
  claim in run terms, that no grant list on that path reaches the fault slot. Deleting it outright
  was the state milestone 142's review found, and the hole it left was a future lane pushing a
  service's highest slot onto `FAULT_EP_SLOT` and booting instead of failing the build. The paragraphs below are
  kept as the record of how the fork was found, priced and decided; they describe a state this tree
  no longer has.

- **A `PageFrame` names one page, and a DMA region is a run of them.** The virtio-gpu driver's region is
  nine contiguous pages, so it holds nine capabilities and issues nine `MAP` calls for memory
  that is adjacent in physics, adjacent in its address space, and covered as a single range by the
  IOMMU domain the kernel programmed for it. That is slots 5 through 13 of a sixteen-slot capability table
  (`cap::CAPABILITY_TABLE_SLOTS`), one of which is reserved for the fault endpoint: it fits with one slot
  spare, and a wider scanout would not fit at all. `display_service::DRIVER_SLOT_DMA` carries a
  `const` assertion so that someone who widens the surface fails the build rather than the boot.

  The milestone's scope note called this out in advance ("if the migration finds the object short of
  something a real driver needs, that is a finding worth recording, and it is a design fork rather
  than a quiet addition"), so it is recorded and not fixed. The fork is whether a `PageFrame` should
  be able to name a run of pages (seL4 has no answer to copy here: it retypes N frames and you hold N
  capabilities, and its capability tables are radix trees rather than sixteen slots, so the pressure lands
  somewhere else). Growing `CAPABILITY_TABLE_SLOTS` is a one-number change paid in TCB size, and is the other
  half of the same question.

  **The fork stopped being hypothetical on 2026-08-19, and here is what it costs measured rather
  than asserted.** Milestone 29's terminal-sized scanout (the terminal-font decision chose gohufont-14, which is 8x14, so
  the 128x64 surface gives a 16x4 grid that is not a terminal) needs 800x600 to reach 100x42
  characters. The lane that tried it got the assertion above, at 640x480, before it got anything
  else:

  ```
  error[E0080]: evaluation panicked: the display driver's DMA region no longer fits its capability table
  beside the fault slot: a PageFrame names one page and this region is a run of them
    --> kernel/src/user/display_service.rs:50:15
  ```

  **The ceiling is nine page frames, and it is the capability table rather than the memory.** The
  driver's DMA region starts at slot 5 and must end below the fault slot at 15, so
  `SURFACE_PAGE_FRAMES <= 9`: at most 36,864 bytes, or 9,216 pixels. Every non-square shape that fits
  (128x72, 144x64, 192x48) gives five text rows or fewer at 8x14. 800x600 needs 469 page frames,
  which no sixteen-slot capability table can hold under any arrangement of the other slots, and the
  same is true of every size a person would call a terminal. The other budgets on the path are all
  comfortable by comparison, which is the part that surprises: 469 page frames is under one percent of
  the free pool, the mapping records are two pages against `AS_OVERHEAD`'s sixteen of slack, and
  `MAP_BUDGET_PAGES`'s eight pages still cover the page tables (see below). Nothing else on this
  path is short. Only the slots are.

  **Two of those numbers have moved since, and the constants a reader would quote should be the
  current ones** (updated 2026-09-02, milestones 230 and 231). The table is twenty-four slots
  rather than sixteen, `Option<Cap<Object>>` is 32 bytes rather than 24, and `MAX_THREADS` is
  **256** rather than 128. The paragraphs above are left as written because they are the record of
  an investigation on the day it was done, and none of the arithmetic's *conclusions* survive being
  updated any differently: 469 page frames does not fit in twenty-four slots either, and option 2
  now prices 512 slots at 16 KiB a thread across twice as many threads.
  `kernel::cap::CAPABILITY_TABLE_SLOTS`' own doc carries why it moved, which is worth reading before
  quoting any figure here.

  **And a boot now says what it used**, which is the thing this section's whole argument
  was conducted without. `kernel::cap::report_peak` prints `capability slots: N of M at peak` from
  the scheduler's idle loop, and `script/swish-check` echoes it on every run: the interactive boot
  reaches 22 of 24. That number cost milestone 230 (`script/shell-check` red) four instrumented boots to learn and now costs
  a boot, which is the difference between a fork priced against a guess and one priced against a
  measurement. See design/roadmap/0231-capability-slot-high-water-mark.md.

  It was 21 until 2026-09-05, when milestone 111 gave the progenitor a reason to hold the entropy service's
  request endpoint for the whole boot rather than release it after the login block, and the gate
  caught the move on the first run rather than at the next silent halt. Two of the three slots of
  headroom are left.

  Three ways out, priced:

  1. **A `PageFrame` that names a run.** `Object::PageFrame(u64)` carries a page count,
     `page_frame::MAP` maps the run, `page_frame::REVOKE` unmaps it. Measured surface: 4 sites
     match on `Object::PageFrame` and 21 construct one through `cap::page_frame_cap`. It is the
     option this tree's own reasoning points at, because the run is already one range in physics, one
     range in the address space and one range in the IOMMU domain, and it collapses 469 capabilities,
     469 syscalls and 469 mapping records into one of each. It is a change to the meaning of a
     syscall method, which is a boundary rather than a habit (`AGENTS.md`, DECISIONS §10 and §16).
  2. **Grow `CAPABILITY_TABLE_SLOTS`.** One number in `kernel/src/cap.rs` and its twin in `crates/abi`, paid
     in TCB size: `Option<Cap<Object>>` is 24 bytes, so 512 slots is 12 KiB of capability table per thread
     against today's 384 bytes, and `MAX_THREADS` is 128. It moves `abi::fault::FAULT_EP_SLOT`,
     which is defined as `CAPABILITY_TABLE_SLOTS - 1` and which every supervised program agrees on. It leaves
     469 `MAP` calls and 469 mapping records in place, so it buys the pixels without buying any of
     the elegance.
  3. **Map the run into the client's space without giving it capabilities**, which
     `address_space::MAP_INTO` already does: the spawner holds the `AddressSpace`, maps each page
     frame into it, and deletes its own cap between iterations, so one slot serves the whole run and
     the client holds none. This needs no model change and no new method, and unlike the
     `Spawn::maps` mechanism it replaced, every
     mapping it makes is recorded and therefore revocable (§13, §67). Its cost is 469 kernel-side
     map operations at spawn and a client that cannot delegate or revoke its own surface, which is
     authority it has no use for. This is the cheapest correct option and the one to reach for if
     the answer to 1 is "not yet".

  **Two sizing facts the same investigation established, because they will be the next questions.**
  The page-table budget survives 800x600: one L3 covers 512 pages and 469 fits, so
  `MAP_BUDGET_PAGES`'s eight pages still hold, but only because `SURFACE_VA` is 0x60_0000 and
  therefore 2 MiB-aligned; the comment justifying that constant says "every mapping here lands
  inside one 2 MiB window" and that sentence is load-bearing. 1024x768 is 768 pages and does not
  fit one L3, which makes 800x600 the last size the current budget justifies. And the userspace VA
  map does not survive it: `display_terminal` puts `OUT_VA` at 0x68_0000, only 128 pages above
  `SURFACE_VA`, so a 469-page surface would run straight through it and through `CTL_VA` at
  0x69_0000. Both have to move above 0x80_0000, which puts them in a second 2 MiB window and costs a
  second L3, still inside the eight-page budget.

  **One more arithmetic trap in the chosen size.** 800 x 600 x 4 is 1,920,000 bytes, which is
  **468.75 pages**, so the surface does not fill whole page frames and `graphics_protocol`'s build-time
  assertion that it must (`SURFACE_BYTES.is_multiple_of(4096)`) fails. With an 800-pixel width the
  height must be a multiple of 32 for the surface to be a whole number of page frames; 608 is the
  nearest, giving 475 page frames exactly and a 100x43 grid. The alternative is to grant `div_ceil`
  page frames and let the last one be three quarters used, which means relaxing an assertion that
  exists so a client is never handed a partial page.

- **Not everything migrated.** The console is deliberately last (a bootstrap that needs a capability
  service to print cannot report its own failure, so it is its own decision with its own argument),
  and the compositor path is not in this milestone at all: `display_terminal` therefore maps its own
  page frames in `MODE_DISPLAY` and still receives spawn-time mappings in `MODE_WINDOW`. `date` keeps
  its `Spawn::maps` clock page in the kernel's test wiring, and it is the one place in the tree where
  both mechanisms appear in a single spawn literal (a `page_frame_cap` whose only job is to be probed
  for presence, beside a `Mapping` that does the actual work). Migrating it means touching the shell's
  spawn path and §67's grant manifest, which is a bigger change than the wart. Note that the *shell*
  spawns `date` through `AddressSpace::MAP_INTO`, which is recorded, so the revocability gap there is in
  the test wiring rather than in the real path.

- **Each migrated program costs one more untyped region.** `untyped::create` takes a contiguous run
  from the page frame allocator and holds it for the process's life, and the region table has a
  finite number of slots. Eight pages apiece is negligible next to what these programs already
  reserve (`mkfs` takes 384), and a small contiguous request is easy where milestone 107's 128-page
  one was not, but it is a reservation added to a machine whose page frame pool that milestone found
  at the edge.

- **The mapping records cost the process, not the kernel.** Every `PageFrame::MAP` writes a record
  into a log page retyped from the *address space's* own backing region (not the untyped named in the
  call, which pays only for page tables). That is 255 records per page and `AS_OVERHEAD` is sixteen
  pages of slack, so nothing here comes close; a program that maps thousands of page frames would
  notice.

## A note about stack frames: a different concept, deliberately not renamed here

*This section predates the `Frame` -> `PageFrame` rename and every "frame" in it refers to a CPU call
frame (the compiler's stack-size accounting for one function activation), never to the kernel object
this note otherwise describes. It is left exactly as it read before the rename, because rewriting
"frame" to "page frame" here would manufacture the opposite of this rename's own purpose: a false
claim that a stack frame is a physical page.*

- **The suite overflowed a kernel thread stack intermittently on this branch, and the cause was not
  this milestone.** Fixed on `main` before this merged; kept here because the next person to meet a
  fault on this path should meet the whole story, and because "the milestone that surfaced it was not
  the milestone that caused it" is the part that would otherwise be lost.

  One run in five faulted (2026-08-13; four green runs on this branch, one red). The kernel binary
  was byte-identical between a run that faulted and a run that passed: the two commits differ only
  in `.github/dependabot.yml`, `.github/workflows/toolchain-bump.yml` and `script/ci-qemu`, with
  nothing under `kernel/`, `crates/`, `user/` or `redoxfs_server/`. So it is depth-dependent rather than
  deterministic, and re-running until green would hide it.

  ```
  ESR_EL1  0x96000047   EC 0x25, data abort taken without a change in EL (so: kernel mode)
                        WnR 1, a write
                        DFSC 0x07, translation fault at level 3
  FAR_EL1  0xffff0010001b3000
  ELR_EL1  0xffff00004012fa34
  x8       0xffff0010001b7a90
  ```

  `FAR` is exactly the guard page of kernel thread stack slot 87. `thread::STACK_AREA` is
  `KERNEL_VA_BASE | 0x10_0000_0000` and the per-thread stride is five pages (`STACK_PAGES` = 4 plus
  one guard), so `FAR - STACK_AREA` is `0x1b3000` = 87 × `0x5000` with a remainder of zero. `x8`
  is `0x4a90` into the same slot, which is that thread's own stack, 1392 bytes below its top. So the
  guard page did its job: a 16 KiB kernel stack ran out and the write below it was caught rather than
  quietly landing on the neighbor.

  It faulted while the supervision and reap tests were running (the console interleaves, so which
  test owns it is not established, and this note should not pretend otherwise).

  **That last sentence about the stack running out is wrong, and the register above it says so**
  (2026-08-16). A stack 1392 bytes below its top has used 8% of itself, not all of it, and the two
  claims sit in the same paragraph. This exact `FAR` came back twice more, byte for byte, after
  #157 shrank `reap_region_objects` and after milestone 124 rebuilt the spawn path, which no
  depth-driven overflow could do; and `script/stack-depth-check` now says the deepest chain a thread
  stack can carry is 13792 bytes against the 20480 this address would need. See
  notes/stack/guard-page-faults-2026-08-16.md. The rest of this entry stands: the fault is real,
  it is not this milestone's, and the binary was byte-identical between a red run and a green
  one.

  **And the answer arrived on 2026-08-17, which vindicates the register and not the reasoning
  beside it.** The recurring address proved nothing: a fault that reaches the exception vector's own
  frame store walks `sp` down and stores upward in aligned steps, so its terminal store lands on the
  guard base every time regardless of what `sp` was doing. What `x8` was telling you is exactly
  right, though, and it is the whole diagnosis in one register: the thread was shallow on
  slot 87's stack, because the stack had been unmapped under it. A supervised corpse is published
  `Dead` while still executing on its own kernel stack, and an out-of-band region reap frees that
  stack before the corpse reaches `switch_to`. See
  notes/stack/kernel-stack-freed-under-its-owner.md, and the block of milestone 124 (a thread is born
  where it lives).

  **A correction worth keeping, because the wrong reading was reasonable and cost an hour.** The
  first pass at this decoded `FAR` through `phys_to_virt` (which is `pa | KERNEL_VA_BASE`), read the
  result as physical `0x1b3000` with a stray bit 36, and concluded the pointer was corrupted. Bit 36
  is not corruption: it is `STACK_AREA`, placed 64 GiB up precisely so that a stack address can
  never collide with the virtual name of a physical one, which `thread.rs` says in the comment
  above the constant. The lesson is that a high-half address is not automatically a physmap address,
  and masking off `KERNEL_VA_BASE` is not a decode unless you have first established which region
  you are in.

  **What it turned out to be, and why this milestone was the wrong suspect.** The milestone was held
  rather than merged on four green runs out of five, and the investigation went looking for what made
  *this branch's* kernel path deeper. It was not this branch. Measuring per-function frames with
  `-Z emit-stack-sizes` and comparing this milestone's test binary against `main`'s says the largest
  single frame growth in the whole milestone is 128 bytes; its biggest new frame is one more
  `spawn_on` instantiation, the same size as the eight already there.

  The cause was `sched::reap_region_objects` on `main`, whose frame was 6816 bytes, of which 4096 was
  one `[u64; MAX_ENDPOINTS]` scratch array, against a measured thread-stack headroom of 4712 bytes.
  **That one frame wanted 2104 bytes more than all the headroom there was**, so any chain reaching the
  measured peak and then entering a reap could not fit. This milestone added one more spawned program
  to a margin that was already short, which is why it faulted here first. Fixed on `main`
  (notes/stack-high-water.md), and `script/stack-frame-check` now fails the build on a frame that size.

  The general lesson is worth more than the bug: the milestone a fault appears in is not
  necessarily the milestone that caused it, and on a shared resource as invisible as stack depth,
  the last change to arrive gets blamed for a margin that many changes spent.
