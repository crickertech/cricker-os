//! **A revocation sweep that lands inside a delegation leaves no copy behind** (the revocation-race
//! lane, 2026-10-04 UTC; fatal risk 7's confinement claim and DECISIONS §13 (capability revocation
//! and untyped reclamation)).
//!
//! **The defect these tests were written to demonstrate was live and is fixed here.** Milestone 761
//! (capability lookup off the global lock) recorded it in its `BUGS`, unverified: a syscall that
//! delegates reads its source capability in one critical section and files the narrowed copy in
//! another, so a sweep can fall between the two. These tests drove a sweep into that gap on every
//! delegating syscall that names a `PageFrame`, and all three filed the copy:
//!
//! - `SEND_CAP` read the source with `sched::current_cap`, then `sched::ipc_send_cap` took
//!   `IPC_TABLES` again to deliver or park the copy.
//! - `ThreadControlBlock::CAP_INSERT` read the same way, then `sched::thread_control_block_insert_cap`
//!   filed the copy in the embryo's table.
//! - `PageFrame::SLICE` read its source at the top of `syscall::invoke` and filed the slice with
//!   `sched::grant`.
//!
//! Not new with milestone 761: before it, `current_cap` took `IPC_TABLES` and released it, and the
//! filing took it again, so the gap was two acquisitions of one lock rather than two locks.
//!
//! **Why it matters more than a stale handle would.** Most objects carry generational names
//! (`crates/slots`, DECISIONS §16 (object revocation)), so a copy minted after its object died names a dead generation and fails on
//! use. A `PageFrame` names physical memory and has no generation: the sweep is the only thing that
//! takes it back. Under `PageFrame::REVOKE` the copy is authority the revoker took back; under
//! `MemoryRegion::DESTROY` it names pages the allocator is about to hand to somebody else, which is
//! §13's use-after-free. `DeviceFrame::REVOKE` and `x86_64`'s `PortRange::REVOKE` sweep the same way
//! and shared the gap through `SEND_CAP` and `CAP_INSERT`; the fix is one path for every object, so
//! the frame is the one these tests drive.
//!
//! **How the interleaving is forced.** Two cores can produce it by timing, a few hundred
//! instructions wide, which no test can rely on. `delegation_pause` holds an armed thread at the
//! start of its delegation with no lock held, the test runs the whole sweep, then lets it go.
//!
//! # BUGS
//!
//! - **The `DeviceFrame` and `PortRange` take-backs are reasoned, not driven.** They go through the
//!   same two sched functions as the frame, so the fix covers them by construction, but no test here
//!   drives either.

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use abi::Error;

use crate::arch::exceptions::TrapFrame;
use crate::cap::{Object, Rights, page_frame_run_cap, page_frame_run_len};
use crate::syscall::invoke;
use crate::{delegation_pause, sched};

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

/// `invoke` through the real dispatcher, the way a program's `svc`/`ecall` reaches it.
fn call(slot: u64, method: u64, a0: u64, a1: u64, a2: u64) -> Result<i64, Error> {
    let mut frame = TrapFrame::for_user_entry(0, 0, [0, 0, 0]);
    invoke(&mut frame, slot, method, a0, a1, a2)
}

/// Does any slot of `tid`'s table name a `PageFrame` overlapping `[base, base + pages)`?
fn holds_frame_in(tid: crate::thread::ThreadId, base: u64, pages: u64) -> bool {
    let end = base + pages * page_frames::FRAME_SIZE;
    sched::capability_table_snapshot(tid).is_some_and(|table| {
        table.iter().flatten().any(|c| match c.object {
            Object::PageFrame(p, n) => p < end && base < p + n.get() * page_frames::FRAME_SIZE,
            _ => false,
        })
    })
}

/// A syscall answer as `x0` carries it: the value, or the error's negative code.
fn encode(answer: Result<i64, Error>) -> u64 {
    answer.unwrap_or_else(|e| e as i64) as u64
}

/// **A `PageFrame::REVOKE` that lands inside a `SEND_CAP` must not let the copy reach anyone.**
///
/// The sender holds a frame with `GRANT` and sends a read-only copy to a rendezvous nobody is
/// receiving on. It is held at the delegation seam; the revoke runs whole-machine; then it goes on.
/// Either the send finds its source gone (`NoSuchSlot`, as if it had started after the revoke) or it
/// parks a copy the receiver then collects. The headline is that the receiver is not handed a
/// capability naming the revoked frame.
///
/// The assertions above it are a vacuity guard (the sender really was held inside the delegation)
/// and a premise check (the revoke really did reach the sender's own table).
///
/// Falsification: replayable `system_tests/falsifications/user.revocation_window_tests.a_revoke_inside_a_send_cap_leaves_the_receiver_no_copy.patch`
#[test_case]
fn a_revoke_inside_a_send_cap_leaves_the_receiver_no_copy() {
    static SENDER: AtomicU64 = AtomicU64::new(0);
    static ANSWER: AtomicU64 = AtomicU64::new(0);
    static RETURNED: AtomicBool = AtomicBool::new(false);

    let region = crate::memory_region::create(4).expect("no region");
    let ep = sched::create_rendezvous_from(region).expect("no rendezvous");
    let phys = crate::memory_region::retype_page(region).expect("no page");

    delegation_pause::disarm();
    sched::spawn(move || {
        let ep_slot =
            sched::grant(crate::cap::rendezvous_cap(ep, Rights::WRITE)).expect("grant ep");
        let frame_slot = sched::grant(page_frame_run_cap(phys, page_frame_run_len(1), Rights::ALL))
            .expect("grant the frame");
        SENDER.store(sched::current(), Ordering::SeqCst);
        delegation_pause::arm(sched::current());
        let answer = call(
            ep_slot,
            abi::rendezvous::SEND_CAP,
            frame_slot,
            u64::from(Rights::READ.bits()),
            7,
        );
        ANSWER.store(encode(answer), Ordering::SeqCst);
        RETURNED.store(true, Ordering::SeqCst);
        let _ = sched::delete_current_cap(frame_slot);
        let _ = sched::delete_current_cap(ep_slot);
    })
    .expect("no sender thread");

    assert!(
        wait_for(delegation_pause::parked),
        "the sender never reached the delegation seam, so no revoke landed inside its SEND_CAP",
    );
    crate::revoke::revoke_page_frame(phys);
    let sender = SENDER.load(Ordering::SeqCst);
    let premise = holds_frame_in(sender, phys, 1);
    delegation_pause::release();

    // Post-fix the send fails and returns; pre-fix it parks the copy. Either way it settles.
    assert!(
        wait_for(|| RETURNED.load(Ordering::SeqCst) || sched::rendezvous_waiting_senders(ep) == 1),
        "the sender neither returned nor parked after the seam released it",
    );
    let delivered = if RETURNED.load(Ordering::SeqCst) {
        None
    } else {
        let [_word, slot, ..] = sched::ipc_receive_cap(ep);
        let got = (slot != abi::rendezvous::NO_CAP)
            .then(|| sched::current_cap(slot).ok())
            .flatten();
        if slot != abi::rendezvous::NO_CAP {
            let _ = sched::delete_current_cap(slot);
        }
        got
    };
    assert!(
        wait_for(|| RETURNED.load(Ordering::SeqCst)),
        "the sender never returned"
    );

    assert!(
        !premise,
        "the revoke did not reach the sender's own table, so this test's premise is false",
    );
    assert!(
        !matches!(delivered, Some(c) if c.object == Object::PageFrame(phys, page_frame_run_len(1))),
        "a PageFrame revoked while its SEND_CAP was in progress reached the receiver: the send read \
         its source before the sweep and filed the copy after it (answer {:#x})",
        ANSWER.load(Ordering::SeqCst),
    );

    crate::memory_region::destroy(region);
}

/// **A `PageFrame::REVOKE` that lands inside a `CAP_INSERT` must not leave the embryo a copy.**
///
/// The loader-side twin of the test above: the spawner endows an embryo with a narrowed copy of a
/// frame it holds, and the revoke lands at the seam. The embryo's table is read directly afterwards.
///
/// Falsification: replayable `system_tests/falsifications/user.revocation_window_tests.a_revoke_inside_a_cap_insert_leaves_the_embryo_no_copy.patch`
#[test_case]
fn a_revoke_inside_a_cap_insert_leaves_the_embryo_no_copy() {
    static SPAWNER: AtomicU64 = AtomicU64::new(0);
    static ANSWER: AtomicU64 = AtomicU64::new(0);
    static RETURNED: AtomicBool = AtomicBool::new(false);

    let frames = crate::memory_region::create(2).expect("no frame region");
    let phys = crate::memory_region::retype_page(frames).expect("no page");
    let embryo_region = crate::memory_region::create(4).expect("no embryo region");
    let embryo = sched::create_thread_control_block(embryo_region).expect("no tcb");

    delegation_pause::disarm();
    sched::spawn(move || {
        let tcb_slot = sched::grant(crate::cap::thread_control_block_cap(embryo, Rights::ALL))
            .expect("grant the tcb");
        let frame_slot = sched::grant(page_frame_run_cap(phys, page_frame_run_len(1), Rights::ALL))
            .expect("grant the frame");
        SPAWNER.store(sched::current(), Ordering::SeqCst);
        delegation_pause::arm(sched::current());
        let answer = call(
            tcb_slot,
            abi::thread_control_block::CAP_INSERT,
            frame_slot,
            u64::from(Rights::READ.bits()),
            0,
        );
        ANSWER.store(encode(answer), Ordering::SeqCst);
        RETURNED.store(true, Ordering::SeqCst);
        let _ = sched::delete_current_cap(frame_slot);
        let _ = sched::delete_current_cap(tcb_slot);
    })
    .expect("no spawner thread");

    assert!(
        wait_for(delegation_pause::parked),
        "the spawner never reached the delegation seam, so no revoke landed inside its CAP_INSERT",
    );
    crate::revoke::revoke_page_frame(phys);
    let premise = holds_frame_in(SPAWNER.load(Ordering::SeqCst), phys, 1);
    delegation_pause::release();
    assert!(
        wait_for(|| RETURNED.load(Ordering::SeqCst)),
        "the spawner never returned"
    );

    assert!(
        !premise,
        "the revoke did not reach the spawner's own table, so this test's premise is false",
    );
    assert!(
        !holds_frame_in(embryo, phys, 1),
        "a PageFrame revoked while its CAP_INSERT was in progress was filed in the embryo's table: \
         the insert read its source before the sweep and filed the copy after it (answer {:#x})",
        ANSWER.load(Ordering::SeqCst),
    );

    let deadline = crate::arch::timer::now() + crate::arch::timer::frequency();
    while sched::reclaim_region(embryo_region).is_err() && crate::arch::timer::now() < deadline {
        sched::yield_now();
    }
    crate::memory_region::destroy(frames);
}

/// **A reclamation sweep that lands inside a `SLICE` must not leave the slicer a slice.**
///
/// `MemoryRegion::DESTROY`'s capability pass deletes every `PageFrame` overlapping the region,
/// because the allocator is about to reuse it. A slice filed after that pass names pages the
/// allocator owns again. This drives the pass itself (`revoke::revoke_region`) rather than a whole
/// destroy, so the pages stay put and the assertion reads a table rather than a reused page.
///
/// Falsification: replayable `system_tests/falsifications/user.revocation_window_tests.a_reclamation_inside_a_slice_leaves_the_slicer_no_slice.patch`
#[test_case]
fn a_reclamation_inside_a_slice_leaves_the_slicer_no_slice() {
    static SLICER: AtomicU64 = AtomicU64::new(0);
    static ANSWER: AtomicU64 = AtomicU64::new(0);
    static RETURNED: AtomicBool = AtomicBool::new(false);

    let region = crate::memory_region::create(4).expect("no region");
    let (base, count) = crate::memory_region::retype_run(region, 2).expect("no run");
    assert_eq!(count, 2, "a two-page run came back {count} pages long");

    delegation_pause::disarm();
    sched::spawn(move || {
        let run_slot = sched::grant(page_frame_run_cap(base, page_frame_run_len(2), Rights::ALL))
            .expect("grant the run");
        SLICER.store(sched::current(), Ordering::SeqCst);
        delegation_pause::arm(sched::current());
        let answer = call(run_slot, abi::page_frame::SLICE, 1, 1, 0);
        ANSWER.store(encode(answer), Ordering::SeqCst);
        RETURNED.store(true, Ordering::SeqCst);
        if let Ok(slot) = answer {
            let _ = sched::delete_current_cap(slot as u64);
        }
        let _ = sched::delete_current_cap(run_slot);
    })
    .expect("no slicer thread");

    assert!(
        wait_for(delegation_pause::parked),
        "the slicer never reached the delegation seam, so no sweep landed inside its SLICE",
    );
    crate::revoke::revoke_region(base, 2 * page_frames::FRAME_SIZE);
    let premise = holds_frame_in(SLICER.load(Ordering::SeqCst), base, 2);
    delegation_pause::release();
    assert!(
        wait_for(|| RETURNED.load(Ordering::SeqCst)),
        "the slicer never returned"
    );
    let answer = ANSWER.load(Ordering::SeqCst) as i64;

    assert!(
        !premise,
        "the reclamation sweep did not reach the slicer's own table, so this test's premise is false",
    );
    // A successful SLICE filed a slice: that is the copy, and it is what the slicer's own cleanup
    // deleted afterwards. So the answer is the evidence, not a later read of the table.
    assert!(
        answer < 0,
        "a SLICE whose run was reclaimed while it was in progress filed a slice in slot {answer}: it \
         read its source before the sweep and filed a capability naming reclaimed pages after it",
    );

    crate::memory_region::destroy(region);
}
