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
system component is a small idea with one hard consequence: a live SMB session's authority exists
only as long as the session does (`notes/smb.md`: "a session is proven at setup and unprotected
afterwards," meaning proven state doesn't persist past the connection), but a scheduled job needs to
keep firing after the user who registered it disconnects.

Concretely: if Chris authenticates, registers `every 24h verify_backup`, and disconnects, that
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

#387's runtime-registration question (milestone 129) can be answered once 49 exists: the registrar
is a user's durable session, kept alive by §16's live-children rule, reattached on reconnect through
the credentialer's identity lookup, torn down (cascading to its scheduled jobs) when credentials are
revoked, and re-derived rather than restored at boot from a durable, measured-boot-trusted store.

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
- This lane picked a new, dedicated process over a phase of `system_initializer`, the smaller
  fork §123 left open: see `session_reviver.rs`'s own module doc for the reasoning (a new binary
  touches nothing else in the tree; growing `system_initializer::boot`, already the kernel's largest
  function, with a second privileged phase is real surgery on a component every boot depends on).
- Proven on every boot with a RedoxFS disk attached, on both aarch64 and riscv64 (no
  network/virtio dependency, so no ISA-specific wiring was needed):
  `kernel::user::session_reviver_tests::the_schedule_store_write_path_and_the_boot_time_re_deriver_agree`
  checks three properties against the re-deriver's own report (success rather than a stage-coded
  failure, the manifest's one identity actually re-derived, and the deletion proof holding), and
  `a_fresh_reader_confirms_the_store_holds_exactly_what_the_seed_wrote` is the independent witness
  that the store itself, not merely the re-deriver's reading of it, holds the right bytes.

What this does not build, on purpose. No real scheduled-job registrar against `DurableSession`
(#387/milestone 129's own question, explicitly out of this lane's scope: the write path above is a
kernel-test fixture, not a live session's real registration flow). No per-identity narrowing of the
re-deriver's own `FS_EP` (it holds one unnarrowed capability for its whole pass, the same bound
`login.rs`/`identity_provisioner.rs` already carry for the identical grant; see `session_reviver.rs`'s
own BUGS). No liveness watchdog for a re-deriver that hangs before its deletion pass runs (§123's
hardening addendum names this gap and explicitly declines to design it; this lane does not either).
Wiring `session_reviver` into a real boot (`crates/system_initializer::boot` or an interactive
`cargo xtask swish-check`) rather than the kernel test harness that spawns it here remains open, the
same "not wired into the interactive boot" bound several of this milestone's own dependencies already
carry.

## What was built (2026-09-26, the live-children proof re-homed on a login session)

`kernel::user::login_tests::a_login_session_with_pending_work_refuses_logout_until_the_work_is_gone`
proves the §16 property on `login`'s delegated budget, driven by `login_test_client`'s
`PENDING_WORK` (provisional, as are its three flags): with a one-page child split off, the budget's
`DESTROY` answers `NotPermitted` and the budget keeps working; once the child is gone the logout
completes with `LOGOUT`'s proofs. It runs with the rest of `login_tests` on all three ISAs, and
`session_reviver.rs`'s six comments citing the deleted type now cite it. It also found that only the
budget is held up: the caretaker region is a sibling and the logout ticket still destroys it, so a
registrar must build a job's directory inside the job's own region.

## Forks this lane found, for an architect

**Status: PROPOSED, 2026-09-26 (this lane).** Options and reasoning for each are in
[notes/durable-delegation.md](../../notes/durable-delegation.md).

1. Who keeps a durable session nameable after its client leaves, and who supervises its timetable.
   Ruled S1 (calef, 2026-09-26): `login` keeps each durable identity's budget and replace endpoint,
   and a per-user session process, `session` (provisional), built from the budget, supervises the
   timetable. The reattach probe is `DESTROY` then a one-page `SPLIT`, because `DESTROY` answers
   `NotPermitted` for a stale name as well as for pending work.
2. §108 (disabling credentials kills the durable session) has no trigger: no credential can be
   disabled, and a reboot would re-derive a disabled user's jobs. Blocks the §108 cascade.
3. A scheduled job never holds the run-unvouched capability, so the automatic drop in §220 (signed
   builds) of a distrusted key's programs reaches scheduled work. Binds whoever builds the registrar.
4. The per-identity narrowing in §123 (the boot-time re-derivation privilege) belongs with the first
   real consumer, sharing `login`'s `mint`; `session_reviver` stays out of the real boot until then.
5. When `login` builds the session process. Ruled L2 (calef, 2026-09-26): on a new request after
   `OK` (provisional word), so a session persists only while it has scheduled work.
6. Where a scheduled job's report goes once nobody is attached. Recommended: a job holds no report
   endpoint and writes through a directory grant in its entry. Blocks the session process itself,
   with milestone 129's replace contract.

The replace handler is milestone 129's.

## BUGS

- ~~`smb_server` has no session/connection separation to build this against.~~ Built 2026-08-24,
  then removed with the SMB code on 2026-08-30; the proof is re-homed (2026-09-26).
- ~~The on-disk, per-user schedule store has no format, no write path, and no read-at-boot path.~~
  Built 2026-08-24; see `crates/schedule_store`'s module doc, §122 (the on-disk schedule store) and §125 (which identities have pending work).
- ~~Boot-time re-derivation's own mechanism was asserted, not designed.~~ Built 2026-08-24; see
  `components/src/session_reviver.rs`'s module doc and BUGS, and §123.
- #387 (milestone 129's `--mem` grant) is still not answerable: no scheduled job is registered
  against a real session anywhere in this tree, and `ROLE_SCHEDULE_SEED` is a kernel-test fixture.
  Wiring a real registrar is the milestone's remaining piece; the forks above are what it waits on.

## Follow-on

- **Outstanding.** Wiring a real registrar remains. Its anchor is `login`'s delegated budget, whose
  pending-job property is proven; it waits on forks 5 and 6 above and on milestone 129's replace contract. Checked
  2026-09-26.
- **Refused.** Per-login narrowing of the directory capability was deliberately not taken, because
  the adapter it applied to was deleted: the SMB implementation went on 2026-08-30, calef's call,
  after journey 2 was retired.
- **Outstanding.** Reattachment on reconnect through a scoped identity lookup, the design's second
  piece, is unbuilt: neither `components/src/login.rs` nor `components/src/credentialer.rs` holds an
  identity-to-session table, and `login` deletes its copy of every budget it delegates. S1 rules
  who keeps it; fork 5 when. Checked 2026-09-26.
- **Outstanding.** `components/src/session_reviver.rs` still holds one unnarrowed filesystem endpoint for
  its whole pass, which is §123's first hardening refinement and is unbuilt. Its own `BUGS` says
  so. Fork 4 above recommends building it with the first real consumer. Checked 2026-09-26.
- **Recorded.** No liveness watchdog exists for a re-deriver that hangs before its deletion pass
  runs. `design/decisions/123-boot-time-rederivation-privilege.md`'s hardening addendum names the
  gap and declines to design it, and this lane did not either.
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
- **Outstanding.** DECISIONS §108's cascade has no trigger: no credential can be disabled, and a
  reboot would re-derive a disabled user's jobs. Fork 2 above. Checked 2026-09-26.
- **Outstanding.** A scheduled job must not hold the run-unvouched capability, or §220's key-trust
  drop does not reach it. Fork 3 above, for whoever builds the registrar. Checked 2026-09-26.

## Index row

calef wants a scheduled job's capabilities to reflect the scheduling user's own authority
(milestone 129's #387), so the registrar is a user's session, and its authority must outlive the
connection that registered it, which DECISIONS §92 (a caretaker is supervised by the client it
serves) does not allow. Built: the on-disk schedule store (§122), boot-time re-derivation
(`session_reviver`, §123, §125), and the §16 live-children proof on a login session's budget
(2026-09-26). The session process is ruled (S1); the rest waits on proposed forks 2 to 6 and on
milestone 129's replace contract.
