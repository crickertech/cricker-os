//! **A plain `SEND` received by `RECV_CAP` hands the receiver `NO_CAP` in `x1`, never a
//! sender-chosen word** (milestone 634 (a plain SEND received by RECV_CAP never hands the receiver
//! a sender-chosen slot), fatal risk 7's confinement claim).
//!
//! **The defect these tests were written to demonstrate was live on `main` and is fixed here.**
//! `RECV_CAP` returns in `x1` the slot a delivered capability landed in, or `NO_CAP` when the
//! message carried none. A `CALL` server reads it as the slot of the one-shot `Reply` capability.
//! On `main`, a plain `SEND` (three data words, no capability) received by `RECV_CAP` left the
//! sender's second word in `x1` when the receiver had parked first, because `ipc_send` drops its
//! three words straight into the receiver's mailbox and the old `ipc_recv_cap` returned that word
//! unchanged. So a client could hand a server a slot number of its own choosing. The sender-first
//! order already returned `NO_CAP`; the fix makes the receiver-first order match it, by having the
//! receive side write `NO_CAP` unless a capability was actually installed for the delivery
//! (`Thread::cap_delivered`, `kernel/src/sched.rs`).
//!
//! **Why this is a confinement claim.** The audit by milestone 613 (a system log service: the
//! in-memory half), PR #1494, found no `RECV_CAP`
//! consumer anywhere that checks the kind of object in the received slot, and no ABI call that
//! would let one. The only guard in the tree is `x1 == NO_CAP`, which did nothing on the
//! receiver-first order. The worst live case is `net_stack` (`components/src/net_stack.rs:227`),
//! reachable by any program that declares network, whose `OP_ATTACH_PAGE_FRAME` does
//! `cap_delete(x1)` with no guard: a chosen slot deletes one of the server's own capabilities. The
//! throwaway attack programs that drove the two escapes under QEMU on 2026-10-03 UTC are described
//! in `design/roadmap/634-*.md`; these are the property they reduce to.

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::cap::{Rights, rendezvous_cap};
use crate::sched;

fn wait_for(mut cond: impl FnMut() -> bool) -> bool {
    let deadline = crate::arch::timer::now() + 2 * crate::arch::timer::frequency();
    while crate::arch::timer::now() < deadline {
        if cond() {
            return true;
        }
        sched::yield_now();
    }
    cond()
}

/// A sentinel the attacker's second word carries. It is not `NO_CAP`, so a receiver that returned
/// it in `x1` would be returning a sender-chosen number.
const CHOSEN: u64 = 7;

/// **Both arrival orders deliver `NO_CAP` in `x1` for a plain `SEND`.** Receiver-first was the
/// exploitable order on `main`; sender-first always returned `NO_CAP`. Asserting both is what makes
/// the fix's symmetry the property rather than one patched branch.
///
/// Falsification: replayable `system_tests/falsifications/user.recv_cap_attack_tests.a_plain_send_to_recv_cap_delivers_no_cap_whichever_side_parks_first.patch`
#[test_case]
fn a_plain_send_to_recv_cap_delivers_no_cap_whichever_side_parks_first() {
    static RECEIVER_FIRST_X1: AtomicU64 = AtomicU64::new(0);
    static SERVER_PARKED: AtomicBool = AtomicBool::new(false);
    static RECEIVER_FIRST_DONE: AtomicBool = AtomicBool::new(false);

    let region = crate::memory_region::create(4).expect("no region");
    let ep = sched::create_rendezvous_from(region).expect("no rendezvous");

    // Receiver-first: the server parks in RECV_CAP, then the attacker sends a chosen word.
    sched::spawn(move || {
        SERVER_PARKED.store(true, Ordering::SeqCst);
        let [_w0, x1, ..] = sched::ipc_recv_cap(ep);
        RECEIVER_FIRST_X1.store(x1, Ordering::SeqCst);
        RECEIVER_FIRST_DONE.store(true, Ordering::SeqCst);
    })
    .expect("no server thread");
    assert!(
        wait_for(
            || SERVER_PARKED.load(Ordering::SeqCst) && sched::rendezvous_waiting_receivers(ep) == 1
        ),
        "the server never parked in RECV_CAP, so nothing proved the receiver-first order",
    );
    sched::ipc_send(ep, [1, CHOSEN, 0]);
    assert!(
        wait_for(|| RECEIVER_FIRST_DONE.load(Ordering::SeqCst)),
        "the receiver-first server never returned from RECV_CAP",
    );
    assert_eq!(
        RECEIVER_FIRST_X1.load(Ordering::SeqCst),
        abi::rendezvous::NO_CAP,
        "a plain SEND handed the receiver x1 = {} on the receiver-first order, a sender-chosen \
         word where a CALL server reads a reply slot",
        RECEIVER_FIRST_X1.load(Ordering::SeqCst),
    );

    // Sender-first: the attacker parks as a sender, then this thread receives.
    static SENT: AtomicBool = AtomicBool::new(false);
    sched::spawn(move || {
        SENT.store(true, Ordering::SeqCst);
        sched::ipc_send(ep, [1, CHOSEN, 0]);
    })
    .expect("no attacker thread");
    assert!(
        wait_for(|| SENT.load(Ordering::SeqCst) && sched::rendezvous_waiting_senders(ep) == 1),
        "the attacker never parked as a sender, so nothing proved the sender-first order",
    );
    let [_w0, x1, ..] = sched::ipc_recv_cap(ep);
    assert_eq!(
        x1,
        abi::rendezvous::NO_CAP,
        "a plain SEND handed the receiver x1 = {x1} on the sender-first order",
    );

    crate::memory_region::destroy(region);
}

const VICTIM_SLOT: u64 = 7;

/// **A server that deletes the received slot deletes nothing of its own** (the `net_stack` shape).
/// The server grants itself a capability at a fixed slot, receives a plain `SEND`, and
/// unconditionally deletes `x1`. With `x1 == NO_CAP` the delete finds nothing, so the server's own
/// capability survives on both orders. On `main` the receiver-first order deleted it.
#[test_case]
fn a_server_that_deletes_the_received_slot_deletes_nothing_of_its_own() {
    for receiver_first in [true, false] {
        static SURVIVED: AtomicBool = AtomicBool::new(false);
        static DONE: AtomicBool = AtomicBool::new(false);
        static READY: AtomicBool = AtomicBool::new(false);
        SURVIVED.store(false, Ordering::SeqCst);
        DONE.store(false, Ordering::SeqCst);
        READY.store(false, Ordering::SeqCst);

        let region = crate::memory_region::create(4).expect("no region");
        let ep = sched::create_rendezvous_from(region).expect("no rendezvous");
        let victim_ep = sched::create_rendezvous_from(region).expect("no victim rendezvous");

        let server = move || {
            sched::grant_at(VICTIM_SLOT, rendezvous_cap(victim_ep, Rights::WRITE))
                .expect("the server could not hold its victim capability");
            READY.store(true, Ordering::SeqCst);
            let [_w0, x1, ..] = sched::ipc_recv_cap(ep);
            let _ = sched::delete_current_cap(x1); // net_stack's unconditional cap_delete(x1)
            SURVIVED.store(sched::current_cap(VICTIM_SLOT).is_ok(), Ordering::SeqCst);
            let _ = sched::delete_current_cap(VICTIM_SLOT); // leave this thread's table clean
            DONE.store(true, Ordering::SeqCst);
        };

        if receiver_first {
            sched::spawn(server).expect("no server thread");
            assert!(
                wait_for(
                    || READY.load(Ordering::SeqCst) && sched::rendezvous_waiting_receivers(ep) == 1
                ),
                "the server never parked (receiver-first)",
            );
            sched::ipc_send(ep, [1, VICTIM_SLOT, 0]);
        } else {
            static SENT: AtomicBool = AtomicBool::new(false);
            SENT.store(false, Ordering::SeqCst);
            sched::spawn(move || {
                SENT.store(true, Ordering::SeqCst);
                sched::ipc_send(ep, [1, VICTIM_SLOT, 0]);
            })
            .expect("no attacker thread");
            assert!(
                wait_for(
                    || SENT.load(Ordering::SeqCst) && sched::rendezvous_waiting_senders(ep) == 1
                ),
                "the attacker never parked (sender-first)",
            );
            sched::spawn(server).expect("no server thread");
        }

        assert!(
            wait_for(|| DONE.load(Ordering::SeqCst)),
            "the server never finished"
        );
        assert!(
            SURVIVED.load(Ordering::SeqCst),
            "the server deleted its own capability at the attacker-chosen slot (receiver_first = {receiver_first})",
        );
        crate::memory_region::destroy(region);
    }
}
