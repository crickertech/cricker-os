---
status: DECIDED
raised: 2026-08-24
decided: 2026-08-24
ratified_by: calef
---

# 122. The on-disk, per-user schedule store: format, write path, read-at-boot path

calef, 2026-08-24, ratified option 1: reuse `timetable::parse`'s document
format unchanged, one file per identity inside that identity's own subtree, written and read through
ordinary `filesystem_proto` verbs. Confirmed this is a persistence-format decision only, not an
invitation to adopt cron's calendar syntax, which stays milestone 129's own deliberately deferred
question. Raised 2026-08-24 by milestone 152's lane, from the milestone's own BUGS section: "the
on-disk, per-user schedule store has no format, no write path, and no read-at-boot path." The number
is **provisional**, minted by this lane against the current `design/decisions/` index (highest
existing was 121 at the time of writing); expect renumbering at merge.

## The problem

Boot-time re-derivation (152's fourth design piece, and §123's own subject) needs a durable, on-disk
record: which identities have a durable session with pending work, and what that work is, so boot can
re-establish each one's authority fresh without a live person presenting credentials. Milestone 129's
own roadmap already named the gap without designing it ("Calendar syntax, wall-clock entries,
persistence... none started"), and today's `timetable::parse` reads a **compile-time** document
(`user/timetable.conf`, baked into the initrd) that a running system cannot write to at all. Nothing
in this tree persists a schedule entry written by a live session and reads it back after a reboot.

## Is the premise true?

Checked directly rather than assumed, in two directions.

Confirmed: no write path or read-at-boot path exists anywhere. A repo-wide grep for anything that
opens a file and writes a schedule-shaped record finds nothing: `crates/timetable::parse` takes a
`&str` that every caller (`components/src/timetable.rs`, its own host tests) gets from `include_str!` or a
literal, never from a live read. There is no `timetable`-side `fs_proto` client at all today.

Corrected: milestone 152's own doc misdescribes its nearest precedent, and this matters for what
"already does this" means below. 152's design section calls "the credential store's own existing
persistence (milestone 56's sealed store)" one of the two things boot-time trust rests on. Reading
`crates/credentialer/src/lib.rs`, `components/src/credentialer.rs` and `notes/credentials.md` directly finds the
opposite: `credentialer::Store<N>` is an in-memory, `no_std`, no-`alloc` structure built during a **Provision**
phase and never written to any block device, and `notes/credentials.md`'s own BUGS section says so in
so many words: "Nothing survives a reboot. The store is memory only, provisioned at boot... Secrets
at rest is the open question." "Sealed" there means *write-locked* (the provision endpoint is
`cap_delete`d once `SEAL` arrives, so no client can ever write another record), not *durable*. It is
real prior art for the *write-once-then-lock* shape a schedule store might also want, but it is not
prior art for on-disk persistence, and citing it as though it were would have this decision reach the
wrong analog. There is, as of this writing, no durable on-disk store of any kind in this tree built
by a userspace service; the only thing on disk is what `RedoxFS` holds through `filesystem_proto`, which
several other things already write and read back across reboots (the SMB share's own files, the
provisioning fixtures milestone 63's gate reads back). That is the tree's actual closest analog, and it
is named as Option 1 below.

## What else was considered, and prior art outside the tree

A binary, fixed-record format (mirroring `credentialer::Record`'s `encode`/`decode` shape: no allocator,
fixed field widths, a round-trip test). Considered and folded into Option 1 below as an *encoding*
choice rather than a separate option, because the harder question is where the bytes live and who may
write them, not whether they are text or binary.

**crontab's on-disk shape**: one file per user in `/var/spool/cron/crontabs/`, permission-gated by the
directory rather than by the file format, with `crontab -e` as the only sanctioned writer. This tree's
own equivalent of "one file per user" is already how milestone 117 (DECISIONS §117) scopes a principal's
subtree: "a principal's subtree is named by its identity string, created at provisioning time." A
per-user schedule file living inside that same subtree, rather than in a shared system location, gets
per-user scoping for free from a mechanism this tree already has, rather than needing a new one.

**systemd timer units**: one `.timer` unit file per job in `/etc/systemd/system/` or a user's own
`~/.config/systemd/user/`, `INI`-shaped, read at boot by `systemd` walking the directory. 152's own doc
already checked `loginctl enable-linger` for the *lifecycle* question and rejected its static-toggle
shape; that finding does not extend to the *format* question, which is separate. The unit-file shape
(one file per schedulable thing, human-editable, no central registry) is closer to crontab's than to a
database, and is worth naming because it is the same shape Option 1 below lands on independently.

## What the tree already does in the analogous case

`crates/timetable::parse`'s own document format is the closest *parsing* precedent, and it is real
prior art worth reusing rather than re-deriving: two schedule words (`every <interval>`, `at-boot`),
`#` comments, one entry per line, a grant expression as the payload, `no_std`/no-`alloc`, fixed-size
inline table (`MAX_ENTRIES = 8`), and host-tested in milliseconds. It already answers "what does a
schedule entry look like" for the in-image case; what it does not answer is where the bytes underneath
that parser come from at boot, or how they get there during a live session, which is what has no
precedent (see "Is the premise true?" above).

`filesystem_proto` plus RedoxFS is the tree's only real durable store, proven repeatedly: milestone
55's write path, the durability-under-crash tests (`filesystem_proto::fixture::durability`), and every SMB
write the gate makes durable through `fs::SYNC`. Nothing about a schedule entry needs a new persistence
primitive; it needs a place in the one that already exists.

## What each option costs

Option 1 (recommended): an ordinary file, `timetable::parse`'s own text format, one file per
identity's subtree, written through `filesystem_proto` by whatever process is standing up a durable
session, read at boot by whatever process performs re-derivation.

- *Format*: reuse `crates/timetable::parse` byte-for-byte. Zero new parsing code; the crate is already
  `no_std`, host-tested, and its own doc already states the reason calendar syntax is deliberately
  excluded (§123-adjacent: same crate, same restraint). A schedule registered at runtime is exactly
  the same two words a compile-time `timetable.conf` line is, so the parser does not need to learn a
  second dialect.
- *Location*: `<identity's own subtree>/schedule` (exact path a detail for whoever builds it), using
  milestone 117's existing per-identity subtree rather than a shared system directory. This is what
  buys per-user scoping for free: a directory capability narrowed to one identity's subtree (which
  `login.rs`'s `mint()` already knows how to build) cannot read or write any other identity's file,
  with no new access-control mechanism to write.
- *Write path*: an ordinary `fs::CREATE`/`fs::WRITE` (or `fs::TRUNCATE` + `fs::WRITE` for a full
  rewrite, which is simpler than an append-and-compact scheme for a file bounded by
  `timetable::MAX_ENTRIES`, 8 lines) through the directory capability the durable session already
  holds a narrowing of. No new capability, no new server: the FS server already answers these verbs.
- *Read-at-boot path*: `fs::OPENDIR` + `fs::OPEN` + `fs::READ` under whatever capability boot-time
  re-derivation is granted (§123's subject; this decision only assumes *some* read capability into the
  identity's subtree exists to grant, the same dependency §123 names in the other direction).
- *Cost, measured against the alternative*: this is the parser this tree already has, tested and
  proven, against a filesystem this tree already writes to and reads back from across reboots. The
  only genuinely new code is the two IPC call sites (write it, read it), which is a page of client code
  each, not a new subsystem.

Option 2: a binary, fixed-record format purpose-built for this store, `credentialer::Record`'s
`encode`/`decode` shape applied to a schedule entry (identity, schedule, grant expression, as fixed
fields).

- *For*: a record this small (an interval, a program id, an arg, at most a few named grants) has a
  natural fixed encoding, and a binary format is not human-editable, which removes "a person hand-edits
  the live store and breaks the parser" as a failure mode a text format has.
- *Against*: buys nothing Option 1 does not already have. `timetable::parse`'s document format is
  already fixed-shape internally (a bounded array, not a `Vec`) and already handles malformed input by
  naming the line, which a binary format would need to reinvent (a bad byte instead of a bad line). A
  person hand-editing the live store is not a threat model anything else in this tree defends against
  (the shipped `timetable.conf` is plain text, checked into the repo, edited by whoever has commit
  access), so the property option 2 buys is not one this decision needs to buy.

Option 3: extend `crates/credentialer`'s `Store<N>` shape (or a sibling crate built the same way) as a
dedicated "schedule store" service, mirroring the credentialer's provision/serve split.

- *For*: reuses a real in-tree pattern (fixed-size, `no_std`, no-`alloc`, a round-trip-tested record
  encoding) rather than the filesystem.
- *Against*: rejected on the corrected premise above. `credentialer::Store` is memory-only *by design*
  ("nothing survives a reboot" is stated as an open problem, not a feature), so building the schedule
  store the same way would import the exact gap this decision exists to close, and would need to solve
  "how does this survive a reboot" from scratch rather than inheriting it from a filesystem that
  already does. This option would also mean a **new service process** holding a new kind of durable
  state, where Option 1 needs no new process at all: the FS server already exists, already serves every
  other durable write in this tree, and does not need to learn what a schedule entry is.

## The recommendation

**Option 1.** Reuse `crates/timetable::parse`'s document format unchanged, store one file per identity
inside that identity's own subtree (milestone 117's existing per-user scoping), and write/read it
through ordinary `filesystem_proto` verbs under capabilities the writer (a live session registering a
job) and the reader (boot-time re-derivation, §123) already need to hold for other reasons. This is the
cheapest of the three by a wide margin because it is not really a new mechanism: it is gluing two
already-built, already-proven pieces (the parser and the filesystem) together with two IPC call sites.

## How reversible is this, and who has already acted on it

Nobody has acted on this yet, and this decision only names a *format* (text, `timetable::parse`'s
dialect) and a *location* (inside the identity's own subtree). Both are the AGENTS.md-flagged expensive
kind of change once something depends on them ("anything two programs agree on"), which is the reason
this is a proposal rather than code: once a durable session's registrar writes files in this shape and
boot-time re-derivation reads them, the format is a wire contract between two programs and changing it
later needs a migration, not an edit. Nothing currently reads or writes any such file, so the decision
is cheap to make now and expensive to unmake later, which is exactly AGENTS.md's test for when to write
a decision doc rather than just build it.

## What this does not decide, and is blocked on

- The exact write path's call sites (which process performs the write: the durable session itself,
  a program it spawns, some other component) are not named here, because that depends on piece 1's
  `DurableSession` (this lane's own structural work) eventually being wired to a real registrar
  (milestone 129/#387), which is explicitly out of this lane's scope.
- Whether a schedule entry needs anything beyond what `timetable::Entry` already carries (line,
  schedule, grant-expression bytes) for the durable case (e.g., a record of *when it was last known to
  have pending work*), which §123's own BUGS-adjacent question ("unconditional re-derivation vs.
  scoped to sessions with pending work at shutdown") may need. Left to whoever builds the write path,
  once §123 answers the scoping question this decision does not.
- This decision is a dependency of §123's boot-time re-derivation mechanism, named there as "some
  read capability over that store," not designed there. The two are independent in shape (this is about
  the bytes, §123 is about the privilege that reads them) but sequenced: re-derivation cannot be built
  until both are answered.
