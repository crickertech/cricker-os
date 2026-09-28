#![no_std]
//! **The wire contract between a client and the login service** (milestone 49's login half).
//!
//! Unix login authenticates and then mutates a global identity field. This contract is the other
//! shape: a client presents an identity and a secret, and on success the service delegates a fresh
//! **capability set** back over the same channel rather than changing anything ambient. See
//! `components/src/login.rs` for the service and notes/login.md for the design.
//!
//! # The exchange
//!
//! Not a `CALL`: a one-shot reply capability carries two words and nothing else (`abi::reply`), and
//! a successful login has to hand back capabilities, which only `abi::rendezvous::SEND_CAP` can do.
//! So this is two persistent endpoints and a fixed message order, the same shape
//! `grant_plan::spawnproto` already uses for exactly this reason (a shell's spawn request that may
//! carry a delegated budget).
//!
//! **Two phases, not one, since milestone 49's channel-per-client update.** `REQUEST`/`RESULT` (the
//! endpoints every client is handed at spawn) are a **front door**, shared by every client this
//! service will ever see, for the width of exactly one message each: a bare [`CONNECT`], answered
//! with a fresh, private [`CONNECTED`] channel. The identity and secret an actual login needs never
//! touch the front door at all, which is what removes the hazard the front door used to carry (see
//! this service's BUGS, "One client at a time", for what that hazard was and why a shared front door
//! carrying only [`CONNECT`] does not reintroduce it):
//!
//! ```text
//!   client --send(REQUEST, connect_word(), 0, 0)------------------------> login
//!   client <----------------- recv(RESULT) -> CONNECTED ---------------- login
//!   client <---- RECV_CAP(RESULT) x 3: priv_request, priv_result, page - login
//!
//!   client --place(), send(priv_request, w0, 0, 0)-----------------------> login
//!   client <----------------- recv(priv_result) -> OK, DENIED or -------- login
//!                                                   NO_TERMINAL
//!   client <---- RECV_CAP(priv_result) x 5, only after OK ---------------  login
//!
//!   client --send(REQUEST, logout_word(), 0, 0)---------------------------> login
//!   client <----------------- recv(RESULT) -> LOGGED_OUT ----------------- login
//! ```
//!
//! [`CONNECT`] carries no page: there is nothing in it a client did not already know, so the front
//! door never maps or reads a shared staging page at all, and two clients racing to connect can only
//! ever contend for two harmless, empty, freshly-minted objects, never for each other's identity or
//! secret. Login answers [`CONNECT`]s one at a time (this process has one thread and no wait-any
//! primitive), but that is service order, not shared state: each answer is a private key handed to
//! exactly one holder before login goes on to serve the actual login it just enabled.
//!
//! **[`LOGOUT`] travels on the front door itself, not on a private channel** (milestone 49's
//! terminal update): unlike [`CONNECT`], it carries no secret and needs no private staging, so
//! there is nothing a shared front door would expose by handling it directly. See
//! [`logout_word`] and this contract's own BUGS for what it does and does not authenticate.
//!
//! On [`OK`], login sends five capabilities over `priv_result`, or six when the reply's second word
//! carries [`RUN_UNVOUCHED_FOLLOWS`], in this order:
//!
//! 1. the **directory** capability: a freshly built `fs_subtree_caretaker`'s endpoint, `WRITE`;
//! 2. the **filesystem's shared page**, a `PageFrame`, `READ | WRITE`: the client maps it itself
//!    (`user_mode_runtime::map_page_frame`) at whatever address it chooses, and uses it for both the request it
//!    stages to the directory endpoint and the caretaker's own hop to the file service, which is
//!    sound for the reason `crates/system_initializer` gives (`fs_subtree_caretaker` and its client
//!    share one frame because every request on both hops is a blocking `CALL`);
//! 3. the **budget**: a `MemoryRegion`, `WRITE | GRANT`, freshly split so the client's memory is
//!    genuinely its own and it may in turn split, spend, or hand pieces of it on. `WRITE` is also
//!    what `MemoryRegion::DESTROY` needs, so this capability doubles as its own reclaim: a client that
//!    calls `DESTROY` on it, alongside the fourth capability below, gives back everything a session
//!    spent rather than only the caretaker's half;
//! 4. the **logout ticket**: a `MemoryRegion`, `WRITE` only, the exact region the directory capability's
//!    caretaker was built from (`components/src/login.rs`'s `mint`, see that program's module docs,
//!    "Reclaiming a session"). It has nothing left to `SPLIT` or `RETYPE` (its whole budget went
//!    into building the caretaker), so its only remaining use is `invoke(cap, abi::memory_region::DESTROY,
//!    0, 0, 0)`, which reclaims the caretaker's TCB, address space and endpoint and returns the
//!    pages to `login`'s own construction budget. **A client should retry `DESTROY` a bounded few
//!    times on refusal rather than treat one attempt as final**: the caretaker can be transiently
//!    mid-request to the file service when a logout arrives, which refuses the very first `DESTROY`
//!    (the same shape `crates/system_initializer::reclaim` already retries for a directory grant's
//!    own caretaker); it never refuses permanently, because the caretaker's own client-facing
//!    endpoint is retyped from this same region, so its steady state (parked in `recv` between
//!    requests) is always reclaimable, never the permanently-blocked case
//!    `notes/hung-component.md` documents as unfixable. Logging out is optional: a client that never
//!    calls `DESTROY` costs `login` exactly what it always cost (see that program's BUGS on
//!    `CONSTRUCTION_UT` exhaustion), this ticket just makes not costing it possible.
//! 5. **the terminal** (milestone 49's terminal update, DECISIONS-recommended "deny cleanly" shape,
//!    `design/roadmap/49-users-and-attribution.md`'s own BUGS): a `Rendezvous`, `WRITE` only, the
//!    same right the interactive boot's shell already holds on it. **Only ever present because it
//!    could be**: `login`'s own `serve_login` refuses with [`NO_TERMINAL`] before authentication is
//!    even attempted while another session already holds it, so every `OK` this contract answers
//!    carries this fifth capability too. There is exactly one physical terminal and exactly one
//!    holder at a time; see this contract's own BUGS and `components/src/login.rs`'s module docs for the
//!    single-session design this is deliberately not more than.
//! 6. **the run-unvouched capability**, only when [`OK`]'s second word says so (DECISIONS §219 (how the shell names an installed program to the spawner)
//!    gate D2, milestone 198 (a package manager) rung 3a): a `Rendezvous`, `WRITE` only, the one the progenitor
//!    receives on. A session that holds it may run bytes nobody vouched for, with only what it
//!    delegates and the clock and configuration pages; a session that was not given it cannot, and
//!    that is how a machine can have users who may not run new native code (calef's consequence in
//!    §219). No `GRANT`, so the session cannot hand it on, and for the same reason it cannot ride a
//!    `SEND_CAP` anywhere: it is used by sending on it (`grant_plan::spawnproto::
//!    RUN_UNVOUCHED_BIT`). `login` gives it only to an identity on the owner's list,
//!    [`RUN_UNVOUCHED_LIST`] (DECISIONS §221 (the boot prompt is the owner's console)), and only
//!    when it holds one itself.
//!
//! **A full logout destroys capability 3 before capability 4, and the order is load-bearing.**
//! `mint()` splits the fourth capability's region from `login`'s own `CONSTRUCTION_UT` first and the
//! third capability's budget second, so the budget sits above the region in `CONSTRUCTION_UT`'s
//! watermark. `crates/regions`' own reclaim only returns a freed child's pages to a reusable state
//! when it is the *top* of its parent's watermark (LIFO, the same rule §16's object revocation and
//! `job_undertaker`'s pool already live under, and DECISIONS §92 already named for a caretaker's own
//! region); destroying the region first, while the budget above it is still alive, still reclaims the
//! caretaker's TCB, address space and endpoint (`DESTROY` still returns success), but leaves its
//! pages a stranded hole that does not come back to `login`'s reusable capacity until
//! `CONSTRUCTION_UT` itself is destroyed. Destroy the budget first, then the region, and both spans
//! return cleanly. **This is a property of this specific pair, not a general promise**: it holds
//! regardless of what any other client does, because nothing else is ever split from
//! `CONSTRUCTION_UT` *between* one login's own two capabilities (`mint()` builds both, back to back,
//! before either is delegated); it does not extend to reclaiming *two different logins'* memory out
//! of the order they were minted in, which needs the same LIFO discipline this tree already accepts
//! elsewhere.
//!
//! On [`DENIED`], [`MALFORMED`] or [`NO_TERMINAL`], nothing follows: the client holds exactly what
//! it held before it asked.
//!
//! # BUGS
//!
//! **Which identities get the run-unvouched capability is a file the owner writes**
//! ([`RUN_UNVOUCHED_LIST`], DECISIONS §221 (the boot prompt is the owner's console), ruling 2).
//! It is read on every login, so an owner's edit holds from the next login on, and it shares the
//! file service's one page with every session: sound only while the terminal rule keeps one
//! session live at a time, which is the assumption every client of that page already makes. No
//! session `login` builds can spawn anything today (none holds a spawn endpoint), so a listed
//! session's capability is delivered and not yet usable; the boot prompt is the only session that
//! presents it. Taking a name off the list holds from that identity's next login; a session
//! already built keeps its copy, because nothing revokes a delegated capability here.
//!
//! **[`LOGOUT`] authenticates nothing.** It is a bare word on the shared front door, deliberately:
//! unlike a login it carries no secret to protect and needs no private channel, but the flip side is
//! that any process holding the front-door request endpoint (every client this service will ever
//! see, by construction)
//! can free the terminal out from under whoever is using it. That is a real, unauthenticated
//! interruption a hostile co-tenant could perform, not merely a discourtesy; it is accepted here
//! because today's actual deployment is one interactive boot with one physical terminal and no
//! untrusted co-tenant reaching this endpoint at all, which is exactly the scope
//! `design/roadmap/49-users-and-attribution.md`'s own terminal BUGS entry names as this slice's
//! bound. A deployment that must defend against a hostile holder of `REQUEST` needs `LOGOUT` to
//! carry proof (the identity that is logging out, or better, a capability only that session holds),
//! which is real work this slice does not build.
//!
//! **A stale terminal handoff has no automatic recovery.** If the process holding the terminal
//! capability exits, crashes, or simply never calls [`logout_word`], `login`'s own `terminal_held`
//! flag stays set forever and every later login is refused `NO_TERMINAL`, indistinguishably from a
//! session that is genuinely still in use. There is no liveness check on the holder (this tree's
//! usual "no wait-any primitive" bound: `login` cannot watch its client and also keep serving new
//! connections), so recovering from an abandoned session today means restarting `login` itself. See
//! `components/src/login.rs`'s own BUGS for the same limitation stated at the component a reader meets
//! first.
//!
//! # The request page
//!
//! Identical in shape to [`credential_protocol`]'s: an identity and a secret, because this service's whole
//! first act is relaying the presented pair to the credential service's own `VERIFY` unchanged. Its
//! `place`/`read`/`req`/`op` do not encode anything specific to that service's own semantics (`op`
//! is a caller-supplied word), so this contract reuses them rather than defining a second copy of
//! the same layout and risking the two drifting the way `credentialer.rs`'s own compile-time
//! assertions exist to catch for `cred`/`credential_protocol`.
//!
//! Name: ratified 2026-08-23 (calef, a kernel-dependency crate naming review). Minted 2026-08-22
//! for milestone 49, following the tree's existing `<subject>_proto` pattern (`credential_proto`,
//! `clock_proto`, `entropy_proto`), which is what those three were called that day.
//!
//! **The suffix became `_protocol` at milestone 265** (calef, 2026-09-05). The stem was left open
//! then, because calef had parked the whole `login` family on 2026-09-14, and 265 accepted that it
//! might cost this crate a second rename. **It will not: calef ruled on 2026-09-15 that the stem stays
//! for the whole family**, so this name is final on both halves. The argument is in
//! `design/naming/vocabulary-rulings.md`, "The `login` stem stays".

pub use credential_protocol::{MAX_IDENTITY, MAX_SECRET, PAGE, op, place, read, wipe};

/// **The front door's only legal request** (milestone 49's channel-per-client update): "give me my
/// own private channel." Carries no lengths and touches no page; build the word with
/// [`connect_word`] rather than [`place`] (there is nothing to stage).
pub const CONNECT: u64 = 2;

/// The one opcode a private, per-client channel accepts. There is only one verb (authenticate), so
/// unlike `credential_protocol` (verify, provision) there is nothing to distinguish it from; it exists so a
/// reader of a captured request word can tell this contract's traffic from any other sharing the
/// encoding. Build a request word with `place(page, identity, secret, LOGIN)`, which returns it
/// already carrying this opcode and the two lengths. Sent on the private `priv_request` endpoint
/// [`CONNECTED`] delegates, never on the front door.
pub const LOGIN: u64 = 1;

/// **Free the terminal** (milestone 49's terminal update): "I am done; the next login may have it."
/// A bare word on the shared front door, like [`CONNECT`], and for the same reason [`CONNECT`]
/// carries no page: there is nothing here a caller did not already know. Build it with
/// [`logout_word`]. See this contract's own BUGS for what `LOGOUT` does not authenticate.
pub const LOGOUT: u64 = 3;
/// **Authenticate, and open this identity's schedule** (milestone 152 (durable delegation),
/// §222 (who holds a user's schedule), calef's L2 ruling of 2026-09-26). Sent on the private
/// channel in place of [`LOGIN`], staged with [`place`] exactly as [`LOGIN`] is. On success `login`
/// builds the session process that holds the identity's timetable, and [`OK`] carries
/// [`SCHEDULE_FOLLOWS`]. A [`LOGIN`] for an identity whose session is already durable reattaches to
/// it and carries [`SCHEDULE_FOLLOWS`] too.
///
/// The ruling said "a new request word after `OK`". It travels as the request itself instead,
/// because `login` blocks on one endpoint at a time and could not wait for a word after `OK`
/// without every existing client sending one. Name and value: provisional.
pub const SCHEDULE: u64 = 7;
/// **The owner's suspended list changed: apply it now** (milestone 152 (durable delegation),
/// calef's §108 (disabling credentials kills the durable session) ruling of 2026-09-26). A bare word on the front door, like [`LOGOUT`], built with
/// [`suspend_word`]. `login` rereads [`SUSPENDED_LIST`] and ends every durable session it keeps for
/// an identity on it, then answers [`APPLIED`] with how many it ended. Anyone holding the front
/// door can send it, which is harmless for the reason [`LOGOUT`] is: it acts only on a list a
/// session cannot write. Name and value: provisional, a wire item.
pub const SUSPEND: u64 = 8;

/// **Report why the start-up pass skipped what it skipped** (milestone 152, added 2026-09-27
/// against CI's own falsification of the "skipped, not failed" contract in `rederive`'s doc: a
/// skip that leaves no reason anywhere is the defect, not the skip). A bare word on the front door,
/// like [`SUSPEND`], built with [`rederive_skips_word`]. Answers [`SKIP_COUNTS`] with
/// [`durable::pack_skip_counts`]'s word on `RESULT`'s second word; the third is always 0, reserved.
/// Unauthenticated, like [`SUSPEND`] and [`LOGOUT`]: it names no identity, only counts. A fielded
/// system has little reason to ask this of its own boot, but the counters cost a saturating add
/// each and are always kept, in production too, so nothing here is test-only. Name and value:
/// provisional.
pub const REDERIVE_SKIPS: u64 = 9;

/// `send(REQUEST, connect_word(), 0, 0)`. The bare word [`CONNECT`] travels as; a client never calls
/// [`place`] for this step, because there is no identity or secret to stage.
pub fn connect_word() -> u64 {
    CONNECT << credential_protocol::OP_SHIFT
}

/// `send(REQUEST, suspend_word(), 0, 0)`. The bare word [`SUSPEND`] travels as.
pub fn suspend_word() -> u64 {
    SUSPEND << credential_protocol::OP_SHIFT
}

/// `send(REQUEST, rederive_skips_word(), 0, 0)`. The bare word [`REDERIVE_SKIPS`] travels as.
pub fn rederive_skips_word() -> u64 {
    REDERIVE_SKIPS << credential_protocol::OP_SHIFT
}

/// `send(REQUEST, logout_word(), 0, 0)`. The bare word [`LOGOUT`] travels as, on the *front door*
/// (unlike [`connect_word`]'s answer, this needs no private channel): see [`LOGOUT`]'s own doc.
pub fn logout_word() -> u64 {
    LOGOUT << credential_protocol::OP_SHIFT
}

/// **A private channel is ready.** Answered on the front door's `RESULT` endpoint, followed by
/// exactly three delegated capabilities, in this order: the private `priv_request` endpoint
/// (`WRITE`), the private `priv_result` endpoint (`READ`), and a page frame (`READ | WRITE`) to stage
/// the actual login on. See the module docs for the two-phase exchange.
pub const CONNECTED: u64 = 4;

/// **Authenticated.** Five capabilities follow on the private result endpoint, or six when the
/// reply's second word carries [`RUN_UNVOUCHED_FOLLOWS`]; see the module docs for the order.
pub const OK: u64 = 1;

/// **A bit of [`OK`]'s second word: the sixth capability, the run-unvouched one, follows**
/// (DECISIONS §219 gate D2). Carried on the reply rather than implied, because a client's sixth
/// `RECV_CAP` must match a sixth `SEND_CAP` exactly: a `login` spawned without the capability
/// (every kernel test harness before this bit) sends five, and a client that always waited for six
/// would block for ever. Name: provisional.
pub const RUN_UNVOUCHED_FOLLOWS: u64 = 1;
/// **A bit of [`OK`]'s second word: the registration page follows** (milestone 152). After the
/// run-unvouched capability when that is announced too, one more `RECV_CAP` delivers a page frame
/// (`WRITE`): the identity's timetable's registration page, `timetable::registration`'s whole
/// protocol. Announced for the reason [`RUN_UNVOUCHED_FOLLOWS`] is. Name: provisional.
pub const SCHEDULE_FOLLOWS: u64 = 2;

/// **The owner's list of identities whose sessions may run unvouched bytes** (DECISIONS §221 (the
/// boot prompt is the owner's console), ruling 2): a file at the root of the file service, beside
/// the identities' own subtrees, which is where this tree already keeps what it knows about an
/// identity (DECISIONS §117 (a principal's subtree is named by its identity string)). A session
/// is confined to its own subtree and cannot name it; the boot prompt, the owner's console, can
/// (`echo chris >> may-run-unvouched`).
///
/// **Empty by default**: no file, an unreadable file, or one larger than a page lists nobody, so
/// the failure is always toward the narrower grant. The format is [`lists`]'s.
///
/// Name: provisional (milestone 198 (a package manager), lane `milestone/198-owner-console`,
/// 2026-09-26).
pub const RUN_UNVOUCHED_LIST: &str = "may-run-unvouched";

/// **Whether `list` names `identity`** ([`RUN_UNVOUCHED_LIST`]'s format): one identity per line,
/// surrounding spaces, tabs and a carriage return ignored, blank lines and lines starting with `#`
/// skipped, and the match exact. `chris` does not list `chr` or `chris2`, and an empty identity
/// is never listed.
///
/// # EXAMPLES
///
/// ```
/// let list = b"# who may run new native code\nchris\n  corinne \r\n";
/// assert!(login_protocol::lists(list, b"chris"));
/// assert!(login_protocol::lists(list, b"corinne"));
/// assert!(!login_protocol::lists(list, b"chr"));
/// assert!(!login_protocol::lists(b"", b"chris"));
/// ```
pub fn lists(list: &[u8], identity: &[u8]) -> bool {
    !identity.is_empty()
        && list
            .split(|&b| b == b'\n')
            .map(|line| line.trim_ascii())
            .filter(|line| !line.is_empty() && !line.starts_with(b"#"))
            .any(|line| line == identity)
}

/// **`list` with `identity` added**, into `out`: `list` unchanged when it already names it, else
/// `list`, a newline if it did not end in one, and `identity` on a line of its own. The byte count,
/// or `None` when `out` is too small or `identity` is not a name a list can hold (empty, or with
/// whitespace, or starting with `#`). What `user suspend <name>` writes (milestone 152 (durable
/// delegation)).
///
/// # EXAMPLES
///
/// ```
/// let mut out = [0u8; 64];
/// let n = login_protocol::with_listed(b"# suspended\nchris", b"corinne", &mut out).unwrap();
/// assert_eq!(&out[..n], b"# suspended\nchris\ncorinne\n");
/// let n = login_protocol::with_listed(b"chris\n", b"chris", &mut out).unwrap();
/// assert_eq!(&out[..n], b"chris\n");
/// assert!(login_protocol::with_listed(b"", b"two words", &mut out).is_none());
/// ```
pub fn with_listed(list: &[u8], identity: &[u8], out: &mut [u8]) -> Option<usize> {
    if !listable(identity) {
        return None;
    }
    if lists(list, identity) {
        out.get_mut(..list.len())?.copy_from_slice(list);
        return Some(list.len());
    }
    let sep = usize::from(!list.is_empty() && !list.ends_with(b"\n"));
    let n = list.len() + sep + identity.len() + 1;
    let dst = out.get_mut(..n)?;
    dst[..list.len()].copy_from_slice(list);
    if sep == 1 {
        dst[list.len()] = b'\n';
    }
    dst[list.len() + sep..n - 1].copy_from_slice(identity);
    dst[n - 1] = b'\n';
    Some(n)
}

/// **`list` with every line naming `identity` removed**, into `out`, other lines and comments kept
/// as they were. What `user resume <name>` writes (milestone 152). `None` when `out` is too small.
///
/// # EXAMPLES
///
/// ```
/// let mut out = [0u8; 64];
/// let n = login_protocol::without_listed(b"# suspended\nchris\n corinne\n", b"corinne", &mut out).unwrap();
/// assert_eq!(&out[..n], b"# suspended\nchris\n");
/// ```
pub fn without_listed(list: &[u8], identity: &[u8], out: &mut [u8]) -> Option<usize> {
    let mut n = 0;
    for line in list.split_inclusive(|&b| b == b'\n') {
        let body = line.strip_suffix(b"\n").unwrap_or(line).trim_ascii();
        if !identity.is_empty() && body == identity {
            continue;
        }
        out.get_mut(n..n + line.len())?.copy_from_slice(line);
        n += line.len();
    }
    Some(n)
}

/// Whether `identity` can be a line of a list: non-empty, no whitespace, not a comment.
fn listable(identity: &[u8]) -> bool {
    !identity.is_empty()
        && !identity.starts_with(b"#")
        && !identity.iter().any(u8::is_ascii_whitespace)
}

/// **Refused.** The identity is unknown, the secret is wrong, the service could not mint a
/// capability set for an otherwise-authenticated principal, or (on the front door) the service could
/// not mint a private channel at all (see this service's BUGS on the second and third cases: both
/// are folded into the same code on purpose, for [`credential_protocol`]'s reason: a caller must not be able
/// to distinguish "wrong password" from "the service is out of memory" by trying the same identity
/// twice and comparing outcomes).
pub const DENIED: u64 = 2;

/// The request word's lengths are out of range, or the front door was sent something other than
/// [`CONNECT`] or [`LOGOUT`]. Not an authentication outcome, and a client that gets this has learned
/// nothing about whether the identity exists.
pub const MALFORMED: u64 = 3;

/// **The terminal is already held by another session** (milestone 49's terminal update, the
/// "deny cleanly" shape). Sent on the private `priv_result` endpoint, *before* the presented
/// identity and secret are even relayed to the credential service: this is global, caller-
/// independent state (there is exactly one physical terminal), not a fact about any identity, so
/// checking it first costs nothing an attacker could turn into a timing oracle and saves a round
/// trip to the credential service on every refusal. A client that gets this has learned nothing
/// about whether its identity or secret were correct.
pub const NO_TERMINAL: u64 = 5;

/// **The terminal is free again.** Sent on the front door's `RESULT` endpoint in answer to
/// [`LOGOUT`]. Idempotent: sent whether or not anything was actually held, since a logout that
/// arrives when nobody holds the terminal is harmless rather than an error.
pub const LOGGED_OUT: u64 = 6;

/// **Authenticated, and suspended** (milestone 152, the §108 ruling of 2026-09-26). The identity
/// and secret were right and the identity is on [`SUSPENDED_LIST`]. Sent only after a successful
/// authentication, so only someone who already holds the secret learns the account is suspended,
/// and learns it plainly rather than as a wrong password. Nothing follows. Name and value:
/// provisional, a wire item.
pub const SUSPENDED: u64 = 7;

/// **[`SUSPEND`]'s answer**, on the front door's `RESULT`; the second word is how many durable
/// sessions were ended. Name and value: provisional.
pub const APPLIED: u64 = 8;

/// **[`REDERIVE_SKIPS`]'s answer**, on the front door's `RESULT`; the second word is
/// [`durable::pack_skip_counts`]'s word. Name and value: provisional.
pub const SKIP_COUNTS: u64 = 9;

/// **The owner's list of suspended identities** (milestone 152, calef's §108 ruling of
/// 2026-09-26), a file at the root of the file service in [`RUN_UNVOUCHED_LIST`]'s place and
/// format, read with [`lists`]. `user suspend <name>` at the owner's console adds a name and
/// `user resume <name>` removes it. `login` refuses a listed identity with [`SUSPENDED`] and ends
/// its durable session; the boot-time re-deriver skips it.
///
/// **Empty by default, and the failure direction is the opposite of the run-unvouched list's**, which
/// is worth saying because it is not a free choice: no file, or one that cannot be read, suspends
/// nobody. Failing toward "suspended" would lock every user out when the file service is slow.
///
/// Name: provisional (milestone 152, 2026-09-26).
pub const SUSPENDED_LIST: &str = "suspended";

/// **One attribution record**, sent once per successful login on the service's own audit endpoint
/// (`components/src/login.rs`'s `AUDIT` slot), so the property DECISIONS §109 names ("a server ... logs
/// which channel a request arrived on") is checkable rather than merely claimed. `w0` is
/// [`ATTRIBUTED`], `w1` is the channel's sequence number (the order this service established
/// channels in, starting at 0), `w2` is [`identity_hint`] of the identity that established it.
///
/// This is **login's own record of what it just established**, not a downstream server logging a
/// later request against that channel; see this service's BUGS for the natural place the second
/// half would live and why nothing in this tree needs it yet.
pub const ATTRIBUTED: u64 = 1;

/// Pack up to the first 8 bytes of `identity` into one `u64`, big-endian, zero-padded. A debugging
/// aid for the audit record, not a general identity encoding: two identities that share an 8-byte
/// prefix are indistinguishable in it, which is fine for this contract (the audit record exists to
/// let a test, or an operator, confirm *which* login produced *which* channel among the identities
/// actually in use, not to serve as a second store of who exists).
pub fn identity_hint(identity: &[u8]) -> u64 {
    let mut buf = [0u8; 8];
    let n = identity.len().min(8);
    buf[..n].copy_from_slice(&identity[..n]);
    u64::from_be_bytes(buf)
}

// ===========================================================================================
// **How the login service is started**, which is a contract between whoever spawns it and the
// program itself rather than between the program and its clients (milestone 233).
//
// It is in this crate because rule 7 leaves nowhere else: three binaries have to agree on these
// two addresses (`components/src/login.rs`, `crates/system_initializer`, and the kernel's own test
// harness in `kernel/src/user/login_service.rs`), and what two binaries agree on is a crate. The
// siting is the honest weak point: this crate's own first line calls itself "the wire contract
// between a client and the login service", and a spawn contract is neither wire nor client. The
// alternative was a crate holding two constants. Provisional, like every name a lane mints.
// ===========================================================================================

/// **Where `fs_subtree_caretaker`'s ELF bytes are mapped, read-only, before `login`'s `_start`
/// runs**, with the length in `x0`/`a0`/`rdi` (milestone 233).
///
/// **This replaced a mapping of the whole initrd archive, and the reason is not economy.** `login`
/// used to read the archive at `user_mode_runtime::initrd::INITRD_VA` and index it by name, which is what the
/// kernel's own test harness handed it and what nothing else ever did: `crates/system_initializer`
/// spawns this program through `supervision_protocol::build_child`, which can map only pages the
/// spawner holds a `PageFrame` capability for, and the archive is reserved RAM the frame allocator
/// does not own and no capability names. So the real interactive boot started `login` with no
/// archive at all and it died at `_start` on every boot for an unknown length of time
/// (design/roadmap/233-login-never-runs.md).
///
/// The fix could have gone the other way, and giving `login` the archive is the option that was
/// refused: it needs one program's bytes and a manifest to check them against, and a service that
/// can read every file in the boot image to answer a password holds authority it never exercises.
/// A blob costs the spawner **no capability-table slots at all** (`supervision_protocol`'s own
/// `fill_and_map` holds one frame at a time and deletes it), which is what let this land against
/// the 21-of-24 peak `kernel::cap::CAPABILITY_TABLE_PEAK_MEASURED` records.
///
/// Zero length means the spawner had no vouched-for caretaker to hand over. That is not a failure
/// to start: `login` comes up and answers every login [`DENIED`], which is exactly what
/// `crates/system_initializer` already does with a program it cannot measure.
pub const CARETAKER_ELF_VA: u64 = 0x0000_0000_0100_0000;

/// **Where `measured_boot::PROGRAM_MEASUREMENTS`' bytes are mapped, read-only, before `login`'s
/// `_start` runs**, with the length in `x1`/`a1`/`rsi` (milestone 233).
///
/// Four megabytes above [`CARETAKER_ELF_VA`] so a caretaker image would have to grow forty-fold
/// before the two could meet. Both sit well below `user_mode_runtime::initrd::INITRD_VA` and well above the
/// per-channel scratch VAs `login` bump-allocates, which is the only other thing in that address
/// space that grows.
pub const PROGRAM_MEASUREMENTS_VA: u64 = 0x0000_0000_0140_0000;

/// **Where `session`'s image is mapped, read-only, before `login`'s `_start` runs** (milestone 152),
/// with `timetable`'s at [`TIMETABLE_ELF_VA`] and both lengths in the third argument register
/// ([`schedule_lengths`]). They are the two programs a durable session is built from, and each is
/// checked against the table at [`PROGRAM_MEASUREMENTS_VA`] before anything is built from it. No
/// program a job runs travels with them: a job runs what the live activation generation names
/// (Fork 8 ruled D by calef on 2026-09-27, on #1377). Zero lengths mean no schedule can be opened
/// on this boot: [`SCHEDULE`] is then answered as [`LOGIN`] is, without [`SCHEDULE_FOLLOWS`].
///
/// Name: provisional, milestone 152's lane, 2026-09-27; it replaces a schedule archive that also
/// carried the jobs.
pub const SESSION_ELF_VA: u64 = 0x0000_0000_0180_0000;

/// **Where `timetable`'s image is mapped**, beside [`SESSION_ELF_VA`], four megabytes above it.
/// Provisional.
pub const TIMETABLE_ELF_VA: u64 = 0x0000_0000_01c0_0000;

/// **The third start argument: `session`'s length in the low half, `timetable`'s in the high.**
/// Two lengths in one register, because the first two carry the caretaker's and the table's. An
/// image of 4 GiB or more cannot be mapped at these addresses anyway. Provisional.
///
/// ```
/// use login_protocol::{schedule_lengths, split_schedule_lengths};
/// assert_eq!(split_schedule_lengths(schedule_lengths(40_960, 409_600)), (40_960, 409_600));
/// assert_eq!(split_schedule_lengths(0), (0, 0));
/// ```
pub const fn schedule_lengths(session: u64, timetable: u64) -> u64 {
    (session & 0xffff_ffff) | (timetable << 32)
}

/// [`schedule_lengths`]' inverse: `(session, timetable)`.
pub const fn split_schedule_lengths(word: u64) -> (u64, u64) {
    (word & 0xffff_ffff, word >> 32)
}

/// **`login`'s slot for its durable sessions' file-service channel**: one page of the file
/// service's window [`DURABLE_WINDOW`], `WRITE | GRANT`, placed by the spawner. A durable session's
/// timetable reads the store through it while its user may be using the store through window 0 at
/// the prompt, and two clients staging bytes in one page overwrite each other (milestone 599 (a
/// frame per filesystem client channel)). `login` badges the file service's endpoint with the
/// window's number itself. Provisional.
pub const DURABLE_WINDOW_SLOT: u64 = 8;

/// **The file service's window the spawner reserves for `login`'s durable sessions**: the last. The
/// progenitor hands jobs behind a directory grant the others, and the kernel harness claims this
/// one for `login`. One window, so one durable session at a time reads the store
/// ([`durable::sessions_held`] is bounded by it too). Provisional.
pub const DURABLE_WINDOW: u64 = filesystem_protocol::fs::CLIENT_WINDOWS as u64 - 1;

/// **How `login` starts a session process** (milestone 152, S1 of 2026-09-26), in this crate for
/// the reason the two constants above are: `components/src/login.rs` and
/// `components/src/session.rs` both read it. Every name here is provisional.
pub mod session {
    /// Slot 0: the endpoint the session process reports readiness on, once, `WRITE`.
    pub const READY_SLOT: u64 = 0;
    /// Slot 1: the region the timetable and every job it fires are built from, `WRITE | GRANT`.
    pub const BUDGET_SLOT: u64 = 1;
    /// Slot 2: the registration page, `WRITE`, which the session maps into its timetable.
    pub const PAGE_SLOT: u64 = 2;
    /// Slot 3: an endpoint to a caretaker serving the store's `activation/` read-only, `WRITE`,
    /// which the session hands its timetable (Fork 8 D).
    pub const ACTIVATION_SLOT: u64 = 3;
    /// Slot 4: likewise for `packages/`.
    pub const PACKAGES_SLOT: u64 = 4;
    /// Slot 5: the page of the durable window the two caretakers stage through, `WRITE`, which the
    /// session maps into its timetable at `timetable::contract::STORE_PAGE_VA`.
    pub const STORE_PAGE_SLOT: u64 = 5;
    /// Where `timetable`'s image is copied into the session process; its length is `a0`.
    pub const TIMETABLE_VA: u64 = 0x0000_0000_0200_0000;
    /// The readiness word: the timetable is built, started, and watching the page.
    pub const READY: u64 = 0x5e55_0000_0000_0001;
    /// The last word, on the same endpoint: the timetable is gone and everything built from the
    /// session process's budget is given back. `login` takes it before reclaiming the process, so
    /// the reclaim never lands mid-teardown and strands a region under the user's budget.
    pub const STOPPED: u64 = 0x5e55_0000_0000_0002;
    /// The failure word; the low byte says which step.
    pub const FAILED: u64 = 0x5e55_0000_0000_0f00;
}

/// **How many durable sessions `login` can keep, and which ones it re-derives at start-up**
/// (milestone 152 (durable delegation), Fork 7 ruled A by calef on 2026-09-27: `login` re-derives
/// every durable session that is not suspended, before it serves its front door). Pure logic, in
/// this crate so a host test reaches it; `components/src/login.rs` is the one caller.
///
/// The slot counts are from `login.rs`'s code, counted in
/// `notes/durable-delegation/boot-rederivation-in-login.md` (question 5). They are counts, not a
/// measurement: no gauge reports one process's table.
///
/// Name: provisional, minted 2026-09-27 (UTC) by milestone 152's lane, for this module and
/// everything in it.
pub mod durable {
    /// Slots `login` holds at rest: ten endowed (`REQUEST`, `RESULT`, `VERIFY`, `FS_EP`,
    /// `FS_PAGE_FRAME`, `CONSTRUCTION_UT`, `AUDIT`, `TERM_EP`, `RUN_UNVOUCHED` and
    /// [`super::DURABLE_WINDOW_SLOT`]) and the three budgets `_start` splits (`own_ut`,
    /// `durable_ut`, `channel_ut`).
    pub const SLOTS_AT_REST: u64 = 13;
    /// Slots an ordinary login adds at its peak, inside `mint`'s `build_child`: the channel's
    /// `result` and `region`, then `region`, `narrow_ep`, `ready`, and the child's address space
    /// and one frame or its thread.
    pub const SLOTS_LOGIN_PEAK: u64 = 7;
    /// Slots each kept durable session holds: the user's budget, the session process's region, the
    /// registration page and the readiness endpoint.
    pub const SLOTS_PER_SESSION: u64 = 4;

    /// **A signed-in client's own spending budget**, in pages, split for every login and delegated
    /// to it. In this crate because `login`, the kernel harness and the progenitor all size budgets
    /// from it and [`BUDGET_PAGES`]. Provisional.
    pub const CLIENT_BUDGET_PAGES: u64 = 64;
    /// **A durable session process's region**: the process, its copy of `timetable`'s image, and
    /// the two store caretakers `login` builds in it (Fork 8 D), 64 pages each as `login` sizes a
    /// caretaker's region. Measured for the process on aarch64's debug build on 2026-09-26 at 192
    /// with room to spare. Provisional.
    pub const SESSION_REGION_PAGES: u64 = 192 + 2 * 64;
    /// **What a durable session process builds its timetable from**: the timetable's region (240)
    /// and its jobs' budget (128), with room for two endpoints. Provisional.
    pub const SESSION_BUDGET_PAGES: u64 = 400;
    /// **One durable session's whole budget**: the client's own, the session process's region, and
    /// what it builds from. `login` splits this many pages per durable session, and its spawner
    /// sizes `login`'s construction budget with it. Provisional.
    pub const BUDGET_PAGES: u64 = CLIENT_BUDGET_PAGES + SESSION_REGION_PAGES + SESSION_BUDGET_PAGES;

    /// **How many durable sessions fit a `table_slots`-slot capability table** with room left for
    /// one ordinary login beside them: the largest `n` with
    /// `SLOTS_AT_REST + SLOTS_PER_SESSION * n + SLOTS_LOGIN_PEAK <= table_slots`. Opening the last
    /// of them at a login peaks at the same count and no higher, because `login` builds that
    /// session's two store caretakers (Fork 8 D) before anything else of the session and mints the
    /// client's own caretaker after it: 16 held at the open, 23 building the second caretaker, 24
    /// building the session process. At start-up it peaks lower. Counted from the code on
    /// 2026-09-27, not measured.
    ///
    /// # EXAMPLES
    ///
    /// ```
    /// use login_protocol::durable::sessions_held;
    /// assert_eq!(sessions_held(24), 1); // today's table: one session, 24 of 24 at a login's peak
    /// assert_eq!(sessions_held(32), 3); // the table PR #1360 proposes
    /// assert_eq!(sessions_held(16), 0); // too small for any
    /// ```
    pub const fn sessions_held(table_slots: u64) -> usize {
        let fixed = SLOTS_AT_REST + SLOTS_LOGIN_PEAK;
        if table_slots < fixed {
            return 0;
        }
        ((table_slots - fixed) / SLOTS_PER_SESSION) as usize
    }

    /// **The identities a start-up pass re-derives, in manifest order**: every entry of the
    /// durable-session manifest (DECISIONS §125 (which identities have pending work)) that the
    /// owner's suspended list (`super::lists`'s format, [`super::SUSPENDED_LIST`]) does not name.
    /// The caller stops at its own capacity and skips an identity whose stored schedule is missing
    /// or empty, which is a stale manifest line rather than a failure (§125).
    ///
    /// # EXAMPLES
    ///
    /// ```
    /// let manifest: [&[u8]; 3] = [b"chris", b"corinne", b"graeme"];
    /// let mut left = login_protocol::durable::to_rederive(&manifest, b"# suspended\ncorinne\n");
    /// assert_eq!(left.next(), Some(&b"chris"[..]));
    /// assert_eq!(left.next(), Some(&b"graeme"[..]));
    /// assert_eq!(left.next(), None);
    /// ```
    pub fn to_rederive<'a>(
        manifest: &'a [&'a [u8]],
        suspended: &'a [u8],
    ) -> impl Iterator<Item = &'a [u8]> + 'a {
        manifest
            .iter()
            .copied()
            .filter(move |identity| !super::lists(suspended, identity))
    }

    /// **Why the start-up pass skipped a manifest entry** (milestone 152, added 2026-09-27 against
    /// CI's own falsification of `rederive`'s "skipped, not failed" contract with no reason
    /// anywhere to read). One variant per `continue`/`break` site in `components/src/login.rs`'s
    /// `rederive`. Provisional: this lane's, not an architect's.
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    #[repr(u8)]
    pub enum RederiveSkip {
        /// No free slot: as many sessions are already kept as this process can hold.
        TableFull = 0,
        /// The name does not fit `filesystem_protocol::grant::MAX_NAME`, or this identity already
        /// has a session kept.
        Identity = 1,
        /// The stored schedule is missing or empty: a stale manifest line.
        NoStoredSchedule = 2,
        /// Splitting the session's budget off `durable_ut` failed: out of pages.
        BudgetOutOfPages = 3,
        /// Opening the session process (its capability table, its timetable, or the wait for its
        /// readiness word) failed.
        SessionBuildFailed = 4,
    }

    /// How many [`RederiveSkip`] variants there are, and the length [`pack_skip_counts`] and
    /// [`unpack_skip_counts`] agree on.
    pub const REDERIVE_SKIP_REASONS: usize = 5;

    /// **Pack one count per [`RederiveSkip`] reason into one word**, one byte each, saturating at
    /// 255 (ample: `components/src/login.rs`'s `Durables` holds at most a handful of sessions and a boot's
    /// manifest is not expected to grow past that). `counts[RederiveSkip::X as usize]` lands in
    /// byte `X`. The whole word travels as `REDERIVE_SKIPS`'s `RESULT` second word.
    pub const fn pack_skip_counts(counts: &[u32; REDERIVE_SKIP_REASONS]) -> u64 {
        let mut word = 0u64;
        let mut i = 0;
        while i < REDERIVE_SKIP_REASONS {
            let byte = if counts[i] > 255 { 255 } else { counts[i] };
            word |= (byte as u64) << (i * 8);
            i += 1;
        }
        word
    }

    /// The reverse of [`pack_skip_counts`], for a test to read the reply back.
    pub const fn unpack_skip_counts(word: u64) -> [u32; REDERIVE_SKIP_REASONS] {
        let mut counts = [0u32; REDERIVE_SKIP_REASONS];
        let mut i = 0;
        while i < REDERIVE_SKIP_REASONS {
            counts[i] = ((word >> (i * 8)) & 0xff) as u32;
            i += 1;
        }
        counts
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        /// **The capacity is the note's arithmetic, at both table sizes in play.** 24 slots hold
        /// one session and 28 would be needed for two, since the durable window's page is held at
        /// rest (Fork 8 D); 32 (PR #1360) holds three. Falsified by dropping `SLOTS_LOGIN_PEAK`
        /// from the sum, which answers two for 24.
        #[test]
        fn the_capacity_leaves_room_for_one_login_beside_the_sessions() {
            assert_eq!(sessions_held(24), 1);
            assert_eq!(sessions_held(27), 1);
            assert_eq!(sessions_held(28), 2);
            assert_eq!(sessions_held(32), 3);
            assert_eq!(sessions_held(19), 0);
            assert_eq!(sessions_held(0), 0);
            for slots in 0..64u64 {
                let n = sessions_held(slots) as u64;
                if n > 0 {
                    assert!(
                        SLOTS_AT_REST + SLOTS_PER_SESSION * n + SLOTS_LOGIN_PEAK <= slots,
                        "{n} sessions overflow a {slots}-slot table at a login's peak"
                    );
                }
                assert!(
                    SLOTS_AT_REST + SLOTS_PER_SESSION * (n + 1) + SLOTS_LOGIN_PEAK > slots,
                    "a {slots}-slot table could hold {} sessions, not {n}",
                    n + 1
                );
            }
        }

        /// **A suspended identity is never re-derived, and nothing else is dropped.** The proof
        /// `session_reviver`'s skip carried, now on the function `login`'s start-up pass calls.
        #[test]
        fn a_suspended_identity_is_skipped_and_the_rest_keep_their_order() {
            let manifest: [&[u8]; 3] = [b"chris", b"corinne", b"graeme"];
            let all: [&[u8]; 3] = manifest;
            assert!(to_rederive(&manifest, b"").eq(all.iter().copied()));
            assert!(to_rederive(&manifest, b"chris\ngraeme\n").eq([&b"corinne"[..]]));
            // A prefix is not a name, and a comment suspends nobody.
            assert!(to_rederive(&manifest, b"chr\n#corinne\n").eq(all.iter().copied()));
            assert_eq!(to_rederive(&[], b"chris\n").count(), 0);
        }

        /// **Skip counts round-trip through one word, and a wild count saturates rather than
        /// overflows into its neighbour's byte.**
        #[test]
        fn skip_counts_round_trip_and_saturate() {
            let counts = [1, 0, 3, 0, 2];
            assert_eq!(unpack_skip_counts(pack_skip_counts(&counts)), counts);
            let wild = [300, 0, 0, 0, 0];
            let word = pack_skip_counts(&wild);
            assert_eq!(unpack_skip_counts(word), [255, 0, 0, 0, 0]);
            // A saturated first byte does not bleed into the second.
            assert_eq!(word & 0xff00, 0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **The list is exact, and every doubtful case lists nobody** (DECISIONS §221 ruling 2).
    /// Falsified once: matching by prefix (`line.starts_with(identity)`) listed `chr` here.
    #[test]
    fn the_run_unvouched_list_names_whole_identities_only() {
        let list = b"# owner-written\nchris\n\n\tcorinne\r\n#graeme\n";
        assert!(lists(list, b"chris"));
        assert!(lists(list, b"corinne"));
        for not in [&b"chr"[..], b"chris2", b"graeme", b"#graeme", b"", b" "] {
            assert!(
                !lists(list, not),
                "{:?} was listed",
                core::str::from_utf8(not)
            );
        }
        assert!(!lists(b"", b"chris"), "an empty list named somebody");
    }

    /// **What `user suspend` and `user resume` write is what `login` reads** (milestone 152
    /// (durable delegation)): a name added is listed, added twice is listed once, removed is not
    /// listed, and a name the list cannot hold is refused rather than written.
    #[test]
    fn the_suspended_list_round_trips_through_its_two_edits() {
        let mut out = [0u8; 128];
        let n = with_listed(b"# owner's list\nchris", b"corinne", &mut out).unwrap();
        let list = out;
        assert!(lists(&list[..n], b"corinne") && lists(&list[..n], b"chris"));
        let again = with_listed(&list[..n], b"corinne", &mut out).unwrap();
        assert_eq!(again, n, "adding a listed name changed the list");
        let n2 = without_listed(&list[..n], b"corinne", &mut out).unwrap();
        assert!(!lists(&out[..n2], b"corinne") && lists(&out[..n2], b"chris"));
        assert!(
            out[..n2].starts_with(b"# owner's list\n"),
            "the comment was lost"
        );
        for bad in [&b""[..], b"two words", b"#chris"] {
            assert!(with_listed(b"", bad, &mut out).is_none(), "{bad:?}");
        }
        assert!(with_listed(b"chris\n", b"corinne", &mut [0u8; 4]).is_none());
        assert!(without_listed(b"chris\n", b"corinne", &mut [0u8; 2]).is_none());
        assert_eq!(with_listed(b"", b"chris", &mut out), Some(6));
        assert_eq!(op(suspend_word()), SUSPEND);
        assert_eq!(op(logout_word()), LOGOUT);
        assert_eq!(op(connect_word()), CONNECT);
    }

    #[test]
    fn identity_hint_round_trips_short_names() {
        assert_eq!(identity_hint(b"chris"), identity_hint(b"chris"));
        assert_ne!(identity_hint(b"chris"), identity_hint(b"corinne"));
    }

    #[test]
    fn req_and_place_agree_with_cred_proto_on_the_shape() {
        let mut page = [0u8; PAGE];
        let w0 = place(&mut page, b"chris", b"secret", LOGIN).expect("fits");
        assert_eq!(op(w0), LOGIN);
        let (id, secret) = read(&page, w0).expect("well-formed");
        assert_eq!(id, b"chris");
        assert_eq!(secret, b"secret");
    }

    #[test]
    fn connect_word_carries_no_lengths_and_reads_back_as_connect() {
        let w0 = connect_word();
        assert_eq!(op(w0), CONNECT);
        // Unlike a LOGIN word, there is nothing else packed into it: the low 32 bits (where `place`
        // packs the two lengths) are zero.
        assert_eq!(w0 & 0xffff_ffff, 0);
        // And it is distinguishable from LOGIN's own opcode, so a front door that only expects
        // CONNECT can refuse a stray LOGIN word rather than mistake it for one.
        assert_ne!(CONNECT, LOGIN);
    }

    #[test]
    fn logout_word_carries_no_lengths_and_reads_back_as_logout() {
        let w0 = logout_word();
        assert_eq!(op(w0), LOGOUT);
        assert_eq!(w0 & 0xffff_ffff, 0);
        // Distinguishable from both existing front-door/private-channel opcodes, so a front door
        // that dispatches on `op(w0)` can never confuse the three.
        assert_ne!(LOGOUT, CONNECT);
        assert_ne!(LOGOUT, LOGIN);
    }

    #[test]
    fn every_wire_code_this_contract_defines_is_distinct() {
        // The two namespaces (a request's opcode, and a private channel's OK/DENIED/... verdict)
        // are read from different fields by different code and are allowed to share numbers; this
        // checks each namespace is internally distinct, which is the property a dispatch `match`
        // actually relies on.
        let request_ops = [LOGIN, CONNECT, LOGOUT];
        for (i, a) in request_ops.iter().enumerate() {
            for b in &request_ops[i + 1..] {
                assert_ne!(a, b, "two request opcodes collide");
            }
        }
        let verdicts = [OK, DENIED, MALFORMED, CONNECTED, NO_TERMINAL, LOGGED_OUT];
        for (i, a) in verdicts.iter().enumerate() {
            for b in &verdicts[i + 1..] {
                assert_ne!(a, b, "two verdict codes collide");
            }
        }
    }
}
