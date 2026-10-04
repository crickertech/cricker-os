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

/// How many slots of the running thread's capability table hold something.
fn held() -> usize {
    (0..crate::cap::CAPABILITY_TABLE_SLOTS as u64)
        .filter(|&slot| sched::current_cap(slot).is_ok())
        .count()
}

/// **A `CALL` server told by `x4` answers its caller and keeps nothing a delegation left behind**
/// (milestone 706 (a `CALL` server can tell a Reply from a delegation), DECISIONS §245 (a `CALL`
/// server tells a Reply from a delegation), fatal risk 7's confinement claim).
///
/// The attack: a client `SEND_CAP`s a rendezvous nobody receives on to a `CALL` server. Before 706
/// `x1` was a real slot either way, so a server that answered it ran `SEND` on that rendezvous and
/// parked for the life of the machine, and a server that did not answer kept the slot. The server
/// here is written the way `user_mode_runtime::recv_request` reads a receive: `x4 ==
/// REPLY_DELIVERED` is a Reply to answer, and any other real slot is a delegation to delete. It
/// takes the delegation and then a real `CALL`, on both arrival orders, and must answer the caller
/// and end holding exactly what it held before.
///
/// **What this tests is the tag, not the hang.** A server that invokes `REPLY` on the delegation
/// does park for good, but a test that demonstrated it would need to end a thread blocked in
/// `SEND`, which needs the timed receive milestone 417 (a usurper that reports instead of hanging)
/// waits on; and the hang was never in doubt, since it is method `0` on a rendezvous. What 706 adds
/// is that the kernel says which slot is the Reply, so that is what is asserted: without the tag
/// the server cannot tell the two apart, deletes the Reply as a delegation, and its caller is never
/// answered.
///
/// Falsification: replayable `system_tests/falsifications/user.recv_cap_attack_tests.a_call_server_tells_its_reply_from_a_delegation_on_both_arrival_orders.patch`
#[test_case]
fn a_call_server_tells_its_reply_from_a_delegation_on_both_arrival_orders() {
    const DELEGATION: u64 = 1;
    const REQUEST: u64 = 2;
    const ANSWER: u64 = 0x706;

    for server_first in [true, false] {
        static RECEIVED: AtomicU64 = AtomicU64::new(0);
        static SERVER_DONE: AtomicBool = AtomicBool::new(false);
        static HELD_BEFORE: AtomicU64 = AtomicU64::new(0);
        static HELD_AFTER: AtomicU64 = AtomicU64::new(0);
        static DELEGATION_X4: AtomicU64 = AtomicU64::new(u64::MAX);
        static CALL_X4: AtomicU64 = AtomicU64::new(u64::MAX);
        static CALL_ANSWER: AtomicU64 = AtomicU64::new(0);
        static CALL_DONE: AtomicBool = AtomicBool::new(false);
        static DELEGATION_SENT: AtomicBool = AtomicBool::new(false);
        RECEIVED.store(0, Ordering::SeqCst);
        SERVER_DONE.store(false, Ordering::SeqCst);
        DELEGATION_X4.store(u64::MAX, Ordering::SeqCst);
        CALL_X4.store(u64::MAX, Ordering::SeqCst);
        CALL_ANSWER.store(0, Ordering::SeqCst);
        CALL_DONE.store(false, Ordering::SeqCst);
        DELEGATION_SENT.store(false, Ordering::SeqCst);

        let region = crate::memory_region::create(4).expect("no region");
        let ep = sched::create_rendezvous_from(region).expect("no rendezvous");
        // The attacker's delegation: a rendezvous nobody ever receives on, so a `REPLY` (method 0,
        // `SEND` on a rendezvous) through it would never return.
        let dead_end = sched::create_rendezvous_from(region).expect("no dead-end rendezvous");
        let delegated = rendezvous_cap(dead_end, Rights::WRITE);

        let server = move || {
            HELD_BEFORE.store(held() as u64, Ordering::SeqCst);
            for _ in 0..2 {
                let [w0, x1, _w1, _badge, x4] = sched::ipc_recv_cap(ep);
                if w0 == DELEGATION {
                    DELEGATION_X4.store(x4, Ordering::SeqCst);
                } else {
                    CALL_X4.store(x4, Ordering::SeqCst);
                }
                RECEIVED.fetch_add(1, Ordering::SeqCst);
                if x4 == abi::rendezvous::REPLY_DELIVERED {
                    // The tag's claim, checked against the table: the slot is a Reply.
                    if let Ok(crate::cap::Cap {
                        object: crate::cap::Object::Reply(caller),
                        ..
                    }) = sched::current_cap(x1)
                    {
                        sched::ipc_reply(caller, [ANSWER, 0]);
                    }
                    let _ = sched::delete_current_cap(x1);
                } else if x1 != abi::rendezvous::NO_CAP {
                    let _ = sched::delete_current_cap(x1); // `Delivered::into_reply`'s delete
                }
            }
            HELD_AFTER.store(held() as u64, Ordering::SeqCst);
            SERVER_DONE.store(true, Ordering::SeqCst);
        };
        let caller = move || {
            let [r0, ..] = sched::ipc_call(ep, [REQUEST, 0]);
            CALL_ANSWER.store(r0, Ordering::SeqCst);
            CALL_DONE.store(true, Ordering::SeqCst);
        };
        let attacker = move || {
            DELEGATION_SENT.store(true, Ordering::SeqCst);
            sched::ipc_send_cap(ep, DELEGATION, delegated, 0);
        };

        if server_first {
            // The server parks, and each message reaches it through the sender's own delivery
            // (`ipc_send_cap`'s and `ipc_call_badged`'s rendezvous arms).
            sched::spawn(server).expect("no server thread");
            assert!(
                wait_for(|| sched::rendezvous_waiting_receivers(ep) == 1),
                "the server never parked in RECV_CAP (server first)",
            );
            sched::spawn(attacker).expect("no attacker thread");
            assert!(
                wait_for(|| RECEIVED.load(Ordering::SeqCst) == 1
                    && sched::rendezvous_waiting_receivers(ep) == 1),
                "the server never took the delegation and parked again (server first)",
            );
            sched::spawn(caller).expect("no caller thread");
        } else {
            // Both clients park first, and the server collects them (`ipc_recv_cap`'s
            // `FromSender` arm), the delegation ahead of the call.
            sched::spawn(attacker).expect("no attacker thread");
            assert!(
                wait_for(|| DELEGATION_SENT.load(Ordering::SeqCst)
                    && sched::rendezvous_waiting_senders(ep) == 1),
                "the attacker never parked in SEND_CAP (clients first)",
            );
            sched::spawn(caller).expect("no caller thread");
            assert!(
                wait_for(|| sched::rendezvous_waiting_senders(ep) == 2),
                "the caller never parked behind the attacker (clients first)",
            );
            sched::spawn(server).expect("no server thread");
        }

        assert!(
            wait_for(|| SERVER_DONE.load(Ordering::SeqCst)),
            "the server never took both messages (server_first = {server_first}); one of them \
             parked it, which is the hang 706 closes",
        );
        assert!(
            wait_for(|| CALL_DONE.load(Ordering::SeqCst)),
            "the caller was never answered (server_first = {server_first}): the server could not \
             tell its Reply from the delegation (x4 = {:#x} on the CALL), so it deleted the Reply",
            CALL_X4.load(Ordering::SeqCst),
        );
        assert_eq!(
            CALL_ANSWER.load(Ordering::SeqCst),
            ANSWER,
            "the caller woke with something other than the server's answer",
        );
        assert_eq!(
            CALL_X4.load(Ordering::SeqCst),
            abi::rendezvous::REPLY_DELIVERED,
            "a CALL's delivery did not carry REPLY_DELIVERED in x4 (server_first = {server_first})",
        );
        assert_eq!(
            DELEGATION_X4.load(Ordering::SeqCst),
            0,
            "a SEND_CAP delegation arrived tagged as a Reply (server_first = {server_first})",
        );
        assert_eq!(
            HELD_AFTER.load(Ordering::SeqCst),
            HELD_BEFORE.load(Ordering::SeqCst),
            "the server ended holding a slot it did not hold before (server_first = \
             {server_first}): a delivery was kept rather than answered or deleted",
        );
        crate::memory_region::destroy(region);
    }
}

/// An interrupt number no device on any of the three machines raises, so binding it steals no real
/// route (the reasoning, and the value, are `irq_send_refusal_tests`'). `bind_irq` has no unbind,
/// so a neighbour that rebinds it simply replaces this test's route.
const QUIET_INTID: u32 = 250;

/// **An interrupt signal received by `RECV_CAP` delivers `NO_CAP` in `x1` whichever side arrives
/// first** (milestone 714 (the sibling `RECV_CAP` paths get a receiver-first test), fatal risk 7's
/// confinement claim; the sibling of milestone 634's plain-`SEND` test above).
///
/// Before 634 `irq_notify` dropped `[1, 0, 0, 0, 0]` into a parked receiver's mailbox, so a
/// receiver-first `RECV_CAP` returned `x1 = 0`, a real-looking slot, where the pending-signal order
/// returned `NO_CAP`. No program chooses that word (the kernel writes it), so this is an
/// order-dependence and a fail-open default rather than an attacker-chosen value; a driver that
/// treated `x1 != NO_CAP` as a delegation to delete would have deleted its own slot 0. 634's
/// `cap_delivered` default now makes the receive side answer `NO_CAP`, and this is the only test
/// that drives an interrupt through that order. Signal-pending is asserted too, as the control that
/// shows the two orders agree.
///
/// Falsification: replayable `system_tests/falsifications/user.recv_cap_attack_tests.an_interrupt_signal_received_by_recv_cap_delivers_no_cap_whichever_side_parks_first.patch`
#[test_case]
fn an_interrupt_signal_received_by_recv_cap_delivers_no_cap_whichever_side_parks_first() {
    static RECEIVER_X0: AtomicU64 = AtomicU64::new(u64::MAX - 1);
    static RECEIVER_X1: AtomicU64 = AtomicU64::new(u64::MAX - 1);
    static PARKING: AtomicBool = AtomicBool::new(false);
    static DONE: AtomicBool = AtomicBool::new(false);

    let region = crate::memory_region::create(4).expect("no region");
    let ep = sched::create_rendezvous_from(region).expect("no rendezvous");
    sched::bind_irq(QUIET_INTID, ep);

    // Receiver-first: the driver parks in RECV_CAP, then the interrupt arrives.
    sched::spawn(move || {
        PARKING.store(true, Ordering::SeqCst);
        let [x0, x1, ..] = sched::ipc_recv_cap(ep);
        RECEIVER_X0.store(x0, Ordering::SeqCst);
        RECEIVER_X1.store(x1, Ordering::SeqCst);
        DONE.store(true, Ordering::SeqCst);
    })
    .expect("no driver thread");
    assert!(
        wait_for(|| PARKING.load(Ordering::SeqCst) && sched::rendezvous_waiting_receivers(ep) == 1),
        "the driver never parked in RECV_CAP, so nothing proved the receiver-first order",
    );
    sched::irq_notify(ep);
    assert!(
        wait_for(|| DONE.load(Ordering::SeqCst)),
        "the interrupt never reached the parked RECV_CAP receiver",
    );
    assert_eq!(
        RECEIVER_X0.load(Ordering::SeqCst),
        1,
        "the interrupt's w0 did not arrive"
    );
    assert_eq!(
        RECEIVER_X1.load(Ordering::SeqCst),
        abi::rendezvous::NO_CAP,
        "an interrupt signal handed the receiver x1 = {} on the receiver-first order, a slot number \
         where the message carried no capability",
        RECEIVER_X1.load(Ordering::SeqCst),
    );

    // Signal-first: the interrupt is counted, then this thread receives.
    sched::irq_notify(ep);
    let [x0, x1, ..] = sched::ipc_recv_cap(ep);
    assert_eq!(x0, 1, "the pending interrupt's w0 did not arrive");
    assert_eq!(
        x1,
        abi::rendezvous::NO_CAP,
        "an interrupt signal handed the receiver x1 = {x1} on the signal-first order",
    );

    sched::reclaim_region(region).expect("the endpoint's region did not come back");
}

/// **A death message received by `RECV_CAP` delivers `NO_CAP` in `x1` whichever side arrives
/// first** (milestone 714, fatal risk 7's confinement claim).
///
/// The kernel's five-word death message is `[event, tid, pc, addr, 0]` (`sched::depart`). Before
/// milestone 634 a supervisor parked in `RECV_CAP` when the child died got that mailbox back
/// unchanged, so `x1` was the dead thread's id, a small integer a supervisor that treats `x1` as a
/// slot would act on; a supervisor that received after the corpse parked got `NO_CAP`. The thread id
/// is the kernel's, not a sender's, so as with the interrupt this is an order-dependence rather
/// than an attacker-chosen word. The test also asserts `x2` carries the tid on both orders (after the `x1` check, so a revert is red at `x1`), so a
/// "fix" that blanked the whole message instead of `x1` would fail it.
///
/// Falsification: replayable `system_tests/falsifications/user.recv_cap_attack_tests.a_death_message_received_by_recv_cap_delivers_no_cap_whichever_side_parks_first.patch`
#[test_case]
fn a_death_message_received_by_recv_cap_delivers_no_cap_whichever_side_parks_first() {
    use super::supervision_tests::{FAULT_STUB, build_child_in};

    static SUPERVISOR_PARKING: AtomicBool = AtomicBool::new(false);
    static SUPERVISOR_DONE: AtomicBool = AtomicBool::new(false);
    static SUPERVISOR_MSG: [AtomicU64; 3] = [const { AtomicU64::new(u64::MAX - 1) }; 3];

    // Receiver-first: the supervisor parks in RECV_CAP, then the child dies.
    let fault_ep = sched::create_rendezvous();
    let region = crate::memory_region::create(16).expect("no region for the child");
    sched::spawn(move || {
        SUPERVISOR_PARKING.store(true, Ordering::SeqCst);
        let [event, x1, x2, ..] = sched::ipc_recv_cap(fault_ep);
        SUPERVISOR_MSG[0].store(event, Ordering::SeqCst);
        SUPERVISOR_MSG[1].store(x1, Ordering::SeqCst);
        SUPERVISOR_MSG[2].store(x2, Ordering::SeqCst);
        SUPERVISOR_DONE.store(true, Ordering::SeqCst);
    })
    .expect("no supervisor thread");
    assert!(
        wait_for(|| SUPERVISOR_PARKING.load(Ordering::SeqCst)
            && sched::rendezvous_waiting_receivers(fault_ep) == 1),
        "the supervisor never parked in RECV_CAP, so nothing proved the receiver-first order",
    );
    let child = build_child_in(region, FAULT_STUB, None, Some(fault_ep));
    assert!(
        wait_for(|| SUPERVISOR_DONE.load(Ordering::SeqCst)),
        "the death message never reached the parked RECV_CAP supervisor",
    );
    assert_eq!(
        SUPERVISOR_MSG[0].load(Ordering::SeqCst),
        abi::fault::EVENT_FAULT,
        "the message that arrived was not the child's fault"
    );
    assert_eq!(
        SUPERVISOR_MSG[1].load(Ordering::SeqCst),
        abi::rendezvous::NO_CAP,
        "a death message handed the supervisor x1 = {} on the receiver-first order, the dead \
         thread's id where the message carried no capability",
        SUPERVISOR_MSG[1].load(Ordering::SeqCst),
    );
    assert_eq!(
        SUPERVISOR_MSG[2].load(Ordering::SeqCst),
        child,
        "the death message's thread id (RECV_CAP's x2) did not arrive on the receiver-first order",
    );
    assert!(
        wait_for(|| sched::reclaim_region(region).is_ok()),
        "reaping the first corpse's region failed",
    );

    // Corpse-first: the child dies and parks on the supervision rendezvous, then this thread receives.
    let fault_ep = sched::create_rendezvous();
    let region = crate::memory_region::create(16).expect("no region for the second child");
    let child = build_child_in(region, FAULT_STUB, None, Some(fault_ep));
    assert!(
        wait_for(|| sched::rendezvous_waiting_senders(fault_ep) == 1),
        "the second child never died and parked its message, so nothing proved the corpse-first order",
    );
    let [event, x1, x2, ..] = sched::ipc_recv_cap(fault_ep);
    assert_eq!(event, abi::fault::EVENT_FAULT, "not the child's fault");
    assert_eq!(
        x1,
        abi::rendezvous::NO_CAP,
        "a death message handed the supervisor x1 = {x1} on the corpse-first order",
    );
    assert_eq!(x2, child, "the death message's thread id did not arrive");
    assert!(
        wait_for(|| sched::reclaim_region(region).is_ok()),
        "reaping the second corpse's region failed",
    );
}
