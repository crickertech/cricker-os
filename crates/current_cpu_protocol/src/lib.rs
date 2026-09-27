//! **A thread's own CPU, as a page rather than a crossing.**
//!
//! calef ruled on 2026-09-21 that two different questions get two different mechanisms: observing
//! *another* thread is a selector on the rendezvous surface, and a thread observing *itself* is a
//! per-thread page on the shape Linux's `rseq(2)` uses. That ruling's `design/decisions/` section
//! is on another branch and is not on `main` yet, so it is named here rather than cited; the
//! citation lands when it does. This crate is the self half: one definition of the page the kernel
//! writes the running core's id into, so the kernel (the one writer) and
//! `user_mode_runtime::current_cpu` (every reader) cannot drift on the layout or the fixed virtual
//! address (AGENTS.md rule 7).
//!
//! # Why a page and not a syscall
//!
//! The consumer that decides this is a memory allocator keeping a per-CPU cache: it asks which CPU
//! it is on **once per allocation**, which is millions of times a second. An IPC round trip in this
//! tree measures ~705 ns and a bare syscall is cheaper than that, but neither is in the same
//! decade as a load. `rseq` exists for exactly this reason, and Linux's own numbers for the move
//! from the vDSO path to a memory read were 20x on x86 and 35x on ARM.
//!
//! # Why a page and not a register, which is the question a reader asks next
//!
//! Checked against the three architectures this tree targets rather than taken from Linux's
//! experience:
//!
//! - **`aarch64`.** `MPIDR_EL1` is EL1 and above, so EL0 cannot read the core's identity at all.
//!   There is one register a kernel could abuse for this, `TPIDRRO_EL0`, which is EL1-writable and
//!   EL0-readable, and this tree does not use it for anything today. It was refused rather than
//!   overlooked: see the refusals below.
//! - **`riscv64`.** `mhartid` is M-mode. S-mode has `sscratch` and U-mode has nothing, so there is no
//!   register path at all, not even an abusable one. This is the architecture that decides it.
//! - **`x86_64`.** `RDPID`, and the older `LSL` against a GDT limit, do give ring 3 a core id, which
//!   is why the Linux vDSO's `getcpu` is an x86 story and arm64 ships no `__vdso_getcpu`.
//!
//! So a register path exists on one of three targets and half exists on a second. Architectural
//! parity is a gate here, §19 (architectural parity is a tenet),
//! and a mechanism that is a register on `x86_64` and a page on `riscv64` is two mechanisms to learn,
//! two to test and two to document, for a saving that is one load either way. The page is also the
//! shape that survives the next fact: the deciding argument in the 2026-09-21 ruling was that calef
//! expects a third and a fourth per-thread fact, and a page has room for them where a register does
//! not.
//!
//! # One writer, and it is the reader's own core
//!
//! This page's word changes while the process runs, which is the one way it differs from
//! `counter_frequency_protocol`'s page (written once at boot, then never again). The discipline that
//! makes it safe is stronger than a seqlock and needs no ordering argument beyond naming it:
//!
//! - A thread runs on exactly one core at a time, and **the core that writes this word is the core
//!   that is about to execute the thread**: the kernel writes it in `schedule()`, on the way in,
//!   on the destination core itself (and in `sched::adopt_address_space`, for a thread that gains
//!   a space while already running, which is that same write arriving late). Writer and reader are
//!   the same hardware thread, so program order does the work and there is nothing to fence.
//! - The handoff between two *successive* writers (this thread moving from one core to another) is
//!   ordered by the scheduler's own release/acquire pair on the run queue, the same pair that
//!   publishes the thread's saved context. This word rides that; it does not need its own.
//! - The store and the load are therefore **`Relaxed`**, and the counterpart is that existing
//!   release/acquire handoff rather than a fence of this crate's own. An aligned 64-bit access is
//!   single-copy-atomic on all three targets, so nothing tears.
//!
//! **A reader can be stale, inherently, and no ordering fixes that**: the thread may be migrated
//! the instruction after it reads. `rseq`'s restartable sequences are Linux's answer to that and
//! are deliberately not attempted here; a per-CPU cache built on this page must be correct (not
//! merely fast) when the answer turns out to be last core's.
//!
//! # The unset case is representable, because a wrong number is worse than no number
//!
//! calef ruled that the same day, about the counter frequency. Two states read as [`None`] rather
//! than as a plausible zero:
//!
//! - A frame nobody prepared reads as zeroes, and zero is a **valid CPU id**. So the page carries
//!   [`MAGIC`] and an unrecognized page has no answer at all.
//! - A prepared page whose thread has never been switched in carries [`UNSCHEDULED`], written at
//!   page-build time, not zero. In practice no code *inside* the thread can observe it, because a
//!   thread cannot execute an instruction without having been switched in first, which is the same
//!   argument by which the value is never stale-in-a-harmful-way on its first read.
//!
//! # A CPU id is not an index, and this is the trap the tree has already paid for
//!
//! The word is a **cpu id**, the kernel's own numbering, and the online set is not `0..count`. On
//! the VisionFive 2 it is `{1, 2, 3}`: slot 0 is an M-mode monitor core with no MMU, and treating
//! the count as a bound put `init` into a parked core's inbox and cost three boots on first
//! silicon (`crates/cpu_set` is the crate that exists because of it). A userspace consumer sizing a
//! per-CPU array must size it by [`CPU_ID_BOUND`] and never by however many cores it has seen.
//!
//! # EXAMPLES
//!
//! A built page reads back the core the kernel last published:
//!
//! ```
//! use current_cpu_protocol::{CurrentCpuPage, build_page, publish};
//!
//! let mut bytes = build_page();
//! let va = bytes.as_mut_ptr() as u64;
//! // SAFETY: `bytes` is a live, 8-aligned buffer of exactly `PAGE_BYTES` for this block, built
//! // by `build_page`, and this thread is its only writer.
//! unsafe { publish(va, 3) };
//! // SAFETY: as above.
//! let page = unsafe { CurrentCpuPage::new(va) };
//! assert_eq!(page.cpu(), Some(3));
//! ```
//!
//! Sizing a per-CPU array, which is the consumer this page exists for:
//!
//! ```
//! # use current_cpu_protocol::{CurrentCpuPage, build_page, publish, CPU_ID_BOUND};
//! let mut caches = [0u64; CPU_ID_BOUND]; // never `[0; cores_i_have_seen]`
//! # let mut bytes = build_page();
//! # let va = bytes.as_mut_ptr() as u64;
//! # // SAFETY: as the example above.
//! # unsafe { publish(va, 3) };
//! # // SAFETY: as the example above.
//! # let page = unsafe { CurrentCpuPage::new(va) };
//! if let Some(cpu) = page.cpu() {
//!     caches[cpu] += 1;
//! }
//! ```
//!
//! Neither a zeroed frame nor a thread that has never run fabricates an answer:
//!
//! ```
//! use current_cpu_protocol::{CurrentCpuPage, PAGE_BYTES, build_page};
//!
//! let zeroed = [0u8; PAGE_BYTES];
//! // SAFETY: a live, aligned buffer of exactly `PAGE_BYTES` for this block.
//! assert_eq!(unsafe { CurrentCpuPage::new(zeroed.as_ptr() as u64) }.cpu(), None);
//!
//! let fresh = build_page();
//! // SAFETY: as above.
//! assert_eq!(unsafe { CurrentCpuPage::new(fresh.as_ptr() as u64) }.cpu(), None);
//! ```
//!
//! # BUGS
//!
//! - **A thread that shares an address space with another thread would share this page, and both
//!   would read one of the two answers.** That cannot happen today: `Tcb::CONFIGURE` consumes the
//!   address-space capability, so no two TCBs name one space: §105 (`std::thread::spawn` stays
//!   declined). This page is per address space,
//!   which is per thread only because of that. **Whoever lifts §105 must make this per thread by
//!   something other than the address space**, and a shared page indexed by a slot the thread
//!   learns at startup is the obvious shape, with the false-sharing cost of packed slots to weigh.
//!   Named here because the constraint is invisible from this crate's own code.
//! - **A process whose address space was built by userspace and never bound to a TCB has no page
//!   here**, and a call to `user_mode_runtime::current_cpu` from it faults on an unmapped read
//!   rather than returning [`None`]. The kernel maps this page when it takes the space
//!   (`AddressSpace::new` for the spaces it builds, `sched::configure_thread_control_block` for the
//!   ones userspace hands it), and those two cover every space that ever runs a thread. The uncovered
//!   shape is a bare address-space object that is used for something other than running a thread,
//!   which by construction has no thread to ask. Deliberately **not** mapped in
//!   `user::user_address_space_create`: doing that for the timebase page cost two regressions on a
//!   hand-sized demo region, and that comment is still beside that function.
//! - **The value can be stale by the time it is used**, for the reason the ordering section gives.
//!   This crate will not fix that, and a consumer that cannot tolerate it wants `rseq`'s
//!   restartable sequences, which are a milestone of their own and not proposed here.
//! - **[`CPU_ID_BOUND`] is a compile-time constant, not this machine's online set.** It is the
//!   kernel's `MAX_CPUS`, single-sourced here so the two cannot drift, and it is deliberately not
//!   the live count: a bound that grew when a secondary came online would be a number userspace
//!   had already sized an array against. Correct for sizing, useless for iterating, and a consumer
//!   that wants to iterate the online set is asking the other question, which the selector answers.
//!
//! Name: provisional (this lane, 2026-09-21). calef names the crates. `current_cpu` is the
//! vocabulary the field already uses for exactly this quantity (Linux's `sched_getcpu`,
//! `smp_processor_id`, `rseq`'s own `cpu_id` field), which puts it in the protected class a reader
//! already knows; `_protocol` is calef's 2026-09-05 ruling on the suffix, the one
//! `counter_frequency_protocol` carries. Refused `self_cpu_protocol`, which reads as a property of
//! a thing called "self" rather than as the current core, and which borrows the branch name rather
//! than the concept. Refused `cpu_id_protocol` as naming the field rather than the page's question.
//! Refused `rseq_protocol`: it is the right prior art and the wrong name, because this carries none
//! of `rseq`'s restartable-sequence machinery and a reader who knew the term would expect it.
//! Refused a register-shaped name (`tpidrro_protocol` and friends) for the reason the register
//! section gives, which is that the register does not exist on two of three targets.

#![cfg_attr(not(test), no_std)]

use core::sync::atomic::{AtomicU64, Ordering};

/// The page's first eight bytes: what tells a prepared page from a frame nobody has written. ASCII,
/// unpadded, the same shape `counter_frequency_protocol::MAGIC` and `clock_protocol::MAGIC` use.
pub const MAGIC: [u8; 8] = *b"CURRCPU1";

/// **What the CPU word holds before the kernel has ever switched this thread in.** Not zero,
/// because zero is a valid CPU id on every machine this tree runs on except the VisionFive 2, and
/// "a wrong number is worse than no number" (calef, 2026-09-21).
pub const UNSCHEDULED: u64 = u64::MAX;

/// **One past the highest CPU id this kernel will ever hand out**: the size a per-CPU array in
/// userspace must have. The kernel's `cpu::MAX_CPUS` *is* this constant rather than a second copy
/// of the number, which is AGENTS.md rule 7 applied to a value two binaries agree on.
///
/// **Not the online count.** See the crate's `BUGS` section, and `crates/cpu_set` for why counting
/// is not indexing.
pub const CPU_ID_BOUND: usize = 8;

const OFF_MAGIC: usize = 0;
const OFF_CPU: usize = OFF_MAGIC + 8;

/// The whole page's size in bytes: the magic, then one little-endian `u64`. Far under one frame
/// (4096 bytes), which is the unit this is mapped as, and inside one cache line, which is what
/// makes the reader a single load with no line shared with anybody else's writes.
pub const PAGE_BYTES: usize = OFF_CPU + 8;

/// **The fixed virtual address the kernel maps this page at** in every process it builds a space
/// for, and it is the same number on all three architectures. Both sides agree on it through this
/// crate rather than through two copies of a constant.
///
/// **Beside the stack, in the same last-level table**, and the reason is a measurement rather than a
/// convention. The first draft put this at three quarters of each architecture's low half, copying
/// `counter_frequency_protocol::PAGE_VA` (seven eighths) and its argument: a page mapped
/// unconditionally into every process must not share a neighbourhood with the low few megabytes
/// where every ELF loads and every fixture maps its own windows. That argument is right about
/// collisions and says nothing about what the address *costs*, and the cost turned out to be the
/// larger fact.
///
/// A virtual address alone in a far corner of the space is alone in its **page tables** too. On
/// `aarch64` a 4 KiB-granule walk indexes L1 on bits 38:30, L2 on 29:21 and L3 on 20:12, so an
/// address sharing no gigabyte with anything the process already maps needs a fresh L1 entry, a
/// fresh L2 table and a fresh L3 table: **three page-table frames retyped and zeroed, per address
/// space**, every spawn. `script/bench`'s `spawn_el0` measured that at **1,245 ticks per spawn**
/// (+10.25% against the recorded baseline, over the gate's 10% bound), of which only 277 was
/// allocating and zeroing this page's own frame. The other 968 was the walk.
///
/// So the page goes **under tables the process already paid for**. From 2026-09-21 that was the
/// last page of the first gigabyte (`0x3FFF_F000`), which shared the L1 and L2 tables every program's
/// segments paid for and bought one L3, measured at 741 ticks a spawn. Sharing the L3 as well would
/// have meant sitting in the same 2 MiB as the program's own segments, a collision hazard, because
/// nothing said which pages were whose.
///
/// **Since 2026-09-26 it is `address_space_map::CURRENT_CPU_PAGE`, `0x7FFF_F000`**, and the hazard is
/// gone rather than avoided (milestone 206 (a program image has under 896 KiB), DECISIONS §171 (where a program image starts) option D). The map moved every program's
/// image and stack to the top of the second gigabyte and reserved the last 64 KiB of it,
/// `PROCESS_PAGES`, for pages the kernel maps into every process unasked. This page is the last of
/// them, one page above the stack's top page and in the same 2 MiB, so it shares the stack's L1, L2
/// **and** L3: the mapping buys no table at all, only its leaf. The first gigabyte's tables, which
/// this page used to lean on, are no longer paid for by every program, so staying there would have
/// cost two more tables per spawn than it did.
///
/// **One number rather than three**, which the first draft could not have: `0x7FFF_F000` is inside
/// every one of these architectures' low halves, Sv39's included (its `SPLIT_SHIFT` is 38, so its
/// low half ends at `0x0000_0040_0000_0000`). The host build gets the same constant and maps
/// nothing anywhere.
pub const PAGE_VA: u64 = address_space_map::CURRENT_CPU_PAGE;

/// Build a fresh page's bytes: the magic, and [`UNSCHEDULED`] for a thread that has not run yet.
/// The kernel writes this into a zeroed frame before mapping it read-only into the process, and
/// nothing else ever constructs one.
///
/// There is no `build_page(cpu)`: the CPU is not known at build time and is not the kind of thing
/// a builder should be able to assert. [`publish`] is the only way the word ever changes.
pub fn build_page() -> [u8; PAGE_BYTES] {
    let mut page = [0u8; PAGE_BYTES];
    page[OFF_MAGIC..OFF_MAGIC + 8].copy_from_slice(&MAGIC);
    page[OFF_CPU..OFF_CPU + 8].copy_from_slice(&UNSCHEDULED.to_le_bytes());
    page
}

/// **Publish the core a thread is about to run on.** The kernel's half, called from the context
/// switch with the id of the core doing the switching, which is the core the thread will execute
/// on. It is one relaxed store and it does not check the magic: this is the hottest line this crate
/// contributes to, and the page it names was built by [`build_page`] two mappings ago.
///
/// `Relaxed` is the whole ordering, and its counterpart is named in the crate docs: the scheduler's
/// own release/acquire handoff, which already orders everything else about a thread moving to a
/// core. Nothing here needs a fence of its own and adding one would be cargo cult.
///
/// # Safety
///
/// `page_va` must name a mapped, 8-byte-aligned buffer of at least [`PAGE_BYTES`] bytes that this
/// caller is the only writer of, live for the duration of the call. In the kernel that is the
/// direct-map view of a frame the address space owns and nothing else can reach.
#[inline]
pub unsafe fn publish(page_va: u64, cpu: u64) {
    // SAFETY: the caller's contract gives us a live, aligned, exclusively written buffer of at
    // least `PAGE_BYTES`, and `OFF_CPU + 8` is exactly `PAGE_BYTES`.
    let word = unsafe { &*((page_va + OFF_CPU as u64) as *const AtomicU64) };
    word.store(cpu, Ordering::Relaxed);
}

/// **The current-CPU page**, as seen through one thread's read-only mapping of it at [`PAGE_VA`].
#[derive(Debug, Clone, Copy)]
pub struct CurrentCpuPage {
    base: *const u8,
}

// **Deliberately NOT `Send` or `Sync`**, which is where this differs from
// `counter_frequency_protocol::TimebasePage`. That page is the same bytes for every thread, so
// handing one across a thread boundary is meaningful; this one is *this* thread's answer and
// carrying it to another thread would carry a wrong one. Nothing in the tree needs to, so nothing
// asserts that it may: the raw pointer makes the type neither by default, and every hand-written
// `unsafe impl Send`/`Sync` in this tree is a claim that the compiler is wrong, counted by
// `script/lint`. Two claims not made is two fewer to be wrong about.

impl CurrentCpuPage {
    /// Name the current-CPU page mapped at `va`.
    ///
    /// # Safety
    ///
    /// `va` must be a mapped, 8-byte-aligned buffer of at least [`PAGE_BYTES`] bytes (a page
    /// [`build_page`] wrote, or a zeroed frame, which reads as "unknown"), and it must stay mapped
    /// for as long as this value is used. The kernel maps exactly such a page, read-only, at
    /// [`PAGE_VA`]; see the crate's `BUGS` section for the shape that does not get one.
    pub const unsafe fn new(va: u64) -> Self {
        CurrentCpuPage {
            base: va as *const u8,
        }
    }

    /// **Which CPU this thread is running on**, or `None` if this page is unrecognized (a frame
    /// nobody prepared) or its thread has never been switched in. Never a fabricated 0.
    ///
    /// Two aligned loads and two comparisons, no syscall. The magic is checked on every call rather
    /// than once at startup because the page is one cache line, so the check is in the line the
    /// answer is in and costs a compare rather than a miss.
    pub fn cpu(&self) -> Option<usize> {
        // SAFETY: `new`'s contract: `base` names at least `PAGE_BYTES` mapped, stable bytes.
        let magic = unsafe { core::slice::from_raw_parts(self.base, 8) };
        if magic != MAGIC {
            return None;
        }
        // SAFETY: `new`'s contract again, plus its alignment clause: the word is 8-aligned
        // because `base` is. Reached through an integer rather than by offsetting the `*const u8`
        // so the cast is not one clippy has to take on trust, which is the same shape `publish`
        // uses on the writing side.
        let word = unsafe { &*((self.base as u64 + OFF_CPU as u64) as *const AtomicU64) };
        match word.load(Ordering::Relaxed) {
            UNSCHEDULED => None,
            cpu => Some(cpu as usize),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The round trip: a fresh page, a published core, the id read back the way a mapped frame
    /// would be named.
    #[test]
    fn a_published_cpu_reads_back() {
        let mut bytes = build_page();
        let va = bytes.as_mut_ptr() as u64;
        // SAFETY: a live, aligned `PAGE_BYTES` buffer this thread alone writes.
        unsafe { publish(va, 2) };
        // SAFETY: as above.
        assert_eq!(unsafe { CurrentCpuPage::new(va) }.cpu(), Some(2));
    }

    /// **Zero is an answer, not an absence**, which is the whole reason the unset state is a magic
    /// and a sentinel rather than a zeroed word.
    #[test]
    fn cpu_zero_is_a_real_answer() {
        let mut bytes = build_page();
        let va = bytes.as_mut_ptr() as u64;
        // SAFETY: as above.
        unsafe { publish(va, 0) };
        // SAFETY: as above.
        assert_eq!(unsafe { CurrentCpuPage::new(va) }.cpu(), Some(0));
    }

    /// A frame nobody prepared reads as unknown, not as CPU 0.
    #[test]
    fn a_zeroed_frame_reads_as_unknown() {
        let zeroed = [0u8; PAGE_BYTES];
        // SAFETY: as above.
        let page = unsafe { CurrentCpuPage::new(zeroed.as_ptr() as u64) };
        assert_eq!(page.cpu(), None);
    }

    /// A prepared page whose thread has never been switched in reads as unknown, and the sentinel
    /// that makes that true is written by `build_page` rather than left to a zeroed frame.
    #[test]
    fn a_thread_that_never_ran_reads_as_unknown() {
        let bytes = build_page();
        assert_eq!(&bytes[OFF_CPU..], &UNSCHEDULED.to_le_bytes());
        // SAFETY: as above.
        let page = unsafe { CurrentCpuPage::new(bytes.as_ptr() as u64) };
        assert_eq!(page.cpu(), None);
    }

    /// An unrecognized magic refuses the answer even when the word after it is a plausible core
    /// id: the magic check runs first, the discipline `counter_frequency_protocol`'s equivalent
    /// test pins.
    #[test]
    fn an_unrecognized_magic_reads_as_unknown() {
        let mut bytes = build_page();
        bytes[OFF_MAGIC] ^= 0xff;
        let va = bytes.as_mut_ptr() as u64;
        // SAFETY: as above.
        unsafe { publish(va, 1) };
        // SAFETY: as above.
        let page = unsafe { CurrentCpuPage::new(va) };
        assert_eq!(page.cpu(), None);
    }

    /// Publishing twice is what a migration looks like from this page's side: the second write
    /// wins and nothing carries over from the first.
    #[test]
    fn a_migration_overwrites_rather_than_accumulates() {
        let mut bytes = build_page();
        let va = bytes.as_mut_ptr() as u64;
        // SAFETY: as above.
        unsafe { publish(va, 3) };
        // SAFETY: as above.
        unsafe { publish(va, 1) };
        // SAFETY: as above.
        assert_eq!(unsafe { CurrentCpuPage::new(va) }.cpu(), Some(1));
    }

    /// Every CPU id this kernel can hand out survives the round trip, including the top of the
    /// range, so a sentinel chosen badly (say `CPU_ID_BOUND`) would fail here.
    #[test]
    fn every_id_in_the_bound_round_trips() {
        let mut bytes = build_page();
        let va = bytes.as_mut_ptr() as u64;
        for id in 0..CPU_ID_BOUND {
            // SAFETY: as above.
            unsafe { publish(va, id as u64) };
            // SAFETY: as above.
            assert_eq!(unsafe { CurrentCpuPage::new(va) }.cpu(), Some(id));
        }
    }

    /// The layout constants do not overlap and the page fits in one frame and one cache line,
    /// pinned so a mutant swapping an offset is caught here rather than by a wrong core id on real
    /// hardware. The shape `counter_frequency_protocol::the_layout_offsets_do_not_overlap` uses.
    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn the_layout_offsets_do_not_overlap() {
        assert_eq!(OFF_CPU, 8);
        assert_eq!(PAGE_BYTES, 16);
        assert!(PAGE_BYTES <= 64, "the page must fit in one cache line");
        assert!(PAGE_BYTES < 4096, "the page must fit in one frame");
    }

    /// **The page shares the stack's last-level table**, which is what makes the mapping cost only
    /// its leaf. A 4 KiB-granule L3 covers 2 MiB on all three architectures, so two addresses share
    /// one when they agree above bit 21.
    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn the_page_shares_the_stacks_last_level_table() {
        assert_eq!(PAGE_VA % 4096, 0);
        assert!(address_space_map::PROCESS_PAGES.contains(PAGE_VA));
        assert_eq!(
            PAGE_VA >> 21,
            address_space_map::STACK_TOP_PAGE >> 21,
            "not in the stack's 2 MiB, so the mapping buys an L3 of its own"
        );
    }
}
