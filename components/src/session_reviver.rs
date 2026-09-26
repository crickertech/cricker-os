//! **The boot-time re-deriver** (milestone 152's third piece, provisional name; DECISIONS §123): a
//! `root_supervisor`-shaped boot-only process that re-establishes durable-session authority fresh at
//! boot, the way a login would, with no live person presenting a credential at that moment.
//!
//! # What it is given, and what it does with it
//!
//! Two capabilities and nothing else: a construction budget ([`UT`], a `MemoryRegion`) and read access
//! to the durable schedule store ([`FS_EP`], the principal tree's root directory capability, the
//! same unnarrowed shape `login.rs` and `identity_provisioner.rs` already hold theirs in). It:
//!
//! 1. Reads [`schedule_store::MANIFEST_FILE_NAME`] at the store's root and parses it
//!    ([`schedule_store::parse_manifest`]): the hard-wired set of identities that currently have a
//!    durable session with pending scheduled work (DECISIONS §125, this lane's own answer to the gap
//!    neither §122 nor §123 fully specified).
//! 2. For every identity the manifest names, reads that identity's own
//!    [`schedule_store::SCHEDULE_FILE_NAME`] file (DECISIONS §117's per-identity subtree, §122's
//!    format) and parses it with [`timetable::parse`] unchanged, proving §122's write path (this
//!    lane's own `fixtures/src/fs_test_client.rs::ROLE_SCHEDULE_SEED`) and this read path agree on the
//!    same bytes through a real filesystem round trip, not a fixture.
//! 3. Mints and tears down one synthetic per-identity session in two steps (split a session region
//!    off [`UT`], split a pending-job region off *that*, prove `MemoryRegion::DESTROY` on the session
//!    refuses while the job lives and succeeds once it does not): the demonstration that boot-time
//!    re-derivation produces something with the identical §16 (object revocation) lifecycle a live login session has,
//!    without wiring a real registrar (#387, out of this lane's scope; see the roadmap doc). The
//!    live-login half is `kernel::user::login_tests`'s
//!    `a_login_session_with_pending_work_refuses_logout_until_the_work_is_gone`, on `login`'s own
//!    delegated budget. It replaced `smb_server.rs`'s `DurableSession`, which this shape was first
//!    written against and which went with the SMB code on 2026-08-30.
//! 4. Deletes its own copies of [`FS_EP`] and [`UT`] (`cap_delete`, not `MemoryRegion::DESTROY`: this
//!    process is giving up its own *name* for the store and the budget, not tearing either down),
//!    then attempts a further store read and a further construction and asserts both now fail,
//!    reporting the proof over [`REPORT`] rather than merely asserting it in a comment
//!    (`root_supervisor.rs`'s own idiom, lines 168-182, applied here to a directory endpoint and an
//!    `MemoryRegion` instead of `ROOT_UT` alone).
//!
//! # Why this is a boot-only process rather than a phase of an existing one
//!
//! DECISIONS §123 leaves "a new dedicated process or a phase folded into `root_supervisor` or
//! `system_initializer`" explicitly open as "a smaller, more reversible question this decision does
//! not need to settle." This lane picked a **new, dedicated process**, for the reason
//! `AGENTS.md`'s "recommend on reversible forks" latitude exists to cover: a new binary is the
//! smaller and more reversible change, here, of the two. Folding this into `system_initializer`
//! would mean growing `crates/system_initializer::boot` (already the kernel's largest single
//! function) with a second privileged phase that runs once and then must itself prove its
//! capabilities are gone, which is real surgery on a component every boot depends on; a new process
//! wired the way `identity_provisioner` and `login` already are (spawned directly, holding exactly
//! two capabilities, nothing that names any other component) touches nothing else in the tree. If
//! this is folded into an existing boot component later, that is a smaller, later change than
//! un-folding one would have been.
//!
//! # What this lane proves, and what it deliberately does not
//!
//! **Proves**: the store write path (§122, this lane's own `ROLE_SCHEDULE_SEED`) and the store
//! read-at-boot path connect through a real filesystem round trip; the manifest (§125) answers "which
//! identities" without enumeration (every `OPENDIR`/`OPEN` this process ever issues names something
//! it already learned from a document it holds a capability to read, never a `READDIR`); a
//! boot-time-re-derived session has the same §16 lifecycle a live login's does; and the re-deriver's
//! own capabilities are provably gone after its one pass, in `root_supervisor`'s exact shape.
//!
//! **Does not build**: a real scheduled-job registrar against a durable session (#387/milestone 129 (scheduled execution),
//! this lane's own explicit non-goal); real per-identity narrowing of [`FS_EP`] (a caretaker built
//! per identity, `login.rs`'s `mint()` shape, rather than the one shared, unnarrowed capability this
//! process holds for its whole run) -- named honestly in this program's own BUGS below, matching
//! `identity_provisioner.rs`'s own recorded bound for the identical gap; the liveness watchdog
//! DECISIONS §123's hardening addendum names and explicitly declines to design (refinement 4); and
//! wiring this process into a real boot (`crates/system_initializer::boot` or an interactive
//! `cargo xtask swish-check`) rather than the kernel test harness that spawns it here
//! (`kernel/src/user/session_reviver_service.rs`), the same "not wired into the interactive boot"
//! bound `login.rs`'s own BUGS names for itself.
//!
//! # Capability contract
//! - slot [`REPORT`]: `WRITE`. One report, then this process exits; it does not loop.
//! - slot [`UT`]: `WRITE` on a fresh `MemoryRegion`, the construction budget every synthetic per-identity
//!   session (step 3 above) is `MemoryRegion::SPLIT` from, sized by whoever spawns this process the way
//!   `login.rs`'s own `mint()` sizes a caretaker's construction budget (DECISIONS §123: "a
//!   construction budget... sized for however many durable sessions the store names").
//! - slot [`FS_EP`]: `WRITE` on the principal tree's root directory capability, the store-read
//!   capability DECISIONS §123 names as this process's other grant. Unnarrowed in this slice's own
//!   wiring; see this program's BUGS.
//! - mapped [`FS_VA`]: one page shared with the FS server, the channel every request in this
//!   contract stages a name into or reads a result out of. One page is enough: every read this
//!   process ever performs (the manifest, one identity's `schedule` file) is well under
//!   `filesystem_protocol::PAGE` bytes, unlike `fs_test_client.rs`'s own channel, which maps the whole
//!   `fs::TRANSFER_MAX` width for its throughput role.
//!
//! Name: provisional, and ruled: calef ruled **`session_rederiver`** on 2026-09-13, working the
//! unratified worklist. The block stays `provisional` because the ratified name is not this file's
//! until the rename is performed, and until then `session_reviver` belongs on the worklist rather
//! than off it. Minted by milestone 152's lane 2026-08-24.
//!
//! **The old name asserted the thing §123 exists to deny, and the block that defended it quoted
//! the denial.** It read: a noun for what it does, *"revive, in the sense DECISIONS §123's own
//! roadmap doc used first: `boot-time bring-up is re-derivation, not restoration`"*. To revive is
//! to bring back to life, which is restoration; `re-derivation` is §122's and §123's established
//! word, and this program's own first line already calls it **the boot-time re-deriver**. So the
//! name contradicted the header above it and the decision beneath it at once.
//!
//! What the program does supports the distinction rather than softening it: it reads the manifest,
//! reads each identity's schedule file, and **mints a fresh synthetic session** with the identical
//! §16 lifecycle a live login session has. Nothing is brought back. Something
//! equivalent is derived again, from durable inputs, with nobody present to hand over a credential.
//!
//! Refused `session_reviver`, above. It was never argued for: §123's own text floated *"something
//! like `session_reviver`"* as a placeholder and this lane took it rather than inventing a second
//! one, which is the right instinct and is why nobody noticed the contradiction for three weeks.
//! Refused `durable_session_rederiver` as more precise about which sessions than it needs to be,
//! since these are the only ones. Refused `boot_session_deriver`, which drops the `re-`, the half
//! carrying the meaning: this happens *again*, from inputs that outlived the last time.
//!
//! # BUGS
//!
//! **[`FS_EP`] is unnarrowed for this process's whole run, identically to `identity_provisioner.rs`'s
//! own recorded bound for the same capability.** DECISIONS §123's first hardening refinement asks for
//! the window to shrink *per identity* using `MemoryRegion::SPLIT`, and this process does that for the
//! *construction budget* (step 3: one session's worth of [`UT`] is split off, used, and destroyed
//! before the next identity's), but not for the *directory* capability: every `OPENDIR`/`OPEN` this
//! process issues goes through the one [`FS_EP`] it was granted at spawn, held for the whole pass
//! rather than narrowed to one identity's own subtree via a fresh `fs_subtree_caretaker` (`login.rs`'s
//! `mint()` shape) and torn down before the next identity. Building that would need this process to
//! also hold the caretaker's own ELF bytes and a measured-boot check on them, which is real additional
//! machinery this lane judged out of proportion to what it demonstrates: this process already never
//! *requests* more than `filesystem_protocol::dir::READ | filesystem_protocol::dir::DESCEND` on any
//! `OPENDIR` (never `ENUMERATE`, so it cannot list what it was not told), so the residual risk this
//! leaves is a compromised re-deriver reading (not writing, not enumerating) every identity's schedule
//! rather than only the one currently being processed. A real deployment building on this shape should
//! close that gap; this lane records it rather than building it, matching the disclosure `login.rs` and
//! `identity_provisioner.rs` already carry for the identical bound.
//!
//! **The manifest is trusted by construction, not verified against a signature or a measured-boot
//! table.** DECISIONS §123's second hardening refinement (gate the re-deriver's own binary through
//! measured boot before it is granted anything) is built, but only for *this process's own bytes*
//! (`kernel/src/user/session_reviver_service.rs` checks them before spawning, mirroring
//! `crates/system_initializer::boot`'s own discipline for every component it spawns). The manifest
//! and schedule *documents themselves* carry no equivalent of measured boot: their trust rests on the
//! roadmap doc's own argument ("the durable store itself only ever having been written by an
//! already-authenticated action"), which this lane's demonstration writer does not itself
//! authenticate (`ROLE_SCHEDULE_SEED` is a kernel-test fixture, not a real registrar). A real
//! deployment's trust chain for the store's own contents is #387's problem, not this lane's.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

use filesystem_protocol::{dir, fs};
use supervision_protocol::{memory_region_destroy, memory_region_split};
use user_mode_runtime::mapped_window::MappedWindow;
use user_mode_runtime::{call, cap_delete, retype_page_frame, send};

/// The report endpoint, `WRITE`. One report, then this process exits.
const REPORT: u64 = 0;
/// The construction budget every synthetic per-identity session is split from, `WRITE`.
const UT: u64 = 1;
/// The principal tree's root directory capability: the store-read capability, `WRITE`.
const FS_EP: u64 = 2;
/// The page shared with the FS server, in the `0x...00eN_0000` family `credentialer.rs`,
/// `login.rs` and `identity_provisioner.rs` already use, one past `identity_provisioner.rs`'s
/// highest (`FS_VA`, `0xe5_0000`).
const FS_VA: u64 = 0x0000_0000_00e6_0000;
// SAFETY: the wiring maps one page read/write at FS_VA before this process runs, shared with the
// FS server and with nothing else.
const FS_WINDOW: MappedWindow =
    unsafe { MappedWindow::new(FS_VA, filesystem_protocol::PAGE as u64) };

/// Each synthetic session's own construction, `MemoryRegion::SPLIT` off [`UT`]. One page is enough:
/// nothing is ever retyped from it.
const SESSION_UT_PAGES: u64 = 4;
/// The synthetic pending-job child of one synthetic session, the same size as
/// `login_test_client`'s `PENDING_JOB_PAGES` for the same reason: nothing is ever retyped from it.
const JOB_UT_PAGES: u64 = 1;

/// **Success.** Word 1 of the report is how many identities the manifest named and this process
/// re-derived (proved the §16 lifecycle for) before deleting its own capabilities.
pub const OK: u64 = 1;

/// The manifest itself could not be read or did not parse. Word 1 is the stage (see [`done`]'s
/// callers below for what each one means).
pub const FAILED: u64 = 2;

/// Scratch page buffers for the manifest and one identity's `schedule` document, in `.bss` rather
/// than on the stack: this process's stack is small (`session_reviver_service::revive`'s own
/// spawn, no extra pages granted), and the manifest buffer stays alive for the whole per-identity
/// loop in [`_start`] (`manifest.entries()` borrows it) while [`rederive_one`] needs a second,
/// simultaneously-live page for the identity's own document, which the two together do not fit.
/// `fixtures/src/fs_test_client.rs`'s `PAGE_BUF_A`/`PAGE_BUF_B` are the identical fix for the identical
/// mistake, found by the same `script/test` run that caught this one.
static mut MANIFEST_BUF: [u8; filesystem_protocol::PAGE] = [0; filesystem_protocol::PAGE];
static mut DOC_BUF: [u8; filesystem_protocol::PAGE] = [0; filesystem_protocol::PAGE];

/// One thread per address space (DECISIONS §33), so there is no concurrent access.
fn manifest_buf() -> &'static mut [u8; filesystem_protocol::PAGE] {
    let p = &raw mut MANIFEST_BUF;
    // SAFETY: see above.
    unsafe { &mut *p }
}
/// Same reasoning as [`manifest_buf`].
fn doc_buf() -> &'static mut [u8; filesystem_protocol::PAGE] {
    let p = &raw mut DOC_BUF;
    // SAFETY: see `manifest_buf`.
    unsafe { &mut *p }
}

#[unsafe(no_mangle)]
pub extern "C" fn _start(_a0: u64, _a1: u64, _a2: u64) -> ! {
    let Some(manifest_len) = read_root_file(schedule_store::MANIFEST_FILE_NAME.as_bytes()) else {
        done(0x10); // no manifest at the store's root: nothing to re-derive, which is a wiring
        // question for whoever spawned this process (an empty manifest is not this: see below)
    };
    // Copied out of FS_VA immediately: every further FS request this process makes overwrites the
    // same shared page, and `Manifest` below borrows this buffer, not FS_VA.
    let manifest_buf = manifest_buf();
    manifest_buf[..manifest_len].copy_from_slice(&fs_page()[..manifest_len]);

    let Ok(text) = core::str::from_utf8(&manifest_buf[..manifest_len]) else {
        done(0x11); // not UTF-8: refuse rather than guess, matching `login.rs`'s own posture for a
        // measurement table that fails to decode
    };
    // The manifest itself does not parse; see `schedule_store::Error`.
    let Ok(manifest) = schedule_store::parse_manifest(text) else {
        done(0x12)
    };

    let mut rederived = 0u64;
    for identity in manifest.entries() {
        if !rederive_one(identity) {
            done(0x20 + rederived.min(0x0F)); // which identity (bounded) this process got to
        }
        rederived += 1;
    }

    // **The scoping mechanism**: delete this process's own name for the store-read capability and
    // the construction budget (DECISIONS §123's own words). Not `MemoryRegion::DESTROY` on either: this
    // process is giving up its own copy, not tearing down objects other capabilities may still
    // name (there are none here, but the distinction is the same one `login.rs`'s own module docs
    // draw between `cap_delete` and `MemoryRegion::DESTROY`).
    cap_delete(FS_EP);
    cap_delete(UT);

    // And prove it, `root_supervisor`'s own idiom (lines 168-182): attempt the operations the
    // deleted capabilities would have permitted, and assert both now fail. `NoSuchSlot` is what the
    // kernel returns to any invocation against a deleted capability, unconditionally, whether the
    // slot held a `MemoryRegion` or a directory endpoint.
    let (store_try, _) = call(FS_EP, fs::req(fs::OPEN, fs::ROOT, 1), 0);
    let store_gone = (store_try as i64) < 0;
    // `UT` names an empty slot by this point, so this either faults on nothing (the kernel checks
    // the slot first) or returns a negative code; it cannot build anything, because there is
    // nothing left to build from.
    let ut_try = retype_page_frame(UT);
    let ut_gone = ut_try < 0;

    send(REPORT, OK, rederived, (store_gone && ut_gone) as u64);
    user_mode_runtime::exit()
}

/// **Re-derive one identity's session**, standing in for what a real registrar (#387) would build:
/// read and parse that identity's `schedule` file through a real filesystem round trip, then mint
/// and tear down a synthetic session in the shape a login session's budget has, proving the §16
/// lifecycle a boot-time-re-derived session has is the identical one a live login's already has.
///
/// `false` on any failure (the identity's subtree or schedule file could not be read or parsed, or
/// the §16 property itself did not hold), which the caller turns into a distinct, bounded [`done`]
/// stage rather than silently skipping an identity the manifest named.
fn rederive_one(identity: &[u8]) -> bool {
    // Descend by name, never enumerate: this process was never granted `dir::ENUMERATE`-worthy
    // intent and never asks for it, matching this program's own BUGS on the residual risk that
    // leaves. `dir::READ | dir::DESCEND` is exactly what `smb_proto`'s own read-only share requests
    // (`FsShare::descend_rights`, the read-only arm) for the identical reason: the smallest rights
    // this process will ever use.
    put_page(identity);
    let (dir, _) = call(
        FS_EP,
        fs::req(fs::OPENDIR, fs::ROOT, identity.len() as u64),
        dir::READ | dir::DESCEND,
    );
    if (dir as i64) < 0 {
        return false;
    }

    let Some(doc_len) = read_under(dir, schedule_store::SCHEDULE_FILE_NAME.as_bytes()) else {
        let _ = call(FS_EP, fs::req(fs::CLOSE, dir, 0), 0);
        return false;
    };
    let doc_buf = doc_buf();
    doc_buf[..doc_len].copy_from_slice(&fs_page()[..doc_len]);
    let _ = call(FS_EP, fs::req(fs::CLOSE, dir, 0), 0);

    let Ok(text) = core::str::from_utf8(&doc_buf[..doc_len]) else {
        return false;
    };
    // **The connective proof this lane exists to make**: the bytes `ROLE_SCHEDULE_SEED` wrote
    // through `filesystem_protocol::fs::WRITE` parse, unchanged, with the exact parser
    // `components/src/timetable.rs` would use for the compile-time document. DECISIONS §122's whole
    // recommendation was that reusing this format costs "the two IPC call sites... a page of client
    // code each, not a new subsystem"; this is where that claim is either true or is not.
    if timetable::parse(text).is_err() {
        return false;
    }

    // The synthetic per-identity session: the same proof `login_tests` makes on a live login's
    // budget (a pending-job child holds the session up), mirrored here, because a session re-derived
    // at boot is supposed to have the identical §16 lifecycle a live login's already does, and this
    // is how that claim is checked rather than assumed.
    let Ok(session) = memory_region_split(UT, SESSION_UT_PAGES) else {
        return false;
    };
    let Ok(job) = memory_region_split(session, JOB_UT_PAGES) else {
        return false;
    };
    if memory_region_destroy(session) {
        // A parent with a live child must refuse `MemoryRegion::DESTROY` (DECISIONS §16). If this
        // succeeded, the property this whole design leans on does not hold for a boot-derived
        // session, which is worth failing loudly on rather than continuing to the next identity.
        return false;
    }
    if !memory_region_destroy(job) {
        return false;
    }
    // And the other half: once the child is gone, the session is destroyable again, which is what
    // returns this identity's slice of [`UT`] to reusable capacity before the next identity's turn
    // (DECISIONS §123's first hardening refinement: shrink the window per identity).
    memory_region_destroy(session)
}

/// Read a name directly under the store's root ([`fs::ROOT`]) into the shared page, returning how
/// many bytes landed there. `None` on any refusal (open or read).
fn read_root_file(name: &[u8]) -> Option<usize> {
    put_page(name);
    let (h, _) = call(FS_EP, fs::req(fs::OPEN, fs::ROOT, name.len() as u64), 0);
    if (h as i64) < 0 {
        return None;
    }
    let n = read_all(h);
    let _ = call(FS_EP, fs::req(fs::CLOSE, h, 0), 0);
    n
}

/// Read a name under directory handle `dir` into the shared page, returning how many bytes landed
/// there. `None` on any refusal (open or read); the caller owns closing `dir`.
fn read_under(dir: u64, name: &[u8]) -> Option<usize> {
    put_page(name);
    let (h, _) = call(FS_EP, fs::req(fs::OPEN, dir, name.len() as u64), 0);
    if (h as i64) < 0 {
        return None;
    }
    let n = read_all(h);
    let _ = call(FS_EP, fs::req(fs::CLOSE, h, 0), 0);
    n
}

/// Read the whole of an already-open handle into the shared page, one `filesystem_protocol::PAGE`-sized
/// request (every document this process ever reads, §125's manifest and one `timetable::parse`
/// document, is well under that by construction: [`schedule_store::MAX_IDENTITIES`] short names, or
/// [`timetable::MAX_ENTRIES`] short lines). `None` on a refusal; `Some(0)` is an empty file, which
/// the caller's own parse turns into an honest "nothing here" rather than a special case here.
fn read_all(handle: u64) -> Option<usize> {
    let (n, _) = call(
        FS_EP,
        fs::req(fs::READ, handle, filesystem_protocol::PAGE as u64),
        0,
    );
    if (n as i64) < 0 {
        return None;
    }
    Some((n as usize).min(filesystem_protocol::PAGE))
}

/// Copy `bytes` to the start of the shared page (a name to resolve).
fn put_page(bytes: &[u8]) {
    let p = fs_page_mut();
    p[..bytes.len()].copy_from_slice(bytes);
}

/// The shared page, immutable: what a completed read landed there.
fn fs_page() -> &'static [u8] {
    // SAFETY: forwarded from FS_WINDOW's own contract. One thread per address space (DECISIONS
    // §33), so there is no concurrent writer.
    unsafe { FS_WINDOW.as_slice() }
}

/// The shared page, mutable: where a name to resolve is staged.
fn fs_page_mut() -> &'static mut [u8] {
    // SAFETY: as `fs_page`'s.
    unsafe { FS_WINDOW.as_mut_slice() }
}

/// Report [`FAILED`] with `stage` as its detail word, and stop. One-shot roles must exit, not spin
/// (the pattern every one-shot program in `components/` follows).
fn done(stage: u64) -> ! {
    send(REPORT, FAILED, stage, 0);
    user_mode_runtime::exit()
}

user_mode_runtime::panic_handler!();
