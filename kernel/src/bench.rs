//! Microbenchmarks over the paths a microkernel lives on (milestone 21).
//!
//! Compiled in only by `--features bench` (`script/bench`); the bench boot diverges here before
//! the milestone tour, runs each benchmark in a fixed order, prints machine-readable lines, and
//! **halts**. It never semihosts: under HVF the semihosting `hlt` traps to the guest instead of
//! exiting (see xtask's `test()`), so the contract is output-based in both modes: `xtask bench`
//! owns the QEMU process, watches for `bench: done`, and terminates it. One exit mechanism,
//! accelerator-independent.
//!
//! # The two instruments (design/roadmap/21-benchmarks.md)
//!
//! - **icount (default):** QEMU virtual time is a deterministic function of instructions
//!   executed, so these counter deltas are *exact and reproducible per binary*: the same kernel
//!   prints the same numbers every run. But they are NOT stable across *different* binaries: adding
//!   unrelated live code shifts even untouched benchmarks by several percent, non-uniformly, because
//!   the compiler remakes whole-crate inlining and monomorphization decisions (notes/benchmarks.md
//!   has the measurement). So `bench/baseline-aarch64.txt` + `--check` is a **coarse tripwire** (10%) for a
//!   gross regression, not a fine attributor. Magnitudes are fiction anyway (TCG models no caches);
//!   the `--real` medians are the fine signal.
//! - **HVF (`--real`):** the kernel runs natively on the host core; real caches, real TLBs, the
//!   hardware counter at its real frequency. Magnitudes are true, determinism is gone (a shared
//!   desktop machine underneath), so real runs report and never gate.
//!
//! # Reading the numbers
//!
//! Each line is `bench: <name> <counter_ticks> <iters>`. The counter is `CNTVCT_EL0` at
//! `CNTFRQ_EL0` Hz (printed first), so ns/iter = ticks * 1e9 / freq / iters; xtask does the
//! division. Warmup iterations run untimed before each measurement so thread spawn and first
//! rendezvous costs land outside the window.
//!
//! **Every program it enters is measured first** (milestone 563 (a seal check that reads bytes
//! cannot see a check that was dropped)), through `trust::require_program`, the chain the
//! progenitor would have run: this boot replaces the hand-over, so without it a bench card would
//! enter unchecked bytes and its kernel would carry no trust root for the seal to find. The hashing
//! happens at set-up, outside every timed window.

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::cap::{Rights, address_space_cap, memory_region_cap, page_frame_cap, rendezvous_cap};
use crate::{println, sched};

/// Iterations per benchmark. Fixed and part of the output, so a baseline is self-describing.
const YIELD_ITERS: u64 = 2000;
const IPC_ITERS: u64 = 1000;
const RELAY_ITERS: u64 = 1000;
const CALL_ITERS: u64 = 1000;
const SPAWN_ITERS: u64 = 64;
const MAP_ITERS: u64 = 64;
const COREMARK_ITERS: u64 = 256;

/// Untimed shakeout before each measured loop: thread startup, first rendezvous, cold paths.
const WARMUP: u64 = 32;

fn timed(name: &str, iters: u64, f: impl FnOnce()) {
    // The monotonic counter, through the arch abstraction: CNTVCT_EL0 on aarch64, `rdtime` on
    // RISC-V. `frequency()` (printed by `run` as `cntfrq`) turns ticks into seconds.
    let t0 = crate::arch::timer::now();
    f();
    let t1 = crate::arch::timer::now();
    println!("bench: {name} {} {iters}", t1 - t0);
}

/// Run every benchmark and halt. Never returns, never semihosts (see the module doc).
pub fn run() -> ! {
    println!();
    println!("bench: cntfrq {}", crate::arch::timer::frequency());
    // Which E3 image this is (milestone 134's layout control): nothing else on a card says, and E3
    // interleaves up to eight images through one card or one TFTP slot. A probe, so it is echoed
    // and never becomes a row. See kernel/src/fastpath_pad.rs for why the numbers are read as data.
    #[cfg(feature = "fastpath_pad")]
    {
        let [units, shift] = *core::hint::black_box(&crate::fastpath_pad::BUILD);
        println!("bench-probe: fastpath_pad units {units} shift {shift}");
    }

    // Milestone 134's per-IPC stack depth, in the build it is otherwise impossible to measure: the
    // release kernel radon boots. First, so nothing earlier in the suite is on any stack it reads,
    // and never in a timing build (the feature is off in every one; see kernel/Cargo.toml). The
    // rows after it still run, and their times are not results with this on.
    #[cfg(feature = "ipc_stack_depth")]
    {
        let bad =
            crate::ipc_stack_depth::kernel_thread_shapes() + crate::ipc_stack_depth::el0_shapes();
        println!("ipc-stack-depth: done ({bad} series were not measurements)");
    }

    yield_switch();
    #[cfg(target_arch = "x86_64")]
    tss_iomap_switch();
    #[cfg(target_arch = "x86_64")]
    tss_iomap_lazy_switch();
    #[cfg(target_arch = "x86_64")]
    tss_iomap_lazy_nop();
    ipc_rtt();
    relay_rtt();
    call_reply();
    broker_rtt();
    spawn_reap();
    map_new();
    #[cfg(target_arch = "riscv64")]
    rfence_self();
    // Milestone 74's riscv64 and aarch64 halves and milestone 309's x86_64 half.
    cycles_per_tick();
    coremark_compute();
    null_syscall_el0();
    ctx_switch_el0();
    ipc_rtt_el0();
    #[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
    ipc_thread_scaling();
    #[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
    app_displacement();
    sink_throughput();
    map_el0();
    spawn_el0();
    smp_throughput();
    fs_read();
    fs_throughput();
    fs_walk();
    fs_walk_bound();
    rg_search();

    println!("bench: done");
    // Parked, not exited: the host side saw the marker and tears QEMU down. `wfi`, so a
    // forgotten bench QEMU costs nothing while it waits to be killed (CLAUDE.md's rule).
    crate::arch::halt(crate::arch::HaltReason::measurement_boot());
}

/// **The context switch, round trip.** Two threads yielding to each other; each of our yields
/// is one switch out and (eventually) one switch back in. Ticks/iter ~= two switches.
fn yield_switch() {
    static DONE: AtomicBool = AtomicBool::new(false);

    sched::spawn(|| {
        while !DONE.load(Ordering::Relaxed) {
            sched::yield_now();
        }
    })
    .expect("bench: no peer thread");

    for _ in 0..WARMUP {
        sched::yield_now();
    }
    timed("yield_switch", YIELD_ITERS, || {
        for _ in 0..YIELD_ITERS {
            sched::yield_now();
        }
    });
    DONE.store(true, Ordering::Relaxed);
    sched::yield_now(); // let the peer see the flag and exit
}

/// **What writing the TSS I/O permission bitmap costs on every switch-in** (`x86_64` only;
/// DECISIONS §121's amendment, 2026-08-24, "a micro-benchmark that writes an 8 KiB bitmap into the
/// current CPU's TSS on every switch, timed against a switch that does not").
///
/// The exact same two-thread yield ping-pong as [`yield_switch`] above, plus one
/// [`crate::arch::segments::bench_write_io_bitmap`] call on every resume, in both threads (a real
/// switch-in hook fires regardless of which thread it lands on, so writing only from the timed
/// loop would undercount by half). Reading this bench's `ns/iter` against `yield_switch`'s from the
/// same boot is the whole measurement: same iteration count, same two threads, same scheduler path,
/// one call different. See the module doc on [`bench_write_io_bitmap`][crate::arch::segments::bench_write_io_bitmap]
/// for what the write is (and is not) a stand-in for.
#[cfg(target_arch = "x86_64")]
fn tss_iomap_switch() {
    use core::sync::atomic::AtomicU8;

    static DONE: AtomicBool = AtomicBool::new(false);
    // Folds every write's last byte in, so the optimizer cannot prove the bitmap writes are dead
    // even though nothing ever reads BENCH_IOMAP back through a port instruction.
    static SINK: AtomicU8 = AtomicU8::new(0);

    fn switch_and_write(pattern: &mut u8) {
        sched::yield_now();
        *pattern = pattern.wrapping_add(1);
        SINK.fetch_xor(
            crate::arch::segments::bench_write_io_bitmap(*pattern),
            Ordering::Relaxed,
        );
    }

    sched::spawn(|| {
        let mut pattern = 0u8;
        while !DONE.load(Ordering::Relaxed) {
            switch_and_write(&mut pattern);
        }
    })
    .expect("bench: no peer thread");

    let mut pattern = 0u8;
    for _ in 0..WARMUP {
        switch_and_write(&mut pattern);
    }
    timed("tss_iomap_switch", YIELD_ITERS, || {
        for _ in 0..YIELD_ITERS {
            switch_and_write(&mut pattern);
        }
    });
    DONE.store(true, Ordering::Relaxed);
    sched::yield_now(); // let the peer see the flag and exit
}

/// **What the LAZY TSS I/O-bitmap write costs on a switch that crosses a port holder** (`x86_64`
/// only; milestone 299, the measurement DECISIONS §121's 2026-08-25 refinement asked for). This is
/// **not** a second run of [`tss_iomap_switch`], which prices the naive always-write of the whole
/// 8 KiB bitmap. This prices what milestone 299 actually ships: [`set_port_range_grant`], which checks the
/// incoming grant against what the core already holds and writes only the bits that move.
///
/// The same two-thread ping-pong as [`yield_switch`], but each thread sets its own port grant on
/// resume, and the two disagree: the peer installs `None` (no ports), the main thread installs the
/// COM1 range. So **every** switch is a holder<->non-holder transition, which is the case that takes
/// the write, and reading this against `yield_switch` from the same boot is the cost of the lazy
/// write per switch that crosses a holder. It is the worst case, not the common one: the common one
/// is [`tss_iomap_lazy_nop`] below, where nothing crosses a holder and the write never happens.
///
/// The COM1 range `(0x3F8, 8)` is the real console driver's grant, so the byte count written is the
/// production one. Nothing executes `in`/`out` during the bench, so installing the grant is harmless;
/// the loop restores `None` at the end so the boot's TSS is left denying all ports.
#[cfg(target_arch = "x86_64")]
fn tss_iomap_lazy_switch() {
    const COM1: Option<(u16, u16)> = Some((0x3F8, 8));
    static DONE: AtomicBool = AtomicBool::new(false);

    sched::spawn(|| {
        while !DONE.load(Ordering::Relaxed) {
            sched::yield_now();
            crate::arch::segments::set_port_range_grant(None); // the non-holder side
        }
    })
    .expect("bench: no peer thread");

    for _ in 0..WARMUP {
        sched::yield_now();
        crate::arch::segments::set_port_range_grant(COM1);
    }
    timed("tss_iomap_lazy_switch", YIELD_ITERS, || {
        for _ in 0..YIELD_ITERS {
            sched::yield_now();
            crate::arch::segments::set_port_range_grant(COM1); // the holder side, every iteration a transition
        }
    });
    DONE.store(true, Ordering::Relaxed);
    sched::yield_now(); // let the peer see the flag and exit
    crate::arch::segments::set_port_range_grant(None); // leave the TSS denying all ports
}

/// **What the lazy write costs when nothing holds a port** (`x86_64` only; milestone 299), which is
/// the case nearly every switch on nearly every machine actually takes. Both threads install `None`
/// on resume, so [`set_port_range_grant`] finds the core already holds `None` and returns after one
/// comparison, writing nothing. Read against `yield_switch`, the difference is the whole cost the
/// lazy form imposes on a system where nobody holds a port: a load and a branch. That gap, not
/// `tss_iomap_switch`'s ~2,682 ns, is what x86 pays for having the mechanism present.
#[cfg(target_arch = "x86_64")]
fn tss_iomap_lazy_nop() {
    static DONE: AtomicBool = AtomicBool::new(false);

    sched::spawn(|| {
        while !DONE.load(Ordering::Relaxed) {
            sched::yield_now();
            crate::arch::segments::set_port_range_grant(None);
        }
    })
    .expect("bench: no peer thread");

    for _ in 0..WARMUP {
        sched::yield_now();
        crate::arch::segments::set_port_range_grant(None);
    }
    timed("tss_iomap_lazy_nop", YIELD_ITERS, || {
        for _ in 0..YIELD_ITERS {
            sched::yield_now();
            crate::arch::segments::set_port_range_grant(None);
        }
    });
    DONE.store(true, Ordering::Relaxed);
    sched::yield_now();
}

/// **Synchronous IPC round trip, the classic microkernel number.** A server loops
/// receive-then-send; the client times send-then-receive. One iteration is two rendezvous, two
/// mailbox copies, two wakes, two switches.
fn ipc_rtt() {
    let request = sched::create_rendezvous();
    let reply = sched::create_rendezvous();

    sched::spawn(move || {
        loop {
            let m = sched::ipc_receive(request);
            if m[0] == u64::MAX {
                break; // the client is done with us
            }
            sched::ipc_send(reply, [m[0], 0, 0]);
        }
    })
    .expect("bench: no server");

    for _ in 0..WARMUP {
        sched::ipc_send(request, [1, 0, 0]);
        sched::ipc_receive(reply);
    }
    timed("ipc_rtt", IPC_ITERS, || {
        for _ in 0..IPC_ITERS {
            sched::ipc_send(request, [1, 0, 0]);
            sched::ipc_receive(reply);
        }
    });
    sched::ipc_send(request, [u64::MAX, 0, 0]); // release the server
}

/// **The confined-server tax: a request routed through a server that fans out to a backend.** This
/// is the microkernel architecture's per-request cost that a monolith does not pay, and the topology
/// both real userspace servers use: the FS server CALLs the block server (`client -> fs -> blk -> fs
/// -> client`), `net_stack` CALLs the NIC driver (`client -> net_stack -> driver -> net_stack -> client`). Each
/// iteration here is that two-hop shape: the client sends to a relay, the relay forwards to a backend
/// and waits, the backend replies, the relay replies to the client. Two rendezvous become four, two
/// context switches become four.
///
/// Read it against `ipc_rtt` above (the one-hop client<->server round trip): the **difference** is
/// what one confined intermediary that delegates to a backend costs, the "server tax" a skeptic asks
/// about, isolated and deterministic. It is on the icount baseline for exactly that reason. The real
/// servers' end-to-end numbers are elsewhere and cannot be gated: `fs_read` (below) is device-latency
/// dominated (~200 us/block under HVF swamps this few-hundred-tick tax), and `net_stack`'s path is DHCP- and
/// timer-driven, neither deterministic under `-icount`. So this kernel-side topology bench is how the
/// server tax gets a gated regression number; see notes/benchmarks.md.
fn relay_rtt() {
    let cl_req = sched::create_rendezvous(); // client -> relay
    let cl_reply = sched::create_rendezvous(); // relay -> client
    let bk_req = sched::create_rendezvous(); // relay -> backend
    let bk_reply = sched::create_rendezvous(); // backend -> relay

    // The backend: the leaf service. Receive a request, send a reply, until the sentinel.
    sched::spawn(move || {
        loop {
            let m = sched::ipc_receive(bk_req);
            if m[0] == u64::MAX {
                break;
            }
            sched::ipc_send(bk_reply, [m[0], 0, 0]);
        }
    })
    .expect("bench: no relay backend");

    // The relay: the confined intermediary. For each client request it does a full round trip to the
    // backend, then answers the client. On the sentinel it releases the backend and exits too.
    sched::spawn(move || {
        loop {
            let m = sched::ipc_receive(cl_req);
            if m[0] == u64::MAX {
                sched::ipc_send(bk_req, [u64::MAX, 0, 0]);
                break;
            }
            sched::ipc_send(bk_req, [m[0], 0, 0]);
            let r = sched::ipc_receive(bk_reply);
            sched::ipc_send(cl_reply, [r[0], 0, 0]);
        }
    })
    .expect("bench: no relay");

    for _ in 0..WARMUP {
        sched::ipc_send(cl_req, [1, 0, 0]);
        sched::ipc_receive(cl_reply);
    }
    timed("relay_rtt", RELAY_ITERS, || {
        for _ in 0..RELAY_ITERS {
            sched::ipc_send(cl_req, [1, 0, 0]);
            sched::ipc_receive(cl_reply);
        }
    });
    sched::ipc_send(cl_req, [u64::MAX, 0, 0]); // release the relay, which releases the backend
}

/// **Call/Reply round trip** (milestone 12): the one-endpoint shape real services use. One
/// iteration mints a one-shot Reply capability, rendezvouses, replies through it, consumes it.
fn call_reply() {
    let ep = sched::create_rendezvous();

    sched::spawn(move || {
        loop {
            let m = sched::ipc_receive_cap(ep); // [word, reply_slot, word2]
            if m[0] == u64::MAX {
                break;
            }
            let slot = m[1];
            let crate::cap::Object::Reply(caller) = sched::current_cap(slot)
                .expect("bench: no reply cap")
                .object
            else {
                panic!("bench: RECEIVE_CAP of a CALL did not deliver a Reply capability");
            };
            sched::ipc_reply(caller, [m[0], 0]);
            let _ = sched::delete_current_cap(slot);
        }
    })
    .expect("bench: no call server");

    for _ in 0..WARMUP {
        sched::ipc_call(ep, [1, 0]);
    }
    timed("call_reply", CALL_ITERS, || {
        for _ in 0..CALL_ITERS {
            sched::ipc_call(ep, [1, 0]);
        }
    });
    // Release the server: it is parked in RECEIVE_CAP, and a plain SEND rendezvouses with it all
    // the same (the cap and plain paths share the wait queues), delivering the sentinel.
    sched::ipc_send(ep, [u64::MAX, 0, 0]);
}

/// **What a queue broker costs when both ends are up** (milestone 23, DECISIONS §41).
///
/// Milestone 23's latency ladder has two rungs built, and the rule that governs where each is used
/// is *opt-in per channel, never the default*. This benchmark is why that rule is a rule.
///
/// - **The default rung has no benchmark of its own, because it has no cost of its own.** A client
///   holds a capability to a stable endpoint and whoever is parked in `RECEIVE_CAP` on it answers; a
///   swap changes who that is. No process stands in the data path, so the steady state *is*
///   [`call_reply`] above, instruction for instruction, and the swap adds nothing to it. That is
///   the number to quote for milestone 23's flagship.
/// - **This is the opt-in rung**: `broker` interposed, so a producer never blocks on an absent
///   consumer. Read it against `call_reply`, which is deliberately the same client and the same
///   backend with nothing in between: the **difference** is the whole tax, and it is paid on every
///   request in the steady state, not only during a swap.
///
/// The topology is the CALL/reply idiom rather than `relay_rtt`'s SEND/RECEIVE pairs, because that is
/// what the broker actually speaks: the broker serves its front endpoint with `RECEIVE_CAP`, holds the
/// client's one-shot Reply capability while it CALLs the backend, and answers through it.
fn broker_rtt() {
    let front = sched::create_rendezvous(); // client -> broker
    let back = sched::create_rendezvous(); // broker -> backend

    // The backend: the leaf service, answering through the Reply capability exactly as the direct
    // server in `call_reply` does.
    sched::spawn(move || {
        loop {
            let m = sched::ipc_receive_cap(back);
            if m[0] == u64::MAX {
                break;
            }
            reply_to(m[1], m[0]);
        }
    })
    .expect("bench: no broker backend");

    // The broker: pass-through. Both ends are up, so it buffers nothing; it forwards the request and
    // hands the backend's answer straight back.
    sched::spawn(move || {
        loop {
            let m = sched::ipc_receive_cap(front);
            if m[0] == u64::MAX {
                sched::ipc_send(back, [u64::MAX, 0, 0]);
                break;
            }
            let r = sched::ipc_call(back, [m[0], 0]);
            reply_to(m[1], r[0]);
        }
    })
    .expect("bench: no broker");

    for _ in 0..WARMUP {
        sched::ipc_call(front, [1, 0]);
    }
    timed("broker_rtt", CALL_ITERS, || {
        for _ in 0..CALL_ITERS {
            sched::ipc_call(front, [1, 0]);
        }
    });
    sched::ipc_send(front, [u64::MAX, 0, 0]); // releases the broker, which releases the backend
}

/// Answer through the one-shot Reply capability `RECEIVE_CAP` delivered, and consume it.
fn reply_to(slot: u64, word: u64) {
    let crate::cap::Object::Reply(caller) = sched::current_cap(slot)
        .expect("bench: no reply cap")
        .object
    else {
        panic!("bench: RECEIVE_CAP of a CALL did not deliver a Reply capability");
    };
    sched::ipc_reply(caller, [word, 0]);
    let _ = sched::delete_current_cap(slot);
}

/// **Thread lifecycle, spawn to reap.** Each iteration creates a thread that exits immediately,
/// then yields until the reaper has returned the table to its baseline: TCB pool slot claim and
/// release, stack map and unmap, generational name mint and death.
fn spawn_reap() {
    let baseline = sched::thread_count();
    let one = || {
        sched::spawn(|| {}).expect("bench: spawn failed");
        while sched::thread_count() > baseline {
            sched::yield_now();
        }
    };

    for _ in 0..4 {
        one(); // warmup: the first spawn pays for cold stack VAs
    }
    timed("spawn_reap", SPAWN_ITERS, || {
        for _ in 0..SPAWN_ITERS {
            one();
        }
    });
}

/// **Mapping a fresh page into an address space**: retype from the region, walk, write the
/// leaf. The exec path's inner loop.
fn map_new() {
    static TOTAL: AtomicU64 = AtomicU64::new(0);

    let mut space = crate::user::AddressSpace::new(MAP_ITERS + 8).expect("bench: no address space");
    let base = 0x40_0000u64;

    // The shootdown probe (notes/benchmarks/riscv-map-new-and-the-rfence-probe.md, the 2026-08-15
    // reading). Counted across exactly the
    // timed window, because the claim under test is about what `map_new` itself issues.
    #[cfg(target_arch = "riscv64")]
    let fences_before = crate::arch::remote_fence_count();

    // Preemption is masked across the window. Milestone 541 (a timed window that excludes
    // preemption) is this, and the masking is the whole of it.
    //
    // **The window is ~250 microseconds of guest time and the scheduler tick is 10 ms**
    // (`TICK_HZ` is 100 on all three architectures), so the window covers about 2.5% of a tick
    // period and whether a timer interrupt lands inside it is decided by the phase the boot
    // happened to leave the timer in. That phase moves when unrelated kernel code changes size.
    // On 2026-09-21 a semantically identical rewrite of one expression in `schedule()` moved it,
    // one preemption landed inside this window, and `map_new` read **+26.4%** and failed the 10%
    // tripwire while the map path's object code was byte-identical (notes/benchmarks.md).
    //
    // Masking removes the coin flip rather than diluting it: with the same perturbation applied,
    // `map_new` measures 180,537 either way on x86_64, where it measured 180,604 and 228,356
    // before. The interrupt is not lost, only deferred: it is pending at `restore` below and the
    // preemption is taken there, outside the window, which is why the probe can still report 1.
    //
    // This is a different measurement under `-icount` and the trade is deliberate. All vCPUs
    // share one virtual clock, so a window that masks interrupts is a window that excludes
    // whatever the rest of the machine would have done inside it. That is exactly what is
    // wanted here: the row's job is the cost of mapping a page, not the cost of being
    // descheduled while mapping one. Rows that mean to measure scheduling (`yield_switch`,
    // `spawn_reap`) do not do this and must not.
    //
    // Safe because nothing on this path blocks: `map_new` retypes from a region, walks the
    // table and writes a leaf, taking only spin locks. A row whose body can block cannot use
    // this.
    let preemptions_before = sched::preemptions_here();

    let irqs_were_on = crate::arch::interrupts::disable();
    timed("map_new", MAP_ITERS, || {
        for i in 0..MAP_ITERS {
            let page = space
                .map_new(
                    base + i * page_frames::FRAME_SIZE,
                    paging::Flags::user_data(),
                )
                .expect("bench: map failed");
            // Touch it so the compiler cannot dissolve the loop.
            TOTAL.fetch_add(page[0] as u64, Ordering::Relaxed);
        }
    });
    crate::arch::interrupts::restore(irqs_were_on);

    // **A probe, not a row**, for the reason the fence probe below states: it is a count, and a
    // 10% tolerance over a count is not a tripwire. It is what makes the masking above auditable
    // instead of asserted. Read it as "how many preemptions this window did not pay for": a
    // non-zero value here is the deferred one taken at `restore`, not one inside the window.
    println!(
        "bench-probe: map_new_preemptions {} over {MAP_ITERS} iters",
        sched::preemptions_here() - preemptions_before
    );

    // **Not a `timed` line, and deliberately not on the baseline.** These are counts and a bitmask,
    // not durations: putting them through `timed` would invite `--check` to police them with a 10%
    // tolerance, when the only interesting values are exact (`0` fences, one bit set). They are
    // printed next to `map_new` because that is the benchmark whose reading they settle.
    #[cfg(target_arch = "riscv64")]
    {
        println!(
            "bench-probe: map_new_remote_fences {} over {MAP_ITERS} iters",
            crate::arch::remote_fence_count() - fences_before
        );
        println!(
            "bench-probe: online_harts_mask {:#x} ({} online, this hart {})",
            crate::smp::online_harts_mask(),
            crate::smp::online_count(),
            crate::cpu::id()
        );
    }
    drop(space); // teardown outside the timed window; it is spawn_reap's kind of cost, not map's
}

/// **What one remote RFENCE costs, measured on a single hart** (the calibration that settles the
/// 2026-08-15 reading; notes/benchmarks.md).
///
/// The reading said a `map_new` regression of +370 ticks over 64 iterations was remote RFENCEs
/// fired against an over-reporting mask, and it could not rule out the rival reading, that the
/// delta was the *mask arithmetic itself* being consulted per flush. Those two differ by orders of
/// magnitude, so measuring the RFENCE once decides it. This is the only way to get that number on
/// one hart: **a hart may name itself in an SBI hart mask**, which is a legal call the firmware
/// serves with a local fence, and it exercises the whole `ecall`-into-firmware path that the
/// shootdown pays and a local `sfence.vma` does not.
///
/// It must be single-hart. A two-hart comparison would be the 2026-07-28 mistake again: under
/// `-icount` all harts share one virtual clock, so a second hart's idle `wfi` dumps quantized time
/// into whatever window is open, and the delta would measure interleaving rather than the call.
/// **What one tick of this machine's clock costs in CPU cycles** (milestone 74's riscv64 and aarch64
/// halves, milestone 309's `x86_64` half).
///
/// Every row of a board bench is denominated in ticks of `crate::arch::timer::now()`, a fixed-rate
/// reference counter. The literature this project is compared against is denominated in **cycles**:
/// notes/benchmarks.md converts by hand, against an assumed clock rate, and milestone 74's block is
/// largely about how badly that has gone. One measured ratio converts every existing row at once,
/// which is why this is the harness change rather than a second number on every line.
///
/// **A probe, not a benchmark row.** It is a rate, not a duration, and `--check` policing it with a
/// 10% tolerance would fail on any machine that scales frequency, which is every machine this is
/// interesting on. `bench-probe:` lines are echoed and never enter the baseline; see `map_new`.
///
/// # The numerator and the denominator are different on each architecture, and the line says so
///
/// This is the whole hazard milestone 309 was written to avoid, and it is the kind that cannot be
/// un-published: a rate printed under one name on three machines will be compared across them.
/// A second `cycles_per_tick_means` line is therefore printed beside the number on every
/// architecture, naming both halves of the ratio in that machine's own vocabulary.
///
/// - **`riscv64`.** Core cycles (SBI PMU `CPU_CYCLES`) over ticks of the `time` CSR, a timebase the
///   device tree states: 10 MHz on QEMU `virt`, 4 MHz on radon's JH7110.
/// - **`aarch64`.** `PMCCNTR_EL0` over ticks of `CNTVCT_EL0`, the generic timer at `CNTFRQ_EL0`
///   (62.5 MHz on QEMU `virt`, 19.2 MHz on argon's TX1). **Which exception levels the numerator
///   counts is `PMCCFILTR_EL0`'s, and that value is provisional** until an architect rules on
///   design/roadmap/353-the-aarch64-half-of-74.md, so the meaning line says so and no aarch64
///   figure is a result yet.
/// - **`x86_64`.** Unhalted core cycles (`IA32_PERF_FIXED_CTR1`) over TSC ticks. **The TSC is
///   constant-rate and core cycles are not**, so this ratio moves with frequency scaling and turbo
///   on a machine that does either. That is information rather than noise, and it is exactly why
///   the TSC alone cannot answer this question: `arch::timer::now()` on this architecture *is*
///   `rdtsc`, so a probe built on it would divide one counter by itself and print `1.00`. See
///   `arch::x86_64::pmu`'s header.
///
/// # What this measures under emulation, which is nothing
///
/// QEMU-TCG's `riscv64` `cycle` CSR is an instruction count, and its `x86_64` build models no
/// performance monitoring at all unless asked. The ratio printed on the merge machine is therefore
/// a fact about the emulator and not about any silicon, and the line says so itself rather than
/// leaving a reader to infer it from the milestone. The number is real only on a real core, which
/// for this harness today means the JH7110 (radon) and nothing else it runs on;
/// notes/riscv-cycle-counters.md is the riscv64 procedure and design/roadmap/309-x86-64-core-cycles.md
/// is the `x86_64` one.
///
/// # Why the window is a timed spin
///
/// Both counters are read across the same span, so the ratio does not depend on how long the spin
/// is or on the loop being compiled a particular way. A fixed iteration count would make the ratio
/// depend on the code inside the loop, which is exactly the thing that differs between an emulator
/// and a core.
fn cycles_per_tick() {
    // The core at each end, because on aarch64 each core's counter is its own number (`C` zeroes it
    // at that core's init; `arch::aarch64::pmu`'s BUGS), so a window that migrated would difference
    // two unrelated counters. Cheap and harmless on the other two, where it has never fired.
    let cpu = crate::cpu::id();
    let Some(c0) = crate::arch::pmu::cycles() else {
        // The outcome says *why*, and on a board that is the whole diagnostic: see
        // `arch::riscv64::pmu::CycleCounter` and `arch::x86_64::pmu::CycleCounter`. **Never a
        // number here**, and in particular never a fallback to the reference clock: on x86_64 that
        // clock is the TSC, and a probe that degraded to it would print an exact 1.00 that reads
        // as a measurement.
        println!(
            "bench-probe: cycles_per_tick unavailable ({:?})",
            crate::arch::pmu::outcome()
        );
        return;
    };

    let t0 = crate::arch::timer::now();
    while crate::arch::timer::now() - t0 < cycle_probe_window_ticks() {
        core::hint::spin_loop();
    }
    let t1 = crate::arch::timer::now();
    let c1 = crate::arch::pmu::cycles().expect("the counter did not vanish mid-window");
    if crate::cpu::id() != cpu {
        println!(
            "bench-probe: cycles_per_tick unavailable (the window moved cores; per-core counters)"
        );
        return;
    }

    let ticks = t1 - t0;
    let cycles = cycle_probe_delta(c0, c1);
    // Two decimal places by integer arithmetic: this is a `no_std` kernel and the ratio is under
    // one on any emulator and in the hundreds on real silicon, so the fraction carries real
    // information at both ends.
    let hundredths = cycles.saturating_mul(100).checked_div(ticks).unwrap_or(0);
    println!(
        "bench-probe: cycles_per_tick {}.{:02} ({cycles} cycles over {ticks} ticks at cntfrq {})",
        hundredths / 100,
        hundredths % 100,
        crate::arch::timer::frequency(),
    );
    println!("bench-probe: cycles_per_tick_means {CYCLE_PROBE_MEANING}");
}

/// **What the ratio above is a ratio of**, in this machine's own vocabulary. Printed rather than
/// left to a note, because the number leaves the machine and the note does not travel with it.
#[cfg(target_arch = "riscv64")]
const CYCLE_PROBE_MEANING: &str = "core cycles (SBI PMU CPU_CYCLES) per tick of the `time` CSR, a fixed-rate timebase the device \
     tree states";

/// The aarch64 twin. **The filter clause is the one a reader must not drop**: until an architect
/// rules on `PMCCFILTR_EL0`, what this counts is a provisional choice, and the line that leaves the
/// machine carries that rather than a note that does not.
#[cfg(target_arch = "aarch64")]
const CYCLE_PROBE_MEANING: &str = "cycles (PMCCNTR_EL0, PMCCFILTR_EL0 PROVISIONAL: EL0+EL1 counted, EL2 not) per tick of \
     CNTVCT_EL0, the generic timer at CNTFRQ_EL0";

/// The `x86_64` twin, and the difference a cross-architecture reader has to know: the denominator
/// here is the TSC, which is constant-rate, so the ratio moves with the core's frequency where
/// riscv64's does not.
#[cfg(target_arch = "x86_64")]
const CYCLE_PROBE_MEANING: &str = "unhalted core cycles (IA32_PERF_FIXED_CTR1) per TSC tick; the TSC is constant-rate and core \
     cycles are not, so this ratio moves with frequency scaling and turbo";

/// Ticks to spin for. At QEMU's 10 MHz `time` CSR this is 10 ms, long enough that the two counter
/// reads at each end are noise and short enough not to stretch a bench run.
///
/// **A constant here and a computed value on `x86_64`**, because RISC-V's timebase is stated by the
/// machine and x86's is measured by this kernel: 100,000 ticks is 10 ms on QEMU and 25 ms on
/// radon's 4 MHz JH7110, both fine. It stays a constant for the same reason `SCALE_MAX_PAIRS` did:
/// milestone 74's block records a reading of `100002 ticks`, and changing the window would make
/// every future radon run incomparable with the one on record.
#[cfg(target_arch = "riscv64")]
fn cycle_probe_window_ticks() -> u64 {
    100_000
}

/// Ten milliseconds of generic timer, whatever `CNTFRQ_EL0` says: 62.5 MHz on QEMU and 19.2 MHz on
/// argon, so a constant would be a different window on each.
#[cfg(target_arch = "aarch64")]
fn cycle_probe_window_ticks() -> u64 {
    crate::arch::timer::frequency() / 100
}

/// Ten milliseconds of TSC, whatever this part's TSC rate turned out to be. A constant would be
/// microseconds on one machine and a visible pause on another, because the TSC's rate is a property
/// of the part rather than an architected number (see `arch::x86_64::timer`'s header on why this
/// kernel has to measure it against the PIT at all).
#[cfg(target_arch = "x86_64")]
fn cycle_probe_window_ticks() -> u64 {
    crate::arch::timer::frequency() / 100
}

/// The difference between two counter reads. The SBI PMU counter this kernel accepts is 64 bits
/// wide (`arch::riscv64::pmu` records the width and QEMU's is 64), and `PMCCNTR_EL0` is 64 bits by
/// architecture, so a plain wrapping subtraction is the whole of it on both.
#[cfg(any(target_arch = "riscv64", target_arch = "aarch64"))]
fn cycle_probe_delta(first: u64, second: u64) -> u64 {
    second.wrapping_sub(first)
}

/// The `x86_64` twin, which cannot be a plain subtraction: the architectural fixed counters are
/// commonly 48 bits, so a wrap mid-window would otherwise produce a difference near 2^64 and print
/// a preposterous ratio rather than an error. `arch::x86_64::pmu` knows the width `CPUID` reported.
#[cfg(target_arch = "x86_64")]
fn cycle_probe_delta(first: u64, second: u64) -> u64 {
    crate::arch::pmu::elapsed_cycles(first, second)
}

#[cfg(target_arch = "riscv64")]
fn rfence_self() {
    const RFENCE_ITERS: u64 = 512;
    // Any mapped kernel address: the firmware fences a range, and which range it is does not change
    // the cost of getting there, which is the whole of what this measures.
    static ANCHOR: AtomicU64 = AtomicU64::new(0);
    let va = &raw const ANCHOR as usize;
    let me = 1usize << crate::cpu::id();

    for _ in 0..WARMUP {
        crate::arch::sbi_remote_sfence_vma(me, va, page_frames::FRAME_SIZE as usize);
    }
    timed("rfence_self", RFENCE_ITERS, || {
        for _ in 0..RFENCE_ITERS {
            crate::arch::sbi_remote_sfence_vma(me, va, page_frames::FRAME_SIZE as usize);
        }
    });
}

// Roles for the `os_primitives_benchmarker` EL0 program (must match fixtures/src/os_primitives_benchmarker.rs). One binary, one micro-
// measurement per role, chosen through `START`'s `arg0`.
const EL_NULL_SYSCALL: u64 = 0;
const EL_YIELDER: u64 = 1;
const EL_CTX_SWITCH: u64 = 2;
const EL_IPC_SERVER: u64 = 3;
const EL_IPC_CLIENT: u64 = 4;
const EL_MAP: u64 = 5;
const EL_SPAWN: u64 = 6;
const EL_SINK_PRODUCER: u64 = 7;
const EL_SINK_CONSUMER: u64 = 8;

/// The spawner's untyped budget, in pages. **Small on purpose:** each child is split, run, and
/// destroyed strictly LIFO, so its pages return to this budget (DECISIONS §16), and only one child
/// lives at a time. The budget need only fund one child (`CHILD_PAGES`) plus the shared code frame
/// and the scratch page tables, not one per iteration. That 64 funds a 100-iteration loop (which
/// would need >1000 pages without return-to-parent) is itself the proof LIFO reclaim works.
const SPAWN_EL0_BUDGET: u64 = 64;

/// Warmup + timed map counts the bench boot must provision the target region for. `MAP_EL0_ITERS`
/// **must equal** `os_primitives_benchmarker`'s `MAP_ITERS` and `MAP_EL0_WARMUP` its `MAP_WARMUP`: the region is sized for
/// their sum plus page-table and record overhead, and if `os_primitives_benchmarker` asks for more maps than the region
/// funds, the surplus fail (a cheap error return, not a real map) and skew the number. Kept here
/// rather than shared because the two crates have no common header, the way EL_* mirror ROLE_*.
const MAP_EL0_ITERS: u64 = 500;
const MAP_EL0_WARMUP: u64 = 8;
const MAP_EL0_OVERHEAD: u64 = 32;

/// Spawn the `os_primitives_benchmarker` EL0 program in a given role, granting it `report` (slot 0) to answer on.
/// `false` if there is no `os_primitives_benchmarker` in the initrd (the bench boot then skips that line).
fn spawn_os_primitives_benchmarker(role: u64, report: sched::RendezvousId) -> bool {
    let Some(image) = crate::trust::require_program("os_primitives_benchmarker") else {
        return false;
    };
    sched::spawn(move || {
        crate::user::run(
            image,
            crate::user::Spawn {
                arg0: role,
                arg1: 0,
                arg2: 0,
                grants: &[rendezvous_cap(report, Rights::WRITE)],
                maps: &[],
            },
        )
    })
    .expect("bench: could not spawn os_primitives_benchmarker");
    true
}

/// **Null syscall latency, measured from EL0 (the primitive suite).** The `bench:` lines above are
/// kernel-internal, no trap. This one is what lmbench measures: the bench boot spawns the `os_primitives_benchmarker`
/// EL0 program, which self-times a loop of the cheapest `svc` and reports `[ticks, iters]`; we print
/// it in the same format. The gap between this and a hypothetical kernel-side null syscall is roughly
/// the EL0<->EL1 boundary cost, which is the whole point of measuring here. See `fixtures/src/os_primitives_benchmarker.rs`.
fn null_syscall_el0() {
    let report = sched::create_rendezvous();
    if !spawn_os_primitives_benchmarker(EL_NULL_SYSCALL, report) {
        println!("bench: null_syscall skipped (no os_primitives_benchmarker in the initrd)");
        return;
    }
    let [ticks, iters, ..] = sched::ipc_receive(report);
    println!("bench: null_syscall {ticks} {iters}");
}

/// **Context switch latency, measured from EL0 (the primitive suite).** lmbench's `lat_ctx`. The
/// bench boot spawns a *yielder* peer and a *timer*, two separate EL0 processes; the timer self-times
/// a loop of `SYS_YIELD`, each handing the CPU to the peer and back, two switches per iteration, each
/// an address-space change. With the boot thread blocked here on the report and only those two ready,
/// the alternation is clean. See `fixtures/src/os_primitives_benchmarker.rs`.
fn ctx_switch_el0() {
    let report = sched::create_rendezvous();
    // The peer first, so the timer always has something to switch to. It shares the report endpoint
    // (it never sends on it); the spawn shape stays uniform.
    if !spawn_os_primitives_benchmarker(EL_YIELDER, report) {
        println!("bench: ctx_switch skipped (no os_primitives_benchmarker in the initrd)");
        return;
    }
    if !spawn_os_primitives_benchmarker(EL_CTX_SWITCH, report) {
        return;
    }
    let [ticks, iters, ..] = sched::ipc_receive(report);
    println!("bench: ctx_switch {ticks} {iters}");
}

/// **IPC round-trip latency, measured from EL0 (the primitive suite).** lmbench's `lat_pipe`. Two
/// EL0 processes and two endpoints: a server (RECEIVE request, SEND reply) and a client that self-times
/// a loop of SEND-then-RECEIVE and reports. The server is spawned first so a request always meets a
/// waiting receiver. Grants differ per role, so the spawns are inline rather than via `spawn_os_primitives_benchmarker`.
fn ipc_rtt_el0() {
    let Some(image) = crate::trust::require_program("os_primitives_benchmarker") else {
        println!("bench: ipc_rtt skipped (no os_primitives_benchmarker in the initrd)");
        return;
    };
    let request = sched::create_rendezvous();
    let reply = sched::create_rendezvous();
    let report = sched::create_rendezvous();

    sched::spawn(move || {
        crate::user::run(
            image,
            crate::user::Spawn {
                arg0: EL_IPC_SERVER,
                arg1: 0,
                arg2: 0,
                grants: &[
                    rendezvous_cap(request, Rights::READ), // slot 0: RECEIVE requests
                    rendezvous_cap(reply, Rights::WRITE),  // slot 1: SEND replies
                ],
                maps: &[],
            },
        )
    })
    .expect("bench: could not spawn the ipc server");

    sched::spawn(move || {
        crate::user::run(
            image,
            crate::user::Spawn {
                arg0: EL_IPC_CLIENT,
                arg1: 0,
                arg2: 0,
                grants: &[
                    rendezvous_cap(report, Rights::WRITE), // slot 0: report the result
                    rendezvous_cap(request, Rights::WRITE), // slot 1: SEND requests
                    rendezvous_cap(reply, Rights::READ),   // slot 2: RECEIVE replies
                ],
                maps: &[],
            },
        )
    })
    .expect("bench: could not spawn the ipc client");

    let [ticks, iters, ..] = sched::ipc_receive(report);
    // Distinct from the kernel-side `ipc_rtt` above: this one crosses the EL0<->EL1 boundary on every
    // send and receive, which is the whole point (comparable to lmbench). The gap between them is roughly
    // the trap cost of the four svcs per round trip.
    println!("bench: ipc_rtt_el0 {ticks} {iters}");
}

// --- Milestone 134, tier A: E1 (thread scaling), E4 (application displacement) ---
//
// design/roadmap/134-the-measurements-that-decide.md. Both decide DECISIONS §96 (process kernel
// or event kernel) from a different angle: E1 asks whether the KERNEL's own IPC path gets cache-
// cold as more threads cycle through it; E4 asks what an IPC-heavy kernel costs an unrelated
// APPLICATION's working set, which is Liedtke's actual claim and the one no kernel-side number can
// see. Both need a real cache (no TCG models one) and one hart (so everything contends for the
// SAME core's cache, the effect under test); see `real_single_hart_or_skip`.
//
// **Both architectures since 2026-09-04, and the second one is the point.** These were aarch64-only
// because this tree has no riscv64 *accelerator* with a real cache, which is true and was never the
// same statement as "no riscv64 machine with a real cache": radon is a StarFive VisionFive 2 whose
// four U74s have 32 KB L1i and 32 KB L1d each, and that is the machine E1's prediction was built
// against and the machine E3's own design says a negative result needs. What was missing was a way
// to RUN them there, which is a board card (`script/board-image --bench`) plus a single-hart boot
// (the `single_hart` feature). Under QEMU nothing changes: the riscv64 runs are TCG, so both still
// self-skip, by the same one-value counter-frequency test the aarch64 half already used. See
// notes/footprint-perturbation.md for the procedure and design/roadmap/134-the-measurements-that-decide.md.

/// QEMU's `virt` machine fixes `CNTFRQ_EL0` at 62.5 MHz under TCG, with or without `-icount`
/// (measured on this tree's pinned QEMU: `cargo xtask bench` and `cargo xtask bench --check` both
/// read it back from the printed `ns/iter` column); a real core reports its own frequency (24 MHz
/// measured on the Apple Silicon dev machine under HVF). Nothing else distinguishes the two
/// accelerators inside the guest: `NIFE_ACCEL` is a host-side environment variable for the QEMU
/// launch, never passed into the boot (see xtask's `bench` and `helpers/qemu-runner-aarch64.sh`).
#[cfg(target_arch = "aarch64")]
const TCG_VIRT_CNTFRQ_HZ: u64 = 62_500_000;

/// The riscv64 twin, and the reason this experiment can reach a small-cache board at all.
///
/// RISC-V has no `CNTFRQ_EL0`; the counter rate is a device-tree property
/// (`/cpus/timebase-frequency`, read by `arch::riscv64::timer::init_frequency`). QEMU `virt`
/// states **10 MHz**; the JH7110 on radon states **4 MHz** (notes/visionfive2.md, "Timebase is
/// 4 MHz"). So the same one-value test that separates HVF from TCG on aarch64 separates the board
/// from TCG here, and it fails safe in the direction that matters: an unknown riscv64 machine
/// reads as real, and the only value that self-skips is the emulator's own.
///
/// **What it does not distinguish**, and this is the honest limitation: it says "not QEMU `virt`",
/// not "has a 32 KB L1". A second riscv64 machine with a different timebase and a large cache
/// would run this and be believed. The operator's runbook (notes/footprint-perturbation.md) is what records
/// which machine a number came from; nothing in the kernel can.
#[cfg(target_arch = "riscv64")]
const TCG_VIRT_CNTFRQ_HZ: u64 = 10_000_000;

/// Shared precondition for E1 and E4: a real core (so there is a cache to model at all) and a
/// single hart (so every thread contends for the SAME core's cache, which is the effect under
/// test; spreading pairs across cores would dilute it, the opposite reason `smp_throughput`
/// requires more than one). Prints why and returns `false` when either does not hold, the same
/// self-skip shape every `--real`-only bench in this file already uses.
///
/// **The single-hart half is why a board card needs `single_hart`** (kernel/Cargo.toml). radon
/// seats four U74s and `smp::bring_up_secondaries` starts all of them, so a plain `board,bench`
/// card satisfies the "real core" half and fails this one, and both E1 and E4 would print a skip
/// line on the machine they were designed for. `single_hart` parks the secondaries at bring-up;
/// the cost is that `smp_throughput` self-skips on the same card, which is why the runbook takes
/// two cards rather than one. See notes/footprint-perturbation.md.
#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
fn real_single_hart_or_skip(name: &str) -> bool {
    let cpus = crate::smp::online_count();
    if cpus != 1 {
        println!(
            "bench: {name} skipped (needs a single hart to isolate one core's cache; this boot has {cpus})"
        );
        return false;
    }
    let freq = crate::arch::timer::frequency();
    if freq == TCG_VIRT_CNTFRQ_HZ {
        println!(
            "bench: {name} skipped (QEMU virt detected via counter frequency {freq} Hz; TCG \
             models no cache, see design/roadmap/134-the-measurements-that-decide.md)"
        );
        return false;
    }
    true
}

/// Pair counts for [`ipc_thread_scaling`] (E1): 1 to `SCALE_MAX_PAIRS` pairs, doubling.
///
/// **The roadmap's own words say "N from 2 to 128"; this sweeps up to 2*`SCALE_MAX_PAIRS` = 96
/// threads, not 128.** When this was written `sched::MAX_THREADS` was 128 for the WHOLE system
/// (DECISIONS §96), so 128 *pairs* (256 threads) could not exist at all, and even 64 pairs (128
/// threads) would have left no headroom for the bench boot's own thread. Read "N" as the
/// roadmap's own prediction text does, "distinct threads cycling through IPC" rather than pairs
/// ("somewhere between 16 and 32 threads"): 96 threads is 3x past the predicted knee.
///
/// **`MAX_THREADS` is 256 as of 2026-08-27 and this sweep did not move with it, deliberately.**
/// The cap stopped being the binding reason, but the sweep is a published measurement: changing
/// where it stops would make every future run incomparable with every recorded one, for a point
/// well past the knee this experiment exists to find. The number to change if the knee ever moves
/// is this one, and it should be changed for that reason rather than because more room appeared.
#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
const SCALE_MAX_PAIRS: usize = 48;
#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
const SCALE_PAIRS: &[usize] = &[1, 2, 4, 8, 16, 32, SCALE_MAX_PAIRS];

/// **E1: IPC round-trip latency against thread count.** Kernel stacks are `STACK_SLOT_SPAN`
/// (28 KiB) apart, so each IPC that switches to a *different* thread touches a different stack; if
/// enough distinct threads cycle through the kernel, those stacks stop fitting in L1d and the
/// round trip goes cache-cold. The prediction: a knee somewhere in the low tens of threads,
/// against the smallest L1d this project targets (the `SiFive` U74's 32 KB).
///
/// Reuses [`tp_batch`]/[`tp_best`] (`smp_throughput`'s own machinery) at each pair count in
/// [`SCALE_PAIRS`]: N independent client/server pairs, released together behind a barrier, timed
/// wall-clock to completion, minimum of [`TP_REPEAT`] batches kept. That already IS "N pairs in
/// rotation": pinned to one hart (`real_single_hart_or_skip`), the scheduler round-robins the
/// ready pairs, so returning to a given pair's stack means other stacks were touched in between,
/// more of them as N grows. A flat line across the sweep says the process-kernel penalty is
/// invisible on this machine's cache; a knee reproduces Warton's effect on this kernel and gives
/// its magnitude.
#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
fn ipc_thread_scaling() {
    if !real_single_hart_or_skip("ipc_thread_scaling") {
        return;
    }

    let mut req = [0u64; SCALE_MAX_PAIRS];
    let mut reply = [0u64; SCALE_MAX_PAIRS];
    for i in 0..SCALE_MAX_PAIRS {
        req[i] = sched::create_rendezvous();
        reply[i] = sched::create_rendezvous();
    }
    let done = sched::create_rendezvous();

    for &pairs in SCALE_PAIRS {
        let ticks = tp_best(&req[..pairs], &reply[..pairs], done, pairs);
        let threads = pairs * 2;
        println!(
            "bench: ipc_scale_{threads} {ticks} {}",
            pairs as u64 * TP_RTT
        );
    }
}

/// Working-set sizes for [`app_displacement`] (E4), in KiB: below, at, and above a typical L1d
/// (32 KB) and into L2 territory, so the sweep can show whether displacement tracks a specific
/// cache level rather than growing uniformly.
#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
const APPDISP_WORKINGSET_KIB: &[usize] = &[4, 16, 32, 64, 128];
/// Bytes per working-set word; the workload reads and writes `u64`s.
#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
const APPDISP_WORD_BYTES: usize = 8;
/// The largest working set above, in words: the static scratch buffer's size.
#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
const APPDISP_MAX_WORDS: usize = 128 * 1024 / APPDISP_WORD_BYTES;
/// Roughly this many word-touches per measurement, regardless of working-set size, so every point
/// in the sweep takes about the same wall time: `passes = TARGET / words`.
#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
const APPDISP_TARGET_TOUCHES: u64 = 8_000_000;
/// Background IPC pairs during the "with traffic" batch: 16 threads, a realistic concurrent load
/// (comfortably inside [`SCALE_PAIRS`]'s own sweep, so its curve says what this load costs the
/// kernel's own IPC path; this bench asks what the same load costs an unrelated application).
#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
const APPDISP_IPC_PAIRS: usize = 8;
/// Background IPC pairs for the second, higher-load condition: [`SCALE_MAX_PAIRS`] itself, 96
/// threads, the same pair count E1's own sweep tops out at and "3x past the predicted knee" by
/// that sweep's own doc comment. The register of milestone 134 (the register of measures) named
/// this the missing half of E4, in a BUGS entry closed on 2026-08-23 that now lives in
/// `notes/register-of-measures/cache-experiments.md`: the 8-pair condition sits inside E1's flat region, so
/// a null result there is expected from E1's own curve rather than independent evidence, and a
/// load nearer or past the knee is the stronger version of the experiment.
#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
const APPDISP_IPC_PAIRS_HIGH: usize = SCALE_MAX_PAIRS;

/// Interior-mutable scratch the workload reads and writes. One `Racy<T>` per benchmark file:
/// `sched.rs`'s corruption canary (`memory_corruption_canary_gate::arm`) defines the same idiom for the same reason,
/// a scratch region one thread at a time uses, serialized by the caller rather than by a lock that
/// would then be part of the very cost this benchmark measures the absence of.
#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
struct Racy<T>(core::cell::UnsafeCell<T>);
// SAFETY: access is serialized by `appdisp_batch`: the workload thread is the only one ever handed
// a reference into this buffer, and one batch's workload always finishes (and its reference drops)
// before the next batch's does (see `appdisp_batch`'s reap-to-baseline wait, which runs after
// `appdisp_workload` returns).
#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
unsafe impl<T> Sync for Racy<T> {}

#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
static APPDISP_BUFFER: Racy<[u64; APPDISP_MAX_WORDS]> =
    Racy(core::cell::UnsafeCell::new([0; APPDISP_MAX_WORDS]));

#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
static APPDISP_STOP: AtomicBool = AtomicBool::new(false);

/// One background IPC pair for the "with traffic" batch: a server that echoes until the sentinel,
/// and a client that sends-then-receives until [`APPDISP_STOP`], then releases the server. Same
/// sentinel idiom as [`ipc_rtt`], parameterized on a stop flag instead of an iteration count
/// because this pair must outlive an unknown number of the workload's passes.
#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
fn appdisp_background_pair(rq: sched::RendezvousId, rp: sched::RendezvousId) {
    sched::spawn(move || {
        loop {
            let m = sched::ipc_receive(rq);
            if m[0] == u64::MAX {
                break;
            }
            sched::ipc_send(rp, [m[0], 0, 0]);
        }
    })
    .expect("bench: no appdisp background server");
    sched::spawn(move || {
        while !APPDISP_STOP.load(Ordering::Relaxed) {
            sched::ipc_send(rq, [1, 0, 0]);
            sched::ipc_receive(rp);
        }
        sched::ipc_send(rq, [u64::MAX, 0, 0]);
    })
    .expect("bench: no appdisp background client");
}

/// **The application**: read-modify-write `words` entries of [`APPDISP_BUFFER`], `passes` times,
/// self-timed. Yields every so often (about 64 times over the whole run, regardless of `passes`)
/// so a cooperative scheduler interleaves it with any background IPC pairs: without a voluntary
/// yield, a compute-only thread that never traps could run to completion before anything else
/// gets to, which would make "concurrent" IPC traffic not actually concurrent. The same yields run
/// in the solo condition too, so the two conditions differ only in whether anything else is ready
/// to run, not in the workload's own control flow.
#[inline(never)]
#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
fn appdisp_workload(words: usize, passes: u64) -> u64 {
    // SAFETY: the caller (`appdisp_batch`) guarantees this thread is the only one with a reference
    // into the buffer for the duration of this call; see `Racy`'s doc.
    let whole: &mut [u64; APPDISP_MAX_WORDS] = unsafe { &mut *APPDISP_BUFFER.0.get() };
    let buf = &mut whole[..words];
    let yield_every = (passes / 64).max(1);
    let mut sum: u64 = 0;
    let t0 = crate::arch::timer::now();
    for p in 0..passes {
        for (i, word) in buf.iter_mut().enumerate() {
            sum = sum.wrapping_add(*word).wrapping_add(i as u64);
            *word = sum;
        }
        if p % yield_every == 0 {
            sched::yield_now();
        }
    }
    let ticks = crate::arch::timer::now() - t0;
    core::hint::black_box(sum);
    ticks
}

/// One measurement: as many background IPC pairs as `req`/`reply` have slots for (zero for the
/// solo condition), running for exactly as long as [`appdisp_workload`] takes, then released.
/// Returns the workload's own ticks, which is the number E4 cares about: what the APPLICATION's
/// throughput did, not what the kernel's did.
#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
fn appdisp_batch(
    words: usize,
    passes: u64,
    req: &[sched::RendezvousId],
    reply: &[sched::RendezvousId],
) -> u64 {
    APPDISP_STOP.store(false, Ordering::SeqCst);
    let base = sched::thread_count();
    for i in 0..req.len() {
        appdisp_background_pair(req[i], reply[i]);
    }
    // Let the background pairs reach their first blocking RECEIVE before the clock starts, so their
    // cold-start cost lands outside the timed window, same reason every other batch here warms up.
    for _ in 0..8 {
        sched::yield_now();
    }
    let ticks = appdisp_workload(words, passes);
    APPDISP_STOP.store(true, Ordering::SeqCst);
    while sched::thread_count() > base {
        sched::yield_now();
    }
    ticks
}

/// Batches per measurement point; the minimum ticks is kept. Same methodology as [`tp_best`] and
/// the same reason ([`tp_best`]'s doc, `smp_throughput`'s methodology note), turned up from
/// [`TP_REPEAT`]'s 4 to 5: a first unrepeated run of this bench showed swings of 2 to 3x between
/// nominally identical conditions (a host-scheduling artifact of a batch running for a much longer
/// window than a ping-pong pipeline does, so it has more chances to catch a host preemption), which
/// is exactly the least-contended-sample problem `tp_best` exists to solve, just louder here.
#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
const APPDISP_REPEAT: usize = 5;

/// Minimum ticks over [`APPDISP_REPEAT`] batches: the least host-contended sample.
#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
fn appdisp_best(
    words: usize,
    passes: u64,
    req: &[sched::RendezvousId],
    reply: &[sched::RendezvousId],
) -> u64 {
    let mut best = u64::MAX;
    for _ in 0..APPDISP_REPEAT {
        best = best.min(appdisp_batch(words, passes, req, reply));
    }
    best
}

/// **E4: application working-set displacement, the Liedtke measurement proper.** Everything else
/// in this file measures the KERNEL's cost; this measures the cost the kernel imposes on an
/// APPLICATION after the syscall returns, which is what Liedtke's argument was actually about and
/// which no kernel-side benchmark can see by construction.
///
/// The "application" is [`appdisp_workload`]: a tunable working set it reads and writes
/// repeatedly, timed alone and then timed again with [`APPDISP_IPC_PAIRS`] background IPC pairs
/// running concurrently on the same core, and a third time with [`APPDISP_IPC_PAIRS_HIGH`], a load
/// near/at E1's own knee. The throughput lost between conditions, swept over
/// [`APPDISP_WORKINGSET_KIB`], is the number: how much an IPC-heavy kernel costs an unrelated
/// application's cache, not how much it costs the kernel's own IPC path.
///
/// The high-load condition is the register's own follow-up, from a BUGS entry that said a load
/// nearer E1's knee was the stronger experiment and had not been taken (closed 2026-08-23; now in
/// `notes/register-of-measures/cache-experiments.md`). The original 8-pair run (16 threads) sits
/// inside E1's flat region, where E1 found no cost on the dev Mac, so a null result there was
/// expected rather than informative. 48 pairs (96 threads) puts the background load past E1's
/// knee: on the dev Mac E1 was flat to 16 threads and rose 8-11% by 64-96, and on radon
/// (2026-09-04) the knee is at 16 threads, 1.68x. That is where this experiment can distinguish
/// "the kernel's own path got slower" from "the application's cache got evicted". The register's
/// "not a knee" is about E4's own result at this load (a small, consistent 2-9% displacement on
/// the dev Mac), not about E1.
#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
fn app_displacement() {
    if !real_single_hart_or_skip("app_displacement") {
        return;
    }

    let mut req = [0u64; APPDISP_IPC_PAIRS_HIGH];
    let mut reply = [0u64; APPDISP_IPC_PAIRS_HIGH];
    for i in 0..APPDISP_IPC_PAIRS_HIGH {
        req[i] = sched::create_rendezvous();
        reply[i] = sched::create_rendezvous();
    }

    for &kib in APPDISP_WORKINGSET_KIB {
        let words = kib * 1024 / APPDISP_WORD_BYTES;
        let passes = (APPDISP_TARGET_TOUCHES / words as u64).max(1);
        let solo = appdisp_best(words, passes, &[], &[]);
        let with_ipc = appdisp_best(
            words,
            passes,
            &req[..APPDISP_IPC_PAIRS],
            &reply[..APPDISP_IPC_PAIRS],
        );
        let with_ipc_high = appdisp_best(words, passes, &req, &reply);
        println!("bench: appdisp_{kib}k_solo {solo} {passes}");
        println!("bench: appdisp_{kib}k_ipc {with_ipc} {passes}");
        println!("bench: appdisp_{kib}k_ipc96 {with_ipc_high} {passes}");
        let lost_pct = |busy: u64| {
            if solo > 0 {
                ((busy as i64 - solo as i64) * 100) / solo as i64
            } else {
                0
            }
        };
        println!(
            "bench-probe: appdisp_{kib}k_throughput_lost_pct {}",
            lost_pct(with_ipc)
        );
        println!(
            "bench-probe: appdisp_{kib}k_highload_throughput_lost_pct {}",
            lost_pct(with_ipc_high)
        );
    }
}

/// **`a | b` throughput, measured from EL0** (milestone 50, notes/pipes.md).
///
/// Two EL0 processes and one endpoint, which is literally what a pipeline is here: the shell mints an
/// endpoint, gives the left stage `WRITE` and the right stage `READ`, and there is no object in
/// between. The producer packs sixteen bytes into a sink message and `SEND`s; the consumer `RECEIVE`s
/// and self-times. The reported pair is `[ticks, bytes]` rather than `[ticks, iters]`, because bytes
/// is the number a Unix pipe can be compared against, which is the whole reason this exists: the
/// design note said measure the lockstep before deciding anything about buffering.
///
/// The comparison is `bench/host/pipe_throughput.rs`, and it deliberately measures a Unix pipe
/// **twice**: once with the same sixteen-byte writes, which isolates the cost of having no buffer,
/// and once with the 64 KiB writes a real Unix program would use, which is what Unix actually gets.
/// Only the first pair is apples to apples.
fn sink_throughput() {
    let Some(image) = crate::trust::require_program("os_primitives_benchmarker") else {
        println!("bench: sink_throughput skipped (no os_primitives_benchmarker in the initrd)");
        return;
    };
    let pipe = sched::create_rendezvous();
    let report = sched::create_rendezvous();

    // The producer first, so the consumer's first `RECEIVE` meets a waiting sender rather than the
    // other way round. Either order works (a rendezvous blocks whichever side arrives first) and
    // this one keeps the warmup honest: the consumer's timed loop starts with the pipe already hot.
    sched::spawn(move || {
        crate::user::run(
            image,
            crate::user::Spawn {
                arg0: EL_SINK_PRODUCER,
                arg1: 0,
                arg2: 0,
                // **`WRITE` and nothing else**, which is exactly what the shell delegates to the
                // left of a `|`. It cannot read back up its own output.
                grants: &[rendezvous_cap(pipe, Rights::WRITE)],
                maps: &[],
            },
        )
    })
    .expect("bench: could not spawn the sink producer");

    sched::spawn(move || {
        crate::user::run(
            image,
            crate::user::Spawn {
                arg0: EL_SINK_CONSUMER,
                arg1: 0,
                arg2: 0,
                grants: &[
                    rendezvous_cap(report, Rights::WRITE), // slot 0: report the result
                    rendezvous_cap(pipe, Rights::READ),    // slot 1: the pipe's read end
                ],
                maps: &[],
            },
        )
    })
    .expect("bench: could not spawn the sink consumer");

    let [ticks, bytes, ..] = sched::ipc_receive(report);
    println!("bench: sink_throughput {ticks} {bytes}");
}

/// **Map latency, measured from EL0 (the primitive suite).** lmbench's `lat_mmap`. Unlike the three
/// above, the map path *consumes* resources per call, so the bench boot builds the target here rather
/// than granting an endpoint: a fresh registry address space (its own untyped region as budget) and a
/// single frame. `os_primitives_benchmarker` (`EL_MAP`) is granted a WRITE cap on that space and a READ cap on the frame,
/// then times a loop of `invoke(address space, MAP_INTO, va_i, frame, MAP_RO)`, aliasing the one frame at a
/// fresh VA each iteration. The target is a separate space, not `os_primitives_benchmarker`'s own (a run()-adopted space
/// is not in the registry `MAP_INTO` resolves), which is immaterial: the map path's cost is the same
/// whoever owns the space. See `fixtures/src/os_primitives_benchmarker.rs`.
fn map_el0() {
    let Some(image) = crate::trust::require_program("os_primitives_benchmarker") else {
        println!("bench: map_el0 skipped (no os_primitives_benchmarker in the initrd)");
        return;
    };
    // The target space, backed by its own region. The region pays for the root, the intermediate
    // tables, and the mapping-record log pages; the leaves are aliases of one frame, so they cost it
    // nothing. Sized for the warmup plus timed maps plus that overhead.
    let Some(region) =
        crate::memory_region::create(MAP_EL0_WARMUP + MAP_EL0_ITERS + MAP_EL0_OVERHEAD)
    else {
        println!("bench: map_el0 skipped (no region)");
        return;
    };
    let Some(name) = crate::user::user_address_space_create(region) else {
        println!("bench: map_el0 skipped (no address space)");
        return;
    };
    // One frame to alias-map, from its own one-page region so the address space region stays pure overhead.
    let Some(frame_region) = crate::memory_region::create(1) else {
        println!("bench: map_el0 skipped (no frame region)");
        return;
    };
    let Some(phys) = crate::memory_region::retype_page(frame_region) else {
        println!("bench: map_el0 skipped (no frame)");
        return;
    };

    let report = sched::create_rendezvous();
    sched::spawn(move || {
        crate::user::run(
            image,
            crate::user::Spawn {
                arg0: EL_MAP,
                arg1: 0,
                arg2: 0,
                grants: &[
                    rendezvous_cap(report, Rights::WRITE), // slot 0: report the result
                    address_space_cap(name, Rights::WRITE), // slot 1: the space we map into
                    page_frame_cap(phys, Rights::READ),    // slot 2: the frame we alias-map
                ],
                maps: &[],
            },
        )
    })
    .expect("bench: could not spawn the map bencher");

    let [ticks, iters, ..] = sched::ipc_receive(report);
    println!("bench: map_el0 {ticks} {iters}");
}

/// **Spawn latency, measured from EL0 (the primitive suite).** lmbench's `lat_proc`, and the payoff
/// of object revocation: a userspace spawner builds a whole child from EL0 through the granular verbs
/// and, crucially, `DESTROY`s the child's region afterward, so the loop repeats. The bench boot hands
/// the spawner three things: a big untyped budget (slot 1), a report endpoint to answer on (slot 0),
/// and a child-done endpoint (slot 2, READ|WRITE|GRANT) it delegates a WRITE view of to each child.
/// See `fixtures/src/os_primitives_benchmarker.rs`.
fn spawn_el0() {
    let Some(image) = crate::trust::require_program("os_primitives_benchmarker") else {
        println!("bench: spawn_el0 skipped (no os_primitives_benchmarker in the initrd)");
        return;
    };
    let Some(region) = crate::memory_region::create(SPAWN_EL0_BUDGET) else {
        println!("bench: spawn_el0 skipped (no budget)");
        return;
    };
    let report = sched::create_rendezvous();
    let child_done = sched::create_rendezvous();
    sched::spawn(move || {
        crate::user::run(
            image,
            crate::user::Spawn {
                arg0: EL_SPAWN,
                arg1: 0,
                arg2: 0,
                grants: &[
                    rendezvous_cap(report, Rights::WRITE), // slot 0: report the result home
                    memory_region_cap(region),             // slot 1: the spawner's whole budget
                    rendezvous_cap(
                        child_done,
                        Rights::READ.union(Rights::WRITE).union(Rights::GRANT),
                    ), // slot 2: children signal done; spawner receives and delegates WRITE
                ],
                maps: &[],
            },
        )
    })
    .expect("bench: could not spawn the spawner");
    let [ticks, iters, ..] = sched::ipc_receive(report);
    println!("bench: spawn_el0 {ticks} {iters}");
}

/// **The compute workload (milestone 19e), for the record.** Unlike the paths above, this touches
/// no OS primitive: it is pure computation (the CoreMark-derived kernel, `crates/coremark`), so its
/// cost is the *core's*, not nife's. It is here because the same crate runs as an EL0 workload
/// and later on macOS and Linux, and this line is where the nife compute number is recorded,
/// on the same two instruments as everything else. Running it in the kernel is fine: compute is
/// privilege-independent, so this number equals the EL0 workload's. `SINK` keeps it live.
fn coremark_compute() {
    static SINK: AtomicU64 = AtomicU64::new(0);
    timed("coremark", COREMARK_ITERS, || {
        let crc = coremark::run(COREMARK_ITERS as u32);
        SINK.store(crc as u64, Ordering::Relaxed);
    });
}

/// **The userspace file-server tax** (DECISIONS §32, the flagship userspace-reuse story). This is the
/// number a microkernel skeptic asks for about a filesystem in userspace: a client opens a file
/// through a granted *directory capability* and reads a block, over the real confined stack, a block
/// server driving the RedoxFS disk by DMA and an FS server (the vendored RedoxFS engine, `no_std`, on
/// its own heap) mounting it over blk IPC. `kernel/src/user/fs_service.rs` wires all three; the
/// client (`fixtures/src/fs_test_client.rs`, `ROLE_BENCH`) times a warm read loop and reports `[ticks, iters]`.
///
/// **Why it is `--real`-only and never gates, unlike the primitives.** The FS server's mount is
/// device-driven: hundreds of block reads gated on the disk's completion interrupt, plus the engine's
/// own logic. Under `-icount shift=0` that path is not deterministic (interrupt timing is not part of
/// the instruction clock), so an icount baseline for it would enshrine exactly the non-determinism the
/// 2026-07-28 lesson warns against. So it runs only on the `--real --smp` boot (HVF, where the whole
/// stack is proven by the `redoxfs_server` test), self-skipping everywhere else via the same
/// `online_count() > 1` gate as the throughput bench, so `bench/baseline-aarch64.txt` never sees it.
///
/// **What the number means: whole-path cost, dominated by the device.** ~204 us/read (HVF,
/// `--release --smp`). A read is **not** served warm from a cache: it goes to the block server, which
/// does a DMA transfer and waits on the disk's completion interrupt, roughly 200 us per block under
/// HVF. That swamps the FS server's own IPC-contract tax, which `relay_rtt` puts at a few hundred
/// *nanoseconds*. So this is the honest whole-path cost of a userspace file read, not an isolated
/// server tax, which is exactly the case milestone 21's rule names: when device latency swamps the
/// isolation, measure the whole path and say so rather than report a fictional isolated number.
///
/// The isolated file-server cost was attempted and abandoned, and this comment used to describe that
/// abandoned design (claiming a warm cache and a contract-cost measurement), which contradicted
/// notes/benchmarks.md for long enough to mislead a reader. Corrected here: a few hundred ns of
/// serving layer sitting on a ~200 us block read with its own run-to-run spread puts the delta inside
/// the device noise. The per-hop tax lives in `relay_rtt`, where it is measurable and gated.
///
/// **Since milestone 606 (a directory walk costs what it does on Linux), the read after the first
/// is served from the FS server's memory**, like a Linux buffered read from the page cache, so this
/// row measures the confined IPC path plus a copy (about 260 ns under HVF, release, 2026-09-26)
/// and no longer the device. The history above is kept because it is why the memo exists. The
/// device path is still what `fs_seq_read` and its siblings measure: their file is 1 MiB, over
/// the memo's 256 KiB limit.
///
/// **What it is not:** a filesystem throughput number. There is no MB/s figure and no comparison
/// against ext4 or APFS, which is DECISIONS §34's unmet condition 2 and milestone 38.
fn fs_read() {
    // Same gate as the throughput bench: meaningful only on the `--real --smp` boot, which is where
    // the RedoxFS disk is attached and the whole stack is proven. Single hart (icount, default
    // `--real`) skips, so this never reaches the deterministic baseline.
    if crate::smp::online_count() <= 1 {
        return;
    }
    // The three binaries the service needs. The block server is `block_driver` on every
    // architecture since milestone 291. Absent any of them, or the RedoxFS disk, skip.
    let (Some(blk_image), Some(redoxfs_server), Some(fs_test_client)) = (
        crate::trust::require_program("block_driver"),
        crate::trust::require_program("redoxfs_server"),
        crate::trust::require_program("fs_test_client"),
    ) else {
        return;
    };
    // Spawn the block server, the FS server, and the client in its ROLE_BENCH (timed) role.
    let Some((readiness, report)) =
        crate::user::fs_service::start(blk_image, redoxfs_server, fs_test_client, 1)
    else {
        return; // no RedoxFS disk on this run
    };
    // Sequence on readiness, exactly as the test does: the block server brings the device up, then
    // the FS server mounts the image, then the client's timed loop reports. The bench boot is the
    // only caller, so it always gets the readiness endpoints (nothing wired the service first).
    if let Some((blk_ready, ready)) = readiness {
        // `blk_ready` is `None` when the block service was already running and its sentinel has
        // been taken; a bench boot always wires it, so this arm is the one that runs here.
        if let Some(blk_ready) = blk_ready {
            let _ = sched::ipc_receive(blk_ready);
        }
        let _ = sched::ipc_receive(ready);
    }
    let [ticks, iters, ..] = sched::ipc_receive(report);
    println!("bench: fs_read {ticks} {iters}");
}

/// **Filesystem throughput through the confined FS server** (milestone 38, DECISIONS §34
/// condition 2). Six phases over one file the client creates on the RedoxFS image: sequential
/// write, sequential read, random read, a **record-aligned** read (the cheapest 4 KiB read RedoxFS
/// can serve, which is what makes the record-geometry claim a measurement rather than arithmetic),
/// random write, and the cost of producing a page of payload, which the write phases pay inside
/// their window and a reader should be able to subtract. `fs_read` above answers "what does one file
/// read cost"; this answers "how fast does this architecture move a file", which is the question a
/// microkernel skeptic actually asks and the one the tree had no number for at all.
///
/// **It reports both a row and a probe line, and the split is deliberate.** The row
/// (`bench: fs_seq_read <ticks> <transfers>`) is the harness's ordinary shape, so the table's
/// ns/iter column is the per-request latency, which is the number that compares to `fs_read`. The
/// probe line carries MiB/s, which is the number that compares to another operating system, and it
/// is a probe rather than a row because the baseline gate must never see any of this: like
/// `fs_read`, every phase here is device- and interrupt-driven, so it self-skips off the
/// `--real --smp` boot and could not be deterministic if it did not.
///
/// **One transfer is 4096 bytes and cannot be more**, because a `filesystem_protocol` request carries its
/// payload through the one page the client shares with the server. So these figures are a request
/// rate in disguise, and any comparison against a system whose client may pass a 64 KiB buffer is
/// comparing two different things. notes/benchmarks.md states that next to the numbers rather than
/// under them.
fn fs_throughput() {
    // The same gate as `fs_read`: this is meaningful only on the `--real --smp` boot, which is the
    // one that attaches the RedoxFS disk and proves the stack.
    if crate::smp::online_count() <= 1 {
        return;
    }
    let (Some(blk_image), Some(redoxfs_server), Some(fs_test_client)) = (
        crate::trust::require_program("block_driver"),
        crate::trust::require_program("redoxfs_server"),
        crate::trust::require_program("fs_test_client"),
    ) else {
        return;
    };
    // `start` reuses the service `fs_read` already wired, so this spawns one more client on the
    // same FS server and the same shared page: `readiness` comes back `None` and there is nothing
    // to wait for but the reports.
    let Some((readiness, report)) = crate::user::fs_service::start(
        blk_image,
        redoxfs_server,
        fs_test_client,
        filesystem_protocol::fixture::throughput::ROLE,
    ) else {
        return; // no RedoxFS disk on this run
    };
    if let Some((blk_ready, ready)) = readiness {
        // `blk_ready` is `None` when the block service was already running and its sentinel has
        // been taken; a bench boot always wires it, so this arm is the one that runs here.
        if let Some(blk_ready) = blk_ready {
            let _ = sched::ipc_receive(blk_ready);
        }
        let _ = sched::ipc_receive(ready);
    }
    let hz = crate::arch::timer::frequency();
    for _ in 0..filesystem_protocol::fixture::throughput::PHASES {
        let [ticks, transfers, phase, ..] = sched::ipc_receive(report);
        let Some(name) = filesystem_protocol::fixture::throughput::name(phase) else {
            println!("bench-probe: fs_throughput unknown phase {phase}");
            continue;
        };
        println!("bench: {name} {ticks} {transfers}");
        // MiB/s, in integer arithmetic on the kernel's side because the kernel is where the counter
        // frequency and the transfer size are both known. `bytes * hz / ticks` is bytes per second
        // and the shift turns that into MiB/s; the extra factor of 100 buys two decimal places,
        // without which the interesting figures here (1.52 and 2.59) both print as "2". Ticks of
        // zero would mean the counter did not advance across a whole phase, which is a broken run
        // rather than a fast one, so it reports zero instead of dividing.
        let bytes = transfers * filesystem_protocol::fixture::throughput::UNIT as u64;
        let hundredths = bytes
            .checked_mul(hz)
            .and_then(|v| v.checked_mul(100))
            .and_then(|v| v.checked_div(ticks))
            .map(|v| v >> 20)
            .unwrap_or(0);
        println!(
            "bench-probe: {name}_mib_per_s {}.{:02}",
            hundredths / 100,
            hundredths % 100
        );
    }
}

/// **A directory walk through a confined grant** (milestone 606 (a directory walk costs what it
/// does on Linux), provisional number). `std_exerciser` holds the tree milestone 121 (`ripgrep` on nife: enumeration as a
/// capability) priced, behind an
/// `fs_subtree_caretaker`, with the rights the confined `rg pattern src/` will hold, and walks it
/// with plain `std::fs` (`walk_pricing`): list every directory, read every file. The row is the
/// median of that program's warm walks; the probes are the rest of what it printed, the split by
/// `std::fs` call first. This is the release-kernel figure the kernel suite cannot give (its kernel
/// is a debug build), and the one `bench/host/run_linux_walk.sh` prices Linux against on the same
/// `virt,accel=hvf` machine and the same virtio disk tier.
///
/// `--real --smp` only, for `fs_read`'s reasons: it is device- and interrupt-driven, so it never
/// reaches the icount baseline. Skips when the boot has no RedoxFS disk or the archive no
/// `std_exerciser` (it is built with `-Zbuild-std` against `nife-dev`; the `--smp` leg builds it).
///
/// # BUGS
///
/// - **Warm only.** The program's first walk counts the tree and fills the FS server's memo; the
///   row is a walk after that, which is what a repeated `rg` or `find` is. A cold walk is a
///   different number, and Linux's `drop_caches` would be the matching control.
fn fs_walk() {
    if crate::smp::online_count() <= 1 {
        return;
    }
    let (Some(blk), Some(server), Some(caretaker), Some(walker)) = (
        crate::trust::require_program("block_driver"),
        crate::trust::require_program("redoxfs_server"),
        crate::trust::require_program("fs_subtree_caretaker"),
        crate::trust::require_program("std_exerciser"),
    ) else {
        println!("bench-probe: fs_walk skipped (a program is missing from the archive)");
        return;
    };
    use filesystem_protocol::dir;
    let Some(spawned) = crate::user::fs_service::start_std_narrowed(
        blk,
        server,
        caretaker,
        walker,
        filesystem_protocol::fixture::walk::ROOT,
        dir::ENUMERATE | dir::READ | dir::DESCEND,
        None,
    ) else {
        return; // no RedoxFS disk on this run
    };
    walk_rows(spawned.report, "fs_walk");
}

/// **The same walk through a grant the FS server enforces itself** (milestone 606, calef's
/// ruling D): no caretaker, the program holding the server's endpoint with a badge the kernel
/// bound to the subtree. The row beside `fs_walk` is the price of the caretaker's hop. It takes
/// the grant back afterwards, so a later row starts from the same server state.
fn fs_walk_bound() {
    if crate::smp::online_count() <= 1 {
        return;
    }
    let (Some(blk), Some(server), Some(walker)) = (
        crate::trust::require_program("block_driver"),
        crate::trust::require_program("redoxfs_server"),
        crate::trust::require_program("std_exerciser"),
    ) else {
        return;
    };
    use filesystem_protocol::dir;
    let Some(spawned) = crate::user::fs_service::start_std_bound(
        blk,
        server,
        walker,
        filesystem_protocol::fixture::walk::ROOT,
        dir::ENUMERATE | dir::READ | dir::DESCEND,
    ) else {
        return;
    };
    walk_rows(spawned.report, "fs_walk_bound");
    while crate::sched::is_thread_present(spawned.thread) {
        sched::yield_now();
    }
    if !spawned.release() {
        println!("bench-probe: fs_walk_bound could not take its grant back");
    }
}

/// Warm `rg` runs [`rg_search`] times, after one untimed run that fills the FS server's memo. Odd,
/// so the median is a run.
const RG_SEARCH_RUNS: usize = 5;

/// **Unmodified `ripgrep` searching the priced tree** (milestone 121, `ripgrep` on nife), the
/// search half of the walk [`fs_walk`] prices. `rg` holds the same grant behind the same caretaker
/// and is told `fixture::walk::RG_SEARCH` as its argv. The row is ripgrep's own `--stats` figure,
/// the median of [`RG_SEARCH_RUNS`] warm runs: it starts after argument parsing and ends after the
/// last file, so this system's process creation and image loading are not in it, and neither are
/// Linux's in `bench/host/run_linux_rg.sh`, which runs the same words through the same function
/// on the same machine. The spawn-to-exit time, which does include them, is a probe beside it.
///
/// Present only when somebody ran `helpers/build-ripgrep.sh`: `rg` is never part of the base image
/// (calef, 2026-10-07), so the row skips on every ordinary bench boot and in CI.
///
/// # BUGS
///
/// - **One process per run.** Each run is a fresh `rg`, so its heap, its allocator and its regex
///   compilation are cold every time; only the FS server is warm. That is what a person typing
///   `rg` twice gets, on both sides.
fn rg_search() {
    if crate::smp::online_count() <= 1 {
        return;
    }
    let (Some(blk), Some(server), Some(caretaker), Some(rg)) = (
        crate::trust::require_program("block_driver"),
        crate::trust::require_program("redoxfs_server"),
        crate::trust::require_program("fs_subtree_caretaker"),
        crate::trust::require_program("rg"),
    ) else {
        println!("bench-probe: rg_search skipped (no rg in the archive; helpers/build-ripgrep.sh)");
        return;
    };
    use filesystem_protocol::dir;
    use filesystem_protocol::fixture::walk as tree;
    let hz = crate::arch::timer::frequency();
    let mut searched = [0u64; RG_SEARCH_RUNS];
    let mut whole = [0u64; RG_SEARCH_RUNS];
    for run in 0..=RG_SEARCH_RUNS {
        let started = crate::arch::timer::now();
        let Some(spawned) = crate::user::fs_service::start_std_narrowed(
            blk,
            server,
            caretaker,
            rg,
            tree::ROOT,
            dir::ENUMERATE | dir::READ | dir::DESCEND,
            Some(tree::RG_SEARCH.as_bytes()),
        ) else {
            return; // no RedoxFS disk on this run
        };
        let mut text = [0u8; 8192];
        let len = drain_report(spawned.report, &mut text);
        while crate::sched::is_thread_present(spawned.thread) {
            sched::yield_now();
        }
        let ended = crate::arch::timer::now();
        if !spawned.release() {
            println!("bench-probe: rg_search could not take its grant back");
            return;
        }
        let text = core::str::from_utf8(&text[..len]).unwrap_or("");
        let Some(ns) = rg_seconds(text) else {
            println!("bench-probe: rg_search printed no stats ({len} bytes)");
            return;
        };
        if run == 0 {
            // The counts, once, so a reader can see the search found what the kernel test asserts.
            for line in text
                .lines()
                .filter(|l| l.contains("files searched") || l.contains("matches"))
            {
                println!("bench-probe: rg_search {line}");
            }
            continue;
        }
        searched[run - 1] = ns;
        whole[run - 1] = (ended - started) * 1_000_000_000 / hz;
    }
    searched.sort_unstable();
    whole.sort_unstable();
    let mid = RG_SEARCH_RUNS / 2;
    println!("bench: rg_search {} 1", searched[mid] * hz / 1_000_000_000);
    println!(
        "bench-probe: rg_search {} ns by rg's own clock, {} to {}",
        searched[mid],
        searched[0],
        searched[RG_SEARCH_RUNS - 1]
    );
    println!(
        "bench-probe: rg_search spawn to exit {} ns, {} to {}",
        whole[mid],
        whole[0],
        whole[RG_SEARCH_RUNS - 1]
    );
}

/// `rg --stats`'s last line, `<seconds> seconds` with six decimals, in nanoseconds.
fn rg_seconds(text: &str) -> Option<u64> {
    let line = text.lines().rev().find(|l| l.ends_with(" seconds"))?;
    let (whole, frac) = line.strip_suffix(" seconds")?.split_once('.')?;
    let mut ns = whole.parse::<u64>().ok()? * 1_000_000_000;
    let mut scale = 100_000_000;
    for b in frac.bytes() {
        ns += u64::from(b.checked_sub(b'0')?) * scale;
        scale /= 10;
    }
    Some(ns)
}

/// A program's whole transcript from its stdout sink, as far as `out` holds it.
fn drain_report(report: crate::sched::RendezvousId, out: &mut [u8]) -> usize {
    let mut len = 0usize;
    loop {
        let words = sched::ipc_receive(report);
        let mut chunk = [0u8; byte_sink_protocol::INLINE_MAX];
        match byte_sink_protocol::unpack(words[0], words[1], words[2], &mut chunk) {
            byte_sink_protocol::Msg::Bytes(n) => {
                let take = n.min(out.len() - len);
                out[len..len + take].copy_from_slice(&chunk[..take]);
                len += take;
            }
            byte_sink_protocol::Msg::Eof | byte_sink_protocol::Msg::Malformed => return len,
        }
    }
}

/// Drain a priced walk's transcript from `report` and print it as the `row` and its probes.
fn walk_rows(report: crate::sched::RendezvousId, row: &str) {
    // The transcript, decoded from the program's stdout sink and kept only as far as this buffer.
    let mut text = [0u8; 2048];
    let mut len = 0usize;
    loop {
        let words = sched::ipc_receive(report);
        let mut chunk = [0u8; byte_sink_protocol::INLINE_MAX];
        match byte_sink_protocol::unpack(words[0], words[1], words[2], &mut chunk) {
            byte_sink_protocol::Msg::Bytes(n) => {
                let take = n.min(text.len() - len);
                text[len..len + take].copy_from_slice(&chunk[..take]);
                len += take;
            }
            byte_sink_protocol::Msg::Eof => break,
            byte_sink_protocol::Msg::Malformed => {
                println!(
                    "bench-probe: fs_walk malformed sink message {:#x}",
                    words[0]
                );
                return;
            }
        }
    }
    let text = core::str::from_utf8(&text[..len]).unwrap_or("");
    let hz = crate::arch::timer::frequency();
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("walk whole ") {
            let ns: u64 = rest
                .split(' ')
                .next()
                .and_then(|n| n.parse().ok())
                .unwrap_or(0);
            // The program timed in nanoseconds; the row is in counter ticks like every other.
            println!("bench: {row} {} 1", ns * hz / 1_000_000_000);
        }
        if let Some(rest) = line.strip_prefix("walk ") {
            println!("bench-probe: {row} {rest}");
        }
    }
}

// --- Multi-hart aggregate throughput (DECISIONS §28, the SMP placement win) ---
//
// Every primitive above is hart-pinned by design: the icount instrument boots `-smp 1` (the
// 2026-07-28 attribution finding, notes/benchmarks.md), so those numbers are per-core path length
// and cannot show §28's placement work at all. This one is different, and its methodology has to be
// different, so it is set apart here on purpose.
//
// It runs N independent ping-pong pipelines and measures the WALL-CLOCK time to complete them. A
// single pipeline is a synchronous rendezvous: only one of its two threads is runnable at a time,
// so it keeps ~one core busy and §28's local-wake rule keeps the pair co-located and warm. N
// independent pipelines are N such streams, which §28's power-of-two placement scatters across the
// cores at spawn. So the aggregate throughput of N pipelines should approach `online_count()` times
// a single pipeline's throughput: the whole machine filled, which is the property §28 exists to
// deliver and the one no hart-pinned primitive can see.
//
// **Why it is NOT on the icount baseline, and never gates.** Two reasons, both structural:
//   1. It is meaningful only with more than one hart. The icount instrument pins `-smp 1`, and so
//      does the default `--real` run (per-core magnitudes; see xtask's `bench`), so it runs ONLY on
//      the `--real --smp` boot (HVF, 4 harts), which the harness already forbids from
//      `--check`/`--save`. Everywhere else `online_count()` is 1 and it is skipped outright, so it
//      never touches `bench/baseline-aarch64.txt`.
//   2. Under `-icount` all vCPUs share one virtual clock (again the 2026-07-28 finding), so a
//      wall-clock throughput number is not even defined there; and TCG serialises vCPUs onto one
//      host thread, so there is no real parallelism to measure. Only HVF gives each core its own
//      counter and real concurrent execution.
//
// So this is a statistical `--real` measurement read by a human with loose bounds, exactly like the
// other HVF magnitudes, not a deterministic tick baseline. It reports two lines, `smp_pipe_solo`
// (one pipeline) and `smp_pipe_all` (N pipelines); the scaling factor is the solo ns/iter divided
// by the all ns/iter, and it should land near `online_count()`.
//
// **A methodology note about the solo baseline, learned by getting it wrong first.** The whole
// result is only as honest as its single-core reference, and the obvious way to take it is a trap.
// The main thread must **block** (a real `RECEIVE`) while a batch runs, exactly as `ipc_rtt` above does,
// NOT busy-yield waiting on a counter. A yield-spinning main stays runnable, so on the solo batch
// the scheduler sees main plus the pair (three runnable-ish threads) and scatters them, turning each
// local rendezvous into a cross-core wake; the solo pair then clocked ~60x slower than `ipc_rtt`'s
// identical pair and the derived scaling went *superlinear* (>cores), which is not physical. With
// main blocked, the solo pair co-locates and runs at the `ipc_rtt` rate, and scaling lands at or
// below `cores`, as it must. Each batch is also run a few times and the **minimum** ticks kept: on a
// shared desktop under HVF the host preempts guest vCPUs, and the min is the least-contended sample,
// the closest thing to the true rate. Loose bounds, read by a human.

/// Independent ping-pong pipelines. 16 on a 4-hart boot is four per core, enough that placement and
/// stealing fill every core and the aggregate is not starved by too few streams.
const TP_PIPES: usize = 16;
/// Round trips per pipeline in a batch. Large enough that spawn and first-rendezvous costs (paid
/// before the clock starts, behind the GO barrier) are noise against the timed steady state.
const TP_RTT: u64 = 2000;
/// Batches per measurement; the minimum ticks is kept (least host contention under HVF).
const TP_REPEAT: usize = 4;

static TP_GO: AtomicBool = AtomicBool::new(false);

/// Run `pipes` independent ping-pong pipelines over the pre-created endpoint pairs and return the
/// wall-clock ticks to complete them all. The clock spans only steady state: threads are spawned
/// first, then released together by `TP_GO` after `now()` is read, so N-pipeline batches are not
/// charged N times the spawn cost that a 1-pipeline batch pays only once. `done` collects one signal
/// per pipeline so the main thread can **block** (not busy-yield) through the timed window.
fn tp_batch(
    req: &[sched::RendezvousId],
    reply: &[sched::RendezvousId],
    done: sched::RendezvousId,
    pipes: usize,
) -> u64 {
    TP_GO.store(false, Ordering::SeqCst);
    let base = sched::thread_count();

    for i in 0..pipes {
        let (rq, rp) = (req[i], reply[i]);
        // The server half: exactly TP_RTT receive-then-send, then it returns and is reaped. It blocks
        // in `ipc_receive` until its client sends, so it effectively starts when the client does.
        sched::spawn(move || {
            for _ in 0..TP_RTT {
                let _ = sched::ipc_receive(rq);
                sched::ipc_send(rp, [1, 0, 0]);
            }
        })
        .expect("bench: throughput server spawn failed");

        // The client half: wait at the barrier, then TP_RTT send-then-receive, then signal done. Yield
        // (not spin) at the barrier so a waiting client does not burn its core before the clock.
        sched::spawn(move || {
            while !TP_GO.load(Ordering::Acquire) {
                sched::yield_now();
            }
            for _ in 0..TP_RTT {
                sched::ipc_send(rq, [1, 0, 0]);
                let _ = sched::ipc_receive(rp);
            }
            sched::ipc_send(done, [1, 0, 0]);
        })
        .expect("bench: throughput client spawn failed");
    }

    // Start the clock, release every client in one step, then BLOCK receiving one done per pipeline.
    // Blocking (not yield-spinning) is what keeps the solo pair co-located; see the methodology note.
    let t0 = crate::arch::timer::now();
    TP_GO.store(true, Ordering::Release);
    for _ in 0..pipes {
        let _ = sched::ipc_receive(done);
    }
    let ticks = crate::arch::timer::now() - t0;

    // Drain: let the servers finish their last iterations and every thread reap before the next
    // batch, so a batch's timing is never charged the previous batch's teardown.
    while sched::thread_count() > base {
        sched::yield_now();
    }
    ticks
}

/// Minimum ticks over `TP_REPEAT` batches of `pipes` pipelines: the least host-contended sample.
fn tp_best(
    req: &[sched::RendezvousId],
    reply: &[sched::RendezvousId],
    done: sched::RendezvousId,
    pipes: usize,
) -> u64 {
    let mut best = u64::MAX;
    for _ in 0..TP_REPEAT {
        best = best.min(tp_batch(req, reply, done, pipes));
    }
    best
}

/// Inner iterations per compute worker. Sized so a single worker runs a few hundred microseconds
/// under HVF, long enough that the 41 ns counter grain and host jitter are noise against the batch.
const TC_WORK: u64 = 300_000;

static TC_SINK: AtomicU64 = AtomicU64::new(0);

/// A non-elidable integer grind: an LCG mixed with an xorshift, folded into a returned value the
/// caller sinks so the optimizer cannot delete the loop. Pure compute, no syscalls, so a running
/// worker touches no other core: this is the workload that isolates §28 **placement** from IPC's
/// cross-core wakes, which is exactly why the pipeline result and this one differ under HVF.
#[inline(never)]
fn busy(iters: u64) -> u64 {
    let mut x = 0x9E37_79B9_7F4A_7C15u64;
    for i in 0..iters {
        x = x
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407 ^ i);
        x ^= x >> 29;
    }
    x
}

/// Run `workers` independent CPU-bound workers behind the GO barrier and return the wall-clock ticks
/// to finish them all. Each worker does the same fixed grind, so N workers is N times the work of
/// one; §28 placement should spread them so the aggregate finishes in about `ceil(N/cores)` worker-
/// times, i.e. aggregate throughput approaches `cores` times a single worker's. No rendezvous during
/// the grind, so no cross-core wakes: the clean placement measurement. Main blocks on `done`.
fn tc_batch(done: sched::RendezvousId, workers: usize) -> u64 {
    TP_GO.store(false, Ordering::SeqCst);
    let base = sched::thread_count();
    for _ in 0..workers {
        sched::spawn(move || {
            while !TP_GO.load(Ordering::Acquire) {
                sched::yield_now();
            }
            let v = busy(TC_WORK);
            TC_SINK.fetch_add(v, Ordering::Relaxed);
            sched::ipc_send(done, [1, 0, 0]);
        })
        .expect("bench: compute worker spawn failed");
    }
    let t0 = crate::arch::timer::now();
    TP_GO.store(true, Ordering::Release);
    for _ in 0..workers {
        let _ = sched::ipc_receive(done);
    }
    let ticks = crate::arch::timer::now() - t0;
    while sched::thread_count() > base {
        sched::yield_now();
    }
    ticks
}

/// Minimum ticks over `TP_REPEAT` compute batches: the least host-contended sample.
fn tc_best(done: sched::RendezvousId, workers: usize) -> u64 {
    let mut best = u64::MAX;
    for _ in 0..TP_REPEAT {
        best = best.min(tc_batch(done, workers));
    }
    best
}

/// **Aggregate multi-hart throughput.** See the block comment above for the methodology and why this
/// is a `--real`-only, non-gating measurement. Skipped on a single hart (the icount boot), where it
/// would measure nothing §28 does.
///
/// Two workloads, on purpose. **Compute** (`smp_compute_*`) is N independent CPU-bound workers: no
/// syscalls, so no cross-core wakes, so the host keeps every busy vCPU on a real core and the
/// aggregate scales cleanly to `cores`. This is the direct picture of §28 placement filling the
/// machine. **Pipelines** (`smp_pipe_*`) is N synchronous IPC ping-pong pairs, and under HVF it does
/// NOT scale, it goes slightly backwards: a semi-idle guest vCPU is descheduled by the host, so the
/// cross-core wakes an IPC pipeline needs whenever placement/stealing splits a pair pay host
/// reschedule latency that a single co-located pair never does. That is a virtualization property,
/// not a scheduler defect (it is the same reason the icount primitive suite is pinned to one hart),
/// and recording both is the honest result: compute parallelises on this instrument, synchronous IPC
/// does not, and the reason is the host underneath.
fn smp_throughput() {
    let cpus = crate::smp::online_count();
    if cpus <= 1 {
        return; // single hart (the icount instrument): there is no placement win to show.
    }

    // Rendezvous pairs, created once and reused across batches so a repeated run does not leak the
    // endpoint table down. One request and one reply endpoint per pipeline, plus a shared done EP.
    let mut req = [0u64; TP_PIPES];
    let mut reply = [0u64; TP_PIPES];
    for i in 0..TP_PIPES {
        req[i] = sched::create_rendezvous();
        reply[i] = sched::create_rendezvous();
    }
    let done = sched::create_rendezvous();

    // Compute: the clean placement win. Warm once, then measure solo and the full machine.
    let _ = tc_batch(done, 1);
    let compute_solo = tc_best(done, 1);
    let compute_all = tc_best(done, TP_PIPES);

    // Pipelines: the IPC workload, which does not parallelise under HVF (see the doc comment).
    let _ = tp_batch(&req, &reply, done, 1);
    let pipe_solo = tp_best(&req, &reply, done, 1);
    let pipe_all = tp_best(&req, &reply, done, TP_PIPES);

    // Each `*_all` is TP_PIPES times the work of its `*_solo`. The scaling factor is solo ns/iter
    // divided by all ns/iter: near `cores` for compute (the machine filled), near or below 1 for
    // pipelines under HVF. The `smp_cpus` line records the ceiling.
    println!("bench: smp_compute_solo {compute_solo} {TC_WORK}");
    println!(
        "bench: smp_compute_all {compute_all} {}",
        TP_PIPES as u64 * TC_WORK
    );
    println!("bench: smp_pipe_solo {pipe_solo} {TP_RTT}");
    println!(
        "bench: smp_pipe_all {pipe_all} {}",
        TP_PIPES as u64 * TP_RTT
    );
    println!("bench: smp_cpus {cpus} {cpus}");
}
