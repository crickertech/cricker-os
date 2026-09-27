//! **The kernel proving it works on this machine, before it hands the machine to anybody.**
//!
//! Milestone 268's second rung. The boot ladder is: say what the machine is
//! ([`crate::print_machine_description`]), prove the kernel works on it (here), then hand over. All
//! three architectures climb the same ladder, and this module is the same code on all three: every
//! check below is portable, and the one place an architecture differs is
//! `arch::exceptions::self_test`, which each one already had to write anyway.
//!
//! # It reports, it does not gate
//!
//! calef, 2026-09-09: *"I could see building tools to diagnose what broke but we can only run them
//! if there is a prompt"*. A failing check prints its verdict and the boot carries on to userspace.
//! A machine you cannot log into is a machine you cannot fix, and a self-test that halted the boot
//! would take the diagnosis away at exactly the moment it is wanted.
//!
//! **So the verdict has to be read by something other than a person**, or milestone 268 ships its
//! own finding 4 back: a check that reports by printing a word and carrying on, with nothing
//! gating. `boot_ladder::SELF_TEST` is that contract. `crates/board_console` matches it, `cargo xtask
//! boot-check` boots all three architectures and fails on a red one, and CI runs that.
//!
//! # The verdict line is a contract, and its wording is provisional
//!
//! ```text
//! nife self-test: 5 of 5 passed
//! nife self-test: 4 of 5 passed, 1 FAILED: exceptions
//! ```
//!
//! Three properties, each of which is a lesson from something that already went wrong in this tree:
//!
//! - **The prefix is always present and identical on every architecture.** Milestone 268's finding
//!   3 is that `Stage::Tour`'s only matchable string lived inside the RISC-V arm of `main.rs`, so
//!   on the other two there was no signal on the channel at all, and a missing marker looks exactly
//!   like a slow board.
//! - **The counts print on success as well as on failure**, so "ran nothing and passed" is
//!   distinguishable from "ran the set and passed". A vacuous pass is the failure a gate acquires
//!   silently.
//! - **`FAILED:` is spelled the way this tree already spells it**, in `main.rs`'s preemption check
//!   (`FAILED: a spinner did not run, or nothing was preempted.`), rather than as a new word.
//!
//! The wording is **provisional** (milestone 268 (every architecture boots the same way)): a line
//! two programs agree on is an architect's under AGENTS.md's *move fast on what can be undone*
//! tenet, and the milestone block says a lane should ship one and say so rather than wait. The
//! other half of the contract is `board_console::progress`, which is tested against this module's
//! own constant rather than against a remembered string.
//!
//! # BUGS
//!
//! - **Nothing here proves an architecture ran the set.** The verdict says five of five passed on a
//!   kernel that ran five checks; it cannot say that the *right* five ran, because the list is a
//!   compile-time fact and a check deleted from it takes its own evidence with it. The count is the
//!   partial defence (a shrunken set prints a smaller total), and `crates/board_console`'s tests
//!   pin the wording, but the set itself is guarded by review.
//! - **The set is cheap on purpose and therefore shallow.** Every check below completes in
//!   microseconds except [`fn@timer`], which waits about 20 ms for a tick. Nothing here spawns a
//!   process, touches a device, or exercises IPC, because this runs on every boot including a
//!   board's, and a boot self-test that cost a second would be turned off. The deep proofs are
//!   `script/test`'s suite; this is the subset that has to be true before a prompt is worth
//!   offering.
//! - **A check can still hang the boot if the operation it measures never returns.** The two
//!   waits here no longer can: both read the counter through [`Counter`], which reports a counter
//!   that has stopped rather than waiting on it, and [`fn@timer`] spins rather than sleeping, so a
//!   machine whose interrupts never arrive cannot park it either. What nothing here bounds is a
//!   `sched::yield_now` or an `mmu::map_page` that itself never comes back; that needs a watchdog
//!   on another core or on the timer interrupt, which this module deliberately does not own.
//!   (Before 2026-09-14 a stopped counter hung [`fn@timer`] forever, and `arch::timer::spin_for`
//!   still has that exposure.)

use core::sync::atomic::{AtomicU64, Ordering};

// **The verdict's two fixed strings come from `crates/boot_ladder`**, not from literals here, so
// that this module and `crates/board_console` cannot hold two copies of one contract (`AGENTS.md`
// rule 7). Each carries its own trailing space, so a matcher that finds one has found a boundary
// rather than the start of a longer word.
use boot_ladder::{SELF_TEST as VERDICT, SELF_TEST_FAILED as FAILED};

use crate::{arch, memory, println, sched};

/// The timer the checks read. A module rather than `arch::timer` directly, so an injected build
/// can stop the counter these checks see without stopping the one the scheduler runs on.
mod timer {
    pub use crate::arch::timer::{frequency, ticks};

    /// The free-running counter, or a stopped one when the build injects a failure.
    pub fn now() -> u64 {
        if super::INJECT {
            0x0268
        } else {
            crate::arch::timer::now()
        }
    }
}

/// How many checks there are: the length of `boot_ladder::SELF_TEST_CHECKS`, and not a count of
/// the calls below. That difference is the fix for this module's old parity hole, where a check cut
/// from one architecture took its own evidence with it and the verdict still said *N of N*.
const CHECKS: usize = boot_ladder::SELF_TEST_CHECKS.len();

/// **Make one check report failure**, so the verdict can be *seen* red rather than reasoned about.
///
/// A verdict that has never been red is a verdict nobody has tested, and milestone 268's proof
/// condition is that an injected failure turns the line red and fails CI. `cargo xtask boot-check
/// --inject` builds each architecture with this on and asserts the gate goes red; without it the
/// only way to know would be to break the kernel for real.
///
/// It is a `cfg!` rather than a `#[cfg]` block so that both arms compile on every build: a fault
/// injector that only type-checks when it is switched on is an injector that has rotted by the time
/// somebody needs it.
///
/// **It injects two failures, and the second is a gate for the first fix.** Besides failing
/// `exceptions` it stops the counter the checks read (this module's `timer::now`), which is the
/// fault that used to hang [`fn@timer`] forever. So every `--inject` run also proves the waits are
/// bounded: a regression there turns a red verdict back into a boot that goes quiet, and
/// `boot-check --inject` fails on that.
///
/// Name: provisional (milestone 268 (every architecture boots the same way)).
const INJECT: bool = cfg!(feature = "self_test_injection");

/// The running count, and the names of whatever failed.
///
/// No allocation: the failure list is a fixed array of `&'static str`, because this runs before the
/// heap is interesting and because a self-test that could fail to report a failure by running out
/// of memory would be reporting on the wrong thing.
struct Tally {
    passed: usize,
    ran: usize,
    /// Which of `boot_ladder::SELF_TEST_CHECKS` recorded an outcome, by index.
    seen: [bool; CHECKS],
    failed: [&'static str; CHECKS],
    failures: usize,
}

impl Tally {
    const fn new() -> Self {
        Self {
            passed: 0,
            ran: 0,
            seen: [false; CHECKS],
            failed: [""; CHECKS],
            failures: 0,
        }
    }

    fn fail(&mut self, name: &'static str) {
        if self.failures < CHECKS {
            self.failed[self.failures] = name;
            self.failures += 1;
        }
    }

    /// Record one check's outcome and print its line.
    ///
    /// The detail is `core::fmt::Arguments` rather than a `&str`, so a check can put the numbers it
    /// measured in its own line without formatting into a buffer first. The numbers are the
    /// diagnosis: `ok` on its own tells a reader nothing they can act on when the next boot says
    /// `FAILED`.
    fn record(&mut self, name: &'static str, ok: bool, detail: core::fmt::Arguments) {
        self.ran += 1;
        // A check not on the shared list is a failure however it went: an architecture running a
        // check the others do not is the same parity hole as one skipping a check they run.
        let listed = boot_ladder::SELF_TEST_CHECKS
            .iter()
            .position(|&n| n == name);
        let ok = ok && listed.is_some();
        match listed {
            Some(i) => self.seen[i] = true,
            None => println!("  self-test       : {name} is not in boot_ladder::SELF_TEST_CHECKS"),
        }
        if ok {
            self.passed += 1;
        } else {
            self.fail(name);
        }
        println!(
            "  self-test       : {name:<10} {:<6}  {detail}",
            if ok { "ok" } else { "FAILED" },
        );
    }

    /// The verdict line. See [`VERDICT`] for why it is shaped the way it is.
    ///
    /// **The total is the shared list's length, not how many checks ran**, and a listed check that
    /// never recorded an outcome is named as a failure here. So a kernel that ran four of the five
    /// says `4 of 5 passed, 1 FAILED: scheduler` on every architecture, rather than `4 of 4 passed`.
    fn verdict(&mut self) {
        for (i, &name) in boot_ladder::SELF_TEST_CHECKS.iter().enumerate() {
            if !self.seen[i] {
                println!("  self-test       : {name:<10} FAILED  did not run on this architecture");
                self.fail(name);
            }
        }
        if self.failures == 0 {
            println!("{VERDICT}{} of {CHECKS} passed", self.passed);
            return;
        }
        // Built by printing rather than by joining, so there is no buffer to size and no allocation
        // on the path that reports a broken machine.
        print_verdict_head(self.passed, CHECKS, self.failures);
        for (i, name) in self.failed[..self.failures].iter().enumerate() {
            if i > 0 {
                crate::print!(" ");
            }
            crate::print!("{name}");
        }
        println!();
    }
}

/// `nife self-test: 4 of 5 passed, 1 FAILED: ` with no newline, so the names follow on the line.
fn print_verdict_head(passed: usize, ran: usize, failures: usize) {
    crate::print!("{VERDICT}{passed} of {ran} passed, {failures} {FAILED}");
}

/// **Run the set and print the verdict.**
///
/// Called from `kernel_main` on all three architectures, after the machine description and before
/// anything is handed to userspace. Everything it needs is up by then: paging, the frame allocator,
/// the timer with interrupts on, and the scheduler with its idle thread.
pub fn run() {
    let mut tally = Tally::new();

    exceptions(&mut tally);
    mapping(&mut tally);
    frames(&mut tally);
    timer(&mut tally);
    scheduler(&mut tally);

    tally.verdict();
}

/// **A breakpoint is caught and stepped over.**
///
/// The oldest self-test in this tree, and the one milestone 268 collected rather than invented:
/// riscv64 and `x86_64` have called `arch::exceptions::self_test` from `main.rs` since their ports
/// were written, and `notes/riscv-port.md` already calls it "a boot self-test". aarch64 had the
/// machinery (`BRK_COUNT`, and a handler that advances past a `brk`) and no function; milestone 268
/// gave it one.
///
/// What it proves is the whole trap round trip: the vector is installed at an address the hardware
/// accepts, a synchronous exception reaches our dispatcher, the dispatcher recognises the cause,
/// and the return lands on the instruction *after* the trapping one rather than on it. Getting the
/// last of those wrong is an infinite loop rather than a wrong answer.
fn exceptions(tally: &mut Tally) {
    let caught = arch::exceptions::self_test();
    let ok = caught >= 1 && !INJECT;
    if INJECT {
        tally.record(
            "exceptions",
            ok,
            format_args!("failure injected by the self_test_injection feature ({caught} caught)"),
        );
    } else {
        tally.record(
            "exceptions",
            ok,
            format_args!("a breakpoint was caught and stepped over ({caught})"),
        );
    }
}

/// **The kernel can map a page it did not have at boot, and unmap it again.**
///
/// The second self-test milestone 268 collected: it was an unnamed block in the RISC-V tour
/// (`kmap test`), and it is what the kernel stack allocator stands on, so a kernel where it fails
/// cannot spawn a thread.
///
/// **The address is computed, not a constant**, and that is the part worth keeping. A constant here
/// was the third QEMU-shaped address the VisionFive 2 caught in one bench session (2026-08-14); the
/// direct map ends at the top of whatever RAM this machine has, so one gigabyte above that clears
/// any alignment the mapper rounds to and is free on every machine rather than on one.
///
/// `phys_to_virt` rather than each architecture's own arithmetic: the three spell the direct map
/// differently (`pa | KERNEL_VA_BASE` on aarch64, `pa + KERNEL_VA_BASE` on riscv64, `pa +
/// DIRECT_MAP_BASE` on `x86_64`, whose kernel *image* base is a different constant entirely), and the
/// function is the one thing all three agree on.
fn mapping(tally: &mut Tally) {
    use paging::Flags;

    let Some(ram_top) = memory::ram_regions()
        .map(|(start, size)| start + size)
        .max()
    else {
        tally.record(
            "mapping",
            false,
            format_args!("this machine described no RAM, so there is no free address to try"),
        );
        return;
    };
    // One gigabyte above the direct map's last byte: past everything `mmu::init` claimed, and
    // inside the kernel half on all three.
    let va = arch::mmu::phys_to_virt(ram_top + (1 << 30));
    let Some(frame) = memory::alloc() else {
        tally.record(
            "mapping",
            false,
            format_args!("the allocator had no frame to map at {va:#x}"),
        );
        return;
    };
    let pa = frame.addr();

    if let Err(e) = arch::mmu::map_page(va, pa, Flags::kernel_data()) {
        tally.record(
            "mapping",
            false,
            format_args!("map_page({va:#x} -> {pa:#x}) refused: {e:?}"),
        );
        return;
    }
    let translated = arch::mmu::translate(va);
    // SAFETY: the line above mapped this VA read/write in the kernel's own tables, and nothing else
    // in the kernel names it.
    unsafe { (va as *mut u64).write_volatile(WITNESS) };
    // SAFETY: the same VA the write above used, still mapped read/write.
    let read_back = unsafe { (va as *const u64).read_volatile() };
    let unmapped = arch::mmu::unmap_page(va);

    let ok = translated.map(|(p, _)| p) == Some(pa)
        && read_back == WITNESS
        && unmapped == Ok(pa)
        && arch::mmu::translate(va).is_none();
    if ok {
        tally.record(
            "mapping",
            ok,
            format_args!("{va:#x} -> {pa:#x}, read back {read_back:#x}, unmapped"),
        );
        memory::free(frame);
    } else {
        tally.record(
            "mapping",
            ok,
            format_args!(
                "{va:#x} -> {pa:#x}: translate {translated:x?}, read back {read_back:#x}, unmap {unmapped:x?}"
            ),
        );
        // Deliberately leaked rather than freed: a frame whose mapping state is unknown must not go
        // back into the pool, where the next allocation would inherit whatever went wrong.
    }
}

/// **How many reads of an unchanged counter mean it has stopped.**
///
/// Every counter this kernel reads is free-running and fast: `CNTVCT_EL0` at 62.5 MHz on QEMU's
/// aarch64, `rdtime` at 10 MHz on QEMU's riscv64 and 4 MHz on the JH7110, the TSC on `x86_64`. The
/// slowest of those changes every 250 ns, and a million reads take far longer than that on any core
/// this runs on, so a healthy counter cannot read the same value this many times running. Counted
/// in reads rather than in time for the obvious reason: time is the thing in doubt.
const STALLED_READS: u32 = 1_000_000;

/// **The free-running counter, read by something that notices when it stops.**
///
/// The two waits below are bounded by the counter, which is right on a working machine (it makes
/// the number mean the same thing on a 62 MHz generic timer and a 2 GHz TSC) and was a hang on a
/// broken one. This makes the broken case an answer instead.
struct Counter {
    last: u64,
    unchanged: u32,
}

impl Counter {
    fn new() -> Self {
        Self {
            last: timer::now(),
            unchanged: 0,
        }
    }

    /// The counter now, or `None` once it has read the same value [`STALLED_READS`] times running.
    fn read(&mut self) -> Option<u64> {
        let now = timer::now();
        if now == self.last {
            self.unchanged += 1;
            if self.unchanged >= STALLED_READS {
                return None;
            }
        } else {
            self.last = now;
            self.unchanged = 0;
        }
        Some(now)
    }
}

/// The value written through the fresh mapping and read back. `0xc0ffee` is the RISC-V tour's, kept
/// so a board transcript from before milestone 268 and one from after it read the same.
const WITNESS: u64 = 0xc0ffee;

/// **A frame goes out of the allocator and comes back**, and the accounting agrees at both ends.
///
/// The cheapest check here and the one with the widest blast radius: every other allocation in the
/// kernel, including the page tables the check above builds, comes through this. What it catches is
/// a bitmap whose free and used counts have drifted apart, which is a leak that reports itself as
/// plenty of memory right up until there is none.
fn frames(tally: &mut Tally) {
    let Some(before) = memory::stats() else {
        tally.record(
            "frames",
            false,
            format_args!("the frame allocator is not up"),
        );
        return;
    };
    let Some(frame) = memory::alloc() else {
        tally.record(
            "frames",
            false,
            format_args!("no frame available ({} free)", before.free()),
        );
        return;
    };
    let addr = frame.addr();
    let held = memory::stats().map(|s| s.used);
    memory::free(frame);
    let after = memory::stats().map(|s| s.used);

    let ok = held == Some(before.used + 1) && after == Some(before.used);
    tally.record(
        "frames",
        ok,
        format_args!(
            "one frame out and back at {addr:#x}; used {} -> {:?} -> {:?}",
            before.used, held, after
        ),
    );
}

/// **The counter advances and the tick interrupt arrives.**
///
/// Two claims in one window, and they are different claims. A free-running counter that moves says
/// the time source exists; a tick count that moves says the interrupt controller routes its
/// interrupt to this core, the core has interrupts unmasked, and the handler runs. The second is
/// what preemption stands on, and the first is what everything that waits stands on, including the
/// wait below.
///
/// **Bounded by the counter rather than by an iteration count**, which is what makes the number
/// mean the same thing on a 62 MHz aarch64 generic timer, a 10 MHz RISC-V `mtime` and a 2 GHz TSC.
///
/// **It spins rather than sleeping, and reads through [`Counter`]**, and both are the fix for the
/// hang this module's `BUGS` used to record. The loop was `wait_for_interrupt` until the counter
/// passed the window, so a counter that never advanced sat here forever, and so did a machine whose
/// interrupts never arrived, because the sleep had nothing to wake it. Spinning with interrupts on
/// still lets the tick arrive, which is half of what this check measures.
fn timer(tally: &mut Tally) {
    let hz = timer::frequency();
    let mut counter = Counter::new();
    let start = counter.last;
    let ticks_before = timer::ticks();
    // 20 ms: two tick periods at the 100 Hz `TICK_HZ` every architecture uses, so a single missed
    // tick does not read as a broken interrupt path.
    let window = hz / 50;
    let advanced = loop {
        let Some(now) = counter.read() else {
            tally.record(
                "timer",
                false,
                format_args!(
                    "the counter stopped at {:#x}: {STALLED_READS} reads without a change",
                    counter.last
                ),
            );
            return;
        };
        let advanced = now.wrapping_sub(start);
        if advanced >= window {
            break advanced;
        }
        core::hint::spin_loop();
    };
    let ticks = timer::ticks() - ticks_before;

    let ok = advanced >= window && ticks >= 1;
    tally.record(
        "timer",
        ok,
        format_args!(
            "the {}MHz counter advanced {advanced} and {ticks} tick(s) arrived in ~20ms",
            hz / 1_000_000
        ),
    );
}

/// **A thread the kernel spawned runs, and carries what it captured.**
///
/// The context switch, end to end: `spawn` builds a thread out of the kernel's own budget,
/// `switch_to` leaves this stack and the trampoline arrives on the new one, the closure runs with
/// the environment it was built with, and the thread exits back into the scheduler. A kernel that
/// fails this cannot run a driver, a service or a process.
///
/// **Clock-bounded rather than yield-bounded**, which the RISC-V tour learned the hard way: the
/// secondaries are online by this point, so placement can put the new thread on another core, and a
/// fixed count of `yield_now` on this core elapses in microseconds, long before another core has
/// scheduled it. Boot 12 on the VisionFive 2 printed "1 of 2" and boot 13 "0 of 2" against a
/// yield-bounded wait (notes/visionfive2.md).
fn scheduler(tally: &mut Tally) {
    static SAW: AtomicU64 = AtomicU64::new(0);

    // Distinctive rather than 1: a zero-initialized static that happened to be read as "it ran"
    // would pass this check on a kernel where nothing ran at all.
    const CAPTURED: u64 = 0x0268_0268_0268_0268;

    SAW.store(0, Ordering::SeqCst);
    let Some(tid) = sched::spawn(move || SAW.store(CAPTURED, Ordering::SeqCst)) else {
        tally.record(
            "scheduler",
            false,
            format_args!("the kernel could not spawn a thread at all"),
        );
        return;
    };

    // Two seconds is far beyond any honest completion and is the same bound the RISC-V tour
    // settled on; the point of the deadline is that the boot continues rather than that the number
    // is tight. Read through [`Counter`], so a stopped counter ends the wait instead of making the
    // deadline unreachable.
    let mut counter = Counter::new();
    let deadline = counter.last + 2 * timer::frequency();
    while SAW.load(Ordering::SeqCst) != CAPTURED {
        match counter.read() {
            Some(now) if now < deadline => sched::yield_now(),
            _ => break,
        }
    }

    let saw = SAW.load(Ordering::SeqCst);
    let ok = saw == CAPTURED;
    if ok {
        tally.record(
            "scheduler",
            ok,
            format_args!("thread {tid:?} ran and carried its captured state ({saw:#x})"),
        );
    } else {
        tally.record(
            "scheduler",
            ok,
            format_args!("thread {tid:?} did not run within 2s (saw {saw:#x})"),
        );
    }
}
