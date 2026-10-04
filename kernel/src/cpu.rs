//! Per-CPU state.
//!
//! Each core reaches its own private data in a single instruction, through a pointer the
//! architecture keeps in a scratch register for exactly this purpose (`TPIDR_EL1` on aarch64;
//! see [`crate::arch::set_percpu`]). This is the foundation the rest of SMP is built on: once a
//! core can name "my own state" cheaply, the scheduler's run queue, its current thread, and its
//! lock bookkeeping can each stop being a single machine-wide global. See DECISIONS §11.
//!
//! **Step 1 of §11.** For now the only thing that lives here is the lock-rank bookkeeping §9
//! keeps, which used to be one global (`HELD_RANK`) and would be clobbered the instant a second
//! core took a lock. Moving it here changes nothing on one core and is the smallest provable
//! piece of the per-CPU machinery. The block grows in step 3 to hold this core's run queue,
//! `current`, idle thread, reschedule flag, and migration inbox.

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};

use intrusive_fifo::Fifo;

use crate::sync::{IrqSafeMutex, rank};
use crate::thread::{Thread, ThreadId};

/// A `current`/`idle` slot holding no thread. Tids are small integers from 0 up, so `u64::MAX`
/// can never collide with a real one.
pub const NO_TID: ThreadId = u64::MAX;

/// The most cores we support: the seats, not the machine. A fixed maximum lets the blocks be a
/// static array, so they exist before there is a heap and can be pointed at from a core's very
/// first Rust instruction, and `smp::read_cpu_list` seats each described core at the slot its
/// hart id names, so this bounds hart *ids*, not just core counts.
///
/// Eight, raised from four on 2026-08-14, when the VisionFive 2 turned out to describe five harts
/// (the unusable S7 monitor plus four U74s) and the fourth U74 fell off a four-seat roster. Eight
/// rather than five because the bound is on ids a board may use, not on one board's count: the
/// next small system-on-chip reopens five, and the mask arithmetic (`cpu_set`, the SBI IPI hart bitmaps)
/// is exhaustively host-tested over exactly eight bits. What each seat costs whether or not a
/// core fills it: one `PerCpu` block, one timer/trap-stash entry, one event ring and a roster
/// slot, about 330 bytes of statics per seat (measured at the bump; it vanished inside the
/// sections' page rounding), and the real cost, one 68 KiB secondary-stack slot (64 KiB stack
/// over a 4 KiB guard page, milestone 90) in the NOLOAD stack region: 272 KiB of RAM for the
/// four new seats. QEMU `virt` gives us as many cores as `-smp` asks; the runners default to 4
/// and `NIFE_SMP` moves the legs up to this ceiling.
///
/// **The number itself lives in `crates/current_cpu_protocol`**, and this is one name for it
/// rather than a second copy. It stopped being a kernel-private constant on 2026-09-21: a
/// userspace program that reads its own CPU out of a page has to size a per-CPU array by
/// something, and "one past the highest id the kernel will hand out" is the only safe answer,
/// because the online set is not `0..count` (`crates/cpu_set`, and the three boots it cost on
/// first silicon). Two binaries agreeing on a number is what AGENTS.md rule 7 is about.
pub const MAX_CPUS: usize = current_cpu_protocol::CPU_ID_BOUND;

/// One core's private data.
pub struct PerCpu {
    /// The lowest lock rank this core currently holds (`rank::NONE` when it holds nothing).
    ///
    /// Only ever touched by this core with interrupts masked, so the atomic is for interior
    /// mutability through the shared static, not for cross-core synchronization: no other core
    /// can reach *this* core's block on the lock path. See [`crate::sync`] and DECISIONS §9.
    pub held_rank: AtomicU32,

    /// The thread currently running on this core (`NO_TID` before the core schedules).
    ///
    /// Was a single field on the global `IpcTables`; per-CPU as of §11 step 3b, because each core
    /// runs a different thread. An atomic because a remote core may *read* it (the reaper checks
    /// whether a thread is current on any core), even though only the owning core ever writes it.
    pub current: AtomicU64,

    /// This core's idle thread (`NO_TID` before it exists): what runs when this core's run queue
    /// is empty. Each core gets its own, so an idle core parks in its *own* `wfi`.
    pub idle: AtomicU64,

    /// Set by this core's timer tick, read on this core's return from the IRQ. Per-CPU so one
    /// core's tick cannot make another core reschedule. See DECISIONS §9's record-and-defer.
    pub need_resched: AtomicBool,

    /// The thread this core just switched **away from**, to be finished up by the thread this
    /// core switched **to** (`NO_TID` between switches). See `sched::finish_switch`.
    ///
    /// Two races end here, both the same shape: something must not happen to a thread while any
    /// core is still switching off its stack, so it is not done by whoever notices it is wanted;
    /// it is recorded by the thread's own core before the switch and done by its successor
    /// *after* the switch, when the thread is provably off its stack.
    ///
    /// - **Reaping** (the original, DECISIONS §11): a `Finished` predecessor is freed by its
    ///   successor, never by a remote observer.
    /// - **Deferred wakes** (milestone 14 phase A.3): a `Blocked` predecessor that a waker
    ///   caught *mid-switch-out* (the handshake's `on_cpu` still set, its saved context still
    ///   stale) is not queued by the waker; the wake is parked in `wake_pending` (both fields on
    ///   `thread_wake_handshake::Handshake`, the loom-searched extraction) and completed by
    ///   the successor, after the context is real. Without this, a rendezvous or interrupt
    ///   arriving in that window put the thread on a second core while its first was still
    ///   running it: two cores in one thread, on a stale register file.
    ///
    /// Only this core touches it, with interrupts masked.
    pub switched_from: AtomicU64,

    /// This core's run queue: the threads ready to run here, in round-robin order.
    ///
    /// **Intrusive** as of milestone 14 phase A.2 (notes/intrusive-queues.md): the queue is two
    /// pointers, the links live inside the `Thread`s, and a push can neither allocate nor fail,
    /// which retires the old pre-reserved-capacity apology to the IRQ path. Entries are TCB
    /// pointers, not Tids: what a pop hands back is the thread itself, no lookup. The pointers
    /// are kept alive by the queue discipline: a queued thread is `Ready`, and only `Finished`
    /// threads are ever reaped.
    ///
    /// **No cross-core lock, by design (DECISIONS §11).** Only this core ever touches its own
    /// queue, and only with interrupts masked, which is exactly what makes the `UnsafeCell`
    /// sound. That a remote core cannot even *name* this queue is the point: it forces cross-core
    /// work movement onto the inbox/SGI path (step 3c) rather than letting one core reach into
    /// another's queue. Access it through [`with_runq`](Self::with_runq).
    runq: UnsafeCell<Fifo<Thread>>,

    /// This core's migration inbox: threads handed to it by *other* cores (SMP step 3c).
    ///
    /// **The one cross-core scheduler structure**, and the reason the run queue needs no lock. To
    /// give this core a thread, another core locks this and pushes the TCB, then sends a
    /// reschedule SGI; this core drains it into its own run queue in the handler. A remote core
    /// touches only the inbox, never the run queue. See [`inbox_of`] and `sched::drain_inbox`.
    pub inbox: IrqSafeMutex<Fifo<Thread>>,

    /// **This core's run-queue length, mirrored as an atomic so other cores may read it** (SMP
    /// placement, DECISIONS §28). The owner keeps it exact by writing `runq.len()` after every
    /// [`with_runq`](Self::with_runq); a placer or a would-be thief reads it (through
    /// [`runnable`](Self::runnable)) **relaxed and possibly stale**, which §28 accepts on purpose
    /// (the gossip lesson: an approximate load reading beats a synchronised one). It is the run
    /// queue's depth, not counting the thread currently on the CPU.
    runq_len: AtomicUsize,

    /// **This core's inbox length, mirrored the same way.** Work handed to this core by a remote
    /// (or a steal being served) sits in the inbox until this core's next scheduler entry drains it.
    /// Counting it in [`runnable`](Self::runnable) is what keeps a burst of remote placements from
    /// looking like an idle core: without it, a core with a full inbox but an empty run queue reads
    /// zero load, so placers pile more on it and idle cores wrongly steal from elsewhere. Updated
    /// under the inbox lock by whoever changed it (the remote pusher, or this core's drain).
    inbox_len: AtomicUsize,

    /// **A pending work-steal request from an idle core** (§28's message-shaped stealing). An idle
    /// core claims this slot and pokes this core with the reschedule SGI; this core, at its next
    /// scheduler entry, hands one runnable thread back to the requester's inbox and clears the
    /// slot. One outstanding request at a time, so a thundering herd of idle cores collapses to one
    /// steal per victim per round.
    ///
    /// **The one lock-free cross-core protocol in the scheduler**, and therefore the one place a
    /// weak-memory bug could hide where no lock would catch it. Milestone 80 lifted it into
    /// `work_steal_slot` so loom can explore every interleaving of it on the host; this field is
    /// the checked type itself, not a copy of it. See notes/interleaving.md.
    pub steal_request: work_steal_slot::Slot,

    /// **How many context switches this core has made**, monotonic since boot: the exact count
    /// behind `vmstat`'s `cs` column (milestone 126 (the `procps` package)).
    ///
    /// Incremented by `sched::schedule` on every switch, beside the `switched_from` store it already
    /// makes through the same block, and copied to the machine statistics page by this core's tick
    /// (`sched::count_tick`). Milestone 629 (the context-switch statistic stops costing the switch
    /// path) moved it here from a `fetch_add` on the page itself, which chased the page pointer on
    /// every switch: about 134 instructions a switch at -O0 and 14 in release before, 31 and 3
    /// after. The page lags this word by up to one tick, which the protocol crate already allows.
    ///
    /// **It took the slot `adopted` had**, which now lives in [`ADOPTED`], so this struct stays 128
    /// bytes; see the assertion below [`PERCPU`] for what one more word once cost. Written only by
    /// this core, with interrupts masked; read relaxed by the tick on this core and by tests.
    pub switches: AtomicU64,

    /// **This core's placement PRNG** (§28's power of two choices): xorshift32, seeded per boot per
    /// core in [`init_this_cpu`], advanced only by this core (spawn runs on the placing core, with
    /// interrupts masked), so a relaxed atomic is interior mutability, not synchronisation. A
    /// deterministic PRNG, not real entropy: there is none before an OS, and determinism keeps the
    /// icount benchmark instrument reproducible.
    rng: AtomicU32,

    /// **`x86_64` only: this core's own trap-entry scratch** (milestone 161's SMP item). Three words
    /// `trap.s` reaches through a `gs`-relative offset, computed at compile time via
    /// `core::mem::offset_of!` (see `arch::x86_64::mod`'s `global_asm!` call): `IA32_GS_BASE`
    /// already names this exact block on this CPU, so a second per-CPU array (RISC-V's `sscratch`
    /// answer, needed there because `tp` is a general register a trap frame can carry stale) buys
    /// nothing here that an offset into the block `gs` already points at does not.
    ///
    /// Used only by this core, only through raw `gs`-relative reads and writes in `trap.s` and
    /// through `segments::init`; nothing ever reads another core's copy. Plain `u64`s behind
    /// `UnsafeCell`, the same reasoning `runq` above gives: only this
    /// core ever touches its own instance, so there is no data race for an atomic to guard against.
    #[cfg(target_arch = "x86_64")]
    pub x86_trap: X86TrapPerCpu,
}

/// See [`PerCpu::x86_trap`].
#[cfg(target_arch = "x86_64")]
pub struct X86TrapPerCpu {
    /// This core's own `TSS.rsp0` field's address (a pointer, not a stack top): `isr_restore`
    /// writes the outgoing thread's kernel-stack top through it on every return to ring 3.
    /// Written once, by `segments::init`, from that CPU's own per-CPU TSS.
    pub tss_rsp0_ptr: UnsafeCell<u64>,
    /// The `syscall` path's kernel stack top for this core: `x86_syscall_entry` loads `rsp` from
    /// here. Kept in step with `tss_rsp0_ptr`'s target by `isr_restore`, which writes both on
    /// every return to ring 3, same as the trap path's `TSS.rsp0`.
    pub syscall_kernel_rsp: UnsafeCell<u64>,
    /// Scratch for the interrupted user `rsp`, parked here by `x86_syscall_entry` between its
    /// `swapgs` and the point it can be pushed into the trap frame (`syscall` does not switch
    /// stacks itself, so there is nowhere else to hold it while it is in flight).
    pub syscall_user_rsp: UnsafeCell<u64>,
    /// **This block's own address**, written by `arch::x86_64::set_percpu` right after it points
    /// `IA32_GS_BASE` here, so `arch::x86_64::percpu` is one `gs`-relative load instead of an
    /// `rdmsr` (milestone 758 (the IPC fast paths shrink back inside their band), provisional).
    /// That is the x86_64 answer to aarch64's `mrs TPIDR_EL1` and RISC-V's `mv tp`: Linux keeps
    /// `this_cpu_off` for the same reason. Read only through `gs`, so only by the owning core.
    pub self_ptr: UnsafeCell<u64>,
}

// SAFETY: as `PerCpu` itself: only the owning core ever touches its own instance.
#[cfg(target_arch = "x86_64")]
unsafe impl Sync for X86TrapPerCpu {}

#[cfg(target_arch = "x86_64")]
impl X86TrapPerCpu {
    const fn new() -> Self {
        Self {
            tss_rsp0_ptr: UnsafeCell::new(0),
            syscall_kernel_rsp: UnsafeCell::new(0),
            syscall_user_rsp: UnsafeCell::new(0),
            self_ptr: UnsafeCell::new(0),
        }
    }
}

// SAFETY: the only non-`Sync` field is `runq`, and the whole contract of this type is that a
// `PerCpu` block is touched only by its owning core, with interrupts masked (see `with_runq`). No
// two cores ever reach the same block, so there is no cross-core data race to guard against, and
// the atomics handle this core's own interrupt-vs-mainline reentrancy on the fields that need it.
unsafe impl Sync for PerCpu {}

impl PerCpu {
    const fn new() -> Self {
        Self {
            held_rank: AtomicU32::new(rank::NONE),
            current: AtomicU64::new(NO_TID),
            idle: AtomicU64::new(NO_TID),
            need_resched: AtomicBool::new(false),
            switched_from: AtomicU64::new(NO_TID),
            runq: UnsafeCell::new(Fifo::new()),
            inbox: IrqSafeMutex::new(rank::INBOX, Fifo::new()),
            runq_len: AtomicUsize::new(0),
            inbox_len: AtomicUsize::new(0),
            steal_request: work_steal_slot::Slot::new(),
            switches: AtomicU64::new(0),
            rng: AtomicU32::new(1), // reseeded per core in init_this_cpu; never left 0 (xorshift)
            #[cfg(target_arch = "x86_64")]
            x86_trap: X86TrapPerCpu::new(),
        }
    }

    /// Run `f` with exclusive access to this core's run queue.
    ///
    /// # Invariant
    ///
    /// The caller must have interrupts masked (asserted in debug builds). Combined with
    /// single-owner access, that makes the `&mut` genuinely exclusive: this core cannot re-enter
    /// through an interrupt mid-borrow, and no other core can reach this block at all. This is the
    /// standard per-CPU pattern; the `UnsafeCell` is sound precisely because of that invariant.
    /// Every caller today already holds `IPC_TABLES` (which masks interrupts) or has masked them
    /// explicitly in `schedule()`.
    pub fn with_runq<R>(&self, f: impl FnOnce(&mut Fifo<Thread>) -> R) -> R {
        debug_assert!(
            !crate::arch::interrupts::is_enabled(),
            "run queue touched with interrupts enabled: single-owner safety needs them masked",
        );
        // SAFETY: interrupts masked (asserted) and single-owner, so this `&mut` is exclusive.
        let q = unsafe { &mut *self.runq.get() };
        let r = f(q);
        // Mirror the depth so other cores can read this core's load without touching its queue
        // (DECISIONS §28). `Fifo::len` is O(1), so this is one store per queue operation. Relaxed: the
        // reader tolerates staleness, and the queue itself is single-owner.
        self.runq_len.store(q.len(), Ordering::Relaxed);
        r
    }

    /// Record this core's inbox length after a push or drain. Called with the inbox lock held, by
    /// the remote core that pushed or by this core's drain, so the store is serialised by the lock.
    pub fn note_inbox_len(&self, len: usize) {
        self.inbox_len.store(len, Ordering::Relaxed);
    }

    /// This block's index in [`PERCPU`], for the per-core arrays that live beside it rather than in
    /// it ([`ADOPTED`]). Off every hot path.
    fn index(&self) -> usize {
        (core::ptr::from_ref(self) as usize - PERCPU.as_ptr() as usize)
            / core::mem::size_of::<PerCpu>()
    }

    /// Count `n` threads adopted from this core's inbox onto its run queue. Called by the owning
    /// core's drain, under the inbox lock.
    pub fn note_adopted(&self, n: u64) {
        ADOPTED[self.index()].fetch_add(n, Ordering::Relaxed);
    }

    /// How many threads this core has taken out of its own inbox since boot. Monotonic, so a test
    /// can read it either side of a placement and see the delivery even if the thread has already
    /// moved on. Relaxed: the only writer is the owning core, and a reader wants "has it happened
    /// yet", which is a question a stale read answers late rather than wrongly.
    #[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
    pub fn adopted(&self) -> u64 {
        ADOPTED[self.index()].load(Ordering::Relaxed)
    }

    /// This core's total load, for a **placement** decision (DECISIONS §28.1): queued threads plus
    /// inbox threads not yet drained. Relaxed and possibly stale by design.
    pub fn runnable(&self) -> usize {
        self.runq_len.load(Ordering::Relaxed) + self.inbox_len.load(Ordering::Relaxed)
    }

    /// This core's **run-queue** backlog alone, for a **steal** decision (DECISIONS §28.3). A thief
    /// targets this, not `runnable`, so it never tries to take work still sitting in a victim's inbox
    /// on its way to that victim (which the victim's own imminent drain will run): only a real
    /// backlog of already-queued threads is worth stealing. Relaxed and possibly stale.
    pub fn runq_len(&self) -> usize {
        self.runq_len.load(Ordering::Relaxed)
    }

    /// Advance this core's placement PRNG (xorshift32) and return the next value. Only the owning
    /// core calls this, on the spawn path with interrupts masked, so the relaxed load/store pair is
    /// interior mutability rather than cross-core synchronisation.
    pub fn rng_next(&self) -> u32 {
        let mut x = self.rng.load(Ordering::Relaxed);
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng.store(x, Ordering::Relaxed);
        x
    }
}

/// **How many threads each core has adopted out of its own inbox**, monotonic since boot.
///
/// The observable end of the cross-core placement path (§11 step 3c): a remote core pushes a
/// thread into this inbox and pokes this core, and this core's `drain_inbox` moves it onto its own
/// run queue. `inbox_len` cannot serve here because it is a depth, not a count: a push and a drain
/// that happen between two reads of it leave it exactly where it was.
///
/// One relaxed increment per adopted thread, on a path that already takes a lock. It exists for
/// `smp::tests::work_can_be_placed_on_every_cpu`, which asserts delivery to the *named* core and
/// cannot ask "did that core end up running it": stealing (§28 (SMP placement), part 3) may legitimately move the thread
/// first, so execution-on-the-target is not a property `spawn_on` promises. See
/// notes/load-sensitive-assertions.md.
///
/// **An array beside `PerCpu` rather than a field in it** since milestone 629, which gave its slot
/// to [`PerCpu::switches`]: a switch is on every hot path and a drain is on none, so the switch
/// count is the one that earns the block's 128 bytes.
static ADOPTED: [AtomicU64; MAX_CPUS] = [const { AtomicU64::new(0) }; MAX_CPUS];

/// The per-CPU blocks, one per core.
///
/// Statically allocated on purpose: a core's block must be reachable from its first instruction
/// of Rust, long before any allocator exists. `TPIDR_EL1` points at one element of this array.
static PERCPU: [PerCpu; MAX_CPUS] = [const { PerCpu::new() }; MAX_CPUS];

/// **`PerCpu`'s size must stay a power of two, and this is the check that says so.**
/// Added by milestone 541 (a timed window that excludes preemption).
///
/// `PERCPU[id]` is an index into an array of these, so the address is `base + id * size_of`. When
/// the size is a power of two that multiply is a shift, and `cpu::current()` inlines into a couple
/// of instructions at every one of its many call sites, several of which are on the IPC fastpath.
/// When it is not, every one of those sites grows.
///
/// **The cost is measured, not feared.** Adding one `AtomicU64` field took this struct from 128
/// bytes to 136 and cost **150 bytes on the riscv64 `ipc_send_receive` closure and 38 on
/// `syscall_entry`**, taking the first from +2.2% to +5.4% against `script/fastpath-footprint`'s
/// 5% bound and turning a green gate red. The field was never read on that path and the increment
/// that wrote it cost nothing measurable: removing the `fetch_add` and keeping the field left every
/// figure byte-identical. It was the size, and nothing in this file had ever said the size
/// mattered.
///
/// So a field added here is not free even when the fastpath never reads it. The cheap place for
/// per-core state that the fastpath does not need is its own array, which is what
/// `sched::PREEMPTIONS_PER_CPU` is and why. This assertion is here so the next person learns that
/// from a failing build rather than from a CI gate they did not run.
///
/// **`x86_64` is exempt, and the exemption is measured rather than assumed.** There this struct
/// carries `x86_trap` and is already **160 bytes**, so it has never indexed with a shift, and
/// `script/fastpath-footprint --arch x86_64` is green with room: +1.0% on `ipc_send_receive`, +1.4%
/// on `ipc_call_reply`, +3.9% on `syscall_entry`. Writing the assertion unconditionally would
/// assert a property this tree does not hold, which is how a gate teaches people to route around
/// it. If the cost ever shows up there, the fix is padding this struct to 256 rather than deleting
/// the check.
#[cfg(not(target_arch = "x86_64"))]
const _: () = assert!(
    core::mem::size_of::<PerCpu>().is_power_of_two(),
    "PerCpu's size must be a power of two so PERCPU[id] indexes with a shift; see the note above \
     this assertion for the 150 bytes of IPC fastpath the last violation cost",
);

/// Point this core's `TPIDR_EL1` at its `PerCpu` block.
///
/// Call once, on each core, **before that core takes its first lock**, because the lock path
/// reads `current().held_rank`. On the boot core that means before `console::init` in
/// `kernel_main`; on a secondary it is the first thing the bring-up entry does.
pub fn init_this_cpu(id: usize) {
    assert!(id < MAX_CPUS, "cpu id {id} exceeds MAX_CPUS {MAX_CPUS}");
    crate::arch::set_percpu(&PERCPU[id] as *const PerCpu as usize);
    // Seed this core's placement PRNG (§28). A per-core deterministic seed: the same boot always
    // makes the same choices (the icount instrument wants that), but the cores differ so they do
    // not sample in lockstep. `| 1` keeps it nonzero, which xorshift requires.
    PERCPU[id].rng.store(
        0x2545_F491 ^ (id as u32).wrapping_mul(0x9E37_79B9) | 1,
        Ordering::Relaxed,
    );
}

/// This core's private block, in one instruction.
pub fn current() -> &'static PerCpu {
    // SAFETY: `init_this_cpu` set TPIDR_EL1 to the address of an element of the `'static`
    // `PERCPU` array, and nothing else ever writes TPIDR_EL1. The one window where this would be
    // wrong is before `init_this_cpu` has run on this core, which is exactly why that call is the
    // first thing `kernel_main` (and each secondary's entry) does, ahead of any lock.
    unsafe { &*(crate::arch::percpu() as *const PerCpu) }
}

/// This core's logical id: its index into `PERCPU`.
pub fn id() -> usize {
    let base = PERCPU.as_ptr() as usize;
    // Wrapping, because the wrap cannot happen and this is on every trap and every switch (release
    // builds check overflow, notes/overflow-checks.md). `init_this_cpu` is the only writer of the
    // per-CPU register and it stores `&PERCPU[id]`, which is at or above `base`. Were that ever
    // broken, the wrapped quotient is far past `MAX_CPUS` and the next `PERCPU[id]` index refuses
    // it, which is a stronger check than the subtraction's.
    crate::arch::percpu().wrapping_sub(base) / core::mem::size_of::<PerCpu>()
}

/// Another core's migration inbox, by id. This is the one place a core reaches into a *different*
/// core's `PerCpu` block, and only the inbox (a real lock), never the run queue. See DECISIONS §11.
pub fn inbox_of(id: usize) -> &'static IrqSafeMutex<Fifo<Thread>> {
    &PERCPU[id].inbox
}

/// Any core's block, by id, for read-only diagnostics (the hang dump). Reading another core's
/// atomics is racy but best-effort, which is all a post-mortem dump needs.
#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
pub fn of(id: usize) -> &'static PerCpu {
    &PERCPU[id]
}

#[cfg(test)]
mod tests {
    use core::sync::atomic::Ordering;

    use super::*;

    /// Whatever core this runs on set up its per-CPU pointer, and `current()` reaches that core's
    /// block and no other.
    ///
    /// This used to additionally assert `id() == arch::boot_cpu_id()`, and that assertion was a
    /// placement claim wearing a per-CPU test's clothes. It failed about one full run in four once
    /// DECISIONS §28's idle stealing started letting a secondary core pull the test thread. The
    /// failure was honest: `id()` reads `TPIDR_EL1` (`tp` on RISC-V), which is per-CORE and set at
    /// core init, and which no context switch saves or restores, so an `id()` of 1 means the test
    /// genuinely ran on core 1. Nothing was broken except the test's assumption about where it runs.
    ///
    /// So it asserts the property its own doc comment always described: `current()` is
    /// self-consistent with `id()`. That holds on every core, which makes it stronger coverage rather
    /// than weaker: under §28 placement the suite scatters, so over a run this gets exercised on
    /// several cores instead of only the boot core.
    ///
    /// The alternative was giving kernel tests boot-core affinity, and it is the wrong trade. Pinning
    /// tests to core 0 would buy this one assertion back at the cost of running the whole suite on
    /// one core, which is precisely where the placement bugs §28 introduced would hide. A harness
    /// that avoids the scheduler it is meant to test is not a harness.
    ///
    /// `boot_cpu_id()` is still read rather than assumed to be 0, because on RISC-V the boot hart is
    /// whichever one QEMU picks; it is used here only to prove the id is a plausible online core.
    #[test_case]
    fn percpu_is_self_consistent_on_whatever_cpu_we_run() {
        let me = id();
        assert!(me < MAX_CPUS, "cpu::id() returned {me}, out of range");
        assert_eq!(
            current() as *const PerCpu,
            &PERCPU[me] as *const PerCpu,
            "current() does not point at this core's own block"
        );
        // The boot core's block is reachable by index from any core, which is what `of()` promises
        // and what the cross-core paths (IPI, stealing) rely on.
        let boot = crate::arch::boot_cpu_id();
        assert_eq!(
            of(boot) as *const PerCpu,
            &PERCPU[boot] as *const PerCpu,
            "of(boot) does not reach the boot core's block"
        );
    }

    /// The lock-rank bookkeeping lives in the per-CPU block now.
    ///
    /// A full lock cycle is exercised by the rank tests in sync.rs; this just proves the storage
    /// moved and reads back coherently. Between tests we hold nothing.
    #[test_case]
    fn held_rank_lives_in_the_percpu_block() {
        assert_eq!(current().held_rank.load(Ordering::Relaxed), rank::NONE);
    }
}
