//! **The shared-frame witness** (milestone 599 (a frame per filesystem client channel), provisional).
//!
//! Finding 1 of `notes/shared-page-audit.md`: the FS server shared one read-write staging frame
//! with every client a boot wired, so what kept two clients apart was which of them was blocked, not
//! what each could map. This test builds two live clients on one file service, one rewriting the
//! frame the other staged its name in, and asserts the outcome. **Since milestone 599 gave each
//! client its own channel window** (keyed by the badge on its endpoint capability), the two map
//! different frames, so it now asserts isolation: the attacker's write cannot reach the victim's
//! window, and the victim's request resolves the name it staged. It runs a negative control
//! first: both clients on one window, the old wiring, where the substitution must land. A pass on
//! separate windows means something only because the same witness substitutes on a shared one.
//!
//! One module for both ISAs, for `dir_capability_tests`'s reason: nothing here is
//! architecture-specific, so the parity gate (DECISIONS §19 (architectural parity is a tenet)) is met by the same test running on
//! every architecture `script/test` boots. It wires two portable `fs_test_client` roles and one
//! portable `redoxfs_server`, and asserts on a verdict word.
//!
//! **Why a handshake and not a race.** The property under test is not "does a race fire" but "can a
//! second holder of the frame change the bytes the server resolves for another client's call". The
//! server reads the name from the page after the message arrives and has no way to tell whose bytes
//! it read. A two-message handshake over a sync endpoint forces the interleaving the audit
//! describes (victim stages its name, attacker overwrites it, victim calls) deterministically, so
//! the witness runs the same interleaving on every architecture and every run, not only when a race
//! lands.
//! The interleaving is one a genuinely runnable second writer reaches by chance on the four-core
//! boots; the handshake removes the flakiness, not the mechanism.

use filesystem_protocol::fixture;

use super::*;
use crate::sched;

/// The two `fs_test_client` roles, mirrored from `fixtures/src/fs_test_client.rs`. The numbers live
/// on both sides because one program is started by role, the same shape as every other role here.
const ROLE_SHARE_VICTIM: u64 = 13;
const ROLE_SHARE_ATTACKER: u64 = 14;

/// Run the witness once and return the victim's verdict, or `None` when there is no disk.
/// `shared` puts both clients on one window (the pre-599 wiring); see [`fs_service::start_shared_frame_witness`].
///
/// A helper returns `None` rather than calling `skip!`, because `skip!` returns from the function
/// it is written in and only the `#[test_case]` can honestly skip.
fn witness(shared: bool) -> Option<u64> {
    let (_readiness, report) = fs_service::start_shared_frame_witness(
        fs_service::blk_server_image(),
        program("redoxfs_server").expect("no redoxfs_server program in the initrd archive"),
        program("fs_test_client").expect("no fs_test_client program in the initrd archive"),
        ROLE_SHARE_VICTIM,
        ROLE_SHARE_ATTACKER,
        shared,
    )?;
    let [verdict, w1, ..] = sched::ipc_recv(report);
    assert_ne!(
        verdict,
        fixture::SHARED_UNEXPECTED,
        "the witness is wired wrong (shared = {shared}): the victim's open failed or returned \
         neither file's body (errno/aux {w1:#x})",
    );
    Some(verdict)
}

/// **Two live clients on one file service; one rewrites the frame the other staged its name in.**
///
/// The victim stages `fixture::SHARED_VICTIM_NAME`, the attacker writes
/// `fixture::SHARED_USURPER_NAME` into the frame it maps, and then the victim calls. The victim
/// reports which file it actually got.
///
/// **Two runs, and the first is the control.** On one shared window (the wiring every client had
/// before milestone 599) the attacker's write lands in the victim's frame and the server resolves
/// the attacker's name: `SHARED_SUBSTITUTED`. That run failing to substitute would mean the attack
/// or the handshake is broken, and then the second run's pass would prove nothing. On separate
/// windows, each keyed by its endpoint's badge, the attacker cannot reach the victim's frame and
/// the victim's own name resolves: `SHARED_ISOLATED`, which is the fix.
#[test_case]
fn a_second_client_cannot_substitute_the_name_on_its_own_window() {
    if fs_service::fs_server_image().is_none() {
        crate::testing::skip!(fs_service::NO_FS_SERVER);
    }
    let Some(control) = witness(true) else {
        crate::testing::skip!("no RedoxFS disk attached");
    };
    assert_eq!(
        control,
        fixture::SHARED_SUBSTITUTED,
        "the negative control did not substitute ({control:#x}): with both clients on one window \
         the attacker's write must land in the victim's frame. If it does not, the handshake or \
         the attack is broken, and the isolated run below would pass for the wrong reason.",
    );
    let isolated = witness(false).expect("the service was wired by the control run");
    assert_eq!(
        isolated,
        fixture::SHARED_ISOLATED,
        "the victim read the attacker's file ({isolated:#x}) on separate windows, so a second \
         client still reached the frame the server resolved the victim's name from. Each client's \
         window is keyed by its endpoint's badge and must be its own.",
    );
}
