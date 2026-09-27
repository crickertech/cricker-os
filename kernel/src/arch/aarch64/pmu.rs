//! **Start the cycle counter, check it counts, and read it** (milestone 74, the aarch64 half).
//!
//! `PMCCNTR_EL0` is the PMU's always-present cycle counter. Until this module it was a **stopped**
//! counter in this kernel: nothing wrote `PMCR_EL0.E` or `PMCNTENSET_EL0.C`, so a thread holding
//! milestone 229's grant could read it legally and got the same number every time. Starting it is a
//! handful of register writes. The rest of this file is what goes around them, and it is the shape
//! `arch::riscv64::pmu` and `arch::x86_64::pmu` already established: check the counter moves before
//! believing it, and record *why* when it does not, as a value the boot line prints.
//!
//! # Why this is a register write and not a firmware call
//!
//! On riscv64 the counters are M-mode's, so the kernel has to ask firmware (SBI PMU). On aarch64
//! the PMU's control registers are directly writable at EL1 once nothing above traps them.
//! `MDCR_EL2.TPM` is the EL2 half of "nothing above", and `boot.s`'s EL2 drop clears it (milestone
//! 127's first prerequisite). `MDCR_EL3.TPM` is the EL3 half and is TF-A's, not ours. **So on
//! aarch64 there is nobody else to blame**: a counter that does not run is this kernel's failure to
//! start it, or a secure-world setting this module can observe but not change.
//!
//! # The registers, per core
//!
//! Every one of them is banked per PE, and every one resets to an architecturally UNKNOWN value
//! (Arm DDI 0487, and the `AArch64-pmcr_el0`, `-pmcntenset_el0` and `-pmccfiltr_el0` pages of the
//! system-register reference at `arm.jonpalmisc.com`, read 2026-09-19). So [`init_this_core`] runs
//! on **every** core from `timer::init`, which is the per-core init site the `PMUSERENR_EL0` write
//! already lives at, and it **assigns** rather than read-modify-writes: a field firmware left set is
//! exactly the thing this is removing.
//!
//! | register | written | why |
//! |---|---|---|
//! | `PMCCFILTR_EL0` | [`PMCCFILTR_PROVISIONAL`] | which exception levels are counted. **Provisional**, and calef's; see below |
//! | `PMCR_EL0` | `E \| C \| LC` | `E` enables; `C` zeroes this core's cycle counter; `LC` makes its overflow 64-bit. `D` (divide by 64) and `DP` (stop when counting is prohibited) are cleared by the assignment, and a set `D` would make every number 64 times too small |
//! | `PMCNTENSET_EL0` | bit 31 (`C`) | the cycle counter's own enable. Write-one-to-set, so no other counter is touched |
//!
//! Linux's `armv8pmu_reset` (`drivers/perf/arm_pmuv3.c`, read 2026-09-19) assigns `PMCR_EL0` the
//! same way, `P | C | LC`, for the same reason. seL4's `arm_init_ccnt` (`src/arch/arm/benchmark/
//! benchmark.c`, read 2026-09-19) writes `E | C | P` and then `PMCNTENSET` bit 31, which is this
//! module minus `LC`.
//!
//! # `PMCCFILTR_EL0` is written with a provisional value, and publishing a number waits on calef
//!
//! Its `P`, `U` and `NSH` bits decide whether cycles spent at EL1, EL0 and EL2 are counted. That
//! decides what every cycle number this kernel ever prints *means*: a count that includes EL1 is
//! comparable to seL4's IPC figures, and one that excludes it is a userspace-only profile. It is a
//! fact that leaves the machine, so it is an architect's, and the options are
//! `design/roadmap/proposals/the-aarch64-half-of-74.md`. What is written today is
//! zero, for the reasons [`PMCCFILTR_PROVISIONAL`] gives, and no number measured under it is a
//! result.
//!
//! # BUGS
//!
//! - **Nothing here has run on silicon.** Every outcome this module can report today is a fact about
//!   QEMU. argon (the Jetson TX1, milestone 127) is the machine that would say something, and its
//!   block's bench procedure names what to read on the console.
//! - **The did-it-move check cannot tell a real counter from an emulated one.** QEMU-TCG models
//!   `PMUv3` on every `-cpu` this tree boots (`cortex-a72`, `-a57`, `-a53`) and drives
//!   `PMCCNTR_EL0` from its own clocks, so the counter moves, is accepted as `Running`, and counts
//!   nothing a core did. Measured 2026-09-19: under `-icount` (the bench boot) it is the
//!   instruction count, and the probe reads `cycles_per_tick 16.00 (10000226 cycles over 625003
//!   ticks at cntfrq 62500000)`, which is `script/icount`'s own `instructions_per_counter_tick 16`
//!   plus the handful of instructions between the two reads; without `-icount` it advances in steps
//!   of 1000 at roughly 32 per tick. The defence is the riscv64 half's: the probe prints its inputs,
//!   and a round ratio is named as the tell rather than as the answer. The `Stuck` path has not run
//!   on any configuration here, because no QEMU model this tree boots leaves the counter constant.
//! - **A trap here is a hang or a fault before the banner.** If TF-A leaves `MDCR_EL3.TPM` set,
//!   every PMU register access at EL1 traps to EL3, and the first one is milestone 228's
//!   `PMUSERENR_EL0` write in `timer::init`, not anything in this file. That ordering predates this
//!   module; milestone 127's bench procedure says what the console looks like if it happens.
//! - **Each core's counter is its own.** `C` zeroes each core's counter at that core's init, so
//!   values from two cores are unrelated numbers, and a difference across a migration is garbage.
//!   [`cycles`] reads the current core's; a caller that cannot rule out migration must check the
//!   core did not change, which is what `bench::cycles_per_tick` does.
//! - **The counter is never stopped.** It runs for the life of the boot, the same call the other two
//!   halves make.

use core::sync::atomic::{AtomicU8, AtomicU64, Ordering};

use aarch64_cpu::registers::{CNTVCT_EL0, ID_AA64DFR0_EL1};
use tock_registers::interfaces::Readable;

use crate::cpu::{self, MAX_CPUS};

/// **The value written to `PMCCFILTR_EL0` on every core. PROVISIONAL**, pending calef's ruling in
/// `design/roadmap/proposals/the-aarch64-half-of-74.md`; do not publish a number
/// measured under it.
///
/// Zero means: `P` = 0 and `U` = 0, so EL1 and EL0 are counted; `NSK` and `NSU` equal to them, so
/// Non-secure EL1 and EL0 are counted too; `NSH` = 0, so **EL2 is not counted**; `M` = `P`, so EL3
/// is counted where `MDCR_EL3` permits it at all.
///
/// Why zero is the provisional value rather than any other, none of which is an argument that it
/// should be the final one:
///
/// - **It is the only value both things built here can run under.** `bench::cycles_per_tick` reads
///   the counter at EL1, and the EL0 grant test (`a_granted_thread_reads_the_cycle_counter_...`)
///   reads it at EL0. `P` = 1 would stop the first and `U` = 1 the second.
/// - **EL2 runs nothing in this kernel.** `boot.s` drops to EL1 and never installs an EL2 vector
///   table, so `NSH` changes nothing measurable on argon or QEMU today; it would matter under a
///   hypervisor that let a guest program it, which is not a configuration this kernel reports
///   numbers from.
/// - **It is a definite value over an UNKNOWN one**, which is the whole reason this register is
///   written at all.
pub const PMCCFILTR_PROVISIONAL: u64 = 0;

/// `PMCR_EL0.E`: enable the counters.
const PMCR_E: u64 = 1 << 0;
/// `PMCR_EL0.C`: write 1 to zero the cycle counter.
const PMCR_C: u64 = 1 << 2;
/// `PMCR_EL0.LC`: record cycle-counter overflow at 64 bits rather than 32.
const PMCR_LC: u64 = 1 << 6;
/// `PMCR_EL0.D`: count every 64th cycle. Asserted clear by the tests.
#[cfg(test)]
const PMCR_D: u64 = 1 << 3;
/// `PMCNTENSET_EL0.C`, and `PMCNTENCLR_EL0.C`: the cycle counter's enable bit.
const PMCNTEN_C: u64 = 1 << 31;

/// How long [`init_this_core`]'s check spins, in generic-timer ticks. 100 ticks is 1.6 us at QEMU's
/// 62.5 MHz and 5.2 us at the TX1's 19.2 MHz, long enough that a real cycle counter has moved by
/// thousands and short enough that four cores doing it are not felt in a boot. The riscv64 half
/// uses the same number for the same reason.
const CHECK_TICKS: u64 = 100;

/// **Why there is or is not a cycle counter on a core**, decided once per core by
/// [`init_this_core`].
///
/// The same vocabulary as `arch::riscv64::pmu::CycleCounter` and `arch::x86_64::pmu::CycleCounter`,
/// with the members this mechanism can actually produce. There is no firmware to refuse a counter
/// and no CSR to be out of range, so the list is short: the PMU is absent, or it was started and did
/// not move, or it runs.
///
/// Ordered so the discriminant is the stored value and `Unknown` is zero, which is what a core that
/// never ran [`init_this_core`] leaves behind.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum CycleCounter {
    /// [`init_this_core`] has not run on this core.
    Unknown = 0,
    /// `ID_AA64DFR0_EL1.PMUVer` is 0 (no PMU) or 0xf (an IMPLEMENTATION DEFINED one that is not
    /// `PMUv3`). `PMCCNTR_EL0` and every register that controls it are UNDEFINED, so nothing was
    /// written and nothing will be read.
    NoPmuV3,
    /// **Enabled, read twice across a timed spin, and unmoved.** A cycle counter cannot do that on a
    /// core that is executing, so it is refused rather than reported as zero. On a board this is
    /// the outcome a secure world that prohibits Non-secure counting produces (`MDCR_EL3.SPME` and
    /// friends), and the boot line says so.
    Stuck,
    /// Enabled and advancing.
    Running,
}

/// [`CycleCounter`] per core, as a byte. Written once by that core's [`init_this_core`] before the
/// core is counted online, read-only afterwards. Atomics rather than a lock: nothing here nests with
/// another lock, so it earns no rank in `sync::rank`.
static OUTCOME: [AtomicU8; MAX_CPUS] =
    [const { AtomicU8::new(CycleCounter::Unknown as u8) }; MAX_CPUS];

/// Each core's `PMCR_EL0` as read back after the enable, for the boot line: `N` (bits 15:11) is how
/// many event counters EL1 can see, which is `MDCR_EL2.HPMN` when an EL2 is implemented and so is
/// the visible trace of `boot.s` having set it.
static PMCR_READBACK: [AtomicU64; MAX_CPUS] = [const { AtomicU64::new(0) }; MAX_CPUS];

/// What the check on each core measured, cycles and generic-timer ticks over the same window, kept
/// for the boot line. **The ticks are what elapsed, not [`CHECK_TICKS`]**: the loop exits at the
/// first read past the threshold, and under TCG that first read can come milliseconds late (the
/// block is being translated), so printing the constant would divide by the wrong number. On the
/// `Stuck` outcome the window the counter failed to move across is the diagnostic.
static CHECK_CYCLES: [AtomicU64; MAX_CPUS] = [const { AtomicU64::new(0) }; MAX_CPUS];
static CHECK_ELAPSED: [AtomicU64; MAX_CPUS] = [const { AtomicU64::new(0) }; MAX_CPUS];

/// **What `PMCCFILTR_EL0` held before this kernel overwrote it**, per core, for the boot line.
///
/// This is evidence for calef's decision rather than state this kernel uses. seL4's published TX1
/// figures come from a build that never writes this register (`arm_init_ccnt` and libsel4bench's
/// `sel4bench_init` write it only under `CONFIG_ARM_HYPERVISOR_SUPPORT`, and the published build
/// line enables no hypervisor), so those numbers counted **whatever the TX1's firmware left here**.
/// argon runs the same family of firmware, so its first boot prints the best available answer to
/// "which exception levels did seL4's 413 and 426 include". An architecturally UNKNOWN reset value
/// is exactly what makes that worth recording rather than assuming.
static FIRMWARE_FILTER: [AtomicU64; MAX_CPUS] = [const { AtomicU64::new(0) }; MAX_CPUS];

/// **Does this core implement `PMUv3`**, and therefore every register this module and
/// `timer::close_cycle_counter_to_el0` write?
///
/// `ID_AA64DFR0_EL1.PMUVer`: 0 is no PMU, 0xf is an IMPLEMENTATION DEFINED PMU that does not follow
/// `PMUv3` and so carries none of these registers either. Per core, because `PMUVer` is a per-PE ID
/// register and this kernel does not assume a homogeneous machine anywhere else.
pub(super) fn is_pmuv3_present() -> bool {
    let pmuver = ID_AA64DFR0_EL1.read(ID_AA64DFR0_EL1::PMUVer);
    pmuver != 0 && pmuver != 0xf
}

/// **Start this core's cycle counter, check it moves, and record the answer.**
///
/// Called from `timer::init`, which every core runs (core 0 from `kernel_main`, each secondary from
/// `smp::secondary_main`), after `PMUSERENR_EL0` has been closed. Init-time only: nothing here is on
/// the context-switch path, which milestone 237 measured and keeps clean.
pub fn init_this_core() {
    let core = cpu::id();
    if !is_pmuv3_present() {
        OUTCOME[core].store(CycleCounter::NoPmuV3 as u8, Ordering::Release);
        return;
    }

    // What firmware left, before it is overwritten; see `FIRMWARE_FILTER` for why it is kept.
    let inherited: u64;
    // SAFETY: as the block below: PMUv3 is present, so the register exists; a read has no effect.
    unsafe {
        core::arch::asm!("mrs {}, pmccfiltr_el0", out(reg) inherited, options(nomem, nostack, preserves_flags));
    }
    FIRMWARE_FILTER[core].store(inherited, Ordering::Relaxed);

    // SAFETY: `is_pmuv3_present` read this core's own `ID_AA64DFR0_EL1.PMUVer` and found PMUv3, so
    // all three registers exist and an EL1 access is not UNDEFINED. `MDCR_EL2.TPM` is clear
    // (boot.s, when this kernel was entered at EL2) so the accesses do not trap to EL2. The values
    // change what the PMU counts and nothing else: no memory, no translation, no exception routing.
    // The `isb` makes the enable take effect before the check below reads the counter.
    unsafe {
        core::arch::asm!(
            "msr pmccfiltr_el0, {filter}",
            "msr pmcr_el0, {pmcr}",
            "msr pmcntenset_el0, {cnten}",
            "isb",
            filter = in(reg) PMCCFILTR_PROVISIONAL,
            pmcr = in(reg) PMCR_E | PMCR_C | PMCR_LC,
            cnten = in(reg) PMCNTEN_C,
            options(nomem, nostack, preserves_flags)
        );
    }
    PMCR_READBACK[core].store(read_pmcr(), Ordering::Relaxed);

    // Does it count? A short window on the generic timer, the one clock this kernel has that does
    // not depend on the answer.
    let (first, t0) = (read_counter(), CNTVCT_EL0.get());
    let mut t1 = t0;
    while t1.wrapping_sub(t0) < CHECK_TICKS {
        core::hint::spin_loop();
        t1 = CNTVCT_EL0.get();
    }
    let moved = read_counter().wrapping_sub(first);
    CHECK_CYCLES[core].store(moved, Ordering::Relaxed);
    CHECK_ELAPSED[core].store(t1.wrapping_sub(t0), Ordering::Relaxed);

    let outcome = if moved == 0 {
        CycleCounter::Stuck
    } else {
        CycleCounter::Running
    };
    // Last, and Release: a reader that sees the outcome sees the records written before it.
    OUTCOME[core].store(outcome as u8, Ordering::Release);
}

/// **Why there is or is not a cycle counter on this core.** See [`CycleCounter`].
pub fn outcome() -> CycleCounter {
    outcome_on(cpu::id())
}

/// [`outcome`] for a named core, for the boot line and for a test whose thread may run anywhere.
pub fn outcome_on(core: usize) -> CycleCounter {
    match OUTCOME[core].load(Ordering::Acquire) {
        x if x == CycleCounter::NoPmuV3 as u8 => CycleCounter::NoPmuV3,
        x if x == CycleCounter::Stuck as u8 => CycleCounter::Stuck,
        x if x == CycleCounter::Running as u8 => CycleCounter::Running,
        _ => CycleCounter::Unknown,
    }
}

/// **This core's cycle count**, or `None` when this core has no counter worth believing.
///
/// One `mrs`, no trap and no call, so a caller may put this on either side of the thing it is
/// measuring. **Only meaningful as a difference of two reads on the same core**: see this module's
/// BUGS. It counts whatever exception levels [`PMCCFILTR_PROVISIONAL`] says, which is not settled.
// The consumers are the bench probe (`bench::cycles_per_tick`, `--features bench`) and the tests. A
// production boot has nothing to measure, so it has no caller, the same call the other two halves
// make; the counter is still started and printed in every build, because whether this machine has
// one is a fact about the machine.
#[cfg_attr(not(any(test, feature = "bench")), allow(dead_code))]
pub fn cycles() -> Option<u64> {
    if outcome() != CycleCounter::Running {
        return None;
    }
    Some(read_counter())
}

/// One `mrs` of `PMCCNTR_EL0`. Only called once [`init_this_core`] found `PMUv3` on this core.
fn read_counter() -> u64 {
    let value: u64;
    // SAFETY: reached only on a core whose PMUVer reported PMUv3 (the outcome is not `NoPmuV3`), so
    // the register exists. A read has no side effect and touches no memory.
    unsafe {
        core::arch::asm!("mrs {}, pmccntr_el0", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// One `mrs` of `PMCR_EL0`, under the same precondition as [`read_counter`].
fn read_pmcr() -> u64 {
    let value: u64;
    // SAFETY: as `read_counter`: PMUv3 is present on this core, and the read has no side effect.
    unsafe {
        core::arch::asm!("mrs {}, pmcr_el0", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// The boot line. Printed once, from core 0, after every core has run [`init_this_core`].
///
/// It names core 0's answer in full and how many of the cores that reached init agree, because the
/// failure worth seeing on a board is one core disagreeing (a big.LITTLE part, or a secondary whose
/// firmware state differs), and a line about core 0 alone would hide it.
pub fn print_summary() {
    let boot = cpu::id();
    let mut initialised = 0;
    let mut running = 0;
    let mut first_other: Option<(usize, CycleCounter)> = None;
    for core in 0..MAX_CPUS {
        let o = outcome_on(core);
        if o == CycleCounter::Unknown {
            continue;
        }
        initialised += 1;
        if o == CycleCounter::Running {
            running += 1;
        }
        if first_other.is_none() && o != outcome_on(boot) {
            first_other = Some((core, o));
        }
    }

    let pmcr = PMCR_READBACK[boot].load(Ordering::Relaxed);
    let event_counters = (pmcr >> 11) & 0x1f;
    let checked = CHECK_CYCLES[boot].load(Ordering::Relaxed);
    let elapsed = CHECK_ELAPSED[boot].load(Ordering::Relaxed);
    let inherited = FIRMWARE_FILTER[boot].load(Ordering::Relaxed);
    match outcome_on(boot) {
        CycleCounter::Running => crate::println!(
            "  cycles      : PMCCNTR_EL0 running on {running} of {initialised} cores ({checked} over \
             {elapsed} ticks at boot), {event_counters} event counters visible, PMCCFILTR_EL0 \
             {PMCCFILTR_PROVISIONAL:#x} PROVISIONAL (EL0+EL1 counted, EL2 not)"
        ),
        CycleCounter::Stuck => crate::println!(
            "  cycles      : PMCCNTR_EL0 enabled but did not advance over {elapsed} ticks; refused \
             ({running} of {initialised} cores running)"
        ),
        CycleCounter::NoPmuV3 => crate::println!(
            "  cycles      : no PMUv3 (ID_AA64DFR0_EL1.PMUVer {:#x}); ticks only",
            ID_AA64DFR0_EL1.read(ID_AA64DFR0_EL1::PMUVer)
        ),
        CycleCounter::Unknown => {
            crate::println!(
                "  cycles      : arch::pmu::init_this_core has not run on the boot core"
            );
        }
    }
    if outcome_on(boot) != CycleCounter::NoPmuV3 {
        // The one number that says what seL4's published TX1 figures counted; see `FIRMWARE_FILTER`.
        crate::println!(
            "  cycles      : firmware left PMCCFILTR_EL0 {inherited:#x} on this core before it was overwritten"
        );
    }
    if let Some((core, o)) = first_other {
        crate::println!("  cycles      : core {core} disagrees with the boot core: {o:?}");
    }
}

#[cfg(test)]
mod tests {
    //! What only a boot can say. **None of these is a claim about cycles**: under QEMU-TCG the
    //! counter runs off the emulator's virtual clock, so every assertion is about the plumbing, that
    //! the registers hold what was written, that the outcome is decided on every core that ran init,
    //! and that a counter called `Running` moves.

    use super::*;

    /// **Every core that came online decided an answer, and the answer agrees with the ID register.**
    ///
    /// `Unknown` on the boot core would mean `timer::init` never reached this module. A `NoPmuV3`
    /// on a core that reports `PMUv3`, or the reverse, would mean the two gates (this module's and
    /// `timer`'s) disagree about which registers exist, which on a board is an undefined-instruction
    /// fault rather than a wrong number.
    #[test_case]
    fn every_core_decided_and_the_boot_core_agrees_with_pmuver() {
        let here = outcome();
        assert_ne!(
            here,
            CycleCounter::Unknown,
            "timer::init did not run init_this_core here"
        );
        assert_eq!(
            here == CycleCounter::NoPmuV3,
            !is_pmuv3_present(),
            "the outcome ({here:?}) and ID_AA64DFR0_EL1.PMUVer disagree about whether there is a PMU"
        );
        assert_eq!(cycles().is_some(), here == CycleCounter::Running);

        for core in crate::smp::online_cpus() {
            assert_ne!(
                outcome_on(core),
                CycleCounter::Unknown,
                "core {core} is online and never ran init_this_core: its counter is whatever reset left"
            );
        }
    }

    /// **The three registers hold what [`init_this_core`] wrote.**
    ///
    /// Read back from the core rather than trusted, because the failure this catches is the one
    /// nothing else would: an enable that the hardware (or an emulator) silently did not take, or a
    /// `D` bit that survived and makes every count 64 times too small.
    #[test_case]
    fn the_counter_is_enabled_undivided_and_filtered_as_written() {
        if !is_pmuv3_present() {
            crate::testing::skip!("this core has no PMUv3, so there are no registers to read back");
        }
        let pmcr = read_pmcr();
        assert_ne!(
            pmcr & PMCR_E,
            0,
            "PMCR_EL0.E is clear: the counter is stopped"
        );
        assert_eq!(
            pmcr & PMCR_D,
            0,
            "PMCR_EL0.D is set: the counter divides by 64"
        );

        let (cnten, filter): (u64, u64);
        // SAFETY: PMUv3 is present on this core (checked above), so both registers exist; reads have
        // no side effect.
        unsafe {
            core::arch::asm!(
                "mrs {}, pmcntenset_el0",
                "mrs {}, pmccfiltr_el0",
                out(reg) cnten,
                out(reg) filter,
                options(nomem, nostack, preserves_flags)
            );
        }
        assert_ne!(
            cnten & PMCNTEN_C,
            0,
            "PMCNTENSET_EL0.C is clear: the cycle counter is not enabled"
        );
        assert_eq!(
            filter, PMCCFILTR_PROVISIONAL,
            "PMCCFILTR_EL0 does not hold the value init wrote"
        );
    }

    /// **QEMU reaches one of the answers it can give, and `cycles` agrees with the gate.**
    ///
    /// A set rather than one value, for the reason the riscv64 twin learned: what an emulator answers
    /// is a property of its configuration. `cortex-a72` under TCG models `PMUv3` and drives the counter
    /// from its virtual clock, so `Running` is expected there; `Stuck` is what a model that left the
    /// counter constant would produce, and is handled. What must never happen is a number handed
    /// back under any outcome but `Running`.
    #[test_case]
    fn qemu_reaches_a_decided_answer() {
        let o = outcome();
        assert!(
            matches!(
                o,
                CycleCounter::Running | CycleCounter::Stuck | CycleCounter::NoPmuV3
            ),
            "outcome {o:?} on a booted core"
        );
        assert_eq!(
            cycles().is_some(),
            o == CycleCounter::Running,
            "{o:?} handed back a count"
        );
    }

    /// **A `Running` counter still moves.** [`init_this_core`] checked once at boot; this re-checks
    /// after the whole boot has run, which is what would catch something later stopping it (a stray
    /// `PMCR_EL0` write, or a `PMCNTENCLR` from code that thinks it owns the PMU).
    ///
    /// Not a measurement: under TCG the delta is virtual time. Interrupts are masked across the two
    /// reads so both are on this core.
    #[test_case]
    fn a_running_counter_advances() {
        let irqs = crate::arch::interrupts::disable();
        let Some(first) = cycles() else {
            crate::arch::interrupts::restore(irqs);
            return;
        };
        let t0 = CNTVCT_EL0.get();
        while CNTVCT_EL0.get().wrapping_sub(t0) < 1000 {
            core::hint::spin_loop();
        }
        let second = cycles().expect("the counter did not vanish mid-test");
        crate::arch::interrupts::restore(irqs);
        assert!(
            second.wrapping_sub(first) > 0,
            "PMCCNTR_EL0 read {first} twice across 1000 ticks; something stopped it after boot"
        );
    }
}
