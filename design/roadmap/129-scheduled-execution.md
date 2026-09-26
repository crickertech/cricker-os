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
narrowed archive and a backable `--mem` grant are built. What is left waits on calef and on
milestone 152 (durable delegation), recorded below and checked against the tree on 2026-09-26.

Runtime registration needs a wire format and a ruling on which
process holds a user's schedule, and both are calef's. The proposal is
[notes/scheduled-execution/registration.md](../../notes/scheduled-execution/registration.md). It
also needs a registrar, which is a user's durable session. That type went with `smb_server` on
2026-08-30, and milestone 152's lane is rebuilding it. Until 2026-09-26 this line read `NONE`, on the
claim that both blockers had cleared; the second half of that stopped being true when the session
type was deleted.

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

## The finding: milestone 106's fifth consumer

There is no timed wait in this kernel, so a program whose whole purpose is to act at a time can only
yield and re-read the counter. A running timetable costs a core's worth of yields, and it reaps dead
children lazily, because a process has one wait point. `Registry::next_deadline` already computes
the instant a timed wait would block until, so the loop changes by one line when one exists.
Milestone 106 (a wait that ends on either the interrupt or the deadline) is gated on milestone 263
(can a userspace process hold a timer).

## What is left

- Runtime registration, including removal. The proposal recommends one timetable per durable
  session, replaced a whole document at a time, so removing an entry is a replace without it. It
  also asks for five smaller rulings. It is calef's because it is a wire format, and it waits on
  milestone 152 for the session that would hold it.
- Wiring `crates/schedule_store` into a running scheduler. Under the proposal this is "spawn the
  session's timetable with its stored document", so it waits on the same ruling.
- One image per entry. The timetable keeps every entry's image mapped for its whole life, so
  splitting the archive does not narrow what a compromise reaches. Narrowing it needs a
  `spawner.rs`-shaped helper per entry, which is machinery nothing has. Under the proposal the reach
  shrinks to one user's plan.
- Calendar syntax and wall-clock entries. Each is its own decision: what a `0 2 * * *` entry does
  across an NTP step or a DST change, and whether a scheduler is ever granted a clock.

## Scope note

Sequenced by need. The first real customer is the housekeeping of milestone 55 (Time Machine), which does not exist, so
the shipped document is a demonstration written to show every answer registration can give.

## BUGS

- `timetable` is a provisional name for the crate, the program and the document, said so in every
  module header. `cron`, `almanac`, `metronome` and `scheduler` were refused; `scheduler` is already
  `kernel/src/sched.rs`.
- The document is compiled in. Where it lives is also what answers who may register, so today that
  right is the right to rebuild the image.
- At most one `--mem` instance may be outstanding, and every other entry fires late while it runs.

## Follow-on

- **Outstanding.** Runtime registration and removal: no opcode exists in `crates/timetable`, and no
  file under `design/decisions/` covers it. The proposal is
  `notes/scheduled-execution/registration.md`, and it waits on calef and on milestone 152's durable
  session, which no file under `components/` or `crates/` defines. Checked 2026-09-26.
- **Outstanding.** Wiring the store into a running scheduler: neither `crates/timetable` nor
  `components/src/timetable.rs` names `schedule_store`. It follows the registration ruling. Checked
  2026-09-26.
- **Outstanding.** One image per entry: `components/src/root_supervisor.rs` still builds exactly one
  spawner, and nothing builds sub-builders sized to a document. Checked 2026-09-26.
- **Outstanding.** Calendar syntax and wall-clock entries: `crates/timetable`'s grammar is still
  `every` and `at-boot`, and the scheduler holds no clock to give. Checked 2026-09-26.
- **Done.** Persistence as a store was built by milestone 152: `crates/schedule_store`, per §122 (the on-disk, per-user schedule store) and §125 (which identities
  have pending work), and `components/src/session_reviver.rs`, per §123 (boot-time re-derivation).
- **Done.** The lifted-session obstacle went with `smb_server` on 2026-08-30, so no private type has
  to be moved into a crate. The session now has to be rebuilt rather than lifted, which is 152's.
- **Done.** A designation the scheduler cannot back reports `Unbacked`, carried by
  `crates/timetable/src/lib.rs` on 2026-09-26, closing milestone 326's recorded finding.
- **Recorded.** The compiled-in document, recorded in `components/src/timetable.rs`.
- **Recorded.** At most one `--mem` instance outstanding, recorded in `components/src/timetable.rs`.
- **Recorded.** The yield loop and lazy reaping, recorded in `notes/scheduled-execution.md`, until
  milestone 106 lands.

## Index row

The backup server owes housekeeping on a schedule; Unix cron is ambient authority made periodic,
and the capability shape inverts it: an entry is a grant expression plus a schedule, checked at
registration like a command line at the prompt. Built: the interval scheduler with four registration
answers (2026-08-18), the archive narrowed to the plan (2026-08-18), a backable `--mem` grant
(2026-08-22), and designations reported as unbacked rather than refused (2026-09-26). Remaining:
runtime registration and removal, proposed for calef and waiting on milestone 152's durable session;
one image per entry; calendar and wall-clock entries.
