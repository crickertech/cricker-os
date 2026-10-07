//! **Unhalted core cycles, `x86_64`** (milestone 309, the `x86_64` half of milestone 74's
//! measurement side).
//!
//! # Why this is not `rdtsc`, which is the whole reason the module exists
//!
//! [`super::timer::now()`] **is** `rdtsc`. So a `cycles_per_tick` probe on this architecture that
//! read the TSC at both ends would divide one counter by itself and print an exact `1.00`, every
//! time, on every part. That is milestone 16a's "implausibly exact 100.00" again, except
//! structural rather than emulated: no boot on real silicon would ever fix it, because the two
//! reads are the same instruction.
//!
//! The other two architectures do not have this problem, because on both of them the OS clock and
//! the cycle counter are different hardware:
//!
//! | | `arch::timer::now()` | the cycle counter |
//! |---|---|---|
//! | `aarch64` | `CNTVCT_EL0`, the architected generic timer | `PMCCNTR_EL0`, a PMU counter |
//! | `riscv64` | `rdtime`, a fixed-rate reference tick | `mcycle`/`hpmcounterN` via SBI PMU |
//! | `x86_64` | `rdtsc`, **a constant-rate counter** | `IA32_PERF_FIXED_CTR1`, **this module** |
//!
//! `IA32_PERF_FIXED_CTR1` is architectural fixed-function counter 1,
//! `CPU_CLK_UNHALTED.CORE`: cycles the core actually ran, at whatever frequency it was running,
//! not counted while it is halted. That is the same quantity riscv64's `SBI_PMU_HW_CPU_CYCLES` and
//! aarch64's `PMCCNTR_EL0` give, so with this in place the three architectures report the same
//! thing under one name. The TSC cannot answer it: on every part since Nehalem the TSC is
//! *invariant*, ticking at a fixed rate regardless of the core's frequency or idle state, which is
//! exactly what makes it a good clock and exactly what makes it not a cycle counter.
//!
//! # No authority question, and this must not create one
//!
//! [DECISIONS §139 part 3](../../../design/decisions/0139-cycle-counter-authority.md) records that
//! `x86_64`'s TSC is already ambient: `CR4.TSD` is clear at reset and this kernel never writes it, so
//! ring 3 can `rdtsc` and the negative half of the cycle-counter grant test skips here with a
//! reason. **This module does not change that and must not.** It enables a counter the kernel reads
//! with `rdmsr`, which is ring 0 only. `CR4.PCE` stays clear, so `rdpmc` from ring 3 still faults
//! and there is no new ambient counter to grant or to revoke.
//!
//! # The interface, from the specification
//!
//! Intel SDM Volume 3B, Chapter 20 (Performance Monitoring), "Architectural Performance
//! Monitoring". Three registers and one `CPUID` leaf:
//!
//! 1. **`CPUID` leaf `0x0A`** says whether any of this exists. `EAX[7:0]` is the architectural
//!    performance monitoring **version**; fixed-function counters arrive at version 2. `EDX[4:0]`
//!    is how many fixed-function counters there are and `EDX[12:5]` how wide they are.
//! 2. **`IA32_FIXED_CTR_CTRL` (`0x38D`)** enables counting, four bits per fixed counter. Counter 1
//!    owns bits 4..7: the low two are "count in ring 0" and "count in ring > 0".
//! 3. **`IA32_PERF_GLOBAL_CTRL` (`0x38F`)** is the master switch; fixed counter *i* is bit `32+i`.
//!    Both have to be set, and a counter enabled in only one of them silently counts nothing.
//! 4. **`IA32_PERF_FIXED_CTR1` (`0x30A`)** is the counter itself, read with one `rdmsr`.
//!
//! # Why the `CPUID` gate comes first and is not optional
//!
//! `rdmsr` on an MSR the part does not implement is a `#GP`, and this kernel has no recovery path
//! for a probe that could have asked. The leaf is therefore checked before any of the three MSRs is
//! touched, which is the same discipline `isa::draw_random_seed` keeps for `RDSEED` one file over.
//! It is also what makes this safe under an emulator that models no PMU at all: QEMU zeroes leaf
//! `0x0A` unless its `pmu` property is on, so [`init`] reads a version of 0, stops, and never
//! issues an `rdmsr` that would fault.
//!
//! # Why a counter is checked before it is believed
//!
//! Because a counter can be present and useless, and this module's whole value is saying so rather
//! than printing a number. `arch::riscv64::pmu` learned this from firmware handing back a counter
//! QEMU models as a constant zero; the same shape is available here two ways, and both are
//! detected:
//!
//! - **Stuck.** Read twice across a timed spin and unmoved. A counter of executed cycles cannot do
//!   that on a core that is executing.
//! - **In step with the TSC.** A hypervisor that satisfies the read by handing back the *same*
//!   counter produces a delta bit-for-bit equal to the TSC's over the same window, and a probe that
//!   believed it would print the exact `1.00` this module exists to avoid. Refused, with its own
//!   reason, so a reader can tell "this machine has no cycle counter" from "this machine's cycle
//!   counter is the clock wearing a hat".
//!
//! **The degradation this module must never do is fall back to the TSC.** A probe that quietly did
//! would reproduce the artifact exactly, in a line somebody would later quote as a fact about a
//! machine. There is no fallback path here; [`cycles`] answers `None` and the caller prints a
//! reason.
//!
//! # BUGS
//!
//! - **Nothing here has been run on silicon.** Every outcome this module can report today is a fact
//!   about QEMU. `xenon` (milestone 87, this project's x86 machine) is the one that would produce a
//!   number. It **has** booted nife, on 2026-09-04 under its own UEFI firmware, and stopped in the
//!   mapper at `mmu.rs`'s `AlreadyMapped` before reaching anything this module touches; that cause
//!   is fixed on `main` and the next boot resumes one line further on. So the honest statement is
//!   that this counter has never been read on silicon, not that the machine has never run.
//!   See design/roadmap/0309-x86-64-core-cycles.md and notes/x86-uefi-boot.md.
//! - **The in-step check is bit-exact equality, and that is a deliberate under-detection.** A real
//!   core pegged at exactly its base frequency has core cycles and TSC ticks at the same *rate*, so
//!   an approximate band would refuse a legitimate counter on a legitimate machine. Two independent
//!   counters read by two different instructions a few tens of cycles apart do not produce equal
//!   deltas over a ten-millisecond window; one counter read twice does. So equality catches
//!   aliasing and a 1.00 *ratio* alone does not, and a hypervisor that aliased the two with an
//!   offset would pass this check and print `1.00`. The printed line says what it counts, which is
//!   the defence that does not depend on a heuristic.
//! - **The boot CPU only.** These MSRs are per-logical-processor: a secondary that never ran
//!   [`init`] has fixed counter 1 disabled and would read zero. [`init`] runs once, on the boot
//!   CPU, and the one consumer is a single-threaded bench probe. A per-CPU record is real work and
//!   belongs with a caller that needs it.
//! - **The counter is never stopped.** It runs for the life of the boot, which is correct for a
//!   free-running counter only ever read as a difference, and is the same call
//!   `arch::riscv64::pmu` makes.
//! - **`IA32_PERF_GLOBAL_CTRL` is read-modify-written, not assigned.** Nothing else in this kernel
//!   programs a performance counter, so the bits preserved are firmware's. Preserving them is the
//!   conservative choice and costs one `rdmsr`; if a second consumer ever appears, the arbitration
//!   between them is its problem to raise and not this module's to assume.

use core::sync::atomic::{AtomicU8, AtomicU32, AtomicU64, Ordering};

use super::{isa, read_msr, write_msr};

/// `IA32_PERF_FIXED_CTR1`, architectural fixed-function counter 1: `CPU_CLK_UNHALTED.CORE`.
const IA32_PERF_FIXED_CTR1: u32 = 0x30A;

/// `IA32_FIXED_CTR_CTRL`, four bits per fixed counter.
const IA32_FIXED_CTR_CTRL: u32 = 0x38D;

/// `IA32_PERF_GLOBAL_CTRL`, the master enable. Fixed counter *i* is bit `32 + i`.
const IA32_PERF_GLOBAL_CTRL: u32 = 0x38F;

/// Counter 1's enable field in [`IA32_FIXED_CTR_CTRL`]: bits 4 and 5, "count in ring 0" and "count
/// in ring > 0".
///
/// **Both rings, deliberately.** The quantity the other two architectures report is cycles the core
/// ran, without regard to privilege, and a counter that stopped at the ring boundary would make
/// `x86_64`'s number mean something different from theirs, which is the exact confusion milestone 309
/// exists to remove. Enabling ring 3 *counting* is not ring 3 *access*: reading still needs `rdmsr`,
/// or `rdpmc` with `CR4.PCE` set, and this kernel sets neither. See this module's header.
const FIXED_CTR1_BOTH_RINGS: u64 = 0b11 << 4;

/// Counter 1's field, all four bits, for masking it out before the enable is written.
const FIXED_CTR1_FIELD: u64 = 0b1111 << 4;

/// Fixed counter 1's bit in [`IA32_PERF_GLOBAL_CTRL`].
const GLOBAL_CTRL_FIXED_CTR1: u64 = 1 << 33;

/// The architectural performance monitoring `CPUID` leaf.
const CPUID_PERFMON_LEAF: u32 = 0x0A;

/// The version at which fixed-function counters exist at all (SDM Vol. 3B §20.2.3).
const PERFMON_VERSION_WITH_FIXED_COUNTERS: u32 = 2;

/// Fixed counter **1** is the second one, so a part must report at least this many.
const FIXED_COUNTERS_NEEDED: u32 = 2;

/// **Why there is or is not a usable cycle counter**, as one value, decided once by [`init`].
///
/// The same vocabulary `arch::riscv64::pmu::CycleCounter` carries, and for the same reason: "no
/// counter" has several distinguishable causes, they want different fixes, and the machine where
/// that matters is one nobody can attach a debugger to. The members differ because the mechanisms
/// differ; the discipline of building the answer into the boot line rather than telling a reader to
/// go and add a print is the part that carries across.
///
/// Ordered so the discriminant is the stored value and `Unknown` is zero, which is what an
/// unreached [`init`] leaves behind.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum CycleCounter {
    /// [`init`] has not run.
    Unknown = 0,
    /// `CPUID` does not reach leaf `0x0A`, or the leaf reports version 0. This part has no
    /// architectural performance monitoring, so there is nothing to ask and nothing safe to read.
    /// **This is what QEMU answers unless its `pmu` property is on**, and it is the reason [`init`]
    /// never faults there.
    NoPerfmonLeaf,
    /// Architectural performance monitoring exists, but at a version below 2 or with fewer than two
    /// fixed-function counters, so `IA32_PERF_FIXED_CTR1` is not implemented.
    NoFixedCounter,
    /// **Enabled, read twice across a timed spin, and unmoved.** A counter of executed cycles cannot
    /// do that on a core that is executing, so it is refused rather than reported as zero. This is
    /// the x86 twin of riscv64's `Stuck`, and it is the outcome a counter modelled as a constant
    /// produces.
    Stuck,
    /// **Enabled, moving, and moving by exactly what the TSC moved by.** One counter answering two
    /// reads: refused, because believing it would print the structural `1.00` this module exists to
    /// avoid. See the header's BUGS on why the test is bit-exact.
    InStepWithTheTsc,
    /// Implemented, enabled, and advancing independently of the TSC.
    Running,
}

/// [`CycleCounter`] as a byte. An atomic rather than a lock: written once by [`init`] on the boot
/// CPU before anything reads it, read-only afterwards. Nothing here nests with another lock, so it
/// earns no rank in `sync::rank`.
static OUTCOME: AtomicU8 = AtomicU8::new(CycleCounter::Unknown as u8);

/// How wide the fixed counters are, in bits, as the leaf reported. Written as soon as `CPUID` names
/// it, so a boot line reporting a *refused* counter can still say what was found; meaningful only
/// alongside [`outcome`]. A counter narrower than 64 bits wraps, and [`cycles_delta`] is what knows
/// where.
static COUNTER_BITS: AtomicU32 = AtomicU32::new(0);

/// The architectural performance monitoring version the leaf reported, for the boot line.
static PERFMON_VERSION: AtomicU32 = AtomicU32::new(0);

/// The two deltas the in-step check compared, kept for the boot line: on the one outcome where the
/// diagnosis is "these two numbers were identical", printing them is the whole diagnostic.
static CHECK_CYCLES: AtomicU64 = AtomicU64::new(0);
static CHECK_TSC: AtomicU64 = AtomicU64::new(0);

/// **Find the architectural fixed cycle counter, enable it, check it is counting something of its
/// own, and remember whether to believe it.**
///
/// Called once from `kernel_main`'s `x86_64` arm, after the TSC has been calibrated: the checks below
/// need a clock to spin against, and the clock they spin against is the very counter they are
/// checking the answer is not. Silent and harmless on a part with no performance monitoring:
/// [`cycles`] then answers `None` forever, which is the honest answer and not an error.
///
/// Every way of failing is recorded in [`outcome`] and printed by [`print_summary`], because on the
/// machine where this matters nobody can attach a debugger.
pub fn init() {
    OUTCOME.store(CycleCounter::Unknown as u8, Ordering::Relaxed);
    COUNTER_BITS.store(0, Ordering::Relaxed);
    PERFMON_VERSION.store(0, Ordering::Relaxed);

    // The gate that keeps every `rdmsr` below from being a `#GP`. Leaf 0x0A is only meaningful if
    // the part answers that far: `CPUID` for a leaf above the maximum returns some *other* leaf's
    // data rather than refusing, so an unchecked read would decode an unrelated register as a
    // version number.
    if isa::get().max_leaf < CPUID_PERFMON_LEAF {
        OUTCOME.store(CycleCounter::NoPerfmonLeaf as u8, Ordering::Release);
        return;
    }
    let leaf = isa::cpuid(CPUID_PERFMON_LEAF);
    let version = leaf.eax & 0xFF;
    PERFMON_VERSION.store(version, Ordering::Relaxed);
    if version == 0 {
        // QEMU's own answer with `pmu` off: the leaf exists and is all zeroes.
        OUTCOME.store(CycleCounter::NoPerfmonLeaf as u8, Ordering::Release);
        return;
    }

    let fixed_count = leaf.edx & 0x1F;
    let bits = (leaf.edx >> 5) & 0xFF;
    COUNTER_BITS.store(bits, Ordering::Relaxed);
    if version < PERFMON_VERSION_WITH_FIXED_COUNTERS
        || fixed_count < FIXED_COUNTERS_NEEDED
        || bits == 0
    {
        OUTCOME.store(CycleCounter::NoFixedCounter as u8, Ordering::Release);
        return;
    }

    // Enable, in the order the SDM requires: the per-counter control first, then the master
    // switch. A counter enabled in only one of the two counts nothing and reports no error, which
    // is exactly the failure the `Stuck` outcome below would then (correctly, but uselessly)
    // report.
    //
    // SAFETY: `CPUID` leaf 0x0A reported architectural performance monitoring version >= 2 with at
    // least two fixed-function counters, which is precisely the SDM's statement that these three
    // MSRs are implemented. The values written enable counting and change nothing else: the
    // read-modify-write preserves every field this kernel does not own, and neither register can
    // change the mode the machine is in.
    unsafe {
        let ctrl = read_msr(IA32_FIXED_CTR_CTRL);
        write_msr(
            IA32_FIXED_CTR_CTRL,
            (ctrl & !FIXED_CTR1_FIELD) | FIXED_CTR1_BOTH_RINGS,
        );
        let global = read_msr(IA32_PERF_GLOBAL_CTRL);
        write_msr(IA32_PERF_GLOBAL_CTRL, global | GLOBAL_CTRL_FIXED_CTR1);
    }

    // Does it count, and does it count something of its own? One window, both questions, because
    // they are the same two reads. The window is the TSC's, which is the one clock this kernel has
    // here that does not depend on the answer: short enough not to be felt in a boot, long enough
    // that any real core has run through millions of cycles.
    let ticks = check_window_ticks();
    let (c0, t0) = (read_counter(), super::timer::now());
    while super::timer::now().wrapping_sub(t0) < ticks {
        core::hint::spin_loop();
    }
    let (t1, c1) = (super::timer::now(), read_counter());

    let cycles = cycles_delta(c0, c1);
    let elapsed = t1.wrapping_sub(t0);
    CHECK_CYCLES.store(cycles, Ordering::Relaxed);
    CHECK_TSC.store(elapsed, Ordering::Relaxed);

    if cycles == 0 {
        OUTCOME.store(CycleCounter::Stuck as u8, Ordering::Release);
        return;
    }
    if cycles == elapsed {
        // Bit-for-bit, over millions of ticks. That is one counter answering both reads, not two
        // counters that happen to agree; see this module's BUGS on why the test is equality.
        OUTCOME.store(CycleCounter::InStepWithTheTsc as u8, Ordering::Release);
        return;
    }

    // Last, and with Release: a reader that sees `Running` must see the width written before it.
    // Every read is Acquire on this one location for the same reason.
    OUTCOME.store(CycleCounter::Running as u8, Ordering::Release);
}

/// How long [`init`]'s check spins, in TSC ticks: ten milliseconds where the TSC's rate is known,
/// and a flat ten million where it is not.
///
/// The fallback exists because [`init`]'s position in the boot is a choice and not a law. It is
/// called after `timer::init_frequency`, so the calibrated number is there; a boot that ever moved
/// it earlier would otherwise take `frequency`'s panic, and a probe is not worth a dead machine.
/// Ten million ticks is between three and ten milliseconds on anything this runs on.
fn check_window_ticks() -> u64 {
    const FALLBACK: u64 = 10_000_000;
    match super::timer::frequency_checked() {
        Some(hz) => hz / 100,
        None => FALLBACK,
    }
}

/// One `rdmsr` of [`IA32_PERF_FIXED_CTR1`]. Only ever called once [`init`] has enabled the counter,
/// which is what makes the read safe.
fn read_counter() -> u64 {
    // SAFETY: reached only after [`init`] confirmed through `CPUID` leaf 0x0A that this MSR is
    // implemented. An MSR read has no architectural side effect.
    unsafe { read_msr(IA32_PERF_FIXED_CTR1) }
}

/// The difference between two counter reads, wrapped at the counter's own width.
///
/// **Fixed counters are usually narrower than 64 bits** (48 on most parts, which `CPUID` leaf 0x0A
/// `EDX[12:5]` states), so `c1 - c0` across a wrap is a number with the top bits set rather than a
/// small positive delta. Masking to the reported width is what makes a wrapped difference come out
/// right, and it is correct for any window shorter than one full wrap: 48 bits is about a day at
/// 3 GHz, and the windows here are milliseconds.
fn cycles_delta(c0: u64, c1: u64) -> u64 {
    let bits = COUNTER_BITS.load(Ordering::Relaxed);
    let raw = c1.wrapping_sub(c0);
    if bits == 0 || bits >= 64 {
        raw
    } else {
        raw & ((1u64 << bits) - 1)
    }
}

/// **Why there is or is not a usable cycle counter on this CPU.** See [`CycleCounter`].
pub fn outcome() -> CycleCounter {
    match OUTCOME.load(Ordering::Acquire) {
        x if x == CycleCounter::NoPerfmonLeaf as u8 => CycleCounter::NoPerfmonLeaf,
        x if x == CycleCounter::NoFixedCounter as u8 => CycleCounter::NoFixedCounter,
        x if x == CycleCounter::Stuck as u8 => CycleCounter::Stuck,
        x if x == CycleCounter::InStepWithTheTsc as u8 => CycleCounter::InStepWithTheTsc,
        x if x == CycleCounter::Running as u8 => CycleCounter::Running,
        _ => CycleCounter::Unknown,
    }
}

/// **Unhalted core cycles on this CPU**, or `None` when there is no counter worth believing.
///
/// One `rdmsr`, no trap and no call, so a caller may put this on either side of the thing it is
/// measuring.
///
/// **The value is only meaningful as a difference**, and only through [`cycles_delta`] if the
/// counter is narrow. It is a count of cycles the core ran, not a duration, and it does not become
/// one by dividing by a nominal clock: that is the whole reason it exists beside the TSC rather than
/// instead of it.
// The consumers are the bench probe (`bench::cycles_per_tick`, `--features bench`) and this
// module's own tests. A production boot has nothing to measure, so it has no caller, and marking
// that rather than manufacturing one is the same call `arch::riscv64::pmu::cycles` makes. The
// counter is still configured and printed in every build, because *whether this machine has one* is
// a fact about the machine and belongs on the boot line either way.
#[cfg_attr(
    not(any(test, feature = "system_tests", feature = "bench")),
    allow(dead_code)
)]
pub fn cycles() -> Option<u64> {
    if outcome() != CycleCounter::Running {
        return None;
    }
    Some(read_counter())
}

/// How wide the counter is, in bits, or `None` when there is none worth reading. For the boot line
/// and for a reader differencing two reads by hand.
pub fn cycle_counter_width() -> Option<u32> {
    if outcome() != CycleCounter::Running {
        return None;
    }
    Some(COUNTER_BITS.load(Ordering::Relaxed))
}

/// Difference two reads of [`cycles`], wrapped at the counter's width. Public because the width is
/// this module's business and not its caller's.
#[cfg_attr(
    not(any(test, feature = "system_tests", feature = "bench")),
    allow(dead_code)
)]
pub fn elapsed_cycles(first: u64, second: u64) -> u64 {
    cycles_delta(first, second)
}

/// The boot line, printed beside the ISA summary.
pub fn print_summary() {
    let version = PERFMON_VERSION.load(Ordering::Relaxed);
    match outcome() {
        CycleCounter::Running => crate::println!(
            "  cycles      : IA32_PERF_FIXED_CTR1 (unhalted core cycles), {} bits, perfmon v{version}",
            cycle_counter_width().unwrap_or(0),
        ),
        CycleCounter::NoPerfmonLeaf => crate::println!(
            "  cycles      : no architectural performance monitoring (cpuid leaf 0x0a); TSC ticks only"
        ),
        CycleCounter::NoFixedCounter => crate::println!(
            "  cycles      : perfmon v{version} has no fixed counter 1; TSC ticks only"
        ),
        CycleCounter::Stuck => crate::println!(
            "  cycles      : IA32_PERF_FIXED_CTR1 enabled but did not advance over {} TSC ticks; refused",
            CHECK_TSC.load(Ordering::Relaxed),
        ),
        CycleCounter::InStepWithTheTsc => crate::println!(
            "  cycles      : IA32_PERF_FIXED_CTR1 advanced {} against {} TSC ticks, identically; \
             refused (it is the TSC)",
            CHECK_CYCLES.load(Ordering::Relaxed),
            CHECK_TSC.load(Ordering::Relaxed),
        ),
        CycleCounter::Unknown => crate::println!("  cycles      : arch::pmu::init has not run"),
    }
}

#[cfg(test)]
mod tests {
    //! What only a boot can say. There is no host-testable arithmetic here beyond
    //! [`super::cycles_delta`], which is proved below; everything else needs the MSRs on the other
    //! end of an instruction.
    //!
    //! **None of these is a claim about cycles.** Under QEMU the expected outcome is
    //! `NoPerfmonLeaf`, because the emulator models no PMU unless asked, so the assertions here are
    //! about the plumbing: that the gate held, that the outcome is decided, and that the three
    //! records agree. The measurement is xenon's first bench boot and it has not been run; see
    //! design/roadmap/0309-x86-64-core-cycles.md.

    use super::*;

    /// **The outcome, the counter and the width all tell the same story.**
    ///
    /// [`outcome`] is the single gate, so the failure this catches is the records disagreeing: a
    /// `Running` with no width, or a width surviving a refusal. `Unknown` in particular must be
    /// impossible by the time any test runs, because `kernel_main` calls [`init`] before the suite.
    #[test_case]
    fn the_outcome_and_the_counter_agree() {
        let outcome = outcome();
        assert_ne!(
            outcome,
            CycleCounter::Unknown,
            "kernel_main calls arch::pmu::init before the suite"
        );

        let running = outcome == CycleCounter::Running;
        assert_eq!(cycles().is_some(), running);
        assert_eq!(cycle_counter_width().is_some(), running);

        if let Some(bits) = cycle_counter_width() {
            assert!(
                (1..=64).contains(&bits),
                "width {bits} is outside what an 8-bit CPUID field can usefully encode"
            );
        }
    }

    /// **The `CPUID` gate and the outcome agree, which is the assertion that keeps `init` from
    /// faulting.**
    ///
    /// Every outcome other than `NoPerfmonLeaf` means [`init`] executed an `rdmsr` on
    /// `IA32_FIXED_CTR_CTRL`. That is a `#GP` on a part without architectural performance
    /// monitoring, so "the leaf said version >= 1" must be true whenever the outcome says the MSRs
    /// were touched. This is the one invariant in the module that a wrong edit could turn into a
    /// dead machine rather than a wrong number.
    #[test_case]
    fn the_msrs_are_only_touched_when_cpuid_allowed_it() {
        let reached_the_msrs = !matches!(
            outcome(),
            CycleCounter::NoPerfmonLeaf | CycleCounter::Unknown
        );
        if reached_the_msrs {
            assert!(
                isa::get().max_leaf >= CPUID_PERFMON_LEAF,
                "init read an MSR on a part whose CPUID does not reach leaf 0x0a"
            );
            assert!(
                PERFMON_VERSION.load(Ordering::Relaxed) > 0,
                "init read an MSR on a part reporting performance monitoring version 0"
            );
        }
    }

    /// **QEMU models no PMU unless told to, and that is the outcome to expect here.**
    ///
    /// Asserted as a *set* rather than a single value, for the reason `arch::riscv64::pmu`'s
    /// equivalent test learned the hard way: what an emulator answers is a property of the
    /// emulator's configuration, and pinning one value pins one `-cpu` line. `-cpu max` with
    /// `pmu=on` would legitimately produce `Running`, `Stuck` or `InStepWithTheTsc`, and all four
    /// are things this kernel handles. What must never happen is `Unknown`, which would mean
    /// [`init`] left the module undecided, and that is what the first test above asserts.
    #[test_case]
    fn qemu_reaches_a_decided_answer() {
        let outcome = outcome();
        assert_ne!(outcome, CycleCounter::Unknown);
        // `cycles()` must agree with the gate on every one of them, which is the property a future
        // reader of a real machine's transcript depends on: no outcome but `Running` hands back a
        // number.
        assert_eq!(
            cycles().is_some(),
            outcome == CycleCounter::Running,
            "{outcome:?} handed back a cycle count it should not have"
        );
    }

    /// **A narrow counter's wrap comes out as a small positive delta.**
    ///
    /// The one piece of arithmetic here that can be wrong silently: unmasked, a 48-bit counter
    /// wrapping mid-window produces a difference near 2^64, which would print as a preposterous
    /// `cycles_per_tick` rather than as an error. Exercised at the width real parts report.
    #[test_case]
    fn a_narrow_counter_wraps_to_a_small_delta() {
        let saved = COUNTER_BITS.load(Ordering::Relaxed);
        COUNTER_BITS.store(48, Ordering::Relaxed);

        let top = (1u64 << 48) - 1;
        assert_eq!(cycles_delta(top, 9), 10, "a 48-bit wrap must be 10 cycles");
        assert_eq!(cycles_delta(100, 200), 100);

        // And a 64-bit counter is the identity case, which is what the `bits >= 64` arm is for.
        COUNTER_BITS.store(64, Ordering::Relaxed);
        assert_eq!(cycles_delta(u64::MAX, 9), 10);

        COUNTER_BITS.store(saved, Ordering::Relaxed);
    }
}
