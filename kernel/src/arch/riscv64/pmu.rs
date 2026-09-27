//! **Ask firmware for a hardware cycle counter, and read it** (milestone 74, riscv64 half).
//!
//! The RISC-V hardware performance counters live in M-mode. `mcycle` and the `mhpmcounterX` CSRs
//! can only be started, stopped and configured from there (`mcountinhibit`, `mhpmeventX`), so a
//! supervisor-mode kernel cannot program them itself. The SBI **PMU** extension (EID `0x504D55`,
//! `"PMU"`) is the interface firmware exposes for exactly this, and it is the whole reason this
//! module exists rather than a `csrw`.
//!
//! # What this is for, and what it is deliberately not
//!
//! `crate::arch::timer::now()` reads the `time` CSR, a **fixed-rate reference tick**. That is the
//! OS's clock and it is the right instrument for a long loop. It is not cycles, and the literature
//! this project is compared against is denominated in cycles: milestone 74's block sets out the
//! difference and notes/pmu.md calls confusing the two a category error.
//!
//! **One counter, configured for one event, started and stopped.** Milestone 74's scope note
//! refuses a profiling framework, and the PMU can count dozens of events. Generalising this to a
//! counter set with a named target is milestone 147's, and it should wait for that second consumer.
//!
//! # The shape of the interface, from the specification
//!
//! Everything here is the RISC-V SBI Specification **v3.0** (ratified 2025-07-16), `src/ext-pmu.adoc`,
//! read 2026-09-03 from `github.com/riscv-non-isa/riscv-sbi-doc` at tag `v3.0`. Four calls:
//!
//! 1. `sbi_pmu_num_counters` (FID #0), how many logical counters this hart has.
//! 2. `sbi_pmu_counter_config_matching` (FID #2), "find a counter from this set which is not
//!    started and can monitor this event", which is how a caller gets a counter without knowing
//!    the platform's event-to-counter mapping.
//! 3. `sbi_pmu_counter_get_info` (FID #1), which says whether the counter it found is hardware or
//!    firmware and, for hardware, **which CSR reads it**.
//! 4. `sbi_pmu_counter_stop` (FID #4).
//!
//! The read itself is not an `ecall`. That is the point of step 3: firmware hands back a CSR
//! number and the counter is then read with one instruction, so a measurement does not contain the
//! call that would dominate it. (A *firmware* counter would have to be read with FID #5, an
//! `ecall` per read; this module refuses those, and [`init`] says why.)
//!
//! # Why the counter this asks for is startable at all
//!
//! OpenSBI leaves the fixed counters running and open to S-mode. Read from
//! `lib/sbi/sbi_hart.c` on `riscv-software-src/opensbi` master, 2026-09-03: it writes
//! `CSR_MCOUNTEREN, -1` ("Supervisor mode usage for all counters are enabled by default") and
//! `CSR_MCOUNTINHIBIT, 0xFFFFFFF8`, which leaves bits 0 (`CY`) and 2 (`IR`) clear, so `mcycle` and
//! `minstret` run and only the programmable counters are inhibited until S-mode asks. **That is
//! upstream OpenSBI, not the firmware flashed on the VisionFive 2 this project benches on
//! (radon)**, and it is a claim about somebody else's build until a
//! bench reads it; notes/riscv-cycle-counters.md makes checking it step one of the procedure.
//!
//! # BUGS
//!
//! - **Nothing here has been run on silicon.** QEMU-TCG models an instruction counter that has
//!   nothing to do with cycles, so a green test under emulation proves the plumbing and says
//!   nothing about the measurement. Every number this module can produce today is an emulator's
//!   fiction. notes/riscv-cycle-counters.md carries the bench procedure that would fix that.
//! - **The boot hart only.** SBI PMU counters are per-hart: `counter_config_matching` and
//!   `counter_start` act on the calling hart, and the counter index a hart is given is that hart's.
//!   [`init`] runs once, on the hart that runs `kernel_main`, and [`cycles`] is therefore only
//!   meaningful to a reader who knows which hart it ran on. The consumer today is a single-hart
//!   bench probe. A per-hart record is real work and belongs with a caller that needs it.
//! - **A firmware counter is refused rather than used.** If firmware answers with a counter it
//!   maintains itself, this module reports no cycle counter at all. That is a deliberate refusal:
//!   reading one costs an `ecall`, and an `ecall` inside a cycle measurement measures the `ecall`.
//! - **`counter_stop` is never called on the happy path.** The counter this configures runs for the
//!   life of the boot. That is correct for a free-running cycle counter, whose value is only ever
//!   read as a difference, and it is why [`stop`] exists but is only used by the test that proves
//!   the call works.

use core::arch::asm;
use core::sync::atomic::{AtomicU8, AtomicU32, AtomicU64, Ordering};

use machine_discovery::riscv64::{
    CounterInfo, EID_PMU, EVENT_HW_CPU_CYCLES, PMU_CFG_AUTO_START, PMU_CFG_CLEAR_VALUE, SBI_PMU,
};

/// `sbi_pmu_num_counters`.
const FID_NUM_COUNTERS: usize = 0;
/// `sbi_pmu_counter_get_info`.
const FID_COUNTER_GET_INFO: usize = 1;
/// `sbi_pmu_counter_config_matching`.
const FID_COUNTER_CONFIG_MATCHING: usize = 2;
/// `sbi_pmu_counter_stop`.
const FID_COUNTER_STOP: usize = 4;

/// **Why there is or is not a cycle counter**, as one value, decided once by [`init`].
///
/// This exists because "no counter" has five distinguishable causes and they need different fixes,
/// and the machine where that matters is one nobody here can attach a debugger to.
/// notes/riscv-cycle-counters.md's bench procedure originally told a reader to *add* a print of the
/// SBI error and the raw `counter_info` word when the answer was disappointing; building the answer
/// in is rung two instead of rung four, and it costs one byte and one boot line.
///
/// Ordered so that the discriminant is the stored value and `Unknown` is zero, which is what an
/// unreached [`init`] leaves behind.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum CycleCounter {
    /// [`init`] has not run.
    Unknown = 0,
    /// Firmware does not implement the SBI PMU extension. Nothing to ask.
    NoPmuExtension,
    /// It does, but no counter on this hart can count `SBI_PMU_HW_CPU_CYCLES`
    /// (`SBI_ERR_NOT_SUPPORTED` from the match), or it has no counters at all.
    NoMatchingCounter,
    /// Firmware offered a counter it maintains itself. Refused: reading one costs an `ecall` per
    /// read, and an `ecall` inside a cycle measurement measures the `ecall`.
    FirmwareCounter,
    /// The CSR firmware named is outside the unprivileged counter block, so this kernel cannot name
    /// it in an instruction.
    UnreadableCsr,
    /// **The counter was accepted, read twice across a spin, and had not moved.** Observed on
    /// QEMU's `rva23s64` model, where OpenSBI allocates `hpmcounter3` and TCG models it as a
    /// constant zero. See [`init`].
    Stuck,
    /// A hardware counter, readable, and advancing.
    Running,
}

/// [`CycleCounter`] as a byte. An atomic rather than a lock: written once by [`init`] on the boot
/// hart before anything reads it, and read-only afterwards. Nothing here nests with another lock,
/// so it earns no rank in `sync::rank`.
static OUTCOME: AtomicU8 = AtomicU8::new(CycleCounter::Unknown as u8);

/// The CSR that reads the counter firmware gave us, or [`NO_CSR`].
///
/// **Data, not the gate.** [`OUTCOME`] is the gate, and this is written as soon as firmware names
/// it so that a boot line reporting a *refused* counter can still say which one it was, which is
/// the whole diagnostic value on a board nobody can attach a debugger to.
static CYCLE_CSR: AtomicU32 = AtomicU32::new(NO_CSR);

/// "No counter", distinct from every legal CSR number (which are 12 bits).
const NO_CSR: u32 = u32::MAX;

/// The logical counter index firmware chose, for the boot line and for [`stop`]. Written as soon as
/// firmware names it, for the reason [`CYCLE_CSR`] gives; meaningful only alongside [`outcome`].
static CYCLE_COUNTER_IDX: AtomicU64 = AtomicU64::new(0);

/// How wide the counter is, in bits, for the boot line. A narrow counter wraps, and a reader
/// comparing two reads across a long interval needs to know where.
static CYCLE_BITS: AtomicU32 = AtomicU32::new(0);

/// **Find a hardware counter that counts CPU cycles, start it, check it is counting, and remember
/// how to read it.**
///
/// Called once, from `kernel_main`, after [`super::isa::init`] has probed which SBI extensions
/// firmware implements. Silent and harmless on a machine without the PMU extension: [`cycles`]
/// then answers `None` forever, which is the honest answer and not an error. Every way of failing
/// is recorded in [`outcome`] and printed by [`print_summary`], because on the machine where this
/// matters nobody can attach a debugger.
///
/// # Why `SKIP_MATCH` is not passed
///
/// The specification's `SBI_PMU_CFG_FLAG_SKIP_MATCH` makes firmware "unconditionally select the
/// first counter from the set" without checking it can count the event. That is the wrong request
/// here by exactly the thing this module needs: we do not know which counter counts cycles on this
/// platform, and asking firmware to match is the entire reason to make the call rather than write
/// a CSR number down.
///
/// # Why the counter is checked before it is believed
///
/// **Because firmware can hand back a counter that does not count**, and this was measured rather
/// than imagined. `script/cpu-matrix`'s `rva23s64` model: OpenSBI answers the match with counter 3
/// (`hpmcounter3`) rather than `mcycle`, `counter_get_info` describes it as a 64-bit hardware
/// counter, `csrr` on it is legal and returns **zero, every time**, because QEMU-TCG does not model
/// the programmable counters. Four of the five models in the matrix answer `mcycle` and are fine.
///
/// A benchmark reading 0 cycles for everything is worse than one that says there is no cycle
/// counter, so this reads the counter twice across a short timed spin and refuses it if it has not
/// moved. A cycle counter cannot fail that check on a hart that is executing instructions. The
/// counter is stopped on the way out, since nothing will read it.
pub fn init() {
    OUTCOME.store(CycleCounter::Unknown as u8, Ordering::Relaxed);
    CYCLE_CSR.store(NO_CSR, Ordering::Relaxed);

    if !super::isa::get().sbi.extensions.contains(SBI_PMU) {
        OUTCOME.store(CycleCounter::NoPmuExtension as u8, Ordering::Release);
        return;
    }

    let (err, count) = sbi_call(FID_NUM_COUNTERS, [0; 5]);
    if err != 0 || count == 0 {
        OUTCOME.store(CycleCounter::NoMatchingCounter as u8, Ordering::Release);
        return;
    }

    // The whole counter space, as one base plus a mask. `counter_idx_mask` is XLEN wide, so one
    // call covers up to 64 counters and every platform's real count is far below that; a machine
    // that reported more would leave the tail unasked, which is a smaller counter set than the
    // machine has and never a wrong answer about the counter we do get.
    let mask = if count >= 64 {
        u64::MAX as usize
    } else {
        (1usize << count) - 1
    };

    // Configure and start in one call. `CLEAR_VALUE` so the first read is not an inherited number
    // from whatever ran before us, `AUTO_START` so this is one `ecall` rather than two.
    let (err, idx) = sbi_call(
        FID_COUNTER_CONFIG_MATCHING,
        [
            0,
            mask,
            PMU_CFG_CLEAR_VALUE | PMU_CFG_AUTO_START,
            EVENT_HW_CPU_CYCLES,
            0,
        ],
    );
    if err != 0 {
        // `SBI_ERR_NOT_SUPPORTED` (-2) here means no counter on this hart can count cycles, which
        // is a legitimate machine and not a failure to report loudly.
        OUTCOME.store(CycleCounter::NoMatchingCounter as u8, Ordering::Release);
        return;
    }
    CYCLE_COUNTER_IDX.store(idx as u64, Ordering::Relaxed);

    let (err, raw) = sbi_call(FID_COUNTER_GET_INFO, [idx, 0, 0, 0, 0]);
    if err != 0 {
        stop_counter(idx);
        OUTCOME.store(CycleCounter::NoMatchingCounter as u8, Ordering::Release);
        return;
    }
    let info = CounterInfo::from_raw(raw as u64);

    // A firmware counter is refused: see this module's BUGS.
    let (Some(csr), Some(bits)) = (info.csr(), info.bits()) else {
        stop_counter(idx);
        OUTCOME.store(CycleCounter::FirmwareCounter as u8, Ordering::Release);
        return;
    };
    CYCLE_CSR.store(csr as u32, Ordering::Relaxed);
    CYCLE_BITS.store(bits, Ordering::Relaxed);

    // A CSR this kernel cannot name in an instruction is the same outcome as no counter, for the
    // same reason: the read must be one instruction or the measurement is measuring the read.
    let Some(first) = read_csr(csr) else {
        stop_counter(idx);
        OUTCOME.store(CycleCounter::UnreadableCsr as u8, Ordering::Release);
        return;
    };

    // Does it count? A short window on the `time` CSR, which is running by now and is the one clock
    // this kernel has that does not depend on the answer. Short enough not to be felt in a boot,
    // long enough that any real cycle counter has moved by thousands.
    const CHECK_TICKS: u64 = 100;
    let t0 = super::timer::now();
    while super::timer::now() - t0 < CHECK_TICKS {
        core::hint::spin_loop();
    }
    if read_csr(csr) == Some(first) {
        stop_counter(idx);
        OUTCOME.store(CycleCounter::Stuck as u8, Ordering::Release);
        return;
    }

    // Last, and with Release: a reader that sees `Running` must see the CSR, index and width
    // written before it. Every read is `Acquire` on this one location for the same reason.
    OUTCOME.store(CycleCounter::Running as u8, Ordering::Release);
}

/// **Why there is or is not a cycle counter on this hart.** See [`CycleCounter`].
pub fn outcome() -> CycleCounter {
    match OUTCOME.load(Ordering::Acquire) {
        x if x == CycleCounter::NoPmuExtension as u8 => CycleCounter::NoPmuExtension,
        x if x == CycleCounter::NoMatchingCounter as u8 => CycleCounter::NoMatchingCounter,
        x if x == CycleCounter::FirmwareCounter as u8 => CycleCounter::FirmwareCounter,
        x if x == CycleCounter::UnreadableCsr as u8 => CycleCounter::UnreadableCsr,
        x if x == CycleCounter::Stuck as u8 => CycleCounter::Stuck,
        x if x == CycleCounter::Running as u8 => CycleCounter::Running,
        _ => CycleCounter::Unknown,
    }
}

/// Stop one counter and discard the answer. Used only on the give-up paths in [`init`], where the
/// counter has been started and nothing is going to read it; the error cannot change what we do.
fn stop_counter(idx: usize) {
    let _ = sbi_call(FID_COUNTER_STOP, [idx, 1, 0, 0, 0]);
}

/// **The cycle count on this hart**, or `None` when no hardware cycle counter was found.
///
/// One CSR read, no `ecall`, so a caller may put this on either side of the thing it is measuring.
///
/// **The value is only meaningful as a difference.** The specification's own note on the event says
/// these "may be variable frequency cycles, and are not counted when the CPU clock is halted", so a
/// count is not a duration and does not become one by dividing by a nominal clock. That is the
/// whole reason this exists beside the `time` CSR rather than instead of it.
// The consumers are the bench probe (`bench::cycles_per_tick`, `--features bench`) and this
// module's own tests. A production boot has nothing to measure, so it has no caller, and marking
// that rather than manufacturing one is the same call `arch::timer::is_cycle_counter_grantable`
// makes four files over. The counter is still configured and printed in every build, because
// *whether this machine has one* is a fact about the machine and belongs on the boot line either
// way.
#[cfg_attr(
    not(any(test, feature = "system_tests", feature = "bench")),
    allow(dead_code)
)]
pub fn cycles() -> Option<u64> {
    if outcome() != CycleCounter::Running {
        return None;
    }
    read_csr(CYCLE_CSR.load(Ordering::Relaxed) as u16)
}

/// How wide the counter is, in bits, or `None` when there is none. For the boot line: a counter
/// narrower than 64 bits wraps, and a reader differencing two reads needs to know where.
pub fn cycle_counter_width() -> Option<u32> {
    if outcome() != CycleCounter::Running {
        return None;
    }
    Some(CYCLE_BITS.load(Ordering::Relaxed))
}

/// **Stop the counter [`init`] started.**
///
/// Not on any normal path: a free-running cycle counter read as a difference never needs stopping,
/// and this module's BUGS says so. It exists because the stop call is half of the interface
/// milestone 74 names, and a call nothing ever makes is a call nothing has ever checked.
///
/// Returns the SBI error code, so a caller can tell "stopped" from "was already stopped"
/// (`SBI_ERR_ALREADY_STOPPED`, -8) from "there was no counter" (-3, which is what an invalid index
/// earns).
#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
pub fn stop() -> isize {
    if outcome() != CycleCounter::Running {
        return -3;
    }
    let idx = CYCLE_COUNTER_IDX.load(Ordering::Relaxed) as usize;
    let (err, _) = sbi_call(FID_COUNTER_STOP, [idx, 1, 0, 0, 0]);
    err
}

/// **Read one of the U/S-readable counter CSRs by number.**
///
/// RISC-V encodes the CSR as a 12-bit immediate in the instruction, so there is no "read the CSR
/// this variable names" instruction and this dispatch cannot be avoided. `None` for anything
/// outside the counter range, which is the same answer as "no counter" to every caller.
///
/// The range is the unprivileged counter block: `cycle` (`0xc00`), `instret` (`0xc02`) and
/// `hpmcounter3` through `hpmcounter31` (`0xc03` to `0xc1f`). `time` (`0xc01`) is deliberately
/// absent: firmware will never name it as a PMU counter, and a `time` read reaching this path
/// would be the fixed-rate tick wearing a cycle counter's name, which is the exact confusion
/// milestone 74 exists to remove.
fn read_csr(csr: u16) -> Option<u64> {
    macro_rules! read {
        ($name:literal) => {{
            let value: u64;
            // SAFETY: reading an unprivileged counter CSR touches no memory and changes no state,
            // which the options state. S-mode reads of these are gated by `mcounteren` in firmware;
            // this path is only reached after SBI PMU handed us the CSR number for a counter it
            // started, which is firmware saying it is readable.
            unsafe {
                asm!(
                    concat!("csrr {}, ", $name),
                    out(reg) value,
                    options(nomem, nostack, preserves_flags),
                );
            }
            value
        }};
    }

    /// Expand one arm per `hpmcounterN`, so the 29 regular cases are written once.
    macro_rules! hpm {
        ($csr:expr, $($num:literal => $name:literal,)*) => {
            match $csr {
                $($num => Some(read!($name)),)*
                _ => None,
            }
        };
    }

    hpm! { csr,
        0xc00 => "cycle",
        0xc02 => "instret",
        0xc03 => "hpmcounter3",
        0xc04 => "hpmcounter4",
        0xc05 => "hpmcounter5",
        0xc06 => "hpmcounter6",
        0xc07 => "hpmcounter7",
        0xc08 => "hpmcounter8",
        0xc09 => "hpmcounter9",
        0xc0a => "hpmcounter10",
        0xc0b => "hpmcounter11",
        0xc0c => "hpmcounter12",
        0xc0d => "hpmcounter13",
        0xc0e => "hpmcounter14",
        0xc0f => "hpmcounter15",
        0xc10 => "hpmcounter16",
        0xc11 => "hpmcounter17",
        0xc12 => "hpmcounter18",
        0xc13 => "hpmcounter19",
        0xc14 => "hpmcounter20",
        0xc15 => "hpmcounter21",
        0xc16 => "hpmcounter22",
        0xc17 => "hpmcounter23",
        0xc18 => "hpmcounter24",
        0xc19 => "hpmcounter25",
        0xc1a => "hpmcounter26",
        0xc1b => "hpmcounter27",
        0xc1c => "hpmcounter28",
        0xc1d => "hpmcounter29",
        0xc1e => "hpmcounter30",
        0xc1f => "hpmcounter31",
    }
}

/// One SBI PMU call, returning `(error, value)` from `a0` and `a1`.
///
/// Unlike `isa::sbi_call`, this one **returns the error**. The base extension's getters cannot
/// fail; every call here can, and every failure means "there is no cycle counter", which is a fact
/// the boot line reports rather than one to discard.
fn sbi_call(fid: usize, args: [usize; 5]) -> (isize, usize) {
    let [a0, a1, a2, a3, a4] = args;
    // SAFETY: the PMU extension configures, starts and stops counters; it changes what the hardware
    // counts and touches no memory of ours.
    let ret = unsafe { super::sbi::call(EID_PMU, fid, [a0, a1, a2, a3, a4, 0]) };
    (ret.error, ret.value)
}

/// The boot line, printed beside the ISA summary.
pub fn print_summary() {
    match outcome() {
        CycleCounter::Running => crate::println!(
            "  cycles      : SBI PMU counter {}, CSR {:#05x}, {} bits",
            CYCLE_COUNTER_IDX.load(Ordering::Relaxed),
            CYCLE_CSR.load(Ordering::Relaxed),
            cycle_counter_width().unwrap_or(0),
        ),
        CycleCounter::NoPmuExtension => {
            crate::println!("  cycles      : no SBI PMU extension; ticks only");
        }
        CycleCounter::NoMatchingCounter => {
            crate::println!("  cycles      : SBI PMU present, no counter can count CPU cycles");
        }
        CycleCounter::FirmwareCounter => crate::println!(
            "  cycles      : SBI PMU offered a firmware counter; refused (an ecall per read)"
        ),
        CycleCounter::UnreadableCsr => {
            crate::println!("  cycles      : SBI PMU named a CSR outside the counter block");
        }
        CycleCounter::Stuck => crate::println!(
            "  cycles      : SBI PMU counter {} (CSR {:#05x}) did not advance; refused",
            CYCLE_COUNTER_IDX.load(Ordering::Relaxed),
            CYCLE_CSR.load(Ordering::Relaxed),
        ),
        CycleCounter::Unknown => crate::println!("  cycles      : arch::pmu::init has not run"),
    }
}

#[cfg(test)]
mod tests {
    //! What only a boot can say. The `counter_info` decode, the event encoding and the config
    //! flags are proved on the host in `crates/machine_discovery/tests/riscv64_isa_strings.rs`;
    //! these are the assertions that need firmware on the other end of an `ecall`.
    //!
    //! **None of these is a claim about cycles.** QEMU-TCG's `cycle` CSR is an instruction count,
    //! so every assertion here is about the plumbing: that the call was made, that the answer was
    //! decoded, that the CSR named is readable and moves forward. The measurement is
    //! notes/riscv-cycle-counters.md's bench procedure and it has not been run.

    use super::*;

    /// **The outcome, the counter and the width all tell the same story.**
    ///
    /// [`outcome`] is the single gate, so the failure this catches is the three atomics
    /// disagreeing: a `Running` with no readable counter, or a counter surviving a refusal.
    /// `Unknown` in particular must be impossible by the time any test runs, because `kernel_main`
    /// calls [`init`] before the suite.
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

        if outcome == CycleCounter::NoPmuExtension {
            assert!(
                !super::super::isa::get().sbi.extensions.contains(SBI_PMU),
                "the outcome says no PMU extension and the probe says there is one"
            );
        }

        if let Some(bits) = cycle_counter_width() {
            assert!(
                (1..=64).contains(&bits),
                "width {bits} is outside what a 6-bit field plus one can encode"
            );
        }
    }

    /// **QEMU's OpenSBI implements the PMU extension, and this kernel reached a decided answer.**
    ///
    /// # This test asserted `CSR == 0xc00` for about an hour, and `script/cpu-matrix` earned its
    /// keep twice over
    ///
    /// Four of the five models in the matrix answer **counter 0, CSR `0xc00`**: `mcycle`, read
    /// through the `cycle` CSR. **`rva23s64` answers counter 3, CSR `0xc03`**, a programmable
    /// `hpmcounter`, which `counter_get_info` describes as a 64-bit hardware counter and which
    /// **reads zero forever**, because QEMU-TCG does not model the programmable counters. Both
    /// measured 2026-09-03 from `target/cpu-matrix/*.log`.
    ///
    /// That is two lessons and the second is the one worth having. The counter a caller gets is
    /// firmware's choice and not a constant, so pinning the number pins one emulator's allocation
    /// policy. And **a counter firmware hands back can be a counter that does not count**, which is
    /// why [`init`] checks before it believes and why the outcome vocabulary has a `Stuck` member
    /// at all.
    ///
    /// So what is asserted here is the pair of outcomes actually observed, and that a `Running` one
    /// names a plausible CSR. A `NoMatchingCounter` or `FirmwareCounter` on QEMU would be a real
    /// regression and fails.
    #[test_case]
    fn qemu_reaches_one_of_the_two_answers_it_has_ever_given() {
        assert!(
            super::super::isa::get().sbi.extensions.contains(SBI_PMU),
            "OpenSBI has implemented SBI PMU since 0.8"
        );

        match outcome() {
            CycleCounter::Running => {
                let csr = CYCLE_CSR.load(Ordering::Relaxed);
                assert!(
                    csr == 0xc00 || (0xc03..=0xc1f).contains(&csr),
                    "CSR {csr:#05x} is neither `cycle` nor an `hpmcounter`; `time` (0xc01) in \
                     particular would be the fixed tick wearing a cycle counter's name"
                );
                assert_eq!(cycle_counter_width(), Some(64));
            }
            // `rva23s64`: `hpmcounter3`, modelled by TCG as a constant zero.
            CycleCounter::Stuck => {}
            other => panic!(
                "SBI PMU is present and the outcome is {other:?}; on QEMU that is a regression"
            ),
        }
    }

    /// **The counter moves forward.**
    ///
    /// [`init`] already checked this once, so on a healthy boot this is a re-check rather than a
    /// discovery, and it is worth running anyway: it is what would catch a counter that firmware
    /// stops later, or an inhibit that arrives with the second hart.
    ///
    /// **This is not a measurement.** Under TCG the delta is an instruction count. The window is a
    /// `time` CSR spin rather than a fixed iteration count so that the two counters are read over
    /// the same span, which is the shape the bench probe uses.
    #[test_case]
    fn the_counter_advances() {
        let Some(first) = cycles() else {
            return;
        };

        let start = super::super::timer::now();
        while super::super::timer::now() - start < 1000 {
            core::hint::spin_loop();
        }

        let second = cycles().expect("the counter did not vanish mid-test");
        assert!(
            second.wrapping_sub(first) > 0,
            "counter read {first} twice across a timed spin; it is not counting"
        );
    }

    /// **Stop is a real call and firmware answers it.**
    ///
    /// The one place `stop` is exercised, and it re-runs [`init`] afterwards so the module is left
    /// in a decided state rather than a stopped one. `SBI_SUCCESS` (0) and `SBI_ERR_ALREADY_STOPPED`
    /// (-8) are both correct answers; anything else means the counter index we are holding is not
    /// one firmware recognises, which would make every other number here suspect.
    ///
    /// # Why it does not assert the counter comes back the way it went away
    ///
    /// **Because on QEMU's `rva23s64` it does not**, measured 2026-09-03. That model's counter is
    /// `hpmcounter3`, and it counts when it is first configured at boot and reads zero after a
    /// `counter_stop` and a fresh `counter_config_matching`, so [`init`]'s did-it-move check
    /// correctly refuses it the second time and the outcome goes `Running` to `Stuck`. That is an
    /// emulator's event-mapping behaviour, not an invariant of the interface, and asserting the
    /// round trip is idempotent would be encoding one emulator's bug as a property of SBI.
    ///
    /// So the assertion is what the interface does promise: the stop was accepted, and `init` left
    /// the module somewhere other than `Unknown`. The transition itself is worth watching rather
    /// than gating, which is what the boot line is for.
    #[test_case]
    fn stopping_the_counter_is_accepted() {
        if outcome() != CycleCounter::Running {
            return;
        }
        let err = stop();
        assert!(
            err == 0 || err == -8,
            "sbi_pmu_counter_stop returned {err}, which is neither success nor already-stopped"
        );

        init();
        assert_ne!(
            outcome(),
            CycleCounter::Unknown,
            "init left the module undecided, which it cannot do: every path stores an outcome"
        );
    }
}
