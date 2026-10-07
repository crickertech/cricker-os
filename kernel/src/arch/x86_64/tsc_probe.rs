//! **Does the TSC tick at a constant rate under QEMU TCG?** A temporary measurement, not a feature.
//!
//! `kernel/src/arch/x86_64/timer.rs` measures the TSC's rate once at boot and everything
//! time-derived on x86 flows from that one number, while TCG refuses to advertise
//! `CPUID.80000007H:EDX[8]` (the invariant-TSC bit) under any invocation. So nothing on this
//! machine promises the rate is constant, and nothing had ever checked. This module checks, by
//! comparing the TSC against a clock QEMU drives from a different source.
//!
//! # The independent clock, and why the obvious one is circular
//!
//! The PIT is **not** usable as the reference, because `timer::init_frequency` calibrates against
//! it: an agreement between the two would be the calibration agreeing with itself. Worse, in QEMU
//! both the i8254 and the TSC are driven from `QEMU_CLOCK_VIRTUAL`, so they are the same clock
//! wearing two device models.
//!
//! The **CMOS RTC** is driven from `rtc_clock`, which is `QEMU_CLOCK_HOST` by default: host wall
//! time, advanced by a host timer rather than by the vCPU. That makes it the one clock in the
//! machine that is independent of the TSC by construction, and it is the reference here. Its
//! resolution is one second, which is coarse, so this does not *read* it as a time: it watches the
//! seconds register for an **edge** and timestamps the TSC there. The error per edge is then one
//! poll-loop iteration rather than one second, and the loop's own period is measured and reported
//! so the bound is a number rather than an assumption.
//!
//! # What is varied, and why that is the discriminating experiment
//!
//! The hypothesis worth killing first is *where TCG gets guest time from*. If it derives the TSC
//! from host wall time, the rate is constant by construction whatever the guest does. If it derives
//! it from the emulated instruction stream, the rate moves with how fast emulation is going.
//!
//! So alternate windows burn two workloads with wildly different host cost per guest instruction:
//!
//! - **`alu`**: a register-only loop. Many guest instructions, very cheap to emulate.
//! - **`port`**: `out 0x80` in a loop (the POST diagnostic port, ignored by QEMU and by every real
//!   chipset). Few guest instructions, each one a TCG exit into device emulation, so it is orders
//!   of magnitude more host time per guest instruction.
//!
//! Every window ends with the same tight RTC poll, so edge detection is identical across them and
//! only the burn differs. Under an instruction-derived clock the two window kinds report different
//! implied frequencies. Under a host-time-derived clock they report the same one.

use super::port::out8;
use super::timer;
use crate::{print, println};

/// The CMOS index port.
const CMOS_INDEX: u16 = 0x70;
/// The CMOS data port.
const CMOS_DATA: u16 = 0x71;
/// The CMOS seconds register.
const CMOS_SECONDS: u8 = 0x00;
/// The POST diagnostic port. Writes are swallowed by QEMU and by real chipsets alike; this is the
/// classic "I/O delay" write, used here as a guest instruction that costs a lot of host time.
const POST_PORT: u16 = 0x80;

/// How many RTC seconds one measurement window spans.
const WINDOW_SECONDS: u64 = 4;
/// How many windows to run. Eight windows of four seconds is about half a minute of boot.
const WINDOWS: u64 = 8;

/// Status register B.
const CMOS_STATUS_B: u8 = 0x0B;
/// Status register B, bit 2: the time fields are binary (set) rather than BCD (clear).
const STATUS_B_BINARY: u8 = 1 << 2;

/// Read one CMOS register.
fn cmos(reg: u8) -> u8 {
    // SAFETY: 0x70/0x71 are the CMOS index/data ports on every PC-compatible machine, driven only
    // from ring 0, and nothing in userspace can name them: §121 (what a device capability is when
    // the device has no page: x86 port I/O) keeps the legacy ports kernel-side. This is the same
    // access `arch::x86_64::rtc` makes.
    unsafe {
        out8(CMOS_INDEX, reg);
        super::port::in8(CMOS_DATA)
    }
}

/// The seconds register **decoded**, 0..=59.
///
/// The decode is not a detail: QEMU's CMOS is BCD by default, so the raw bytes step 0x09 -> 0x10
/// across every decade and a subtraction of two raw readings overstates the elapsed seconds. The
/// first version of this harness subtracted the raw bytes and reported windows of ten seconds that
/// had taken four.
fn seconds() -> u8 {
    let raw = cmos(CMOS_SECONDS);
    if BINARY.load(core::sync::atomic::Ordering::Relaxed) {
        raw
    } else {
        (raw & 0x0F) + 10 * (raw >> 4)
    }
}

/// Whether this CMOS reports binary rather than BCD, read once by [`probe`]. A static rather than a
/// second port read per poll, because every poll is on the edge-detection path and the resolution
/// bound this harness reports is the cost of one poll.
static BINARY: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

/// Spin until the CMOS seconds register changes, and return the TSC reading taken immediately
/// after the change was observed, along with the number of poll iterations it took.
fn wait_for_edge(previous: u8) -> (u64, u8, u64) {
    let mut iterations = 0u64;
    loop {
        let now = seconds();
        if now != previous {
            return (timer::now(), now, iterations);
        }
        iterations += 1;
    }
}

/// A register-only burn: many guest instructions, cheap for TCG to emulate.
fn burn_alu(rounds: u64) -> u64 {
    let mut x = 1u64;
    for _ in 0..rounds {
        x = x.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        core::hint::black_box(&mut x);
    }
    x
}

/// A port-I/O burn: few guest instructions, each an exit into device emulation, so it is expensive
/// for TCG in host time.
fn burn_port(rounds: u64) {
    for _ in 0..rounds {
        // SAFETY: 0x80 is the POST diagnostic port. Writes to it are ignored by QEMU and by every
        // chipset this kernel can run on, and nothing in this tree drives it.
        unsafe { out8(POST_PORT, 0) };
    }
}

/// Run the measurement and print one line per window. Called from the x86 tour when the kernel is
/// built with the `tsc_probe` feature; absent from every other build.
pub fn probe() {
    BINARY.store(
        cmos(CMOS_STATUS_B) & STATUS_B_BINARY != 0,
        core::sync::atomic::Ordering::Relaxed,
    );
    let hz = timer::frequency();
    println!(
        "tscprobe: calibrated tsc {hz} Hz (source {}), reference CMOS RTC seconds edge",
        timer::frequency_source(),
    );

    // Every calibration window, not just the one that won. This is what turns the probe from "is
    // the calibration right" into "is the *estimator* right": the truth is at or below the
    // smallest of these, so a run where the smallest is close to the RTC-measured rate and the
    // largest is far above it is the min-of-N argument being demonstrated rather than asserted.
    // See `timer::calibration_windows`, and milestone 571 (the x86 boot calibrates the TSC once, and
    // can be wrong by 4x), in
    // design/roadmap/0571-the-x86-boot-calibrates-once-and-can-be-wrong-by-4x.md, for the sweep
    // that chose the count.
    print!("tscprobe: calibration windows");
    for window in timer::calibration().windows() {
        print!(" {window}");
    }
    println!();

    // How long one poll iteration takes, in TSC counts. This is the resolution bound on every edge
    // timestamp below: the edge happened somewhere inside the last iteration.
    let before = timer::now();
    let mut probe_seconds = seconds();
    for _ in 0..1000 {
        probe_seconds = core::hint::black_box(seconds());
    }
    let poll_cost = (timer::now() - before) / 1000;
    println!(
        "tscprobe: one rtc poll costs {poll_cost} tsc counts, so each edge is timestamped to \
         within that ({} ns at the calibrated rate)",
        poll_cost.saturating_mul(1_000_000_000) / hz.max(1),
    );

    // Sync to an edge so the first window starts aligned.
    let (mut tsc_at_edge, mut last, _) = wait_for_edge(probe_seconds);

    let mut lowest = u64::MAX;
    let mut highest = 0u64;
    let first_tsc = tsc_at_edge;
    let first_seconds = last;
    let mut reference_seconds_total = 0u64;

    for window in 0..WINDOWS {
        let alu = window % 2 == 0;
        let start = tsc_at_edge;
        let start_seconds = last;

        // Burn for most of the window, then tight-poll for the edge that ends it. The tail is one
        // tight poll in both window kinds, so edge detection is identical and only the burn differs.
        //
        // The burn's length is set by the *calibrated* rate, which is exactly the number under
        // suspicion, so it is deliberately not load-bearing: how long the burn ran does not enter
        // the result. What the window spans is counted from the RTC's own seconds register below.
        let burn_until = start + hz.saturating_mul(WINDOW_SECONDS - 1);
        let mut work = 0u64;
        while timer::now() < burn_until {
            if alu {
                core::hint::black_box(burn_alu(10_000));
            } else {
                burn_port(1_000);
            }
            work += 1;
        }

        // The edge that ends the window. `elapsed` is read off the RTC rather than assumed: the
        // burn may have crossed more or fewer seconds than intended, and an assumed count is how
        // the first version of this harness reported a 1.25 GHz TSC that does not exist.
        let current = seconds();
        let (tsc, seen, poll_iterations) = wait_for_edge(current);
        last = seen;
        tsc_at_edge = tsc;
        let elapsed = u64::from((60 + seen - start_seconds) % 60);
        reference_seconds_total += elapsed;

        let delta = tsc_at_edge - start;
        let implied = delta / elapsed.max(1);
        lowest = lowest.min(implied);
        highest = highest.max(implied);
        println!(
            "tscprobe: window {window} burn {} work {work} rtc-seconds {elapsed} delta {delta} \
             implied {implied} Hz (tail poll {poll_iterations} iterations)",
            if alu { "alu " } else { "port" },
        );
    }

    let total = tsc_at_edge - first_tsc;
    let wrapped = u64::from((60 + last - first_seconds) % 60);
    let implied = total / reference_seconds_total.max(1);
    let spread = highest - lowest;
    println!(
        "tscprobe: over {reference_seconds_total} rtc seconds (register moved {wrapped} mod 60) \
         the tsc advanced {total}, implied {implied} Hz"
    );
    println!(
        "tscprobe: per-window implied rate lowest {lowest} highest {highest} spread {spread} Hz \
         ({} ppm of the measured rate)",
        spread.saturating_mul(1_000_000) / implied.max(1),
    );
    println!(
        "tscprobe: calibrated {hz} Hz vs measured {implied} Hz, calibration error {} ppm",
        hz.abs_diff(implied).saturating_mul(1_000_000) / implied.max(1),
    );
    println!("tscprobe: done");
}
