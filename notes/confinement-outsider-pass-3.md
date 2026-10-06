# Confinement, a third outsider pass (milestone 633, provisional name)

An outside review of the 33 confinement claims in `notes/confinement-claims.md`'s table, done
without reading that file's narrative, the audit reports, the fatal-risk notes, the roadmap, the
earlier outsider notes, or any git history. The rule was to assume each claim is false and look for
the case that makes it so, and to count an attack only when it boots.

**Result: one booted escape, at the application boundary, and the kernel's own confinement held.**
The capability core, the rights gate in `syscall::invoke`, the paging gates, the DMA shadow ring,
the port bitmap and the filesystem subtree scope all held under the attacks I mounted. The escape is
a confused deputy in the swap demonstrator: a client holding only `WRITE` on a server's endpoint
makes the server write a byte into a device register page the client was never granted, because the
server's `OPERATION_PUT` handler does an unbounded write at a client-chosen offset. It boots red on
aarch64 and riscv64. It is the server's bug, not a kernel gate failure, but it is a real reach to an
ungranted object and so it is an escape of the end-to-end claim. Alongside it are four near misses,
each either in a demonstrator or in a test that stays green while its property breaks.

## What I added and booted

`system_tests/src/user/confinement_attack_tests.rs`, three tests driven through the real dispatcher
(`syscall::invoke`), green on aarch64, riscv64 and x86_64 with zero frames leaked:

- `a_write_only_rendezvous_holder_cannot_receive_reap_or_survey` drives `RECEIVE`, `RECEIVE_CAP`,
  `REAP` (all `READ`) and `SURVEY` (`ENUMERATE`) against a `WRITE`-only rendezvous capability, with a
  sender already parked so a gate let open returns a message rather than hanging. Claim 26's kernel
  gate, and the kernel half of 6 to 9's rights distinction.
- `a_read_only_holder_cannot_send_and_an_ungranted_slot_names_nothing` drives `SEND` against a
  `READ`-only capability (refused) and an invoke of an ungranted slot (`NoSuchSlot`). Claim 2.
- `a_grant_less_budget_mints_a_grant_bearing_frame` retypes a page out of a `WRITE`-only (GRANT-less)
  budget and asserts the returned frame carries `Rights::ALL`. Claim 3's retype scope gap,
  characterized green so it cannot close unnoticed.

The confused-deputy escape (claim 26) is `live_swap_tests::a_confined_client_drives_the_server_to_write_past_its_log`,
red by design and **opt-in** (skipped unless named, so CI stays green): run it with
`script/test --test a_confined_client_drives`. It drives a new `ROLE_DEPUTY` channel in
`components/src/swapper.rs` and a new `ROLE_CONFUSED` role in `fixtures/src/chatty.rs`, both added
this pass, and `crates/swap_protocol` carries the new constants.

## Independence

I did not open `notes/confinement-claims.md` below its claims table, `design/audit-reports/`,
`design/fatal-risks/`, `design/roadmap/` (including `proposals/`), any `notes/*outsider*` or
`notes/*falsif*` file, or git log, blame or GitHub. Source, code comments, `BUGS` sections,
`design/decisions/` and how-to notes were fair game. Encounters to record, all source rather than
the listed files: `kernel/src/cap.rs:609-619` carries a `BUGS` note from this milestone's second
pass (claim 3), `system_tests/src/user/receive_cap_attack_tests.rs` records earlier passes' fixed
escapes, and subsystem recon reported that some source comments name forbidden files by path (a
`design/roadmap/proposals/` file, `notes/confinement-outsider-pass-2.md`, `notes/page-frame-slice.md`,
`notes/compositor-claim-25.md`); none of those were opened.

## The claims

Outcome is one of escape, near miss, held, untestable. Grade is the strongest evidence behind the
outcome: `booted <isa>` means observed under QEMU on that ISA this pass, `host` means a host-crate
test suite I ran green this pass, `read` means source review only.

| # | Claim | Outcome | Grade | Test / evidence |
|---|---|---|---|---|
| 1 | A derive holds no more than its source | held | host | `capability::derive_never_widens_rights` (ran green) |
| 2 | Userspace cannot forge a right from a register | held | booted aarch64/riscv64/x86_64 | `confinement_attack_tests::a_read_only_holder_cannot_send_and_an_ungranted_slot_names_nothing` |
| 3 | A non-delegating budget cannot split a delegating child | near miss | booted aarch64/riscv64/x86_64 | `confinement_attack_tests::a_grant_less_budget_mints_a_grant_bearing_frame`; `split_never_widens_rights` (host) |
| 4 | A consumed capability cannot be reused | held | host | `capability::a_deleted_capability_stays_deleted` (ran green) |
| 5 | Dropping one capability spares the others | held | host | `capability::delete_touches_only_its_slot` (ran green) |
| 6 | A supervisor cannot reap a corpse it does not supervise | held | host, booted aarch64/riscv64/x86_64 | `capability` reap suite (host); supervision suite ran green on every ISA |
| 7 | A refusal about a stranger's thread discloses nothing | held | host, booted aarch64/riscv64/x86_64 | `capability::a_stranger_reveals_nothing_about_its_liveness`; survey/reap suites ran green |
| 8 | A view shows exactly the rendezvous's own children | held | host, booted aarch64/riscv64/x86_64 | `capability::a_survey_shows_exactly_the_endpoints_own_children`; survey suite ran green |
| 9 | What a supervisor may see and reap are one domain | held | host, booted aarch64/riscv64/x86_64 | `capability::the_view_and_the_reap_have_the_same_scope`; suites ran green |
| 10 | A user VA is low-half and page-aligned, every ISA | held | host | `paging` user-VA gate suite (ran green) |
| 11 | No page is both writable and executable | held | host | `paging` W^X suite (ran green) |
| 12 | An IOMMU entry sets no reserved bit | held | host | `paging::x86_64` vtd suite (ran green) |
| 13 | A device cannot touch memory outside its region | held | host | `direct_memory_access_validator` suite (ran green) |
| 14 | A driver cannot send its device to unvalidated descriptors | held | host | `direct_memory_access_validator::an_accepted_descriptor_is_confined` |
| 15 | A driver cannot make the validator walk out or forever | held | host | `direct_memory_access_validator` walk/batch suite |
| 16 | One queue's validation cannot touch another's rings | held | host | `direct_memory_access_validator::distinct_queues_occupy_disjoint_blocks` |
| 17 | A descriptor changed after validation cannot reach the device | near miss | host, read | device reads the shadow (`virtio.rs:956-959`); the test is vacuous, see below |
| 18 | A wiring plan grants no unasked right | held | host | `component_plan` suite (ran green, 31 tests) |
| 19 | A directory capability reaches its subtree and nothing above | held | host, booted aarch64/riscv64 | `filesystem_protocol` (66) and `subtree_scope` (7) ran green; `dir_capability_tests` ran green on aarch64/riscv64 (x86 has no disk); cited proofs cover rights bits only, see below |
| 20 | A memory-unsafe C component faults and changes nothing outside its grant | held | booted aarch64/riscv64 | `c_seam_tests::...` ran green; x86 skips (no C program for `x86_64-unknown-none`) |
| 21 | A user program cannot read a kernel address, every ISA | held | booted aarch64/riscv64/x86_64 | `tests::a_user_program_cannot_read_a_kernel_address`, `..._el0_cannot_read...`, `riscv_virtio_tests::the_page_tables_say_u_mode_cannot_read...` |
| 22 | An ELF cannot load over the kernel or ask for a W+X page | held | booted aarch64/riscv64/x86_64 | `tests::an_elf_that_asks_to_be_loaded_over_the_kernel_is_refused` and sibling, ran green |
| 23 | The progenitor cannot rebuild after dropping construction authority | held | booted aarch64/riscv64/x86_64 | `authority_tests::init_drops_its_construction_authority_and_cannot_build_again` |
| 24 | Two shells with different roots cannot name each other's files | near miss | booted aarch64/riscv64 (the sequential test); read (the reuse leak) | `shell_navigation_tests::two_shells_...` ran green; a reachable window-reuse leak, see below |
| 25 | A client cannot reach its neighbour's pixels or read the screen | near miss | read | `compositor_tests` suite; harness-policy and a respawn scrub gap, see below |
| 26 | A client of a rendezvous cannot become its server | escape | booted aarch64/riscv64 (x86 skip) | kernel gate held (my gate test, all ISAs); `live_swap_tests::a_confined_client_drives_the_server_to_write_past_its_log` is red, see below |
| 27 | A non-holder cannot touch a port; ports do not leak across a switch (x86_64) | held | booted x86_64 | `x86_port_tests::port_holder_transmits_then_a_non_holder_faults` (ran green) |
| 28 | A revoked port holder faults on its next in/out (x86_64) | held | booted x86_64 | `x86_port_tests::a_revoked_holder_faults_on_its_next_port_write` (ran green); the test does not check the fault pc, see below |
| 29 | A thread that deletes its own port cap faults on its next in/out (x86_64) | held | booted x86_64 | `x86_port_tests::a_holder_that_deletes_its_port_capability_faults_on_its_next_port_write` (ran green) |
| 30 | A revocation reaches a capability in flight | held | booted aarch64/riscv64/x86_64 | `revocation_in_flight_tests::a_capability_revoked_while_it_is_in_flight_does_not_reach_the_receiver` |
| 31 | An unvouched child holds no undelegated capability | held | read | `script/swish-check` installed/unvouched census |
| 32 | The boot shell holds no display device | held | read | `script/swish-check` caps census |
| 33 | No WRITE, no x86_64 port I/O | held | booted x86_64 | `x86_port_tests::a_read_only_port_capability_must_not_grant_port_output` (ran green) |

Counts: 1 escape, 4 near miss, 28 held, 0 untestable.

## The escape

### Claim 26: a confined client drives the server to write into a device page it never held

The kernel gate holds, and my `a_write_only_rendezvous_holder_cannot_receive_reap_or_survey` proves
it on all three ISAs: a `WRITE`-only client is refused `RECEIVE`, `RECEIVE_CAP`, `REAP` and `SURVEY`,
so it cannot become the server. But `swap_protocol::serve` handles `OPERATION_PUT` with
`log_put(log_base + arg, version)` where `arg` is the client's own request word and `log_put` is an
unbounded `write_volatile((LOG_VA as *mut u8).add(seq), version)`
(`crates/swap_protocol/src/lib.rs:802-804`, 1037-1038). So a client that holds only `WRITE` on the
stable endpoint asks the server to write at an offset of the client's choosing. At offset
`DEV_VA - LOG_VA` the server writes the version byte into the first register of the device page it
holds and the client does not, and replies, echoing the offset. The booted test drives this through
the real harness: a new `ROLE_DEPUTY` channel starts one server holding the device and the log, and
a new `ROLE_CONFUSED` chatty client with only the honest client's capabilities makes the call. Red
on aarch64 and riscv64 (the server echoed tag `0x100100000` for offset `0x100000`); x86_64 skips, it
has no device page. This is the server's bug, not the kernel's: the kernel gave the client exactly
`WRITE` on an endpoint, and the fix belongs on `swap_protocol::serve` (a bound on the offset, and a
sender check on the control verbs `OPERATION_QUIESCE`), which this pass does not write. The test is
opt-in because a permanently red test would break CI.

## The near misses

### Claim 3: a spend-only budget still mints delegable frames

`memory_region_cap` hands a child a `WRITE`-only (GRANT-less) untyped, and `SPLIT` honours it:
`Cap::mint_child` copies the parent's rights, so a GRANT-less region cannot split a GRANT-bearing
child, which `split_never_widens_rights` proves and which is claim 3 as stated. But
`MemoryRegion::RETYPE` needs only `WRITE` and returns the new `PageFrame` with `Rights::ALL`, GRANT
included (`kernel/src/syscall.rs:829-845`), so a holder of a spend-only budget can retype a page and
`SEND_CAP` it onward. `a_grant_less_budget_mints_a_grant_bearing_frame` asserts exactly this at boot
on all three ISAs: the budget holds `WRITE` alone, its retype holds `Rights::ALL`. It is a near miss
and not an escape because the frame is carved from the holder's own budget, so it reaches nothing it
was not already given. Already recorded at `kernel/src/cap.rs:609-619`; a claim or ruling is owed on
whether a spend-only budget should also withhold GRANT from its retypes.

### Claim 17: the device reads the shadow, but the test proves only that two arrays are two arrays

The property holds in its narrow form: a virtio queue's descriptor table and avail ring point at a
kernel-private shadow (`kernel/src/virtio.rs:956-959`), and the device never reads the driver's
writable copy, so mutating a descriptor after validation cannot steer it. The regression test does
not prove this. Both the Kani harness
(`crates/direct_memory_access_validator/src/lib.rs:650-686`) and the kernel twin
(`kernel/src/virtio.rs:1288-1328`) write the driver's descriptor frame and read back the shadow
frame, which are separate allocations, so the final assertion cannot fail whatever the code does.
Nothing checks the one line the whole claim rests on, that the device's queue registers point at the
shadow. There is also a real race the crate's own `BUGS` names
(`direct_memory_access_validator/src/lib.rs:27-37`, 290-293): the shadow descriptor is published as
two unbarriered stores into a slot the device may still own, so a device reading mid-publish on an
asynchronous queue could pair a new address with a stale length and read past the region. Triggering
it needs a concurrent device read during the two stores, which under QEMU's synchronous block
completion is not reachable, so it stays read; it wants a lane that barriers the publish or shadows
into an idle slot.

### Claim 24: window 0 is safe, but a reused FS window leaks file replies between concurrent jobs

The headline test runs its two shells one at a time because caretaker clients share a window page in
the kernel harness, and the kernel property it proves (neither shell holds a capability reaching the
other's subtree) is sound. The production wiring is more interesting. Window 0 is shared, but only
among root-authority clients (the shell, login and the progenitor), and the identity provisioner is
not concurrent with them (the progenitor waits for its one report before building the shell), so no
confined client is ever on window 0. The reachable gap is windows 1 to 6, the directory-granted job
pool: `Windows::take` hands them out round robin and never checks whether the previous holder is
still alive (`crates/system_initializer/src/lib.rs:4958-4970`, BUGS at 4931-4947). A new job can get
a window an older, still-running job still maps at `FS_CLIENT_PAGE_VA`, because reuse zeroes and
re-slices the page but does not remove the old mapping. The server writes a READ reply into the
window chosen by the requester's badge (`redoxfs_server/src/dispatch.rs:110-111`,
`redoxfs_server/src/bin/redoxfs_server.rs:451`), so the new job's file bytes land in a page the old
job can read without holding any capability to the new job's subtree. The code's own BUGS says the
fix needs a "job reaped" signal that does not exist yet. Settled by read rather than booted because
forcing the reuse needs the full production job pool, a RedoxFS disk and six jobs cycled to roll the
rotation onto a live holder, which is beyond a targeted boot.

### Claim 25: the confinement is a test-harness spawner, and a respawn would show stale pixels

A compositor client holds capabilities for its own control page and surface frames and nothing else;
the screen capability goes only to the capture role (`kernel/src/user/compositor_service.rs`), and
`compositor_tests` runs green, so the claim holds against the probes mounted. Three caveats keep it a
near miss. The spawner is called only from `system_tests`; there is no boot path, so this is a
property of a kernel-side test harness. The tests check a narrow set of addresses and slots. And the
spawner maps a slot's frames without scrubbing (`compositor_service.rs:225-233`, BUGS at 226-228), so
a client respawned into a slot an earlier client used would see that client's last pixels until it
painted. No shipped path respawns a compositor client, so it is a latent scrub-on-reuse gap rather
than a live leak; booting it would need the full client-registration handshake re-run on a respawn,
which this pass did not build.

## What I could not do

- The DMA publish race (claim 17) and the FS window-reuse leak (claim 24) are concurrency and
  boot-orchestration questions this pass identified and settled by read, for the reasons in each
  section.
- The compositor respawn gap (claim 25) is latent (no shipped respawn) and booting it needs the
  registration handshake re-run on a respawn, which this pass did not build.
- Kani proofs (the claims the table marks "milestone 194") were not re-run; `script/verify` is a
  heavy job and those were graded by the host `#[test]` suites that ran green beside them.
- Claim 28's test does not assert the fault pc, so a fault anywhere would pass it; the property
  itself held under source review, so the row stays held with this noted.
