//! **The machine statistics page, the kernel's half** (milestone 126 (the `procps` package), DECISIONS §225 (`free` sees the machine and your share) part 2).
//!
//! One frame holding the machine-wide counters `free`, `vmstat` and `top` read: the free frame
//! count, and per core the busy and idle ticks, the context switches, the interrupts and the run
//! queue. The layout is `crates/machine_statistics_protocol`'s; this module is the writes.
//!
//! **The counters live in the page itself**, so there is no copy and no refresh to schedule. Each
//! per-core word is written only by its own core (the context switch, the tick and the
//! cross-core interrupt all run on the core they count), and each core's words are one cache line
//! of their own, so nothing here contends. The frame count is written under the allocator's own
//! lock, by whoever changed it.
//!
//! **Before [`publish`] the writes land in a static sink** rather than behind a branch: the page
//! pointer starts at the sink, so a context switch pays one load and one add whether or not the
//! page exists yet, and never a test. Nothing counted before `publish` reaches the page; `publish`
//! runs before the scheduler starts, so that is the boot path's own few switches at most.
//!
//! The capability to the page is minted once, for the progenitor, with `READ` and `GRANT`
//! (`kernel::user::boot_progenitor`). Nothing else in the system can name the frame.
//!
//! # BUGS
//!
//! - A counter written before [`publish`] is lost, by the design above.
//! - Interrupts are counted where they reach the scheduler: the tick, a cross-core poke and a
//!   device interrupt routed to a driver. A spurious or unrouted interrupt is not counted.
//! - The soak build's timer-driven `irq_notify` counts as a device interrupt, because it takes the
//!   same path on purpose (`kernel/src/soak.rs`).
//!
//! Name: provisional, minted 2026-09-26 by milestone 126's `free` lane.

use core::sync::atomic::{AtomicPtr, AtomicU64, Ordering};

use machine_statistics_protocol::{WORDS, build_header, word};

use crate::cpu;

/// Where writes go until the page exists. Aligned like the page, so a word is never torn.
#[repr(C, align(64))]
struct Sink([AtomicU64; WORDS]);

static SINK: Sink = Sink([const { AtomicU64::new(0) }; WORDS]);

/// The page's first word, as the kernel reaches it: the sink until [`publish`], then the frame's
/// direct-map address.
static PAGE: AtomicPtr<AtomicU64> =
    AtomicPtr::new(&SINK.0 as *const [AtomicU64; WORDS] as *mut AtomicU64);

/// The frame's physical address once published, for the one capability minted to it.
static PHYS: AtomicU64 = AtomicU64::new(0);

#[inline(always)]
fn at(index: usize) -> &'static AtomicU64 {
    debug_assert!(index < WORDS);
    // SAFETY: `PAGE` names either `SINK` or a frame `publish` allocated and never frees, both at
    // least `WORDS` words long, and `index < WORDS` at every call site (each is a protocol
    // constant, and `cpu::id()` is under `CPU_ID_BOUND`, which `word::cpu` is sized by).
    unsafe { &*PAGE.load(Ordering::Relaxed).add(index) }
}

#[inline(always)]
fn mine(offset: usize) -> &'static AtomicU64 {
    at(word::cpu(cpu::id()) + offset)
}

/// **Make the page**: a zeroed frame, the header, the current frame counts, and from here on every
/// counter lands in it. Called once, at boot, after the frame allocator and before the scheduler.
/// Returns the frame's physical address.
pub fn publish() -> u64 {
    let frame = crate::memory::alloc_zeroed().expect("no frame for the machine statistics page");
    let phys = frame.addr();
    let base = crate::arch::mmu::phys_to_virt(phys) as *mut AtomicU64;
    let header = build_header(page_frames::FRAME_SIZE, crate::arch::timer::TICK_HZ);
    for (i, w) in header.iter().enumerate() {
        // SAFETY: a frame just allocated and zeroed for us alone, reached through the direct map,
        // and the header is one line, far under a frame.
        unsafe { (*base.add(i)).store(*w, Ordering::Relaxed) };
    }
    PHYS.store(phys, Ordering::Relaxed);
    // Release: a core that sees the new pointer sees the header written above.
    PAGE.store(base, Ordering::Release);
    if let Some(s) = crate::memory::stats() {
        frames(s.total, s.total - s.used);
    }
    phys
}

/// The page's physical address, or 0 before [`publish`].
pub fn page_phys() -> u64 {
    PHYS.load(Ordering::Relaxed)
}

/// The allocator's counts, stored by whoever just changed them, under the allocator's lock.
#[inline]
pub fn frames(total: usize, free: usize) {
    at(word::TOTAL_FRAMES).store(total as u64, Ordering::Relaxed);
    at(word::FREE_FRAMES).store(free as u64, Ordering::Relaxed);
}

/// This core made a context switch. On the switch path, so it is one load, one index and one add.
#[inline(always)]
pub fn context_switch() {
    mine(word::CONTEXT_SWITCHES).fetch_add(1, Ordering::Relaxed);
}

/// This core took an interrupt other than its tick (the tick counts itself in [`tick`]). Out of
/// line because its callers sit in the exception dispatchers `script/fastpath-footprint` measures.
#[inline(never)]
pub fn interrupt() {
    mine(word::INTERRUPTS).fetch_add(1, Ordering::Relaxed);
}

/// This core took a tick with `idle` running or not, and `runnable` threads were waiting on it.
#[inline]
pub fn tick(idle: bool, runnable: u64) {
    mine(word::ONLINE).store(1, Ordering::Relaxed);
    let spent = if idle {
        word::IDLE_TICKS
    } else {
        word::BUSY_TICKS
    };
    mine(spent).fetch_add(1, Ordering::Relaxed);
    mine(word::INTERRUPTS).fetch_add(1, Ordering::Relaxed);
    mine(word::RUNNABLE).store(runnable, Ordering::Relaxed);
}
