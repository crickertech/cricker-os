//! **The machine statistics page: how the box is doing, held as a capability rather than ambient**
//! (milestone 126 (the `procps` package), DECISIONS §225 (`free` sees the machine and your share) part 2).
//!
//! One read-only page the kernel keeps its machine-wide counters in: physical frames, and per core
//! the ticks spent busy and idle, the context switches, the interrupts taken and the run queue.
//! `free` reads the memory line, `vmstat` reads all of it, and `top`'s summary reads the run queue
//! and the busy share. This crate is the one definition of where each word lives, so the kernel
//! (the one writer) and every reader agree through a dependency rather than two copies of an
//! offset (AGENTS.md rule 7).
//!
//! # A page, not a method, and not ambient
//!
//! §225 ruled the shape: the figures arrive in a page somebody was granted, so an owner can
//! withhold them, and they are read with a load rather than a crossing. Nothing makes the page
//! reachable by default. The progenitor holds the kernel's only capability to it and maps it into a
//! child whose manifest declares `machine`, the same way it hands `date` the clock.
//!
//! # The kernel writes the counters in place
//!
//! There is no copy step and no refresh. Each word is an `AtomicU64` the kernel updates where the
//! event happens: a context switch bumps its core's switch count, a tick bumps its core's busy or
//! idle count and samples its run queue, the frame allocator stores the free count after each change.
//! Every per-core word has exactly one writer, the core it describes, and each core's words sit in
//! their own 64-byte line, so no two cores ever write the same cache line.
//!
//! So a reader sees each word as it was a moment ago, and **the words are not a consistent snapshot
//! of each other**. `busy + idle` on one core can be a tick ahead of another core's. That is the
//! bargain `/proc/stat` makes too, and for the same reason: a seqlock over counters written from
//! the context switch would be a store and a fence on the kernel's hottest path, bought for a
//! consistency no reader of these figures needs.
//!
//! # EXAMPLES
//!
//! A reader over a page a kernel filled, here built on the host from words:
//!
//! ```
//! use machine_statistics_protocol::{Snapshot, WORDS, word};
//!
//! let mut words = [0u64; WORDS];
//! words[word::MAGIC] = u64::from_le_bytes(*b"MACHSTA1");
//! words[word::FRAME_BYTES] = 4096;
//! words[word::TICK_HZ] = 100;
//! words[word::TOTAL_FRAMES] = 32768;
//! words[word::FREE_FRAMES] = 30000;
//! words[word::cpu(0) + word::ONLINE] = 1;
//! words[word::cpu(0) + word::BUSY_TICKS] = 25;
//! words[word::cpu(0) + word::IDLE_TICKS] = 75;
//!
//! let s = Snapshot::from_words(&words).expect("a page the kernel prepared");
//! assert_eq!(s.total_bytes(), 32768 * 4096);
//! assert_eq!(s.online_cpus(), 1);
//! assert_eq!(s.busy_ticks(), 25);
//!
//! // A frame nobody prepared has no answer, rather than an answer of zero.
//! assert!(Snapshot::from_words(&[0u64; WORDS]).is_none());
//! ```
//!
//! # BUGS
//!
//! - The words are not a consistent snapshot of each other; see above.
//! - `busy` is not split into user and system time. The tick knows which thread was running, not
//!   which privilege level it interrupted, and carrying that through three architectures' trap
//!   paths to one counter was not worth a `vmstat` column. So `vmstat` prints `us` and `sy` as one.
//! - The run queue is sampled once per tick per core, so it is up to one tick stale, and a core
//!   that has stopped ticking (a parked core) keeps its last sample.
//! - There is no swap counter, because nife refuses paging out for now (pull request #1356). There
//!   is no page cache or block I/O counter either, because the kernel has none of them to count:
//!   the filesystem and the block driver are userspace programs.
//!
//! Name: provisional, minted 2026-09-26 by milestone 126's `free` lane, for the crate and for "the
//! machine statistics page". §225 called it the machine memory page; it carries the scheduler's
//! counters too, because `vmstat` needs them and a second page would be a second grant for one
//! question.

#![cfg_attr(not(test), no_std)]

use core::sync::atomic::{AtomicU64, Ordering};

pub use current_cpu_protocol::CPU_ID_BOUND;

/// The page's first word: what tells a prepared page from a frame nobody has written.
pub const MAGIC: [u8; 8] = *b"MACHSTA1";

/// The words the page carries: one header line, then one line per possible core.
pub const WORDS: usize = word::LINE * (1 + CPU_ID_BOUND);

/// The page's size in bytes. Far under one frame, which is the unit it is mapped as.
pub const PAGE_BYTES: usize = WORDS * 8;

const _: () = assert!(
    PAGE_BYTES <= 4096,
    "the page must fit the frame it is mapped as"
);
const _: () = assert!(
    PAGE_VA.is_multiple_of(4096),
    "a mapping starts on a page boundary"
);

/// **Where a child that declares `machine` finds the page**, read-only: the last page of the 2 MiB
/// block every child's program and stack already live in (the ELF loads at `0x40_0000` and the
/// stack sits under `supervision_protocol::CHILD_STACK_VA`, `0x50_0000`), so mapping it costs a
/// spawn no page-table frame of its own, the measurement `current_cpu_protocol::PAGE_VA` records.
///
/// **A measured choice, not a first guess.** The first build put it at `0x00f0_0000`, beside the
/// configuration page, which reads as tidy and cost a fresh last-level table on every spawn of a
/// program that maps no configuration page. On `x86_64`, whose debug `top` is nineteen pages, that
/// one table took the job over its forty-page region, and `top` could not be spawned. Here the only
/// thing that could collide is a program over a megabyte, which would already have collided with
/// its own stack. Provisional.
pub const PAGE_VA: u64 = 0x005F_F000;

/// **Word indices into the page.** A per-core word is `cpu(id) + OFFSET`.
pub mod word {
    /// Words per 64-byte line.
    pub const LINE: usize = 8;

    /// [`MAGIC`](super::MAGIC), as a little-endian word.
    pub const MAGIC: usize = 0;
    /// Bytes per frame, so a reader can print bytes without assuming a page size.
    pub const FRAME_BYTES: usize = 1;
    /// How many ticks make a second, so a reader can turn tick counts into time.
    pub const TICK_HZ: usize = 2;
    /// Frames the allocator tracks in total.
    pub const TOTAL_FRAMES: usize = 3;
    /// Frames free right now.
    pub const FREE_FRAMES: usize = 4;

    /// The first word of core `id`'s line. `id` is a cpu id, a name and not an index into the
    /// online set (`current_cpu_protocol`'s own warning).
    #[must_use]
    pub const fn cpu(id: usize) -> usize {
        LINE * (1 + id)
    }

    /// 1 once the core has taken a tick. A core never seen stays 0.
    pub const ONLINE: usize = 0;
    /// Ticks this core spent running something other than its idle thread.
    pub const BUSY_TICKS: usize = 1;
    /// Ticks this core spent in its idle thread.
    pub const IDLE_TICKS: usize = 2;
    /// Context switches this core has made.
    pub const CONTEXT_SWITCHES: usize = 3;
    /// Interrupts this core has taken: timer, cross-core and device.
    pub const INTERRUPTS: usize = 4;
    /// Threads runnable on this core at its last tick, counting the running one unless it is idle.
    pub const RUNNABLE: usize = 5;
}

/// Build a fresh page's header: the magic, the frame size and the tick rate. The kernel writes this
/// into a zeroed frame once, at boot, before any counter moves.
#[must_use]
pub fn build_header(frame_bytes: u64, tick_hz: u64) -> [u64; word::LINE] {
    let mut line = [0u64; word::LINE];
    line[word::MAGIC] = u64::from_le_bytes(MAGIC);
    line[word::FRAME_BYTES] = frame_bytes;
    line[word::TICK_HZ] = tick_hz;
    line
}

/// **One word of the page, for its one writer.** Relaxed, and the reason is in the crate docs: each
/// word has one writer and no reader needs the words consistent with each other.
///
/// # Safety
///
/// `page_va` must name a mapped, 8-byte-aligned buffer of at least [`PAGE_BYTES`] bytes, live for
/// the call, and `index` must be under [`WORDS`].
#[inline]
pub unsafe fn counter(page_va: u64, index: usize) -> &'static AtomicU64 {
    // SAFETY: the caller's contract: in bounds, aligned, live.
    unsafe { &*((page_va + (index as u64) * 8) as *const AtomicU64) }
}

/// One core's line, as a reader sees it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cpu {
    /// The core has taken at least one tick.
    pub online: bool,
    /// Ticks spent running something other than the idle thread.
    pub busy_ticks: u64,
    /// Ticks spent idle.
    pub idle_ticks: u64,
    /// Context switches.
    pub context_switches: u64,
    /// Interrupts taken.
    pub interrupts: u64,
    /// Runnable threads at the last tick.
    pub runnable: u64,
}

/// **The page, read once.** Each field is one word as it stood when it was read; see the crate docs
/// for why the fields are not consistent with each other.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Snapshot {
    /// Bytes per frame.
    pub frame_bytes: u64,
    /// Ticks per second.
    pub tick_hz: u64,
    /// Frames the allocator tracks.
    pub total_frames: u64,
    /// Frames free.
    pub free_frames: u64,
    /// Every possible core's line, indexed by cpu id.
    pub cpus: [Cpu; CPU_ID_BOUND],
}

impl Snapshot {
    /// Read a page from its words, or `None` if the magic is missing: a frame nobody prepared
    /// reads as zeroes, and zero free memory is a plausible, wrong answer.
    #[must_use]
    pub fn from_words(words: &[u64; WORDS]) -> Option<Snapshot> {
        if words[word::MAGIC] != u64::from_le_bytes(MAGIC) {
            return None;
        }
        let mut cpus = [Cpu::default(); CPU_ID_BOUND];
        for (id, c) in cpus.iter_mut().enumerate() {
            let at = |offset: usize| words[word::cpu(id) + offset];
            *c = Cpu {
                online: at(word::ONLINE) != 0,
                busy_ticks: at(word::BUSY_TICKS),
                idle_ticks: at(word::IDLE_TICKS),
                context_switches: at(word::CONTEXT_SWITCHES),
                interrupts: at(word::INTERRUPTS),
                runnable: at(word::RUNNABLE),
            };
        }
        Some(Snapshot {
            frame_bytes: words[word::FRAME_BYTES],
            tick_hz: words[word::TICK_HZ],
            total_frames: words[word::TOTAL_FRAMES],
            free_frames: words[word::FREE_FRAMES],
            cpus,
        })
    }

    /// Read the page mapped at `va`.
    ///
    /// # Safety
    ///
    /// `va` must be a mapped, 8-byte-aligned buffer of at least [`PAGE_BYTES`] bytes that stays
    /// mapped for the call. A progenitor maps exactly such a page, read-only, at [`PAGE_VA`].
    #[must_use]
    pub unsafe fn read(va: u64) -> Option<Snapshot> {
        let mut words = [0u64; WORDS];
        for (i, w) in words.iter_mut().enumerate() {
            // SAFETY: the caller's contract, and `i < WORDS`.
            *w = unsafe { counter(va, i) }.load(Ordering::Relaxed);
        }
        Snapshot::from_words(&words)
    }

    /// Bytes the allocator tracks.
    #[must_use]
    pub fn total_bytes(&self) -> u64 {
        self.total_frames.saturating_mul(self.frame_bytes)
    }

    /// Bytes free.
    #[must_use]
    pub fn free_bytes(&self) -> u64 {
        self.free_frames.saturating_mul(self.frame_bytes)
    }

    fn sum(&self, f: impl Fn(&Cpu) -> u64) -> u64 {
        self.cpus.iter().filter(|c| c.online).map(f).sum()
    }

    /// Cores that have ticked.
    #[must_use]
    pub fn online_cpus(&self) -> usize {
        self.cpus.iter().filter(|c| c.online).count()
    }

    /// Busy ticks over every online core.
    #[must_use]
    pub fn busy_ticks(&self) -> u64 {
        self.sum(|c| c.busy_ticks)
    }

    /// Idle ticks over every online core.
    #[must_use]
    pub fn idle_ticks(&self) -> u64 {
        self.sum(|c| c.idle_ticks)
    }

    /// Context switches over every online core.
    #[must_use]
    pub fn context_switches(&self) -> u64 {
        self.sum(|c| c.context_switches)
    }

    /// Interrupts over every online core.
    #[must_use]
    pub fn interrupts(&self) -> u64 {
        self.sum(|c| c.interrupts)
    }

    /// Runnable threads over every online core, as of each core's last tick.
    #[must_use]
    pub fn runnable(&self) -> u64 {
        self.sum(|c| c.runnable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prepared() -> [u64; WORDS] {
        let mut w = [0u64; WORDS];
        w[..word::LINE].copy_from_slice(&build_header(4096, 100));
        w
    }

    #[test]
    fn every_line_is_its_own_cache_line() {
        assert_eq!(word::cpu(0), 8);
        assert_eq!(word::cpu(CPU_ID_BOUND - 1) + word::LINE, WORDS);
    }

    #[test]
    fn an_offline_core_counts_for_nothing_even_if_its_words_are_not_zero() {
        let mut w = prepared();
        w[word::cpu(1) + word::ONLINE] = 1;
        w[word::cpu(1) + word::CONTEXT_SWITCHES] = 7;
        w[word::cpu(3) + word::CONTEXT_SWITCHES] = 1000; // never ticked
        let s = Snapshot::from_words(&w).unwrap();
        assert_eq!(s.online_cpus(), 1);
        assert_eq!(s.context_switches(), 7);
    }

    #[test]
    fn the_header_carries_what_a_reader_needs_to_print_units() {
        let s = Snapshot::from_words(&prepared()).unwrap();
        assert_eq!((s.frame_bytes, s.tick_hz), (4096, 100));
    }

    #[test]
    fn a_page_without_the_magic_has_no_answer() {
        let mut w = prepared();
        w[word::MAGIC] ^= 1;
        assert!(Snapshot::from_words(&w).is_none());
    }
}
