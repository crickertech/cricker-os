//! **How deep one IPC reaches into a thread's kernel stack** (milestone 134, 2026-09-19).
//!
//! Milestone 134's E1 predicts a knee in IPC latency against thread count from one piece of
//! arithmetic: thread kernel stacks sit `STACK_SLOT_SPAN` apart, each IPC touches a different one,
//! and "taking one IPC to touch roughly 1 to 2 KiB of its own kernel stack" they fill a 32 KB L1d
//! somewhere between 16 and 32 threads. That 1 to 2 KiB was an estimate. notes/stack-high-water.md
//! reports the deepest standing path across the whole suite, which is spawn and teardown rather than
//! IPC, so nothing measured the per-IPC figure. This does.
//!
//! # The method
//!
//! The same watermark as milestone 84 (stack high-water), through its `crate::stack::paint` and
//! `crate::stack::high_water`, re-armed **once per operation** instead of once per stack. Immediately before one IPC operation
//! the thread paints its own kernel stack from the bottom up to [`MARGIN`] bytes below its live `sp`;
//! immediately after, it scans for the deepest word that is no longer paint. The result is the
//! deepest byte that one operation reached, as a distance from the stack's top. Many operations,
//! many samples, and the report gives the minimum, the median and the maximum.
//!
//! Two shapes, each measured two ways:
//!
//! - **Kernel threads** ([`kernel_thread_shapes`]), which is E1's own shape: `bench.rs`'s
//!   `ipc_thread_scaling` runs kernel-thread pairs through `sched::ipc_send`/`ipc_recv`, and
//!   `call_reply` runs `ipc_call` against `ipc_recv_cap`/`ipc_reply`. The thread wraps each call in
//!   [`measured`], so the sample is that one call.
//! - **EL0 threads** ([`el0_shapes`]), which is the shape every real service runs. A user thread
//!   cannot paint its kernel stack, so the kernel does it for a registered thread at the end of every
//!   syscall ([`after_syscall`], called from `syscall::dispatch`): scan what the syscall that is
//!   ending reached, then paint for the next one. An EL0 thread's trap frame sits at the very top of
//!   its kernel stack (`sched::current_kernel_stack_top`'s note), so here the distance from the top
//!   is exactly the stack one syscall touches, trap frame included.
//!
//! # What the instrument costs, and why it does not perturb what it reads
//!
//! **Nothing in a default build**: all of this, including the one call in `syscall::dispatch`, is
//! `any(test, feature = "ipc_stack_depth")`. `script/fastpath-footprint`, `script/bench` and the
//! icount tripwire build neither. In a test build every syscall pays one relaxed load of
//! [`EL0_ARMED`] and returns.
//!
//! **In the path it measures, nothing below the paint.** Painting and scanning are done by the
//! measuring thread in its own frames, which sit *above* the painted ceiling; the operation then runs
//! below them. The one frame that is counted and is not the kernel's is the closure [`measured`] calls
//! (kernel shapes only), a few dozen bytes, and the report prints each series' caller offset so the
//! excursion below the call site can be read directly. What the instrument does cost is time (a
//! paint of up to 24 KiB per sample), which is why it is off in every build that measures time, and
//! why sampling stops once each series is full.
//!
//! # What it does not capture
//!
//! - **Interrupt nesting at the deepest instant.** A timer interrupt taken while an operation is at
//!   its deepest adds a trap frame to that sample (the handler itself runs on the interrupt stack
//!   when the trap is from the kernel; from EL0 it runs on this stack, before the next syscall). Those
//!   samples are the long tail, which is why the **median** is the per-IPC figure and the maximum is
//!   reported beside it as the interrupt-contaminated bound, not as the IPC's depth.
//! - **Real hardware.** Depth is decided by which calls run, not by timing, so QEMU measures it
//!   honestly (notes/stack-high-water.md, "Why the numbers are host-load-immune"), and this is not a
//!   timing number. What differs on silicon is codegen only if the build differs: the release kernel
//!   radon boots and the debug kernel `script/test` boots are different programs, and both are
//!   measured (see notes/stack-high-water.md for which is which).
//! - **Which lines are cache-hot.** A depth says how many bytes one operation *can* touch; it does not
//!   say they are all touched, or that they miss. That is M7's question in milestone 134's tier B.
//! - **Placement.** Under `script/test` the machine has four cores and the two ends may land on
//!   different ones, so a sample can come from the cross-core wake path or the same-core one. Both
//!   ends are spawned on the caller's core, and the report's spread says whether it mattered.
//!
//! Name: provisional (milestone 134 (the register of measures)'s lane, 2026-09-19): module, feature
//! and the `ipc-stack-depth:` line prefix alike. Names are an architect's.

use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};

use crate::{println, sched};

/// Bytes between the arming frame's `sp` and the top of the painted region. It has to cover the
/// frames that run below [`arm`] while it paints (a debug build makes `write_volatile` and even
/// `wrapping_add` real calls) and the scan's frame afterwards. `floor` is this margin plus
/// everything above it, and a sample **at or above** its floor (a reading of `floor` or less) means
/// the operation never went below the margin and the number is the instrument's rather than the
/// kernel's; [`report`] refuses those. A reading can be *less* than `floor`: `KernelStack::new`
/// paints the whole stack in a test build and [`arm`] never repaints `[ceiling, sp)`, so the words
/// just above the ceiling keep the creation paint until something writes them (see
/// [`kernel_thread_shapes`] for what that did to the calibration).
///
/// **Two values, because the first attempt used one and it was wrong in the release build.**
/// milestone 84's boot-stack paint uses 512, and at 512 every release-build series on aarch64 read
/// exactly its floor: the whole IPC path, below the measuring frame, is shallower than 512 bytes
/// there. In release the paint loop is inlined into `arm` and has no callees, so a small margin
/// is enough; in debug it keeps a real call chain, and 256 was tried there and measured too small.
/// Neither number is trusted on its own say-so: [`kernel_thread_shapes`] measures a null operation
/// first (`null` in the report) and that line is the instrument's own reach, whose median must not
/// sit below its floor.
#[cfg(debug_assertions)]
const MARGIN: u64 = 512;
#[cfg(not(debug_assertions))]
const MARGIN: u64 = 64;

/// Samples kept per series. Depth is nearly deterministic (milestone 84 found two runs identical to
/// the byte), so the value of more samples is only in characterising the interrupt tail.
const SAMPLES: usize = 256;

/// One series: one operation, by one role, in one shape. Single writer (the thread that owns the
/// role), read only after that thread has finished, so relaxed atomics are the whole story; they
/// are atomics rather than a `static mut` so the storage needs no `unsafe` at all.
struct Series {
    n: AtomicUsize,
    /// `top - ceiling`: the shallowest value a sample can read, recorded from the last arming.
    floor: AtomicU64,
    /// `top - sp` at the call site, so a reader can subtract the caller's own frames.
    caller: AtomicU64,
    depth: [AtomicU32; SAMPLES],
}

impl Series {
    const fn new() -> Self {
        Series {
            n: AtomicUsize::new(0),
            floor: AtomicU64::new(0),
            caller: AtomicU64::new(0),
            depth: [const { AtomicU32::new(0) }; SAMPLES],
        }
    }

    fn reset(&self) {
        self.n.store(0, Ordering::Relaxed);
    }

    fn is_full(&self) -> bool {
        self.n.load(Ordering::Relaxed) >= SAMPLES
    }

    fn record(&self, used: u64, floor: u64, caller: u64) {
        let i = self.n.fetch_add(1, Ordering::Relaxed);
        if i < SAMPLES {
            self.depth[i].store(used as u32, Ordering::Relaxed);
            self.floor.store(floor, Ordering::Relaxed);
            self.caller.store(caller, Ordering::Relaxed);
        } else {
            self.n.store(SAMPLES, Ordering::Relaxed);
        }
    }
}

/// The current thread's kernel stack as `(bottom, top)`. `KernelStack::new` puts the top exactly
/// `STACK_PAGES` pages above the bottom, and this does the same arithmetic rather than holding the
/// scheduler lock for a second lookup.
fn own_span() -> (u64, u64) {
    let top = sched::current_kernel_stack_top()
        .expect("ipc_stack_depth: this thread has no kernel stack");
    (top - (crate::thread::STACK_PAGES as u64) * 4096, top)
}

/// Paint `[bottom, sp - MARGIN)` and return the ceiling. `inline(never)` so that its frame, and the
/// paint loop's below it, are the same size every time and sit inside the margin.
#[inline(never)]
fn arm(bottom: u64) -> u64 {
    let ceiling = (crate::arch::current_sp() - MARGIN) & !7;
    // SAFETY: `[bottom, ceiling)` is inside the current thread's own kernel stack (from `own_span`),
    // mapped and 8-byte aligned, and everything below `sp - MARGIN` is dead: no frame of this thread
    // lives below its own `sp`, and no other thread ever runs on this stack. The margin covers this
    // function's callees while the loop runs.
    unsafe { crate::stack::paint(bottom, ceiling) };
    ceiling
}

/// The deepest byte reached since the last [`arm`], as a distance from `top`.
#[inline(never)]
fn scan(bottom: u64, top: u64) -> u64 {
    // SAFETY: the span `arm` painted up to its ceiling, on this thread's own mapped stack; above the
    // ceiling is live data, which the scan reads as used, which is correct.
    unsafe { crate::stack::high_water(bottom, top) }
}

/// Run `op` with the stack painted beneath it and record how deep it went.
#[inline(never)]
fn measured<R>(series: &Series, span: (u64, u64), op: impl FnOnce() -> R) -> R {
    let (bottom, top) = span;
    let caller = top - crate::arch::current_sp();
    let ceiling = arm(bottom);
    let r = op();
    let used = scan(bottom, top);
    series.record(used, top - ceiling, caller);
    r
}

// --- The kernel-thread shapes: E1's own ---

static K_NULL: Series = Series::new();
static K_SR_CLIENT_SEND: Series = Series::new();
static K_SR_CLIENT_RECV: Series = Series::new();
static K_SR_SERVER_RECV: Series = Series::new();
static K_SR_SERVER_SEND: Series = Series::new();
static K_CR_CLIENT_CALL: Series = Series::new();
static K_CR_SERVER_RECV_CAP: Series = Series::new();
static K_CR_SERVER_REPLY: Series = Series::new();

/// Unmeasured round trips first, so a lazily built path (the first rendezvous, a first reply
/// capability) is not in the samples. The same idea as `bench.rs`'s `WARMUP`.
const WARMUP: usize = 16;

/// SEND/RECV between two kernel threads, the shape `bench.rs`'s `ipc_rtt` and E1's
/// `ipc_thread_scaling` time.
fn send_recv_kernel() {
    for s in [
        &K_SR_CLIENT_SEND,
        &K_SR_CLIENT_RECV,
        &K_SR_SERVER_RECV,
        &K_SR_SERVER_SEND,
    ] {
        s.reset();
    }
    let request = sched::create_rendezvous();
    let reply = sched::create_rendezvous();
    let done = sched::create_rendezvous();
    let core = crate::cpu::id();

    sched::spawn_on(core, move || {
        let span = own_span();
        loop {
            let m = measured(&K_SR_SERVER_RECV, span, || sched::ipc_recv(request));
            if m[0] == u64::MAX {
                break;
            }
            measured(&K_SR_SERVER_SEND, span, || {
                sched::ipc_send(reply, [m[0], 0, 0]);
            });
        }
    })
    .expect("ipc_stack_depth: no send/recv server");

    sched::spawn_on(core, move || {
        let span = own_span();
        for _ in 0..WARMUP {
            sched::ipc_send(request, [1, 0, 0]);
            sched::ipc_recv(reply);
        }
        for _ in 0..SAMPLES {
            measured(&K_SR_CLIENT_SEND, span, || {
                sched::ipc_send(request, [1, 0, 0]);
            });
            measured(&K_SR_CLIENT_RECV, span, || sched::ipc_recv(reply));
        }
        sched::ipc_send(request, [u64::MAX, 0, 0]);
        sched::ipc_send(done, [0, 0, 0]);
    })
    .expect("ipc_stack_depth: no send/recv client");

    sched::ipc_recv(done);
}

/// CALL against `RECV_CAP` and REPLY between two kernel threads, the shape `bench.rs`'s `call_reply`
/// times and the one real services issue.
fn call_reply_kernel() {
    for s in [&K_CR_CLIENT_CALL, &K_CR_SERVER_RECV_CAP, &K_CR_SERVER_REPLY] {
        s.reset();
    }
    let ep = sched::create_rendezvous();
    let done = sched::create_rendezvous();
    let core = crate::cpu::id();

    sched::spawn_on(core, move || {
        let span = own_span();
        loop {
            let m = measured(&K_CR_SERVER_RECV_CAP, span, || sched::ipc_recv_cap(ep));
            if m[0] == u64::MAX {
                break;
            }
            // The whole of `bench.rs`'s `call_reply` server body, so the sample is what a server
            // does to answer: find the reply capability, reply through it, drop the slot.
            measured(&K_CR_SERVER_REPLY, span, || {
                let slot = m[1];
                let crate::cap::Object::Reply(caller) = sched::current_cap(slot)
                    .expect("ipc_stack_depth: no reply capability")
                    .object
                else {
                    panic!(
                        "ipc_stack_depth: RECV_CAP of a CALL did not deliver a Reply capability"
                    );
                };
                sched::ipc_reply(caller, [m[0], 0]);
                let _ = sched::delete_current_cap(slot);
            });
        }
    })
    .expect("ipc_stack_depth: no call/reply server");

    sched::spawn_on(core, move || {
        let span = own_span();
        for _ in 0..WARMUP {
            sched::ipc_call(ep, [1, 0]);
        }
        for _ in 0..SAMPLES {
            measured(&K_CR_CLIENT_CALL, span, || sched::ipc_call(ep, [1, 0]));
        }
        // A plain SEND meets a server parked in RECV_CAP all the same (`bench.rs`'s `call_reply`).
        sched::ipc_send(ep, [u64::MAX, 0, 0]);
        sched::ipc_send(done, [0, 0, 0]);
    })
    .expect("ipc_stack_depth: no call/reply client");

    sched::ipc_recv(done);
}

/// Both kernel-thread shapes, then their report. Returns how many series fell short of a usable
/// measurement (see [`report`]); zero is a clean run.
pub fn kernel_thread_shapes() -> usize {
    // The calibration: the same `measured` around an operation that does nothing. Whatever it
    // reads is the instrument's own reach (the closure call, `scan`'s frame), and it must be its
    // floor; anything deeper means the margin is too small for this build and every other line
    // is contaminated by the instrument. On a spawned thread like every other series, because the
    // caller may be the boot thread, which runs on the linker-script stack and owns no kernel stack.
    K_NULL.reset();
    let done = sched::create_rendezvous();
    sched::spawn_on(crate::cpu::id(), move || {
        let span = own_span();
        for _ in 0..SAMPLES {
            measured(&K_NULL, span, || core::hint::black_box(()));
        }
        sched::ipc_send(done, [0, 0, 0]);
    })
    .expect("ipc_stack_depth: no calibration thread");
    sched::ipc_recv(done);
    //
    // **Judged on the median, like every other series, and the first version judged it on every
    // sample and was wrong.** Its maximum sits a few hundred bytes below the floor whatever the
    // margin (504 bytes at a 1,024-byte margin in the debug build), which is a trap frame from a
    // timer tick that landed mid-sample, not the instrument: an excursion the margin cannot move
    // is not the margin's. The tail is printed so it stays visible.
    let null_floor = K_NULL.floor.load(Ordering::Relaxed);
    let mut null = [0u32; SAMPLES];
    for (dst, src) in null.iter_mut().zip(K_NULL.depth.iter()) {
        *dst = src.load(Ordering::Relaxed);
    }
    null.sort_unstable();
    let (null_median, null_max) = (u64::from(null[SAMPLES / 2]), u64::from(null[SAMPLES - 1]));
    //
    // **Clean means "no deeper than the floor", not "exactly the floor"** (2026-09-24). The check
    // said `==` until the HVF leg, which runs this suite on the physical core, failed it on every
    // run from the day it merged: median 856 against a floor of 976, 120 bytes *shallower*. That is
    // the instrument's true reach. Its own callees stop 392 bytes into the 512-byte margin, so the
    // words between them and the ceiling are never written and keep the paint `KernelStack::new`
    // laid down, which `arm` never repaints because it paints only below the ceiling. Under TCG
    // the same thread read exactly its floor, and not because the instrument reaches it: measured
    // over three TCG runs, the first 29 to 76 samples read the true reach (920 against 1040 on that
    // build) and every sample after one moment read the floor, because one interrupt trap frame
    // pushed while `sp` sat below the ceiling writes the ceiling word, and nothing ever paints it
    // again. Whether that moment comes before sample 128 (median at floor) or after (median below
    // it) is timing, so `==` was a timing assertion that TCG's slowness happened to pass and the
    // physical core, finishing all 256 samples in microseconds, did not. What the check exists to
    // catch is the margin being too small, which reads *deeper* than the floor, and `<=` still
    // catches exactly that on every accelerator. Nothing TCG could fail on for that reason passes
    // now. See notes/stack-high-water.md.
    let null_clean = null_median <= null_floor;
    println!(
        "ipc-stack-depth: kernel null (the instrument alone) floor {null_floor} median \
         {null_median} max {null_max} margin {MARGIN} {}",
        if null_clean {
            "clean"
        } else {
            "REACHED BELOW ITS MARGIN"
        },
    );

    send_recv_kernel();
    call_reply_kernel();
    let mut bad = usize::from(!null_clean);
    bad += report("kernel", "send_recv", "client", "SEND", &K_SR_CLIENT_SEND);
    bad += report("kernel", "send_recv", "client", "RECV", &K_SR_CLIENT_RECV);
    bad += report("kernel", "send_recv", "server", "RECV", &K_SR_SERVER_RECV);
    bad += report("kernel", "send_recv", "server", "SEND", &K_SR_SERVER_SEND);
    bad += report("kernel", "call_reply", "client", "CALL", &K_CR_CLIENT_CALL);
    bad += report(
        "kernel",
        "call_reply",
        "server",
        "RECV_CAP",
        &K_CR_SERVER_RECV_CAP,
    );
    bad += report(
        "kernel",
        "call_reply",
        "server",
        "REPLY",
        &K_CR_SERVER_REPLY,
    );
    bad
}

// --- The EL0 shapes: what a service runs ---

/// One registered EL0 thread. `tid` is `u64::MAX` when the slot is empty.
struct El0Slot {
    tid: AtomicU64,
    bottom: AtomicU64,
    top: AtomicU64,
    /// The ceiling of the last paint, or 0 before the thread's first syscall: the first window
    /// covers the spawn path into EL0, which is not an IPC, so it primes and records nothing.
    ceiling: AtomicU64,
    /// One series per rendezvous method number (`abi::rendezvous::*` are 0 to 4, and a Reply
    /// capability's `REPLY` is 0). Within one role the methods it issues are distinct, which is
    /// what makes a per-slot index enough to tell SEND from REPLY.
    series: [Series; 8],
}

impl El0Slot {
    const fn new() -> Self {
        El0Slot {
            tid: AtomicU64::new(u64::MAX),
            bottom: AtomicU64::new(0),
            top: AtomicU64::new(0),
            ceiling: AtomicU64::new(0),
            series: [const { Series::new() }; 8],
        }
    }

    /// This role has started at least one series and every series it started is full, so painting
    /// can stop. The "at least one" matters: before the first sample every series is empty, and an
    /// `all` over nothing is true.
    fn is_done(&self) -> bool {
        self.series.iter().any(|s| s.n.load(Ordering::Relaxed) > 0)
            && self
                .series
                .iter()
                .all(|s| s.n.load(Ordering::Relaxed) == 0 || s.is_full())
    }
}

/// Two roles at a time: a client and a server.
static EL0: [El0Slot; 2] = [const { El0Slot::new() }; 2];

/// The only thing an unregistered syscall pays in a test build.
static EL0_ARMED: AtomicBool = AtomicBool::new(false);

/// Register the calling kernel thread as EL0 role `slot`, just before it enters user mode.
fn register(slot: usize) {
    let (bottom, top) = own_span();
    let s = &EL0[slot];
    s.bottom.store(bottom, Ordering::Relaxed);
    s.top.store(top, Ordering::Relaxed);
    s.ceiling.store(0, Ordering::Relaxed);
    s.tid.store(sched::current(), Ordering::Release);
}

fn reset_el0() {
    EL0_ARMED.store(false, Ordering::Release);
    for s in &EL0 {
        s.tid.store(u64::MAX, Ordering::Relaxed);
        for series in &s.series {
            series.reset();
        }
    }
}

/// **Called by `syscall::dispatch` at the end of every syscall** in a test build or with the
/// feature on. `method` is `frame.arg(1)` as the syscall found it (the result registers may since
/// have been overwritten). For an unregistered thread this is one load and a return.
///
/// For a registered one: scan how deep the syscall now ending went, record it against its method,
/// then paint for the next. The window between two of these calls is the syscall itself plus
/// whatever ran on this stack while the thread was in user mode, which is only an interrupt taken
/// from EL0; that is the tail the median discards.
#[inline(never)]
pub fn after_syscall(nr: u64, method: u64) {
    if !EL0_ARMED.load(Ordering::Relaxed) {
        return;
    }
    let me = sched::current();
    let Some(s) = EL0.iter().find(|s| s.tid.load(Ordering::Acquire) == me) else {
        return;
    };
    if s.is_done() {
        return;
    }
    let (bottom, top) = (
        s.bottom.load(Ordering::Relaxed),
        s.top.load(Ordering::Relaxed),
    );
    let ceiling = s.ceiling.load(Ordering::Relaxed);
    if ceiling != 0 && nr == abi::SYS_INVOKE {
        let used = scan(bottom, top);
        // The trap frame is at the top, so the caller's own depth is zero by construction.
        s.series[(method & 7) as usize].record(used, top - ceiling, 0);
    }
    s.ceiling.store(arm(bottom), Ordering::Relaxed);
}

/// `os_primitives_benchmarker`'s roles, as `bench.rs` spells them (`EL_IPC_SERVER`,
/// `EL_IPC_CLIENT`) and the fixture's `_start` matches them. A third copy of a number two binaries
/// agree on, which AGENTS.md rule 7 says should be a crate; recorded in notes/stack-high-water.md's
/// BUGS rather than fixed here, because the fix moves the fixture's protocol.
#[cfg(initrd)]
const EL_IPC_SERVER: u64 = 3;
#[cfg(initrd)]
const EL_IPC_CLIENT: u64 = 4;

/// `soaker`'s roles, likewise already copied once into `soak.rs`.
#[cfg(initrd)]
const SOAK_RESPONDER: u64 = 0;
#[cfg(initrd)]
const SOAK_CALLER: u64 = 1;

/// SEND/RECV between two EL0 processes: `bench.rs`'s `ipc_rtt_el0`, lmbench's `lat_pipe`. The
/// client runs its own fixed loop, reports and exits; the server parks in RECV forever, so the
/// endpoints come from a region a `Holding` reclaims, which is what wakes it to die.
#[cfg(initrd)]
fn send_recv_el0() -> Option<()> {
    use crate::cap::{Rights, rendezvous_cap};
    use crate::user::Spawn;
    use crate::user::holding::Holding;

    let Some(image) = crate::user::program("os_primitives_benchmarker") else {
        println!(
            "ipc-stack-depth: el0 send_recv skipped (no os_primitives_benchmarker in the initrd)"
        );
        return None;
    };
    let region = crate::memory_region::create(4).expect("ipc_stack_depth: no endpoint region");
    let request = sched::create_rendezvous_from(region).expect("no request endpoint");
    let reply = sched::create_rendezvous_from(region).expect("no reply endpoint");
    let report = sched::create_rendezvous_from(region).expect("no report endpoint");
    let core = crate::cpu::id();

    reset_el0();
    EL0_ARMED.store(true, Ordering::Release);

    let server = sched::spawn_on(core, move || {
        register(0);
        crate::user::run(
            image,
            Spawn {
                arg0: EL_IPC_SERVER,
                arg1: 0,
                arg2: 0,
                grants: &[
                    rendezvous_cap(request, Rights::READ),
                    rendezvous_cap(reply, Rights::WRITE),
                ],
                maps: &[],
            },
        )
    })
    .expect("ipc_stack_depth: could not spawn the EL0 server");
    let client = sched::spawn_on(core, move || {
        register(1);
        crate::user::run(
            image,
            Spawn {
                arg0: EL_IPC_CLIENT,
                arg1: 0,
                arg2: 0,
                grants: &[
                    rendezvous_cap(report, Rights::WRITE),
                    rendezvous_cap(request, Rights::WRITE),
                    rendezvous_cap(reply, Rights::READ),
                ],
                maps: &[],
            },
        )
    })
    .expect("ipc_stack_depth: could not spawn the EL0 client");

    sched::ipc_recv(report);
    EL0_ARMED.store(false, Ordering::Release);

    let mut held = Holding::new();
    held.add_thread(server);
    held.add_thread(client);
    held.add_region(region);
    held.release_or_fail("ipc_stack_depth's EL0 send/recv pair");
    Some(())
}

/// CALL against `RECV_CAP` and REPLY between two EL0 processes, using the soak workload's caller and
/// responder, which loop forever: this waits until both have filled their series and then takes
/// them back.
#[cfg(initrd)]
fn call_reply_el0() -> Option<()> {
    use crate::cap::{Rights, rendezvous_cap};
    use crate::user::holding::Holding;
    use crate::user::{Mapping, Spawn};

    let Some(image) = crate::user::program("soaker") else {
        println!("ipc-stack-depth: el0 call_reply skipped (no soaker in the initrd)");
        return None;
    };
    let region = crate::memory_region::create(2).expect("ipc_stack_depth: no endpoint region");
    let ep = sched::create_rendezvous_from(region).expect("no call endpoint");
    // The soaker writes its progress counters to this page; nothing here reads them.
    let page = crate::memory::alloc_zeroed().expect("ipc_stack_depth: no page for the soaker");
    let phys = page.addr();
    let core = crate::cpu::id();

    reset_el0();
    EL0_ARMED.store(true, Ordering::Release);

    let spawn = |slot: usize, role: u64, rights: Rights| {
        sched::spawn_on(core, move || {
            register(slot);
            crate::user::run(
                image,
                Spawn {
                    arg0: role,
                    arg1: slot as u64,
                    arg2: 2 * slot as u64 + 1,
                    grants: &[rendezvous_cap(ep, rights)],
                    maps: &[Mapping {
                        va: soak_page::VA,
                        phys,
                        flags: paging::Flags::user_data(),
                    }],
                },
            )
        })
        .expect("ipc_stack_depth: could not spawn a soaker")
    };
    let responder = spawn(0, SOAK_RESPONDER, Rights::READ);
    let caller = spawn(1, SOAK_CALLER, Rights::WRITE);

    // Wall clock, not a yield count, for `Holding`'s own reason (threads may run on other cores).
    let deadline = crate::arch::timer::now() + 30 * crate::arch::timer::frequency();
    let filled = || {
        EL0[0].series[abi::rendezvous::RECV_CAP as usize].is_full()
            && EL0[0].series[abi::reply::REPLY as usize].is_full()
            && EL0[1].series[abi::rendezvous::CALL as usize].is_full()
    };
    while !filled() && crate::arch::timer::now() < deadline {
        sched::yield_now();
    }
    EL0_ARMED.store(false, Ordering::Release);

    let mut held = Holding::new();
    held.add_thread(responder);
    held.add_thread(caller);
    held.add_region(region);
    held.release_or_fail("ipc_stack_depth's EL0 soaker pair");
    // Only now: a live soaker was writing to it until `release` returned.
    crate::memory::free(page);
    Some(())
}

/// Both EL0 shapes, then their report, in the same form as [`kernel_thread_shapes`].
#[cfg(initrd)]
pub fn el0_shapes() -> usize {
    use abi::rendezvous::{CALL, RECV, RECV_CAP, SEND};
    let mut bad = 0;
    if send_recv_el0().is_some() {
        bad += report(
            "el0",
            "send_recv",
            "client",
            "SEND",
            &EL0[1].series[SEND as usize],
        );
        bad += report(
            "el0",
            "send_recv",
            "client",
            "RECV",
            &EL0[1].series[RECV as usize],
        );
        bad += report(
            "el0",
            "send_recv",
            "server",
            "RECV",
            &EL0[0].series[RECV as usize],
        );
        bad += report(
            "el0",
            "send_recv",
            "server",
            "SEND",
            &EL0[0].series[SEND as usize],
        );
    }
    if call_reply_el0().is_some() {
        let reply = abi::reply::REPLY as usize;
        bad += report(
            "el0",
            "call_reply",
            "client",
            "CALL",
            &EL0[1].series[CALL as usize],
        );
        bad += report(
            "el0",
            "call_reply",
            "server",
            "RECV_CAP",
            &EL0[0].series[RECV_CAP as usize],
        );
        bad += report(
            "el0",
            "call_reply",
            "server",
            "REPLY",
            &EL0[0].series[reply],
        );
    }
    reset_el0();
    bad
}

/// Print one series as a line and say whether it is a measurement. Returns 1 when it is not: too
/// few samples, or a median at the floor (the operation did not go below the paint's ceiling, so
/// the number is the instrument's own frames and margin rather than the kernel's depth). A minimum
/// at the floor with a median above it is a measurement: it is the shorter of two paths one
/// operation can take (RECV that finds its sender already waiting does not block).
fn report(plane: &str, shape: &str, role: &str, op: &str, s: &Series) -> usize {
    let n = s.n.load(Ordering::Relaxed).min(SAMPLES);
    let mut v = [0u32; SAMPLES];
    for (dst, src) in v.iter_mut().zip(s.depth.iter()).take(n) {
        *dst = src.load(Ordering::Relaxed);
    }
    let v = &mut v[..n];
    v.sort_unstable();
    let floor = s.floor.load(Ordering::Relaxed);
    let caller = s.caller.load(Ordering::Relaxed);
    if n == 0 {
        println!("ipc-stack-depth: {plane} {shape} {role} {op} NO SAMPLES");
        return 1;
    }
    let (min, median, max) = (v[0] as u64, v[n / 2] as u64, v[n - 1] as u64);
    if median <= floor {
        // Not a depth: the operation stayed inside the frames the instrument itself occupies (for
        // EL0, `after_syscall` and `arm` below `dispatch`, plus the margin). What is true is the
        // bound, so that is what the line says, rather than a median that is the instrument's.
        println!(
            "ipc-stack-depth: {plane} {shape} {role} {op} AT FLOOR: at most {floor}, shallower than \
             the instrument can see (max {max}, samples {n})"
        );
        return 1;
    }
    // The excursion below the measuring frame: what the operation itself costs, whatever the
    // caller's own depth. For EL0 the caller offset is zero and the two numbers are the same.
    println!(
        "ipc-stack-depth: {plane} {shape} {role} {op} median {median} min {min} max {max} \
         below-caller {} floor {floor} samples {n}",
        median - caller,
    );
    usize::from(n < SAMPLES / 2)
}

#[cfg(test)]
mod tests {
    /// **Per-IPC kernel stack depth, measured** (milestone 134). Prints one `ipc-stack-depth:` line
    /// per role and operation, kernel-thread shapes first, then EL0. Asserts only that each line is
    /// a measurement (enough samples, every one below the paint ceiling), not what the numbers are:
    /// this is the first time they exist, and a threshold belongs after the spread has been seen.
    /// notes/stack-high-water.md carries the readings.
    #[test_case]
    fn one_ipc_reaches_a_measured_depth_into_its_kernel_stack() {
        let bad = super::kernel_thread_shapes();
        #[cfg(initrd)]
        let bad = bad + super::el0_shapes();
        assert_eq!(
            bad, 0,
            "{bad} ipc-stack-depth series were not measurements (see the lines above): too few \
             samples, a median at the paint floor, or an instrument that reached below its margin"
        );
    }
}
