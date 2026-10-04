//! The soaker: the workload that does not stop (milestone 219).
//!
//! `design/fatal-risks/README.md`'s fifth entry ("it cannot be made reliable on multicore, and the bugs
//! appear only on silicon") names its decisive experiment as sustained multi-core stress with the
//! load-sensitive assertions live. Nothing in this tree could sustain anything: the boot tour ran
//! its checks and the kernel halted. This is the thing that keeps running.
//!
//! # What it stresses, and why this and not something else
//!
//! **Cross-core IPC rendezvous, at the highest rate the machine will do them.** That is not a
//! guess about where the bugs are. It is where the one bug this risk has actually produced was: a
//! receiver made `Ready` with nothing delivered, on three harts of a VisionFive 2, in
//! `sched::wake_load_aware`. Every other candidate workload (a compute loop, an allocator churn, a
//! filesystem soak) exercises paths that a hundred other tests already cover; this one drives the
//! exact code the risk points at, through the real syscall boundary, from user mode.
//!
//! One round trip is `CALL` -> `RECEIVE_CAP` -> `REPLY` -> the caller waking. That is **two** block/wake
//! handshakes, and each one may place its peer on a *different* core (`sched::pick_wake_target`,
//! `place_on`, the migration inbox, the reschedule IPI). With two runnable threads per core the run
//! queues also go empty often enough that `serve_steal_request` fires, so the work-steal protocol
//! is under load too rather than merely present.
//!
//! # The waiter, and what it is for (milestone 221)
//!
//! The workload above is saturated, and milestone 219 measured that a saturated workload **never
//! crosses cores**: the `crossings` count froze inside the first second and stayed frozen through
//! two million round trips. DECISIONS 138 (*how a saturated workload is made to hand threads across
//! cores*) explains why, and calef approved the fix on 2026-09-02: a soak build signals a
//! rendezvous from `sched::on_tick`, and one role per group blocks on it through `Irq::WAIT`.
//!
//! **A waiter is deliberately the least interesting program in this file.** All of the mechanism is
//! in the kernel's wake path, which is where the one defect this risk has produced lived; the
//! waiter's job is only to be a thread that is blocked when a real interrupt handler decides where
//! to put it.
//!
//! **What crosses is the waiters, not the pairs.** Rendezvous wakes stay local whatever else is
//! happening, so the callers and responders are as pinned as they ever were. This sustains the wake
//! protocol across cores under load; it does not make the IPC workload migrate.
//!
//! # Why the pairs do not run in lockstep
//!
//! A soak that repeats one interleaving a billion times has explored one interleaving. Each worker
//! spins a small, pseudo-random number of iterations between round trips (an xorshift seeded from
//! `arg2`, which the kernel makes distinct per worker), so the phase relationship between pairs
//! drifts continuously instead of locking. That is the difference between a long run and a wide
//! one, and it costs four instructions.
//!
//! # The soaker's world
//!
//! - **slot 0**: the request endpoint. `WRITE` for a caller (it `CALL`s), `READ` for a responder
//!   (it `RECEIVE_CAP`s). Nothing else. A soaker cannot name the console, the clock, or its peer. A
//!   **waiter** (milestone 221) holds an `Irq` in the same slot instead, naming the tick route and
//!   nothing else, so it can `WAIT` and cannot touch its group's IPC at all.
//! - **the shared page**, mapped read/write at [`soak_page::VA`], where it publishes its own two
//!   counters and reads nothing. The kernel is the reader.
//!
//! It never exits. That is the whole point, and it is why the kernel spawns it rather than init:
//! the operator ends the run by taking the power away or by the watcher's deadline expiring.
//!
//! # BUGS
//!
//! - **A green soak proves nothing about the concurrency being correct.** It reports a number, and
//!   the number's only use is comparison against another run. `design/roadmap/219-a-workload-that-does-not-stop.md`
//!   says this at more length and says it as the milestone's own headline caveat.
//! - **The counters are not synchronised with the kernel's read.** The kernel samples the page
//!   while workers are writing it, so a heartbeat's `rounds` total may be a few short. It is a
//!   progress measure, not an audit; a systematic error of a handful of round trips against tens of
//!   millions changes no verdict this workload can reach.
//! - **A waiter's wakes are not round trips and are counted apart from them.** Its `rounds` stays
//!   at zero forever and its `wakes` is its own field in the heartbeat, because folding the two
//!   together would make the number a later run is compared against mean something different
//!   depending on which build produced it.
//! - **A waiter whose `WAIT` is refused spins instead of reporting.** It cannot print and has no
//!   channel to say anything on, so it stops counting and lets the kernel's stall check speak for
//!   it one beat later. The report then says "stalled" where "refused" would be more use, which is
//!   the same limitation the role above it already has.
//! - **A responder cannot detect a caller that stops calling.** It simply blocks in `RECEIVE_CAP`
//!   forever, and its own counter stops moving, which is what the kernel's stall check sees. The
//!   report cannot say which half of the pair wedged; the thread dump can.
//!
//! Name: ratified 2026-09-13 (calef, working the unratified worklist). Coined by milestone 219's
//! lane on 2026-09-01 and kept unchanged, which on that day's evidence is unusual: six bare agent
//! nouns were expanded with a qualifier in the same session (`budgeter`, `worker`, `heeder`,
//! `spinner`, `flaky`, `chatty`), because none of them said what it acted on. This one does.
//!
//! **"Soak" is a term of art used correctly**, which is the protected class AGENTS.md calls the
//! best name available: a soak test runs a system under sustained load long enough to surface what
//! does not appear quickly, and that is this program. Contrast `flaky`, ruled the same day, which
//! borrowed the field's word for intermittent failure while failing deterministically; a borrowed
//! word is the best name when it is true and the worst when it is not.
//!
//! It is also **this tree's own vocabulary already**, at 947 occurrences of the stem:
//! `crates/soak_page`, `kernel/src/soak.rs`, `--features soak_test`, `board_console`'s soak census and
//! the captured board logs. Renaming the program would leave it the odd member of a family it
//! anchors.
//!
//! **And the umbrella is honest across all four roles**, which is the test `chatty` and `outlaw`
//! failed the same day. `ROLE_RESPONDER`, `ROLE_CALLER`, `ROLE_GRINDER` and `ROLE_WAITER` are all
//! soakers, and this file's own comments say so ("this soaker calls", "this soaker computes",
//! "this soaker blocks"), where `outlaw`'s round-trip role broke no rule and `chatty`'s attacker
//! was not chatty.
//!
//! Refused `ipc_soaker`, the qualifier-plus-agent-noun shape ratified six times that day, for
//! being false: the grinder computes and never blocks and the waiter blocks on a tick route, so
//! neither is IPC. Refused `stressor`, because "stress" in this tree already names
//! `script/repeat-under-load`'s *induced host load*, which is a different thing pointed the other
//! way. Refused `churner` for naming the motion rather than the purpose.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`.
#![allow(missing_docs)]
#![no_main]

use user_mode_runtime::mapped_window::{self, MappedWindow};
use user_mode_runtime::{call, irq_wait, receive_request, reply};

/// The one capability a soaker holds: the request endpoint, in slot 0.
const ENDPOINT: u64 = 0;

/// The one capability a **waiter** holds: the tick route's `Irq`, also in slot 0 (milestone 221).
///
/// The same slot, a different object, and that is the point rather than a collision: a waiter can
/// no more `CALL` its group's endpoint than a caller can `WAIT` on the tick route, because neither
/// holds the other's capability. One slot each is the least authority the role needs.
const TICK_IRQ: u64 = 0;

/// `arg0`: this soaker answers calls.
const ROLE_RESPONDER: u64 = 0;
/// `arg0`: this soaker calls.
const ROLE_CALLER: u64 = 1;
/// `arg0`: this soaker computes and never blocks.
const ROLE_GRINDER: u64 = 2;
/// `arg0`: this soaker blocks on the tick route and counts its wakes (milestone 221).
const ROLE_WAITER: u64 = 3;

// SAFETY: the kernel's `soak::run` maps one page read/write at `soak_page::VA` in every worker's
// address space before starting it, and `soak_page`'s host tests prove every offset this program
// uses lands inside that page.
const PAGE: MappedWindow = unsafe { MappedWindow::new(soak_page::VA, mapped_window::PAGE) };

/// The soaker's entry.
///
/// `x0` is the role, `x1` the worker index (which slot of the shared page is this one's), and `x2`
/// the seed for the jitter. The kernel gives every worker a distinct index and a distinct seed.
#[unsafe(no_mangle)]
pub extern "C" fn _start(role: u64, index: u64, seed: u64) -> ! {
    let index = index as usize;
    let rounds_at = soak_page::rounds(index);
    let mismatches_at = soak_page::mismatches(index);
    let wakes_at = soak_page::wakes(index);

    // Never zero: an xorshift seeded with zero produces zeros forever, and a worker that never
    // jitters would silently drop out of the widening this program exists to do.
    let mut state = if seed == 0 {
        0x243F_6A88_85A3_08D3
    } else {
        seed
    };
    let mut rounds: u64 = 0;
    let mut wakes: u64 = 0;
    let mut mismatches: u64 = 0;
    // Start from the seed rather than from zero, so two callers sharing an index-derived sequence
    // are not sending each other's words at the same moment by construction.
    let mut sequence: u64 = seed;

    loop {
        if role == ROLE_WAITER {
            // **The whole of the waiter role** (milestone 221): block on the tick route, count the
            // wake, block again. `irq_wait` is the same `Irq::WAIT` a device driver uses, and the
            // kernel's soak build is what signals the route, from `sched::on_tick` on every core.
            //
            // No `irq_ack`. A driver acknowledges because its controller masked a line that a
            // device is still asserting; there is no line here and nothing was masked, so an ack
            // would be a real controller write for an interrupt that does not exist.
            //
            // A negative return is a refusal (`abi::Error`), and the honest response is to stop
            // counting rather than to spin: the counter stops moving, and the kernel's stall check
            // reports it at the next beat, which is the same way every other failure in this
            // workload surfaces.
            if irq_wait(TICK_IRQ) < 0 {
                loop {
                    core::hint::spin_loop();
                }
            }
            wakes = wakes.wrapping_add(1);
            PAGE.write(wakes_at, wakes);
            continue;
        }
        if role == ROLE_GRINDER {
            for _ in 0..2048 {
                core::hint::spin_loop();
            }
        } else if role == ROLE_RESPONDER {
            // `receive_request` (a `RECEIVE_CAP`), not `receive`: a CALL arrives with a reply capability,
            // and answering it is what completes the caller's parked rendezvous. A responder that
            // used `receive` would leave every caller blocked forever, which is a hang this workload
            // would then report as a finding about the kernel.
            let req = receive_request(ENDPOINT);
            if let Some(to) = req.delivered.into_reply() {
                reply(to, soak_page::answer(req.w0), 0);
            } else {
                // Nobody to answer. Counted as a mismatch rather than ignored: a CALL that arrived
                // without its reply capability is exactly the shape of defect this workload is
                // looking for, and swallowing it would be the soak lying about itself.
                mismatches = mismatches.wrapping_add(1);
                PAGE.write(mismatches_at, mismatches);
            }
        } else {
            debug_assert!(role == ROLE_CALLER);
            sequence = sequence.wrapping_add(1);
            let (got, _) = call(ENDPOINT, sequence, 0);
            if got != soak_page::answer(sequence) {
                mismatches = mismatches.wrapping_add(1);
                PAGE.write(mismatches_at, mismatches);
            }
        }

        rounds = rounds.wrapping_add(1);
        PAGE.write(rounds_at, rounds);

        // The jitter. xorshift64, four instructions, no memory traffic; the low bits decide how
        // many spins to burn before the next round trip. 0..63 is enough to slide a pair's phase
        // against its neighbours without meaningfully lowering the round-trip rate.
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        for _ in 0..(state & 0x3f) {
            core::hint::spin_loop();
        }
    }
}

user_mode_runtime::panic_handler!();
