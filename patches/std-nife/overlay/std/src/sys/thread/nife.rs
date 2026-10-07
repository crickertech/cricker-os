//! Threads on nife, phase one: `spawn` is honestly `Unsupported`, `sleep` is real.
//!
//! The kernel has everything a `thread::spawn` needs (retype a TCB from the process's own
//! untyped, CONFIGURE it into this address space, START it); what does not exist yet is the
//! std-side plumbing that makes the result safe (a TLS story, park/unpark on a kernel
//! primitive, join). Milestone 27 phase one ships without it rather than shipping it wrong,
//! so `spawn` returns `Unsupported` (the roadmap's named fallback) and the sync primitives are
//! std's single-threaded `no_threads` implementations.
//!
//! `sleep` blocks (milestone 106 (a wait that ends on either the interrupt or the deadline), DECISIONS
//! §147 (a timer a userspace service cannot hold)): the first sleep retypes a notification and a
//! timer out of the process's own untyped (slot 0), and every sleep arms the timer and waits on the
//! notification, so the hart is idle for the whole duration. Until 2026-09-26 it was a `SYS_YIELD`
//! loop, which cost a hart and `10^5` to `10^6` syscalls per second slept (`notes/timed-wait.md`).
//!
//! **The loop is still here as the fallback**, for a process whose slot 0 cannot pay for the two
//! pages (no untyped, or an exhausted one). A sleep that spins is worse than one that blocks and
//! better than one that panics, and which one a program got is visible to nobody: that is this
//! file's BUGS entry, in `design/roadmap/0106-deadline-wait.md`.

use crate::ffi::CStr;
use crate::io;
use crate::num::NonZero;
use crate::thread::ThreadInit;
use crate::time::Duration;

// Silence dead code warnings for the otherwise unused ThreadInit::init() call.
#[expect(dead_code)]
fn dummy_init_call(init: Box<ThreadInit>) {
    drop(init.init());
}

pub struct Thread(!);

pub const DEFAULT_MIN_STACK_SIZE: usize = 64 * 1024;

impl Thread {
    // unsafe: see thread::Builder::spawn_unchecked for safety requirements
    pub unsafe fn new(_stack: usize, _init: Box<ThreadInit>) -> io::Result<Thread> {
        Err(io::Error::UNSUPPORTED_PLATFORM)
    }

    pub fn join(self) {
        self.0
    }
}

pub fn available_parallelism() -> io::Result<NonZero<usize>> {
    // The process model is one thread today, and that is an answer, not an error.
    Ok(NonZero::new(1).unwrap())
}

pub fn current_os_id() -> Option<u64> {
    None
}

pub fn yield_now() {
    crate::sys::pal::nife::rt::yield_now();
}

pub fn set_name(_name: &CStr) {
    // No kernel-side thread names yet.
}

/// The notification and timer slots a blocking sleep uses, made on first use. `0` means "not
/// made yet" (slot 0 is the heap's untyped, never one of these); `u64::MAX` means "tried and the
/// untyped refused", so a process that cannot pay does not retry on every sleep.
static SLEEP_NOTIFICATION: crate::sync::atomic::AtomicU64 = crate::sync::atomic::AtomicU64::new(0);
static SLEEP_TIMER: crate::sync::atomic::AtomicU64 = crate::sync::atomic::AtomicU64::new(0);

/// The (timer, notification) pair, made from slot 0 on the first sleep, or `None` if it cannot be.
/// Single-threaded (`spawn` is unsupported), so there is no race to make two.
fn sleep_objects() -> Option<(u64, u64)> {
    use crate::sync::atomic::Ordering::Relaxed;
    use crate::sys::pal::nife::{abi, rt};
    match (SLEEP_TIMER.load(Relaxed), SLEEP_NOTIFICATION.load(Relaxed)) {
        (u64::MAX, _) => return None,
        (0, _) => {}
        (t, n) => return Some((t, n)),
    }
    // A new capability lands in the first free slot, and an empty contract slot is how the PAL
    // knows a service was not granted (no network, no directory). A sleeper landing in one would
    // read as that service, so a process with any contract slot empty keeps the yield loop. Probed
    // with a method no object has: `NoSuchSlot` is empty, anything else is held.
    for slot in 0..crate::sys::pal::nife::runtimeproto::SLOTS {
        // SAFETY: a syscall that does nothing to a held capability.
        if unsafe { rt::invoke(slot, u64::MAX, 0, 0, 0) } == abi::Error::NoSuchSlot as i64 {
            SLEEP_TIMER.store(u64::MAX, Relaxed);
            return None;
        }
    }
    // SAFETY: `invoke` is a syscall; the kernel validates the capability in slot 0.
    let retype = |objtype| unsafe {
        rt::invoke(rt::MEMORY_REGION_SLOT, abi::memory_region::RETYPE_OBJ, objtype, 0, 0)
    };
    let n = retype(abi::objtype::NOTIFICATION);
    let t = if n >= 0 { retype(abi::objtype::TIMER) } else { n };
    if n < 0 || t < 0 {
        SLEEP_TIMER.store(u64::MAX, Relaxed);
        return None;
    }
    SLEEP_NOTIFICATION.store(n as u64, Relaxed);
    SLEEP_TIMER.store(t as u64, Relaxed);
    Some((t as u64, n as u64))
}

pub fn sleep(dur: Duration) {
    use crate::sys::pal::nife::{abi, rt};
    // Rounded up, so the sleep is never short: `std` promises at least `dur`.
    let ticks = abi::timer::counter_ticks_for(dur.as_secs(), dur.subsec_nanos(), rt::cntfrq());
    let deadline = rt::now().saturating_add(ticks);
    if let Some((timer, notification)) = sleep_objects() {
        // One word only this function signals, so WAIT returns when the deadline passes. A
        // refusal (never expected: both objects are ours) falls through to the loop below, which
        // finds the deadline passed or spins out the rest.
        // SAFETY: syscalls on this process's own capabilities, validated by the kernel.
        let armed = unsafe { rt::invoke(timer, abi::timer::ARM, deadline, notification, 1) };
        if armed == 0 {
            unsafe { rt::invoke(notification, abi::notification::WAIT, 0, 0, 0) };
        }
    }
    while rt::now() < deadline {
        rt::yield_now();
    }
}
