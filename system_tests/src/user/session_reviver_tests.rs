use fs_service as fss;
use session_reviver_service as srs;

use super::*;

/// The name `redoxfs_server`'s image travels under; present only when `test` built it.
fn redoxfs_server_image() -> &'static [u8] {
    program("redoxfs_server").expect("no redoxfs_server program in the initrd archive")
}

/// `fixtures/src/fs_test_client.rs`'s own `ROLE_SCHEDULE_SEED`, matched by number the way every other
/// FS-service seed roles already were: the role
/// numbers are bare integers both sides agree on by comment, not by a shared crate, following that
/// same file's own convention.
const ROLE_SCHEDULE_SEED: u64 = 11;

/// The construction budget `session_reviver` spends re-deriving this suite's one identity:
/// `SESSION_UT_PAGES + JOB_UT_PAGES` (`components/src/session_reviver.rs`, 4 + 1) with margin for the
/// page-table cost `MemoryRegion::SPLIT` itself pays.
const REVIVER_BUDGET_PAGES: u64 = 32;

/// Extra stack pages both `fs_test_client` roles this suite spawns need beyond `spawn_fs_client`'s
/// own one-page default (`fs_service::CLIENT_EXTRA_STACK`'s own reasoning): both
/// `ROLE_SCHEDULE_SEED` and `ROLE_SCHEDULE_VERIFY` were found short by `script/test`'s own aarch64
/// run (a data abort at the stack's guard page) even after their own page-sized scratch buffers
/// moved to `.bss` (`PAGE_BUF_A`/`PAGE_BUF_B` in `fixtures/src/fs_test_client.rs`); this margin covers
/// the ordinary call-frame depth the rest of each role's body carries.
const SCHEDULE_ROLE_EXTRA_STACK: usize = 2;

/// **Seed the durable schedule store (this lane's own `ROLE_SCHEDULE_SEED`) and run
/// `session_reviver` against it, once per boot.**
///
/// `None` when no RedoxFS disk is attached (`fs_service::root_directory`'s own `None`), which every
/// test below folds into `crate::testing::skip!`, matching every other FS-backed suite in this
/// tree. Memoized the way `identity_provisioning_tests::wired` is, for the identical reason: this
/// suite's tests each want to assert a different property of the **same** run, not re-run the seed
/// and the re-deriver once per assertion.
fn wired() -> Option<[u64; 3]> {
    use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    static DONE: AtomicBool = AtomicBool::new(false);
    static OK: AtomicBool = AtomicBool::new(false);
    #[allow(clippy::declare_interior_mutable_const)]
    const ZERO: AtomicU64 = AtomicU64::new(0);
    static SAVED: [AtomicU64; 3] = [ZERO; 3];

    if !DONE.load(Ordering::Acquire) {
        let result = (|| {
            let client =
                program("fs_test_client").expect("no fs_test_client program in the initrd archive");
            // `root_directory` wires the service (or reuses this boot's) and drains both readiness
            // sentinels itself, so the spawn right after it is safe the moment it returns
            // (`fs_service::wait_for_caretaker`'s own reasoning, one level up: no caretaker here,
            // but the identical ordering hazard against a client that could exist before the
            // service is fully up). Called directly, rather than through `start`, because `start`
            // hardcodes a one-page stack and both `fs_test_client` roles this suite spawns need
            // more (see `SCHEDULE_ROLE_EXTRA_STACK`).
            let (fs_ep, fs_page_frame) =
                fss::root_directory(fss::blk_server_image(), redoxfs_server_image())?;
            let seed_report = fss::spawn_fs_client(
                client,
                fs_ep,
                fs_page_frame,
                ROLE_SCHEDULE_SEED,
                0,
                0,
                SCHEDULE_ROLE_EXTRA_STACK,
            );
            let seed = crate::sched::ipc_recv(seed_report);
            assert_eq!(
                seed[0],
                filesystem_protocol::fixture::SUCCESS,
                "the schedule-store seed (ROLE_SCHEDULE_SEED) did not report success; word 1 \
                 carries the stage/errno `fail`'s own encoding packs (see \
                 `fixtures/src/fs_test_client.rs`'s `fail`): {seed:?}",
            );

            // The same store, read the other way: `session_reviver` gets its own grant of the
            // identical `(fs_ep, fs_page_frame)` pair the seed just wrote through, matching
            // `login.rs`/`identity_provisioner.rs`'s own unnarrowed grant for the identical
            // directory.
            let revived = srs::revive(fs_ep, fs_page_frame, REVIVER_BUDGET_PAGES).expect(
                "session_reviver was either not packed into the archive or refused by its own \
                 measured-boot check (DECISIONS §123's second hardening refinement); both are \
                 build problems, not a property this suite is testing",
            );
            Some(revived)
        })();

        if let Some(w) = &result {
            for (slot, v) in SAVED.iter().zip(w.iter()) {
                slot.store(*v, Ordering::Relaxed);
            }
            OK.store(true, Ordering::Release);
        }
        DONE.store(true, Ordering::Release);
    }
    if !OK.load(Ordering::Acquire) {
        return None;
    }
    Some([
        SAVED[0].load(Ordering::Relaxed),
        SAVED[1].load(Ordering::Relaxed),
        SAVED[2].load(Ordering::Relaxed),
    ])
}

/// **The headline: what `ROLE_SCHEDULE_SEED` wrote through `filesystem_protocol`,
/// `session_reviver` reads back, parses with the real `timetable::parse`, re-derives a session for
/// in the durable-session shape milestone 152 first built for the SMB adapter, and then provably
/// relinquishes.**
///
/// Three properties, checked against `session_reviver`'s own report (`components/src/session_reviver.rs`'s
/// `_start`, its final `send`), none of which the seed's own report or a passing build could fake:
///
/// 1. **`RPT_OK`**, not `RPT_FAILED`: the manifest at the store's own root parsed, named this
///    suite's one identity, and that identity's own `schedule` file parsed with the unmodified
///    `timetable::parse`, exactly §122's recommendation that reusing the format costs "a page of
///    client code each, not a new subsystem".
/// 2. **Exactly one identity re-derived**: the manifest (§125) named precisely what the seed wrote,
///    which is the connective proof between the write path and the read-at-boot path this lane
///    exists to make, not a fixed constant either side merely repeats.
/// 3. **The deletion proof holds**: after its one pass, `session_reviver`'s attempt to read the
///    store again and its attempt to build anything from its construction budget again both failed,
///    which is DECISIONS §123's whole scoping mechanism ("local capability deletion... `NoSuchSlot`
///    is what the kernel returns to any invocation attempt against a deleted capability,
///    unconditionally") demonstrated rather than merely asserted in a comment.
#[test_case]
fn the_schedule_store_write_path_and_the_boot_time_re_deriver_agree() {
    if fs_service::fs_server_image().is_none() {
        crate::testing::skip!(fs_service::NO_FS_SERVER);
    }
    let Some(w) = wired() else {
        crate::testing::skip!("no RedoxFS disk attached");
    };
    assert_eq!(
        w[0],
        srs::RPT_OK,
        "session_reviver reported RPT_FAILED (word 1 is the stage; see `_start`'s own `done` \
         call sites in components/src/session_reviver.rs): 0x10 no manifest at the store's root, 0x11 \
         the manifest was not UTF-8, 0x12 the manifest itself did not parse \
         (schedule_store::parse_manifest), 0x2N re-deriving the Nth identity failed (its subtree \
         or schedule file could not be read, timetable::parse refused the document, or the §16 \
         lifecycle proof itself did not hold): {w:?}",
    );
    assert_eq!(
        w[1], 1,
        "session_reviver re-derived a different count of identities than the one this suite's \
         own seed wrote (schedule_store::fixture::DEMO_IDENTITY); the manifest and the seed have \
         drifted apart: {w:?}",
    );
    assert_eq!(
        w[2], 1,
        "session_reviver's own capability-deletion proof did not hold: after cap_delete, either \
         the store-read capability or the construction budget answered something other than a \
         negative code, which is DECISIONS §123's whole scoping mechanism failing to hold: {w:?}",
    );
}

/// `fixtures/src/fs_test_client.rs`'s own `ROLE_SCHEDULE_VERIFY`: a **fresh** descent and fresh
/// handles, independent of `session_reviver`'s own read, confirming the store holds exactly the
/// bytes the seed wrote.
const ROLE_SCHEDULE_VERIFY: u64 = 12;

/// **The write path, witnessed independently of the read-at-boot path** (the seed/verify
/// shape, one level over): a second, freshly spawned client re-opens this suite's identity's own
/// subtree and its `schedule` file, and the manifest at the store's own root, and confirms both hold
/// exactly the bytes `ROLE_SCHEDULE_SEED` wrote. This is deliberately **not** the same read
/// `session_reviver` performs: the headline test above could pass even if the store held the wrong
/// bytes, as long as `session_reviver` misread them the same wrong way, which is precisely the
/// failure mode an independent witness exists to rule out.
#[test_case]
fn a_fresh_reader_confirms_the_store_holds_exactly_what_the_seed_wrote() {
    if fs_service::fs_server_image().is_none() {
        crate::testing::skip!(fs_service::NO_FS_SERVER);
    }
    // Runs `wired()` first (unused beyond that): its own seed step is what this test's fresh
    // reader depends on, and memoization means this either reuses that pass's own work or, on a
    // boot with no RedoxFS disk, skips for the identical reason `wired` itself would.
    let Some(_) = wired() else {
        crate::testing::skip!("no RedoxFS disk attached");
    };

    let client =
        program("fs_test_client").expect("no fs_test_client program in the initrd archive");
    // `wired()` above already brought the service up; `root_directory` hands back the same
    // memoized `(fs_ep, fs_page_frame)` pair with its own readiness handshake already drained.
    let (fs_ep, fs_page_frame) =
        fss::root_directory(fss::blk_server_image(), redoxfs_server_image())
            .expect("the FS service was wired a moment ago by `wired()`");
    let report = fss::spawn_fs_client(
        client,
        fs_ep,
        fs_page_frame,
        ROLE_SCHEDULE_VERIFY,
        0,
        0,
        SCHEDULE_ROLE_EXTRA_STACK,
    );
    let verify = crate::sched::ipc_recv(report);
    assert_eq!(
        verify[0],
        filesystem_protocol::fixture::SUCCESS,
        "a fresh read of the store did not match what ROLE_SCHEDULE_SEED wrote (code {:#x}, word \
         1 = {}); 0xBAD50001 means the schedule file's bytes did not match \
         schedule_store::fixture::DEMO_SCHEDULE_DOC (word 1 is how many bytes were actually \
         read), 0xBAD50002 means the manifest's bytes did not match a freshly rendered \
         one-identity manifest (word 1 is how many bytes were actually read); see \
         `fixtures/src/fs_test_client.rs`'s `schedule_verify`",
        verify[0],
        verify[1],
    );
}
