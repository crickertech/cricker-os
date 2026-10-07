---
status: PARTIAL
raised: 2026-09-26
milestone_dependencies: 601, 670
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 608. Kernel tests give back their rendezvous points

*(Number and title provisional: the integrator confirms both at merge.)* Approved by calef
2026-09-26 as urgent. It comes from the ledger milestone 601 (the region table prints its peak)
built at `sched::PEAK_RENDEZVOUS`. `uuid_tests` panicked in #1344: "out of rendezvous points: 512
live at once", after only two tests were added. Every lane adding a test is at the same risk.
Built against #1366 (milestone 601), which had not merged when this lane started and has since
merged; this branch is rebased onto `main` past it.

## Why

`sched::MAX_RENDEZVOUS` is 512. With #1347, aarch64 ends `timetable_tests` at 505 of 512 live (7
spare). riscv64 ends at 491, `x86_64` at 315. 466 of the aarch64 505 sit on the kernel's own chunks
(`create_rendezvous`), which are never freed. So the number only climbs across a suite run. A
rendezvous created with `create_rendezvous_from(region)` instead goes when its region is reclaimed.
Milestone 152 (durable delegation: authority that outlives the session that requested it) already
used that pattern to fix its own test.

The ledger's per-module breakdown of the kernel-chunk count: `kernel::sched` tests (46),
`user::tests` (45), `ntp_tests` (37), `login_tests` (36 of 66; the other 30 already retyped from
session regions). Also `compositor_tests`/`display_tests` (34), and the file-service tests (51
across `rm_program_tests`, `dir_capability_tests`, `disk_tests`). Also
`sink_tests`/`date_tests`/`time_tests` (40), 23 other modules (97), and 80 created between tests
(services still wiring after a test returned).

## What is built

**Round 1: `kernel/src/sched.rs`'s own `mod tests`.** Eleven tests changed. Each used to create a
scratch rendezvous with `create_rendezvous()`, good for exactly that one test. Each now creates a
one- or two-page region with `memory_region::create`, and retypes the endpoint from it with
`create_rendezvous_from`. Each waits for its own spawned thread(s) to exit
(`sched::is_thread_present`), then reclaims the region before returning. The eleven:
`a_wake_without_delivery_cannot_complete_a_parked_receive`,
`a_reply_to_a_thread_parked_as_a_receiver_is_dropped`, `a_receiver_blocks_until_a_sender_arrives`,
`a_sender_blocks_until_a_receiver_arrives`, `a_request_gets_a_reply`, `a_call_gets_a_reply`,
`a_reply_reaches_the_caller_that_called`, `other_threads_run_while_one_is_blocked`,
`an_interrupt_becomes_a_message`, `a_spawn_quota_caps_live_children_and_replenishes_on_reap`,
`an_interrupt_that_arrives_before_the_wait_is_not_lost`.

Measured by CI, `rendezvous:` closing line, peak live of 512 (kernel-chunk count in parens):

| architecture | before (milestone 601, #1347) | after round 1 |
|---|---|---|
| aarch64 | 505 (466) | 493 (454) |
| riscv64 | 491 (452) | 479 (440) |
| `x86_64` | 315 (302) | 303 (290) |

A clean −12 on every architecture: the round moved twelve rendezvous total (one test,
`a_request_gets_a_reply`, retypes two from one region), and the ISA-independent kernel logic
under test means the same twelve move on every architecture. Round 1 landed first so round 2 could
be measured against it separately; see below.

**Round 2: the other candidates a single lane could safely reach.** Seven tests in
`kernel/src/user/tests.rs` changed the same way. Each test's own `report` rendezvous now comes from
a one-page region it creates and reclaims itself, independent of `spawn_hello`'s own
`Holding::release_or_fail`. The seven: `a_spawned_least_authority_demo_computes_and_reports`, and
six `spawn_hello`-built init-role tests (`userspace_init_delegates_an_interrupt_to_a_child`,
`userspace_init_brings_up_the_console_server`, `userspace_init_builds_a_driver_that_reads_real_hardware`,
`userspace_init_parses_an_elf_and_builds_a_running_child`,
`init_builds_the_demo_and_passes_it_an_argument`,
`init_runs_the_coremark_workload_and_it_checks_out`).

`kernel/src/user/date_tests.rs`'s `spawn_date` and
`spawn_date_with_diagnostics` now return the region their endpoint(s) were retyped from alongside
the endpoint(s), and every call site reclaims it once it has read what it needs.

Three cases in `user::tests.rs` were left alone, deliberately.
`a_user_client_moves_data_through_shared_memory`'s endpoints: the comment says the client spins
forever, so there is no safe moment to reclaim. `reclaim_frees_a_started_then_exited_childs_regions`'s
`report`: its own comment says the kernel's pinned region is deliberate, so the test's frame
accounting is not confused by it. `a_process_can_build_start_and_run_a_child_thread`'s `report`: its
`as_region`, `frames_region` and `thread_control_block_region` are never reclaimed either, so moving
`report` onto one of them would not free anything.

Measured by CI, same convention:

| architecture | after round 1 | after round 2 |
|---|---|---|
| aarch64 | 493 (454) | 476 (437) |
| riscv64 | 479 (440) | 464 (425) |
| `x86_64` | 303 (290) | 289 (276) |

From the original baseline that is aarch64 505 → 476 (−29), riscv64 491 → 464 (−27), `x86_64`
315 → 289 (−26). Not the clean, identical delta round 1 had: some of round 2's tests are
architecture-gated (`#[cfg(target_arch = "aarch64")]`), so the exact count freed differs slightly
per ISA.

## What is left

Eighteen tests fixed is the test-side work this lane judged it could do safely in one sitting.
aarch64's peak (476) is still above the 400 this milestone aimed at. The remaining modules in the
ledger, largest first, are all behind shared, always-compiled test-infrastructure files
(`fs_service.rs`, `ntp_service.rs`, `compositor_service.rs`/`display_service.rs`) that build real
disk I/O, a real network stack, or a several-mutually-distrusting-clients security model under
test. Giving each a region to retype its clients' rendezvous from is real design work on each one,
not a mechanical swap, so this lane stopped and filed it rather than rushing it:

- File-service tests (51): `fs_service.rs`'s `spawn_fs_client` and siblings build a client per
  test call with no region of its own to retype from today. Its servers (`wire_servers`,
  `spawn_block_server`) are `WIRED`-once for the whole boot and are correctly out of scope.
- `ntp_tests` (37): mostly behind `ntp_service.rs` and the network stack rather than the test file
  itself.
- `sink_tests` (part of the 40 shared with `date_tests`/`time_tests`): its own helpers carry real
  disk I/O and frame-accounting tests this lane judged too risky to touch without more time.
  `time_tests` runs through `pipeline_service.rs`'s scripted shells, which are deliberately leaked
  for the whole boot and are not candidates at all.
- `compositor_tests`/`display_tests` (34): mostly `compositor_service.rs::start`, which creates a
  doorbell, a report, and one rendezvous per client (up to `compositor::MAX_WINDOWS`) for a scene
  built fresh per test. `display_service.rs`'s `start_screen_terminal` already retypes from a
  region, so the pattern is proven. It is just not yet applied to the compositor's own arrays.
- `login_tests` (36): likely mostly the login service's own boot-lived endpoints rather than
  per-test residue, unconfirmed.

## Follow-on

- **Milestone 670.** Milestone 670 (test helpers give their clients a region to retype their rendezvous from).
  `design/roadmap/0670-test-helpers-give-their-clients-a-region-to-retype-from.md`. A lane of
  its own for the file-service, NTP, sink and compositor/display test helpers. Each needs its
  per-client spawn given a region of its own, mirroring `x86_userspace_round`, #1344's
  `fs_subtree_caretaker`-narrowed walk tests, and `display_service::start_screen_terminal`, before
  its report endpoint can move off a kernel chunk. This is most of the remaining aarch64 residue,
  and it covers the `login_tests` question below too.
- **Recorded.** Whether `login_tests`' 36 kernel-chunk rendezvous are the login service's own
  boot-lived endpoints, not reclaimable, same as the FS server, or per-test residue is unconfirmed.
  A future lane should check before assuming either.

## Index row

Eighteen of the kernel's own IPC and init-role tests stopped spending the 512-entry rendezvous
registry permanently, moving the aarch64 kernel-chunk floor `sched::PEAK_RENDEZVOUS` reports from
505 to 476 of 512. The remaining large holders are filed as a follow-on proposal rather than rushed.
