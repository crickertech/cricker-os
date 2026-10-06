//! **A client holding one right on a rendezvous cannot reach the operations another right gates**
//! (milestone 633 (an outside agent attacks the confinement claim), fatal risk 7's confinement
//! claim, third outsider pass).
//!
//! The two properties here are the kernel half of DECISIONS §41 (the endpoint is the broker)'s "a client of a rendezvous cannot
//! become its server" (confinement claim 26) and §14 (the project's direction)'s "userspace cannot forge a right out of a
//! syscall register" (claim 2). Both are driven through the real dispatcher (`syscall::invoke`), the
//! way a program's `svc`/`ecall` reaches it, so what is under test is the rights gate in
//! `invoke`'s `Object::Rendezvous` arm and nothing re-implemented beside it. `irq_send_refusal_tests`
//! is the pattern: grant a capability into this thread's own table and invoke a method it must be
//! refused.
//!
//! **Why this exists next to `live_swap_tests`.** That test (claim 26's cited one) drives a single
//! READ-gated method, `RECEIVE_CAP`, from one fixture, and only on the two architectures with a
//! console device page; a break of the gate surfaces there as a watchdog hang rather than a failed
//! assertion, because a `RECEIVE_CAP` let through simply blocks with nothing to receive. These tests
//! cover the other READ-gated methods a would-be server would reach for (`RECEIVE`, `REAP`) and the
//! `ENUMERATE`-gated `SURVEY`, on every architecture, and each is shaped so that a gate let open
//! returns promptly (a sender is already queued, or the method does not block) rather than hanging:
//! a regression is red at an assertion, not at the watchdog.
//!
//! Cross-ISA: `invoke`, the rights check and the rendezvous are portable kernel code, so the parity
//! gate (DECISIONS §19 (architectural parity is a tenet)) is met by the same test on each
//! architecture.

use abi::Error;

use crate::arch::exceptions::TrapFrame;
use crate::cap::{Rights, rendezvous_cap};
use crate::sched;
use crate::syscall::invoke;

/// `invoke` through the real dispatcher, the way a program's `svc`/`ecall` reaches it. Returns the
/// dispatcher's `Result` and the frame, so a caller that cares about the delivered message words
/// (none here do) can read them. Copied in shape from `irq_send_refusal_tests::call`.
fn call(slot: u64, method: u64, a0: u64, a1: u64, a2: u64) -> Result<i64, Error> {
    let mut frame = TrapFrame::for_user_entry(0, 0, [0, 0, 0]);
    invoke(&mut frame, slot, method, a0, a1, a2)
}

/// **A holder of `WRITE` alone on a rendezvous is refused every operation the *other* rights gate.**
///
/// `WRITE` lets a holder `SEND` and `CALL`; it is the right a client of a server holds. Becoming the
/// server means `RECEIVE`, `RECEIVE_CAP` or `REAP` (all `READ`), or `SURVEY` (`ENUMERATE`). Each must
/// answer `NotPermitted`, because the right lives on the capability and no method or argument
/// register can add one.
///
/// The two blocking methods (`RECEIVE`, `RECEIVE_CAP`) are tried with a sender already parked on the
/// endpoint, so a gate let open would *return that sender's message* and drop the waiting-sender
/// count rather than block: the break is observable here, not a sixty-second hang. The assertion on
/// the sender count after each is the other half of the claim: a refused receive consumes nothing.
/// `REAP` and `SURVEY` do not block, so they need no queued peer.
///
/// The positive control is the last step: the same endpoint, received on with a `READ` capability
/// through the same dispatcher, does deliver the queued message. Without it a `RECEIVE` that was
/// refused for some unrelated reason would read as the property holding.
#[test_case]
fn a_write_only_rendezvous_holder_cannot_receive_reap_or_survey() {
    let region = crate::memory_region::create(4).expect("no region for the endpoint");
    let ep = sched::create_rendezvous_from(region).expect("no rendezvous");

    // A sender parked on the endpoint. A WRITE-only RECEIVE/RECEIVE_CAP let through would collect
    // this and the waiting-sender count would fall to zero; refused, it stays at one.
    const SENT_WORD: u64 = 0x633;
    let sender = sched::spawn(move || {
        sched::ipc_send(ep, [SENT_WORD, 0, 0]);
    })
    .expect("spawn the sender");
    assert!(
        super::wait_for(|| sched::rendezvous_waiting_senders(ep) == 1),
        "the sender never parked, so a let-open RECEIVE would hang rather than be observed",
    );

    // The would-be server's capability: WRITE only, the right a client holds. GRANT is absent too,
    // so this is strictly a client's authority.
    let client = sched::grant(rendezvous_cap(ep, Rights::WRITE)).expect("grant the WRITE cap");

    assert_eq!(
        call(client, abi::rendezvous::RECEIVE, 0, 0, 0),
        Err(Error::NotPermitted),
        "a WRITE-only holder was let RECEIVE: a client became its server",
    );
    assert_eq!(
        sched::rendezvous_waiting_senders(ep),
        1,
        "a refused RECEIVE still consumed the queued sender",
    );
    assert_eq!(
        call(client, abi::rendezvous::RECEIVE_CAP, 0, 0, 0),
        Err(Error::NotPermitted),
        "a WRITE-only holder RECEIVE_CAPd: a client became its server",
    );
    assert_eq!(
        sched::rendezvous_waiting_senders(ep),
        1,
        "a refused RECEIVE_CAP still consumed the queued sender",
    );
    // REAP (READ) and SURVEY (ENUMERATE) do not block, so a let-open gate returns some other answer
    // (a reap decision, a survey cursor) rather than NotPermitted, which is still observable.
    assert_eq!(
        call(client, abi::rendezvous::REAP, 0, 0, 0),
        Err(Error::NotPermitted),
        "a WRITE-only holder REAPed: it could collect a corpse it does not supervise",
    );
    assert_eq!(
        call(client, abi::rendezvous::SURVEY, 0, abi::survey::record::STATE, 0),
        Err(Error::NotPermitted),
        "a WRITE-only holder SURVEYed: it could enumerate a domain it does not supervise",
    );

    // Positive control: a READ capability on the same endpoint receives the still-queued message
    // through the same dispatcher. This both proves the sender really was parked (so the refusals
    // above meant something) and drains it so the sender thread exits.
    let server = sched::grant(rendezvous_cap(ep, Rights::READ)).expect("grant the READ cap");
    let got = call(server, abi::rendezvous::RECEIVE, 0, 0, 0);
    assert_eq!(
        got,
        Ok(SENT_WORD as i64),
        "a READ holder could not receive the queued message: the endpoint was not really live",
    );
    assert_eq!(
        sched::rendezvous_waiting_senders(ep),
        0,
        "the queued sender was not released by the legitimate receive",
    );

    let _ = sched::delete_current_cap(client);
    let _ = sched::delete_current_cap(server);
    assert!(
        super::wait_for(|| !sched::is_thread_present(sender)),
        "the sender thread never finished after its message was collected",
    );
    sched::reclaim_region(region).expect("the endpoint's region did not come back");
}

/// **A right cannot be forged out of a syscall register, and an ungranted slot names nothing**
/// (claim 2, §14).
///
/// A holder of `READ` alone invokes `SEND`, which takes `WRITE`: the right is not in the method
/// number or the argument words, it is on the capability, so the answer is `NotPermitted`. A receiver
/// is parked first, so a gate let open would *deliver* to it and drop the waiting-receiver count
/// rather than block.
///
/// Then a slot this thread was never granted is invoked, and the answer is `NoSuchSlot`: not
/// "permission denied" but "there is nothing there", which is what no-ambient-authority means from
/// the inside. A program cannot reach an object by naming a slot number it was not handed.
#[test_case]
fn a_read_only_holder_cannot_send_and_an_ungranted_slot_names_nothing() {
    let region = crate::memory_region::create(4).expect("no region for the endpoint");
    let ep = sched::create_rendezvous_from(region).expect("no rendezvous");

    // A receiver parked on the endpoint, so a WRITE forged from the SEND path would be observed as a
    // delivery rather than a hang.
    let receiver = sched::spawn(move || {
        let _ = sched::ipc_receive(ep);
    })
    .expect("spawn the receiver");
    assert!(
        super::wait_for(|| sched::rendezvous_waiting_receivers(ep) == 1),
        "the receiver never parked, so a let-open SEND would hang rather than be observed",
    );

    let read_only = sched::grant(rendezvous_cap(ep, Rights::READ)).expect("grant the READ cap");
    assert_eq!(
        call(read_only, abi::rendezvous::SEND, 1, 0, 0),
        Err(Error::NotPermitted),
        "a READ-only holder SENT: WRITE was forged from the method or an argument register",
    );
    assert_eq!(
        sched::rendezvous_waiting_receivers(ep),
        1,
        "a refused SEND still reached the parked receiver",
    );

    // A slot never granted in this table. The highest table index exists, so this is "empty", not
    // "out of range", and both answer NoSuchSlot.
    let ungranted = (crate::cap::CAPABILITY_TABLE_SLOTS as u64) - 2;
    assert!(
        sched::current_cap(ungranted).is_err(),
        "the slot chosen as ungranted was occupied; pick another",
    );
    assert_eq!(
        call(ungranted, abi::rendezvous::SEND, 1, 0, 0),
        Err(Error::NoSuchSlot),
        "invoking an ungranted slot named an object: ambient authority",
    );

    // Positive control and cleanup: a WRITE capability delivers to the parked receiver, releasing it.
    let writer = sched::grant(rendezvous_cap(ep, Rights::WRITE)).expect("grant the WRITE cap");
    assert_eq!(
        call(writer, abi::rendezvous::SEND, 1, 0, 0),
        Ok(0),
        "a WRITE holder could not send: the endpoint was not really live",
    );
    let _ = sched::delete_current_cap(read_only);
    let _ = sched::delete_current_cap(writer);
    assert!(
        super::wait_for(|| !sched::is_thread_present(receiver)),
        "the receiver thread never finished after a message reached it",
    );
    sched::reclaim_region(region).expect("the endpoint's region did not come back");
}
