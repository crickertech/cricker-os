//! **The test kernel fails one thread's Nth retype** (milestone 757 (a test kernel fails a process
//! on its Nth retype), provisional). Names here are provisional: the module, [`arm`], [`disarm`],
//! [`Tally`] and [`fails_now`]. calef names things.
//!
//! A system test names a thread and a number N. From then on, every time that thread makes the
//! kernel spend a page of one of its regions, the count goes up by one, and the Nth time the kernel
//! answers exactly as it would for a region with nothing left: the spending function returns `None`
//! with the region untouched, and the syscall above it returns `OutOfMemory`. Sweeping N from 1
//! until a run never reaches N fails a program at every allocation it makes, in order, which is the
//! only way to run the cleanup paths `notes/untested-error-paths.md` counted and no test reached.
//!
//! **What counts as one retype** is one call into the three functions of `memory_region` that take
//! pages out of a region: `split`, `retype_run` (which `retype_page` is) and `retype_object_page`.
//! So `MemoryRegion::SPLIT`, `RETYPE` and `RETYPE_OBJ` each count once, and a `MAP`,
//! `PageFrame::MAP` or `AddressSpace::MAP_INTO` counts once per page table it has to build. The
//! last kind is the point of hooking here rather than at the syscall: a mapping that fails after it
//! has built one table runs the kernel's own unwind, not only the program's.
//!
//! **The target is a thread, read from the core's current-thread word**, because that is what the
//! kernel knows at a spending function without taking a lock. A kernel-built service (the test
//! harness spawning it, as `login_service::start` does) spends from the harness's thread, so its
//! construction is never counted; only what the service asks for itself is.
//!
//! # Why a shipping kernel cannot contain this
//!
//! The module is declared under `cfg(feature = "system_tests")`, and so is every call into it
//! (`memory_region.rs`). In any other build the module does not exist, so a hook written without
//! that `cfg` fails to compile rather than shipping, and a hook written with it compiles to nothing.
//! The feature itself cannot reach a bootable kernel: a kernel *binary* built with `system_tests`
//! fails to link, on purpose (`lib.rs`, `system_tests_main`). That is the first rung of
//! `CLAUDE.md`'s ladder (the shipping binary has no code that could fire), held up by the second (a
//! loud link failure if someone turns the feature on where it does not belong). There is no
//! separate cargo feature for this knob: a second feature would be a second thing to keep out of
//! shipping builds, and it would buy nothing, because an unarmed hook costs one relaxed load on a
//! path that is never the IPC round trip.
//!
//! # BUGS
//!
//! - **One thread, not a process.** A process with two threads that both spend is counted on the
//!   named one only. Every program this has been pointed at is single-threaded; a multi-threaded
//!   target wants a process identity at the spending function, which the kernel does not keep
//!   there today.
//! - **Only the Nth fails, not the Nth and every one after.** A real exhaustion keeps failing; this
//!   fails once and then lets the program carry on, which is what makes a sweep cheap (each N is
//!   one run) and what lets the test check the program is still healthy afterwards. A program whose
//!   cleanup itself allocates is therefore tested with that allocation succeeding.
//! - **One target at a time.** Arming again replaces the earlier target. Nothing in the suite runs
//!   two sweeps at once.

use core::sync::atomic::{AtomicU64, Ordering};

use crate::thread::ThreadId;

/// No thread is armed. `cpu::NO_TID`, which no live thread ever carries.
const UNARMED: ThreadId = crate::cpu::NO_TID;

static TARGET: AtomicU64 = AtomicU64::new(UNARMED);
static FAIL_AT: AtomicU64 = AtomicU64::new(0);
static SEEN: AtomicU64 = AtomicU64::new(0);
static REFUSED: AtomicU64 = AtomicU64::new(0);

/// What one armed run saw, from [`disarm`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tally {
    /// How many times the target spent, or tried to spend, a page of one of its regions.
    pub seen: u64,
    /// Whether the Nth arrived and was refused. `false` means the run made fewer than N retypes,
    /// which is how a sweep knows it has passed the last one.
    pub refused: bool,
}

/// **Fail `target`'s `n`th retype from now on** (`n` counts from 1). `u64::MAX` counts without ever
/// failing, which is how a test learns how many retypes an exchange makes.
pub fn arm(target: ThreadId, n: u64) {
    assert!(n >= 1, "retype_fault::arm counts from 1");
    assert_ne!(target, UNARMED, "retype_fault::arm needs a real thread");
    // Disarm first, so a spender racing this on another core sees either the old target with its
    // old count or nothing, never the new target with a stale count.
    TARGET.store(UNARMED, Ordering::SeqCst);
    SEEN.store(0, Ordering::SeqCst);
    REFUSED.store(0, Ordering::SeqCst);
    FAIL_AT.store(n, Ordering::SeqCst);
    TARGET.store(target, Ordering::SeqCst);
}

/// Stop counting, and say what the run saw.
pub fn disarm() -> Tally {
    TARGET.store(UNARMED, Ordering::SeqCst);
    Tally {
        seen: SEEN.load(Ordering::SeqCst),
        refused: REFUSED.load(Ordering::SeqCst) != 0,
    }
}

/// **Whether this retype is the one to fail.** Called at the top of each spending function in
/// `memory_region`, before the region lock is taken, so a refusal leaves the region exactly as an
/// exhausted one would be left: untouched.
#[inline]
pub(crate) fn fails_now() -> bool {
    let target = TARGET.load(Ordering::Acquire);
    if target == UNARMED || crate::sched::current() != target {
        return false;
    }
    let n = SEEN.fetch_add(1, Ordering::AcqRel) + 1;
    if n == FAIL_AT.load(Ordering::Acquire) {
        REFUSED.store(1, Ordering::Release);
        return true;
    }
    false
}
