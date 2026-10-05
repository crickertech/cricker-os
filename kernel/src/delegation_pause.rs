//! **The test kernel holds one thread at the start of a delegation** (the revocation-race lane,
//! 2026-10-04 UTC). Names here are provisional: the module, [`disarm`], [`arm`], [`parked`],
//! [`release`] and [`here`]. calef names things.
//!
//! A delegation is a syscall that files a copy of a capability its caller holds: `SEND_CAP`,
//! `ThreadControlBlock::CAP_INSERT` and `PageFrame::SLICE`. Each one decodes its arguments, finds the
//! source capability, and files a narrowed copy somewhere. A revocation sweep running on another core
//! must either see the copy (and delete it) or find the source gone before the copy is made. The
//! question this seam lets a test ask is what happens when the sweep lands *after* the syscall has
//! started and *before* the copy is filed.
//!
//! [`here`] sits at that point in each of the three syscalls, and since the map-revocation-window
//! lane (2026-10-04 UTC) at the same point in `PageFrame::MAP`, `AddressSpace::MAP_INTO` and
//! `MemoryRegion::MAP`: a mapping is a use rather than a delegation, but the question is the same,
//! and one seam is one thing to keep in step. Its name says delegation for that history. A test arms one thread, that thread
//! makes the syscall, and at [`here`] it reports itself [`parked`] and yields until the test calls
//! [`release`]. Nothing is held at the seam: no lock, no guard, no interrupt mask, so the test can
//! run a whole revocation sweep while the delegation waits. That is the interleaving two cores can
//! produce by timing, made certain.
//!
//! **Where the seam sits matters to what a green result means.** Before the fix it sat between the
//! syscall's read of the source (`sched::current_cap`) and the call that filed the copy, which is the
//! window milestone 761 (capability lookup off the global lock)'s `BUGS` described. After it, the read and the filing are one critical
//! section serialized against every sweep, so the seam sits just before that section: a sweep that
//! lands anywhere before the delegation's own lock is held is the case it forces.
//!
//! # Why a shipping kernel cannot contain this
//!
//! `retype_fault`'s reason, in its words: the module is declared under `cfg(feature =
//! "system_tests")`, and so is every call into it, so a hook written without that `cfg` fails to
//! compile rather than shipping. A kernel binary built with the feature fails to link (`lib.rs`).
//!
//! # BUGS
//!
//! - **One thread, one pause.** Arming again replaces the target, and the pause disarms itself when
//!   it fires, so a second delegation by the same thread runs straight through. Every test that uses
//!   it delegates once.
//! - **The protocol is the caller's to follow**: [`disarm`] before starting the thread, [`arm`] from
//!   inside it. Nothing enforces the order.

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::thread::ThreadId;

const UNARMED: ThreadId = crate::cpu::NO_TID;

static TARGET: AtomicU64 = AtomicU64::new(UNARMED);
static PARKED: AtomicBool = AtomicBool::new(false);
static RELEASED: AtomicBool = AtomicBool::new(false);

/// **Forget any earlier pause**: no target, nothing parked, nothing released. A test calls this
/// before it starts the thread that will [`arm`], so that a [`parked`] left `true` by the test
/// before it cannot be read as this one's thread arriving. That stale read happened: the second
/// test in a run revoked before its own thread had armed, released nothing, and its thread then
/// parked for good.
pub fn disarm() {
    TARGET.store(UNARMED, Ordering::SeqCst);
    PARKED.store(false, Ordering::SeqCst);
    RELEASED.store(false, Ordering::SeqCst);
}

/// **Hold `target` at its next delegation's seam** until [`release`]. Called by the thread itself,
/// after the test has called [`disarm`].
pub fn arm(target: ThreadId) {
    assert_ne!(target, UNARMED, "delegation_pause::arm needs a real thread");
    TARGET.store(target, Ordering::SeqCst);
}

/// Whether the armed thread has reached its seam and is waiting there.
pub fn parked() -> bool {
    PARKED.load(Ordering::SeqCst)
}

/// Let the held delegation continue.
pub fn release() {
    RELEASED.store(true, Ordering::SeqCst);
}

/// **The seam.** A relaxed load and a compare on every delegation of the test kernel; nothing in any
/// other build. Must be called with no lock held, which each call site states.
#[inline]
pub(crate) fn here() {
    let target = TARGET.load(Ordering::Acquire);
    if target == UNARMED || crate::sched::current() != target {
        return;
    }
    TARGET.store(UNARMED, Ordering::SeqCst);
    PARKED.store(true, Ordering::SeqCst);
    while !RELEASED.load(Ordering::SeqCst) {
        crate::sched::yield_now();
    }
}
