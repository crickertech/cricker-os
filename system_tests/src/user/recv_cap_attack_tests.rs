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

/// **A `SEND_CAP` aborted by its rendezvous's teardown stages nothing for a later plain `SEND`**
/// (the 2026-10-03 security audit's follow-up, item (c)). A sender parked in `SEND_CAP` keeps its
/// delegation in `Thread::outgoing_cap` until a receiver takes it. `reclaim_region` drains the
/// rendezvous and wakes the sender aborted; before the fix in `set_ipc_aborted` the capability
/// stayed staged, and the sender's next plain `SEND`, parked on an unrelated rendezvous, handed it
/// to whoever `RECV_CAP`ed there: a delegation made to one endpoint, delivered to another.
#[test_case]
fn a_send_cap_aborted_by_teardown_stages_nothing_for_a_later_plain_send() {
    static PARKED_ON_DOOMED: AtomicBool = AtomicBool::new(false);
    static ABORTED: AtomicBool = AtomicBool::new(false);
    static DONE: AtomicBool = AtomicBool::new(false);

    let doomed = crate::memory_region::create(4).expect("no doomed region");
    let doomed_ep = sched::create_rendezvous_from(doomed).expect("no doomed rendezvous");
    let region = crate::memory_region::create(4).expect("no region");
    let other_ep = sched::create_rendezvous_from(region).expect("no other rendezvous");
    // Any capability will do as the delegation; what matters is that it must not arrive at
    // `other_ep`.
    let delegated = rendezvous_cap(other_ep, Rights::WRITE);

    sched::spawn(move || {
        PARKED_ON_DOOMED.store(true, Ordering::SeqCst);
        sched::ipc_send_cap(doomed_ep, 1, delegated, 0);
        // The teardown below wakes us aborted; a syscall would read and clear the flag here.
        let _ = sched::take_ipc_aborted();
        ABORTED.store(true, Ordering::SeqCst);
        sched::ipc_send(other_ep, [2, CHOSEN, 0]);
        DONE.store(true, Ordering::SeqCst);
    })
    .expect("no sender thread");
    assert!(
        wait_for(|| PARKED_ON_DOOMED.load(Ordering::SeqCst)
            && sched::rendezvous_waiting_senders(doomed_ep) == 1),
        "the sender never parked in SEND_CAP on the doomed rendezvous",
    );
    sched::reclaim_region(doomed).expect("the doomed region did not reclaim");
    assert!(
        wait_for(
            || ABORTED.load(Ordering::SeqCst) && sched::rendezvous_waiting_senders(other_ep) == 1
        ),
        "the sender never came back aborted and parked its plain SEND on the other rendezvous",
    );
    let [w0, x1, ..] = sched::ipc_recv_cap(other_ep);
    assert_eq!(w0, 2, "the plain SEND's first word did not arrive");
    assert_eq!(
        x1,
        abi::rendezvous::NO_CAP,
        "a plain SEND after an aborted SEND_CAP delivered the stale delegation at slot {x1}",
    );
    assert!(
        wait_for(|| DONE.load(Ordering::SeqCst)),
        "the sender never finished"
    );
    crate::memory_region::destroy(region);
}

/// **A `SEND_CAP` collected by a plain `RECV` stages nothing for a later plain `SEND`** (milestone
/// 633 (an outside agent attacks the confinement claim), fatal risk 7's first outsider pass,
/// 2026-10-03 UTC). The non-abort sibling of
/// `a_send_cap_aborted_by_teardown_stages_nothing_for_a_later_plain_send`.
///
/// There, teardown wakes the parked `SEND_CAP` sender *aborted*, and `set_ipc_aborted` clears its
/// staged delegation. Here the sender is woken by a plain `RECV` that *successfully* collects it
/// (`ipc_recv`'s `!leave_blocked` branch). That path does not go through `set_ipc_aborted`, so
/// before this lane's fix the delegation stayed in `Thread::outgoing_cap`, and the sender's next
/// plain `SEND`, parked on an unrelated rendezvous, handed it to whoever `RECV_CAP`ed there: a
/// delegation made to one endpoint, delivered to another.
///
/// **Milestone 634's `cap_delivered` guard does not catch this.** That guard covers the order where
/// the *receiver* parks first (`ipc_recv_cap`'s blocked path reads `cap_delivered`); this escape
/// takes `outgoing_cap` on the order where the *sender* parks first (`ipc_recv_cap`'s immediate
/// `FromSender` path, which calls `outgoing_cap.take()` unconditionally). A plain `SEND` cannot set
/// `cap_delivered`, so the leak is the stale `outgoing_cap`, not the mailbox slot 634 fixed.
///
/// Falsification: replayable `system_tests/falsifications/user.recv_cap_attack_tests.a_send_cap_collected_by_a_plain_recv_stages_nothing_for_a_later_plain_send.patch`
#[test_case]
fn a_send_cap_collected_by_a_plain_recv_stages_nothing_for_a_later_plain_send() {
    static PARKED_ON_FIRST: AtomicBool = AtomicBool::new(false);
    static DONE: AtomicBool = AtomicBool::new(false);

    let first = crate::memory_region::create(4).expect("no first region");
    let first_ep = sched::create_rendezvous_from(first).expect("no first rendezvous");
    let region = crate::memory_region::create(4).expect("no region");
    let other_ep = sched::create_rendezvous_from(region).expect("no other rendezvous");
    // Any capability will do as the delegation; what matters is that it must not arrive at
    // `other_ep`, where the sender only does a plain `SEND`.
    let delegated = rendezvous_cap(other_ep, Rights::WRITE);

    sched::spawn(move || {
        PARKED_ON_FIRST.store(true, Ordering::SeqCst);
        // Parks as a SEND_CAP sender on first_ep; the plain RECV below collects it and delivers no
        // capability, completing the rendezvous without an abort.
        sched::ipc_send_cap(first_ep, 1, delegated, 0);
        // Then a plain SEND on an unrelated rendezvous. If the delegation stayed staged, this hands
        // it to the RECV_CAP below.
        sched::ipc_send(other_ep, [2, CHOSEN, 0]);
        DONE.store(true, Ordering::SeqCst);
    })
    .expect("no sender thread");
    assert!(
        wait_for(|| PARKED_ON_FIRST.load(Ordering::SeqCst)
            && sched::rendezvous_waiting_senders(first_ep) == 1),
        "the sender never parked in SEND_CAP on the first rendezvous",
    );
    // Plain RECV: takes the data word, no capability, and wakes the sender through the successful
    // path (not set_ipc_aborted). This is the vacuity guard's premise: a SEND_CAP really was in
    // flight and was collected by a receiver that cannot hold a capability.
    let [w0, ..] = sched::ipc_recv(first_ep);
    assert_eq!(
        w0, 1,
        "the SEND_CAP's data word did not arrive at the plain RECV, so nothing was collected",
    );
    assert!(
        wait_for(|| sched::rendezvous_waiting_senders(other_ep) == 1),
        "the sender never parked its plain SEND on the other rendezvous",
    );
    let [w0, x1, ..] = sched::ipc_recv_cap(other_ep);
    assert_eq!(w0, 2, "the plain SEND's first word did not arrive");
    assert_eq!(
        x1,
        abi::rendezvous::NO_CAP,
        "a plain SEND after a plain-RECV-collected SEND_CAP delivered the stale delegation at slot \
         {x1}, a capability granted to one endpoint reaching a receiver on another",
    );
    assert!(
        wait_for(|| DONE.load(Ordering::SeqCst)),
        "the sender never finished"
    );
    crate::memory_region::destroy(region);
}
