use credential_protocol::fixture::{CHRIS, CORINNE, GRAEME, NONE, WRONG};
use login_service as ls;

use super::*;
use crate::sched;

/// The login service's construction budget (`components/src/login.rs`'s `CONSTRUCTION_UT`), sized for
/// this suite's exact needs rather than guessed generously: `crate::memory_region::create` reserves this
/// many frames from the boot's free pool the moment `ls::start` runs, and this number is a direct,
/// permanent charge against `kernel::testing::SUITE_PAGE_FRAME_BUDGET` **for the logins this suite never
/// tears down**. The first version of this constant (2048) blew that budget by itself; this is the
/// account the ledger's own message asks for.
///
/// 128 for `login`'s own scratch (`OWN_UT_PAGES`), plus 128 per successful login that this suite
/// leaves logged in (that program's `CARETAKER_REGION_PAGES` 64 + `CLIENT_BUDGET_PAGES` 64) for the
/// twelve it performs in total: one in the headline test, two in the two-identity test, six in
/// `the_login_service_serves_past_the_old_capability_table_ceiling`, and three in
/// `login_scopes_each_identity_to_its_own_provisioned_subtree` (DECISIONS §117's per-identity
/// scoping: `chris`'s mark, `corinne`'s mark, and `chris`'s second, independent session). 128 +
/// 12*128 = 1664. `login_denies_an_authenticated_identity_with_no_provisioned_subtree`'s own login
/// (`graeme`, a real credential and no home subtree) used to add a permanent 64-page charge on top
/// (`mint`'s descent-refused path leaked `region` before this milestone's fix); it does not anymore,
/// `mint` reclaims it before returning, so it costs this ledger nothing.
///
/// **`caretaker_teardown_reclaims_a_full_session_worth_of_memory` below adds no permanent charge at
/// all**, which is the property that test exists to demonstrate: every login it performs logs back
/// out before the next one starts, so its own peak transient need (one login's 128 pages, on top of
/// the 1664 permanent above) is smaller than the headroom already here, and it proves reclaim works
/// by needing far more than that headroom would allow if it did not. 1856 - 1664 = 192, still more
/// than the 128 that test (or `mint`'s own scratch bookkeeping) ever needs live at once, so this
/// number is unchanged by this milestone's fix rather than lowered to the new, tighter 1664 + 128 =
/// 1792 minimum: the margin costs nothing further against `SUITE_PAGE_FRAME_BUDGET` and a lane six months
/// from now adding one more permanent login to this suite should not have to touch this constant to
/// do it. Raising this past 640 raised `kernel::testing::SUITE_PAGE_FRAME_BUDGET` too; that comment
/// carries the account.
///
/// **Raised, milestone 49's channel-per-client update: 1856 -> 2176, and the account is two lines
/// rather than the four figures an earlier version of this comment carried.** That version set this
/// to 4096 while chasing this suite's second-login failure, on the theory that the failure was a
/// sizing problem. It was not (it was `login`'s own sixteen-slot capability table; see that file's
/// BUGS), and neither 16384 here nor 8192 for `OWN_UT_PAGES` ever changed the outcome. Both are back
/// to sizes with a measurement behind them:
///
/// - **+256, two more permanently-logged-in sessions.** This update adds
///   `two_clients_connecting_together_get_independent_channels_and_neither_observes_the_others_secret`,
///   whose two roles both log in and neither logs out, at the same 128 pages a live session costs
///   every other test above. Fourteen, not twelve.
/// - **+32, `login::CHANNEL_UT_PAGES`**, split from this budget once at that process's startup, the
///   same one-time shape `OWN_UT_PAGES` already has. See that constant's own doc for why a channel's
///   region must come from a budget with exactly one spender.
///
/// `connect`'s own channel objects spend nothing permanent: each channel is retyped from a small,
/// dedicated region that is destroyed once the channel is served, and because that region is carved
/// from `CHANNEL_UT_PAGES` rather than from here, it is always the LIFO top when it is destroyed, so
/// its pages actually come home (`crates/regions`' `return_to_parent`). The earlier arrangement,
/// carving it from this budget, stranded **368 pages of holes** in one run, measured.
///
/// 128 (`OWN_UT_PAGES`) + 32 (`CHANNEL_UT_PAGES`) + 14 * 128 = 1952 permanently resident; 2176 is
/// that with the same margin 1856 carried over 1664.
///
/// **Unchanged by milestone 49's terminal update.**
/// `login_hands_out_the_terminal_once_and_denies_a_concurrent_second_login_until_logout` performs
/// two successful logins, both of `caretaker_teardown_reclaims_a_full_session_worth_of_memory`'s own
/// "logs back out before returning" shape, plus one refusal that never reaches `mint` at all
/// (`NO_TERMINAL`, checked before authentication) and one bare front-door word: no new permanent
/// charge.
const CONSTRUCTION_PAGES: u64 = 2176;

/// `EEXIST`, matching `identity_provisioner.rs`'s own local constant: `fs_proto` does not re-export
/// it under a name (that file's own comment), so every direct caller of `fs::MKDIR` names it again.
const EEXIST: i32 = 17;

/// **Ensure `name` exists as a subtree under `fs_ep`'s root, tolerating `EEXIST`** exactly as
/// `identity_provisioner.rs`'s own `mkdir_home` does: the same opcode
/// ([`filesystem_protocol::fs::MKDIR`]), the same rights ([`filesystem_protocol::dir::ALL`]), and the identity string
/// itself as the name (DECISIONS §117). Issued directly from this test rather than through a
/// spawned `identity_provisioner`, because this suite's fs root is the tree-wide shared fixture
/// (`fs_service`'s own memoized `ensure`, one FS server for the whole kernel test binary): another
/// suite may have already created this name (`identity_provisioning_tests.rs` provisions `chris`
/// too, against its own, separate credential store, sharing only the filesystem), and `EEXIST` is
/// the correct, expected answer then, not a failure.
///
/// This is what makes `wired`'s login instance below able to attenuate `chris` and `corinne` to a
/// subtree of their own at all: `login.rs` no longer creates one (DECISIONS §117: provision-time
/// creation, never auto-vivified at login), so something upstream of it must, exactly as a real
/// deployment's `identity_provisioner` would before anyone logs in.
fn ensure_home_subtree(fs_ep: sched::RendezvousId, fs_page_frame: u64, name: &[u8]) {
    // SAFETY: `fs_page_frame` is the file service's own shared page, already wired and idle (no client
    // exists yet at the point `wired` calls this, before the login service or any test client is
    // spawned), so writing into it directly and then calling through `fs_ep` is the same shape
    // `identity_provisioner_service.rs`'s own `provision` uses for its request pages.
    let page = unsafe {
        core::slice::from_raw_parts_mut(
            mmu::phys_to_virt(fs_page_frame) as *mut u8,
            filesystem_protocol::PAGE,
        )
    };
    page[..name.len()].copy_from_slice(name);
    let r = sched::ipc_call(
        fs_ep,
        [
            filesystem_protocol::fs::req(
                filesystem_protocol::fs::MKDIR,
                filesystem_protocol::fs::ROOT,
                name.len() as u64,
            ),
            filesystem_protocol::dir::ALL,
        ],
    );
    match filesystem_protocol::reply_errno(r[0] as i64) {
        Some(errno) => assert_eq!(
            errno,
            EEXIST,
            "MKDIR of {:?} failed for a reason other than already existing",
            core::str::from_utf8(name),
        ),
        // A fresh directory handle this call minted; close it, best-effort, the same choice
        // `identity_provisioner.rs`'s own `mkdir_home` makes for the same reason (this test has no
        // further use for it, and a leaked handle-table slot in the FS server is not a correctness
        // bug a caller of this helper could observe).
        None => {
            let _ = sched::ipc_call(
                fs_ep,
                [
                    filesystem_protocol::fs::req(filesystem_protocol::fs::CLOSE, r[0], 0),
                    0,
                ],
            );
        }
    }
}

/// **Write the owner's run-unvouched list** (`login_protocol::RUN_UNVOUCHED_LIST`, DECISIONS §221
/// (the boot prompt is the owner's console), ruling 2) at the root of the shared test file service,
/// as the boot prompt would; `None` removes it, which is the default: no list lists nobody.
///
/// Issued the way [`ensure_home_subtree`] issues its `MKDIR`, directly on the file page, and for
/// its reason: called between logins, when no session is live, so nothing else is mid-request on
/// that page. Every test that depends on the list sets it first, because the file service is one
/// fixture for the whole suite and a test order is not a contract.
fn set_run_unvouched_list(list: Option<&[u8]>) {
    use filesystem_protocol::fs;
    let (fs_ep, fs_page_frame) =
        fs_service::root_directory(fs_service::blk_server_image(), redoxfs_server_image())
            .expect("wired() already brought the file service up");
    // SAFETY: as in `ensure_home_subtree`: the file service's own shared page, idle between logins.
    let page = unsafe {
        core::slice::from_raw_parts_mut(
            mmu::phys_to_virt(fs_page_frame) as *mut u8,
            filesystem_protocol::PAGE,
        )
    };
    let name = login_protocol::RUN_UNVOUCHED_LIST.as_bytes();
    let named = |page: &mut [u8], verb: u64| {
        page[..name.len()].copy_from_slice(name);
        sched::ipc_call(fs_ep, [fs::req(verb, fs::ROOT, name.len() as u64), 0])[0] as i64
    };
    let Some(list) = list else {
        // `ENOENT` is the state asked for, so the answer is not checked.
        named(page, fs::UNLINK);
        return;
    };
    let mut h = named(page, fs::OPEN);
    if h < 0 {
        h = named(page, fs::CREATE);
    }
    assert!(
        h >= 0,
        "could not open or create the run-unvouched list ({h})"
    );
    let h = h as u64;
    let truncated = sched::ipc_call(fs_ep, [fs::req(fs::TRUNCATE, h, 0), 0])[0] as i64;
    assert_eq!(truncated, 0, "could not truncate the run-unvouched list");
    page[..list.len()].copy_from_slice(list);
    let wrote = sched::ipc_call(fs_ep, [fs::req(fs::WRITE, h, list.len() as u64), 0])[0] as i64;
    assert_eq!(
        wrote,
        list.len() as i64,
        "short write of the run-unvouched list"
    );
    sched::ipc_call(fs_ep, [fs::req(fs::CLOSE, h, 0), 0]);
}

/// **Wire the whole system once**: entropy, the credential service (`credential_tests::provisioned`'s
/// own fixture, which already provisions `chris`, `corinne` and `graeme` among the three family
/// logins design/roadmap/56-secrets-and-entropy.md names), the file service, and the login service
/// itself. Reusing that fixture rather than provisioning a second store is deliberate: a login-only
/// store would duplicate `credentialer_test_client.rs`'s `PEOPLE` fixture for no reason a reader
/// could point at.
///
/// **`chris` and `corinne` are given a home subtree** ([`ensure_home_subtree`]) before the login
/// service starts; `graeme` deliberately is not, so a credential that authenticates but has nowhere
/// to attenuate to is a fixture this suite already has on hand rather than one built to order. See
/// DECISIONS §117 and `login.rs`'s own BUGS on why an authenticated identity with no subtree is
/// refused rather than served, and `login_denies_an_authenticated_identity_with_no_provisioned_subtree`
/// below for the test that checks it.
///
/// Memoized on `credential_tests::provisioned`'s own terms and for the same reason: the login
/// service's construction budget is spent and never reclaimed by design in this slice (see
/// `components/src/login.rs`'s BUGS), so a re-wiring test-to-test would exhaust it for nothing.
///
/// `None` when a dependency this depends on is not attached to this run: no virtio-rng
/// (`credential_tests::provisioned`'s own condition) or no RedoxFS disk (`fs_service`'s). Both are
/// legitimate boot configurations this tree supports, so a caller skips rather than fails.
fn wired() -> Option<ls::Wiring> {
    use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    static DONE: AtomicBool = AtomicBool::new(false);
    static OK: AtomicBool = AtomicBool::new(false);
    static SAVED: [AtomicU64; 5] = [
        AtomicU64::new(0),
        AtomicU64::new(0),
        AtomicU64::new(0),
        AtomicU64::new(0),
        AtomicU64::new(0),
    ];

    if !DONE.load(Ordering::Acquire) {
        let result = (|| {
            let (cred_wiring, _, _) = credential_tests::provisioned()?;
            let (fs_ep, fs_page_frame) =
                fs_service::root_directory(fs_service::blk_server_image(), redoxfs_server_image())?;
            ensure_home_subtree(fs_ep, fs_page_frame, b"chris");
            ensure_home_subtree(fs_ep, fs_page_frame, b"corinne");
            let login_image = program("login").expect("no login program in the initrd archive");
            let w = ls::start(
                login_image,
                cred_wiring.verify,
                cred_wiring.verify_page_frame,
                fs_ep,
                fs_page_frame,
                CONSTRUCTION_PAGES,
            );
            Some(w)
        })();

        if let Some(w) = &result {
            SAVED[0].store(w.request, Ordering::Relaxed);
            SAVED[1].store(w.result, Ordering::Relaxed);
            SAVED[2].store(w.audit, Ordering::Relaxed);
            SAVED[3].store(w.term_ep, Ordering::Relaxed);
            SAVED[4].store(w.run_unvouched, Ordering::Relaxed);
            OK.store(true, Ordering::Release);
        }
        DONE.store(true, Ordering::Release);
    }
    if !OK.load(Ordering::Acquire) {
        return None;
    }
    Some(ls::Wiring {
        request: SAVED[0].load(Ordering::Relaxed),
        result: SAVED[1].load(Ordering::Relaxed),
        audit: SAVED[2].load(Ordering::Relaxed),
        term_ep: SAVED[3].load(Ordering::Relaxed),
        run_unvouched: SAVED[4].load(Ordering::Relaxed),
    })
}

/// The name `redoxfs_server`'s image travels under, on the same terms `dir_capability_tests` reads it:
/// present only when `test` built the FS server, absent for a plain interactive run.
fn redoxfs_server_image() -> &'static [u8] {
    program("redoxfs_server").expect("no redoxfs_server program in the initrd archive")
}

/// **Free the terminal directly, without spawning a client** (milestone 49's terminal update).
///
/// Every successful login now also claims the single, shared terminal
/// (`login_protocol::NO_TERMINAL` denies any concurrent second one until it is freed), and `wired()`
/// memoizes **one** login instance across this entire file. Every test below that performs a
/// successful login and does not itself exercise the terminal property calls this first, as a
/// defensive, idempotent reset: `login_protocol::LOGOUT` is answered `LOGGED_OUT` whether or not
/// anything was actually held (`login_protocol::LOGGED_OUT`'s own doc), so this is safe to call
/// regardless of what a previous test (in whatever order this suite happens to run them) left
/// behind. Issued as a raw front-door exchange rather than through `login_test_client`, since
/// freeing the terminal needs no identity and carries no secret (`login_protocol`'s own module docs on
/// why `LOGOUT` travels on the shared front door at all).
fn free_terminal(w: &ls::Wiring) {
    sched::ipc_send(w.request, [login_protocol::logout_word(), 0, 0]);
    let r = sched::ipc_recv(w.result);
    assert_eq!(
        r[0],
        login_protocol::LOGGED_OUT,
        "login_protocol::logout_word on the front door was not answered LOGGED_OUT",
    );
}

/// **The headline.** A correct identity and secret produce a directory capability and a budget
/// that actually work: a real `READDIR` through the minted caretaker, and a real page retyped from
/// the minted budget. Neither is asked to match the fixture's exact contents, which is a fact about
/// the image and not about this capability (see `login_test_client.rs`).
#[test_case]
fn login_grants_a_working_capability_set_to_the_identity_it_verified() {
    if fs_service::fs_server_image().is_none() {
        crate::testing::skip!(fs_service::NO_FS_SERVER);
    }
    let Some(w) = wired() else {
        crate::testing::skip!("no virtio-rng device or no RedoxFS disk attached");
    };
    free_terminal(&w);
    // No owner's list, which is a fresh machine's state: `login` holds the run-unvouched
    // capability and must give it to nobody (DECISIONS §221 (the boot prompt is the owner's
    // console), ruling 2). Checked on this login rather than a login of its own, because every
    // extra run in this suite is paid for by a later test's free frames (`CLIENT_SCRATCH_UT_PAGES`).
    set_run_unvouched_list(None);
    let cli =
        program("login_test_client").expect("no login_test_client program in the initrd archive");
    let r = ls::client(cli, &w, ls::LOGIN, CHRIS, CHRIS);
    assert_eq!(
        r[0],
        ls::RPT_OK,
        "a correct identity and secret were not authenticated",
    );
    assert_eq!(
        r[1] & ls::F_RUN_UNVOUCHED_ANNOUNCED,
        0,
        "with no owner's list, chris was announced the run-unvouched capability",
    );
    assert_eq!(
        r[1] & ls::F_DIR_WORKS,
        ls::F_DIR_WORKS,
        "the delegated directory capability did not answer a real READDIR",
    );
    assert_eq!(
        r[1] & ls::F_BUDGET_WORKS,
        ls::F_BUDGET_WORKS,
        "the delegated budget could not retype a page",
    );
    // `login`'s `send(AUDIT, ...)` is a blocking rendezvous: it does not return, and the service
    // cannot serve the *next* login, until something receives it. `wired()` memoizes one service
    // instance across this whole suite, so every test that causes a successful login must drain
    // this endpoint or leave the service permanently stuck here for every later test.
    let a = sched::ipc_recv(w.audit);
    assert_eq!(
        a[0],
        login_protocol::ATTRIBUTED,
        "no attribution record followed a successful login",
    );
}

/// **The refusal, and the promise that nothing follows it.** A real identity with the wrong secret
/// is denied. `login_test_client` never calls `RECV_CAP` after a refusal; if the service ever
/// sent a capability after a denial anyway, that role would simply never reach its report and this
/// test would hang rather than fail cleanly, which is the honest failure mode for a protocol
/// promise broken at the sender.
#[test_case]
fn login_denies_a_wrong_secret_and_sends_nothing_further() {
    if fs_service::fs_server_image().is_none() {
        crate::testing::skip!(fs_service::NO_FS_SERVER);
    }
    let Some(w) = wired() else {
        crate::testing::skip!("no virtio-rng device or no RedoxFS disk attached");
    };
    free_terminal(&w);
    let cli =
        program("login_test_client").expect("no login_test_client program in the initrd archive");
    // `chris`'s own identity with a secret that is nobody's: the same behaviour as the honest
    // login above, composed from a different credential rather than run as a different arm.
    let r = ls::client(cli, &w, ls::LOGIN, CHRIS, WRONG);
    assert_eq!(r[0], ls::RPT_DENIED, "a wrong secret was not refused",);
}

/// **Two principals, two channels, correctly attributed.** `chris` and `corinne` both log in and
/// both get a working capability set (this suite's headline test already proves one identity's
/// set works; this proves a second, different identity's does too, independently). The login
/// service's own audit trail then names, in order, which sequence number belongs to which
/// identity, which is DECISIONS §109's property made checkable: a server that establishes a
/// channel and can say who it belongs to.
///
/// **What this does not prove**, and it is named rather than assumed: that the two channels are
/// not secretly the same underlying object. That claim rests on the kernel's own guarantee that
/// retyping from a freshly split region always yields a new object, which is a capability-system
/// primitive this tree relies on everywhere and does not re-derive at this integration test's
/// level. What this test *does* prove is that both channels work independently and that the audit
/// record is honest about which principal established which.
#[test_case]
fn two_different_identities_get_independently_working_channels_and_correct_attribution() {
    if fs_service::fs_server_image().is_none() {
        crate::testing::skip!(fs_service::NO_FS_SERVER);
    }
    let Some(w) = wired() else {
        crate::testing::skip!("no virtio-rng device or no RedoxFS disk attached");
    };
    free_terminal(&w);
    // **And the owner's list tells them apart** (DECISIONS §221 ruling 2): it names corinne and not
    // chris, so one read of one file must answer both ways. Riding these two logins rather than
    // two of its own, for the headline test's reason.
    set_run_unvouched_list(Some(b"corinne\n"));
    let cli =
        program("login_test_client").expect("no login_test_client program in the initrd archive");

    let r_chris = ls::client(cli, &w, ls::LOGIN, CHRIS, CHRIS);
    assert_eq!(r_chris[0], ls::RPT_OK, "chris was not authenticated");
    let a_chris = sched::ipc_recv(w.audit);
    assert_eq!(
        a_chris[0],
        login_protocol::ATTRIBUTED,
        "no attribution record followed chris's login",
    );
    assert_eq!(
        a_chris[2],
        login_protocol::identity_hint(b"chris"),
        "the attribution record named the wrong identity for the first channel",
    );
    // Free the terminal before corinne's login (milestone 49's terminal update): unrelated to the
    // channel-attribution property this test exists to prove, but every successful login now also
    // claims the single, shared terminal.
    free_terminal(&w);

    let r_corinne = ls::client(cli, &w, ls::LOGIN, CORINNE, CORINNE);
    assert_eq!(r_corinne[0], ls::RPT_OK, "corinne was not authenticated",);
    let a_corinne = sched::ipc_recv(w.audit);
    assert_eq!(
        a_corinne[0],
        login_protocol::ATTRIBUTED,
        "no attribution record followed corinne's login",
    );
    assert_eq!(
        a_corinne[2],
        login_protocol::identity_hint(b"corinne"),
        "the attribution record named the wrong identity for the second channel",
    );

    assert!(
        a_corinne[1] > a_chris[1],
        "the second channel's sequence number ({}) did not follow the first's ({})",
        a_corinne[1],
        a_chris[1],
    );

    for (label, r) in [("chris", r_chris), ("corinne", r_corinne)] {
        assert_eq!(
            r[1] & ls::F_DIR_WORKS,
            ls::F_DIR_WORKS,
            "{label}'s directory capability did not work",
        );
        assert_eq!(
            r[1] & ls::F_BUDGET_WORKS,
            ls::F_BUDGET_WORKS,
            "{label}'s budget did not work",
        );
    }
    assert_eq!(
        r_chris[1] & ls::F_RUN_UNVOUCHED_ANNOUNCED,
        0,
        "chris is not on the owner's list and was announced the run-unvouched capability",
    );
    let listed = ls::F_RUN_UNVOUCHED_ANNOUNCED | ls::F_RUN_UNVOUCHED_NOT_GRANTABLE;
    assert_eq!(
        r_corinne[1] & listed,
        listed,
        "corinne is on the owner's list and was not given a run-unvouched capability she cannot \
         pass on",
    );
    set_run_unvouched_list(None);
}

/// **Two clients reaching the front door together get independent channels, and neither observes
/// the other's secret.** Milestone 49's channel-per-client update (`components/src/login.rs`'s own BUGS,
/// "Resolved"): before it, `REQUEST`/`RESULT` and a single shared staging page carried every
/// client's identity and secret for the service's whole life, so two callers reaching the front door
/// close together could corrupt or observe each other's presented credential. `ls::spawn_client`
/// (unlike `ls::client`) returns the instant a run is spawned, without waiting for it to run at
/// all, so calling it twice before waiting on either puts both roles genuinely in flight together,
/// racing each other to `CONNECT` on the kernel's own schedule: this is the scenario the old design's
/// hazard needed, and the new one's whole point is that it no longer matters who wins the race.
///
/// `login` is still one thread and answers `CONNECT` one at a time, so whichever role is served
/// first is served to completion, private channel and all, before the second is even accepted; that
/// is service *order*, the module docs' own distinction, not shared state.
///
/// **Rewritten for milestone 49's terminal update.** Before it, both concurrent logins succeeded
/// and this test's own property was that the two attribution records named exactly one chris and
/// one corinne, never crossed. Now only one of them *can* succeed: the single, shared terminal has
/// exactly one holder at a time, and the loser is refused `login_protocol::NO_TERMINAL` before its
/// identity or secret is even relayed to the credential service (`login.rs`'s own `serve_login`
/// checks this first, deliberately, so the refusal cannot become a timing or outcome oracle about a
/// specific identity). The property this test can still prove, and does: whichever one wins, its
/// own channel works and its own attribution record names *it*, never the other identity and never
/// a garbled third value -- a shared, corrupted staging page (the pre-fix hazard this test was
/// originally written to catch) could have produced either.
#[test_case]
fn two_clients_connecting_together_get_independent_channels_and_neither_observes_the_others_secret()
{
    if fs_service::fs_server_image().is_none() {
        crate::testing::skip!(fs_service::NO_FS_SERVER);
    }
    let Some(w) = wired() else {
        crate::testing::skip!("no virtio-rng device or no RedoxFS disk attached");
    };
    free_terminal(&w);
    let cli =
        program("login_test_client").expect("no login_test_client program in the initrd archive");

    // Both spawned before either is waited on: this is the race. Which role's CONNECT actually
    // reaches `login`'s front door first is the kernel scheduler's call, not this test's, and the
    // property under test holds either way.
    let chris_report = ls::spawn_client(cli, &w, ls::LOGIN, CHRIS, CHRIS);
    let corinne_report = ls::spawn_client(cli, &w, ls::LOGIN, CORINNE, CORINNE);

    // Whichever role wins the race, `login` cannot accept the *other*'s `CONNECT` until it finishes
    // fully serving the first: its own `send(AUDIT, ...)` is a blocking rendezvous (every other
    // successful-login test in this file drains it for the same reason), so this drain is what lets
    // the second role make any progress at all -- reach *its own* verdict, `NO_TERMINAL` now rather
    // than `OK` -- not merely bookkeeping. The loser's `CONNECT` sits queued on the front door until
    // this happens. Exactly one audit record follows, unlike before this update: the loser's
    // refusal never reaches `mint`, so it never reaches `AUDIT` either.
    let a = sched::ipc_recv(w.audit);
    assert_eq!(
        a[0],
        login_protocol::ATTRIBUTED,
        "no attribution record followed the winning login",
    );

    let r_chris = ls::wait_client(chris_report);
    let r_corinne = ls::wait_client(corinne_report);

    let (winner_label, winner, loser_label, loser) = if r_chris[0] == ls::RPT_OK {
        ("chris", r_chris, "corinne", r_corinne)
    } else {
        ("corinne", r_corinne, "chris", r_chris)
    };
    assert_eq!(
        winner[0],
        ls::RPT_OK,
        "neither chris ({:#x}) nor corinne ({:#x}) was authenticated",
        r_chris[0],
        r_corinne[0],
    );
    assert_eq!(
        loser[0],
        ls::RPT_NO_TERMINAL,
        "{loser_label} was refused for a reason other than the terminal already being held \
         (chris: {:#x}, corinne: {:#x})",
        r_chris[0],
        r_corinne[0],
    );
    assert_eq!(
        winner[1] & ls::F_DIR_WORKS,
        ls::F_DIR_WORKS,
        "{winner_label}'s directory capability did not work",
    );
    assert_eq!(
        winner[1] & ls::F_BUDGET_WORKS,
        ls::F_BUDGET_WORKS,
        "{winner_label}'s budget did not work",
    );

    // The property under test, stated positively: the one attribution record names exactly the
    // identity that won, never the other one and never a garbled third value.
    let expected_hint = login_protocol::identity_hint(winner_label.as_bytes());
    assert_eq!(
        a[2], expected_hint,
        "the attribution record named the wrong identity for the winning channel \
         (winner was {winner_label})",
    );

    free_terminal(&w);
}

/// **Nothing else would have caught this**: `login`'s own capability table has sixteen slots
/// (`kernel::cap::CAPABILITY_TABLE_SLOTS`), eight are spent at rest (`REQUEST`..`AUDIT` plus its own scratch
/// untyped), and before `components/src/login.rs`'s `mint` learned to drop its own copy of the
/// caretaker's construction region, that region's capability was never freed on a successful
/// login. That left room for exactly eight successful logins ever; a ninth, correctly
/// authenticated, was silently answered [`login_protocol::DENIED`], indistinguishable from a wrong
/// password, at a far tighter ceiling than the memory bound `components/src/login.rs`'s BUGS documents.
///
/// This performs **six** successful logins against one service instance, on top of the three the
/// headline test and the two-identity test above already performed against the same memoized
/// instance (`wired()`): nine in total, one past that old eight-login ceiling. It asserts every one
/// of its own six actually worked (not merely that the verdict was `OK`): a real `READDIR` through
/// the minted directory and a real page retyped from the minted budget, the same proof the headline
/// test above asks for. Alternates `chris` and `corinne` rather than repeating one identity, so a
/// regression that only leaked on one particular identity's path would not hide from this test by
/// accident.
///
/// Six rather than nine performed here directly: a real login's `CARETAKER_REGION_PAGES` and
/// `CLIENT_BUDGET_PAGES` are permanently unreclaimable memory (see `components/src/login.rs`'s BUGS), so
/// every attempt this test adds is a permanent charge against `kernel::testing::SUITE_PAGE_FRAME_BUDGET`
/// and not merely against `CONSTRUCTION_PAGES`. Reusing the other two tests' three logins rather
/// than repeating them here is what keeps that charge to what actually proves the fix.
#[test_case]
fn the_login_service_serves_past_the_old_capability_table_ceiling() {
    if fs_service::fs_server_image().is_none() {
        crate::testing::skip!(fs_service::NO_FS_SERVER);
    }
    let Some(w) = wired() else {
        crate::testing::skip!("no virtio-rng device or no RedoxFS disk attached");
    };
    free_terminal(&w);
    let cli =
        program("login_test_client").expect("no login_test_client program in the initrd archive");

    const ATTEMPTS: usize = 6;
    for i in 0..ATTEMPTS {
        // Alternating identities, which is what this loop always meant and can now say: one
        // behaviour, two credentials, rather than two roles that happened to run the same code.
        let who = if i % 2 == 0 { CHRIS } else { CORINNE };
        let r = ls::client(cli, &w, ls::LOGIN, who, who);
        assert_eq!(
            r[0],
            ls::RPT_OK,
            "login {i} of {ATTEMPTS} in this test (the 4th through 9th against this shared \
             instance, past the old eight-login capability table ceiling) was not authenticated; a real \
             password was answered as though it were wrong",
        );
        assert_eq!(
            r[1] & ls::F_DIR_WORKS,
            ls::F_DIR_WORKS,
            "login {i}'s directory capability did not work",
        );
        assert_eq!(
            r[1] & ls::F_BUDGET_WORKS,
            ls::F_BUDGET_WORKS,
            "login {i}'s budget did not work",
        );
        // Drain the attribution record so the service is free to serve the next login (the
        // headline test's own note: `send(AUDIT, ...)` is a blocking rendezvous).
        let a = sched::ipc_recv(w.audit);
        assert_eq!(
            a[0],
            login_protocol::ATTRIBUTED,
            "no attribution record followed login {i}",
        );
        // Free the terminal so login {i+1} is authenticated rather than refused NO_TERMINAL
        // (milestone 49's terminal update: every successful login also claims the single, shared
        // terminal).
        free_terminal(&w);
    }
}

/// **DECISIONS §117: each identity is attenuated to its own provisioned subtree, not the old
/// shared fixture and not another identity's.**
///
/// `chris` and `corinne` each log in and, through the directory capability `login` delegated,
/// `CREATE` a marker file naming themselves ([`ls::WRITE_MARKER`], run once per identity); both
/// also confirm
/// [`filesystem_protocol::fixture::tree::INNER`] is absent, which is this suite's own proof (not merely a
/// stated intent) that neither landed in the old shared subtree every identity used to be
/// attenuated to before this milestone. `chris` then logs in a **second, independent** time
/// ([`ls::READ_MARKER`]) and reads the marker back: it must read `chris`'s own,
/// which is the property under test stated positively: the *same* identity's two sessions land in
/// the *same* subtree, and it is not `corinne`'s (had the old bug still been present, `corinne`'s
/// later write would have overwritten `chris`'s marker in the one subtree they would have shared,
/// and this final read would come back `corinne`'s instead).
///
/// `wired`'s own `ensure_home_subtree` issues the identical `filesystem_protocol::fs::MKDIR` request
/// `identity_provisioner.rs`'s own `mkdir_home` does (same opcode, same
/// [`filesystem_protocol::dir::ALL`] rights, the identity string as the name); that milestone's own suite
/// (`identity_provisioning_tests.rs`, `provisioning_creates_a_working_credential_and_a_real_subtree`)
/// already proves the *tool* produces a real, descendable subtree end to end, so this test's job is
/// the other half: that `login`, given one, finds and attenuates to the *right* one.
#[test_case]
fn login_scopes_each_identity_to_its_own_provisioned_subtree() {
    if fs_service::fs_server_image().is_none() {
        crate::testing::skip!(fs_service::NO_FS_SERVER);
    }
    let Some(w) = wired() else {
        crate::testing::skip!("no virtio-rng device or no RedoxFS disk attached");
    };
    free_terminal(&w);
    let cli =
        program("login_test_client").expect("no login_test_client program in the initrd archive");

    let r_chris = ls::client(cli, &w, ls::WRITE_MARKER, CHRIS, CHRIS);
    assert_eq!(r_chris[0], ls::RPT_OK, "chris was not authenticated");
    assert_eq!(
        r_chris[1] & ls::F_MARKER_WRITTEN,
        ls::F_MARKER_WRITTEN,
        "chris's marker was not created and written",
    );
    assert_eq!(
        r_chris[1] & ls::F_NOT_SHARED_SUBTREE,
        ls::F_NOT_SHARED_SUBTREE,
        "chris's granted directory carries the old shared fixture's own file \
         (filesystem_protocol::fixture::tree::INNER), so it is not a subtree of chris's own",
    );
    let a_chris = sched::ipc_recv(w.audit);
    assert_eq!(
        a_chris[0],
        login_protocol::ATTRIBUTED,
        "no attribution record followed chris's marking login",
    );
    // Free the terminal before corinne's own login (milestone 49's terminal update): unrelated to
    // the per-identity subtree property this test exists to prove, but every successful login now
    // also claims the single, shared terminal.
    free_terminal(&w);

    let r_corinne = ls::client(cli, &w, ls::WRITE_MARKER, CORINNE, CORINNE);
    assert_eq!(r_corinne[0], ls::RPT_OK, "corinne was not authenticated");
    assert_eq!(
        r_corinne[1] & ls::F_MARKER_WRITTEN,
        ls::F_MARKER_WRITTEN,
        "corinne's marker was not created and written",
    );
    assert_eq!(
        r_corinne[1] & ls::F_NOT_SHARED_SUBTREE,
        ls::F_NOT_SHARED_SUBTREE,
        "corinne's granted directory carries the old shared fixture's own file \
         (filesystem_protocol::fixture::tree::INNER), so it is not a subtree of corinne's own",
    );
    let a_corinne = sched::ipc_recv(w.audit);
    assert_eq!(
        a_corinne[0],
        login_protocol::ATTRIBUTED,
        "no attribution record followed corinne's marking login",
    );
    free_terminal(&w);

    let r_check = ls::client(cli, &w, ls::READ_MARKER, CHRIS, CHRIS);
    assert_eq!(
        r_check[0],
        ls::RPT_OK,
        "chris's second, independent login was not authenticated",
    );
    assert_eq!(
        r_check[1] & ls::F_DIR_WORKS,
        ls::F_DIR_WORKS,
        "chris's second login's directory capability did not work",
    );
    assert_eq!(
        r_check[2],
        login_protocol::identity_hint(b"chris"),
        "chris's second, independent login did not read back chris's own marker: either it did \
         not land in chris's subtree, or corinne's write clobbered it, which is exactly the \
         everyone-shares-one-subtree bug this milestone fixes",
    );
    let a_check = sched::ipc_recv(w.audit);
    assert_eq!(
        a_check[0],
        login_protocol::ATTRIBUTED,
        "no attribution record followed chris's second login",
    );
    free_terminal(&w);
}

/// **The considered fold this milestone adds: a real, authenticated identity with no provisioned
/// subtree is refused, indistinguishably from a wrong password.**
///
/// `graeme` has a real credential in `wired`'s store (`credential_tests::provisioned`'s own family
/// fixture) and, deliberately, no home subtree (`wired` only calls `ensure_home_subtree` for `chris`
/// and `corinne`): the case `identity_provisioner` never having run for a real identity, or its
/// `MKDIR` never reaching this file service's disk. `mint`'s caretaker construction reaches the
/// same `OPENDIR`-against-a-missing-name refusal, and `login`'s existing fold answers it with
/// [`login_protocol::DENIED`], the same code [`login_denies_a_wrong_secret_and_sends_nothing_further`]
/// above already proves a wrong password gets. See `components/src/login.rs`'s own BUGS for the reasoning
/// (a caller must not be able to tell "your identity has no home" from "your password is wrong" by
/// comparing outcomes across attempts).
///
/// No audit record follows this login, by the same rule a wrong secret gets none: `login`'s `AUDIT`
/// send lives only on the success path.
#[test_case]
fn login_denies_an_authenticated_identity_with_no_provisioned_subtree() {
    if fs_service::fs_server_image().is_none() {
        crate::testing::skip!(fs_service::NO_FS_SERVER);
    }
    let Some(w) = wired() else {
        crate::testing::skip!("no virtio-rng device or no RedoxFS disk attached");
    };
    free_terminal(&w);
    let cli =
        program("login_test_client").expect("no login_test_client program in the initrd archive");
    // `graeme`: a real credential this suite provisions and a home subtree it deliberately never
    // creates. An honest login, refused for the subtree rather than for the secret.
    let r = ls::client(cli, &w, ls::LOGIN, GRAEME, GRAEME);
    assert_eq!(
        r[0],
        ls::RPT_DENIED,
        "graeme's real credential, with no provisioned subtree, was not refused",
    );
}

/// **The measurement gap this milestone's BUGS used to name, resolved.** `login`'s `_start` now
/// reads [`measured_boot::PROGRAM_MEASUREMENTS`] out of the same archive it reads
/// `fs_subtree_caretaker` from and calls [`measured_boot::verify_in_manifest`] before it will ever
/// build a caretaker from those bytes, the identical check `crates/system_initializer::measured`
/// performs for its own six boot components (see `components/src/login.rs`'s own BUGS, "Resolved,
/// 2026-08-24", for the full reasoning, including why this is a boot-time check rather than a
/// per-login one, and why the fold into [`login_protocol::DENIED`] it produces on refusal is not the
/// same anti-oracle reasoning a wrong password or a missing subtree gets).
///
/// **This suite cannot make the real, already-booted archive disagree with itself** to prove a live
/// refusal end to end: the initrd is one physical region the whole kernel test binary shares, wired
/// once at boot, and `cargo xtask` always packs a measurement table that agrees with the bytes it
/// just packed alongside it. A second login instance spawned against a deliberately tampered archive
/// is not something this suite (or `ls::start`, which always takes the one real
/// `memory::initrd_region()`) can construct. So this proves the two things a live end-to-end refusal
/// would have proven, at the exact level `login`'s own check operates on: the real
/// `fs_subtree_caretaker` bytes verify against the real table (which is *also* the reason `wired()`'s
/// own instance, and every other test in this file, can log anyone in at all, now that the check
/// exists), and the identical [`measured_boot::verify_in_manifest`] call `_start` makes refuses a
/// tampered copy of those same bytes and a name the table does not mention, mirroring
/// `measured_boot_tests::the_measurement_table_refuses_a_tampered_program_and_an_unnamed_one`, which
/// covers `console` and the boot-time components but not this one (`fs_subtree_caretaker` is not in
/// that test's own `BOOT_COMPONENTS`, since it is not a boot-time component; it is what a directory
/// grant is made of, per-invocation, and this milestone is what first gave it a measurement
/// consumer at all).
#[test_case]
fn logins_caretaker_measurement_matches_the_real_table_and_a_tampered_one_would_be_refused() {
    let table = program(measured_boot::PROGRAM_MEASUREMENTS)
        .and_then(|b| core::str::from_utf8(b).ok())
        .expect("the measurement table is missing or is not text");
    let bytes = program("fs_subtree_caretaker").expect("the archive has no 'fs_subtree_caretaker'");

    assert_eq!(
        measured_boot::verify_in_manifest(table, "fs_subtree_caretaker", bytes),
        Ok(()),
        "the caretaker every login builds a session from does not match the table login's own \
         _start now consults; login would refuse every login rather than serve one",
    );

    let mut h = measured_boot::Sha256::new();
    h.update(&[bytes[0] ^ 1]);
    h.update(&bytes[1..]);
    let tampered = h.finalize();
    assert_ne!(
        measured_boot::expected_in_manifest(table, "fs_subtree_caretaker"),
        Some(tampered),
        "a tampered fs_subtree_caretaker still satisfied the table",
    );
    assert_eq!(
        measured_boot::verify_in_manifest(table, "fs_subtree_caretaker", b"not the real caretaker"),
        Err(measured_boot::VerifyError::Mismatch),
        "login's own check would wave through a caretaker whose bytes do not match the table",
    );
    assert_eq!(
        measured_boot::verify_in_manifest(table, "no-such-caretaker", bytes),
        Err(measured_boot::VerifyError::Unmeasured),
        "login's own check would vouch for a name the table does not mention",
    );
}

/// **Nothing else would have caught this**: the caretaker-teardown fix (this milestone; see
/// `components/src/login.rs`'s own BUGS, "Resolved"), proven by needing more memory than could possibly
/// fit if a session's pages did not actually come back. `wired()`'s shared instance holds
/// `CONSTRUCTION_PAGES` in total, and all but a few hundred pages of that is already permanently
/// spent by the tests above (this file's own accounting comment on that constant); what is left
/// would not survive a *second* leaked session at the old 128-pages-per-login rate, let alone the
/// ten this test performs (2560 pages, an order of magnitude past the headroom). Every one of the
/// ten logs fully back out
/// (both delegated `MemoryRegion`s destroyed, `login_test_client.rs`'s `LOGOUT`) before the next
/// begins, so this test could only pass by the memory genuinely coming home each time.
///
/// Each iteration asserts the whole chain, not merely that the final `DESTROY` returned success: a
/// real `READDIR` and a real `RETYPE` before teardown (the same proof every other successful-login
/// test in this file asks for), both `DESTROY` calls succeeding, and both capabilities answering
/// nothing afterward (`login_test_client.rs`'s own re-check, not merely the syscalls' return codes).
///
/// **It also caught a second bug, and this is the one worth knowing about**: for two days this test
/// failed reproducibly on its *second* iteration, refusing a correct password. Ten cycles in a row
/// is what made it visible at all, because the cause was a two-slot-per-connect leak of `login`'s
/// own sixteen-slot capability table, and a service that never serves two logins in a row cannot
/// show it. See `components/src/login.rs`'s BUGS ("Resolved, 2026-08-26") for the mechanism and for the
/// four memory hypotheses that were measured and ruled out before it was found.
#[test_case]
fn caretaker_teardown_reclaims_a_full_session_worth_of_memory() {
    if fs_service::fs_server_image().is_none() {
        crate::testing::skip!(fs_service::NO_FS_SERVER);
    }
    let Some(w) = wired() else {
        crate::testing::skip!("no virtio-rng device or no RedoxFS disk attached");
    };
    free_terminal(&w);
    let cli =
        program("login_test_client").expect("no login_test_client program in the initrd archive");

    const ATTEMPTS: usize = 10;
    for i in 0..ATTEMPTS {
        let r = ls::client(cli, &w, ls::LOGOUT, CHRIS, CHRIS);
        assert_eq!(
            r[0],
            ls::RPT_OK,
            "login {i} of {ATTEMPTS} (the teardown role) was not authenticated",
        );
        assert_eq!(
            r[1] & ls::F_DIR_WORKS,
            ls::F_DIR_WORKS,
            "login {i}'s directory capability did not work before teardown",
        );
        assert_eq!(
            r[1] & ls::F_BUDGET_WORKS,
            ls::F_BUDGET_WORKS,
            "login {i}'s budget did not work before teardown",
        );
        assert_eq!(
            r[1] & ls::F_TEARDOWN_OK,
            ls::F_TEARDOWN_OK,
            "login {i}'s logout ticket did not destroy the caretaker's construction region; the \
             client waited {} us for the refusal to clear, against its own ceiling of {} us. A \
             number near that ceiling means the caretaker never died, which is a kernel bug and \
             not this host being slow; a number far under it means the client stopped waiting \
             early, which is the defect that ceiling replaced. See \
             fixtures/src/login_test_client.rs's `destroy_with_retry`.",
            r[2],
            ls::DESTROY_WAIT_MICROS,
        );
        assert_eq!(
            r[1] & ls::F_DEAD_AFTER_TEARDOWN,
            ls::F_DEAD_AFTER_TEARDOWN,
            "login {i}'s directory capability still answered a READDIR after its region was destroyed",
        );
        assert_eq!(
            r[1] & ls::F_BUDGET_TEARDOWN_OK,
            ls::F_BUDGET_TEARDOWN_OK,
            "login {i}'s own budget did not destroy",
        );
        assert_eq!(
            r[1] & ls::F_BUDGET_DEAD_AFTER_TEARDOWN,
            ls::F_BUDGET_DEAD_AFTER_TEARDOWN,
            "login {i}'s budget still answered a RETYPE after its own destroy",
        );
        // Drain the attribution record, the same as every other successful-login test in this file
        // (the headline test's own note: `send(AUDIT, ...)` is a blocking rendezvous).
        let a = sched::ipc_recv(w.audit);
        assert_eq!(
            a[0],
            login_protocol::ATTRIBUTED,
            "no attribution record followed login {i}",
        );
        // **Free the terminal, after draining `AUDIT`, not before.** `LOGOUT` deliberately
        // does not do this itself (`login_test_client.rs`'s own comment on why: it would deadlock
        // against this same drain). Every iteration after the first would otherwise be refused
        // `NO_TERMINAL` instead of authenticated, since milestone 49's terminal update means every
        // successful login also claims the single, shared terminal.
        free_terminal(&w);
    }
}

/// **Milestone 49's terminal update, end to end: single-session, deny cleanly.** The roadmap's own
/// recommendation (`design/roadmap/49-users-and-attribution.md`'s BUGS) built and proven here in one
/// sequence against the one memoized service instance every other test in this file shares:
///
/// 1. `chris` logs in ([`ls::HOLD_TERMINAL`]) and receives a fifth delegated capability, the
///    terminal, which this test confirms is real (not merely present) by receiving the word the
///    role sends through it on `w.term_ep` -- the stand-in terminal this suite's own harness wires
///    in place of a real one (`ls::Wiring::term_ep`'s own doc). `chris` then tears the *session*
///    down (the same `MemoryRegion::DESTROY` pair [`ls::LOGOUT`] already proves) **without**
///    freeing the terminal: no `login_protocol::logout_word` is ever sent.
/// 2. `corinne` then presents a **real, correct** credential (a plain [`ls::LOGIN`], which is all
///    that case ever was) while the
///    terminal is still on loan. She is refused [`login_protocol::NO_TERMINAL`], not `DENIED`: this is
///    provably not about her identity or secret, both of which are genuine.
/// 3. [`ls::FREE_TERMINAL`] frees the terminal, a bare word on the front door with no identity
///    involved at all, and is answered [`login_protocol::LOGGED_OUT`].
/// 4. `chris` logs in again ([`ls::HOLD_TERMINAL`], a second, independent time) and receives the
///    terminal a second time, proven the same way as step 1 -- the property stated positively: a
///    session freeing the terminal is what makes it available to the *next* login, not merely what
///    stops it being refused.
///
/// **Costs nothing permanent against [`CONSTRUCTION_PAGES`].** Every login this test performs either
/// tears its own session down before returning (`HOLD_TERMINAL`, exactly like `LOGOUT`) or is
/// refused before `mint` ever runs (`corinne`'s `NO_TERMINAL`, checked in `login.rs`'s
/// `serve_login` before authentication is even attempted), so this test's own two successful logins
/// are the same "logs back out" shape `caretaker_teardown_reclaims_a_full_session_worth_of_memory`
/// already established, not a third kind of permanent charge.
#[test_case]
fn login_hands_out_the_terminal_once_and_denies_a_concurrent_second_login_until_logout() {
    if fs_service::fs_server_image().is_none() {
        crate::testing::skip!(fs_service::NO_FS_SERVER);
    }
    let Some(w) = wired() else {
        crate::testing::skip!("no virtio-rng device or no RedoxFS disk attached");
    };
    free_terminal(&w);
    let cli =
        program("login_test_client").expect("no login_test_client program in the initrd archive");

    // Step 1: chris takes the terminal. `spawn_client`/`wait_client`, not `client`, because the
    // role blocks inside `send(term_ep, ...)` until this test's own `ipc_recv(w.term_ep)` catches
    // it (a genuine rendezvous, not merely `RECV_CAP` succeeding): waiting on the report first would
    // deadlock against the role's own forward progress.
    let report1 = ls::spawn_client(cli, &w, ls::HOLD_TERMINAL, CHRIS, CHRIS);
    let a1 = sched::ipc_recv(w.audit);
    assert_eq!(
        a1[0],
        login_protocol::ATTRIBUTED,
        "no attribution record followed chris's first terminal login",
    );
    let term_msg = sched::ipc_recv(w.term_ep);
    assert_eq!(
        term_msg[0],
        ls::TERM_MAGIC,
        "the delegated terminal capability did not deliver a real message to a real receiver",
    );
    let r1 = ls::wait_client(report1);
    assert_eq!(
        r1[0],
        ls::RPT_OK,
        "chris's first terminal login was not authenticated"
    );
    for (bit, what) in [
        (ls::F_DIR_WORKS, "the directory capability"),
        (ls::F_BUDGET_WORKS, "the budget"),
        (ls::F_TERM_WORKS, "the terminal capability"),
        (ls::F_TEARDOWN_OK, "the logout ticket's own DESTROY"),
        (
            ls::F_DEAD_AFTER_TEARDOWN,
            "the directory's post-teardown refusal",
        ),
        (ls::F_BUDGET_TEARDOWN_OK, "the budget's own DESTROY"),
        (
            ls::F_BUDGET_DEAD_AFTER_TEARDOWN,
            "the budget's post-teardown refusal",
        ),
    ] {
        assert_eq!(
            r1[1] & bit,
            bit,
            "chris's first terminal login: {what} did not work"
        );
    }

    // Step 2: corinne, a real credential, refused purely because the terminal is spoken for.
    let r2 = ls::client(cli, &w, ls::LOGIN, CORINNE, CORINNE);
    assert_eq!(
        r2[0],
        ls::RPT_NO_TERMINAL,
        "corinne's real credential was not refused NO_TERMINAL while chris still held the terminal",
    );

    // Step 3: free it.
    let r3 = ls::client(cli, &w, ls::FREE_TERMINAL, NONE, NONE);
    assert_eq!(
        r3[0],
        ls::RPT_LOGGED_OUT,
        "login_protocol::logout_word on the front door was not answered LOGGED_OUT",
    );

    // Step 4: chris again, a second, independent time, and the terminal is available once more.
    let report4 = ls::spawn_client(cli, &w, ls::HOLD_TERMINAL, CHRIS, CHRIS);
    let a4 = sched::ipc_recv(w.audit);
    assert_eq!(
        a4[0],
        login_protocol::ATTRIBUTED,
        "no attribution record followed chris's second terminal login",
    );
    let term_msg2 = sched::ipc_recv(w.term_ep);
    assert_eq!(
        term_msg2[0],
        ls::TERM_MAGIC,
        "the terminal was not delegated again after being freed",
    );
    let r4 = ls::wait_client(report4);
    assert_eq!(
        r4[0],
        ls::RPT_OK,
        "chris's second terminal login (after logout freed it) was not authenticated",
    );
    assert_eq!(
        r4[1] & ls::F_TERM_WORKS,
        ls::F_TERM_WORKS,
        "the re-delegated terminal capability did not work",
    );
}

/// **A listed identity's session is handed the run-unvouched capability, and cannot pass it on**
/// (DECISIONS §219 (how the shell names an installed program to the spawner) gate D2, milestone 198 (a package manager) rung 3a;
/// DECISIONS §221 (the boot prompt is the owner's console), ruling 2, for "listed").
///
/// What this proves that nothing else would: that the sixth capability a session receives names
/// the very endpoint the spawner gave `login` (a word sent on it arrives on that endpoint, which is
/// where the progenitor receives a presentation in the real boot), and that the session's copy
/// lacks `GRANT` (its `SEND_CAP` is refused), which is limitation 2's reason the capability is a
/// presentation rather than a token. `script/swish-check` proves the progenitor's half; no session
/// `login` builds holds a spawn endpoint, so this is the only place the delivery runs.
///
/// Falsified by hand before it was committed (aarch64, 2026-09-26): delegating the sixth capability
/// `WRITE | GRANT` in `components/src/login.rs` turned this test red. The client's `SEND_CAP` of it
/// then succeeded, onto the report endpoint, and arrived in place of the report (word 0 read 0).
#[test_case]
fn login_hands_a_listed_session_the_run_unvouched_capability_and_it_cannot_be_passed_on() {
    if fs_service::fs_server_image().is_none() {
        crate::testing::skip!(fs_service::NO_FS_SERVER);
    }
    let Some(w) = wired() else {
        crate::testing::skip!("no virtio-rng device or no RedoxFS disk attached");
    };
    free_terminal(&w);
    set_run_unvouched_list(Some(b"# the owner's list\nchris\n"));
    let cli =
        program("login_test_client").expect("no login_test_client program in the initrd archive");
    // `spawn_client`/`wait_client` for the terminal test's reason: the role blocks in its `send`
    // on the sixth capability until this test takes the word.
    let report = ls::spawn_client(cli, &w, ls::PRESENT_RUN_UNVOUCHED, CHRIS, CHRIS);
    let a = sched::ipc_recv(w.audit);
    assert_eq!(
        a[0],
        login_protocol::ATTRIBUTED,
        "no attribution record followed the login",
    );
    let presented = sched::ipc_recv(w.run_unvouched);
    assert_eq!(
        presented[0],
        ls::RUN_UNVOUCHED_MAGIC,
        "the session's run-unvouched capability did not reach the endpoint login was given",
    );
    let r = ls::wait_client(report);
    assert_eq!(
        r[0],
        ls::RPT_OK,
        "no OK report: chris was not authenticated, or the session's run-unvouched capability \
         carried GRANT and its delegation arrived in place of the report",
    );
    for (bit, what) in [
        (
            ls::F_RUN_UNVOUCHED_NOT_GRANTABLE,
            "the run-unvouched capability could be delegated onward",
        ),
        (
            ls::F_RUN_UNVOUCHED_WORKS,
            "the run-unvouched capability did not deliver a word",
        ),
        // The session comes home, so nothing this test built outlives it: left standing, its
        // caretaker starved `timetable_tests` into a hang later in the suite (riscv64, aarch64).
        (ls::F_TEARDOWN_OK, "the logout ticket's own DESTROY"),
        (ls::F_BUDGET_TEARDOWN_OK, "the budget's own DESTROY"),
    ] {
        assert_eq!(r[1] & bit, bit, "{what}");
    }
    free_terminal(&w);
    set_run_unvouched_list(None);
}

/// **A login session with pending work cannot be logged out until the work is gone**, for
/// milestone 152 (durable delegation), on DECISIONS §16 (object revocation).
///
/// Milestone 152's design keeps a user's session alive past a disconnect for exactly as long as it
/// has scheduled work, and it builds no new mechanism for that: a scheduled job's authority is a
/// child split off the session's budget, and §16 already refuses `MemoryRegion::DESTROY` on a
/// parent with a live child. This proves that on the object a login session actually is. The
/// session's own budget, delegated by `login`, carries a one-page child; the logout's first step is
/// refused `NotPermitted`; the budget still works afterwards; and once the child is destroyed the
/// same logout goes through in [`ls::LOGOUT`]'s order, with its proofs that both capabilities came
/// down.
///
/// The proof used to live on `smb_server`'s `DurableSession`, which went with the SMB code on
/// 2026-08-30. `session_reviver` keeps a synthetic copy of it for boot-derived sessions; this is the
/// live-login half those comments point at.
///
/// **Costs nothing permanent against [`CONSTRUCTION_PAGES`]**: the session logs fully back out.
#[test_case]
fn a_login_session_with_pending_work_refuses_logout_until_the_work_is_gone() {
    if fs_service::fs_server_image().is_none() {
        crate::testing::skip!(fs_service::NO_FS_SERVER);
    }
    let Some(w) = wired() else {
        crate::testing::skip!("no virtio-rng device or no RedoxFS disk attached");
    };
    free_terminal(&w);
    let cli =
        program("login_test_client").expect("no login_test_client program in the initrd archive");
    let r = ls::client(cli, &w, ls::PENDING_WORK, CHRIS, CHRIS);
    assert_eq!(r[0], ls::RPT_OK, "chris was not authenticated");
    for (bit, what) in [
        (
            ls::F_BUDGET_WORKS,
            "the budget did not work before the child was split",
        ),
        (
            ls::F_LOGOUT_REFUSED_WHILE_PENDING,
            "the budget's DESTROY was not refused NotPermitted while a pending-job child lived; a \
             session with scheduled work could be logged out from under it",
        ),
        (
            ls::F_SESSION_SURVIVED_REFUSAL,
            "the budget stopped working after its refused DESTROY",
        ),
        (
            ls::F_PENDING_WORK_DESTROYED,
            "the pending-job child did not destroy",
        ),
        (
            ls::F_BUDGET_TEARDOWN_OK,
            "the budget did not destroy once its child was gone",
        ),
        (
            ls::F_BUDGET_DEAD_AFTER_TEARDOWN,
            "the budget still answered a RETYPE after its own destroy",
        ),
        (ls::F_TEARDOWN_OK, "the logout ticket's own DESTROY"),
        (
            ls::F_DEAD_AFTER_TEARDOWN,
            "the directory still answered a READDIR after the session came down",
        ),
    ] {
        assert_eq!(r[1] & bit, bit, "{what}");
    }
    let a = sched::ipc_recv(w.audit);
    assert_eq!(
        a[0],
        login_protocol::ATTRIBUTED,
        "no attribution record followed the login",
    );
    free_terminal(&w);
}
