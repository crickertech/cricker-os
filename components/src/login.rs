//! **The login service: authentication produces capabilities** (milestone 49,
//! design/roadmap/49-users-and-attribution.md, DECISIONS §109).
//!
//! Unix login authenticates and then mutates a global identity field, which is uid's whole trick
//! and the thing this system refuses to have. This process authenticates a presented identity
//! against the credential service milestone 56 already built, and on success hands back **a
//! capability set** instead: a fresh directory, a fresh budget, a **logout ticket** (see "Reclaiming
//! a session" below), and, when it is available, **the terminal** (see "The terminal:
//! single-session, deny cleanly" below). It is the powerbox pattern with the human at one end,
//! answering the question milestone 49's own doc named and left open: who gets which capabilities
//! at startup, which used to be a fact baked into `crates/system_initializer` at build time and is,
//! for the one login path this program serves, a fact decided here at run time instead.
//!
//! # What "produces capabilities" means, concretely
//!
//! A successful login does not narrow a capability this process already holds and hand a copy on
//! (that would make every principal a viewer of the same underlying object, which is the shared-
//! endpoint anti-pattern DECISIONS §109 names and rejects three times over: the compositor, the FS
//! server's handle table, the fault endpoint). It **builds a fresh `fs_subtree_caretaker`**, the
//! same construction `crates/system_initializer` performs for a directory-granted spawn, out of a
//! region split off this process's own construction budget. Two different successful logins are
//! therefore two different endpoint *objects*: distinguishable, independently revocable, and each
//! nameable only by the principal that established it, which is the channel-shaped attribution
//! DECISIONS §109 decided on.
//!
//! **The same "a fresh object per principal, not a shared view" idea now starts one step earlier
//! too** (milestone 49's channel-per-client update, resolving this program's own former "one client
//! at a time" limit). [`REQUEST`]/[`RESULT`] are a front door every client is handed identically at
//! spawn, but the only thing a client may say there is [`login_protocol::CONNECT`]: "give me my own
//! channel." Login answers with a freshly minted, private request/result pair and a freshly minted
//! staging page ([`connect`]), delegated to exactly that caller. The identity and secret an actual
//! login needs are staged and read on that private pair, never on the shared front door, so two
//! clients that reach the front door close together can never see or overwrite each other's secret:
//! the only thing they could ever contend for is which of them gets served *first*, which is a wait,
//! not a hazard. See `login_protocol`'s own module docs for the two-phase exchange in full.
//!
//! # Who may run new native code (DECISIONS §219 (how the shell names an installed program to the spawner) gate D2)
//!
//! When its spawner placed the run-unvouched capability at [`RUN_UNVOUCHED`] (the progenitor does,
//! after building this process), a session gets a `WRITE` copy of it as a sixth capability,
//! announced on the `OK` reply, **only when its identity is on the owner's list**
//! ([`login_protocol::RUN_UNVOUCHED_LIST`], empty by default; DECISIONS §221 (the boot prompt is
//! the owner's console), ruling 2). It is what lets a session run bytes nobody vouched for; a
//! session built without it cannot. So which users may run new code is decided here, per session,
//! from a file the owner writes and no session can reach ([`listed`]), as calef's consequence in
//! §219 asks, and it is never something a session can pass on: no `GRANT`. No session built here
//! holds a spawn endpoint to use it with yet, so it is delivered and proven
//! (`kernel::user::login_tests`) rather than exercised.
//!
//! # Which subtree a principal gets (see BUGS for the rest)
//!
//! **Each identity is attenuated to its own subtree, named by the identity string itself, used
//! directly** (DECISIONS §117, 2026-08-23: no separate lookup table). `chris` and `corinne` land in
//! two different, independently-scoped subtrees; neither can name the other's, and neither is the
//! old shared fixture subtree earlier slices of this program used for everyone. The subtree must
//! already exist: this program never creates one (`identity_provisioner`, milestone 155, does that
//! at provisioning time, deliberately not auto-vivified here; see that decision's own reasoning for
//! why provision-time creation was chosen over creating it on a principal's first login). An
//! authenticated identity with no provisioned subtree is refused, folded into the same
//! [`login_protocol::DENIED`] a wrong password gets (see this program's BUGS on why). Per-principal
//! subtree *scoping* (which subtree a grant may name at all, as opposed to *which one this program
//! picks*) is milestone 47's already-built mechanism (`fs_subtree_caretaker`'s whole reason to
//! exist); this program only decides the name.
//!
//! # Reclaiming a session, resolved 2026-08-23
//!
//! This program's BUGS used to name two candidate shapes for giving back a caretaker's construction
//! memory and pick neither: a principal's supervision endpoint reaching this process, or a caretaker
//! `MemoryRegion::DESTROY`ed by name. Investigating both against this tree's own precedent found a third,
//! smaller than either, and it is what [`mint`] now builds.
//!
//! **The fourth delegated capability is [`mint`]'s own `region`, undropped.** A successful login
//! used to end with this process calling `cap_delete` on its own copy of the caretaker's
//! construction region the moment the caretaker confirmed descent (see the capability-table-ceiling
//! fix this BUGS section used to describe below `mint`'s own comment). That capability is not discarded
//! anymore: it is delegated to the authenticated client, narrowed to `WRITE` (the one right
//! `MemoryRegion::DESTROY` needs, per `abi::memory_region::DESTROY`'s own doc), the same "delegate, then drop
//! our own copy" pattern already used for the directory and the budget. The client now holds its own
//! **logout ticket**: a `MemoryRegion` capability with nothing left to `SPLIT` or `RETYPE` (the region's
//! whole budget was spent building the caretaker), whose only remaining use is `DESTROY`. Calling it
//! reclaims the caretaker's TCB, address space, and endpoints, and the pages come home to
//! [`CONSTRUCTION_UT`] under §13 region ownership (the region's builder, not its destroyer), exactly
//! the outcome the BUGS section asked for and none of the two originally-named options were small
//! enough to build outright.
//!
//! **Why this needed no session and no new supervision plumbing.** The candidate this process never
//! supervises its caretaker at all (`mint` calls `cap_delete` on the caretaker's own TCB the instant
//! it starts, and sets no fault endpoint), which milestone 152's own doc names as the gap DECISIONS
//! §92 left open ("this says nothing about a caretaker with no client, because none exists yet").
//! Building a supervision endpoint that reaches back into this process to ask for a specific
//! principal's teardown is exactly the durable-session machinery 152 is scoped to design in general;
//! this slice does not need it, because the caretaker's own construction region is the only thing
//! that ever needs tearing down, and its builder (this process, transiently, for the width of one
//! `mint` call) can hand the means to do that directly to the one party who should hold it, the
//! client, without keeping anything itself.
//!
//! **Why `MemoryRegion::DESTROY` actually works here, checked against §32's own documented gap rather
//! than assumed.** A supervisor's `Endpoint::REAP` only collects an *already-dead* thread (§32:
//! "it authorizes collecting a corpse, not killing"); killing a *live* one needs `MemoryRegion::DESTROY`'s
//! stronger right, and that refuses permanently against a thread `Blocked` on an endpoint outside the
//! region being destroyed (notes/hung-component.md's case (c), the open, unsolved half of the hung-
//! component taxonomy). The caretaker built by [`mint`] is never in that shape: its own client-facing
//! endpoint (`narrow_ep`, the fourth capability's sibling) is retyped directly from `region`, so the
//! caretaker's steady state (parked in `receive_cap` between requests) is case (b), "blocked on an
//! endpoint whose region the supervisor can destroy", which `notes/hung-component.md` already
//! documents as working, with collateral: destroying `region` drains `narrow_ep`'s wait queue,
//! aborts the caretaker's blocked receive, and the armed kill lands at the caretaker's next
//! scheduling. The one narrow exception is the instant the caretaker is mid-`forward` to the file
//! service (a `CALL` on `FS_EP`, which `region` does not own): a `DESTROY` attempted in that exact
//! window is refused, transiently, the same shape `crates/system_initializer::reclaim` already
//! retries for a directory grant's own caretaker. A client is expected to retry a bounded few times
//! on `NotPermitted`, the same idiom, rather than treat one refusal as final; see
//! `fixtures/src/login_test_client.rs`'s own teardown role for a worked example.
//!
//! **This is why `mint`'s failed-descent path no longer leaks either.** The same `region` this
//! function used to abandon on a refused descent (the caretaker had already `exit()`ed, so nothing
//! was running in it) is now reclaimed right there, with the same bounded retry, before this
//! function returns `None`. That removes the one case this program's own BUGS used to note as
//! unaffected by the capability table fix.
//!
//! **A full logout needs no fifth capability, because the third one already carried enough right.**
//! [`CLIENT_BUDGET_PAGES`] is delegated with `WRITE | GRANT` (every principal's own spending money),
//! and `WRITE` is the one right `MemoryRegion::DESTROY` needs. Nothing before this fix had a reason to
//! call it, so this program's BUGS never named it, but any client holding `budget` could always
//! reclaim it the same way the logout ticket reclaims `region`. `fixtures/src/login_test_client.rs`'s
//! `LOGOUT` behaviour does both, so a full logout gives back everything a session spent:
//! [`CARETAKER_REGION_PAGES`] through the fourth capability and [`CLIENT_BUDGET_PAGES`] through the
//! third, both returning to [`CONSTRUCTION_UT`].
//!
//! **The order the two are destroyed in is load-bearing, and getting it wrong does not fail loudly.**
//! `mint` splits `region` first and `budget` second, both off [`CONSTRUCTION_UT`], so `budget` sits
//! at the top of its watermark. `crates/regions`' own LIFO reclaim (the same rule §16's object
//! revocation and `job_undertaker`'s pool already live under, and the one DECISIONS §92 already named
//! for a caretaker's own region) only returns a freed child's pages to reusable capacity when it is
//! the current top; destroying `region` while `budget` is still alive still tears down the caretaker
//! correctly (`DESTROY` returns success either way) but strands `region`'s pages until
//! `CONSTRUCTION_UT` itself goes away. **This was found, not merely reasoned about**: the first
//! version of this fix's own test destroyed them in the wrong order, every one of its own assertions
//! passed, and it silently starved a later, unrelated test in the same suite of real login attempts
//! by leaving thirteen logins' worth of stranded pages behind (see
//! `kernel::user::login_tests::caretaker_teardown_reclaims_a_full_session_worth_of_memory`'s own doc
//! comment). The fix is ordering, not a capability change: destroy `budget` (the third capability)
//! before `region` (the fourth). See `crates/login_protocol`'s own module docs for the client-facing
//! version of this note, including why it holds regardless of what other clients do (nothing else is
//! ever split from `CONSTRUCTION_UT` between one login's two capabilities) but does not generalize to
//! reclaiming two different logins' memory out of the order they were minted in.
//!
//! # The terminal: single-session, deny cleanly
//!
//! Resolved 2026-08-27, executing the recommendation
//! `design/roadmap/49-users-and-attribution.md`'s own BUGS already recorded rather than deciding it
//! fresh here. Milestone 49's own text names three things a login hands back (a directory, a
//! budget, a terminal); this program used to hand back two. The reason "hand one back" was ever a
//! real design question rather than an unbuilt feature: a terminal in this system is a singleton
//! hardware-backed resource, wired once at interactive boot (`crates/system_initializer::boot`), so
//! handing it to a login-authenticated principal has to say what a *second* concurrent login gets
//! told, which naming a fourth capability slot does not answer by itself.
//!
//! **The shape built is the narrow one, matching what this boot actually is**: one interactive
//! boot, one physical terminal, one session live at a time. [`TERM_EP`] arrives from
//! `crates/system_initializer::boot` exactly like [`FS_EP`] does (a capability this process holds
//! for its whole life, never `cap_delete`d, delegated fresh to whichever session currently
//! qualifies). `_start`'s own `terminal_held: bool`, `false` at start-up, is the whole of the
//! state this needs:
//!
//! - A login is refused [`login_protocol::NO_TERMINAL`], **before** its identity and secret are even
//!   relayed to the credential service, whenever `terminal_held` is already `true`. See
//!   [`serve_login`]'s own comment for why the check runs first rather than after authentication.
//! - The first login to arrive while `terminal_held` is `false` receives [`TERM_EP`] as its fifth
//!   delegated capability (`WRITE`, the same right the interactive boot's shell already holds on
//!   it), and `terminal_held` becomes `true`.
//! - [`login_protocol::LOGOUT`], a bare word on the *front door* (not a private channel: it carries no
//!   secret, so there is nothing a shared endpoint exposes by handling it directly), sets
//!   `terminal_held` back to `false` and answers [`login_protocol::LOGGED_OUT`]. The next login to
//!   arrive after that may receive the terminal again.
//!
//! **What this does not build, named rather than assumed away.** `LOGOUT` authenticates nothing (a
//! deliberate choice for today's actual deployment, one interactive boot with no untrusted co-tenant
//! reaching the front door; see `login_protocol`'s own BUGS for the real interruption hazard a hostile
//! holder of [`REQUEST`] would pose in a different deployment). There is no liveness check on
//! whoever currently holds the terminal: a session that exits, crashes, or simply never calls
//! `LOGOUT` leaves `terminal_held` stuck `true` for the rest of this process's life, indistinguishable
//! from a session genuinely still in use, because this process has no wait-any primitive with which
//! to watch a client and keep serving new connections at the same time (the same structural bound
//! `notes/hung-component.md` names elsewhere in this tree). Recovering from an abandoned session
//! today means restarting this process. This is explicitly **not** the "real multiplexing" shape
//! (more than one live session, each with its own view, composed the way the compositor composes
//! windows): the roadmap's own BUGS entry recommends against building that now, because the
//! single-session boot this program actually serves does not need it, and choosing the narrow shape
//! here commits to nothing the wider one would later have to unwind.
//!
//! # Capability contract
//!
//! - slot [`REQUEST`]: `RECEIVE`. The front door. A client sends exactly one
//!   [`login_protocol::connect_word`] here, ever, or exactly one [`login_protocol::logout_word`], any
//!   number of times; the actual [`login_protocol::LOGIN`] request travels on the private endpoint
//!   [`connect`] delegates in answer (see "Two phases" above and `login_protocol`'s own module docs).
//!   No page is read for either front-door word.
//! - slot [`RESULT`]: `WRITE | GRANT`. [`login_protocol::CONNECTED`], followed by three delegated
//!   capabilities: a private request endpoint (`WRITE`), a private result endpoint (`READ`), and a
//!   staging page (`READ | WRITE`). This process keeps its own copies of all three (the "delegate,
//!   then keep going" pattern [`FS_PAGE_FRAME`] already uses, not the "delegate, then drop" pattern
//!   [`mint`]'s three capabilities use), because it is the one that goes on to serve the login this
//!   channel was minted for.
//! - slot [`VERIFY`]: `WRITE` on the credential service's verify endpoint (milestone 56). This
//!   process never provisions it and never could: the provision endpoint is deleted at both ends
//!   before any client of the credential service exists (`components/src/credentialer.rs`).
//! - slot [`FS_EP`]: `WRITE | GRANT` on the file service's root directory capability. What every
//!   minted caretaker attenuates, and where this process reads the owner's run-unvouched list
//!   ([`listed`]).
//! - slot [`FS_PAGE_FRAME`]: a `PageFrame`, `WRITE` (resolved, milestone 49's boot-wiring update: not
//!   `READ | WRITE` -- see [`serve_login`]'s own comment on why `WRITE` alone already produces a
//!   fully read+write mapping for every holder, and why a real boot could not have delegated `READ`
//!   here regardless). The page the file service shares with its clients. Delegated on to each
//!   authenticated principal (see the module docs on why one frame serves every hop), and mapped
//!   here at [`LIST_VA`] for [`listed`]'s one read per login.
//! - slot [`CONSTRUCTION_UT`]: `WRITE | GRANT`. Everything a connecting client's own private channel,
//!   a caretaker, and a client budget are all built from. Never given away, unlike
//!   `root_supervisor`'s: this process keeps serving logins for its whole life, so unlike a progenitor
//!   that hands its authority away once, it must keep some.
//! - slot [`AUDIT`]: `WRITE`. One [`login_protocol::ATTRIBUTED`] message per successful login, so the
//!   property DECISIONS §109 names (a server logging which channel it just established, and for
//!   whom) is checkable rather than merely claimed. See this program's BUGS on the scope of what
//!   this endpoint proves. **Must be drained by something**, or the first successful login blocks
//!   this process forever on this `send` (a plain rendezvous `SEND` queues until a receiver
//!   arrives): `crates/system_initializer::boot` wires a dedicated `login_audit_receiver` for
//!   exactly this reason at real boot, and the kernel test harness's own `Wiring::audit` is what
//!   the guest test suite drains it with.
//! - slot [`TERM_EP`]: `WRITE | GRANT` on the interactive terminal `crates/system_initializer::boot`
//!   already wires. See "The terminal: single-session, deny cleanly" above.
//! - mapped [`CRED_VA`]: the page shared with the credential service, for the relayed `VERIFY`.
//! - mapped [`login_protocol::CARETAKER_ELF_VA`]: `fs_subtree_caretaker`'s own ELF bytes, read-only,
//!   with the length in `x0`. This is the image every caretaker this process mints is built from,
//!   and it is all of the boot archive this process is given: it needs one program, not a
//!   filesystem. Zero length means the spawner had nothing vouched-for to hand over.
//! - mapped [`login_protocol::PROGRAM_MEASUREMENTS_VA`]: [`measured_boot::PROGRAM_MEASUREMENTS`]'
//!   bytes, read-only, with the length in `x1`, so the image above can be checked against the same
//!   table `crates/system_initializer` checks everything it loads against.
//! - mapped, dynamically, starting at [`CONNECT_VA_BASE`]: one page per channel [`connect`] mints,
//!   for as long as this process runs (see BUGS: never unmapped or reused in this slice).
//! - mapped [`login_protocol::SESSION_ELF_VA`] and [`login_protocol::TIMETABLE_ELF_VA`]:
//!   `session`'s and `timetable`'s images, read-only, with both lengths in `x2`
//!   ([`login_protocol::schedule_lengths`]) (milestone 152 (durable delegation)), each checked
//!   against the table above. No job's program travels with them: a scheduled job runs what the
//!   live activation generation names (Fork 8 ruled D by calef on 2026-09-27, on #1377). Zero
//!   lengths mean [`login_protocol::SCHEDULE`] is answered as a plain login and no durable session
//!   is re-derived at start-up ([`rederive`]).
//! - slot [`DURABLE_WINDOW`] ([`login_protocol::DURABLE_WINDOW_SLOT`]): `WRITE | GRANT` on one
//!   page of the file service's window [`login_protocol::DURABLE_WINDOW`], the channel a durable
//!   session's timetable reads the store through (Fork 8 D). This process badges [`FS_EP`] with the
//!   window's number for the two store caretakers it builds per durable session
//!   ([`open_schedule`]); nothing else of its own uses the window.
//! - mapped [`DURABLE_PAGE_VA`]: the durable session's registration page, read-only, while there is
//!   one.
//!
//! Name: ratified 2026-09-15 (calef, for the whole `login` family: the stem stays on `login`,
//! `login_protocol`, `login_test_client` and the kernel's `login_service` and `login_tests`). Minted
//! 2026-08-22 for milestone 49.
//!
//! **The basis is the term of art, and not the one this block used to give.** It used to say this
//! is the Unix name for the program that answers this request, which is true, and leaned on a person
//! meeting it, which is not: nothing types `login`. The kernel starts it and only other programs reach
//! it, by `CONNECT` on its front door. And it authenticates nothing itself; it relays to
//! `credentialer`, which checks the secret, and then turns an identity into a **principal** (the term
//! ratified 2026-09-14, `design/naming.md`). By milestone 63's own test a login service never hands you
//! a login, so the stem was genuinely open. calef kept it: `login` is the field's name for this role
//! whoever speaks it, a reader arriving from Unix lands in the right place, and the stem is carried by a
//! wire vocabulary two programs agree on. The argument and the refused alternatives are in
//! `design/naming/vocabulary-rulings.md`, "The `login` stem stays".
//!
//! # BUGS
//!
//! **One durable session at a time** (milestone 152), at start-up and at a login alike. Each costs
//! four capability slots (the user's budget, the session process's region, the registration page,
//! its readiness endpoint). [`DURABLE_SESSIONS`] is the fewest of three limits, and today all are
//! one. The table: `login_protocol::durable::sessions_held` of `abi::CAPABILITY_TABLE_SLOTS`,
//! 24, is one, because the durable window's page is held at rest and an ordinary login beside two
//! sessions would need 28; one session leaves no slot spare at a login's peak. The memory:
//! [`DURABLE_UT_PAGES`] holds one budget. The channel: one durable window, and two timetables
//! staging in one page would overwrite each other (Fork 8 D). A second identity asking for its
//! schedule while another's is durable gets an ordinary session, not a refusal, and the start-up
//! pass keeps the first identity in manifest order and leaves the rest to their next login. PR
//! #1360 (open on 2026-09-27) raises the table to 32 slots; the table's limit then derives to three
//! with no edit here. Raising the other two takes a window and a budget per session from
//! `crates/system_initializer` and the kernel harness alike, and a fix for what several budgets in
//! one parent do: a budget retired out of order leaves a hole in `durable_ut` until the ones above
//! it go (the LIFO rule [`CHANNEL_UT_PAGES`] explains), so a slot freed in the middle may not be
//! reusable. The slot counts are from the code, not measured;
//! `notes/durable-delegation/boot-rederivation-in-login.md` has them.
//!
//! **The durable window is shared by a session's two caretakers and its timetable**, and by
//! nothing else. That is sound for the reason window 0 is: the timetable is the only client, and
//! every request on both hops is a blocking `CALL`.
//!
//! **Start-up waits on each re-derived session's timetable.** [`rederive`] restores each stored
//! document before the front door opens, and [`Durable::restore`] waits up to [`END_WAIT_SECS`]
//! for the timetable's answer, so a slow timetable delays the first login by up to that much per
//! session kept. Nobody has measured the usual wait, in the suite or on a board.
//!
//! **`LOGIN_CONSTRUCTION_PAGES` must hold a durable budget.** Handed `session` and `timetable`,
//! `_start` splits [`OWN_UT_PAGES`], [`DURABLE_UT_PAGES`] and [`CHANNEL_UT_PAGES`] before it serves
//! anyone, and a budget short of them stops this process at `fail(2)`. `crates/system_initializer`
//! derives its figure from `login_protocol::durable::BUDGET_PAGES`, and so does the kernel harness.
//!
//! **A session process that fails half way leaves what it split off.** `open_schedule` reclaims the
//! session process's region on a failure, but anything the process split off its own budget before
//! failing stays a child of the user's budget, which then never comes down. Only a build that runs
//! out of room can do this, and the durable budget is sized so this suite's build does not.
//!
//! **`SUSPEND` does nothing while a session holds the terminal.** Reading the suspended list
//! borrows the file page that session's caretaker shares with the file service, so it waits: the
//! cascade then runs at the suspended identity's next login attempt, which ends its durable session
//! before refusing it. A suspended user's scheduled jobs can therefore run on while somebody else is
//! logged in. A page of this process's own for the file service would close it.
//!
//! **Ending a durable session waits for a running job to finish**, because the timetable is the
//! only process that can name its jobs' regions and §222 (who holds a user's schedule) has it let a job finish. A job that never
//! finishes keeps the session past [`END_WAIT_SECS`], and `SUSPEND` then answers that it ended
//! nothing. Ending one outright needs either a timetable operation that destroys its jobs or a
//! kernel way to revoke a whole region subtree (the first caveat of DECISIONS §40 (a supervisor's death
//! is its subtree's death)), and both are
//! architect's calls.
//!
//! **A durable session nobody logs back into is never retired.** Retirement happens at the next
//! login for that identity, when the exit word says the timetable has stopped; until then the
//! stopped session's region and budget stay in [`DURABLE_UT_PAGES`]'s budget.
//!
//! **This program spells measured boot's load-or-refuse decision itself, and milestone 246 moved the
//! other copy of it into a crate.** `_start` runs `measured_boot::verify_in_manifest` over the
//! caretaker blob and folds all three outcomes (absent, refused, not an ELF) into `None`, which is
//! the considered answer here and is argued at the call site. `crates/system_initializer` used to
//! carry an eighteen-line version of the same decision; it now calls `measured_boot::verdict`, whose
//! signature takes `Option<&[u8]>` and therefore already fits this program. Switching would cost a
//! `Verdict` whose `unvouched` field this program has nothing to do with, and would buy the one
//! thing 246 was about: the refusal branch tested rather than assumed. Not done here, on the record,
//! because it is a change to a boot path this lane was not gating.
//!
//! **Resolved, milestone 233 (2026-09-02): this program used to die at `_start` on every real
//! interactive boot, on both architectures.** `_start` read the boot archive to find
//! `fs_subtree_caretaker`, from `initrd_len` in `a1` and the kernel's mapping at
//! `user_mode_runtime::initrd::INITRD_VA`. That is what `kernel::user::login_service::start` handed it, and it
//! is what `crates/system_initializer` could never hand it: `supervision_protocol::build_child` maps
//! only pages the spawner holds a `PageFrame` capability for, and the archive is reserved RAM the
//! frame allocator does not own and no capability names. So the progenitor started this process with
//! `start_child(login_child, 0, 0, 0)` and no archive, `initrd_bytes` yielded a
//! zero-length slice, `nifefs::Fs::parse` refused it, and this process took `fail(1)` before serving
//! anything, while the boot went on printing `init: login ready` with a generated password.
//!
//! **The real defect was that the two spawners disagreed**, and only one of them was tested. A
//! harness that starts a program differently from the way the system starts it is not testing that
//! program, and this one had been passing for an unknown length of time. Both now lay down the same
//! two blobs (see the endowment above), and `login_protocol::CARETAKER_ELF_VA` carries the account of
//! why the fix went this way rather than by finding some way to give this process the archive: it
//! needs one program's bytes and a table to check them against, and a service that can read every
//! file in the boot image to answer a password holds authority it never exercises.
//!
//! Two things are gone with it. This process no longer refuses to start over the caretaker at all:
//! absent, unparseable and unvouched-for all become `care_elf = None` and a `DENIED` per login,
//! which is the posture `crates/system_initializer` already had toward this exact component. And
//! `init: login ready` is now `progenitor: login credentials provisioned`, which is what the first
//! process actually measured; the survival claim moved to `script/swish-check`, which fails if the kernel reported
//! killing any user thread during the run.
//!
//! **What this cost in the currency that was scarce: nothing.** Milestone 231's gauge says the
//! boot's capability-slot high-water mark is 21 of 24 before this change and 21 of 24 after it, on
//! both architectures, because `supervision_protocol`'s `fill_and_map` holds one frame capability at a
//! time and deletes it.
//!
//! **The measurement check's trust root moved and is weaker.** When this process read the initrd it
//! read the same physical archive the kernel maps for the progenitor, so the check was independent of whoever
//! spawned it. Under `crates/system_initializer` both the bytes and the table now arrive from the progenitor,
//! which has already run the identical `measured_boot::verify_in_manifest` over them, so what
//! remains is a consistency check on the hand-over rather than an independent verification. It is
//! kept because it costs one hash and catches a spawner that pairs the wrong two blobs.
//!
//! **How long the original defect had been live is unknown** and nobody bisected it. The audit trail
//! did carry which step refused (`fail`'s `0xDEAD_0000_0000_0000 | step`) and
//! `login_audit_receiver` discards it, which is that program's own recorded limitation and is part
//! of why nobody saw this.
//!
//! **Resolved, milestone 49's channel-per-client update.** [`REQUEST`] and [`RESULT`] used to be a
//! single endpoint pair carrying an actual login's identity and secret, on a single shared staging
//! page reused by every client this process ever spawned: two concurrent callers could interleave
//! their words on that one page, exactly the limit `credentialer.rs` still documents for its own
//! verify page. [`filesystem_protocol`]'s answer, a channel per client, is now copied here: the front door's
//! only legal message is [`login_protocol::CONNECT`], carrying nothing a caller did not already know, and
//! [`connect`] answers it with a freshly minted, private request/result pair and staging page,
//! delegated to exactly the caller that asked. Two clients reaching the front door together can
//! contend only for which one is served first (this process still has one thread and no wait-any
//! primitive, so [`connect`] answers exactly one caller at a time), never for each other's secret:
//! the object each receives is theirs alone from the instant it is minted, and nothing else in this
//! process, and no other client, ever holds a capability to it. Proven by
//! `kernel::user::login_tests::two_clients_connecting_together_get_independent_channels_and_neither_observes_the_others_secret`.
//!
//! **Each channel is retyped from its own dedicated region and reclaimed by destroying it, not by
//! `cap_delete`.** An earlier version of [`connect`] retyped the request/result rendezvous and the
//! staging page directly from [`CONSTRUCTION_UT`], and `_start` answered a finished channel with
//! `cap_delete` on this process's own three capabilities. That removes this process's own
//! *reference*, not the underlying kernel objects: a rendezvous retyped by `RETYPE_OBJ` lives in the
//! kernel's own global registry (`kernel::sched::MAX_RENDEZVOUS`, 512 slots, shared by every process
//! the machine is running) until the *region* it came from is destroyed, so every connect leaked two
//! of those slots, permanently, machine-wide. This suite's own tests caught it (a later, unrelated
//! test failed with "out of rendezvous points" after this program's test-suite connects had quietly
//! spent 58 of the 512 the whole machine shares): `connect` now splits a small, dedicated region per
//! channel, retypes everything from it, and `_start` destroys that region once [`serve_login`]
//! returns, which reclaims the rendezvous objects, the page frame, and this process's own
//! capability-table slots for all three in one call. The one cost this still cannot avoid: a channel
//! nobody finishes connecting (`connect` succeeds but the caller never follows up) has no second
//! party to trigger the destroy, so its region's pages are abandoned in the same way this program's
//! other unreclaimed resources are (see `CONSTRUCTION_UT`'s exhaustion, below).
//!
//! **Resolved, 2026-08-26: `MemoryRegion::DESTROY` does not free the destroyer's own capability
//! table slot, and this process leaked two slots per connect.** The symptom was
//! `kernel::user::login_tests::caretaker_teardown_reclaims_a_full_session_worth_of_memory` refusing
//! its **second** of ten back-to-back connect-login-logout cycles with [`login_protocol::DENIED`], as
//! though `chris`'s password were wrong, which it is not. The first cycle always succeeded, and an
//! earlier version of this entry recorded that later cycles succeed too; **that was wrong**, and
//! finding out cost nothing but letting the test run past its first failed assertion: cycles two
//! through nine all fail, the second inside [`mint`] and the rest earlier still, in [`connect`].
//!
//! The cause was found by instrumenting rather than by reasoning, in four steps, each narrowing the
//! previous one: which branch answers `DENIED` (`mint` returning `None`), which step of `mint`
//! (`supervision_protocol::build_child`), which step of `build_child_space` (`fill_and_map`'s own
//! `RETYPE`), and finally which half of the kernel's `memory_region_retype` refused it. That last
//! step is the one that mattered, because the syscall collapses two unrelated causes into one
//! `Error::OutOfMemory` (`kernel::memory_region`'s own BUGS says so): the region was **not**
//! exhausted, `sched::grant` had nowhere to put the capability. This process's capability table has
//! sixteen slots (`kernel::cap::CAPABILITY_TABLE_SLOTS`).
//!
//! What filled it: `_start` destroyed each served channel's region and never `cap_delete`d its own
//! `channel.result` or `channel.region`. A comment here claimed the `DESTROY` covered them, and it
//! does not and cannot. `MemoryRegion::DESTROY` tears down the objects retyped from a region and
//! returns its pages, and `revoke_region` deletes every `PageFrame` capability naming a page it just
//! freed (which is why `channel.page` needed nothing). Neither touches a `Rendezvous` capability, and
//! nothing anywhere deletes the `MemoryRegion` capability *naming the region being destroyed*: both
//! stay as live table entries, now stale, until their holder clears them. Eight of sixteen slots are
//! spent at rest here, and a login at its peak needs six more, so two leaked slots per connect is
//! exactly one login's worth of headroom: the second login after this process starts gets through
//! `build_child`'s address space and fails on the next page.
//!
//! **The fix is [`discard`]** (destroy *and* `cap_delete`), used at every site in this program that
//! stops wanting a region, plus a `cap_delete` for the channel's own result endpoint. It also closes
//! the same leak on six failure paths that had it silently, including the one `mint`'s own comment
//! used to describe as unfixable ("this process has no `DESTROY` capability on its own construction
//! budget's children today", which was never true).
//!
//! **The general fact worth carrying away, since nothing about it is specific to this program**: a
//! long-lived server that destroys a region per request runs out of *capability table slots* while
//! its memory budget still looks healthy, and the failure surfaces as whatever that server says when
//! it cannot serve. Every one of the four things ruled out before this was found (`CONSTRUCTION_UT`
//! sizing to 16384, [`OWN_UT_PAGES`] to 8192, `kernel::sched::MAX_RENDEZVOUS`,
//! `kernel::memory_region::MAX_REGIONS`) was a *memory* hypothesis, and the sixteen-slot table was
//! looked at and passed over because tightening and restoring one slot of margin changed nothing:
//! it would not, against a leak that spends two slots per request.
//!
//! **A second, unrelated cost was measured while sizing the fix, and it is fixed too.** A channel's
//! region is minted before the login it carries and destroyed after it, so a channel region split
//! from [`CONSTRUCTION_UT`] is never the LIFO top when it is destroyed (`crates/regions`'
//! `return_to_parent` only un-bumps a parent's watermark for a child freed at the top; the same rule
//! this program's module docs already name for the logout ticket's destroy order). Every connect
//! therefore stranded [`CHANNEL_REGION_PAGES`] of `CONSTRUCTION_UT` permanently: **368 pages of
//! holes** in one suite run, against 1664 pages of real residents. [`CHANNEL_UT_PAGES`] is a budget
//! with exactly one spender, so a channel region is always its only live child and always comes home
//! whole. See that constant's own doc.
//!
//! **Resolved, 2026-08-27 (executing `design/roadmap/49-users-and-attribution.md`'s own recorded
//! recommendation).** The roadmap's own text names three things a login hands back: a root
//! directory, a budget, a terminal. This program now hands back all three, in the single-session,
//! deny-cleanly shape that recommendation named: see "The terminal: single-session, deny cleanly"
//! above for the design, [`TERM_EP`] for the capability, and [`login_protocol::NO_TERMINAL`] /
//! [`login_protocol::LOGOUT`] for the wire contract. What that recommendation explicitly declined to
//! build (a real multiplexing primitive, more than one live session with its own view) remains
//! undecided and unbuilt, on purpose; see the same section for why choosing the narrow shape now
//! commits to nothing the wider one would later have to unwind.
//!
//! **Resolved, 2026-08-23 (DECISIONS §117).** Every successful login used to be attenuated to the
//! same fixed subtree, with the same rights, for every identity. It is now attenuated to a subtree
//! named by the identity string itself; see the module docs above for what that does and does not
//! cover, and the two bounds this brought with it, named honestly rather than left implicit:
//!
//! - **An identity longer than [`filesystem_protocol::grant::MAX_NAME`] (16 bytes) cannot get a per-identity
//!   subtree in this slice at all**, even though [`login_protocol::MAX_IDENTITY`] (64 bytes) would
//!   otherwise accept it. The grant name travels in two `START` argument words to the caretaker,
//!   not a frame (`filesystem_protocol::grant`'s own doc explains why: a per-file or per-subtree grant this way
//!   costs no extra page and no extra mapping), and that encoding is the 16-byte one, not
//!   `login_protocol`'s wider one. `mint` refuses (folded into [`login_protocol::DENIED`], next bullet)
//!   rather than silently truncating the name `filesystem_protocol::grant::pack_name` would otherwise produce,
//!   which would attenuate the caretaker to a *different* subtree than the one
//!   `identity_provisioner` created (a name collision hazard, not merely a usability one: two
//!   identities that agree on their first 16 bytes would silently share a subtree). Lifting this
//!   bound to `login_protocol`'s own 64 means giving the caretaker a frame for its grant instead of two
//!   argument words, which is a change to `filesystem_protocol::grant`'s contract and every caretaker built
//!   against it, not a one-line fix here.
//! - **An authenticated identity with no provisioned subtree is refused, indistinguishably from a
//!   wrong password.** `mint`'s caretaker construction reaches the same `OPENDIR`-against-a-missing-
//!   name refusal an unprovisioned identity's descent gets, and this program's existing fold (below,
//!   "an otherwise-authenticated principal") already answers it with [`login_protocol::DENIED`] rather
//!   than a distinguishable code. **This is a considered answer, not the accident of reusing the
//!   fold**: the same reasoning [`login_protocol::DENIED`]'s own doc gives for a wrong password applies
//!   just as much here (a caller must not be able to tell "your identity has no home" from "your
//!   password is wrong" by comparing outcomes across attempts, which would let a caller probe which
//!   identities are provisioned without ever presenting a right password for one). The honest cost is
//!   that an operator who forgot to run `identity_provisioner` for a real identity sees the same
//!   denial a typo would produce; the audit trail (see below) does not help here either, since it
//!   only records a *successful* login. Distinguishing the two would need a new, deliberately-weaker
//!   channel than the login result itself (an operator-facing log the login result is not), which is
//!   real work this slice does not build.
//!
//! **Not wired into the interactive boot. The device-grant half of the blocker is now built; login
//! itself, and what it needs to serve a real password, are the remaining piece** (investigated
//! 2026-08-23, milestone/49-login-boot-prompt; the grant chain built 2026-08-26 on DECISIONS §120's
//! amendment). This process is still spawned directly by the kernel's guest test harness
//! (`kernel/src/user/login_service.rs`), the same way `credentialer` is, and is not itself reachable
//! from `crates/system_initializer::boot`'s real prompt.
//!
//! **What changed 2026-08-26.** §120 reversed its own 2026-08-23 decline ("calef is that customer,
//! for a reason specific to this project's own method": a QEMU boot is reachable unattended, a real
//! board is not). `kernel::user::spawn_init` (aarch64) and `kernel::user::riscv_shell_boot`
//! (riscv64) now discover a real virtio-rng device (MMIO only; the PCIe transport
//! `kernel::user::entropy_service::start` also offers is real follow-on, not built here) and grant
//! it to the progenitor as three capabilities (`BootEndowment::virtio_rng`/`virtio_rng_irq`/`virtio_rng_dma`),
//! and the interactive boot's own QEMU invocation now attaches one (`xtask`'s `swish_check_leg` and
//! `"shell"` command both set `NIFE_RNG`, where before it was a test-leg-only flag). `crates/
//! system_initializer::boot` builds a real entropy service from that grant, at the very top of the
//! function, and proves it drew real device bytes before building anything else (`script/swish-check`
//! now reads `"progenitor: entropy service up; drew real bytes from a virtio-rng device"` on both
//! ISAs; it said `init:` until milestone 266 renamed the program).
//! **This is the harder half of the original blocker, and it required no help from this program**:
//! `credentialer.rs` and `entropy.rs` are unmodified, because the entropy service they both already
//! assumed now genuinely exists under a real boot.
//!
//! **Why entropy had to be built before the console, and that ordering is load-bearing rather than
//! tidy**: the virtio-rng trio is granted by the kernel, at spawn, so it inflates the progenitor's resting
//! capability-table baseline for the *whole* function, and the earliest peak `boot` ever reaches
//! (retyping the terminal's six capabilities, before the console is even built) was already close to
//! the wall on its own account. Building entropy anywhere after that peak, including in the
//! reasonable-looking gap the console's own three capabilities free, pushes it over and boots in
//! total silence; building it first, and releasing its three slots before the terminal plumbing ever
//! runs, restores every peak downstream to exactly what it was before this landed. Found by
//! bisection: an isolated test granted the progenitor one single harmless extra capability, unused by any
//! code, and the identical silent fault reproduced. See `crates/system_initializer::boot`'s own
//! comment on this block for the full account.
//!
//! **What is still missing, now that the device chain is real.** Reaching *this program*, and a
//! real credential, from the prompt needs three more pieces, none of them plumbing gaps in the sense
//! the device grant was:
//!
//! 1. **`credentialer` and this program, wired into `boot` the same way entropy now is** (built via
//!    `build_child`, holding narrowed views of capabilities `boot` already retypes or is granted:
//!    the file service pair, a construction budget, and the entropy service's own request endpoint,
//!    which `boot` would keep a client view of alongside the one it hands `credentialer`).
//! 2. **A real subtree and a real credential for whoever is meant to log in**, which
//!    `identity_provisioner` (milestone 155) already builds the tool for, but that tool has the
//!    identical "spawned only by the kernel's guest test harness" bound this program's own BUGS
//!    used to name (`design/roadmap/155-*`'s own BUGS, unchanged). Wiring it in is the same shape
//!    of `build_child` call as (1); it is listed separately because it raises the next point.
//! 3. **Where the demo credential's password comes from**, which is a real, undecided fork and not
//!    a wiring detail: nothing today provisions a subtree or a credential for a real boot, and a
//!    lane should not silently choose one. Two shapes were considered, not built: a password baked
//!    into the image at build time (rejected here as a recommendation, not decided against
//!    absolutely: a fixed secret shipped in a public repository is exactly the "a fact that leaves
//!    the machine" category `AGENTS.md`'s own tenet reserves for an architect, and it is also the
//!    harder
//!    one to undo); and a password the boot itself generates, from the entropy service already
//!    built here, provisioned once per boot and printed to the console before the prompt (in the
//!    shape cloud images already use for a generated first-boot password). The second is the
//!    recommendation: it needs no permanent secret, no decision about *whose* password to bake in,
//!    and it is reversible (a later boot can trivially do something else) in exactly the sense that
//!    makes it a lane's call rather than calef's under the *move fast on what can be undone* tenet.
//!    Not built here because it is new work on top of (1) and (2), not because it is undecided in
//!    the sense that would block a lane from attempting it.
//!
//! **Why (1)-(3) are not this file's own BUGS to carry alone.** They live one level up, in
//! `design/roadmap/49-users-and-attribution.md`'s own BUGS, because they are facts about the boot's
//! wiring and milestone 155's own tool, not about what this program does or does not do; this
//! program's own contract (VERIFY, `FS_EP`, `FS_PAGE`, a construction budget, AUDIT, and now what a
//! wired `boot` would need to add: a terminal, see below) is unchanged by any of them.
//!
//! **Resolved, 2026-08-24.** This process used to load `fs_subtree_caretaker` by name with no check
//! at all, inconsistent with `crates/system_initializer`'s own discipline (milestone 104: refuse a
//! program whose bytes do not match the archive's measurement table). Investigating "how a non-progenitor
//! loader joins that chain" (the open question this BUGS entry used to leave unanswered) found the
//! premise did not hold: this process maps the *same physical archive* the kernel already maps for
//! `system_initializer`, the same read-only way, at the same address (`kernel::user::spawn_init` for
//! aarch64's the progenitor, `kernel::user::login_service`'s own `start` for this process, both taking the
//! physical range from `memory::initrd_region()`), so the kernel's boot already vouches for this
//! process's copy exactly as much as it vouches for the progenitor's. There is no new trust boundary to cross:
//! `_start` now reads
//! [`measured_boot::PROGRAM_MEASUREMENTS`] out of that same archive and calls
//! [`measured_boot::verify_in_manifest`] against `fs_subtree_caretaker`'s bytes, the identical
//! function `system_initializer::measured` calls for its own six components.
//!
//! **Not folded into a boot failure.** `system_initializer` treats this exact program as optional
//! (its own doc: an unvouched `fs_subtree_caretaker` "costs `rm` and nothing else"), and this process
//! mirrors that instead of refusing to start: the check runs once, at `_start`, before any client
//! exists, and on refusal `care_elf` becomes `None` rather than a call to [`fail`]. [`mint`] then
//! returns `None` immediately for every future login (`care?`, its very first line), which its
//! caller already folds into [`login_protocol::DENIED`], the same code "the construction budget is
//! spent" and "the caretaker's descent was refused" already share.
//!
//! **This fold is not the anti-oracle reasoning the other two folded cases get, and should not be
//! read as one.** A wrong password and a missing subtree both vary with what a caller presents, so
//! folding them prevents a caller from learning something about a specific identity by comparing
//! outcomes across attempts. A failed caretaker measurement varies with *nothing* a caller controls:
//! the archive is immutable RAM fixed for the whole boot, so every identity, on every attempt, for
//! the rest of this process's life, gets the identical answer. There is nothing to probe. The honest
//! reason for the fold is narrower and more mundane: [`login_protocol`] has no separate wire code for
//! "this service's core dependency failed to verify," and DENIED's own doc already covers "the
//! service could not mint a capability set for an otherwise-authenticated principal," which this is
//! one more instance of. The cost this leaves unaddressed is operational rather than a security gap:
//! an operator whose build produced a tampered or unmeasured `fs_subtree_caretaker` sees every real
//! login denied with no signal that the *cause* is the caretaker rather than, say, a misconfigured
//! credential store, and the audit endpoint does not help (it only records a *successful* login, the
//! same limitation already named below for the no-subtree case). A deployment that wants that
//! distinguished needs an operator-facing log distinct from the login result, which this slice does
//! not build.
//!
//! Proven by
//! `kernel::user::login_tests::logins_caretaker_measurement_matches_the_real_table_and_a_tampered_one_would_be_refused`:
//! the real archive's `fs_subtree_caretaker` bytes verify against the real measurement table (so
//! `wired()`'s own instance, and every other test in that file, depends on this check passing), and
//! the identical [`measured_boot::verify_in_manifest`] call `_start` now makes refuses a tampered
//! copy and a name the table does not mention. That test cannot spawn a second login instance against
//! a deliberately corrupted archive to prove the wire-level `DENIED` end to end: the initrd is one
//! physical region the whole kernel test binary shares, set up once at boot, and `cargo xtask` always
//! packs a table that agrees with the bytes it just packed, so no test in this suite can make the real
//! archive disagree with itself. Proving the exact check `_start` performs, against the exact name and
//! table it uses, is the strongest proof available without a second, deliberately-tampered kernel
//! image, which is out of scope for this fix.
//!
//! **Resolved, 2026-08-23.** A caretaker's construction memory used to never come back: every
//! successful login spent [`CARETAKER_REGION_PAGES`] and [`CLIENT_BUDGET_PAGES`] out of
//! [`CONSTRUCTION_UT`] for the rest of this process's life, with no logout that gave the memory
//! back. `mint` now returns its own copy of the caretaker's construction region as a fourth
//! delegated capability (narrowed to `WRITE`, the one right `MemoryRegion::DESTROY` needs) instead of
//! dropping it, and the authenticated client holds it as its own logout ticket: a `MemoryRegion` with
//! nothing left to `SPLIT` or `RETYPE` (the region's whole budget already went into building the
//! caretaker), whose only remaining use is `DESTROY`. Calling it reclaims the caretaker's TCB,
//! address space and endpoints, and the pages come home to [`CONSTRUCTION_UT`] under §13 region
//! ownership (the region's builder, not its destroyer). See this program's module docs, "Reclaiming
//! a session", for the two candidate shapes this replaces (a supervision endpoint reaching this
//! process, or a caretaker `DESTROY`ed by name) and why the second, refined this way, needed neither
//! a new supervision mechanism nor overlap with milestone 152's durable-session scope.
//!
//! **The client's own budget is the other half, and needed no new capability at all**: it was
//! always delegated with `WRITE | GRANT`, and `WRITE` is what `DESTROY` needs, so a client that
//! wants to give back everything a session spent (both [`CARETAKER_REGION_PAGES`] and
//! [`CLIENT_BUDGET_PAGES`]) calls `DESTROY` on both the fourth capability and the third, **in that
//! order** (budget, then region): the module docs, "Reclaiming a session", explain why the order is
//! load-bearing rather than a preference (a `crates/regions` LIFO rule, the same one DECISIONS §92
//! already names) and record that this was caught empirically, not merely reasoned about.
//!
//! [`CONSTRUCTION_UT`] is still sized by whoever spawns this process, and running out (a client that
//! never logs out) still answers every further login with [`login_protocol::DENIED`] rather than a
//! distinguishable error, for `login_protocol`'s own stated reason (a caller must not learn "the
//! service is out of resources" by comparing outcomes across two attempts with the same identity);
//! a deployment that wants that not to happen relies on clients actually calling `DESTROY`, which
//! this program cannot compel and does not police (see "one client at a time" above: policing would
//! need to know when a client is genuinely done, which is exactly the session concept this fix
//! avoided building).
//!
//! **The capability table ceiling this shares history with is unaffected and stays fixed.** This process's own
//! capability table has sixteen slots (`kernel::cap::CAPABILITY_TABLE_SLOTS`) and eight are spent at rest;
//! `mint` used to leak one of the remaining eight per successful login by keeping `region`'s
//! capability past a confirmed descent, which left room for exactly eight logins ever before the
//! capability table itself (not `CONSTRUCTION_UT`) answered every further attempt with `DENIED`. That was
//! fixed separately, by dropping `mint`'s own copy of `region` once the caretaker confirmed descent
//! (a `cap_delete`, not a `DESTROY`). This slice's fix keeps that shape: `region` is delegated and
//! then this process's own copy is deleted, the same "delegate, then drop our own copy" pattern
//! already used for the directory and the budget, so a live login costs this process's capability table
//! nothing beyond the width of one `mint` call, regardless of how many clients are logged in at
//! once. See `kernel::user::login_tests::the_login_service_serves_past_the_old_capability_table_ceiling`.
//!
//! **This update reopened that ceiling from a different direction and closed it again**, which is
//! worth saying here rather than only under the resolved entry above: a per-connect channel is three
//! more objects and a region, and two of those four capabilities were never given back. The lesson
//! this file now states in two places is the one that generalizes: `MemoryRegion::DESTROY` frees the
//! region, never the destroyer's own table slot naming it, so every abandon site here goes through
//! [`discard`].
//!
//! **The audit endpoint proves establishment, not per-request attribution.** [`login_protocol::ATTRIBUTED`]
//! records which identity established which channel at the moment this process minted it. It does
//! not prove that a *downstream* server, later, can say which channel one of its own requests
//! arrived on; DECISIONS §109's own text describes both halves and this program is only the first.
//! No server in this tree today needs the second: `fs_subtree_caretaker` already serves exactly one
//! principal by construction (there is nothing to distinguish), and the credential service is
//! anonymous by design (DECISIONS §109 predates this and neither wants nor needs to know who is
//! asking). Wiring the second half into a real multi-tenant consumer is follow-on for whenever such
//! a consumer exists.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

use supervision_protocol::{
    ChildEndowment, Retention, build_child, memory_region_destroy, memory_region_split,
    retype_obj_from as retype_obj, retype_page_frame_from, start_child,
};
use timetable::registration;
use user_mode_runtime::{call, cap_delete, map_page_frame, receive, send, send_cap, yield_now};

/// The front door: a bare [`login_protocol::CONNECT`], `RECEIVE` (milestone 49).
const REQUEST: u64 = 0;
/// [`login_protocol::CONNECTED`], then three delegated capabilities, `WRITE | GRANT`.
const RESULT: u64 = 1;
/// The credential service's verify endpoint (milestone 56), `WRITE`.
const VERIFY: u64 = 2;
/// The file service's root directory capability, `WRITE | GRANT`.
const FS_EP: u64 = 3;
/// The file service's shared page, a `PageFrame`, `READ | WRITE`.
const FS_PAGE_FRAME: u64 = 4;
/// Everything a connecting client's own channel, a caretaker, and a client budget are all built
/// from, `WRITE | GRANT`.
const CONSTRUCTION_UT: u64 = 5;
/// One [`login_protocol::ATTRIBUTED`] message per successful login, `WRITE`.
const AUDIT: u64 = 6;
/// **The interactive terminal** (milestone 49's terminal update), `WRITE | GRANT`. The same
/// endpoint `crates/system_initializer::boot` already hands the shell, `WRITE` only: this process
/// never reads a keystroke or writes a line itself, it only ever hands a narrowed `WRITE` copy on to
/// whichever session currently holds it (see `_start`'s own `terminal_held` flag and this program's
/// module docs, "The terminal: single-session, deny cleanly"). `GRANT` is what lets [`delegate`]
/// narrow and re-delegate it at all; absent that right the capability could be held but never handed
/// on.
const TERM_EP: u64 = 7;
/// **The run-unvouched capability** (DECISIONS §219 gate D2), `WRITE | GRANT`, when the spawner
/// placed one: the endpoint the progenitor receives a presentation on. Delegated `WRITE` alone to
/// every session this process builds ([`serve_login`]), so a session can present it and cannot
/// pass it on. A named slot rather than the ninth, because a spawner that has none leaves it
/// empty and the other eight where they were; probed once at [`_start`] ([`HOLDS_RUN_UNVOUCHED`]).
const RUN_UNVOUCHED: u64 = grant_plan::spawnproto::RUN_UNVOUCHED_SLOT;
/// One page of the file service's durable window, `WRITE | GRANT` (milestone 152, Fork 8 D). See
/// the capability contract above.
const DURABLE_WINDOW: u64 = login_protocol::DURABLE_WINDOW_SLOT;
// `grant_plan` states the slot without depending on `abi`; the relation is held by each reader.
const _: () = assert!(RUN_UNVOUCHED == abi::fault::FAULT_EP_SLOT - 1);

/// Whether [`RUN_UNVOUCHED`] holds a capability. Probed at [`_start`], before this process has
/// allocated anything, which is what makes the probe sound: later, a slot this process retyped
/// into could be the one being asked about.
static HOLDS_RUN_UNVOUCHED: core::sync::atomic::AtomicBool =
    core::sync::atomic::AtomicBool::new(false);

/// **Where this process maps the file service's shared page itself, to read the owner's
/// run-unvouched list** ([`listed`]). Below [`CRED_VA`] and the per-channel window above it, which
/// is the only thing in this space that grows.
const LIST_VA: u64 = address_space_map::pair_page(0x0000_0000_00e2_0000);

/// Whether [`LIST_VA`] holds the file page. Mapped once, at [`_start`], from [`OWN_UT_PAGES`]'
/// region, before any session exists, so its page tables sit at the bottom of that watermark and
/// never between a session's carves. `false` lists nobody.
static LIST_MAPPED: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

/// The page shared with the credential service, for the relayed `VERIFY`.
const CRED_VA: u64 = address_space_map::pair_page(0x0000_0000_00e3_0000);
/// The base of a scratch VA range [`connect`] bump-allocates one page from per channel it mints.
/// Distinct from `credentialer.rs`'s own request pages (a different process, so no collision is
/// possible), but numbered in the same family so a reader who knows one contract's addresses
/// recognises the shape of the other's. Nothing before milestone 49's channel-per-client update
/// mapped anything at or past this address.
const CONNECT_VA_BASE: u64 = address_space_map::pair_page(0x0000_0000_00e4_0000);
/// One channel's whole cost: two `RETYPE_OBJ`s (request, result), one `RETYPE` (the staging page),
/// and the page tables `page_frame::MAP` needs for that page's own mapping. Three pages minimum,
/// with margin over a tight count for the same reason [`CARETAKER_REGION_PAGES`] is (a region too
/// small fails as `Err(())`, which [`connect`] can only answer with `login_protocol::DENIED`).
const CHANNEL_REGION_PAGES: u64 = 8;

/// **A budget of its own that only [`connect`] ever spends, so a served channel's pages actually
/// come home.** Four channels' worth, though only one is ever live at a time (see below).
///
/// This is a `crates/regions` LIFO consequence, measured rather than reasoned about, and it is the
/// same rule this program's module docs already name for the logout ticket's own destroy order.
/// `MemoryRegion::DESTROY` returns a child's pages to its parent's watermark **only when the child
/// sits at the top of it**; a child freed out of order leaves a hole that does not come back until
/// no child above it is live (until 2026-10-03 UTC, not until the parent itself was destroyed;
/// `RegionTable::return_to_parent` has the change). A channel is minted before the login it carries and destroyed
/// after it, so a channel region split from [`CONSTRUCTION_UT`] is *never* the top when it is
/// destroyed: the caretaker region and the client budget that login minted sit above it, and both
/// outlive it whenever the client stays logged in. Every connect therefore stranded
/// [`CHANNEL_REGION_PAGES`] of `CONSTRUCTION_UT`, permanently, for the life of this process, which
/// this suite measured directly as **368 pages of holes** in a `CONSTRUCTION_UT` whose real
/// residents accounted for 1664 (`kernel::user::login_tests::CONSTRUCTION_PAGES`' own account).
///
/// A budget with exactly one spender fixes it outright rather than sizing around it. This process
/// has one thread and serves one channel at a time ([`connect`] answers one caller, [`_start`]
/// destroys that channel before it receives the next `CONNECT`), so a channel region carved from
/// here is always this region's **only** live child, therefore always the LIFO top, therefore
/// always fully returned. Nothing else may ever be split or retyped from this region, or that
/// property quietly stops holding; that is why it is its own constant rather than a share of
/// [`OWN_UT_PAGES`], whose spender ([`mint`]'s `build_child`) allocates *during* a login, with the
/// channel still live.
const CHANNEL_UT_PAGES: u64 = 32;

/// Where a built caretaker and the file service's shared page meet. Must match
/// `components/src/fs_subtree_caretaker.rs`'s `PAGE_VA` (the same address every caretaker in this tree
/// uses, since the caretaker itself hardcodes it and this process copies its ELF, not its address).
const CARETAKER_FS_VA: u64 = address_space_map::pair_page(0x0000_0000_0060_0000);

/// This process's own scratch: page tables for [`build_child`]'s own temporary mappings (never a
/// child's). [`connect`] draws from [`CHANNEL_UT_PAGES`] instead, for the LIFO reason that
/// constant's own doc gives, so this budget's only spender is [`mint`]'s own `build_child` calls.
///
/// **Not "one build's worth," corrected.** An earlier version of this comment claimed only one
/// caretaker is ever mid-construction, so this budget never holds more than one build's worth of
/// intermediate page tables. That is wrong about `supervision_protocol::fill_and_map`'s own mechanism:
/// its scratch cursor is a single, process-wide VA allocator, so every segment and blob page of
/// every caretaker this process builds (successfully or not: `mint` calls `build_child` before it
/// knows whether the caretaker's own descent will be refused) moves it on, and each 2 MiB it first
/// reaches costs this region a page table. Since milestone 604 (provisional) the cursor wraps inside
/// `supervision_protocol::SCRATCH_WINDOW`, so the most it can ever cost is
/// `SCRATCH_TABLE_PAGES` (32); before, it grew for as long as this process ran.
///
/// **The real accumulation is one page, measured** (2026-08-26, over a whole aarch64 suite run: the
/// twenty-six caretakers this file's tests build between them spend `usage() == (1, 128)` of this
/// region). An earlier version of this comment raised it from 128 to 1024 and justified the raise by
/// asserting 128 was "closer to this suite's own real accumulation than comfortable margin should
/// ever be"; that assertion was never measured and is wrong by three orders of magnitude. It was
/// raised while chasing this program's second-login failure, whose cause turned out to be this
/// process's own sixteen-slot capability table (see BUGS), and it is back at 128, which is 128 times
/// the observed high-water rather than a tight count.
const OWN_UT_PAGES: u64 = 128;

/// One caretaker's whole construction: its address space, TCB, and stack.
/// `crates/system_initializer::DIR_JOB_REGION_PAGES` (96) covers a caretaker **and** the program
/// behind it; this process builds only the caretaker, so a smaller region should hold it, with
/// margin rather than a tight fit (a region too small fails as `Err(())` mid-login, which this
/// process can only answer with the one code `login_protocol::DENIED` already carries for "could not
/// mint", see this program's BUGS).
const CARETAKER_REGION_PAGES: u64 = 64;

/// Stack pages beyond the one `build_child` maps, matching
/// `crates/system_initializer::CARETAKER_STACK_PAGES`: measured for that program rather than
/// guessed, and this is the same program.
const CARETAKER_STACK_PAGES: u64 = 4;

/// What this process hands each authenticated principal as its own budget. Arbitrary and modest,
/// for the demonstration this slice is; a real deployment sizes it against what a session actually
/// needs, which is not yet a question this program has enough callers to answer.
const CLIENT_BUDGET_PAGES: u64 = login_protocol::durable::CLIENT_BUDGET_PAGES;

/// **The budget durable sessions are split from** (milestone 152 (durable delegation)), split once at start-up and only
/// when a schedule can be opened at all. Its own parent for the reason [`CHANNEL_UT_PAGES`] is:
/// a durable session outlives the logins around it, so carving it from [`CONSTRUCTION_UT`] would
/// leave a hole there each time one is reclaimed out of order. Room for exactly one, which is the
/// binding term of [`DURABLE_SESSIONS`] even once the table allows more (BUGS).
const DURABLE_UT_PAGES: u64 = DURABLE_BUDGET_PAGES;
/// **How many durable sessions this process keeps**: the fewer of what its capability table
/// holds beside one ordinary login (`login_protocol::durable::sessions_held`, derived from
/// `abi::CAPABILITY_TABLE_SLOTS`, so a wider table raises it with no edit here) and what
/// [`DURABLE_UT_PAGES`] has memory for, and one, because the durable window is one channel and two
/// timetables staging in one page would overwrite each other (Fork 8 D). One today on all three
/// counts; BUGS says what a 32-slot table changes.
const DURABLE_SESSIONS: usize = {
    let slots = login_protocol::durable::sessions_held(abi::CAPABILITY_TABLE_SLOTS);
    let memory = (DURABLE_UT_PAGES / DURABLE_BUDGET_PAGES) as usize;
    let fewer = if slots < memory { slots } else { memory };
    if fewer < 1 { fewer } else { 1 }
};
/// A durable session's budget: the client's own spending, plus the session process and its
/// timetable, both built from regions split off it (`login_protocol::durable::BUDGET_PAGES`).
const DURABLE_BUDGET_PAGES: u64 = login_protocol::durable::BUDGET_PAGES;
/// The session process's construction: its segments, its stack, the timetable's image copied in,
/// its tables, the registration page, and the two store caretakers
/// (`login_protocol::durable::SESSION_REGION_PAGES`).
const SESSION_REGION_PAGES: u64 = login_protocol::durable::SESSION_REGION_PAGES;
/// The session process's own budget: its timetable's region and the budget its jobs fire from
/// (`components/src/session.rs`), and the two endpoints.
const SESSION_BUDGET_PAGES: u64 = login_protocol::durable::SESSION_BUDGET_PAGES;
/// Stack pages for the session process, beyond `build_child`'s default.
const SESSION_STACK_PAGES: u64 = 8;
/// Where this process maps the first durable session's registration page, to read the timetable's
/// exit word; the k-th kept session's is [`DURABLE_PAGE_STRIDE`] times k above it. Far above
/// `CONNECT_VA_BASE`, which grows a page per connect for ever.
const DURABLE_PAGE_VA: u64 = 0x0000_0000_0300_0000;
/// The distance between two kept sessions' registration pages: 64 KiB, a page on every
/// architecture this tree builds for.
const DURABLE_PAGE_STRIDE: u64 = 0x1_0000;
/// How long [`Durable::end`] waits for a timetable to stop, in seconds of counter time: a job
/// already running finishes first, and a scheduled job is short-lived by design.
const END_WAIT_SECS: u64 = 5;

#[unsafe(no_mangle)]
pub extern "C" fn _start(caretaker_len: u64, table_len: u64, schedule_len: u64) -> ! {
    // First, before anything is allocated: see [`HOLDS_RUN_UNVOUCHED`].
    HOLDS_RUN_UNVOUCHED.store(
        user_mode_runtime::is_granted(RUN_UNVOUCHED),
        core::sync::atomic::Ordering::Relaxed,
    );
    // **Two blobs rather than the archive** (milestone 233). Whoever started this process mapped
    // `fs_subtree_caretaker`'s ELF bytes read-only at `login_protocol::CARETAKER_ELF_VA` and the
    // measurement table at `login_protocol::PROGRAM_MEASUREMENTS_VA`, with their lengths in the first
    // two argument registers. Those constants carry the whole account of why this is not a mapping
    // of the initrd any more; the short version is that the real boot could not hand over the
    // initrd at all, so this program died at `_start` on every interactive boot until milestone 233.
    //
    // SAFETY: the spawner maps `caretaker_len` bytes read-only at `CARETAKER_ELF_VA`, for the
    // lifetime of this process, before `_start` runs (`login_protocol::CARETAKER_ELF_VA`'s own
    // contract). Both spawners do: `crates/system_initializer` through `supervision_protocol`'s
    // `blobs`, and `kernel::user::login_service::start` through `map_new`. A zero length is a slice
    // of nothing rather than a read of nothing, which is the case below.
    let care_bytes = unsafe {
        core::slice::from_raw_parts(
            login_protocol::CARETAKER_ELF_VA as *const u8,
            caretaker_len as usize,
        )
    };

    // **The table `crates/system_initializer` already consults before loading anything it did not
    // build itself** (milestone 104), read the identical way: bytes that are not UTF-8 become the
    // empty table rather than a fault (`measured_boot`'s own reasoning for why that is the safe
    // direction to fail), and an empty table vouches for nothing. Checked once, here, rather than
    // per login: both blobs are read-only pages fixed for the whole life of this process, so every
    // future request would see exactly the same verdict, and there is nothing to gain by re-hashing
    // the same bytes on every request.
    //
    // **Not a reason to refuse to start**, which is the whole of milestone 233's second half.
    // `crates/system_initializer` treats this exact program as optional (its own doc: "without one,
    // a directory grant cannot be delivered, ... which costs `rm` and nothing else"), and this
    // process now mirrors that in every case rather than only some: an unvouched, unparseable or
    // absent caretaker costs this service the one thing it exists to hand out, so every login is
    // answered `DENIED` below (via `mint`) instead. See this program's BUGS for why that fold is
    // not the same anti-oracle reasoning a wrong password gets, and is the considered answer anyway.
    //
    // **This check's trust root moved with the blob and the docs have to say so.** When this
    // program read the initrd it was reading the same physical archive the kernel maps for the progenitor, so
    // the check was independent of whoever spawned this process. It is not any more: under
    // `crates/system_initializer` both the bytes and the table arrive from the progenitor, which has already
    // run the identical `verify_in_manifest` over them. What survives is a consistency check on the
    // hand-over rather than an independent verification, and it is kept because it costs one hash
    // and catches a spawner that pairs the wrong two blobs. This program's BUGS records the loss.
    //
    // SAFETY: as above, for `table_len` bytes at `PROGRAM_MEASUREMENTS_VA`.
    let table = unsafe {
        core::slice::from_raw_parts(
            login_protocol::PROGRAM_MEASUREMENTS_VA as *const u8,
            table_len as usize,
        )
    };
    let table = core::str::from_utf8(table).unwrap_or("");
    // An empty table vouches for nothing, an unparseable image is not a caretaker, and a zero-length
    // blob is a spawner saying it had nothing vouched-for to hand over. All three end here, as
    // `None`, and all three are answered the same way: see the `None` arm of `mint` and this
    // program's own docs above on why that fold is the considered answer.
    let care_elf = elf::Elf::parse(care_bytes).ok().filter(|_| {
        measured_boot::verify_in_manifest(table, "fs_subtree_caretaker", care_bytes).is_ok()
    });

    // **What a durable session is built from** (milestone 152): `session` and `timetable`, each
    // checked against the same table the caretaker is, and the caretaker itself, since each
    // session's timetable reads the store through two of them (Fork 8 D). Anything unvouched and no
    // schedule opens on this boot, which is the caretaker's own fold: `SCHEDULE` is then answered
    // as `LOGIN` is.
    let (session_len, timetable_len) = login_protocol::split_schedule_lengths(schedule_len);
    // SAFETY: the spawner maps each image's bytes read-only at its address for the life of this
    // process before `_start` runs (`login_protocol::SESSION_ELF_VA`'s contract).
    let (session_bytes, timetable_bytes) = unsafe {
        (
            core::slice::from_raw_parts(
                login_protocol::SESSION_ELF_VA as *const u8,
                session_len as usize,
            ),
            core::slice::from_raw_parts(
                login_protocol::TIMETABLE_ELF_VA as *const u8,
                timetable_len as usize,
            ),
        )
    };
    let schedule = care_elf
        .as_ref()
        .and_then(|care| vouched_schedule(session_bytes, timetable_bytes, care, table));

    let Ok(own_ut) = memory_region_split(CONSTRUCTION_UT, OWN_UT_PAGES) else {
        fail(1)
    };
    // Split once, and only when a schedule can be opened at all: see [`DURABLE_UT_PAGES`].
    let durable_ut = match schedule {
        Some(_) => memory_region_split(CONSTRUCTION_UT, DURABLE_UT_PAGES).ok(),
        None => None,
    };
    let mut durables = Durables::new();
    // Why the start-up pass below skipped what it skipped (milestone 152, 2026-09-27): one count
    // per `login_protocol::durable::RederiveSkip` reason, read back over `REDERIVE_SKIPS`.
    let mut rederive_skips = [0u32; login_protocol::durable::REDERIVE_SKIP_REASONS];
    // Split once, here, and never anywhere else: [`CHANNEL_UT_PAGES`]' own doc explains why a
    // channel's region must come from a budget nothing else spends.
    let Ok(channel_ut) = memory_region_split(CONSTRUCTION_UT, CHANNEL_UT_PAGES) else {
        fail(2)
    };
    // The owner's list is read through the file page ([`listed`]). A map that fails leaves every
    // session without the run-unvouched capability, which is the list's own default.
    LIST_MAPPED.store(
        map_page_frame(FS_PAGE_FRAME, LIST_VA, true, own_ut),
        core::sync::atomic::Ordering::Relaxed,
    );

    // **Boot re-derivation, before the front door opens** (milestone 152, Fork 7 ruled A by calef
    // on 2026-09-27; DECISIONS §123 (the boot-time re-derivation privilege) as amended). Every
    // durable session the manifest names and the owner has not suspended is opened again, by the
    // code that opens one at a login. No session is live yet, so the file page is this process's
    // alone.
    if let (Some(images), Some(ut)) = (schedule, durable_ut) {
        rederive(own_ut, images, ut, &mut durables, &mut rederive_skips);
    }

    // How many logins this process has established, in order. The audit trail's sequence number,
    // not a capacity: `CONSTRUCTION_UT` is what actually bounds how many logins this process can
    // serve (see BUGS).
    let mut seq: u64 = 0;
    // How many channels this process has minted, in order: [`connect`]'s own bump allocator for a
    // fresh scratch VA per channel (see `CONNECT_VA_BASE`'s own doc on why a VA is never reused).
    let mut connect_seq: u64 = 0;
    // **Whether [`TERM_EP`] is currently on loan to a session** (milestone 49's terminal update).
    // `false` at start-up: nobody has logged in yet, so the terminal is free. See this program's
    // module docs, "The terminal: single-session, deny cleanly", for the whole design and this
    // program's BUGS for what freeing it does and does not guarantee.
    let mut terminal_held = false;

    loop {
        let (w0, _w1, _w2) = receive(REQUEST);
        let op = login_protocol::op(w0);
        if op == login_protocol::LOGOUT {
            // **Travels on the front door itself**, unlike an actual login: it carries no secret,
            // so there is nothing a shared endpoint would expose by handling it here directly (see
            // `login_protocol`'s own BUGS on what this does and does not authenticate). Idempotent: a
            // logout that arrives when nobody holds the terminal simply finds `terminal_held`
            // already `false`.
            terminal_held = false;
            send(RESULT, login_protocol::LOGGED_OUT, 0, 0);
            continue;
        }
        if op == login_protocol::SUSPEND {
            // **The §108 (disabling credentials kills the durable session) cascade** (milestone 152, calef's ruling of 2026-09-26): reread the owner's
            // suspended list and end the durable session of anyone on it. Unauthenticated, like
            // `LOGOUT`, for the reason `login_protocol::SUSPEND` gives.
            //
            // Not while a session holds the terminal: reading the list takes the file page that
            // session's caretaker shares with the server (see [`listed`]). The cascade then runs at
            // the suspended identity's next login instead, which ends its durable session too.
            let mut ended = 0;
            for kept in &mut durables.held {
                if !terminal_held
                    && kept
                        .as_ref()
                        .is_some_and(|d| on_list(login_protocol::SUSPENDED_LIST, d.name()))
                    && let Some(d) = kept.take()
                {
                    match d.end() {
                        Ok(()) => ended += 1,
                        Err(d) => *kept = Some(d),
                    }
                }
            }
            send(RESULT, login_protocol::APPLIED, ended, 0);
            continue;
        }
        if op == login_protocol::REDERIVE_SKIPS {
            // Unauthenticated, like `SUSPEND` and `LOGOUT`: see `login_protocol::REDERIVE_SKIPS`'s
            // own doc for why this names no identity and costs nothing to keep in production.
            send(
                RESULT,
                login_protocol::SKIP_COUNTS,
                login_protocol::durable::pack_skip_counts(&rederive_skips),
                0,
            );
            continue;
        }
        if op != login_protocol::CONNECT {
            // The front door's only other legal word; see `login_protocol`'s own module docs. Not an
            // authentication outcome (no identity has been presented yet), so `MALFORMED` rather
            // than `DENIED`.
            send(RESULT, login_protocol::MALFORMED, 0, 0);
            continue;
        }
        let Some(channel) = connect(channel_ut, connect_seq) else {
            // The construction budget is spent (see BUGS); folded into `DENIED` for
            // `login_protocol::DENIED`'s own stated reason, even though no identity is in play yet:
            // this program has exactly one code for "authenticated or not, I could not serve you".
            send(RESULT, login_protocol::DENIED, 0, 0);
            continue;
        };
        connect_seq += 1;
        send(RESULT, login_protocol::CONNECTED, 0, 0);
        // Delegate narrowed copies and **keep our own for the width of this one exchange**: unlike
        // `FS_PAGE_FRAME` (shared with every future client, forever), this channel is this process's
        // for exactly as long as `serve_login` is running and no longer, so its objects are reclaimed
        // the moment it returns (below).
        delegate(RESULT, channel.request, abi::rights::WRITE);
        delegate(RESULT, channel.result, abi::rights::READ);
        delegate(RESULT, channel.page, abi::rights::READ | abi::rights::WRITE);

        serve_login(
            &channel,
            own_ut,
            care_elf.as_ref(),
            &mut seq,
            &mut terminal_held,
            &mut Schedules {
                archive: schedule,
                durable_ut,
                durables: &mut durables,
            },
        );
        // **Reclaim the whole channel by destroying the region it was built from, not by deleting
        // our own capabilities to its pieces.** An earlier version of this loop only called
        // `cap_delete` on `channel.request`/`channel.result`/`channel.page`, which removes *this
        // process's own reference* but does nothing to the underlying kernel objects: a rendezvous
        // retyped by `RETYPE_OBJ` lives in the kernel's own global rendezvous registry
        // (`kernel::sched`'s `MAX_RENDEZVOUS`, 512 slots, shared by *every* process in the machine,
        // not this one's own capability table or `CONSTRUCTION_UT`'s own page count) until the
        // *region* it was retyped from is destroyed. `cap_delete` alone left two of those slots
        // permanently spent per connect, and this suite's own tests found it: a later, unrelated
        // test failed with "out of rendezvous points" after this file's ~29 connects had quietly
        // spent 58 of the 512 the whole machine shares. `channel.region` is destroyed here instead,
        // which reclaims the request and result rendezvous and the staging page frame in one call.
        // No thread ever runs in this region (only `RETYPE_OBJ`/`RETYPE`, never a
        // `THREAD_CONTROL_BLOCK`), so `DESTROY` has nothing to wait on and cannot be transiently
        // refused, so [`reclaim`]'s bounded retry (shared with every other
        // `MemoryRegion::DESTROY` in this program) is expected to return on its first attempt here;
        // reused anyway rather than a bare call, so an assumption this comment states does not have
        // to also be a correctness dependency if it is ever wrong.
        //
        // **The two `cap_delete`s are the other half, and an earlier version of this comment was
        // wrong to say the `DESTROY` covered them.** It claimed destroying the region reclaimed
        // "this process's own capability-table slots for all three"; it does not, and cannot.
        // `MemoryRegion::DESTROY` tears down the objects inside the region and gives its pages back,
        // and `revoke_region` deletes every `PageFrame` capability naming a page it just freed
        // (which is why `channel.page` needs nothing here). Neither touches a `Rendezvous`
        // capability, and nothing anywhere deletes the `MemoryRegion` capability *naming the region
        // being destroyed*: both stay as live entries in this process's sixteen-slot table, now
        // stale, until this process clears them itself. So every served connect used to spend two of
        // those sixteen slots permanently, which is exactly two logins' worth of headroom: the
        // second login after this process started would reach `mint`, get through `build_child`'s
        // address space, and fail on the next `RETYPE` with the table full, and the caller reads
        // that as `login_protocol::DENIED` on a correct password. See this program's BUGS.
        cap_delete(channel.result);
        discard(channel.region);
    }
}

/// **Serve one login on its own private channel**, exactly the exchange every client used to run on
/// the shared front door before milestone 49's channel-per-client update: relay the presented
/// identity and secret to the credential service, and on success mint a fresh capability set. `seq`
/// is the audit trail's own counter, shared across every channel this process ever serves (not
/// `channel`'s own connect-sequence number, which is a different count with a different purpose: see
/// `CONNECT_VA_BASE`'s doc).
///
/// **Drops `channel.request` and `channel.page` itself, as early as each stops being needed, rather
/// than leaving both live for `_start` to drop after this returns.** This process's own capability
/// table has sixteen slots (`kernel::cap::CAPABILITY_TABLE_SLOTS`) and eight are spent at rest; at
/// this function's peak, [`mint`] is itself mid-construction holding up to four of its own
/// (`region`, `narrow_ep`, `ready`, and briefly `tcb`), which left only one slot of headroom if this
/// channel's three stayed live for the whole call, down from `mint`'s own four-slot margin before
/// this channel existed at all. `channel.request` is done being useful the instant its one expected
/// message has been received (below); `channel.page` is done the instant it has been read and wiped.
/// Freeing both before `mint` ever runs restores the margin `mint`'s own four slots already assumed.
fn serve_login(
    channel: &Channel,
    own_ut: u64,
    care: Option<&elf::Elf>,
    seq: &mut u64,
    terminal_held: &mut bool,
    schedules: &mut Schedules,
) {
    let (w0, _w1, _w2) = receive(channel.request);
    cap_delete(channel.request);
    // `LOGIN`, or `SCHEDULE`: log in and open this identity's schedule (milestone 152).
    let wants_schedule = login_protocol::op(w0) == login_protocol::SCHEDULE;
    // SAFETY: `connect` mapped one page read/write at `channel.va` before delegating `channel.page`
    // to the same client this request now arrives from.
    let page =
        unsafe { core::slice::from_raw_parts(channel.va as *const u8, login_protocol::PAGE) };
    let Some((identity, secret)) = login_protocol::read(page, w0) else {
        wipe_page(channel.va);
        cap_delete(channel.page);
        send(channel.result, login_protocol::MALFORMED, 0, 0);
        return;
    };
    // **The terminal check runs before anything about `identity` or `secret` is acted on**
    // (milestone 49's terminal update). This is global, caller-independent state (there is exactly
    // one physical terminal, held or not, regardless of who is asking), not a fact about any
    // identity, so checking it first costs nothing an attacker could turn into a per-identity
    // timing or outcome oracle, and it saves a round trip to the credential service on every
    // refusal while the terminal is spoken for.
    if *terminal_held {
        wipe_page(channel.va);
        cap_delete(channel.page);
        send(channel.result, login_protocol::NO_TERMINAL, 0, 0);
        return;
    }
    // Computed before the page is wiped: `identity` borrows `channel.va` and must not be read after.
    let hint = login_protocol::identity_hint(identity);
    // **Copy the identity out before it is gone.** `identity` borrows `channel.va`, and the page is
    // wiped a few lines below (`wipe_page`, right after the credential relay); `mint` needs the
    // identity's own bytes to name the subtree to attenuate to (DECISIONS §117), which happens after
    // that wipe, on success. An owned, fixed-size copy (bounded by `login_protocol::MAX_IDENTITY`,
    // which `login_protocol::read` has already checked `identity` fits within) is the only way to carry
    // it that far without reading freed/zeroed memory.
    let mut identity_buf = [0u8; login_protocol::MAX_IDENTITY];
    let identity_len = identity.len();
    identity_buf[..identity_len].copy_from_slice(identity);

    // SAFETY: the wiring mapped one page read/write at CRED_VA before this process ran, shared
    // with the credential service and with nothing else.
    let cred_page =
        unsafe { core::slice::from_raw_parts_mut(CRED_VA as *mut u8, credential_protocol::PAGE) };
    let placed = credential_protocol::place(
        cred_page,
        identity,
        secret,
        credential_protocol::verify::VERIFY,
    );
    // The presented secret has now been copied to CRED_VA (or the placement failed and never will
    // be); either way `channel.va`'s copy is done being read.
    wipe_page(channel.va);
    cap_delete(channel.page);
    let Some(cw0) = placed else {
        send(channel.result, login_protocol::MALFORMED, 0, 0);
        return;
    };
    let (cr0, _) = call(VERIFY, cw0, 0);
    credential_protocol::wipe(cred_page);

    if !credential_protocol::is_authenticated(cr0) {
        send(channel.result, login_protocol::DENIED, 0, 0);
        return;
    }
    // **Suspended** (milestone 152, the §108 ruling): checked after authentication, so only the
    // holder of the secret learns it. A durable session this identity still has ends here too, so
    // the cascade holds even if the owner's console never sent `SUSPEND`.
    if is_suspended(&identity_buf[..identity_len]) {
        if let Some(k) = schedules.durables.find(&identity_buf[..identity_len])
            && let Some(d) = schedules.durables.held[k].take()
            && let Err(d) = d.end()
        {
            schedules.durables.held[k] = Some(d);
        }
        send(channel.result, login_protocol::SUSPENDED, 0, 0);
        return;
    }

    let identity = &identity_buf[..identity_len];

    // **An identity whose session is already durable gets that session back** (milestone 152,
    // reattachment). If its timetable has stopped, the session is retired first and this login is
    // an ordinary one; see [`Durable::exited`].
    if let Some(k) = schedules.durables.find(identity)
        && schedules.durables.held[k]
            .as_ref()
            .is_some_and(Durable::exited)
        && let Some(d) = schedules.durables.held[k].take()
    {
        // The timetable stopped because its user emptied it: nothing is pending, so the next
        // start-up has nothing to bring back for them either.
        d.retire(true);
    }
    let reattach = schedules
        .durables
        .find(identity)
        .and_then(|k| schedules.durables.held[k].as_ref())
        .map(|d| (d.budget, d.page));

    // The budget: the durable session's own on a reattach, a fresh one otherwise. A session that
    // asks for its schedule gets one from [`DURABLE_UT_PAGES`]'s budget, sized for the session
    // process and its timetable as well as the client's own spending.
    //
    // **A stored schedule comes back at login** (milestone 152; calef's §108 ruling: "the stored
    // schedule resumes at the next login"): an identity with a non-empty `schedule` in its subtree
    // and no durable session gets one opened, as though it had asked, with that document in force.
    let room = schedules.durables.free();
    let can_open = reattach.is_none() && room.is_some() && schedules.archive.is_some();
    let mut stored = [0u8; login_protocol::PAGE];
    let stored_len = if can_open {
        read_stored_schedule(identity, &mut stored).unwrap_or(0)
    } else {
        0
    };
    let opening = can_open && (wants_schedule || stored_len > 0);
    let (budget, page, keep_budget) = match reattach {
        Some((budget, page)) => (budget, Some(page), true),
        None => {
            let split = match (opening, schedules.durable_ut) {
                (true, Some(ut)) => memory_region_split(ut, DURABLE_BUDGET_PAGES),
                _ => memory_region_split(CONSTRUCTION_UT, CLIENT_BUDGET_PAGES),
            };
            let Ok(budget) = split else {
                send(channel.result, login_protocol::DENIED, 0, 0);
                return;
            };
            let opened = match (opening, schedules.archive, room) {
                (true, Some(images), Some(k)) => {
                    Durable::open(own_ut, images, budget, identity, k).map(|d| (k, d))
                }
                _ => None,
            };
            match opened {
                Some((k, d)) => {
                    if stored_len > 0 {
                        // The client reads the verdict in the page, so a refused document is its
                        // to see, and the session stays.
                        d.restore(&stored[..stored_len]);
                    }
                    record_in_manifest(identity, true);
                    let page = d.page;
                    schedules.durables.held[k] = Some(d);
                    (budget, Some(page), true)
                }
                None => (budget, None, false),
            }
        }
    };

    // **The client's own caretaker, minted after any durable session is opened**, and the order is
    // for the capability table: a durable session's two store caretakers (Fork 8 D) are built while
    // this process holds as little as it can, and one ordinary login's peak comes after, as it
    // does beside a kept session. Counted in `login_protocol::durable`.
    let Some((dir_ep, region)) = mint(own_ut, care, identity) else {
        // Authenticated, and the service still could not serve it (the construction budget is
        // spent, or the caretaker's descent was refused). Answered identically to a wrong
        // secret; see login_protocol::DENIED's own doc on why that fold is deliberate rather than
        // a missed distinction. A durable session opened above stays kept, as one re-derived at
        // start-up does, for its user's next login.
        if !keep_budget {
            discard(budget);
        }
        send(channel.result, login_protocol::DENIED, 0, 0);
        return;
    };

    // Read after the session is built, so a list read that fails costs the grant and nothing else
    // (DECISIONS §221 ruling 2: only a listed identity's session gets it).
    let run_unvouched =
        HOLDS_RUN_UNVOUCHED.load(core::sync::atomic::Ordering::Relaxed) && listed(identity);
    let mut flags = 0;
    if run_unvouched {
        flags |= login_protocol::RUN_UNVOUCHED_FOLLOWS;
    }
    if page.is_some() {
        flags |= login_protocol::SCHEDULE_FOLLOWS;
    }
    send(channel.result, login_protocol::OK, flags, 0);
    delegate(channel.result, dir_ep, abi::rights::WRITE);
    // **`WRITE` alone, not `READ | WRITE`** (resolved, milestone 49 (users, login, and attribution)'s boot-wiring update): the
    // kernel's own `page_frame_map` checks only `Rights::WRITE` for a writable mapping and grants a
    // fully read+write page table entry either way; `crates/system_initializer::boot` itself holds
    // only `WRITE | GRANT` on the real file service's shared page, so a real boot could never have
    // delegated `READ` here at all (found by `script/swish-check` refusing this exact `SEND_CAP`).
    delegate(channel.result, FS_PAGE_FRAME, abi::rights::WRITE);
    delegate(
        channel.result,
        budget,
        abi::rights::WRITE | abi::rights::GRANT,
    );
    // The logout ticket: `WRITE` is the one right `MemoryRegion::DESTROY` needs (this program's own
    // module docs, "Reclaiming a session"). Not `GRANT`: a client that could delegate its own
    // logout ticket onward could hand another principal the means to end this one's session.
    delegate(channel.result, region, abi::rights::WRITE);
    // **The terminal, fifth** (milestone 49's terminal update). Reaching here already proved
    // `!*terminal_held`, so this is never refused. `WRITE` only: a login session gets to write the
    // terminal, never to hand the capability to read keystrokes on to anything it spawns.
    delegate(channel.result, TERM_EP, abi::rights::WRITE);
    // **The run-unvouched capability, sixth, and only as announced** (DECISIONS §219 gate D2;
    // `login_protocol`'s module docs). `WRITE` alone: the session may present it to the progenitor
    // and may hand it to nothing.
    if run_unvouched {
        delegate(channel.result, RUN_UNVOUCHED, abi::rights::WRITE);
    }
    // **The registration page, last, and only as announced** (milestone 152). `WRITE` alone: the
    // client writes a document into it and reads the plan back, and has no reason to lend it.
    if let Some(page) = page {
        delegate(channel.result, page, abi::rights::WRITE);
    }
    *terminal_held = true;
    cap_delete(dir_ep);
    cap_delete(region);
    // A durable session's budget stays named here, which is the whole of reattachment.
    if !keep_budget {
        cap_delete(budget);
    }
    send(AUDIT, login_protocol::ATTRIBUTED, *seq, hint);
    *seq += 1;
}

/// **The schedule side of this process's state** (milestone 152), passed to [`serve_login`] as one
/// argument rather than three.
struct Schedules<'a, 'e> {
    /// What a durable session is built from, vouched, or `None` when no schedule can be opened on
    /// this boot.
    archive: Option<ScheduleImages<'e>>,
    /// The budget durable sessions are split from: see [`DURABLE_UT_PAGES`].
    durable_ut: Option<u64>,
    /// The durable sessions this process keeps.
    durables: &'a mut Durables,
}

/// **The durable sessions this process keeps**, at most [`DURABLE_SESSIONS`] (milestone 152).
/// Slot `k` maps its registration page at `DURABLE_PAGE_VA + k * DURABLE_PAGE_STRIDE`, so a
/// session's page address is fixed by where it is kept.
struct Durables {
    held: [Option<Durable>; DURABLE_SESSIONS],
}

impl Durables {
    fn new() -> Self {
        Durables {
            held: [const { None }; DURABLE_SESSIONS],
        }
    }

    /// Where `identity`'s session is kept, if it has one.
    fn find(&self, identity: &[u8]) -> Option<usize> {
        self.held
            .iter()
            .position(|d| d.as_ref().is_some_and(|d| d.is(identity)))
    }

    /// A slot with nothing kept in it, if there is one.
    fn free(&self) -> Option<usize> {
        self.held.iter().position(Option::is_none)
    }
}

/// **Re-derive, at start-up, every durable session the manifest names** (milestone 152; Fork 7,
/// option A, calef's ruling of 2026-09-27; DECISIONS §123 as amended that day). For each identity
/// the manifest (§125) lists and the owner's suspended list does not, in manifest order, until
/// [`DURABLE_SESSIONS`] are kept: read its stored schedule (§122), split a budget off
/// `durable_ut`, build the session process ([`Durable::open`], what a login that opens a schedule
/// calls) and put the stored document in force ([`Durable::restore`], what a login after a reboot
/// calls). No credential is presented, and none is needed for what this does: the session is
/// handed to nobody until its user logs in and the credential service says yes, which is the
/// ordinary reattachment path in [`serve_login`].
///
/// Skipped, not failed: an identity with no stored schedule or an empty one (a stale manifest
/// line), one whose name is too long to keep, one already kept, and one whose budget or session
/// process cannot be built. A stored document the timetable refuses ends its session again at
/// once, and the manifest line stays, so the refusal is shown to its user at their next login,
/// where the same document is restored with a client there to read the verdict.
///
/// `skips` counts each reason (`login_protocol::durable::RederiveSkip`), so the skip above stays
/// silent to a caller but not to `REDERIVE_SKIPS`: the missing count this program's own module
/// docs used to leave was the defect, not the skip.
fn rederive(
    own_ut: u64,
    images: ScheduleImages<'_>,
    durable_ut: u64,
    durables: &mut Durables,
    skips: &mut [u32; login_protocol::durable::REDERIVE_SKIP_REASONS],
) {
    use login_protocol::durable::RederiveSkip;
    let mut record = |reason: RederiveSkip| {
        let c = &mut skips[reason as usize];
        *c = c.saturating_add(1);
    };
    use filesystem_protocol::fs;
    let mut manifest = [0u8; login_protocol::PAGE];
    let Some(n) = read_file(
        fs::ROOT,
        schedule_store::MANIFEST_FILE_NAME.as_bytes(),
        &mut manifest,
    ) else {
        return;
    };
    let Ok(text) = core::str::from_utf8(&manifest[..n]) else {
        return;
    };
    let Ok(manifest) = schedule_store::parse_manifest(text) else {
        return;
    };
    // No list, or one that cannot be read, suspends nobody: `login_protocol::SUSPENDED_LIST`'s
    // direction, the same one [`is_suspended`] takes at a login.
    let mut suspended = [0u8; login_protocol::PAGE];
    let s = read_file(
        fs::ROOT,
        login_protocol::SUSPENDED_LIST.as_bytes(),
        &mut suspended,
    )
    .unwrap_or(0);
    for identity in login_protocol::durable::to_rederive(manifest.entries(), &suspended[..s]) {
        let Some(k) = durables.free() else {
            record(RederiveSkip::TableFull);
            break;
        };
        // `Durable` keeps a name of at most `grant::MAX_NAME` bytes, the bound `mint` holds a login
        // to; the manifest's own bound is wider.
        if !filesystem_protocol::grant::fits(identity) || durables.find(identity).is_some() {
            record(RederiveSkip::Identity);
            continue;
        }
        let mut doc = [0u8; login_protocol::PAGE];
        let len = read_stored_schedule(identity, &mut doc).unwrap_or(0);
        if len == 0 {
            record(RederiveSkip::NoStoredSchedule);
            continue;
        }
        let Ok(budget) = memory_region_split(durable_ut, DURABLE_BUDGET_PAGES) else {
            record(RederiveSkip::BudgetOutOfPages);
            break;
        };
        let Some(d) = Durable::open(own_ut, images, budget, identity, k) else {
            discard(budget);
            record(RederiveSkip::SessionBuildFailed);
            continue;
        };
        if d.restore(&doc[..len]) {
            durables.held[k] = Some(d);
        } else if let Err(d) = d.end() {
            // Still running past the wait: keep it, as `SUSPEND` does, so it can be ended later.
            durables.held[k] = Some(d);
        }
    }
}

/// **A durable session, as `login` keeps it** (milestone 152, S1 of 2026-09-26): the identity it
/// belongs to, and the four capabilities that let this process hand it back and, later, take it
/// down. Four slots each, which is what bounds [`DURABLE_SESSIONS`].
struct Durable {
    identity: [u8; filesystem_protocol::grant::MAX_NAME],
    len: usize,
    /// The user's budget. The session process and its timetable are built from regions split off
    /// it, so it refuses `DESTROY` for as long as either lives (DECISIONS §16 (object revocation)).
    budget: u64,
    /// The region the session process was built from; the registration page was retyped from it.
    session: u64,
    /// The registration page, mapped at [`Durable::va`] so this process can read the timetable's
    /// exit word.
    page: u64,
    /// Where the registration page is mapped: [`DURABLE_PAGE_VA`] plus the slot's stride.
    va: u64,
    /// The session process's readiness endpoint, which it also says `STOPPED` on once it has given
    /// its budget back. Retyped from [`Durable::session`].
    ready: u64,
}

impl Durable {
    /// **Open a durable session for `identity` on `budget`**, to be kept in slot `k` of
    /// [`Durables`]: build its session process ([`open_schedule`]) with the registration page
    /// mapped at that slot's address. The one way a durable session comes to exist, at a login
    /// and at start-up ([`rederive`]) alike. `None` leaves `budget` childless and the caller's.
    fn open(
        own_ut: u64,
        images: ScheduleImages<'_>,
        budget: u64,
        identity: &[u8],
        k: usize,
    ) -> Option<Self> {
        let va = DURABLE_PAGE_VA + k as u64 * DURABLE_PAGE_STRIDE;
        let (session, page, ready) = open_schedule(own_ut, budget, images, va)?;
        let mut id = [0u8; filesystem_protocol::grant::MAX_NAME];
        id[..identity.len()].copy_from_slice(identity);
        Some(Durable {
            identity: id,
            len: identity.len(),
            budget,
            session,
            page,
            va,
            ready,
        })
    }

    fn is(&self, identity: &[u8]) -> bool {
        self.name() == identity
    }

    fn name(&self) -> &[u8] {
        &self.identity[..self.len]
    }

    /// **End this session now** (the §108 cascade): replace its document with an empty one through
    /// the page, as its user could, wait for the timetable's exit word, then [`Durable::retire`].
    /// The timetable stops firing at once and lets a job already running finish (§222's fourth
    /// sub-ruling), so "now" means within one job's run. `Err` hands the session back when the
    /// timetable has not stopped within [`END_WAIT_SECS`]; see this program's BUGS.
    fn end(self) -> Result<(), Self> {
        use core::sync::atomic::{AtomicU64, Ordering};
        let word = |off: usize| {
            // SAFETY: `open_schedule` mapped the page read/write at `self.va` for as long as this
            // session is kept, and every offset here is an aligned word inside it
            // (`timetable::registration`'s layout).
            unsafe { &*((self.va + off as u64) as *const AtomicU64) }
        };
        let ceiling = user_mode_runtime::cntfrq().saturating_mul(END_WAIT_SECS);
        let started = user_mode_runtime::now();
        let mut asked = 0u64;
        while !self.exited() {
            if user_mode_runtime::now().wrapping_sub(started) >= ceiling {
                return Err(self);
            }
            // Ask again whenever a client's own request has overtaken ours.
            let last = registration::sequence(word(registration::REQUEST).load(Ordering::Acquire));
            if last != asked {
                asked = last + 1;
                word(registration::LEN).store(0, Ordering::Relaxed);
                word(registration::REQUEST).store(
                    registration::request(registration::REPLACE, asked),
                    Ordering::Release,
                );
            }
            yield_now();
        }
        // Suspended, not emptied: the stored schedule and the manifest line stay, so `user resume`
        // brings the schedule back at the next login and the start-up pass skips it only while the
        // mark is there.
        self.retire(false);
        Ok(())
    }

    /// **Put a stored document in force** before the page is handed out: the replace its user
    /// staged last time, staged again by this process as sequence 1. Waits for the answer, up to
    /// [`END_WAIT_SECS`]; a client reads the verdict and the plan in the page either way. `true`
    /// when the timetable answered and took the document ([`registration::STATUS_REPLACED`]).
    fn restore(&self, doc: &[u8]) -> bool {
        use core::sync::atomic::{AtomicU64, Ordering};
        let n = doc.len().min(registration::BODY_MAX);
        // SAFETY: as in `end`: the page is mapped read/write at `self.va`, and nothing else
        // writes it until it is handed out after this returns.
        let page = unsafe {
            core::slice::from_raw_parts_mut(self.va as *mut u8, registration::PAGE_BYTES)
        };
        page[registration::BODY..registration::BODY + n].copy_from_slice(&doc[..n]);
        let word = |off: usize| {
            // SAFETY: an aligned word inside the page above.
            unsafe { &*((self.va + off as u64) as *const AtomicU64) }
        };
        word(registration::LEN).store(n as u64, Ordering::Relaxed);
        word(registration::REQUEST).store(
            registration::request(registration::REPLACE, 1),
            Ordering::Release,
        );
        let ceiling = user_mode_runtime::cntfrq().saturating_mul(END_WAIT_SECS);
        let started = user_mode_runtime::now();
        while word(registration::REPLY).load(Ordering::Acquire) != 1 {
            if user_mode_runtime::now().wrapping_sub(started) >= ceiling {
                return false;
            }
            yield_now();
        }
        word(registration::STATUS).load(Ordering::Relaxed) == registration::STATUS_REPLACED
    }

    /// **Whether the timetable has stopped**, read from the exit word it writes into the page just
    /// before it exits (`timetable::registration::EXIT`). This is the liveness test reattachment
    /// needs, and it is why the page lives in [`Durable::session`] rather than in the region the
    /// session process gives back: a `DESTROY` probe cannot tell a stale budget from a busy one,
    /// because the kernel answers both `NotPermitted` (`notes/durable-delegation.md`, question 1).
    fn exited(&self) -> bool {
        // SAFETY: `open_schedule` mapped this page at `self.va`, and it stays mapped until
        // `retire` reclaims the region it lives in. The timetable writes the word; a volatile read
        // sees its latest store.
        let word = unsafe {
            core::ptr::read_volatile((self.va + registration::EXIT as u64) as *const u64)
        };
        word & registration::EXITED != 0
    }

    /// **Take a stopped session down.** The session process destroyed its own timetable budget
    /// before exiting, so what is left is the region it was built from, then the user's budget,
    /// which is childless once that region is gone. Both come home to [`DURABLE_UT_PAGES`]'s budget.
    fn retire(self, unrecord: bool) {
        // Wait for the session process to finish its own teardown: reclaiming it earlier would kill
        // it between its two destroys and strand a region under the user's budget for good. It is
        // already blocked sending this word by the time anything calls `retire` after a clean stop.
        receive(self.ready);
        cap_delete(self.ready);
        discard(self.session);
        cap_delete(self.page);
        discard(self.budget);
        if unrecord {
            record_in_manifest(self.name(), false);
        }
    }
}

/// **What a durable session is built from**: `session`'s and `timetable`'s images, both vouched
/// for, and the caretaker image the store caretakers are built from (milestone 152, Fork 8 D).
#[derive(Clone, Copy)]
struct ScheduleImages<'e> {
    session: &'static [u8],
    timetable: &'static [u8],
    care: &'e elf::Elf<'static>,
}

/// **Check `session` and `timetable` against the measurement table**, and answer them back with
/// `care` only if both pass.
fn vouched_schedule<'e>(
    session: &'static [u8],
    timetable: &'static [u8],
    care: &'e elf::Elf<'static>,
    table: &str,
) -> Option<ScheduleImages<'e>> {
    if session.is_empty() || timetable.is_empty() {
        return None;
    }
    measured_boot::verify_in_manifest(table, "session", session).ok()?;
    measured_boot::verify_in_manifest(table, "timetable", timetable).ok()?;
    Some(ScheduleImages {
        session,
        timetable,
        care,
    })
}

/// **Build a user's session process** out of `budget` (milestone 152, S1 and L2 of 2026-09-26):
/// split its construction region and its own budget off the user's, retype the registration page
/// from the first, start it, and wait for the one word it answers with. `Some((region, page))`
/// once it says its timetable is running, with the page mapped at `va`.
///
/// A failure leaves the user with an ordinary session, which is what `SCHEDULE` degrades to.
fn open_schedule(
    own_ut: u64,
    budget: u64,
    images: ScheduleImages<'_>,
    va: u64,
) -> Option<(u64, u64, u64)> {
    let elf = elf::Elf::parse(images.session).ok()?;
    let timetable = images.timetable;
    let session = memory_region_split(budget, SESSION_REGION_PAGES).ok()?;
    // **The store, read-only, for the timetable** (Fork 8 D): a caretaker each for `activation/`
    // and `packages/`, built in the session process's own region so reclaiming it takes them too,
    // on the durable window's channel. First, before anything else is held, because building them
    // is where this function's use of the capability table peaks (`login_protocol::durable`).
    let Some((activation, packages)) = store_caretakers(own_ut, session, images.care) else {
        discard(session);
        return None;
    };
    let fail = |held: &[u64]| {
        for &c in held {
            cap_delete(c);
        }
        discard(session);
    };
    let Ok(its_budget) = memory_region_split(budget, SESSION_BUDGET_PAGES) else {
        fail(&[activation, packages]);
        return None;
    };
    let (Ok(page), Ok(ready)) = (
        retype_page_frame_from(session),
        retype_obj(session, abi::objtype::RENDEZVOUS),
    ) else {
        discard(its_budget);
        fail(&[activation, packages]);
        return None;
    };
    let built = build_child(
        own_ut,
        session,
        &elf,
        &ChildEndowment {
            caps: &[
                (ready, abi::rights::WRITE),
                // `GRANT` too: a region split off this one inherits its rights, and the session
                // process hands its timetable a split of it, which `CAP_INSERT` refuses without
                // `GRANT`.
                (its_budget, abi::rights::WRITE | abi::rights::GRANT),
                (page, abi::rights::WRITE),
                // `GRANT` on the two endpoints the session process only hands on; the page it
                // only maps into its timetable, which `WRITE` alone allows.
                (activation, abi::rights::WRITE | abi::rights::GRANT),
                (packages, abi::rights::WRITE | abi::rights::GRANT),
                (DURABLE_WINDOW, abi::rights::WRITE),
            ],
            blobs: &[(login_protocol::session::TIMETABLE_VA, timetable)],
            stack_pages: SESSION_STACK_PAGES,
            ..ChildEndowment::new(Retention::Nothing)
        },
    );
    cap_delete(activation);
    cap_delete(packages);
    let started = match built {
        Ok(child) => start_child(child, timetable.len() as u64, 0, 0),
        Err(()) => false,
    };
    let answer = if started { receive(ready).0 } else { 0 };
    cap_delete(its_budget);
    if answer != login_protocol::session::READY || !map_page_frame(page, va, true, own_ut) {
        cap_delete(ready);
        // The session process has stopped (it reports a failure and exits) or never ran. What it
        // split off its budget before failing, if anything, stays until the user's budget is
        // reclaimed: see this program's BUGS. Discarding `session` also takes the two caretakers.
        cap_delete(page);
        discard(session);
        return None;
    }
    Some((session, page, ready))
}

/// **The two read-only store caretakers a durable session's timetable reads through** (milestone
/// 152, Fork 8 ruled D by calef on 2026-09-27, on #1377): `activation/` and `packages/`, each an
/// `fs_subtree_caretaker` holding the file service's endpoint badged with
/// [`login_protocol::DURABLE_WINDOW`] and staging through [`DURABLE_WINDOW`]'s page, never window
/// 0, which this process and every signed-in user's caretaker share. Built in `region` from
/// `own_ut`'s scratch, the way [`mint`] builds one. The two narrowed endpoints, or `None` with
/// nothing of this process's own left behind (whatever started in `region` goes with it).
fn store_caretakers(own_ut: u64, region: u64, care: &elf::Elf) -> Option<(u64, u64)> {
    let badged = u64::try_from(user_mode_runtime::badge(
        FS_EP,
        login_protocol::DURABLE_WINDOW,
    ))
    .ok()?;
    let activation = store_caretaker(own_ut, region, care, badged, activation_set::DIRECTORY);
    let packages = activation
        .and_then(|_| store_caretaker(own_ut, region, care, badged, activation_set::PACKAGES));
    cap_delete(badged);
    match (activation, packages) {
        (Some(a), Some(p)) => Some((a, p)),
        (Some(a), None) => {
            cap_delete(a);
            None
        }
        _ => None,
    }
}

/// One of [`store_caretakers`]: `name` at the store's root, `READ | DESCEND` and nothing more.
fn store_caretaker(
    own_ut: u64,
    region: u64,
    care: &elf::Elf,
    badged: u64,
    name: &str,
) -> Option<u64> {
    let name = name.as_bytes();
    let narrow_ep = retype_obj(region, abi::objtype::RENDEZVOUS).ok()?;
    let Ok(ready) = retype_obj(region, abi::objtype::RENDEZVOUS) else {
        cap_delete(narrow_ep);
        return None;
    };
    let (lo, hi) = filesystem_protocol::grant::pack_name(name);
    let spec = filesystem_protocol::grant::spec(
        name.len(),
        filesystem_protocol::dir::READ | filesystem_protocol::dir::DESCEND,
    );
    let started = build_child(
        own_ut,
        region,
        care,
        &ChildEndowment {
            caps: &[
                (badged, abi::rights::WRITE),
                (narrow_ep, abi::rights::READ),
                (ready, abi::rights::WRITE),
            ],
            maps: &[(CARETAKER_FS_VA, DURABLE_WINDOW, abi::address_space::MAP_RW)],
            stack_pages: CARETAKER_STACK_PAGES,
            ..ChildEndowment::new(Retention::Nothing)
        },
    )
    .is_ok_and(|child| start_child(child, lo, hi, spec));
    let verdict = if started { receive(ready).0 } else { 0 };
    cap_delete(ready);
    if verdict != filesystem_protocol::fixture::READY {
        cap_delete(narrow_ep);
        return None;
    }
    Some(narrow_ep)
}

/// One connecting client's own private channel: this process's own copies of the request/result
/// endpoints [`connect`] minted (narrowed copies went to the client; see `_start`), where the
/// staging page they share landed in this process's own address space, and the region every one of
/// those objects was retyped from (`region`, never delegated: this process's own means of reclaiming
/// the whole channel in one call once it is done with it, `_start`'s own [`reclaim`] after
/// [`serve_login`] returns).
struct Channel {
    /// `RECEIVE`, this process's own copy (the client's is `WRITE`).
    request: u64,
    /// `WRITE | GRANT`, this process's own copy (the client's is `READ`).
    result: u64,
    /// `READ | WRITE`, this process's own copy (the client's is also `READ | WRITE`: both ends
    /// stage into and read the same frame, the way `FS_PAGE_FRAME` already works for the caretaker
    /// hop).
    page: u64,
    /// Where `page` is mapped in this process's own address space.
    va: u64,
    /// The `MemoryRegion` `request`, `result` and `page` were all retyped from, and the page tables
    /// for `page`'s own mapping were drawn from too. This process's own, never delegated.
    region: u64,
}

/// **Mint one connecting client's own private channel**: a fresh, dedicated region, and a
/// request/result rendezvous pair and a staging page retyped from it. `connect_seq` picks a scratch
/// VA this process has never mapped before (`page_frame::MAP` refuses a second mapping at an
/// already-mapped `va`, so `_start`'s own counter bumps by one page per successful call rather than
/// reusing one). `None` on any failure, which the caller answers with [`login_protocol::DENIED`].
///
/// **Retyped from their own region, not from a shared budget directly, and that choice is the
/// whole reason this channel is reclaimable at all.** An earlier version of this function retyped
/// `request`/`result`/`page` straight out of `CONSTRUCTION_UT`, which this process's `_start` could
/// only ever answer with `cap_delete` (removing this process's own reference) and never with
/// `MemoryRegion::DESTROY` (which needs a region, not a bare object, to act on). A rendezvous
/// retyped by `RETYPE_OBJ` lives in the kernel's own global registry (`kernel::sched::MAX_RENDEZVOUS`,
/// 512 slots, shared by every process the machine is running, not this one's own budget) until the
/// region it came from is destroyed, so `cap_delete` alone leaked two of those slots, permanently,
/// per connect. This suite's own tests caught it: a later, unrelated test failed with "out of
/// rendezvous points" after this program's ~29 test-suite connects had quietly spent 58 of the 512
/// the whole machine shares. Splitting a small region here, and destroying it in `_start` once
/// [`serve_login`] is done with it, is what makes the channel's objects, not merely this process's
/// own capabilities to them, actually go away.
///
/// **`channel_ut` is [`CHANNEL_UT_PAGES`], not [`CONSTRUCTION_UT`]**, and that is the second half of
/// the same story: a region carved here is destroyed while the login it carried may still be alive,
/// so carving it from the budget that also holds live sessions left one LIFO hole per connect. See
/// that constant's own doc for the measurement.
fn connect(channel_ut: u64, connect_seq: u64) -> Option<Channel> {
    let region = memory_region_split(channel_ut, CHANNEL_REGION_PAGES).ok()?;
    // **Every abandoned step below gives back its capability slots as well as its memory**
    // ([`discard`], and `cap_delete` for what was retyped before the step that failed). A partial
    // connect that left them behind would spend this process's sixteen-slot table down exactly the
    // way the served path used to; see this program's BUGS.
    let Ok(request) = retype_obj(region, abi::objtype::RENDEZVOUS) else {
        discard(region);
        return None;
    };
    let Ok(result) = retype_obj(region, abi::objtype::RENDEZVOUS) else {
        cap_delete(request);
        discard(region);
        return None;
    };
    let Ok(page) = retype_page_frame_from(region) else {
        cap_delete(result);
        cap_delete(request);
        discard(region);
        return None;
    };
    let va = CONNECT_VA_BASE + connect_seq * login_protocol::PAGE as u64;
    // Page tables for this new mapping come from `region` itself: the channel's whole cost, objects
    // and page tables alike, lives in one place and comes home in one `DESTROY`.
    if !map_page_frame(page, va, true, region) {
        // The whole region is abandoned here rather than picked apart: nothing in it has a live
        // thread (only `RETYPE_OBJ`/`RETYPE`, never a `THREAD_CONTROL_BLOCK`), so `reclaim` is
        // expected to succeed on its first attempt, the same assumption `_start`'s own call after a
        // *successful* connect makes.
        cap_delete(page);
        cap_delete(result);
        cap_delete(request);
        discard(region);
        return None;
    }
    // SAFETY: `va` was just mapped, read/write, by this process and by no one else yet (the frame
    // has not been delegated to a client at the point this runs).
    unsafe {
        core::ptr::write_bytes(va as *mut u8, 0, login_protocol::PAGE);
    }
    Some(Channel {
        request,
        result,
        page,
        va,
        region,
    })
}

/// **Mint one principal's capability set**: a fresh `fs_subtree_caretaker` attenuated to
/// `identity`'s own home subtree (DECISIONS §117: the identity string, used directly, with no
/// separate lookup table), a fresh budget, and the construction region itself (the caretaker's own
/// logout ticket; see this program's module docs, "Reclaiming a session"), all three held with full
/// rights so [`delegate`] can narrow them on the way out. `None` on any failure, which this
/// process's caller answers with [`login_protocol::DENIED`] (see this program's BUGS on why that is the
/// honest fold rather than a missing distinction, and on the two failures this now folds in
/// alongside "the construction budget is spent": an identity too long for the grant mechanism, and
/// an authenticated identity with no provisioned subtree).
///
/// `identity` must already name a subtree `identity_provisioner` created; this process never
/// creates one (DECISIONS §117: provision-time creation, not auto-vivified at login).
///
/// `care` is `None` when `_start`'s own measurement check refused `fs_subtree_caretaker`'s bytes
/// (this program's BUGS, "does not consult `measured_boot::PROGRAM_MEASUREMENTS`", resolved): this
/// function has nothing to build from and returns `None` immediately, the same as every other
/// reason it cannot serve a login.
fn mint(own_ut: u64, care: Option<&elf::Elf>, identity: &[u8]) -> Option<(u64, u64)> {
    let care = care?;

    // **The grant name travels in two `START` argument words, not a frame** (`filesystem_protocol::grant`'s own
    // doc), so it is capped at `grant::MAX_NAME` (16 bytes): smaller than `login_protocol::MAX_IDENTITY`
    // (64), the bound `identity` already satisfies by construction (`login_protocol::read` checked it).
    // `pack_name` does not itself refuse an oversized name; it silently stops copying at the 16th
    // byte, which would otherwise mint a caretaker attenuated to a *different, truncated* name than
    // the one `identity_provisioner` created. Refusing here, before anything is built, is what keeps
    // that silent truncation from ever happening. See this program's BUGS: identities over 16 bytes
    // cannot get a per-identity subtree in this slice at all.
    if !filesystem_protocol::grant::fits(identity) {
        return None;
    }

    let region = memory_region_split(CONSTRUCTION_UT, CARETAKER_REGION_PAGES).ok()?;
    // As in [`connect`]: an abandoned step gives back its capability slots as well as its memory
    // ([`discard`]). See this program's BUGS for what leaving them behind cost.
    let Ok(narrow_ep) = retype_obj(region, abi::objtype::RENDEZVOUS) else {
        discard(region);
        return None;
    };
    let Ok(ready) = retype_obj(region, abi::objtype::RENDEZVOUS) else {
        cap_delete(narrow_ep);
        discard(region);
        return None;
    };

    let (lo, hi) = filesystem_protocol::grant::pack_name(identity);
    let spec = filesystem_protocol::grant::spec(identity.len(), filesystem_protocol::dir::ALL);

    // Its whole authority: the file service to attenuate, the endpoint it will serve, one place to
    // say it is ready, and the frame it shares with the file service. No untyped of its own, no
    // clock, nothing that could name another process. See `crates/system_initializer::build_caretaker`,
    // which this mirrors.
    let built = build_child(
        own_ut,
        region,
        care,
        &ChildEndowment {
            caps: &[
                (FS_EP, abi::rights::WRITE),
                (narrow_ep, abi::rights::READ),
                (ready, abi::rights::WRITE),
            ],
            maps: &[(CARETAKER_FS_VA, FS_PAGE_FRAME, abi::address_space::MAP_RW)],
            stack_pages: CARETAKER_STACK_PAGES,
            ..ChildEndowment::new(Retention::Nothing)
        },
    );
    let Ok(child) = built else {
        cap_delete(ready);
        cap_delete(narrow_ep);
        // **`build_child` leaks its own capability slots on failure**, which this cannot reach: it
        // returns `Err(())` with nothing named, so the address space it retyped and the frame it
        // was mid-way through stay in this process's table. Recorded in
        // `supervision_protocol::build_child_space`'s own BUGS rather than worked around here, since
        // every caller of that function has the same problem and none of them can fix it.
        discard(region);
        return None;
    };
    let started = start_child(child, lo, hi, spec);
    if !started {
        cap_delete(ready);
        cap_delete(narrow_ep);
        // **`region` used to be abandoned here**, on a comment claiming this process had no
        // `DESTROY` capability on its own construction budget's children. That was never true (the
        // descent-refused path below already destroyed it, with the same capability), and it is the
        // same leak the served path had; see BUGS. A caretaker that failed to start left nothing
        // running in the region, so `DESTROY` has nothing to wait on.
        discard(region);
        return None;
    }
    // The one bounded wait: the caretaker's descent against the file service, exactly the
    // handshake `crates/system_initializer::build_caretaker` performs.
    //
    // **This is also where "the credential is real but nobody ever provisioned this identity's
    // subtree" is answered**, and deliberately with no special case: `identity_provisioner` didn't
    // run, or its `MKDIR` never reached this file service's disk, so the caretaker's `OPENDIR`
    // against `identity` comes back `ENOENT` and it reports [`filesystem_protocol::fixture::DESCENT_REFUSED`]
    // here instead of `READY`, which this function already turns into `None` and this program's
    // caller already folds into `login_protocol::DENIED`, indistinguishable from a wrong password. See
    // this program's BUGS for why that fold is the considered answer for this case too, not merely
    // an accident of reusing the same code path.
    let (verdict, _, _) = receive(ready);
    cap_delete(ready);
    if verdict != filesystem_protocol::fixture::READY {
        cap_delete(narrow_ep);
        // **`region` used to be abandoned here.** The caretaker's `OPENDIR` was refused, so it has
        // already called `exit()` (`fs_subtree_caretaker.rs`'s own descent handshake): nothing is
        // running in `region` anymore, and `narrow_ep` (retyped from it) was just deleted above, so
        // this is case (a) or a plain corpse, never case (c) (`notes/hung-component.md`'s
        // taxonomy; see this program's module docs, "Reclaiming a session", for why that
        // distinction is what makes `DESTROY` usable at all rather than merely desired). Reclaiming
        // it here, rather than leaving it for a client that will never receive this `region` (a
        // failed mint hands back nothing), is what keeps this failure path from being a second,
        // silent leak alongside the one this function's caller already answers with `DENIED`.
        // [`discard`] rather than [`reclaim`]: the capability naming the region is a table slot of
        // its own and `DESTROY` does not free it (BUGS).
        discard(region);
        return None;
    }

    // **`region` is not dropped here.** It is returned to the caller, which delegates it to the
    // authenticated client as the fourth capability (the caretaker's own logout ticket; see this
    // program's module docs, "Reclaiming a session") and only then deletes its own copy. The
    // budget is the caller's too since milestone 152, because a reattaching login hands back an
    // existing one rather than splitting a fresh one.
    Some((narrow_ep, region))
}

/// **Reclaim a construction region, retrying while something in it can still run.**
/// `crates/system_initializer::reclaim`'s own idiom, reused rather than re-derived: a `DESTROY` of a
/// region holding a live thread is refused with §16's kill armed, and preemptions later the retry
/// succeeds.
///
/// **The bound is 1024, not the 64 this used to share with `system_initializer`.** Until milestone
/// 152 the only residents this waited on were caretakers that had already exited or were parked on
/// their own endpoints (never on a foreign one; see this program's module docs), and one preemption
/// was enough. [`Durable::retire`] added a third shape: the session region's two store caretakers
/// being killed by §40's cascade as their supervisor's supervisor exits, an asynchronous teardown
/// this process can only wait out. Measured 2026-09-30 on the riscv64 TCG leg: at 64 the wait gave
/// up inside the race **4 boots in 4**, `discard` silently leaked the whole 784-page durable
/// budget, and the next `SCHEDULE` login was answered `DENIED` for want of a split; at 1024 the
/// same boot passed 7 in 7. A caller stuck past this many attempts still has a different problem
/// than this loop can fix, which is the same sentence `components/src/session.rs`'s `ATTEMPTS`
/// (1024, from `job_undertaker`) carries for the same wait.
fn reclaim(region: u64) {
    for _ in 0..RECLAIM_ATTEMPTS {
        if memory_region_destroy(region) {
            return;
        }
        yield_now();
    }
}

/// How many times [`reclaim`] retries. Not `system_initializer`'s 64; see [`reclaim`] for the
/// durable-retirement measurement that separates them.
const RECLAIM_ATTEMPTS: usize = 1024;

/// **Give a region back completely: the memory *and* this process's own capability-table slot.**
///
/// [`reclaim`] alone does the first half only, and the difference is what made this program's own
/// second login fail on a correct password (see BUGS). `MemoryRegion::DESTROY` tears down the
/// objects retyped from the region and returns its pages; it leaves the `MemoryRegion` capability
/// that named it sitting in the caller's table, stale but still occupying one of
/// `kernel::cap::CAPABILITY_TABLE_SLOTS` (sixteen). A long-lived server that destroys a region per
/// request therefore runs out of *slots* while its budget still looks healthy, and the failure
/// arrives as whatever that server says when it cannot serve.
///
/// Every site in this program that stops wanting a region calls this rather than [`reclaim`],
/// because there is no case here where keeping the stale capability is useful.
fn discard(region: u64) {
    reclaim(region);
    cap_delete(region);
}

/// **Is `identity` on the owner's run-unvouched list?** (DECISIONS §221 (the boot prompt is the
/// owner's console), ruling 2.) Opens [`login_protocol::RUN_UNVOUCHED_LIST`] at the root of the
/// file service and asks [`login_protocol::lists`]. Every way of not reading it (no mapped page,
/// no file, a file larger than a page, a refused read) lists nobody.
///
/// Read on every login rather than once at start, so the owner's edit at the boot prompt holds
/// from the next login without a reboot. The page is the one every session and caretaker shares
/// with the server; this is sound for the reason those clients' use of it is, that only one session
/// is live at a time (the terminal rule, checked before this runs), and that session is still
/// being built.
fn listed(identity: &[u8]) -> bool {
    on_list(login_protocol::RUN_UNVOUCHED_LIST, identity)
}

/// **Whether `identity` is suspended** (milestone 152, calef's §108 ruling of 2026-09-26): on the
/// owner's [`login_protocol::SUSPENDED_LIST`]. The one place this program reads it. An absent or
/// unreadable list suspends nobody; `login_protocol::SUSPENDED_LIST` says why that direction.
fn is_suspended(identity: &[u8]) -> bool {
    on_list(login_protocol::SUSPENDED_LIST, identity)
}

/// **Whether the owner's list `file`, at the file service's root, names `identity`**, read fresh
/// on every call so an edit at the console holds without a restart. Every way of failing to read it
/// answers `false`.
fn on_list(file: &str, identity: &[u8]) -> bool {
    let mut list = [0u8; login_protocol::PAGE];
    // A short read lists by what arrived, which can only be fewer names.
    read_file(filesystem_protocol::fs::ROOT, file.as_bytes(), &mut list)
        .is_some_and(|n| login_protocol::lists(&list[..n], identity))
}

/// **Read `name` under directory handle `at` into `out`**, through the file page at [`LIST_VA`]:
/// the byte count, or `None` for no mapped page, no such file, or one larger than a page. The one
/// way this program reads a file, for the owner's lists, the durable-session manifest and an
/// identity's stored schedule alike.
fn read_file(at: u64, name: &[u8], out: &mut [u8]) -> Option<usize> {
    use filesystem_protocol::fs;
    let page = file_page()?;
    page[..name.len()].copy_from_slice(name);
    let opened = call(FS_EP, fs::req(fs::OPEN, at, name.len() as u64), 0).0 as i64;
    if opened < 0 {
        return None;
    }
    let handle = opened as u64;
    let size = call(FS_EP, fs::req(fs::FSTAT, handle, 0), 0).0 as i64;
    let read = if (0..=out.len().min(login_protocol::PAGE) as i64).contains(&size) {
        call(FS_EP, fs::req(fs::READ, handle, size as u64), 0).0 as i64
    } else {
        -1
    };
    call(FS_EP, fs::req(fs::CLOSE, handle, 0), 0);
    let n = usize::try_from(read).ok()?.min(out.len());
    out[..n].copy_from_slice(&page[..n]);
    Some(n)
}

/// **Replace `name` at the file service's root with `bytes`**, creating it if absent. `true` when
/// every byte landed.
fn write_root_file(name: &[u8], bytes: &[u8]) -> bool {
    use filesystem_protocol::fs;
    let Some(page) = file_page() else {
        return false;
    };
    page[..name.len()].copy_from_slice(name);
    let mut h = call(FS_EP, fs::req(fs::CREATE, fs::ROOT, name.len() as u64), 0).0 as i64;
    if h < 0 {
        page[..name.len()].copy_from_slice(name);
        h = call(FS_EP, fs::req(fs::OPEN, fs::ROOT, name.len() as u64), 0).0 as i64;
        if h < 0 || (call(FS_EP, fs::req(fs::TRUNCATE, h as u64, 0), 0).0 as i64) < 0 {
            return false;
        }
    }
    let h = h as u64;
    page[..bytes.len()].copy_from_slice(bytes);
    let wrote = call(FS_EP, fs::req(fs::WRITE, h, bytes.len() as u64), 0).0 as i64;
    call(FS_EP, fs::req(fs::CLOSE, h, 0), 0);
    wrote == bytes.len() as i64
}

/// **Read `identity`'s stored schedule** (`<root>/<identity>/schedule`, DECISIONS §122 (the on-disk
/// schedule store)) into `out`: the byte count, or `None` when there is none. Descends by name, as
/// [`rederive`] and the file service's other clients do, and never enumerates.
fn read_stored_schedule(identity: &[u8], out: &mut [u8]) -> Option<usize> {
    use filesystem_protocol::{dir, fs};
    let page = file_page()?;
    page[..identity.len()].copy_from_slice(identity);
    let d = call(
        FS_EP,
        fs::req(fs::OPENDIR, fs::ROOT, identity.len() as u64),
        dir::READ | dir::DESCEND,
    )
    .0 as i64;
    if d < 0 {
        return None;
    }
    let n = read_file(d as u64, schedule_store::SCHEDULE_FILE_NAME.as_bytes(), out);
    call(FS_EP, fs::req(fs::CLOSE, d as u64, 0), 0);
    n
}

/// **Add `identity` to, or take it off, the durable-session manifest** (DECISIONS §125 (which identities have
/// pending work)), which is what [`rederive`] reads at the next start-up to learn whose schedules to
/// bring back. A failure is not fatal to a login, and costs only that start-up pass.
fn record_in_manifest(identity: &[u8], present: bool) {
    let name = schedule_store::MANIFEST_FILE_NAME.as_bytes();
    let mut now = [0u8; login_protocol::PAGE];
    let had = read_file(filesystem_protocol::fs::ROOT, name, &mut now).unwrap_or(0);
    let mut next = [0u8; login_protocol::PAGE];
    let n = if present {
        login_protocol::with_listed(&now[..had], identity, &mut next)
    } else {
        login_protocol::without_listed(&now[..had], identity, &mut next)
    };
    if let Some(n) = n
        && next[..n] != now[..had]
    {
        write_root_file(name, &next[..n]);
    }
}

/// The file page at [`LIST_VA`], when `_start` managed to map it.
fn file_page() -> Option<&'static mut [u8]> {
    if !LIST_MAPPED.load(core::sync::atomic::Ordering::Relaxed) {
        return None;
    }
    // SAFETY: `_start` mapped the file page read/write at `LIST_VA`, one page, for this process's
    // life. One thread per address space, and every caller is done with the slice before the next
    // takes it.
    Some(unsafe { core::slice::from_raw_parts_mut(LIST_VA as *mut u8, login_protocol::PAGE) })
}

/// Delegate our own copy of `slot`, narrowed to `rights`, over `ep`. `GRANT` must already be on our
/// own copy for the kernel to allow this at all (`abi::rendezvous::SEND_CAP`'s contract); every
/// capability this process delegates was retyped or split by this process, so it always is. `ep` is
/// [`RESULT`] for a [`login_protocol::CONNECTED`] answer and a channel's own private `result` for a
/// [`login_protocol::OK`] one; both are this process's own copy, always held with `GRANT`.
fn delegate(ep: u64, slot: u64, rights: u64) {
    send_cap(ep, slot, rights, 0);
}

/// Zero the identity/secret staged at `va`, on every path: a malformed request, a denial, and a
/// success all leave a presented secret sitting in a page two processes share until this runs.
fn wipe_page(va: u64) {
    // SAFETY: `connect` mapped one page read/write here, and this process is the only writer
    // between a request arriving on the channel it belongs to and that channel's reply going out.
    let page = unsafe { core::slice::from_raw_parts_mut(va as *mut u8, login_protocol::PAGE) };
    login_protocol::wipe(page);
}

fn fail(step: u64) -> ! {
    send(AUDIT, 0xDEAD_0000_0000_0000 | step, 0, 0);
    supervision_protocol::fail()
}

user_mode_runtime::panic_handler!();
