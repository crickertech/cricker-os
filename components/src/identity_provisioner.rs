//! **A provisioning tool: create an identity and its home subtree together** (milestone 155,
//! design/roadmap/155-user-provisioning.md; DECISIONS §117).
//!
//! Unix's `useradd`, one level down: this process is spawned once per new identity by whoever
//! already holds the two administrative capabilities that authority requires, presents the
//! identity and secret it was handed, and does two things as one act rather than two: `PUT`s them
//! into the credential store over [`credential_protocol::provision::PUT`], and `MKDIR`s that identity's home
//! subtree, named by the identity string itself (DECISIONS §117), under whatever directory
//! capability holds the principal tree.
//!
//! # Who may run this, and how (the milestone's own first question)
//!
//! **Answered by precedent, not invented here.** This tree's whole model is that authority is a
//! capability held, never a checked identity: `credentialer.rs`'s provision endpoint has no
//! authentication of its own, because by the time a caller holds it there is nothing left to check
//! (the receive end that would refuse anybody is the one thing the seal deletes). This process
//! follows the same shape. It performs **no authentication of its caller** and needs none: "who may
//! run this tool" is answered completely by "whoever is handed [`PROV`] and [`FS_EP`] at spawn",
//! exactly as `login.rs`'s BUGS name "who is granted these capabilities" as a boot-wiring question
//! rather than a runtime check, and exactly as `credentialer_test_client.rs`'s provisioner role is
//! trusted by construction rather than by a password. A real deployment's answer to "how does an
//! operator come to hold these two capabilities" is the same open question `login.rs`'s BUGS already
//! name for "not wired into the interactive boot": real work this slice does not attempt.
//!
//! # Ordering: the subtree first, and why
//!
//! **These are two independent calls to two different servers, not one transaction**, and this
//! system has no cross-server commit protocol to make them one (building one would be a real
//! architectural undertaking, not a detail of this tool). So a failure between them is possible, and
//! the milestone's own text asks which partial state is acceptable.
//!
//! **The subtree is created first.** If [`filesystem_protocol::fs::MKDIR`] fails, nothing has been written to
//! the credential store: the failure is a clean no-op, and the caller can simply try again. If the
//! subtree succeeds but [`credential_protocol::provision::PUT`] then fails (the store is
//! [`credential_protocol::FULL`], the store has [`credential_protocol::NO_ENTROPY`] to draw a salt from, or the
//! identity is already there), the result is an **orphaned, empty subtree with no live identity**.
//! That is the safe direction: nobody can authenticate as an identity whose credential was never
//! stored, so the orphaned subtree is inert, and it is recoverable by re-running this tool (see
//! below). The alternative order risks the opposite state, a credential that verifies for an
//! identity with no home, which is dangerous rather than merely untidy: once a caller wires
//! per-identity subtree lookup (DECISIONS §117's own follow-on), a login against that identity would
//! authenticate and then have nowhere defined to go. This tree's standing posture is to refuse
//! toward safety on every ambiguity it can (`credential_protocol::NO_ENTROPY` over a weak salt,
//! [`credential_protocol::MISMATCH`] over a distinguishable miss); this ordering is that same posture applied
//! to a two-step provisioning act.
//!
//! **A pre-existing subtree is not treated as failure.** `MKDIR` answers [`EEXIST`] when the name is
//! already there (`filesystem_protocol::fs::MKDIR`'s own contract; "create is create, not create-or-open"), and
//! this process treats that one errno as "already created, proceed" rather than as a refusal: it is
//! exactly the state a previous, partially-failed run would leave, and re-running this tool is the
//! prescribed recovery. Every other `MKDIR` failure ([`filesystem_protocol::dir::EPERM`], a name that does not
//! fit, no `CREATE` right on the parent) stops the request before the credential store is touched
//! at all.
//!
//! # Capability contract
//!
//! - slot [`PROV`]: `WRITE` on the credential service's provision endpoint
//!   (`components/src/credentialer.rs`), before its seal. Never the verify endpoint; this process could
//!   not authenticate anybody even by mistake, because it never holds that capability.
//! - slot [`FS_EP`]: `WRITE` on the directory capability that holds the principal tree. Unnarrowed
//!   in this slice's own wiring, the same "not wired into the interactive boot" bound `login.rs`
//!   names for its own fixed subtree grant: a real deployment scopes this to a dedicated parent
//!   directory once one exists, which is follow-on and not guessed at here.
//! - slot [`REPORT`]: `WRITE`. One report, then this process exits; it does not loop.
//! - mapped [`REQ_VA`]: the page the caller (kernel test harness today; a real operator's shell,
//!   eventually) stages the identity and secret into, in [`credential_protocol::place`]'s exact layout. This
//!   process never chooses this content; it only relays it.
//! - mapped [`PROV_VA`]: the page shared with the credential service, at the address
//!   `components/src/credentialer.rs`'s own `PROV_VA` names. Must be the same physical frame; the wiring
//!   guarantees this the way `login_service.rs` guarantees it for `CRED_VA`.
//! - mapped [`FS_VA`]: the page shared with the file service, for the one `MKDIR` request.
//! - `a0`: the identity's length in [`REQ_VA`]. `a1`: the secret's length. Both bounded the way
//!   [`credential_protocol::read`] bounds them; a length outside that range is refused before anything is
//!   sent anywhere.
//!
//! # What this does not do (see the milestone's own "what it does not decide")
//!
//! No [`credential_protocol::provision::SEAL`]. Sealing ends provisioning for the *whole store*, forever, and
//! is a decision an operator makes once after every identity it wants is in, not a thing one
//! identity's provisioning should trigger as a side effect. This tool's caller seals when it is
//! done, exactly as `credentialer_test_client.rs`'s provisioner role already does in its own tests.
//!
//! No deprovisioning. Named as out of scope by the milestone's own text and sequenced against
//! `login.rs`'s reclamation bound instead of invented here.
//!
//! Name: ratified 2026-09-13 (calef, working the unratified worklist). Minted with milestone 155.
//! Qualifier plus agent noun, the shape ratified nine times that day, and the qualifier is not
//! decoration: `provisioner` alone is already spent as a role name inside
//! `credentialer_test_client.rs`.
//!
//! **`identity` is the load-bearing half, and it is the same argument that gave `user_mode_` its
//! prefix the same day.** In this tree `user` means a person about 342 times, so it is overloaded
//! against the privilege level; `identity` is the word the capability model actually manipulates,
//! and this program is where that shows: `credential_protocol::provision::PUT` takes an identity, and
//! DECISIONS §117 names the home subtree by the identity string itself. Where the tree means the
//! person's record it says identity, and this name keeps that consistent.
//!
//! Refused `useradd`, Unix's name for the shape: not a term of art this tree has adopted, it would
//! import the `user` ambiguity ruled against the same day, and it promises a command a person
//! types where this is spawned by a holder of two administrative capabilities. Refused
//! `provisioner`, spent and generic. Refused `identity_creator`, narrower than the act: the home
//! subtree is the other half and doing both as one act is the milestone's point. Refused
//! `account_provisioner`, which introduces a third word for what `identity` already names.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

use filesystem_protocol::fs;
use user_mode_runtime::call;
use user_mode_runtime::mapped_window::MappedWindow;

/// The credential service's provision endpoint (slot 0), `WRITE`, before its seal.
const PROV: u64 = 0;
/// The directory capability holding the principal tree (slot 1), `WRITE`.
const FS_EP: u64 = 1;
/// The report endpoint (slot 2), `WRITE`.
const REPORT: u64 = 2;

/// Where the caller stages the identity and secret, in [`credential_protocol::place`]'s layout. Numbered in
/// the `0x...00eN_0000` family `credentialer.rs` and `login.rs` already use, one past `login.rs`'s
/// highest (`CRED_VA`, `0xe3_0000`).
const REQ_VA: u64 = address_space_map::pair_page(0x0000_0000_00e4_0000);
// SAFETY: the wiring maps one page read/write at REQ_VA before this process runs (milestone 139
// round 6).
const REQ_WINDOW: MappedWindow =
    unsafe { MappedWindow::new(REQ_VA, credential_protocol::PAGE as u64) };
/// The page shared with the credential service. Must match `components/src/credentialer.rs`'s `PROV_VA`.
const PROV_VA: u64 = address_space_map::pair_page(0x0000_0000_00e0_0000);
// SAFETY: the wiring maps one page read/write at PROV_VA before this process runs, the same
// physical frame `credentialer.rs` maps at its own PROV_VA.
const PROV_WINDOW: MappedWindow =
    unsafe { MappedWindow::new(PROV_VA, credential_protocol::PAGE as u64) };
/// The page shared with the file service, for the one `MKDIR` request this process ever sends.
const FS_VA: u64 = address_space_map::pair_page(0x0000_0000_00e5_0000);
// SAFETY: the wiring maps one page read/write at FS_VA before this process runs, shared with the
// file service and with nothing else.
const FS_WINDOW: MappedWindow =
    unsafe { MappedWindow::new(FS_VA, filesystem_protocol::PAGE as u64) };

/// **Success**: the subtree exists (freshly made or already there) and the credential was stored.
pub const RPT_OK: u64 = 1;
/// **The staged request was not one this contract allows**: an identity or secret length outside
/// [`credential_protocol`]'s bounds. Nothing was sent to either server.
pub const RPT_MALFORMED: u64 = 2;
/// **`MKDIR` failed for a reason other than the name already being there.** Word 1 of the report is
/// the errno. Nothing was written to the credential store.
pub const RPT_SUBTREE_FAILED: u64 = 3;
/// **The subtree exists, but the credential `PUT` was refused.** Word 1 of the report is the raw
/// [`credential_protocol`] verdict ([`credential_protocol::FULL`], [`credential_protocol::MALFORMED`] on a duplicate identity,
/// or [`credential_protocol::NO_ENTROPY`]), or the raw kernel word if the provision endpoint could not be
/// reached at all. This is the accepted partial-failure state; see this program's module docs.
pub const RPT_CRED_FAILED: u64 = 4;

/// `EEXIST`: the standard value (matching `redoxfs`/`syscall`'s own, which `redoxfs_server` returns and
/// nothing in `filesystem_protocol` re-exports under a name). The one `MKDIR` failure this process does not
/// treat as a refusal.
const EEXIST: i32 = 17;

#[unsafe(no_mangle)]
pub extern "C" fn _start(id_len: u64, secret_len: u64, _a2: u64) -> ! {
    // SAFETY: forwarded from REQ_WINDOW's own contract; staged by the caller with the identity and
    // secret this run provisions.
    let req_page = unsafe { REQ_WINDOW.as_slice() };
    let w0 = credential_protocol::req(
        credential_protocol::provision::PUT,
        id_len as usize,
        secret_len as usize,
    );
    let Some((identity, secret)) = credential_protocol::read(req_page, w0) else {
        wipe_req();
        report(RPT_MALFORMED, 0);
    };

    // **The subtree first.** See the module docs: this ordering is what keeps a partial failure
    // inert (an unreachable subtree) rather than dangerous (a live identity with no home).
    //
    // A `None` here, or `Some(EEXIST)`, both fall through to the credential half: `EEXIST` means
    // the subtree is already there, from this run or a previous attempt, which is recovery rather
    // than a reason to refuse provisioning the identity that owns it.
    if let Some(errno) = mkdir_home(identity)
        && errno != EEXIST
    {
        wipe_req();
        report(RPT_SUBTREE_FAILED, errno as i64 as u64);
    }

    // SAFETY: forwarded from PROV_WINDOW's own contract.
    let prov_page = unsafe { PROV_WINDOW.as_mut_slice() };
    let Some(cw0) = credential_protocol::place(
        prov_page,
        identity,
        secret,
        credential_protocol::provision::PUT,
    ) else {
        // Unreachable in practice: `identity` and `secret` already round-tripped `credential_protocol::read`
        // with the same bounds `place` enforces. Refused rather than assumed, because a caller bug
        // here would otherwise be a silent no-op rather than a wrong-looking but honest report.
        wipe_req();
        report(RPT_MALFORMED, 0);
    };
    // `identity` and `secret` were read from REQ_VA and are done being read: `identity` was also
    // used by `mkdir_home` above, and both have now been copied to PROV_VA.
    wipe_req();
    let (cr0, _) = call(PROV, cw0, 0);
    credential_protocol::wipe(prov_page);

    match credential_protocol::code(cr0) {
        Some(credential_protocol::OK) => report(RPT_OK, 0),
        Some(other) => report(RPT_CRED_FAILED, other),
        // A kernel error (no such capability, the endpoint is dead) reads as a huge u64; carried
        // through rather than collapsed, so the report is still diagnosable from one word.
        None => report(RPT_CRED_FAILED, cr0),
    }
}

/// `MKDIR` the identity's home subtree under [`FS_EP`]'s root, asking for full rights on it
/// ([`filesystem_protocol::dir::ALL`]), the same rights `login.rs` requests when it descends into the fixed
/// fixture subtree today. `None` on success (the handle this process minted is closed immediately;
/// it has no further use for it), `Some(errno)` otherwise.
fn mkdir_home(identity: &[u8]) -> Option<i32> {
    // SAFETY: forwarded from FS_WINDOW's own contract.
    let page = unsafe { FS_WINDOW.as_mut_slice() };
    let n = identity.len().min(page.len());
    page[..n].copy_from_slice(&identity[..n]);
    let (r0, _) = call(
        FS_EP,
        fs::req(fs::MKDIR, fs::ROOT, identity.len() as u64),
        filesystem_protocol::dir::ALL,
    );
    match filesystem_protocol::reply_errno(r0 as i64) {
        Some(errno) => Some(errno),
        None => {
            // A directory handle, minted and immediately released: this process confirms the
            // subtree exists and has no further use for a handle to it. Best-effort; a close that
            // failed would leak a handle-table slot in the file service, not corrupt anything a
            // caller of this tool could observe.
            let _ = call(FS_EP, fs::req(fs::CLOSE, r0, 0), 0);
            None
        }
    }
}

/// Zero the staged request. The caller must not reuse [`REQ_VA`] after this returns; this process
/// is the only one that ever maps it.
fn wipe_req() {
    // SAFETY: forwarded from REQ_WINDOW's own contract, and this process is the only writer.
    let page = unsafe { REQ_WINDOW.as_mut_slice() };
    credential_protocol::wipe(page);
}

fn report(code: u64, detail: u64) -> ! {
    user_mode_runtime::send(REPORT, code, detail, 0);
    user_mode_runtime::exit()
}

user_mode_runtime::panic_handler!();
