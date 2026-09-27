---
status: PARTIAL
raised: 2026-08-22
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 152. Durable delegation: authority that outlives the session that requested it

Updated 2026-09-26 (UTC). Minted 2026-08-22, from a milestone 129 (scheduled execution) discussion: calef
wants nife to support multiple users, and wants the jobs a user schedules to carry capabilities that
reflect that user's own authority. Working through what that requires surfaced a gap this tree has
not needed to close before, and #387 (milestone 129's `--mem` grant, held pending this) is where it
was found. The design below was worked out the same day, in conversation; three of its four design
pieces are now built and tested (the durable session itself, the on-disk schedule store, and
boot-time re-derivation; see "What was built" below, both entries), and moved from `NOT-STARTED`
because the milestone is no longer nothing but a design: what remains is wiring a real registrar
against the pieces already proven (#387), which is real, separate work rather than a detail of what
is already built.

The gate on milestone 49 cleared 2026-08-27: milestone 49 (users, login, and attribution) reached BUILT, so
the real identity this milestone's design needed to attach to now exists. The design fork itself
(what durably represents a user) was already answered below; what remains is wiring a real
registrar against the pieces already proven (#387), which is real, separate work and not a further
decision of this milestone's own -- see "What was built" and the BUGS entry below for exactly what
that is. Not attempted by this update: milestone 49's own lane was scoped to milestone 49 alone.

## In brief

Option 3 of #387's runtime-registration question already gives "a scheduled job can never hold more
authority than its registrar held" for free: `timetable::Registry::register(doc, held)` already
takes an arbitrary `Held`, so a registrar handed a narrower bundle than the scheduler's own produces
narrower jobs, structurally. Making that registrar a user's own session instead of a fixed
system component is a small idea with one hard consequence: a session's authority exists only as
long as the session does, but a scheduled job needs to keep firing after the user who registered it
disconnects.

Concretely: if calef authenticates, registers `every 24h verify_backup`, and disconnects, that
job still needs to fire tomorrow, on the authority he held at registration. Whatever holds that
authority between registration and firing has to outlive the connection.

## The existing rule this collides with, and what it implies

DECISIONS §92 already decided the general shape of "a component holding derived authority," and
its rule is the opposite of durable: *"A caretaker holds authority derived from a grant made to
one client. When the grantee dies, the derived authority should die."* `fs_subtree_caretaker` is
supervised by, and dies with, the client it serves (§40's subtree-death rule). That is the right
rule for the case it was built for (a directory grant narrowed for one live command), and it is
exactly the wrong rule for a scheduled job, which is supposed to survive its registrar disconnecting.

This does not mean §92 is wrong; it means a durable delegation needs a different supervisor than a
live connection. §92's own logic says the authority should die when *its client* dies. The design
below answers what that client should be for a durable delegation, rather than inventing a new
supervision rule.

§92's own BUGS section already named the adjacent gap: *"This says nothing about a caretaker
with no client, because none exists yet."* This milestone is that case, with a real motivating case
instead of a hypothetical.

## The design, worked out 2026-08-22

Four pieces, discussed in order and each answered before moving to the next.

### The durable principal is the session itself, kept alive by its own live children

No new kind of object. The durable thing a scheduled job's authority is supervised by is the
user's own login session, and what keeps it alive past a disconnect is exactly the rule
`Untyped::DESTROY` already has (§16): *a parent with live children refuses to be destroyed, and
becomes destroyable again once they're gone.* A session that refuses to tear down while it still has
scheduled-job children is that rule, applied to a session instead of a region. Nothing here needed
before was a snapshot of the user's authority or a live re-check against an external identity
source: the job's capability is the *same* live one from the original login, continuing to exist
because nothing tore it down.

This was checked against real prior art rather than assumed. systemd's `loginctl enable-linger`
solves almost exactly this problem (keep a user's session manager running after logout so pending
timers keep firing), but it is a static, explicit admin toggle: an account either lingers or it
does not, independent of whether anything is actually pending. The design here is the dynamic
version instead: a session persists *because* it has scheduled work, and becomes destroyable once it
does not, which is more minimal (nothing outlives its own reason to exist) and maps onto §16's
existing mechanism directly rather than needing a new opt-in flag and a new place to store it.
[loginctl (systemd)](https://www.freedesktop.org/software/systemd/man/latest/loginctl.html).

### Reattachment on reconnect, via a scoped lookup, not enumeration

When a user reconnects and a durable session already exists for them (kept alive by pending jobs),
the new connection needs to find and attach to *that* session rather than minting a second one
alongside it, or the design degrades into either session proliferation or an orphaned session with
no way back to it.

This cannot be built as "list all sessions and find the match." This tree already refuses that
shape on principle (milestone 126: enumeration is itself authority). It has to be a targeted lookup:
given a *proven* identity, return the one record for it, never a list. The natural home is the
credential service (milestone 56's credentialer) or a login broker built beside it (milestone 49
already describes login as "authentication produces capabilities"): a small table, identity to
durable-session capability (or nothing yet), where a successful authentication returns the existing
entry or creates and records a fresh one. Additive to what 56 already does, not a new kind of ambient
power: still "prove who you are, get back exactly your own thing," never a directory of everyone's.

### Decided: disabling a user's login credentials kills their durable session

calef, 2026-08-22: yes. Revoking credentials cascades to killing the durable session, which
cascades to everything derived from it (§40's subtree-death rule), including every scheduled job the
session was supervising. One action, one consequence, using mechanism that already exists rather
than inventing a second revocation path that has to be kept in sync with the first. Recorded as
[DECISIONS §108](../decisions/108-credential-revocation-kills-durable-session.md).

### Boot-time bring-up is re-derivation, not restoration

Capabilities do not survive a reboot; nothing in the kernel does. So bring-up at boot cannot be
"reload the session's old authority from disk," because there is no such operation. It has to be
re-derivation: boot re-establishes each durable session's authority fresh, the same way a login
would, without a live person presenting credentials at that moment.

This needs a durable, on-disk record that does not exist yet. Milestone 129's own "Still to
build" list already names this gap without solving it: *"Calendar syntax, wall-clock entries,
persistence... none started."* The runtime-registered, per-user schedule needs its own durable store
(today's `timetable.conf` is the compile-time equivalent, baked into the image), written when a job
is registered during a live session, read back at boot.

Trust at boot does not come from a fresh secret challenge, because nobody is presenting one. It
comes from the durable store itself only ever having been written by an already-authenticated
action: the user proved who they were once, at registration time, and boot only has to trust that
the store is authentic and untampered, which is what measured boot (§22, already one of milestone
49's three answered pillars) and the credential store's own existing persistence (milestone 56's
sealed store) are for. Boot-time re-derivation is a privileged, boot-only operation, in the same
shape as `root_supervisor` handing out its authority once at boot and never again, not a standing
"impersonate any user" capability left lying around afterward.

This belongs to 152, not to milestone 129's own "persistence" line. 129's item is really about
the schedule's data format (calendar syntax, wall-clock semantics); this is about the durable
session's lifecycle at its most extreme boundary, the kernel itself restarting. 129's BUGS entry
points here for the mechanism.

## What this unblocks

#387's runtime registration: §222 (who holds a user's schedule), built here.

## What was built (2026-08-24, `smb_server`'s session/connection split)

`user/src/smb_server.rs` split its accept-serve-close loop in two: the per-connection protocol
handler, rebuilt on every socket, and `DurableSession`, a budget split once before the accept loop
and never torn down by a connection. DECISIONS §16 (object revocation) kept it alive: `DESTROY` refused while it had a
live child. Its `mint_pending_job` was the primitive a registrar would hold a job's authority
against, and the SMB gate proved the refusal on aarch64 and riscv64 with stage codes
`0xE140`-`0xE146`. The SMB implementation, and this type with it, was removed on 2026-08-30; the
proof is re-homed below (2026-09-26), and the full account is in git and `notes/smb.md`.

## What was built (2026-08-24, the schedule store and the boot-time re-deriver)

The second and third BUGS items below (the on-disk schedule store, and boot-time re-derivation's own
mechanism) are closed, once [DECISIONS §122](../decisions/122-durable-schedule-store-format.md) and
[§123](../decisions/123-boot-time-rederivation-privilege.md) were ratified (option 1 and option (a)
respectively) and this lane built what they recommended, plus the manifest question neither decision
fully specified ([DECISIONS §125](../decisions/125-durable-schedule-manifest.md), PROPOSED,
provisional number, this lane's own finding).

- `crates/schedule_store` (provisional name) holds the shared names two programs agree on: the
  schedule file's own filename inside an identity's subtree (§122), the manifest's filename and
  document format (§125, this lane's own answer to "which identities"), and the render/parse
  functions for the manifest. It depends on nothing and reuses `timetable::parse` for the schedule
  document itself unchanged, exactly §122's recommendation.
- The write path: `fixtures/src/fs_test_client.rs`'s new `ROLE_SCHEDULE_SEED` (this lane's own
  demonstration writer, not a real registrar; #387 remains that) `MKDIR`s one identity's subtree,
  writes its `schedule` file through ordinary `filesystem_protocol::fs::CREATE`/`WRITE`, and records
  that identity in the manifest at the store's own root. `ROLE_SCHEDULE_VERIFY` reads both back
  through a fresh descent, independent of the re-deriver's own read, and confirms the bytes match
  exactly (the `smb_seed`/`smb_verify` shape, one level over).
- `components/src/session_reviver.rs` (provisional name; §123 itself floated this placeholder) is the
  boot-only re-deriver: granted a construction budget and the store-read capability, checked against
  the boot's measurement table before either is handed over
  (`kernel/src/user/session_reviver_service.rs`, §123's second hardening refinement), it reads the
  manifest by name (never `READDIR`, milestone 126's rule honored throughout), reads and parses each
  named identity's `schedule` file with the real `timetable::parse`, mints and tears down a synthetic
  per-identity session in `smb_server.rs`'s own `DurableSession` shape (proving a boot-derived
  session has the identical §16 lifecycle a live login's already does), then `cap_delete`s its own
  store-read capability and construction budget and proves both gone by attempting the now-forbidden
  operations and asserting they fail, `root_supervisor`'s own idiom.
- A new, dedicated process rather than a phase of `system_initializer`, the smaller fork §123 left
  open; `session_reviver.rs`'s module doc gives the reasoning. Proven on every boot with a RedoxFS
  disk, on aarch64 and riscv64, by `kernel::user::session_reviver_tests`.

What this did not build: a real registrar (built 2026-09-26, below), per-identity narrowing of the
re-deriver's `FS_EP`, a liveness watchdog (§123's addendum declines to design one), and wiring into
the real boot. The Follow-on list below tracks each.

## What was built (2026-09-26, the live-children proof re-homed on a login session)

`kernel::user::login_tests::a_login_session_with_pending_work_refuses_logout_until_the_work_is_gone`
proves the §16 property on `login`'s delegated budget, driven by `login_test_client`'s
`PENDING_WORK` (provisional, as are its three flags): with a one-page child split off, the budget's
`DESTROY` answers `NotPermitted` and the budget keeps working; once the child is gone the logout
completes with `LOGOUT`'s proofs. It runs with the rest of `login_tests` on all three ISAs, and
`session_reviver.rs`'s six comments citing the deleted type now cite it. It also found that only the
budget is held up: the caretaker region is a sibling and the logout ticket still destroys it, so a
registrar must build a job's directory inside the job's own region.

## What was built (2026-09-26, the session process and `SCHEDULE`)

To calef's rulings S1 and L2. `login_protocol::SCHEDULE` (provisional) is `LOGIN` plus "open my
schedule": `login` splits a durable budget, builds `components/src/session.rs` (provisional) from
a region of it, and `OK` announces the timetable's registration page. The session process builds the
timetable to `timetable::contract` and blocks on one endpoint that carries both the timetable's
death and every job's report. A later login for that identity is handed the same budget and page;
`login` tells a live session from a stopped one by the timetable's exit word in the page, because
`DESTROY` answers `NotPermitted` for a stale name and a busy one alike. An empty replace stops the
timetable, the session process gives its budget back and exits, and the next login retires what is
left. `kernel::user::login_tests::a_users_schedule_outlives_their_login_and_ends_when_they_empty_it`
runs that whole life on every ISA. The ruling put the request after `OK`; it is the request itself,
because `login` cannot wait for a word after `OK` without every client sending one.

## What was built (2026-09-26, suspending a user)

To calef's §108 ruling. `login_protocol::SUSPENDED_LIST` (`suspended`, provisional) sits at the
file service's root in `may-run-unvouched`'s format; the credential store stays sealed. `user
suspend <name>` and `user resume <name>` edit it at the owner's console (`swish-check` types both).
`login` refuses a listed identity `SUSPENDED` after authentication and ends its durable session;
the front-door word `SUSPEND` ends it at once. `session_reviver` skips a listed identity at boot.
`SUSPEND`, `SUSPENDED` and `APPLIED` are provisional wire items. The console holds no capability to
`login`'s front door, so on the real boot the cascade runs at the next login attempt; the real boot
has no durable sessions yet. Proven by `suspending_a_user_ends_their_schedule_and_refuses_them_until_resumed`
and `a_suspended_identity_is_not_re_derived_at_boot`, each falsified by hand on aarch64.

## Forks this lane found, for an architect

Options and reasoning are in [notes/durable-delegation.md](../../notes/durable-delegation.md).

1. Who keeps and supervises a durable session. Ruled S1 (calef, 2026-09-26); built.
2. §108 (disabling credentials kills the durable session) has no trigger. Ruled 2026-09-26:
   `user suspend` and `user resume` at the owner console, the mark kept as a list file; built.
3. A scheduled job never holds the run-unvouched capability, so the key-trust drop of §220 (signed
   builds) reaches it. Built: the timetable refuses to run holding it (milestone 129), and the session
   process never passes it.
4. The per-identity narrowing in §123 (the boot-time re-derivation privilege) belongs with the first
   real consumer, sharing `login`'s `mint`. **Status: PROPOSED.**
5. When `login` builds the session process. Ruled L2 (calef, 2026-09-26); built.
7. Boot re-derivation moves into `login`; `session_reviver` is retired. Reasoning in
   [the fork 7 appendix](../../notes/durable-delegation/boot-rederivation-in-login.md).
   **Status: PROPOSED.** Blocks real sessions at boot.
8. Which programs a scheduled job may run on the real boot, which decides the schedule archive the
   progenitor hands `login`. **Status: PROPOSED.** Blocks `SCHEDULE` on the real boot.
6. Where a scheduled job's report goes once nobody is attached. Recommended: a job holds no report
   endpoint and writes through a directory grant in its entry. **Status: PROPOSED.** Until then the
   session process receives and drops reports.

## BUGS

- ~~`smb_server` has no session/connection separation to build this against.~~ Built 2026-08-24,
  then removed with the SMB code on 2026-08-30; the proof is re-homed (2026-09-26).
- ~~The on-disk, per-user schedule store has no format, no write path, and no read-at-boot path.~~
  Built 2026-08-24; see `crates/schedule_store`'s module doc, §122 (the on-disk schedule store) and §125 (which identities have pending work).
- ~~Boot-time re-derivation's own mechanism was asserted, not designed.~~ Built 2026-08-24; see
  `components/src/session_reviver.rs`'s module doc and BUGS, and §123.
- ~~#387 (milestone 129's `--mem` grant): no scheduled job is registered against a real session.~~
  Built 2026-09-26: a user's session registers through `SCHEDULE` and the registration page.

## Follow-on

- **Outstanding.** Registration does not persist: nothing writes the identity's schedule file or
  the manifest when a document is replaced, and the boot-time re-deriver still mints synthetic
  sessions rather than opening real ones. Checked 2026-09-26.
- **Outstanding.** The real boot hands `login` no schedule archive (`crates/system_initializer`
  starts it with a zero third argument), so `SCHEDULE` there is an ordinary login. Checked
  2026-09-26.
- **Refused.** Per-login narrowing of the directory capability was deliberately not taken, because
  the adapter it applied to was deleted: the SMB implementation went on 2026-08-30, calef's call,
  after journey 2 was retired.
- **Done.** Reattachment: `login` keeps the durable session and hands it back (2026-09-26). It
  keeps one; `components/src/login.rs`'s BUGS says why.
- **Outstanding.** `components/src/session_reviver.rs` still holds one unnarrowed filesystem endpoint for
  its whole pass, which is §123's first hardening refinement and is unbuilt. Its own `BUGS` says
  so. Fork 4 above recommends building it with the first real consumer. Checked 2026-09-26.
- **Outstanding.** `LOGIN_CONSTRUCTION_PAGES` (768) cannot hold a durable budget: see fork 7's
  appendix. Found 2026-09-27.
- **Recorded.** No liveness watchdog exists for a re-deriver that hangs before deleting its
  capabilities; §123's addendum declines to design one.
- **Outstanding.** `components/src/session_reviver.rs` is spawned only under the kernel test harness by
  `kernel/src/user/session_reviver_service.rs`; `crates/system_initializer` never names it, so it
  is not in the real interactive boot. Fork 4 above says why it should stay out until a scheduler
  receives what it derives. Checked 2026-09-26.
- **Done.** The manifest question is settled: `design/decisions/125-durable-schedule-manifest.md`
  is DECIDED, ratified by calef on 2026-08-25, and its own text notes the recommended shape was
  already built rather than merely proposed.
- **Done.** The §16 live-children proof is re-homed on a real login session's budget
  (`a_login_session_with_pending_work_refuses_logout_until_the_work_is_gone`), and the six
  `session_reviver.rs` comments that cited the deleted type now cite it. 2026-09-26.
- **Done.** DECISIONS §108's cascade, as `user suspend`/`user resume` (fork 2). 2026-09-26.
- **Recorded.** Ending a durable session waits for a running job to finish, and `SUSPEND` waits
  while a session holds the terminal; `components/src/login.rs`'s BUGS has both.
- **Done.** No scheduled job holds the run-unvouched capability (fork 3). 2026-09-26.

## Index row

calef wants a scheduled job's capabilities to reflect the scheduling user's own authority
(milestone 129's #387), so the registrar is a user's session, and its authority must outlive the
connection that registered it, which DECISIONS §92 (a caretaker is supervised by the client it
serves) does not allow. Built: the on-disk schedule store (§122), boot-time re-derivation
(`session_reviver`, §123, §125), and the §16 live-children proof on a login session's budget
(2026-09-26). The session process is ruled (S1); the rest waits on proposed forks 2 to 6 and on
milestone 129's replace contract.
