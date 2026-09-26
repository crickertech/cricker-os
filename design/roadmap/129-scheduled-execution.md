---
status: PARTIAL
raised: 2026-08-15
milestone_dependencies: 152
decision_dependencies: unwritten
machine_requirements: none
specific_machine: none
needs_person: no
---
# 129. Scheduled execution: a cron whose every entry is a grant

Minted 2026-08-15 at calef's request. A backup server owes housekeeping on a
schedule (snapshot thinning, scrub passes, log rotation) even though the Mac initiates the backups
themselves, and nothing else on the roadmap ran anything on a schedule. The interval scheduler, a
narrowed archive, a backable `--mem` grant and runtime replacement under §222 (who holds a user's
schedule) are built. One image per entry was refused on 2026-09-26. What is left is a calendar
proposal awaiting calef and one connection waiting on milestone 152 (durable delegation), both
recorded below and checked on 2026-09-26.

One proposal awaits calef:
[calendar-and-wall-clock.md](../../notes/scheduled-execution/calendar-and-wall-clock.md) asks for a
grammar and a clock-step rule. Connecting the built `REPLACE` handler to a real session
waits on milestone 152, because the registrar is a user's durable session and that type went with
`smb_server` on 2026-08-30.

## In brief

Unix cron is ambient authority made periodic: whatever root's crontab says happens, and the crontab
is the attack surface. The capability shape inverts it. An entry is a grant expression plus a
schedule, checked at registration by `grant_plan::plan`, the same function the shell checks a prompt
line with; milestone 31 (a capability shell) built that check. An entry cannot name what its manifest does not declare, and compromising
the scheduler yields the entries' summed endowments rather than the system. The schedule vocabulary
is deliberately small: `every <interval>` and `at-boot`. The full account is
[notes/scheduled-execution.md](../../notes/scheduled-execution.md).

## Built: the interval scheduler, 2026-08-18

- `crates/timetable` holds the decision and no IO. Registration's four answers are host-tested and
  the fire arithmetic is reached by five Kani harnesses.
- `components/src/timetable.rs` holds four capabilities: an output endpoint, an untyped budget, a
  child report endpoint and a supervision endpoint. No clock, no directory, no console, no network.
- `components/timetable.conf` is the document, compiled in.
- `kernel/src/user/timetable_tests.rs` is one module that runs on aarch64, riscv64 and x86_64.

The demonstration is the refusals. A crontab has one answer; registration here has four, and the
split that matters is "the line is wrong" against "this scheduler holds nothing to back it". `every
1s date` runs in any crontab, because there the clock is ambient. Here it is refused in writing,
before the first tick, because the wall clock is a capability under §43 (reading the clock is a page) and this scheduler holds none.
The cross-ISA test asserts the refusal and that the entry never runs.

The block's first sketch said the service would hold one clock capability. It holds none and needs
none: monotonic time is ambient (`user_rt::monotonic_nanos`), and the wall clock is the thing §43
made a capability. That is why a scheduled entry can be refused a clock at all.

## Built: the archive narrowed to the plan, 2026-08-18

The scheduler used to be handed the whole initrd. It now holds exactly the programs its plan builds,
because `Registry::programs` is known before the first tick. The set is a written list at the spawn
site, kept honest by a host test. `script/stack-frame-check` refuses computing it in the kernel,
since `Registry::register` carries a 21,632-byte frame against a 4,096-byte guard page. The program
also audits what it was handed and prints one of two sentences, and the test asserts the narrow one.

## Built: a backable `--mem` grant, 2026-08-22

`timetable::SHIPPED_HELD.mem_pages` is 4, and `at-boot memory_grant_depleter --mem 4` is planned,
backed and fires. The grant nests inside its instance's region, because `regions::destroy_outcome`
refuses a region with a live child. A supervisor learns a dead child's tid and nothing else, so at
most one `--mem` instance may be outstanding: that is what pairs a death with its grant. Every other
entry pays by firing late while it runs. `components/src/timetable.rs`'s `BUGS` has the whole cost.

## Built: a designation it cannot back is unbacked, 2026-09-26

`admit` planned each entry against the scheduler's own directory bit, so `rm -r logs` in a
timetable holding no directory came back `Refused`, which tells a reader to edit a line with nothing
wrong in it. Mutants from milestone 326 (turn a mutation score upward) found `Unbacked::File` and `Unbacked::Directory` unreachable.
The planner is now lent a directory and `unbacked` alone decides, including for a streamed operand
such as `wc report.txt`. Host test:
`a_designation_the_scheduler_cannot_back_is_unbacked_rather_than_refused`.

## Built: replacing a running document, 2026-09-26

§222 ruled one timetable per durable session, changed by replacing the whole document. Built:

- `crates/timetable/src/registration.rs` is the page layout, `REPLACE`, the status codes and the
  verdict word (eight bits per entry, with a kept-beat bit). `Registry::arm_after` keeps the beat
  of every byte-identical line. Host-tested. All provisional names.
- `components/src/timetable.rs` takes a registration page in `a2`. It polls the request word once
  per pass, because a blocking receive would stop it watching the clock (milestone 106). A
  replacement is parsed, registered and resolved whole before it replaces anything. An empty one
  ends the process, since an idle timetable would hold its session up under §16 (object
  revocation). A timetable handed the run-unvouched capability runs nothing, so no job can hold it,
  which is what lets §220 (signed builds, and trusting a key is scoped) reach scheduled work.
- `kernel/src/user/timetable_tests.rs` is the registrar until milestone 152 rebuilds the session. It
  sends `schedule_store::fixture::DEMO_SCHEDULE_DOC`, the bytes the store test writes to disk, then
  a document that does not parse, an edit and an empty document. A second test hands the timetable
  the run-unvouched capability and asserts it refuses. Both run on all three architectures.

## The finding: milestone 106's fifth consumer

There is no timed wait in this kernel, so a program whose whole purpose is to act at a time can only
yield and re-read the counter. A running timetable costs a core's worth of yields, and it reaps dead
children lazily, because a process has one wait point. `Registry::next_deadline` already computes
the instant a timed wait would block until, so the loop changes by one line when one exists.
Milestone 106 (a wait that ends on either the interrupt or the deadline) is gated on milestone 263
(can a userspace process hold a timer).

## What is left

- Connecting a real session: the durable session spawns its timetable, writes the store and sends
  `REPLACE`, and at boot `session_reviver` does the same from the stored file. Waits on milestone
  152, whose session type does not exist yet.
- Calendar and wall-clock entries, proposed together: `daily` and `weekly` in UTC, and a rule for a
  clock that steps.

## Scope note

Sequenced by need. The first real customer is the housekeeping of milestone 55 (Time Machine), which does not exist, so
the shipped document is a demonstration written to show every answer registration can give.

## BUGS

- `timetable` is a provisional name for the crate, the program and the document, said so in every
  module header. `cron`, `almanac`, `metronome` and `scheduler` were refused; `scheduler` is already
  `kernel/src/sched.rs`.
- Without a registration page the document is still compiled in, and the shipped boot-time test
  runs that way. With one, the right to register is holding the page, which §222 gives the session.
- Registration control is a polled word in the page, not a message, until a deadline wait exists.
- At most one `--mem` instance may be outstanding, and every other entry fires late while it runs.

## Follow-on

- **Outstanding.** Connecting a real registrar: only `kernel/src/user/timetable_tests.rs` writes a
  registration page, and no file under `components/` or `crates/` defines milestone 152's durable
  session. The same session is what feeds `crates/schedule_store`'s file to a running timetable.
  Checked 2026-09-26.
- **Refused.** One image per entry, by calef on 2026-09-26 ("Refuse it?", "Yes"): an image is code,
  not authority, so a helper per entry would buy nothing. The reason is
  `notes/scheduled-execution/one-image-per-entry.md`.
- **Outstanding.** Calendar syntax and wall-clock entries: proposed in
  `notes/scheduled-execution/calendar-and-wall-clock.md`, awaiting calef; `timetable::parse` still
  knows `every` and `at-boot` only. Checked 2026-09-26.
- **Done.** Runtime registration and removal, per §222: `crates/timetable/src/registration.rs` and
  `components/src/timetable.rs`, 2026-09-26.
- **Done.** Persistence as a store was built by milestone 152: `crates/schedule_store`, per §122 (the on-disk, per-user schedule store) and §125 (which identities
  have pending work), and `components/src/session_reviver.rs`, per §123 (boot-time re-derivation).
- **Done.** The lifted-session obstacle went with `smb_server` on 2026-08-30, so no private type has
  to be moved into a crate. The session now has to be rebuilt rather than lifted, which is 152's.
- **Done.** A designation the scheduler cannot back reports `Unbacked`, carried by
  `crates/timetable/src/lib.rs` on 2026-09-26, closing milestone 326's recorded finding.
- **Recorded.** The compiled-in document without a registrar, recorded in `components/src/timetable.rs`.
- **Recorded.** At most one `--mem` instance outstanding, recorded in `components/src/timetable.rs`.
- **Recorded.** The yield loop, lazy reaping and the polled registration word, recorded in
  `components/src/timetable.rs`, until milestone 106 lands.

## Index row

The backup server owes housekeeping on a schedule; Unix cron is ambient authority made periodic,
and the capability shape inverts it: an entry is a grant expression plus a schedule, checked at
registration like a command line at the prompt. Built: the interval scheduler with four registration
answers (2026-08-18), the archive narrowed to the plan (2026-08-18), a backable `--mem` grant
(2026-08-22), designations reported as unbacked (2026-09-26), and whole-document replacement under
§222 (2026-09-26). One image per entry was refused (2026-09-26). Remaining: connecting a real
session (milestone 152), and calendar and wall-clock entries, proposed for calef.
