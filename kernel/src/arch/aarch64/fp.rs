//! **A thread's floating-point and SIMD registers, aarch64.** The register file itself, the two
//! control-register writes that decide who may touch it, and nothing else: the policy that says
//! *when* to move it lives in the portable [`crate::fp`], which is the only caller.
//!
//! # What this architecture gives you, and what it does not
//!
//! `CPACR_EL1.FPEN` is a two-bit field that decides whether FP and Advanced SIMD instructions
//! execute or trap, per exception level. `0b00` traps at EL0 **and** EL1; `0b11` traps at neither.
//! Those are the only two values this kernel uses, and [`enable`] and [`disable`] are the whole of
//! the mechanism.
//!
//! What aarch64 does **not** give you is a dirty bit. RISC-V has `sstatus.FS`, a four-state machine
//! that says in hardware whether the registers have been written since they were last loaded, which
//! is exactly the question a context switch wants to ask. Here there is no such bit, so the
//! question has to be asked a step earlier and coarser: *has this thread ever executed an FP
//! instruction at all?* The answer is recorded in [`FpState::live`], it is set by the trap `FPEN =
//! 0b00` produces on the first such instruction, and it never goes back to false. That is a weaker
//! approximation than RISC-V's and it is the best the ISA offers.
//!
//! # BUGS
//!
//! - **`live` is a ratchet: a thread that touches FP once pays the save on every switch for the
//!   rest of its life**, even if it never touches FP again. Clearing it would mean knowing the
//!   registers are dead, which nothing here can know. RISC-V's `FS` could do better and this
//!   implementation does not exploit that either; see [`crate::fp`] for why the policy is uniform.
//! - **SVE and SME are not saved, and [`init`] now closes them rather than trusting reset**
//!   (2026-09-24 security audit). `CPACR_EL1.ZEN` and `CPACR_EL1.SMEN` were left at their reset
//!   values, which on every machine this kernel has run on (QEMU's `cortex-a72`, HVF's Apple core,
//!   argon's A78AE) means trapped, and none of those parts has SVE at all. The architecture says the
//!   reset value is UNKNOWN, so a part that reset them open would let a thread keep `Z`/`P`/`ZA`
//!   state that [`crate::fp::hand_over`] neither saves nor scrubs. `init` writes both fields to
//!   trap alongside `FPEN`; on a part without SVE the bits are RES0 and the write is a no-op. A
//!   thread that then executes an SVE instruction takes the same trap FP does, and `crate::fp`
//!   treats it as a fault, because nothing here can save what it would enable. Nothing in this tree
//!   emits SVE. No test can see this on the emulator's default CPU; it is a one-instruction closure
//!   of a window the emulator cannot open.

use super::instructions;

/// **How many vector registers this architecture saves**: `q0`-`q31`.
///
/// Read by the tests in [`crate::fp`], which name the register that failed rather than reporting
/// that one did, and which therefore cannot hard-code a count that is 32 here, 32 on RISC-V and 16
/// on `x86_64`.
#[cfg(any(test, feature = "system_tests"))]
pub const REGISTERS: usize = 32;

/// **The whole of a thread's FP/SIMD register file**, plus the flag that says whether any of it is
/// worth moving.
///
/// `#[repr(C, align(16))]` and the padding are a contract with `fp.s`, which reaches the vector
/// area at a fixed offset of 16 and needs it 16-byte aligned for `stp q`. Reorder a field here and
/// the assembly writes the flag with `q0`.
#[repr(C, align(16))]
#[derive(Clone, Copy)]
pub struct FpState {
    /// Nonzero once this thread has executed an FP or SIMD instruction. **The save and the restore
    /// are both conditioned on it**, which is what makes a kernel full of soft-float threads pay
    /// two loads and two branches per switch instead of a kilobyte of memory traffic.
    live: u64,
    /// Padding, so the vector area below lands at offset 16 and 16-byte aligned. Never read.
    _pad: u64,
    /// `q0`-`q31`. 512 bytes, and the reason this file exists.
    q: [u128; 32],
    /// `FPCR`: rounding mode, exception masks, flush-to-zero. Architectural state, so it moves.
    fpcr: u64,
    /// `FPSR`: the cumulative exception flags a program reads to find out what its arithmetic did.
    fpsr: u64,
}

const _: () = assert!(size_of::<FpState>() == 544);
const _: () = assert!(core::mem::offset_of!(FpState, q) == 16);
const _: () = assert!(core::mem::offset_of!(FpState, fpcr) == 528);

impl FpState {
    /// **The state a thread starts with, and the state the registers are scrubbed to.**
    ///
    /// Zeros throughout, which for `FPCR` means round-to-nearest with every exception trap disabled
    /// and flush-to-zero off: the architectural reset configuration, and what a program compiled by
    /// anything sane expects to find. It doubles as the scrub value, so
    /// [`crate::fp::hand_over`] has exactly one thing to install when it takes the registers away
    /// from a thread rather than a second code path that zeroes them.
    pub const INITIAL: Self = FpState {
        live: 0,
        _pad: 0,
        q: [0; 32],
        fpcr: 0,
        fpsr: 0,
    };

    /// A fresh thread's state: no registers worth saving, and FP trapped until it asks.
    pub const fn new() -> Self {
        Self::INITIAL
    }

    /// Has this thread ever executed an FP instruction?
    pub fn is_live(&self) -> bool {
        self.live != 0
    }

    /// Record that it has. Called once, from the trap the disabled `FPEN` produced.
    pub fn set_live(&mut self) {
        self.live = 1;
    }
}

impl Default for FpState {
    fn default() -> Self {
        Self::new()
    }
}

/// `CPACR_EL1.FPEN`, bits 21:20. `0b11` here means "do not trap"; `0b00` means "trap at both EL0
/// and EL1".
const FPEN: u64 = 0b11 << 20;

/// `CPACR_EL1.ZEN` (bits 17:16) and `CPACR_EL1.SMEN` (bits 25:24): the SVE and SME enables, which
/// [`init`] closes with `FPEN` because this module saves neither. RES0 on a part without the
/// extension, so clearing them is a no-op there.
const ZEN_AND_SMEN: u64 = (0b11 << 16) | (0b11 << 24);

/// **Put this core into the state the rest of this module assumes**: FP and SIMD trapped, for EL1
/// as well as EL0.
///
/// Called from `sched::init` and `sched::adopt_secondary_idle`, per core rather than once per
/// machine: `CPACR_EL1` is banked, and a secondary that skipped this would run its threads with
/// whatever its reset left behind. The reason it is there rather than in `arch::init` is at the
/// call site, and it is RISC-V's boot path rather than this one's.
///
/// **Written rather than assumed, and that is the point.** QEMU resets `CPACR_EL1` to zero, which
/// is already the value this wants, so on the emulator this line changes nothing and could be
/// deleted without a test noticing. The architecture says the reset value is UNKNOWN. A part that
/// reset it to `0b11` would run every thread with FP open, [`crate::fp`] would never see a trap,
/// `live` would stay false on every thread, and two threads would quietly share a register file:
/// the exact failure this milestone exists to prevent, on hardware, silently, with the emulator
/// green.
pub fn init() {
    instructions::write_cpacr_synchronized(instructions::read_cpacr() & !(FPEN | ZEN_AND_SMEN));
}

/// Let the current core execute FP and SIMD instructions.
///
/// The `isb` is not decoration. `CPACR_EL1` is a context-synchronizing control register: without
/// it, an FP instruction already in the pipeline may be resolved against the old value, and the
/// caller here is about to execute thirty-two `ldp q` in [`restore`].
pub fn enable() {
    instructions::write_cpacr_synchronized(instructions::read_cpacr() | FPEN);
}

/// Is the FP unit open on this core right now?
///
/// Read by `crate::fp::enable_for_current`, which asks whether the enable it just wrote took (it
/// does not on a RISC-V hart with no FP unit, and the question is asked uniformly), and by the test
/// helpers in [`crate::fp`], which borrow the register file and have to put the core back the way
/// they found it. **Nothing on the switch path asks**: there the answer is `live`, which is a fact
/// about a thread rather than about a core, and reading `CPACR_EL1` to learn it would be a
/// system-register read per switch for something already in memory.
pub fn is_enabled() -> bool {
    instructions::read_cpacr() & FPEN != 0
}

/// Trap FP and SIMD again, at both exception levels.
///
/// **The caller owes the scrub, not this function.** Disabling the unit does not clear the
/// registers, and a thread that cannot read them today may be able to read them tomorrow through a
/// speculation window nobody has thought of yet; that is the whole of CVE-2018-3665. The invariant
/// [`crate::fp`] keeps is that the file holds the running thread's data or the initial state, and
/// it installs the latter before calling this.
pub fn disable() {
    instructions::write_cpacr_synchronized(instructions::read_cpacr() & !FPEN);
}

/// Copy the live register file into `state`.
///
/// # Safety
/// `state` must be a valid, writable, 16-byte-aligned `FpState`, and FP must be enabled on this
/// core. Both are the caller's, and the only caller is [`crate::fp::hand_over`].
pub unsafe fn save(state: *mut FpState) {
    // SAFETY: forwarded. `fp_save` writes 544 bytes at `state` and touches nothing else.
    unsafe { fp_save(state) }
}

/// Load `state` into the live register file.
///
/// # Safety
/// `state` must be a valid, readable, 16-byte-aligned `FpState`, and FP must be enabled on this
/// core. Both are the caller's.
pub unsafe fn restore(state: *const FpState) {
    // SAFETY: forwarded. `fp_restore` reads 544 bytes at `state` and writes only registers.
    unsafe { fp_restore(state) }
}

unsafe extern "C" {
    /// `q0`-`q31`, `FPCR`, `FPSR` out to memory. See fp.s.
    fn fp_save(state: *mut FpState);
    /// The mirror. See fp.s.
    fn fp_restore(state: *const FpState);
}

/// **One harmless FP instruction, to take the first-use trap on purpose.**
///
/// `fmov d0, xzr` writes zero into a register [`crate::fp::load_pattern`] is about to overwrite
/// anyway, so it has no effect beyond the trap it provokes. Tests only: the shipping kernel is
/// `softfloat` and asks for the unit nowhere.
#[cfg(any(test, feature = "system_tests"))]
pub fn touch() {
    // SAFETY: writes one vector register. Under a trapping `CPACR_EL1.FPEN` this takes the enable
    // trap and is re-executed after it, which is the entire point of calling it.
    unsafe {
        core::arch::asm!(
            ".arch_extension fp",
            "fmov d0, xzr",
            ".arch_extension nofp",
            options(nomem, nostack, preserves_flags),
        );
    }
}

#[cfg(any(test, feature = "system_tests"))]
impl FpState {
    /// Write [`crate::fp::register_pattern`] across all 32 lanes, each one's complement in the high
    /// half so the full 128 bits are exercised rather than only the `d` half a scalar `double`
    /// would occupy.
    pub fn set_pattern(&mut self, seed: u64) {
        for (index, lane) in self.q.iter_mut().enumerate() {
            let low = crate::fp::register_pattern(seed, index);
            *lane = (u128::from(!low) << 64) | u128::from(low);
        }
    }

    /// Is [`Self::set_pattern`]'s pattern still there, every lane of it?
    pub fn has_pattern(&self, seed: u64) -> bool {
        self.q.iter().enumerate().all(|(index, &lane)| {
            let low = crate::fp::register_pattern(seed, index);
            lane == (u128::from(!low) << 64) | u128::from(low)
        })
    }

    /// The low 64 bits of one lane, so a test can name the register that went wrong rather than
    /// reporting that something did.
    pub fn lane_low(&self, index: usize) -> u64 {
        self.q[index] as u64
    }

    /// Do the registers hold [`Self::INITIAL`]'s values, i.e. has the file been scrubbed?
    pub fn is_scrubbed(&self) -> bool {
        self.q.iter().all(|&lane| lane == 0) && self.fpcr == 0 && self.fpsr == 0
    }
}

/// Proofs of the `CPACR_EL1` edits, reachable since 2026-09-25 because the `mrs` and `msr` they make
/// are functions in [`instructions`] that a harness can stub. Until then each edit was its arithmetic
/// and its two instructions in one `asm!` block, and the arithmetic was unprovable.
///
/// # What the stub assumes
///
/// `CPACR_EL1` is modelled as a plain 64-bit register: a write is visible to the next read, exactly,
/// every bit. On real parts some fields are RES0 (`ZEN` without SVE, `SMEN` without SME) and read as
/// zero whatever was written; the model does not do that, so these proofs say nothing about which
/// fields exist, only that the code never writes one it did not mean to. The `isb` inside
/// [`instructions::write_cpacr_synchronized`] is assumed to do its job; the model has no pipeline.
/// See notes/kernel-proofs/stubbing-an-instruction.md.
#[cfg(kani)]
mod proofs {
    use core::sync::atomic::AtomicU64;
    use core::sync::atomic::Ordering::Relaxed;

    use super::*;

    /// The modelled `CPACR_EL1`.
    static CPACR: AtomicU64 = AtomicU64::new(0);

    #[allow(dead_code)] // named only by `#[kani::stub]`, which rustc cannot see
    fn read_cpacr_model() -> u64 {
        CPACR.load(Relaxed)
    }

    #[allow(dead_code)] // named only by `#[kani::stub]`
    fn write_cpacr_model(cpacr: u64) {
        CPACR.store(cpacr, Relaxed);
    }

    /// **Every `CPACR_EL1` edit changes exactly the field it names, and [`is_enabled`] reads back
    /// what the last one did.**
    ///
    /// The register holds three independent enables (FP/SIMD, SVE, SME) and a trace bit, and the
    /// module's whole guarantee rests on them moving separately: [`init`] closes all three, while
    /// [`enable`] and [`disable`] run on every first-use trap and hand-over and must touch `FPEN`
    /// alone. A mask with one bit wrong would either leave SVE open (a thread keeps vector state
    /// nobody saves) or trap FP in a thread that owns the unit, and QEMU resets the register to zero,
    /// where most wrong masks are invisible. Checked for every starting value of the register.
    ///
    /// Falsification: attested 2026-09-25. On patagonia (aarch64 host), three times and one edit at
    /// a time: `init`'s mask without `ZEN_AND_SMEN`, `enable` writing `FPEN >> 1` (half the field),
    /// and `disable` clearing `FPEN | ZEN_AND_SMEN`. Each turns this red. `attested` rather than
    /// `replayable` for the reason `script/falsifications` gives for every architecture-specific
    /// harness.
    #[kani::proof]
    #[kani::stub(super::super::instructions::read_cpacr, read_cpacr_model)]
    #[kani::stub(
        super::super::instructions::write_cpacr_synchronized,
        write_cpacr_model
    )]
    fn every_cpacr_edit_changes_exactly_the_field_it_names() {
        let before: u64 = kani::any();
        CPACR.store(before, Relaxed);

        let which: u8 = kani::any();
        kani::assume(which < 3);
        let (touched, want) = match which {
            0 => {
                init();
                (FPEN | ZEN_AND_SMEN, 0)
            }
            1 => {
                enable();
                (FPEN, FPEN)
            }
            _ => {
                disable();
                (FPEN, 0)
            }
        };

        let after = CPACR.load(Relaxed);
        assert!(
            after & touched == want,
            "the named field did not reach its value"
        );
        assert!(
            after & !touched == before & !touched,
            "an edit moved a field it does not name"
        );
        assert!(
            is_enabled() == (which == 1),
            "is_enabled disagrees with the last edit"
        );
    }
}
