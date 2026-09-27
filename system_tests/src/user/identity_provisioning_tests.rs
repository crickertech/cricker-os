use credential_service as cs;
use identity_provisioner_service as ips;

use super::*;

/// The identity this suite provisions: the roster's first person, so `credentialer_test_client`'s
/// `ROLE_HONEST` can be reused unmodified as the verification half. Its first check (the right
/// password for that person) becomes a real proof that what this tool `PUT` is what a client can
/// later verify, and its other three checks (a wrong password, an unknown identity, another
/// identity's password against this one) are unaffected by which store they run against.
///
/// **Taken from `credential_protocol::fixture` rather than matched to it by hand** (milestone 293).
/// The comment that used to sit here said this suite had *chosen* `chris`/`correct horse battery
/// staple` to line up with a constant in a fixture program, which is a coupling with nothing holding
/// it: the next person to edit either copy would have had no way to know the other existed.
const IDENTITY: &[u8] =
    credential_protocol::fixture::PEOPLE[credential_protocol::fixture::CHRIS as usize].0;
/// The same identity as a `&str`, because `fs_service::narrow_dir` names a path component rather
/// than a credential. **Derived rather than re-typed** (milestone 293): a second spelling of the
/// same name is the exact shape this milestone spent its diff removing, and a `const` conversion
/// means the two cannot be made to disagree at all rather than merely being checked.
const IDENTITY_STR: &str = match core::str::from_utf8(IDENTITY) {
    Ok(s) => s,
    Err(_) => panic!("the roster's identity is not UTF-8, so it cannot name a directory"),
};
const SECRET: &[u8] =
    credential_protocol::fixture::PEOPLE[credential_protocol::fixture::CHRIS as usize].1;

/// A second `PUT` for the same identity, with a different secret, used only to provoke the refusal
/// this suite's second test asks for. Never expected to be stored. Another person's, which is what
/// makes it certainly not this one's.
const DUPLICATE_SECRET: &[u8] =
    credential_protocol::fixture::PEOPLE[credential_protocol::fixture::CORINNE as usize].1;

/// What [`wired`] hands each test: the sealed store's own wiring (for a real `VERIFY`) and both
/// provisioning attempts' raw reports. The file service's root that both attempts were run against
/// is not carried past setup: every test reaches the filesystem through `fs_service`'s own memoized
/// globals instead (`fs_service::narrow_dir` re-derives the same root `ensure` already wired).
struct Wired {
    cs: cs::Wiring,
    /// The first attempt's report: a fresh identity, expected [`ips::RPT_OK`].
    fresh: [u64; 2],
    /// The second attempt's report: the same identity again, expected [`ips::RPT_CRED_FAILED`].
    duplicate: [u64; 2],
}

/// The name `redoxfs_server`'s image travels under; present only when `test` built it.
fn redoxfs_server_image() -> &'static [u8] {
    program("redoxfs_server").expect("no redoxfs_server program in the initrd archive")
}

/// **Wire a fresh entropy service, a fresh (unsealed) credential service, and the file service, then
/// run `identity_provisioner` against the fresh store twice** before sealing it: once for a new
/// identity, and once more for the same identity, to capture the tool's own report on a genuine
/// duplicate. Both reports are memoized alongside the wiring, once per boot.
///
/// **Deliberately its own store, not `credential_tests::provisioned()`'s.** That fixture is sealed
/// by the time it returns (its own doc: "the seal is irreversible... a second provisioner would
/// `CALL` an endpoint whose receiver is gone and block forever") and already holds six of six
/// slots, so there is no open provision endpoint left for this milestone's own tool to be the first
/// real caller of. This suite needs the store *before* anyone has sealed it, which no existing
/// fixture offers.
///
/// Both provisioning attempts happen inside this one-time setup, against `w.provision_page_frame`
/// (`cs::Wiring::provision_page_frame`, added by this milestone: `credential_service` used to look this
/// up through a global shared by every instance, and this suite is the first caller to need a
/// second, independent one in the same boot). `w.verify_page_frame` is carried past setup too, in
/// [`Wired`]'s own `cs` field, because `credential_service::client` reads it directly.
fn wired() -> Option<Wired> {
    use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    static DONE: AtomicBool = AtomicBool::new(false);
    static OK: AtomicBool = AtomicBool::new(false);
    #[allow(clippy::declare_interior_mutable_const)]
    const ZERO: AtomicU64 = AtomicU64::new(0);
    static SAVED: [AtomicU64; 8] = [ZERO; 8];

    if !DONE.load(Ordering::Acquire) {
        let result = (|| {
            let rng = program("entropy").expect("no entropy program in the initrd archive");
            let e = entropy_service::ensure(rng, entropy_service::Bus::Mmio)?;
            if let Some(r) = e.wait_for_ready() {
                assert_eq!(
                    r[0],
                    entropy_protocol::READY,
                    "the entropy service did not come up, so no salt could be drawn",
                );
            }
            let svc =
                program("credentialer").expect("no credentialer program in the initrd archive");
            let w = cs::start(svc, e.request);
            let (fs_ep, fs_page_frame) =
                fs_service::root_directory(fs_service::blk_server_image(), redoxfs_server_image())?;

            let prov_image = program("identity_provisioner")
                .expect("no identity_provisioner program in the initrd archive");

            // Attempt one: a fresh identity. Expected `RPT_OK`.
            let r_a = ips::provision(
                prov_image,
                w.provision,
                w.provision_page_frame,
                fs_ep,
                fs_page_frame,
                IDENTITY,
                SECRET,
            );
            // Attempt two: the same identity again, before anyone has sealed the store. The subtree
            // this second attempt's `MKDIR` finds is the first attempt's own (`EEXIST`, tolerated),
            // so this isolates the credential half's own duplicate refusal.
            let r_b = ips::provision(
                prov_image,
                w.provision,
                w.provision_page_frame,
                fs_ep,
                fs_page_frame,
                IDENTITY,
                DUPLICATE_SECRET,
            );

            // Seal now, the way a real operator would once every identity for this boot is in:
            // `credentialer_test_client`'s provisioner role fills its own three people and three
            // shares and then seals. `chris` is already there from attempt one above, so that
            // role's own `PUT` for it is refused too (uninteresting here; not asserted), and
            // `corinne`/`graeme`/the three shares fill the rest of this six-slot store before the
            // seal that this suite's own verification step below depends on.
            let cli = program("credentialer_test_client")
                .expect("no credentialer_test_client program in the initrd archive");
            cs::provisioner(cli, &w);
            let ready = crate::sched::ipc_recv(w.ready);
            assert_eq!(
                ready[0],
                cs::RPT_READY,
                "the credential service did not reach phase two after the seal",
            );

            Some((w, r_a, r_b))
        })();

        if let Some((w, r_a, r_b)) = &result {
            for (slot, v) in SAVED.iter().zip([
                w.ready,
                w.verify,
                w.provision,
                r_a[0],
                r_a[1],
                r_b[0],
                r_b[1],
                w.verify_page_frame,
            ]) {
                slot.store(v, Ordering::Relaxed);
            }
            OK.store(true, Ordering::Release);
        }
        DONE.store(true, Ordering::Release);
    }
    if !OK.load(Ordering::Acquire) {
        return None;
    }
    let v = |i: usize| SAVED[i].load(Ordering::Relaxed);
    Some(Wired {
        cs: cs::Wiring {
            ready: v(0),
            verify: v(1),
            provision: v(2),
            // Carried across the memoization boundary because `credential_service::client` now
            // reads it directly (milestone 155's own fix to that module): a hardcoded 0 here once
            // looked harmless and was not, the first time this file shipped. `provision_page_frame` truly
            // is dead past setup (`ips::provision` is never called again after the seal above), so
            // it is the one field this struct does not bother carrying.
            verify_page_frame: v(7),
            provision_page_frame: 0,
        },
        fresh: [v(3), v(4)],
        duplicate: [v(5), v(6)],
    })
}

/// **The headline: a fresh identity gets a working credential and a real, descendable subtree, from
/// one tool invocation.** Two independent proofs, neither of which the tool's own report could fake:
/// a real `VERIFY` against what was `PUT` (`credentialer_test_client`'s `ROLE_HONEST`, reused
/// unmodified because this suite provisions the roster's first person), and a real subtree caretaker whose descent succeeds (`fs_service::narrow_dir`, which
/// asserts rather than returning `None` on a refused descent, so a subtree that was not actually
/// created fails this test loudly rather than silently).
#[test_case]
fn provisioning_creates_a_working_credential_and_a_real_subtree() {
    if fs_service::fs_server_image().is_none() {
        crate::testing::skip!(fs_service::NO_FS_SERVER);
    }
    let Some(w) = wired() else {
        crate::testing::skip!("no virtio-rng device or no RedoxFS disk attached");
    };
    assert_eq!(
        w.fresh,
        [ips::RPT_OK, 0],
        "provisioning a fresh identity did not report success",
    );

    let cli = program("credentialer_test_client")
        .expect("no credentialer_test_client program in the initrd archive");
    let honest = cs::client(cli, &w.cs, cs::ROLE_HONEST);
    assert_eq!(honest[0], cs::RPT_DONE, "the honest client did not report");
    assert_eq!(
        cs::nth(honest[1], 0),
        credential_protocol::MATCH,
        "the identity this tool PUT did not verify with the secret it was given",
    );

    let caretaker = program("fs_subtree_caretaker")
        .expect("no fs_subtree_caretaker program in the initrd archive");
    // `narrow_dir` asserts a successful descent internally (`wait_for_caretaker`); reaching this
    // line at all is the proof the subtree is real. See this program's own module docs on why a
    // panic here is the correct failure mode for this suite, matching every other filesystem test.
    let _ = fs_service::narrow_dir(
        fs_service::blk_server_image(),
        redoxfs_server_image(),
        caretaker,
        IDENTITY_STR,
        filesystem_protocol::dir::ALL,
    )
    .expect("the subtree identity_provisioner created did not open");
}

/// **A genuine duplicate is refused, and nothing already there is disturbed.** The second attempt
/// inside `wired()`'s setup reused `chris`, the identity the first attempt already provisioned; its
/// `MKDIR` found the subtree already there (`EEXIST`, tolerated, per this tool's own module docs)
/// and its credential `PUT` was refused as a duplicate (`credentialer::Store::put`'s own rule: "a duplicate
/// identity is refused"). This test asserts both halves of that outcome, and that the *original*
/// credential from attempt one still verifies afterward, which is the property the whole ordering
/// argument in `identity_provisioner.rs`'s module docs is for: a failed second attempt must not cost
/// the first one anything.
#[test_case]
fn a_duplicate_identity_is_refused_without_disturbing_the_original() {
    if fs_service::fs_server_image().is_none() {
        crate::testing::skip!(fs_service::NO_FS_SERVER);
    }
    let Some(w) = wired() else {
        crate::testing::skip!("no virtio-rng device or no RedoxFS disk attached");
    };
    assert_eq!(
        w.duplicate,
        [ips::RPT_CRED_FAILED, credential_protocol::MALFORMED],
        "a genuine duplicate PUT was not refused the way credentialer::Store::put's own rule says it must \
         be (a duplicate identity answers MALFORMED, the same code a malformed request gets, \
         because neither is an authentication outcome)",
    );

    let cli = program("credentialer_test_client")
        .expect("no credentialer_test_client program in the initrd archive");
    let honest = cs::client(cli, &w.cs, cs::ROLE_HONEST);
    assert_eq!(honest[0], cs::RPT_DONE, "the honest client did not report");
    assert_eq!(
        cs::nth(honest[1], 0),
        credential_protocol::MATCH,
        "the refused second PUT disturbed the identity the first, successful PUT already stored",
    );
}
