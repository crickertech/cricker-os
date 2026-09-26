//! **The login service's test client** (milestone 49; see `components/src/login.rs`).
//!
//! **One binary, one endowment, and two separate things a run is handed: a behaviour and a
//! credential** (milestone 293). It shares `credentialer_test_client`'s reason for being one
//! program rather than several (a program that shares the honest path with an attempted-wrong-secret
//! run is a fairer test of a refusal than a different program failing for its own reasons), but it
//! delivers that by construction rather than by convention: the wrong-secret run is not a separate
//! arm that could drift away from the honest one, it is [`LOGIN`] with a different secret.
//!
//! Until 293 this file had **eleven roles** and its first act was a lookup from role number to a
//! pair of byte strings. Five of the eleven ran byte-identical code and differed only in that pair
//! (`chris`, `corinne`, the same `chris` with a wrong secret, a `graeme` with no home subtree, and a
//! `corinne` presenting a real credential while the terminal was on loan), and two more differed
//! only in which identity they wrote into a marker file. A role that differs only in its credentials
//! is not a role.
//!
//! So a run is `_start(behaviour, identity, secret)`:
//!
//! - **`behaviour`** is what this process does with the session it gets, and the eight below are
//!   eight genuinely different flows rather than eight spellings of one.
//! - **`identity` and `secret`** are indices into [`credential_protocol::fixture::PEOPLE`], the roster
//!   three files used to keep their own copy of. They are composed at the call site, so
//!   `(CHRIS, CHRIS)` is an honest login and `(CHRIS, WRONG)` is the same program presenting a
//!   secret that is nobody's. [`FREE_TERMINAL`] takes `(NONE, NONE)` and authenticates nothing.
//!
//! Every behaviour holds the identical endowment: the login service's front-door request endpoint,
//! its front-door result endpoint, a report endpoint, and a small scratch region (milestone 49's
//! channel-per-client update: a run must map the page [`login_protocol::CONNECTED`] delegates before it
//! holds anything else of its own to draw page tables from).
//!
//! # The eight behaviours
//!
//! - [`LOGIN`] presents the credential it was handed and stops at the verdict, then proves what it
//!   received. Every credential case is this one behaviour: two identities' correct credentials
//!   (both must succeed, and the kernel test compares what each received; two distinct caretaker
//!   endpoints is the channel-shaped attribution DECISIONS §109 decided on), a real identity with
//!   the wrong secret (must be refused, and nothing may follow: this behaviour never calls
//!   `RECV_CAP` after a refusal, because the protocol promises nothing does on a denial and a client
//!   that tried would block forever), a real authenticated identity nobody provisioned a subtree for
//!   (refused identically, DECISIONS §117's "no distinguishable signal" answer; see `login.rs`'s own
//!   BUGS), and a real credential presented while [`HOLD_TERMINAL`]'s loan is outstanding (refused
//!   [`login_protocol::NO_TERMINAL`], which the same "nothing follows a refusal" flow covers for free
//!   since `NO_TERMINAL != OK`).
//! - [`WRITE_MARKER`] (DECISIONS §117) logs in and then, through the delegated directory, `CREATE`s
//!   a one-shot marker file naming **the identity it was handed**, and checks that
//!   [`filesystem_protocol::fixture::tree::INNER`] is *not* there (that name lives only in the old,
//!   shared fixture subtree every identity used to be attenuated to before §117; its absence is this
//!   client's own proof that the granted directory is not that one). Two identities run it; it is
//!   one behaviour because the only thing that differed was the name it wrote, and that name is now
//!   an argument.
//! - [`READ_MARKER`] logs in in a second, independent channel and reads the marker back: the kernel
//!   test compares what it reads against what each [`WRITE_MARKER`] run wrote, which is the whole
//!   property under test: two different identities land in two different, isolated subtrees, and the
//!   same identity's two sessions land in the *same* one.
//! - [`LOGOUT`] logs in, proves the directory works exactly like [`LOGIN`] does, then calls
//!   `MemoryRegion::DESTROY` on the fourth delegated capability (`login_protocol`'s own logout ticket)
//!   and proves the *directory* came down with it: a further `READDIR` through it must fail. This is
//!   milestone 49's caretaker-teardown fix, proven end to end rather than merely by the syscall's own
//!   return code. It reports how long that `DESTROY` waited (microseconds) in the third report word,
//!   where the others put an identity hint; see `destroy_with_retry`.
//! - [`HOLD_TERMINAL`] (milestone 49's terminal update) logs in, proves the fifth delegated
//!   capability (the terminal) actually names a real, working endpoint by sending a known word
//!   through it (which the kernel test catches with its own `sched::ipc_recv` on the stand-in
//!   `Wiring::term_ep`, the same "prove it works, not merely that it arrived" standard the directory
//!   and budget already get), then tears its own session down exactly like [`LOGOUT`] -- but,
//!   deliberately, **without** sending [`login_protocol::logout_word`], so the terminal itself stays on
//!   loan even though the session's memory came home. That is the property under test: session
//!   teardown and freeing the terminal are two independent acts.
//! - [`PRESENT_RUN_UNVOUCHED`] (DECISIONS §219 (how the shell names an installed program to the spawner) gate D2) logs in like [`LOGIN`] and sends a known
//!   word on the sixth delegated capability, the run-unvouched endpoint, which the kernel test
//!   catches on the endpoint it gave `login`. Every successful login that was announced a sixth
//!   capability also tries to delegate it and reports the refusal ([`F_RUN_UNVOUCHED_NOT_GRANTABLE`]).
//! - [`PENDING_WORK`], for milestone 152 (durable delegation), logs in, splits a one-page child off
//!   the delegated budget to stand for a scheduled job's authority, and then attempts the logout
//!   [`LOGOUT`] performs. The first step of that logout, `MemoryRegion::DESTROY` on the budget, must
//!   be refused `NotPermitted` while the child lives, the live-children rule of DECISIONS §16 (object
//!   revocation), and the budget must still work after the refusal. Only once the child is
//!   destroyed does the logout go through, in [`LOGOUT`]'s order and with its proofs. This is the property milestone 152's
//!   durable session leans on, proven on the object a login session actually is.
//! - [`FREE_TERMINAL`] sends [`login_protocol::logout_word`] on the front door directly, without ever
//!   calling `CONNECT`: there is no identity or secret in this word at all (`login_protocol`'s own BUGS
//!   on what this does and does not authenticate). It is the one behaviour that takes no credential,
//!   and it says so with [`credential_protocol::fixture::NONE`] rather than by convention.
//!
//! A run that succeeds does not stop at the verdict. It **uses** what it received: a directory
//! read through the caretaker's endpoint, the file service's shared frame mapped and used for that
//! read, and a page retyped from the delegated budget. A capability that arrived but does not work
//! would pass every earlier assertion and fail only this one.
//!
//! # Capability contract
//!
//! - slot 0: the login service's front-door request endpoint, `WRITE`. Carries exactly one word
//!   this program ever sends: [`login_protocol::connect_word`].
//! - slot 1: the login service's front-door result endpoint, `READ`. [`login_protocol::CONNECTED`],
//!   then three delegated capabilities: a private request endpoint, a private result endpoint, and
//!   a staging page. The actual [`login_protocol::LOGIN`] exchange happens on the first two of those,
//!   never on slots 0/1 again. [`login_protocol::logout_word`] also travels here directly ([`FREE_TERMINAL`]),
//!   since it needs no private channel at all.
//! - slot 2: a report endpoint, `WRITE`.
//! - slot 3: a small `MemoryRegion`, `WRITE`: this program's own scratch, for `map_page_frame`'s
//!   page-table cost when it self-maps the page [`login_protocol::CONNECTED`] delegates (see
//!   `kernel::user::login_service::CLIENT_SCRATCH_UT_PAGES`).
//! - `a0`: the behaviour.
//! - `a1`: the identity, an index into [`credential_protocol::fixture::PEOPLE`].
//! - `a2`: the secret: the same index for that person's own, [`credential_protocol::fixture::WRONG`]
//!   for one that is nobody's, [`credential_protocol::fixture::NONE`] for no credential at all.
//!
//! # BUGS
//!
//! - **The identity and the secret travel as indices, not as bytes, because nothing in this tree can
//!   hand a `no_std` program a string it was not compiled against.** A process is born with three
//!   integer registers, a capability list and a set of mappings (`kernel::user::Spawn`), and that is
//!   the whole of it: `grant_plan::ArgSpec` carries exactly one *integer*, `environment_protocol`'s
//!   config page is validated against curated domains precisely so a secret cannot ride on it, and
//!   a page of bytes needs a frame the spawner allocates, writes and maps. So the bytes live in a
//!   crate both sides link and the register names which one. That is enough for a fixture and is not
//!   a credential-passing mechanism: a real program needs a capability to something that holds the
//!   secret, which is DECISIONS §41's answer and already what `login` itself uses.
//! - **A spawn argument is visible wherever a spawn is recorded.** These indices are not, which is
//!   an accident of them being small integers rather than a property anything enforces; had the
//!   bytes themselves gone into the register the same argument would print. `swish`'s `caps` preview
//!   reads a manifest and this program has none (it is spawned by the kernel test harness, not by
//!   the shell), so nothing prints them today. Nothing here is a design for carrying real secrets.
//!
//! Name: ratified 2026-09-14 (calef, milestone 293), **on the name and not the shape**: the same
//! ruling minted 293 to decide what the eleven roles should have been, so that this file's shape
//! could be fixed without the name blocking it. Minted 2026-08-22 for milestone 49 on the
//! `<service>_test_client` pattern, which was not that lane's invention: calef signed that exact
//! shape for the credential service, turning down `credcli` as a squished abbreviation and
//! `credentialer_client` because that name belongs to the real client a later milestone needs. The
//! same reservation holds here, so a future real login client is not squatted.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

use user_mode_runtime::mapped_window::MappedWindow;
use user_mode_runtime::{
    call, destroy_region, exit, map_page_frame, recv, recv_cap, retype_page_frame, send, send_cap,
    split_region, yield_now,
};

/// The login service's front-door request endpoint (slot 0), `WRITE`.
const SERVICE: u64 = 0;
/// The login service's front-door result endpoint (slot 1), `READ`.
const RESULT: u64 = 1;
/// The report endpoint (slot 2), `WRITE`.
const REPORT: u64 = 2;
/// This program's own scratch `MemoryRegion` (slot 3), `WRITE`. Used once, for `map_page_frame`'s
/// own page-table cost when this program self-maps the page [`login_protocol::CONNECTED`] delegates.
const SCRATCH: u64 = 3;

/// Where this program maps the private staging page [`login_protocol::CONNECTED`] delegates, at
/// runtime, once `CONNECT` hands back the capability naming it (milestone 49's channel-per-client
/// update; see `_start`'s own `map_page_frame` call). Not a `MappedWindow` (round 6's usual
/// collapse for a statically pre-mapped page): nothing is mapped here before this process runs.
const PAGE_VA: u64 = 0x0000_0000_00e2_0000;
/// Where this process maps the delegated file-service frame, once it has one. Distinct from
/// [`PAGE_VA`]: they are two different pages (this process's login request, and the filesystem
/// contract's shared page), and a client of both must not confuse them the way `credentialer.rs`'s
/// own two-frame rule exists to prevent for its own pair.
const FS_VA: u64 = 0x0000_0000_00f0_0000;

// SAFETY: constructing the window touches no memory; only `put_page`/`get_page` do, and every
// caller of those runs behind `if mapped { .. }` (this process's own `map_page_frame(fs_page_frame, FS_VA,
// ..)` having already returned true), the same condition the hand-rolled comment this replaces
// relied on (milestone 139 round 2; see `user_mode_runtime::mapped_window`).
const FS_WINDOW: MappedWindow =
    unsafe { MappedWindow::new(FS_VA, filesystem_protocol::PAGE as u64) };

/// **Present the credential and prove what came back.** The whole of what every credential case
/// needs: two identities' correct credentials, a wrong secret, an identity with no home subtree, and
/// a real credential presented while [`HOLD_TERMINAL`]'s loan is outstanding are one behaviour with
/// four different arguments, not four behaviours. See the module docs.
pub const LOGIN: u64 = 0;
/// Log in, write **the identity this run was handed** into the marker file, and check the old fixed
/// subtree's own name is absent (DECISIONS §117). See the module docs.
pub const WRITE_MARKER: u64 = 1;
/// Log in in an independent channel and read the marker [`WRITE_MARKER`] wrote, packed into the
/// report the same way `login`'s own audit trail packs an identity
/// ([`login_protocol::identity_hint`]). See the module docs.
pub const READ_MARKER: u64 = 2;
/// Log in, prove the directory and budget work exactly like [`LOGIN`], then use the fourth delegated
/// capability (the logout ticket; `login_protocol`'s own module docs) to tear the session down and
/// confirm it actually came down: a further `READDIR` through the now-`DESTROY`ed directory
/// capability must fail. See the module docs.
pub const LOGOUT: u64 = 3;
/// Log in, prove the fifth delegated capability (the terminal) is real by sending [`TERM_MAGIC`]
/// through it, then tear the session down like [`LOGOUT`] **without** freeing the terminal. See the
/// module docs.
pub const HOLD_TERMINAL: u64 = 4;
/// Send [`login_protocol::logout_word`] on the front door directly; no identity involved, and the one
/// behaviour that takes [`credential_protocol::fixture::NONE`] for both halves of its credential. See
/// the module docs.
pub const FREE_TERMINAL: u64 = 5;
/// Log in, and prove the sixth delegated capability (the run-unvouched one, DECISIONS §219 gate D2)
/// names the endpoint `login` was given, by sending [`RUN_UNVOUCHED_MAGIC`] on it for the kernel
/// test to receive, then tears its session down as [`HOLD_TERMINAL`] does.
pub const PRESENT_RUN_UNVOUCHED: u64 = 6;
/// Log in, split a pending-job child off the budget, and prove the logout is refused while it lives
/// and goes through once it is gone. Milestone 152; see the module docs. Provisional name.
pub const PENDING_WORK: u64 = 7;

/// **[`PRESENT_RUN_UNVOUCHED`]'s proof of life**, [`TERM_MAGIC`]'s twin for the sixth capability.
const RUN_UNVOUCHED_MAGIC: u64 = 0x_7e12_0000_0000_0002;

/// **[`PENDING_WORK`]'s stand-in for a scheduled job's authority**, split off the delegated budget.
/// One page: nothing is retyped from it. What matters is that it is a live child, which is all
/// DECISIONS §16's refusal looks at.
const PENDING_JOB_PAGES: u64 = 1;

/// The one-shot marker file [`WRITE_MARKER`] writes and [`READ_MARKER`] reads, inside the identity's own
/// granted subtree. Chosen to collide with nothing else this tree's fixtures use.
const MARKER_NAME: &str = "whoami";

/// **[`HOLD_TERMINAL`]'s proof of life for the delegated terminal.** Sent through the fifth
/// delegated capability once it is in hand; the kernel test's own `sched::ipc_recv` on the stand-in
/// `Wiring::term_ep` is what confirms the delegated copy names the real object rather than merely
/// having arrived. The exact value carries no meaning beyond being recognisable in a test assertion.
const TERM_MAGIC: u64 = 0x_7e12_0000_0000_0001;

/// The report's first word: what the service answered.
pub const RPT_OK: u64 = login_protocol::OK;
pub const RPT_DENIED: u64 = login_protocol::DENIED;
pub const RPT_MALFORMED: u64 = login_protocol::MALFORMED;
/// Milestone 49's terminal update: the terminal was already on loan to another session.
pub const RPT_NO_TERMINAL: u64 = login_protocol::NO_TERMINAL;
/// [`FREE_TERMINAL`]'s own answer: the terminal is free again.
pub const RPT_LOGGED_OUT: u64 = login_protocol::LOGGED_OUT;

/// Bits of the report's second word, set only when [`RPT_OK`] is the first: which of the delegated
/// capabilities this process proved actually work, rather than merely that they arrived.
pub const F_DIR_WORKS: u64 = 1 << 0;
pub const F_BUDGET_WORKS: u64 = 1 << 1;
/// **Set when [`filesystem_protocol::fixture::tree::INNER`] is confirmed absent** from the granted directory:
/// that name exists only in the old, shared fixture subtree every identity used to be attenuated to.
/// Its absence is this client's own proof (not merely an assertion) that the identity this run
/// authenticated as is
/// not looking at that subtree. Set only by [`WRITE_MARKER`].
pub const F_NOT_SHARED_SUBTREE: u64 = 1 << 2;
/// **Set when the marker file was created and written successfully.** Set only by
/// [`WRITE_MARKER`]; its absence there means the isolation proof below did
/// not get to run at all, which the kernel test must treat as its own failure rather than silence.
pub const F_MARKER_WRITTEN: u64 = 1 << 3;
/// **Set when the fourth capability's `MemoryRegion::DESTROY` returned success.** Set only by
/// [`LOGOUT`]; retried on refusal until the region comes down or [`DESTROY_WAIT_SECS`] runs out
/// (`login_protocol`'s own module docs, on the fourth capability, name the transient window this
/// covers, and `destroy_with_retry` says why the bound is a clock).
pub const F_TEARDOWN_OK: u64 = 1 << 4;
/// **Set when a `READDIR` through the directory capability failed *after* teardown.** Set only by
/// [`LOGOUT`]; this is the proof that the capability, not merely the syscall, came down: a
/// `DESTROY` that returned success but left the directory answering requests would pass every check
/// up to this one and fail only this one.
pub const F_DEAD_AFTER_TEARDOWN: u64 = 1 << 5;
/// **Set when `MemoryRegion::DESTROY` on the *budget* (the third delegated capability) also returned
/// success.** Set only by [`LOGOUT`], and proves the other half of `login.rs`'s BUGS: a full
/// logout needs no capability beyond what every run already receives, because `budget` was always
/// delegated with `WRITE`, the one right `DESTROY` needs. See the module docs.
pub const F_BUDGET_TEARDOWN_OK: u64 = 1 << 6;
/// **Set when a further `RETYPE` on the budget failed *after* its own teardown.** The budget's half
/// of [`F_DEAD_AFTER_TEARDOWN`].
pub const F_BUDGET_DEAD_AFTER_TEARDOWN: u64 = 1 << 7;
/// **Set when the fifth delegated capability (the terminal) delivered [`TERM_MAGIC`] to a real
/// receiver.** Set only by [`HOLD_TERMINAL`]. A capability that merely arrived (`RECV_CAP`
/// succeeded) would pass every earlier check and never set this one: `send` only returns once a
/// receiver is actually matched.
pub const F_TERM_WORKS: u64 = 1 << 8;
/// **Set when a sixth capability arrived and could not be passed on**: `SEND_CAP` of it was refused
/// `NotPermitted`, which the kernel answers before it matches any receiver, for a capability that
/// lacks `GRANT`. That refusal is DECISIONS §219 limitation 2 holding: a session holds the right to
/// run unvouched bytes and cannot lend it by delegation. Set on every successful login that was
/// announced a sixth capability.
pub const F_RUN_UNVOUCHED_NOT_GRANTABLE: u64 = 1 << 9;
/// **Set when [`RUN_UNVOUCHED_MAGIC`] was taken on the sixth capability.** Set only by
/// [`PRESENT_RUN_UNVOUCHED`]; like [`F_TERM_WORKS`], `send` returns only once a receiver matched.
pub const F_RUN_UNVOUCHED_WORKS: u64 = 1 << 10;
/// **Set when `OK` announced a sixth capability at all** ([`login_protocol::RUN_UNVOUCHED_FOLLOWS`]),
/// for every behaviour. Its absence is how the kernel test sees an identity that is not on the
/// owner's list (DECISIONS §221 (the boot prompt is the owner's console), ruling 2) without waiting
/// on an endpoint nothing will send to.
pub const F_RUN_UNVOUCHED_ANNOUNCED: u64 = 1 << 11;
/// **Set when the budget's `DESTROY` was refused `NotPermitted` while a pending-job child lived.**
/// Set only by [`PENDING_WORK`]. Any other answer, success included, leaves it clear: a logout that
/// succeeded here would have torn down a session with scheduled work still hanging off it.
pub const F_LOGOUT_REFUSED_WHILE_PENDING: u64 = 1 << 12;
/// **Set when the budget still retyped a page after that refusal.** Set only by [`PENDING_WORK`].
/// A refused `DESTROY` that half-tore the session down would set the flag above and fail this one.
pub const F_SESSION_SURVIVED_REFUSAL: u64 = 1 << 13;
/// **Set when the pending-job child itself was destroyed.** Set only by [`PENDING_WORK`], and it is
/// what makes the budget destroyable again.
pub const F_PENDING_WORK_DESTROYED: u64 = 1 << 14;

/// `a0` is the behaviour, `a1` the identity and `a2` the secret; see the module docs. Three
/// registers because that is what a process is born with (`kernel::user::Spawn`), and the two
/// credential halves are indices because a register cannot carry a byte string.
#[unsafe(no_mangle)]
pub extern "C" fn _start(behaviour: u64, identity: u64, secret: u64) -> ! {
    if behaviour == FREE_TERMINAL {
        // **Never calls `CONNECT` at all.** `login_protocol::logout_word` travels on the shared front
        // door directly (`login_protocol`'s own module docs): there is no secret to protect, so there
        // is nothing a private channel would buy here.
        send(SERVICE, login_protocol::logout_word(), 0, 0);
        let (verdict, _, _) = recv(RESULT);
        done(verdict, 0, 0);
    }

    // **A credential this program cannot resolve is malformed, not a default.** Every other
    // behaviour authenticates, so an out-of-range index (or `NONE`, which is what
    // [`FREE_TERMINAL`] is handed and nothing else should be) must not fall back to whoever sits at
    // index zero: a run that silently logged in as `chris` would pass every assertion that follows
    // and prove nothing about the identity the caller meant.
    let (Some(identity), Some(secret)) = (
        credential_protocol::fixture::identity(identity),
        credential_protocol::fixture::secret(secret),
    ) else {
        done(RPT_MALFORMED, 0, 0);
    };

    // **Milestone 49's channel-per-client update: connect first.** The front door's only legal
    // word; see `login_protocol`'s own module docs for the two-phase exchange. Nothing is staged for
    // this step, so there is no page to write before sending it.
    send(SERVICE, login_protocol::connect_word(), 0, 0);
    let (connect_verdict, _, _) = recv(RESULT);
    if connect_verdict != login_protocol::CONNECTED {
        // The front door answered something other than CONNECTED (MALFORMED or DENIED): nothing
        // follows, the same promise the private channel's own OK/DENIED gives.
        done(connect_verdict, 0, 0);
    }
    let (_, priv_request, _) = recv_cap(RESULT);
    let (_, priv_result, _) = recv_cap(RESULT);
    let (_, priv_page, _) = recv_cap(RESULT);

    // Map the delegated staging page using this program's own small scratch region: unlike the
    // post-auth `budget` the rest of this function uses, nothing else has been received yet at this
    // point.
    if !map_page_frame(priv_page, PAGE_VA, true, SCRATCH) {
        done(RPT_MALFORMED, 0, 0);
    }

    // SAFETY: `priv_page` was just mapped read/write at PAGE_VA, private to this process and to the
    // login service's own copy; no other client holds a capability to it. Not `PAGE_WINDOW`
    // (round 6's collapse): that type's contract assumes the wiring maps this page before the
    // process runs, which milestone 49's channel-per-client update made false here specifically:
    // the page is now mapped dynamically, per connection, from a capability CONNECT hands back at
    // runtime, so there is nothing for a compile-time-constant window to be a window onto yet.
    let page = unsafe { core::slice::from_raw_parts_mut(PAGE_VA as *mut u8, login_protocol::PAGE) };
    let Some(w0) = login_protocol::place(page, identity, secret, login_protocol::LOGIN) else {
        done(RPT_MALFORMED, 0, 0);
    };
    send(priv_request, w0, 0, 0);
    let (verdict, extra, _) = recv(priv_result);

    if verdict != login_protocol::OK {
        // `login_protocol`'s own promise: nothing follows a refusal. Reporting here, rather than
        // attempting `RECV_CAP`, is the check that the promise holds; a service that sent a fourth
        // message anyway would leave the *next* login's first `RECV_CAP` reading this one's leftover
        // word instead of blocking as it should, which is exactly the kind of protocol desync a
        // client that blindly tried to receive here would hide rather than catch.
        done(verdict, 0, 0);
    }

    // Five capabilities, in login_protocol's fixed order, on the private channel `priv_result` names.
    let (_, dir_ep, _) = recv_cap(priv_result);
    let (_, fs_page_frame, _) = recv_cap(priv_result);
    let (_, budget, _) = recv_cap(priv_result);
    let (_, region, _) = recv_cap(priv_result);
    let (_, term_ep, _) = recv_cap(priv_result);
    // The sixth, only as announced, and taken here with the other five: `login` is blocked sending
    // it, and anything this process sends first (the terminal's proof below) waits on a kernel test
    // that is itself waiting on `login`.
    let run_unvouched =
        (extra & login_protocol::RUN_UNVOUCHED_FOLLOWS != 0).then(|| recv_cap(priv_result).1);

    let mut flags = 0u64;
    if run_unvouched.is_some() {
        flags |= F_RUN_UNVOUCHED_ANNOUNCED;
    }
    let mut hint = 0u64;

    // **Prove the sixth is real (for one behaviour), then that it cannot be passed on.** In that
    // order, because the delegation is attempted on the report endpoint, which the kernel test
    // reads only after it has taken the proof of life: a capability wrongly carrying `GRANT` is then
    // sent there and garbles the report, failing the test rather than hanging it.
    if let Some(ru) = run_unvouched {
        if behaviour == PRESENT_RUN_UNVOUCHED {
            send(ru, RUN_UNVOUCHED_MAGIC, 0, 0);
            flags |= F_RUN_UNVOUCHED_WORKS;
        }
        if send_cap(REPORT, ru, abi::rights::WRITE, 0) == abi::Error::NotPermitted as i64 {
            flags |= F_RUN_UNVOUCHED_NOT_GRANTABLE;
        }
    }

    // **Prove the terminal, before anything else touches `budget`/`region`.** `send` on a plain
    // rendezvous only returns once a receiver is actually matched
    // (`crates/inter_process_communication`'s own model), so this blocks until the kernel test's
    // own `sched::ipc_recv(w.term_ep)` catches it -- a stronger proof than `RECV_CAP` alone, which
    // would pass even for a capability naming a dead or wrong object.
    if behaviour == HOLD_TERMINAL {
        send(term_ep, TERM_MAGIC, 0, 0);
        flags |= F_TERM_WORKS;
    }

    // Prove the directory capability works: map the delegated frame (which needs `budget` alive, to
    // supply page-table pages for the mapping: `user_mode_runtime::map_page_frame`'s own contract), then read the
    // granted subtree's listing through the delegated caretaker endpoint. A capability that merely
    // arrived would pass every check up to this line and fail only this one.
    let mapped = map_page_frame(fs_page_frame, FS_VA, true, budget);

    // Prove the budget works: retype one page from it. `RETYPE`'s reply is the new frame's slot
    // (>= 0) or a negative error. Done here, right after the one use of `budget` that needs it
    // alive (`map_page_frame` above), and before anything below tears it down.
    if retype_page_frame(budget) >= 0 {
        flags |= F_BUDGET_WORKS;
    }

    // **Milestone 152's durable-session property, on a real login session.** A scheduled job's
    // authority would be split off the session's budget, so a live child here stands for pending
    // work. The logout's first step must be refused for exactly as long as that child lives: one
    // attempt, not `destroy_with_retry`, because this refusal is not a kill waiting for a tick. It is
    // permanent until the child goes, and retrying would only spend the watchdog.
    if behaviour == PENDING_WORK {
        let job = split_region(budget, PENDING_JOB_PAGES);
        if job >= 0 {
            if destroy_region(budget) == abi::Error::NotPermitted as i64 {
                flags |= F_LOGOUT_REFUSED_WHILE_PENDING;
            }
            if retype_page_frame(budget) >= 0 {
                flags |= F_SESSION_SURVIVED_REFUSAL;
            }
            if destroy_region(job as u64) == 0 {
                flags |= F_PENDING_WORK_DESTROYED;
            }
        }
    }

    // **`LOGOUT` destroys `budget` before `region`, and the order is load-bearing, not a
    // style choice.** `mint()` splits both from `login`'s own `CONSTRUCTION_UT`, `region` first and
    // `budget` second (`components/src/login.rs`), so `budget` sits at the top of `CONSTRUCTION_UT`'s
    // watermark and `region` sits below it. `crates/regions`' own `return_to_parent` only un-bumps
    // a parent's watermark for a child freed at the *top* of it (LIFO, the same rule §16's object
    // revocation and `job_undertaker`'s pool already live under); a child freed out of order leaves
    // its pages a stranded hole that does not come back until the parent itself is destroyed. Get
    // this backwards (as an earlier version of this file did) and `MemoryRegion::DESTROY` still succeeds
    // on both calls, so every flag below still sets, but `CONSTRUCTION_UT`'s reusable capacity never
    // recovers: `kernel::user::login_tests::caretaker_teardown_reclaims_a_full_session_worth_of_memory`
    // starved a *later*, unrelated test in this suite of real login attempts before this ordering was
    // fixed, which is exactly the anti-oracle failure `login_protocol::DENIED`'s own fold exists to
    // prevent (a real password silently answered as though it were wrong). See `login_protocol`'s own
    // module docs on the fourth capability for the client-facing version of this note.
    if (behaviour == LOGOUT
        || behaviour == HOLD_TERMINAL
        || behaviour == PRESENT_RUN_UNVOUCHED
        || behaviour == PENDING_WORK)
        && destroy_with_retry(budget)
    {
        flags |= F_BUDGET_TEARDOWN_OK;
        if retype_page_frame(budget) < 0 {
            flags |= F_BUDGET_DEAD_AFTER_TEARDOWN;
        }
    }

    if mapped {
        let (r0, _) = call(
            dir_ep,
            filesystem_protocol::fs::req(
                filesystem_protocol::fs::READDIR,
                filesystem_protocol::fs::ROOT,
                0,
            ),
            0,
        );
        // `call` returns the reply word as a `u64`; a negative errno reads as a huge one
        // (`entropy_protocol`'s convention, followed here and by `smb_server.rs`'s own `fs_readdir`).
        // A `READDIR` answers the byte count written (>= 0) or a negative errno. Any non-negative
        // answer is the capability working; this test does not pin the fixture's exact contents,
        // which is a fact about the image and not about this capability.
        if (r0 as i64) >= 0 {
            flags |= F_DIR_WORKS;

            // DECISIONS §117's own proof: which subtree did this login actually land in? Only the
            // `WRITE_MARKER`/`READ_MARKER` do this; `LOGIN` stops at proving what it received.
            match behaviour {
                // **One arm where there were two.** `chris`'s and `corinne`'s halves of this proof
                // differed only in the name written into the marker, and that name is the identity
                // this run authenticated as, which it now holds as an argument rather than as a
                // second copy of the same fact in a second match arm.
                WRITE_MARKER => {
                    if write_marker(dir_ep, identity) {
                        flags |= F_MARKER_WRITTEN;
                    }
                    if is_absent(dir_ep, filesystem_protocol::fixture::tree::INNER) {
                        flags |= F_NOT_SHARED_SUBTREE;
                    }
                }
                READ_MARKER => {
                    if let Some(h) = read_marker(dir_ep) {
                        hint = h;
                    }
                }
                // `budget` is already gone by construction (above); `region` is now the top of
                // `CONSTRUCTION_UT`'s watermark, so this `DESTROY` un-bumps it too, and this
                // login's whole 128-page contribution comes home.
                LOGOUT => {
                    flags |= teardown_directory(dir_ep, region);
                    // The third report word, for this behaviour only: how many microseconds the
                    // caretaker region's `DESTROY` waited (the budget's own wait is not reported;
                    // nothing lives in that region, so it has never needed one). The kernel test
                    // quotes it in the failure message.
                    hint = waited_micros();
                }
                // `HOLD_TERMINAL` shares the teardown but not the wait report (see the module
                // docs: freeing the session's memory and freeing the terminal are two independent
                // acts, and this behaviour deliberately only does the first). It keeps the identity
                // hint every other behaviour puts in the third word, so the wait report stays
                // `LOGOUT`'s alone, exactly as that comment above claims.
                HOLD_TERMINAL => flags |= teardown_directory(dir_ep, region),
                // Torn down as `HOLD_TERMINAL` is, and for a measured reason: left standing, its
                // caretaker lived for the rest of the suite and a later, unrelated test
                // (`timetable_tests`) hung on riscv64 and aarch64 for want of what it held.
                PRESENT_RUN_UNVOUCHED => flags |= teardown_directory(dir_ep, region),
                // Its whole point is that the session comes home once the work is gone.
                PENDING_WORK => flags |= teardown_directory(dir_ep, region),
                _ => {}
            }
        }
    }

    // **`LOGOUT` deliberately does not also free the terminal here.** `login`'s own thread is
    // blocked inside `send(AUDIT, ...)` (a blocking rendezvous) until the *caller* drains it, which
    // happens after this behaviour's report, not before; sending `login_protocol::logout_word` from inside
    // this behaviour and waiting for its answer here would deadlock against that (its own report
    // would never arrive, because `login` cannot get back to `RECV(REQUEST)` to answer the logout
    // until the caller has already drained `AUDIT`, which it does *after* waiting for this report).
    // A caller that also wants the terminal freed does that itself, after draining `AUDIT` --
    // `kernel::user::login_tests::free_terminal`, used exactly this way by
    // `caretaker_teardown_reclaims_a_full_session_worth_of_memory`.
    done(RPT_OK, flags, hint);
}

/// Copy `bytes` into the shared filesystem page (a name to open/create, or data to write).
fn put_page(bytes: &[u8]) {
    for (i, &b) in bytes.iter().enumerate() {
        FS_WINDOW.w8(i as u64, b);
    }
}

/// Read `n` bytes out of the shared filesystem page into `out`.
fn get_page(n: usize, out: &mut [u8]) {
    for (i, b) in out.iter_mut().take(n).enumerate() {
        *b = FS_WINDOW.r8(i as u64);
    }
}

/// `CREATE` [`MARKER_NAME`] under `dir` and `WRITE` `content` into it, then `CLOSE` the handle.
/// `true` only if every step succeeded. Create is create, not create-or-open
/// (`filesystem_protocol::fs::CREATE`'s own contract), so this fails loudly rather than overwriting a marker a
/// previous run left behind, which would silently defeat the isolation proof this behaviour exists
/// for.
fn write_marker(dir: u64, content: &[u8]) -> bool {
    put_page(MARKER_NAME.as_bytes());
    let (h, _) = call(
        dir,
        filesystem_protocol::fs::req(filesystem_protocol::fs::CREATE, 0, MARKER_NAME.len() as u64),
        0,
    );
    if (h as i64) < 0 {
        return false;
    }
    put_page(content);
    let (w, _) = call(
        dir,
        filesystem_protocol::fs::req(filesystem_protocol::fs::WRITE, h, content.len() as u64),
        0,
    );
    let ok = w as i64 == content.len() as i64;
    let _ = call(
        dir,
        filesystem_protocol::fs::req(filesystem_protocol::fs::CLOSE, h, 0),
        0,
    );
    ok
}

/// `true` if `OPEN`ing `name` under `dir` is refused. The expected answer for a name that lives only
/// in the old, shared fixture subtree ([`filesystem_protocol::fixture::tree::INNER`]) when `dir` is a genuinely
/// different, identity-scoped one.
fn is_absent(dir: u64, name: &str) -> bool {
    put_page(name.as_bytes());
    let (r0, _) = call(
        dir,
        filesystem_protocol::fs::req(filesystem_protocol::fs::OPEN, 0, name.len() as u64),
        0,
    );
    (r0 as i64) < 0
}

/// `OPEN` and `READ` [`MARKER_NAME`] under `dir`, packed the same way `login`'s own audit trail
/// packs an identity ([`login_protocol::identity_hint`]), so the kernel test can compare what this run
/// read against what [`write_marker`]'s caller wrote without a second encoding to keep in sync.
/// `None` if the marker could not be opened or read.
fn read_marker(dir: u64) -> Option<u64> {
    put_page(MARKER_NAME.as_bytes());
    let (h, _) = call(
        dir,
        filesystem_protocol::fs::req(filesystem_protocol::fs::OPEN, 0, MARKER_NAME.len() as u64),
        0,
    );
    if (h as i64) < 0 {
        return None;
    }
    let (n, _) = call(
        dir,
        filesystem_protocol::fs::req(filesystem_protocol::fs::READ, h, 16),
        0,
    );
    let _ = call(
        dir,
        filesystem_protocol::fs::req(filesystem_protocol::fs::CLOSE, h, 0),
        0,
    );
    if (n as i64) < 0 {
        return None;
    }
    let mut buf = [0u8; 16];
    get_page((n as usize).min(buf.len()), &mut buf);
    Some(login_protocol::identity_hint(
        &buf[..(n as usize).min(buf.len())],
    ))
}

/// **The fourth capability, `login_protocol`'s own logout ticket: `MemoryRegion::DESTROY` reclaims the
/// caretaker `dir` names.** Sets [`F_TEARDOWN_OK`] on success and, only then, re-checks `dir` with a
/// `READDIR`: [`F_DEAD_AFTER_TEARDOWN`] if it now fails, which is the proof that the capability, not
/// merely the syscall, came down.
fn teardown_directory(dir: u64, region: u64) -> u64 {
    let Some(mut flags) = destroy_with_retry(region).then_some(F_TEARDOWN_OK) else {
        return 0;
    };
    let (r0, _) = call(
        dir,
        filesystem_protocol::fs::req(
            filesystem_protocol::fs::READDIR,
            filesystem_protocol::fs::ROOT,
            0,
        ),
        0,
    );
    if (r0 as i64) < 0 {
        flags |= F_DEAD_AFTER_TEARDOWN;
    }
    flags
}

/// **How long [`destroy_with_retry`] waits for a refusal to clear, in seconds of counter time.**
///
/// A watchdog, deliberately, and not a rate: it is set so far above what the wait actually costs
/// that only a region which will *never* come down can reach it. Measured on 2026-08-27, an
/// eight-core Mac under TCG, ten logouts per run, 70 waits over seven full-suite runs:
///
/// | condition | worst single wait |
/// |---|---|
/// | quiet host, aarch64 | 12.8 ms |
/// | one-minute load average 13 to 29, aarch64 | 44.3 ms |
/// | one-minute load average 27 to 47, riscv64 | **126.4 ms** |
///
/// The tail grows with contention, which is the whole reason this is not a two-digit number: five
/// seconds is 40x the worst of those. **Only a failing run ever pays it**, because the loop returns
/// the moment the region comes down, and the test that drives this stops at its first bad logout, so
/// the worst a wedged caretaker costs is this twice (the budget and the region) against the
/// harness's 90 s per-test ceiling. That is the trade: a red run takes ten seconds to arrive and
/// says which of the two things happened, instead of arriving in eight milliseconds and being wrong.
const DESTROY_WAIT_SECS: u64 = 5;

/// **`MemoryRegion::DESTROY` on `ut`, retried until the region actually becomes destroyable.** Used
/// on both the fourth delegated capability (the caretaker's construction region) and the third (the
/// client's own budget, already held with `WRITE` by every behaviour): `login_protocol`'s own module docs, on
/// the fourth capability, name the one transient refusal the *caretaker's* region can give
/// (mid-`forward` to the file service, blocked on an endpoint the region does not own, at the exact
/// instant `DESTROY` is attempted; that window closes on its own). The budget has no such window:
/// nothing else is ever running in it, so its own `DESTROY` is expected to succeed on the first
/// attempt, and this loop costs it nothing to share.
///
/// # Why this is a clock and not a count of attempts
///
/// It was `for _ in 0..64 { destroy; yield }` until 2026-08-27, which is
/// notes/load-sensitive-assertions.md's family in one line: **a yield count is not a duration.**
/// What the refusal is waiting on is a timer tick. `sched::reap_region_objects` refuses while the
/// caretaker is still live and *arms* DECISIONS §16's kill on it, and the kill lands at that
/// thread's next preemption, so the wait is a tick period (10 ms at 100 Hz) rather than a number of
/// syscalls. A `yield_now` that finds work on this core returns in ~130 us, so sixty-four of them
/// can elapse in 8 ms and give up **before the tick that does the work arrives**; a `yield_now`
/// that finds none parks until the next tick, and then two attempts are enough. Which of the two
/// happens is a scheduling outcome the host decides, which is why the old form passed quiet, passed
/// CI, and failed at about 2x oversubscription. Measured, both shapes, in one run: 23, 36, 2, then
/// 64-and-refused, at 130 to 300 us per attempt.
///
/// So the loop waits on the property (the region genuinely coming down) and bounds itself with
/// [`DESTROY_WAIT_SECS`] of counter time.
///
/// # BUGS
///
/// **The bound is wall clock, which is the unit milestone 62 argues against**, and it is used here
/// because the better one is not reachable from a process. That milestone re-denominated
/// `smp.rs`'s migration drain in *delivered timer ticks* precisely because a counter deadline keeps
/// running while the guest is descheduled; userspace has no delivered-tick counter (`user_mode_runtime::now`
/// is the raw counter and nothing publishes the kernel's per-core tick count), so what makes this
/// safe is the 40x margin above rather than the unit. The margin is what would have to be
/// re-measured if the wait ever grew. Giving a process a delivered-tick reading is an ABI addition
/// and therefore calef's call, not a lane's.
fn destroy_with_retry(ut: u64) -> bool {
    let ceiling = user_mode_runtime::cntfrq().saturating_mul(DESTROY_WAIT_SECS);
    let started = user_mode_runtime::now();
    loop {
        if destroy_region(ut) == 0 {
            record(user_mode_runtime::now().wrapping_sub(started));
            return true;
        }
        if user_mode_runtime::now().wrapping_sub(started) >= ceiling {
            record(user_mode_runtime::now().wrapping_sub(started));
            return false;
        }
        yield_now();
    }
}

/// How long the last [`destroy_with_retry`] waited, in counter ticks. [`LOGOUT`] reports it as
/// microseconds in the third report word (see [`_start`]), so a red run carries the observation the
/// wait actually decided on instead of sending the reader back to re-run under load.
static mut WAITED: u64 = 0;

/// Store [`WAITED`]. Every run of this program is single-threaded and this is its only writer.
fn record(ticks: u64) {
    // SAFETY: one thread, one writer, and the only reader runs after it in the same thread.
    unsafe { WAITED = ticks }
}

/// [`WAITED`] in microseconds, for the report.
fn waited_micros() -> u64 {
    // SAFETY: as `record`.
    let ticks = unsafe { WAITED };
    ticks.saturating_mul(1_000_000) / user_mode_runtime::cntfrq().max(1)
}

fn done(tag: u64, w1: u64, w2: u64) -> ! {
    send(REPORT, tag, w1, w2);
    exit()
}

user_mode_runtime::panic_handler!();
