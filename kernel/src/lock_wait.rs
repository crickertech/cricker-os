//! **How often the job mix's threads found a kernel lock held, and how long they waited**
//! (2026-10-04, fatal risk 4). Module, feature and the `job-mix-lock:` line are provisional names,
//! a lane's coinage, per the naming tenet.
//!
//! Built only with `--features lock_wait` (which implies `job_mix`), so no other kernel carries a
//! byte of it, the job-mix image that is not asked for it is the kernel that ships, and
//! `script/fastpath-footprint` (which builds neither) is untouched.
//!
//! # The question it answers
//!
//! On radon on 2026-10-04 the job mix's `null_syscall` job cost 108 ticks at one task and 202 at four
//! while `compute` grew 6% (notes/job-mix/radon-2026-10-04.md). That job's syscall takes exactly one
//! lock: `sched::current_cap` took the global `IPC_TABLES` to read the caller's own capability
//! table, and every other core's IPC, `schedule()` and capability operation takes the same lock.
//! (Since the proposal "capability lookup off the global lock", later on 2026-10-04 UTC, that one
//! lock is the running thread's own table, rank `capability_table`.) This module counts, per sweep
//! point:
//!
//! - **Per lock rank**: acquisitions that found the lock held, and the ticks spent spinning for it.
//! - **For the reaper**: how many threads it reaped, the ticks spent freeing their kernel stacks
//!   (outside any lock since 2026-10-04, which is the fix this instrument found), and the ticks
//!   its final `IPC_TABLES` section held the lock to remove them.
//! - **For `current_cap` alone**: every call, and the calls whose acquisition found its lock held,
//!   with their wait. Every `invoke` makes exactly one such call, so `contended / calls` is
//!   the chance the cheapest syscall waits, and `wait_ticks / calls` is what that costs it on
//!   average, which is directly comparable with the `null_syscall` job's per-trap growth.
//!
//! # What it costs, and what it cannot see
//!
//! **An acquisition that finds the lock free costs what it always did**: `IrqSafeMutex::lock` opens
//! with `try_lock`, which is the same compare-and-swap `spin`'s `lock` opens with. `current_cap`
//! pays a flag store, a counter increment and a flag clear, all on this core's own line. The clock
//! is read **only on the contended path**, twice per wait, and that is deliberate: on radon a
//! `rdtime` may be emulated by firmware rather than read from hardware (a recollection about the
//! `SiFive` U74, not checked against radon's OpenSBI; marked as such), and reading it on every call
//! would have added to the cheapest syscall the very cost this module is trying to find.
//!
//! - A wait is quantised to the platform timer (4 MHz on radon, 250 ns a tick), so one wait means
//!   nothing and only sums over many do. The sums are unbiased because a wait's start is not
//!   phase-locked to the timer. If `rdtime` is emulated, each recorded wait also carries part of the
//!   cost of reading the clock.
//! - An acquisition that finds the lock free but has to pull its cache line out of another core is
//!   not counted at all. That cost is real, it rises with busy cores, and this cannot see it; the
//!   residual between the job's growth and the wait this counts is where it would show.
//! - The `current_cap` attribution is a per-core flag set around the acquisition. An interrupt that
//!   arrived between the flag and the lock masking interrupts, and took a contended lock of its own,
//!   would be charged to `current_cap`. On the syscall path interrupts are already masked there.
//!
//! Counted only while [`ARMED`]: the supervisor arms it for each timed window and disarms it for the
//! untimed breakdown exchange, so the numbers are the subruns' and not the bookkeeping's.

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::cpu::MAX_CPUS;

/// Lock ranks are below this (`sync::rank` tops out at 61). A rank at or above it is not counted.
const RANKS: usize = 64;

/// True for the duration of a timed subrun. Read on every counted path, written twice a subrun by
/// the supervisor, so its line is read-mostly and costs a reader nothing after the first read.
static ARMED: AtomicBool = AtomicBool::new(false);

/// One core's counters, on lines of their own so the instrument does not add the cross-core traffic
/// it is trying to measure. Written only by the owning core; read and reset by the supervisor
/// between subruns, when no task is running.
#[repr(align(64))]
struct Core {
    /// Set by `current_cap` around its one lock acquisition.
    in_current_cap: AtomicBool,
    cap_calls: AtomicU64,
    cap_contended: AtomicU64,
    cap_wait_ticks: AtomicU64,
    reaps: AtomicU64,
    stack_free_ticks: AtomicU64,
    remove_ticks: AtomicU64,
    contended: [AtomicU64; RANKS],
    wait_ticks: [AtomicU64; RANKS],
}

impl Core {
    const fn new() -> Self {
        Self {
            in_current_cap: AtomicBool::new(false),
            cap_calls: AtomicU64::new(0),
            cap_contended: AtomicU64::new(0),
            cap_wait_ticks: AtomicU64::new(0),
            reaps: AtomicU64::new(0),
            stack_free_ticks: AtomicU64::new(0),
            remove_ticks: AtomicU64::new(0),
            contended: [const { AtomicU64::new(0) }; RANKS],
            wait_ticks: [const { AtomicU64::new(0) }; RANKS],
        }
    }
}

static CORES: [Core; MAX_CPUS] = [const { Core::new() }; MAX_CPUS];

fn here() -> &'static Core {
    &CORES[crate::cpu::id() & (MAX_CPUS - 1)]
}

fn bump(counter: &AtomicU64, by: u64) {
    counter.fetch_add(by, Ordering::Relaxed);
}

/// Called by `IrqSafeMutex::lock` when its first `try_lock` failed and it then spun `ticks` for the
/// lock. Interrupts are masked, so this is the core that will hold the lock.
pub fn contended(rank: u32, ticks: u64) {
    if !ARMED.load(Ordering::Relaxed) {
        return;
    }
    let core = here();
    if let (Some(c), Some(w)) = (
        core.contended.get(rank as usize),
        core.wait_ticks.get(rank as usize),
    ) {
        bump(c, 1);
        bump(w, ticks);
    }
    if core.in_current_cap.load(Ordering::Relaxed) {
        bump(&core.cap_contended, 1);
        bump(&core.cap_wait_ticks, ticks);
    }
}

/// `sched::current_cap` is about to take its lock: count the call and mark the acquisition. That lock
/// was `IPC_TABLES` until 2026-10-04 UTC and is the running thread's own capability table since, so
/// `site=current_cap` reads before and after the change with one meaning: how often the cheapest
/// syscall waited for anything.
pub fn enter_current_cap() {
    if !ARMED.load(Ordering::Relaxed) {
        return;
    }
    let core = here();
    bump(&core.cap_calls, 1);
    core.in_current_cap.store(true, Ordering::Relaxed);
}

/// `sched::current_cap` holds its lock now: any later contended lock on this core is not its.
pub fn leave_current_cap() {
    here().in_current_cap.store(false, Ordering::Relaxed);
}

/// Called by `sched::reap_switched_out` after freeing a dead thread's kernel stack with no lock
/// held: how long the free took. The reaper is cold (one call per thread exit), so reading the
/// clock here costs the measured paths nothing.
pub fn stack_freed(ticks: u64) {
    if !ARMED.load(Ordering::Relaxed) {
        return;
    }
    let core = here();
    bump(&core.reaps, 1);
    bump(&core.stack_free_ticks, ticks);
}

/// Called by `sched::reap_switched_out` for its second critical section: how long removing the
/// thread held `IPC_TABLES`.
pub fn reaped(ticks: u64) {
    if !ARMED.load(Ordering::Relaxed) {
        return;
    }
    bump(&here().remove_ticks, ticks);
}

/// Start counting (the supervisor, just before it releases a subrun's tasks).
pub fn arm() {
    ARMED.store(true, Ordering::Relaxed);
}

/// Stop counting (the supervisor, once the subrun's last task has reported).
pub fn disarm() {
    ARMED.store(false, Ordering::Relaxed);
}

/// Zero every core's counters. Between sweep points, with counting disarmed.
pub fn reset() {
    for core in &CORES {
        for c in core.contended.iter().chain(core.wait_ticks.iter()) {
            c.store(0, Ordering::Relaxed);
        }
        core.cap_calls.store(0, Ordering::Relaxed);
        core.cap_contended.store(0, Ordering::Relaxed);
        core.cap_wait_ticks.store(0, Ordering::Relaxed);
        core.reaps.store(0, Ordering::Relaxed);
        core.stack_free_ticks.store(0, Ordering::Relaxed);
        core.remove_ticks.store(0, Ordering::Relaxed);
    }
}

/// Summed over every core: `(reaps, stack_free_ticks, remove_ticks)` for the reaper.
pub fn reap_totals() -> (u64, u64, u64) {
    CORES.iter().fold((0, 0, 0), |(n, f, r), core| {
        (
            n + core.reaps.load(Ordering::Relaxed),
            f + core.stack_free_ticks.load(Ordering::Relaxed),
            r + core.remove_ticks.load(Ordering::Relaxed),
        )
    })
}

/// Summed over every core: `(contended, wait_ticks)` for one rank.
pub fn rank_totals(rank: usize) -> (u64, u64) {
    CORES.iter().fold((0, 0), |(c, w), core| {
        (
            c + core
                .contended
                .get(rank)
                .map_or(0, |a| a.load(Ordering::Relaxed)),
            w + core
                .wait_ticks
                .get(rank)
                .map_or(0, |a| a.load(Ordering::Relaxed)),
        )
    })
}

/// Summed over every core: `(calls, contended, wait_ticks)` for `current_cap`.
pub fn current_cap_totals() -> (u64, u64, u64) {
    CORES.iter().fold((0, 0, 0), |(n, c, w), core| {
        (
            n + core.cap_calls.load(Ordering::Relaxed),
            c + core.cap_contended.load(Ordering::Relaxed),
            w + core.cap_wait_ticks.load(Ordering::Relaxed),
        )
    })
}

/// The word a rank is printed as. A rank `sync::rank` gives to more than one lock (59 is the
/// inbox, the kernel heap and the mappings table) is printed by number only, because a name would
/// claim an attribution the counter cannot make.
pub fn rank_name(rank: usize) -> Option<&'static str> {
    use crate::sync::rank as r;
    match u32::try_from(rank).ok()? {
        r::IPC_TABLES => Some("ipc_tables"),
        r::CAPABILITY_TABLE => Some("capability_table"),
        r::MEMORY_REGION => Some("memory_region"),
        r::ADDRESS_SPACES => Some("address_spaces"),
        _ => None,
    }
}

/// How many ranks there are to walk when printing.
pub const fn ranks() -> usize {
    RANKS
}
