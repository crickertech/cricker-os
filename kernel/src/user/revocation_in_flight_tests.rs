//! **Revocation against a capability that is in flight** (risk 7's adversarial pass, 2026-09-21).
//!
//! **The defect this module was written to demonstrate was live in the tree and is fixed here.**
//! What follows describes the tree as it stood on 2026-09-21; the three sweeps now clear the
//! hand-off slot too, and `kernel/falsifications/` carries the patch that puts the defect back and
//! turns this test red.
//!
//! Every revocation sweep in this kernel walked `Thread::capability_table` and stopped there. A
//! capability handed to a rendezvous whose receiver has not arrived yet is not in any table: it is
//! parked in `Thread::outgoing_cap`, the hand-off slot `sched::ipc_send_cap` writes and
//! `sched::ipc_recv_cap` takes. So a `PageFrame::REVOKE` that runs in that window deletes every
//! capability the sweep can see, unmaps every page the log records, and leaves a live capability
//! naming the revoked run sitting in a slot no sweep read. The next `RECV_CAP` filed it in the
//! receiver's own table, and the receiver could then `MAP` a page the revoker believed it took
//! back.
//!
//! **The tree already wrote this lesson down, once, for a different object.**
//! `sched::delete_reply_caps_naming` sweeps `outgoing_cap` beside the tables and says why in its own
//! doc comment: *"`outgoing_cap` goes too, and it is the half a second copy would forget ... a live
//! `Reply` in a hand-off slot is the same forgery one step earlier."* That is exactly this defect,
//! stated by the one sweep that did not have the defect. The three that did
//! (`sched::delete_page_frame_caps_where`, which is both `PageFrame` policies,
//! `sched::delete_device_frame_caps_from_others`, and `x86_64`'s
//! `sched::delete_port_range_caps_impl`) each read `t.capability_table` alone. All three clear the
//! hand-off slot now.
//!
//! **Why this is a confinement claim and not a tidiness one.** `notes/confinement-claims.md` row 4
//! is the *a consumed capability cannot be used again* of DECISIONS §12 (call/reply IPC: a
//! one-shot reply capability), and the same note's opening says
//! the enumeration cannot reach a claim nobody made. `crate::revoke::revoke_region` is the sharper
//! case: `MemoryRegion::DESTROY` returns the region's pages to an allocator that will hand them out
//! again, and its capability sweep exists precisely so that "no capability still names a page this
//! allocator is about to hand out" is true. An in-flight capability makes it false, which is
//! the use-after-free DECISIONS §13 (capability revocation and untyped reclamation) exists to
//! prevent, reopened through the one slot the sweep does not look in.
//!
//! **This module is a new file on purpose.** `kernel/src/user/tests.rs` is this tree's worst merge
//! hotspot and AGENTS.md's lane rules say to stay out of it.
//!
//! # BUGS
//!
//! - **One object, one sweep.** The test below drives the `PageFrame` sweep. The `DeviceFrame` and
//!   `PortRange` sweeps carry the same two lines and no test drives either through a parked
//!   hand-off, so their correctness is reasoned from the code rather than measured, which is the
//!   grade `notes/confinement-claims.md` already asks a reader to apply to an unmeasured verdict.
//!   The `PortRange` case has a second limb that is not tested here either:
//!   `ipc_recv_cap` files the delivered capability with
//!   `CapabilityTable::insert` directly rather than through
//!   `sched::thread_control_block_insert_cap`, so the receiver holds a `PortRange` capability whose
//!   `Thread::port_range_grant` was never set. That direction fails closed.
//! - **The test proves delivery, not exploitation.** It stops at "the receiver holds a capability
//!   naming the revoked run". Mapping it and reading a page the revoker reclaimed would be the
//!   stronger demonstration and needs a region destroy plus a re-retype to be worth anything.

use core::sync::atomic::{AtomicBool, Ordering};

use crate::cap::{Object, Rights, page_frame_run_cap, page_frame_run_len};
use crate::sched;

/// **A capability revoked while it sits in the hand-off slot must not reach the receiver.**
///
/// The sequence is the one a shared-frame un-share actually takes, with one thread parked in the
/// middle of it:
///
/// 1. A sender holds `PageFrame(phys, 1)` in its own table and hands a copy to a rendezvous nobody
///    is receiving on yet. `ipc_send_cap` parks the copy in `outgoing_cap` and blocks it.
/// 2. `PageFrame::REVOKE` runs over that frame, whole-machine. The sender's own table slot goes,
///    which this test checks so that a revoke which never reached the sender cannot be mistaken for
///    a revoke which reached it and worked.
/// 3. The receiver arrives. It must be handed nothing.
///
/// **Which assertion fires, since this tree has been bitten three times by the readable one being
/// unreachable** (`notes/confinement-claims.md`, milestones 305 and 307): the headline is the last
/// assertion and it is reached on every path, because `ipc_recv_cap` returns either `NO_CAP` or a
/// slot and both are inspected. The assertions above it are a vacuity guard (the sender really did
/// park) and a premise check (the revoke really did reach the sender's table); either failing means
/// this test proved nothing, which is why they say that rather than stating the claim a second time.
///
/// Falsification: replayable `kernel/falsifications/user.revocation_in_flight_tests.a_capability_revoked_while_it_is_in_flight_does_not_reach_the_receiver.patch`
#[test_case]
fn a_capability_revoked_while_it_is_in_flight_does_not_reach_the_receiver() {
    static PARKED: AtomicBool = AtomicBool::new(false);
    static RETURNED: AtomicBool = AtomicBool::new(false);
    /// Did the sender still hold the frame in its **own table** once its send completed? The
    /// premise check: a `false` here is the revoke having reached the one holder a sweep can see.
    static SENDER_STILL_HELD: AtomicBool = AtomicBool::new(true);

    let region = crate::memory_region::create(4).expect("no region");
    let ep = sched::create_rendezvous_from(region).expect("no rendezvous");
    let phys = crate::memory_region::retype_page(region).expect("no page to revoke");

    // The sender holds the capability in its table, the way a real delegator does, and hands a copy
    // to the rendezvous. Nobody is receiving, so `ipc_send_cap` parks the copy in `outgoing_cap`
    // and blocks. Holding it in the table too is what lets step 2 be checked.
    sched::spawn(move || {
        let slot = sched::grant(page_frame_run_cap(phys, page_frame_run_len(1), Rights::ALL))
            .expect("the sender could not be granted the frame it is about to delegate");
        let cap = sched::current_cap(slot).expect("the grant did not land");
        PARKED.store(true, Ordering::SeqCst);
        sched::ipc_send_cap(ep, 0, cap, 0);
        SENDER_STILL_HELD.store(sched::current_cap(slot).is_ok(), Ordering::SeqCst);
        RETURNED.store(true, Ordering::SeqCst);
    })
    .expect("no sender thread");

    // Clock-bounded rather than yield-counted: under DECISIONS §28 (SMP placement at spawn) the
    // sender is on some other core, so this core's yields say nothing about whether it has run.
    assert!(
        super::wait_for(
            || PARKED.load(Ordering::SeqCst) && sched::rendezvous_waiting_senders(ep) == 1
        ),
        "the sender never parked on the rendezvous, so nothing was ever in flight to revoke",
    );

    crate::revoke::revoke_page_frame(phys);

    let [_word, slot, _second, _badge] = sched::ipc_recv_cap(ep);
    let delivered = if slot == abi::rendezvous::NO_CAP {
        None
    } else {
        sched::current_cap(slot).ok()
    };

    assert!(
        super::wait_for(|| RETURNED.load(Ordering::SeqCst)),
        "the sender never returned from its send, so its table could not be re-read",
    );
    assert!(
        !SENDER_STILL_HELD.load(Ordering::SeqCst),
        "the revoke did not reach the sender's own capability table, so this test's premise is \
         false and its result means nothing",
    );

    assert!(
        !matches!(delivered, Some(c) if c.object == Object::PageFrame(phys, page_frame_run_len(1))),
        "a revoked PageFrame capability was delivered out of the hand-off slot: revocation swept \
         every capability table and not `outgoing_cap`, so the receiver now names a run the \
         revoker took back",
    );

    crate::memory_region::destroy(region);
}
