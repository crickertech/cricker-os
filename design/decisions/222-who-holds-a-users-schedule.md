---
status: DECIDED
raised: 2026-09-26
decided: 2026-09-26
ratified_by: calef
---

# 222. Who holds a user's schedule, and how it is changed while their session lives

Raised 2026-09-26 by the maintainer, from the proposal the lane for milestone 129 (scheduled
execution) wrote in
[notes/scheduled-execution/registration.md](../../notes/scheduled-execution/registration.md) on pull
request #1345. The note is the lane's argument and holds the detail; this file is the ask to calef,
and it links to the note rather than restating it. *(Section number provisional until the merge
queue lands it. It was the next free number on 2026-09-26, after §221 (the boot prompt is the
owner's console), claimed by #1340.)*

## The ruling

calef, 2026-09-26 (recorded 16:37 UTC): *"Yes, C with all five."* A user's schedule is held by a
timetable their durable session spawns and supervises, and it is changed by replacing the whole
document; removing an entry is a replace without that line. The five sub-rulings hold as
recommended below:

1. The document travels in one shared page, and the reply is one word of verdicts.
2. A replace is parsed and planned whole, or it leaves the old schedule running.
3. A byte-identical entry keeps its phase.
4. A job from a removed entry runs to completion.
5. The session writes the store, and the timetable holds no directory.

Options A and B are refused for the reasons in
the table. `REPLACE` is still a provisional name.

The handler can be built now, with the kernel test harness standing in as registrar. Connecting a
real session still waits on milestone 152 (durable delegation).

The rest of this file is the section as it stood before the ruling.

## What is being decided

`components/timetable.conf` is compiled into the scheduler, so the right to register a scheduled job
is the right to rebuild the image. A person who has logged in should be able to schedule a job on
their own authority, change it, and remove one entry of it. Two things here are an architect's call
under AGENTS.md, because two programs will agree on them and because they decide where authority
pools:

1. Which process holds a user's schedule.
2. What crosses the wire to change it, including how one entry is removed.

Five smaller rulings sit inside the recommended answer and are listed below it. Each can be answered
on its own.

## The options

| | shape | why it loses |
|---|---|---|
| A | One system timetable, a separate registration endpoint and document per session | The scheduler must back every entry it fires, so it holds the union of every user's delegated authority. A compromise reaches all of it, which is the sum milestone 129 exists to bound, now taken across users. Revoking one user under §108 (disabling a user's login credentials kills their durable session) becomes a message the scheduler must act on instead of a subtree dying. |
| B | One system timetable, `ADD` returns a handle, `REMOVE` takes one | A's union problem, plus two of its own. A capability handle names nothing after a reboot, once `session_reviver` has re-derived the schedule. A numeric handle is an ambient name the scheduler has to check against its caller. And §122 (the on-disk, per-user schedule store) stores one whole document per identity, so every per-entry call would be folded back into a document before it could be written. |
| C | One timetable per durable session, changed by replacing the whole document | Recommended. Its cost is the yield loop below. |

## The recommendation: C

The session spawns its own timetable from its own budget and supervises it. The authority it passes
in is exactly what it lets its jobs hold. The endpoint to that timetable is the registration right,
so there is no badge and no table of registrars. There is one operation, `REPLACE` (a provisional
name), which carries a whole document, and the reply carries the plan. Removing an entry is a
replace without that line; removing all of them is an empty document, or the session destroying its
timetable.

The reasons, in the order they carry weight:

- A compromise reaches one user's delegated authority, so the milestone's claim holds per user.
- Revocation is free. §108's cascade kills the session, and §40 (a supervisor's death is its
  subtree's death) takes the timetable and every job with it. Nothing new has to be written.
- The store is already this shape. §122 keeps one document per identity, so registering becomes
  "write the file, then replace", and boot becomes "spawn a timetable with the file".

### The five smaller rulings inside C

Each is cheap to reverse until a second program speaks the protocol. The note has the reasoning for
each; the recommendation is:

1. Transport. The document goes in one shared 4096-byte page, as `filesystem_protocol` already
   passes names and file bytes. `MAX_ENTRIES` is 8 and the shipped document is about 200 bytes. The
   reply is one word of verdicts, eight bits per entry, with the printed plan in the page.
2. Atomicity. The new document is parsed and planned whole before it replaces the old. A parse error
   leaves the old schedule running and returns the line number.
3. Phase. An entry whose text is byte-identical keeps its beat; a new or changed one arms fresh.
   Otherwise editing one line shifts every other schedule.
4. In-flight jobs. A job from a removed entry runs to completion and is reaped normally.
5. Who writes the store. The session writes the file and then sends the replace. The timetable keeps
   holding no directory, because its four-capability list is the milestone's demonstration.

## The seven questions

1. What else was considered. A and B, each refused in the table above for a stated reason.
2. What the tree already does. `components/src/login.rs` builds a fresh budget and directory for
   every login, and §109 (attribution is a property of a channel, not of a capability) already chose
   a fresh object per principal over a shared view. C applies the same rule once more.
   `Registry::register(doc, held)` needs no change, since `held` becomes whatever the session gives.
3. Prior art, read by the lane on 2026-09-26. crontab(1) is whole-file: install replaces the table,
   `-e` edits and reinstalls, `-r` removes. systemd-run(1) makes one transient timer unit per call,
   which is B's shape. Stopping one by unit name through `systemctl` is recalled rather than read.
4. Is the premise true. The lane checked four claims against `256815e56`, and the maintainer
   rechecked the two that matter at `18dab9c6a` on 2026-09-26. `MAX_ENTRIES` is 8 in
   `crates/timetable/src/lib.rs`. No durable session type exists: `components/src/session_reviver.rs`
   names one only in comments, so the registrar waits on milestone 152 (durable delegation).
   `crates/system_initializer` does not name the timetable, so C adds no process to a real boot
   until a session schedules something.
5. What each costs. Memory: the kernel test sizes one timetable at a 768-page budget and a 32-page
   stack, and a one-entry document needs one 48-page instance at a time. CPU: there is no timed
   wait, so every running timetable yields in a loop. C pays that once per scheduling user where A
   pays it once. That cost is not measured, because no boot runs two timetables, and it ends when
   milestone 106 (a wait that ends on either the interrupt or the deadline) lands. This is the best
   argument for A. Code: C is the smallest build, and that is not the reason.
6. How reversible. The operation number and page layout are a wire format, expensive once a second
   program speaks them. Today none does. Which process holds the schedule is cheap to change while
   no user has registered anything. The on-disk store does not depend on the choice, since every
   option reads §122's document. Nobody has acted on any of it.
7. At equal cost, C still wins, on compromise radius and the free cascade. It also narrows the
   one-image-per-entry gap, since a compromised timetable reaches one user's plan rather than the
   machine's.

## What is blocked until this is answered

- A `REPLACE` handler in `components/src/timetable.rs` and its constant in `crates/timetable`, on
  all three architectures.
- Wiring `crates/schedule_store` into a running scheduler rather than only into `session_reviver`.
- Milestone 129's removal item, which has had no answer since #387.

Connecting a real session also waits on milestone 152, whatever the answer. The handler does not: a
lane can build it with the kernel test harness standing in as registrar.

## What each answer means

- Yes to C and the five rulings: a lane builds the handler as above.
- A instead: the same handler plus a per-registrar table, and a `BUGS` entry at the scheduler's
  authority list recording the cross-user union.
- Refuse all three: the document stays compiled in, and milestone 129 stays PARTIAL on this item.
