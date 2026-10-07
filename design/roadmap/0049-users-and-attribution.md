---
status: BUILT
raised: 2026-07-31
built: 2026-08-27
---
# 49. Users, login, and attribution: what identity is for once it stops being authority

A login service exists, proven end to end (see "What is built" below), and it
authenticates against the credential service milestone 56 already built. **It is now wired into the
real interactive boot, on both ISAs.** `crates/system_initializer::boot` builds `credentialer`,
`identity_provisioner` and `login` (plus `audit_sink`, which drains `login`'s own audit trail so its
blocking send never parks the service) from the real virtio-rng-backed entropy service DECISIONS
§120's 2026-08-26 amendment unblocked; provisions a demo identity (`operator`) with a password the
boot itself generates from that entropy service, printed once before the prompt
(`"init: login ready -- generated credentials: identity 'operator' password '...' (shown once; use
it now)"`, the shape a cloud image's generated first-boot password already takes); and hands `login`
a `WRITE | GRANT` view of the interactive terminal so a successful login receives it, single-session,
deny-cleanly (see `components/src/login.rs`'s "The terminal: single-session, deny cleanly"). `login` now
hands back all three capabilities the milestone's own text names: a directory, a budget, and a
terminal. Proven by `script/swish-check` on both ISAs (the generated-credential line above, printed
by a real boot) and by `kernel::user::login_tests` (all ten tests, including the new
`login_hands_out_the_terminal_once_and_denies_a_concurrent_second_login_until_logout`) on both ISAs.
See BUGS for what remains a named, accepted limitation rather than a blocker.

Per-identity subtree scoping, an earlier update's own piece, is resolved. `login` used to
attenuate every principal to the same fixed subtree; it now attenuates each identity to a subtree
named by the identity string itself (DECISIONS §117), created beforehand at provisioning time by
milestone 155's `identity_provisioner`.

Channel-per-client, this update's own piece, is resolved. `login`'s front door
(`REQUEST`/`RESULT`) used to be a single endpoint pair sharing one staging page across every client
this process would ever serve, for its whole life: the structural "one client at a time" limit this
file's BUGS named. It now accepts exactly one word there, [`login_proto::CONNECT`], and mints a
fresh, private request/result pair and staging page per caller before any identity or secret is ever
staged, `filesystem_proto`'s own "a fresh object per client" answer copied here. Two callers reaching
the front door together can now only contend for service order, never for each other's secret; see
`components/src/login.rs`'s own BUGS for the full design and
`kernel::user::login_tests::two_clients_connecting_together_get_independent_channels_and_neither_observes_the_others_secret`
for the proof. What remains open (interactive boot wiring, the terminal) is exactly what it was
before and is blocked on the same thing (DECISIONS §120: no interactive login needs to work before
real hardware entropy is sorted, milestone 159); this update does not move the status line, and says
so per this file's own convention for a piece that lands without changing it.

Two resource leaks that piece introduced were found and fixed before it landed, both of the same
shape and both worth reading past this milestone, because neither is specific to `login`:

- **`MemoryRegion::DESTROY` does not free the destroyer's own capability-table slot.** It tears down
  the objects retyped from a region and returns its pages, and `revoke_region` deletes every
  `PageFrame` capability naming a freed page; it touches no `Rendezvous` capability and never the
  `MemoryRegion` capability *naming the region being destroyed*. So a server that destroys a region
  per request runs out of sixteen-slot capability table while its memory budget still looks
  healthy, and the failure arrives as whatever that server says when it cannot serve. Here that was
  `login_proto::DENIED` on a correct password, on the second login after start-up, for two days.
  The kernel's `Error::OutOfMemory` collapsing "your budget is empty" and "your table is full" into
  one code (milestone 153) is what made it expensive to find: four separate memory hypotheses were
  measured and ruled out first.
- A region destroyed out of LIFO order strands its pages until its parent dies (`crates/regions`'
  `return_to_parent`, DECISIONS §16's documented half-answer). A channel is minted before the login
  it carries and destroyed after it, so a channel region carved from the same budget as that login's
  caretaker and client budget is never the top when it goes: 368 pages of holes in one suite run.
  The fix is a budget with exactly one spender, which makes the channel region always its only live
  child; the general form is that **a short-lived region wants a parent nothing long-lived is carved
  from.**

A named prerequisite for milestone 152 (durable delegation), minted 2026-08-22: a scheduled job
registered by a specific user (milestone 129's #387) needs a durable principal to be supervised by,
and login-produces-capabilities is where that principal would first exist. 152 gates on this
milestone rather than guessing at identity itself, and inherits §109's channel-shaped answer to how
attribution composes with its own per-user sessions.

In brief. Unix's uid does four different jobs at once. Three of them are already answered here,
structurally and without anyone having declared it; the fourth has no mechanism whatsoever. This
milestone writes down the first three, builds a login service that produces capabilities instead of
changing an identity field, and then decides what to do about the fourth.

Why it matters. Users and groups **are** Unix's ambient authority mechanism. A process's authority
comes from who it belongs to rather than from what it was given, which makes every program a confused
deputy by default; `setuid` is that idea in its purest form, a program running with the union of its
owner's authority and its invoker's intent, and it has been a security disaster for fifty years.
Saying "we do not have uids" is not the interesting claim. The interesting claim is that *the work a
uid does still has to get done*, and here it gets done by four different mechanisms rather than one
overloaded number.

## The starting position: the tree is already identity-free, by accident of good design

Verified rather than assumed: **no `uid` or `gid` appears anywhere in our logic.** The vendored
RedoxFS on-disk `Node` carries the fields and `create_node` inherits them from the parent, because
that is the format; nothing ever reads them for an access decision. The `std` PAL lists permissions
under Unsupported, and `set_permissions` refuses rather than pretending. So this milestone documents
and completes a position the code already holds instead of migrating to a new one.

## What each of the uid's four jobs becomes

| Unix uses uid for | Here | Status |
|---|---|---|
| **Authorization** | Capabilities. There is no check to bypass because there is no check | Built |
| **Isolation between humans** | Milestone 47's per-shell root. Two people's shells hold different directory capabilities and neither can *name* the other's files | Built (never demonstrated multi-user) |
| **Resource accounting** | The untyped budget. A user's allowance is the region they were granted; `run --mem 16` splits from it | Built |
| **Attribution** (*who did this?*) | A channel (DECISIONS §109) | Decided and built: `login` mints a fresh channel per successful login and records who established it (see BUGS on the scope of what is proven) |

Isolation is the one worth dwelling on, because it is stronger than what a uid buys. Unix isolates by
*refusing* a request that names another user's file; the name is still sayable, the check is still
code that can be wrong, and root skips it. Here no capability reaching those files exists in that
shell, so the request cannot be phrased. A check that cannot be wrong because it is not performed.

## There is no root, and that is a statable property

Milestone 22 did something Unix structurally cannot: `root_supervisor` **gives its authority away**, deleting
its untyped once the sub-servers are running. The consequence is worth stating plainly next to the
benchmarks, because it is the kind of claim a demonstrator exists to make: **there is no point after
boot at which any principal can do everything**, not as a policy or a hardening measure but because no
capability naming everything survives. Unix's root is always one `sudo` away by construction.

## Groups are a delegation pattern, not a mechanism to build

Sharing is two parties holding the same capability, or capabilities derived from a common one;
nothing needs to be added to support it. Managed sharing (revocable, narrower for some holders,
auditable) is a **caretaker**, and `fs_file_caretaker` is already that shape: a component holding a
resource and serving several clients on its own terms. So this milestone builds no group mechanism
and instead documents the two patterns, because the alternative is someone inventing a group table
later.

## Login: authentication produces capabilities

Unix login authenticates and then mutates an identity field. Here it authenticates and then **hands
over a capability set**: a root directory, a budget, a terminal. That is a better failure mode as well
as a cleaner model, since a compromised login service leaks *what it can grant* rather than the
ability to become anyone. It is the powerbox pattern with the human at one end, and it needs a real
answer to a question we have never faced: **who gets which capabilities at startup**, which used to be
a build-time fact baked into `crates/system_initializer`'s boot wiring and is, for the one login path
this milestone builds, a fact decided at run time instead (see "What is built" below).

## What is built

**`login` (provisional name), `components/src/login.rs`, and its wire contract `crates/login_proto`
(provisional).** It holds the credential service's verify endpoint (milestone 56, unmodified: neither
`cred_proto` nor `credentialer.rs` changed), the file service's root directory capability, and a
construction budget. A client presents an identity and a secret over `login_proto`; on a match, `login`
builds a fresh `fs_subtree_caretaker` (the same construction `crates/system_initializer` performs for
a directory-granted spawn) and splits a fresh budget, then delegates both to the caller over
`abi::endpoint::SEND_CAP`. Two different successful logins are therefore two different endpoint
*objects*, not two views of one shared endpoint, which is the channel-shaped attribution DECISIONS
§109 decided on rather than the badge mechanism it refused. `login` also sends one attribution record
per successful login (identity, sequence number) on its own audit endpoint, which is the "establishes
a channel and can say who it belongs to" half of §109's property, made checkable rather than merely
claimed.

Proven end to end by `kernel/src/user/login_tests.rs` (both ISAs, `login_test_client.rs`): a correct
identity and secret produce a directory capability that answers a real `READDIR` and a budget that
retypes a real page (not merely that the capabilities arrived); a wrong secret is refused and nothing
follows the refusal, which the client checks by never calling `RECV_CAP` on that path rather than by
asserting a negative; and two different identities each get an independently working channel, correctly
named in the audit trail and in the order they were established. A fourth test proves the service
survives past what used to be a hard, silent ceiling of eight logins ever (a leaked capability-table
slot per login; see BUGS), by taking the shared instance's login count to nine and checking each one's
directory and budget work.

Per-identity subtree scoping (DECISIONS §117), landed in this update. `login` now attenuates
each authenticated identity to a subtree named by the identity string itself, used directly with no
lookup table, matching exactly what milestone 155's `identity_provisioner` creates at provisioning
time. Proven by `login_scopes_each_identity_to_its_own_provisioned_subtree`: `chris` and `corinne`
each write a self-naming marker into the directory `login` delegated and confirm the old, shared
fixture subtree's own file is absent from it (proof, not assertion, that neither landed there);
`chris` then logs in a second, independent time and reads back `chris`'s own marker, not `corinne`'s,
which is the isolation property stated positively. A companion test,
`login_denies_an_authenticated_identity_with_no_provisioned_subtree`, checks the considered fold for
a real credential with no provisioned subtree: refused, indistinguishably from a wrong password (see
`components/src/login.rs`'s own BUGS for the reasoning). A third bound surfaced by this work and now
recorded rather than left implicit: an identity longer than sixteen bytes cannot get a per-identity
subtree in this slice at all, because the grant name travels in two argument words to the caretaker
(`fs_proto::grant::MAX_NAME`), narrower than `login_proto::MAX_IDENTITY`'s sixty-four; `login`
refuses rather than silently truncating.

See `components/src/login.rs`'s own BUGS for the itemized remainder (the terminal and boot integration,
plus the two bounds above; measured-boot consultation and reclamation are both resolved), summarized
in this milestone's BUGS below.

## Attribution is the actual work, and the one place Unix does something we do not

A capability says what you may do. It says nothing about who did it. Unix gets audit almost free
because the uid is present at every syscall and doubles as the answer. Measured boot (§22) establishes
*what code* is running and capabilities establish *what it can reach*, but nothing records *who
asked*, and that gap is real rather than rhetorical.

Decided (calef, 2026-08-22, DECISIONS §109): channel. A server that wants to know who is asking
gives each principal its own endpoint, established once, and logs which one a request arrived on.
This tree has already faced this exact question three times, independently, under different names
(the compositor's shared-endpoint identity, the FS server's confinement, the fault endpoint's sender
trust) and answered it the same way every time without anyone naming it as one policy: give each
principal its own channel rather than badge a shared one (seL4's mechanism, never built here). It
also composes for free with milestone 152's durable per-user sessions: once a user's session exists,
every service reached through it already has a per-user channel, which is attribution at exactly the
granularity audit wants. §109 has the full reasoning, including where the channel model's own cost
eventually bites (roughly tens of concurrently-durable sessions against `MAX_REGIONS`). This is a
general-purpose OS, not just a file server, so that headroom should be measured against cron's
uncapped, resource-bound norm rather than a consumer file-sharing throttle; §109 corrects an
earlier draft that used the wrong comparison. Raising `MAX_REGIONS` again, the same cheap move
already made once, is the expected response as durable sessions are actually built and measured.

Sequencing. After 47 (isolation is 47's per-shell root, and login hands out exactly what 47
defines; 47 landed 2026-08-22). Built across several lanes: the login service and its channel-shaped
attribution logging, per-identity subtree scoping (DECISIONS §117), the channel-per-client front
door, the real virtio-rng entropy chain under the interactive boot (DECISIONS §120), and finally the
boot-wiring and terminal work this update closes out. **Milestone 152 (durable delegation) gates on
this milestone reaching BUILT**, which it now has; that gate is clear.

## BUGS

Named here rather than only at the component, because a reader of the milestone should meet the scope
in the same place they meet the status line. Each item is also recorded where the reader meets the
feature (`components/src/login.rs`'s own BUGS, more precisely worded per item).

- Resolved. Every principal used to be attenuated to the same subtree,
  `fs_proto::fixture::tree::SUB`, with the same rights. `login` now attenuates each identity to a
  subtree named by the identity string itself, used directly (DECISIONS §117, 2026-08-23), created
  at provisioning time by milestone 155's `identity_provisioner` rather than auto-vivified at login.
  Two bounds this brought with it, named rather than left implicit: an identity longer than sixteen
  bytes (`fs_proto::grant::MAX_NAME`) cannot get a per-identity subtree in this slice at all, and an
  authenticated identity with no provisioned subtree is refused indistinguishably from a wrong
  password (a considered fold, not an oversight; see `components/src/login.rs`'s own BUGS for both).
- ~~No terminal.~~ **Resolved, 2026-08-27.** The roadmap's own recorded recommendation (the deny-
  cleanly shape, quoted in full in this entry's own prior text) is what got built, executing rather
  than re-deciding it. `login` now holds a `WRITE | GRANT` view of the real interactive terminal
  (granted by `crates/system_initializer::boot`, the same capability the shell already holds `WRITE`
  on) and hands `WRITE` on to the first successful caller; every login after that is refused
  [`login_proto::NO_TERMINAL`] (a dedicated code, not folded into `DENIED`) before its identity or
  secret is even relayed to the credential service, until [`login_proto::LOGOUT`] (a bare word on
  the front door, since it carries no secret) frees it. See `components/src/login.rs`'s own "The terminal:
  single-session, deny cleanly" for the full design and its own BUGS for what this slice does not
  build (an unauthenticated `LOGOUT`, no liveness check on an abandoned holder -- both named
  limitations, not oversights, and both scoped to what today's single-tenant boot actually needs).
  Real multiplexing (the second shape this entry used to name as the undecided fork) remains
  undecided and unbuilt, on purpose: choosing the narrow shape commits to nothing the wider one would
  later have to unwind. Proven end to end by
  `kernel::user::login_tests::login_hands_out_the_terminal_once_and_denies_a_concurrent_second_login_until_logout`
  on both ISAs.

- ~~Not wired into the interactive boot.~~ **Resolved, 2026-08-27.** `credentialer`,
  `identity_provisioner`, `login` and a new `audit_sink` (provisional name; drains `login`'s own
  `AUDIT` endpoint so its blocking send never parks the service; its own doc is now
  `components/src/login_audit_receiver.rs`, renamed 2026-09-24)
  are now built by `crates/system_initializer::boot` on both ISAs, from the real virtio-rng-backed
  entropy service DECISIONS §120's amendment already unblocked (see this entry's own prior text for
  that half's account, unchanged). Executing this entry's own three-item plan, in order:

  1. `credentialer` and `login`, wired into `boot`. Built via `build_child`, holding narrowed
     views of capabilities `boot` already has (the file service pair, a fresh construction budget
     apiece) plus a client view of the entropy service `boot` built first. **Positioned after the
     shell's own build, not after the sink adapter** where an earlier version of this lane placed it
     and found `script/swish-check` trapped in total silence: by the sink adapter, this table is
     already resting at eleven capabilities, and `credentialer`'s own six retypes on top of that is
     seventeen against sixteen usable slots. Right after `term_in` goes back and before `term_sink`/
     the undertaker's own supervision endpoint are retyped, this table rests at eight instead,
     found by bisecting the same way the entropy-ordering fault above was.
  2. A real subtree and a real credential for whoever logs in, through `identity_provisioner`
     (milestone 155), run once per boot against the generated password below. `design/roadmap/0155-*`
     is updated to match.
  3. The demo credential's password, generated rather than baked in, executing this entry's own
     recommendation: `boot` draws twelve bytes from the entropy service it just built, hex-encodes
     them, provisions the demo identity `operator` with the result through `identity_provisioner`,
     and prints it once, before the prompt: `"init: login ready -- generated credentials: identity
     'operator' password '...' (shown once; use it now)"`. No permanent secret; a later boot can
     trivially do something else.

  **Two real bugs found and fixed while wiring this, worth carrying past this milestone.**
  `credentialer.rs`'s own readiness message (`RPT_READY`) fires only *after* its provision endpoint
  is sealed, not at startup (`credentialer.rs`'s own "Two phases" doc); an earlier version of this
  wiring read it *before* provisioning, which is not "wait for the service to come up" the way the
  entropy block's own handshake is, it is a wait for a message that cannot exist yet, and the boot
  hung rather than faulted. And `login`'s own delegation of the file service's shared page asked for
  `READ | WRITE`, which `crates/system_initializer::boot` itself cannot hold (the kernel's own grant
  to init is `WRITE | GRANT` only) and which a writable mapping never needed anyway
  (`kernel::syscall::page_frame_map`'s own comment: a read/write mapping checks only `WRITE`); see
  `components/src/login.rs`'s own comment on that delegation for the full account. Both were found by
  `script/swish-check`, not reasoned to in advance.

  `kernel::user::spawn_init`'s and `riscv_shell_boot`'s own construction budget was raised,
  2048 -> 12288 pages, for the same reason `kernel::cap::CAPABILITY_TABLE_SLOTS` was raised
  16 -> 17 (that constant's own comment carries the account): four more permanent components is a
  real, measured cost, and "a one-number change here, paid in TCB size" (`kernel::cap`'s own words)
  is this tree's own established answer to a real feature needing more of a cheap resource, the same
  shape `MAX_REGIONS`/`nifefs::NAME_LEN` were each raised for once already. Neither number is tuned
  to a minimum; both carry real margin, found empirically rather than derived, and a later lane that
  wants either number tighter has real bisection work ahead of it, not a guess to correct.

  Milestone 159, a real hardware entropy source (the JH7110's TRNG, minted alongside §120), remains
  unaffected by any of this either way, exactly as §120's own "what this does not decide" already
  said.
- Resolved, 2026-08-24. `login` used to load `fs_subtree_caretaker` by name with no check at all,
  inconsistent with milestone 104's discipline (init refuses to load a program whose bytes do not
  match the archive's measurement table). Investigating "how a loader outside the boot chain joins
  that chain" found the premise false: `login` maps the identical physical archive the kernel already
  maps for `crates/system_initializer`, the same read-only way, so the kernel's boot already vouches
  for `login`'s copy exactly as much as it vouches for init's. No new trust decision was needed;
  `login`'s `_start` now reads `measured_boot::PROGRAM_MEASUREMENTS` from that same archive and calls
  the same `measured_boot::verify_in_manifest` `system_initializer::measured` already calls, once, at
  startup, before any client exists. A refusal does not crash the service (mirroring
  `system_initializer`'s own "an unvouched `fs_subtree_caretaker` costs a feature, not a boot"): every
  login is answered `login_proto::DENIED` instead, and `components/src/login.rs`'s own BUGS explains why
  that fold is not the same anti-oracle reasoning the wrong-password and no-subtree folds get (this
  check varies with nothing a caller controls, so there is nothing to probe). See
  `kernel::user::login_tests::logins_caretaker_measurement_matches_the_real_table_and_a_tampered_one_would_be_refused`.
- Resolved, 2026-08-23 (milestone/49-caretaker-teardown). A caretaker's construction memory used
  to be spent forever, with no logout that gave it back. `mint()` now delegates its own copy of the
  caretaker's construction region to the authenticated client as a fourth capability, narrowed to
  `WRITE` (a "logout ticket"), instead of dropping it once the caretaker confirms descent (the
  narrower capability-table-slot fix an earlier lane already landed for that same drop). The region
  has nothing left to `SPLIT` or `RETYPE`, so its only remaining use is `MemoryRegion::DESTROY`, and
  calling it reclaims the caretaker's TCB, address space and endpoints, with the pages returning to
  `CONSTRUCTION_UT` under §13 region ownership. The client's own budget (the third capability) needed
  no new mechanism at all: it was always delegated with `WRITE`, which is what `DESTROY` needs, so a
  full logout destroys both, **in a specific order**: `mint()` splits the region first and the budget
  second from the same `CONSTRUCTION_UT`, and `crates/regions`' own LIFO reclaim (already accepted
  elsewhere in this tree, DECISIONS §92) only returns a freed child's pages to reusable capacity when
  it is freed at the top of its parent's watermark; a client that destroys them in the wrong order
  still tears the caretaker down correctly but strands the region's pages. Caught empirically, not
  reasoned about: an earlier version of this fix's own test destroyed them in the wrong order, every
  assertion in it passed, and it silently starved a later, unrelated test in the same suite of real
  login attempts. See `components/src/login.rs`'s own BUGS ("Resolved, 2026-08-23") for the full
  design, including why this needed neither a new supervision endpoint nor overlap with milestone
  152's durable-session scope: the two candidate shapes this milestone's earlier text named (a
  principal's supervision endpoint reaching `login`, or a caretaker `DESTROY`ed by name) turned out
  to collapse into the second, once checked against `notes/hung-component.md`'s own documented gap
  for what `Untyped::DESTROY` can and cannot reclaim while a thread is still live. Proven by
  `kernel::user::login_tests::caretaker_teardown_reclaims_a_full_session_worth_of_memory`: ten
  logins against one shared, tightly-budgeted service instance, each fully logging back out before
  the next begins, which could only pass if the memory genuinely came home every time.
- **The audit endpoint proves establishment, not per-request attribution.** DECISIONS §109 describes
  both a server establishing a channel and (separately) a server logging which channel a later request
  arrived on. `login` is only the first half. No server in this tree today needs the second: every
  existing multi-client server either serves exactly one principal by construction
  (`fs_subtree_caretaker`) or is anonymous by design (the credential service). Wiring the second half
  into a real multi-tenant consumer is follow-on for whenever such a consumer exists.
- Resolved. `login`'s request and result endpoints used to be a single endpoint pair sharing one
  staging page across every client, the same structural limit `credentialer.rs` still documents for
  its own verify page. The front door now accepts exactly one word, `login_proto::CONNECT`, and mints
  a fresh, private request/result pair and staging page per caller before any identity or secret is
  staged: `filesystem_proto`'s "a fresh object per client" answer, copied here. See
  `components/src/login.rs`'s own BUGS for the full design and its cost (a channel, answered or not, is
  never reclaimed in this slice), and
  `kernel::user::login_tests::two_clients_connecting_together_get_independent_channels_and_neither_observes_the_others_secret`
  for the proof that two callers reaching the front door together can no longer observe or corrupt
  each other's secret.

## Follow-on

- **Milestone 152.** Durable delegation, which gates on this milestone reaching BUILT so that a
  scheduled job has a durable principal to be supervised by. That gate is now clear.
- **Milestone 153.** `OutOfMemory` collapsing a spent budget, a full capability table and
  `MAX_REGIONS` into one error code. This milestone is where it first cost real time: four memory
  hypotheses were measured and ruled out before the capability table was the answer.
- **Milestone 159.** A real hardware entropy source, the JH7110's TRNG. The boot wiring here runs on
  the virtio-rng backend, which does not exist on silicon.
- **Recorded.** `MemoryRegion::DESTROY` frees the region and the objects retyped from it but never
  the destroyer's own capability-table slot naming it, so a server that destroys a region per request
  runs out of table while its budget still looks healthy.
  `design/roadmap/0153-out-of-memory-causes.md` carries the account.
- **Recorded.** A region destroyed out of LIFO order strands its pages until its parent dies, which
  cost 368 pages of holes in one suite run here. The general form, that a short-lived region wants a
  parent nothing long-lived is carved from, sits with the mechanism in
  `crates/memory_regions/src/lib.rs`.
- **Recorded.** An identity longer than sixteen bytes cannot get a per-identity subtree, because the
  grant name travels in two argument words (`fs_proto::grant::MAX_NAME`) and `login_proto` allows
  sixty-four; `login` refuses rather than truncating. An authenticated identity with no provisioned
  subtree is refused indistinguishably from a wrong password, a considered fold. Both are in
  `components/src/login.rs`.
- **Recorded.** A per-client channel, answered or not, is never reclaimed in this slice; `LOGOUT` is
  unauthenticated; and nothing checks the liveness of a terminal holder that walked away.
  `components/src/login.rs` has each of the three beside the code that causes it.
- **Recorded.** The credential service still shares one verify page across every client, the
  structural limit `login`'s front door just escaped by minting a page per caller. It is written up
  in `notes/credentials.md`, along with the sealed store's inability to revoke one secret.
- **Recorded.** The construction budget (2048 to 12288 pages) and `CAPABILITY_TABLE_SLOTS` (16 to 17)
  were raised with real margin found empirically, not tuned to a minimum, so a later lane that wants
  either tighter has bisection work rather than a guess to correct.
  `design/roadmap/0049-users-and-attribution.md` records what was paid and why.
- **Refused.** The second half of DECISIONS §109, a server logging which channel a request arrived
  on. No server in this tree needs it: every multi-client server either serves exactly one principal
  by construction (`fs_subtree_caretaker`) or is anonymous by design (the credential service).
  Building it now would be a mechanism with no consumer to shape it.
  This refusal is
  milestone 480 (design/roadmap/0480-a-server-that-logs-which-channel-a-request-arrived-on.md), which
  carries it with the condition that would change it.
- **Refused.** Real terminal multiplexing. `login` hands the terminal to the first successful caller
  and refuses the rest with a dedicated code until `LOGOUT`, deliberately, because the narrow shape
  commits to nothing the wider one would later have to unwind.
  This refusal is milestone 481 (design/roadmap/0481-terminal-multiplexing.md), superseded on
  2026-10-06 (UTC) by milestone 806 (concurrent login sessions).

## Index row

three of Unix's four uses for a uid are already answered structurally; the fourth, attribution, is
a channel (DECISIONS §109). `login` is wired into the real interactive boot on both ISAs,
provisions a boot-generated demo credential, and hands back all three capabilities the milestone
names: a directory, a budget, and (single-session, deny cleanly) the terminal
