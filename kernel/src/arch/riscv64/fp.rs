//! **A thread's floating-point registers, RISC-V.** The register file, the `sstatus.FS` writes that
//! decide who may touch it, and nothing else: the policy is [`crate::fp`]'s.
//!
//! # `sstatus.FS` is the best of the three, and this kernel uses a quarter of it
//!
//! Where aarch64 has an on/off switch and x86 has a one-bit "somebody else owns this", RISC-V has a
//! **four-state machine** in `sstatus[14:13]`, designed by people who had context switching in mind:
//!
//! | value | name | meaning |
//! |---|---|---|
//! | `0b00` | Off | FP instructions raise an illegal-instruction trap |
//! | `0b01` | Initial | the registers hold their initial values |
//! | `0b10` | Clean | the registers match the copy in memory |
//! | `0b11` | Dirty | the hardware set this because something *wrote* an FP register |
//!
//! Dirty is a hardware-maintained dirty bit. A kernel that keeps a thread's saved copy in a stable
//! place can ask "have these registers been written since I loaded them" and skip the save when the
//! answer is no, which is a strictly better question than the one aarch64 can answer.
//!
//! **This kernel asks the coarse question anyway**, using `Off` against everything else as a plain
//! enable bit, because [`crate::fp`]'s rule is uniform across three ISAs and no workload exists yet
//! to measure the difference on. That is a deliberate cost, recorded in that module's `BUGS` and
//! here, rather than an oversight.
//!
//! # `sstatus` travels in the trap frame, and that is the trap
//!
//! On the other two architectures the control register this module writes is invisible to the trap
//! path. Here it is not: `TrapFrame::sstatus` is saved on trap entry and written back by `trap.s`'s
//! `csrw sstatus, t0` on the way out, so a handler that sets `FS` in the live CSR and returns has
//! its write undone by its own return. The first-use handler in `exceptions.rs` therefore sets the
//! **frame's** copy as well, and it is the only place that has to think about this: every later
//! switch sets the live CSR from [`crate::fp::hand_over`] to match the incoming thread's `live`,
//! and that thread's frame carries the same value because it was captured while the same flag held.
//!
//! # BUGS
//!
//! - **A hart with no D extension cannot run this at all**, and says so rather than looping.
//!   `sstatus.FS` is hardwired to zero on such a part, so [`enable`] does not take, [`is_enabled`]
//!   reports it, and `crate::fp::enable_for_current` turns the first-use trap into an ordinary
//!   fault. The thread dies with an illegal instruction, which is the truth. Nothing tests this:
//!   every machine in this tree's matrix (`qemu-system-riscv64 -cpu rv64`, the JH7110's U74) has D.
//! - **F without D is not handled.** A part with F only has 32-bit registers where [`fp.s`] writes
//!   64, so the save would be wrong rather than refused. `arch::isa` already reads the ISA string
//!   and is where that check belongs; it is recorded rather than built.

use super::instructions;

/// **How many FP registers this architecture saves**: `f0`-`f31`. See the aarch64 twin.
#[cfg(any(test, feature = "system_tests"))]
pub const REGISTERS: usize = 32;

/// **A thread's FP register file**, plus the flag that says whether any of it is worth moving.
///
/// `#[repr(C, align(16))]` is a contract with `fp.s`, which reaches `fcsr` at offset 8 and `f0` at
/// offset 16.
#[repr(C, align(16))]
#[derive(Clone, Copy)]
pub struct FpState {
    /// Nonzero once this thread has executed an FP instruction. See the aarch64 twin: the save and
    /// the restore are both conditioned on it.
    live: u64,
    /// `fcsr`: the rounding mode and the accrued exception flags, in one CSR. aarch64 splits the
    /// same information across `FPCR` and `FPSR`.
    fcsr: u64,
    /// `f0`-`f31`, 64 bits each under D. 256 bytes, and the reason this file exists.
    f: [u64; 32],
}

const _: () = assert!(size_of::<FpState>() == 272);
const _: () = assert!(core::mem::offset_of!(FpState, fcsr) == 8);
const _: () = assert!(core::mem::offset_of!(FpState, f) == 16);

impl FpState {
    /// The state a thread starts with, and the state the registers are scrubbed to: zeros, which is
    /// what `FS == Initial` means in the table above, and round-to-nearest-even with no accrued
    /// exceptions in `fcsr`.
    pub const INITIAL: Self = FpState {
        live: 0,
        fcsr: 0,
        f: [0; 32],
    };

    /// A fresh thread's state.
    pub const fn new() -> Self {
        Self::INITIAL
    }

    /// Has this thread ever executed an FP instruction?
    pub fn is_live(&self) -> bool {
        self.live != 0
    }

    /// Record that it has. Called once, from the first-use trap.
    pub fn set_live(&mut self) {
        self.live = 1;
    }
}

impl Default for FpState {
    fn default() -> Self {
        Self::new()
    }
}

/// `sstatus.FS`, bits 14:13. Zero is Off; anything else lets FP instructions execute.
pub(super) const SSTATUS_FS: u64 = 0b11 << 13;
/// `FS = Initial`: the unit is open and the registers hold their initial values.
const SSTATUS_FS_INITIAL: u64 = 0b01 << 13;
/// `FS = Dirty`: what the hardware sets the instant an FP register is written, and therefore what
/// the first-use handler writes into the trap frame once it has loaded a register file.
pub(super) const SSTATUS_FS_DIRTY: u64 = 0b11 << 13;
/// `sstatus.VS`, bits 10:9: the vector extension's own enable, the same four-state field as `FS`.
/// This module saves no vector register, so [`init`] turns it Off beside `FS` (2026-09-24 security
/// audit). Hardwired to zero on a hart without V (QEMU's `rv64` default, radon's U74), so the
/// clear is a no-op there; on a hart that arrives from firmware with it open, a thread could
/// otherwise keep vector state that `hand_over` neither saves nor scrubs.
const SSTATUS_VS: u64 = 0b11 << 9;

/// **Put this hart into the state the rest of this module assumes**: FP off.
///
/// Called from `sched::init` and `sched::adopt_secondary_idle`, per hart, rather than from
/// `arch::init`. **This architecture is the reason for that choice**: `main`'s RISC-V tour installs
/// `stvec` by calling `arch::exceptions::init()` directly and never passes through `arch::init` at
/// all, so a hart booted by OpenSBI reached `sched::init` with `sstatus.FS` already set, no thread
/// ever took the first-use trap, and two threads shared a register file. The reasoning is at the
/// call site.
pub fn init() {
    disable();
    instructions::clear_sstatus(SSTATUS_VS);
}

/// Let this hart execute FP instructions, by moving `FS` out of Off.
///
/// `csrs` rather than a read-modify-write: setting bit 13 takes Off to Initial and leaves Clean or
/// Dirty alone, which is exactly the "at least open" this wants. No fence is needed; a CSR write is
/// ordered with respect to the instructions after it.
pub fn enable() {
    // On a hart with no FP unit the field is hardwired to zero and this is a no-op, which
    // [`is_enabled`] is how the caller finds out.
    instructions::set_sstatus(SSTATUS_FS_INITIAL);
}

/// Is the FP unit open on this hart right now?
///
/// Also the "does this hart have one at all" probe: on a part without D the field is hardwired to
/// zero, so this reports false immediately after [`enable`] and `crate::fp::enable_for_current`
/// turns the first-use trap into a fault rather than retrying an instruction that will never
/// execute.
pub fn is_enabled() -> bool {
    instructions::read_sstatus() & SSTATUS_FS != 0
}

/// Take the unit away again: `FS = Off`.
///
/// **The caller owes the scrub**, as on every architecture here. See [`crate::fp`]'s header for the
/// CVE that makes that sentence load-bearing rather than tidy.
pub fn disable() {
    instructions::clear_sstatus(SSTATUS_FS);
}

/// Copy the live register file into `state`.
///
/// # Safety
/// `state` must be a valid, writable, 16-byte-aligned `FpState`, and `sstatus.FS` must not be Off.
/// The only caller is [`crate::fp::hand_over`].
pub unsafe fn save(state: *mut FpState) {
    // SAFETY: forwarded. `fp_save` writes 272 bytes at `state` and touches nothing else.
    unsafe { fp_save(state) }
}

/// Load `state` into the live register file.
///
/// # Safety
/// As [`save`], readable rather than writable.
pub unsafe fn restore(state: *const FpState) {
    // SAFETY: forwarded. `fp_restore` reads 272 bytes at `state` and writes only registers.
    unsafe { fp_restore(state) }
}

unsafe extern "C" {
    /// `f0`-`f31` and `fcsr` out to memory. See fp.s.
    fn fp_save(state: *mut FpState);
    /// The mirror. See fp.s.
    fn fp_restore(state: *const FpState);
}

/// **One harmless FP instruction, to take the first-use trap on purpose.** Tests only.
///
/// `fmv.d.x f0, zero` writes zero into a register the caller is about to overwrite anyway.
#[cfg(any(test, feature = "system_tests"))]
pub fn touch() {
    // SAFETY: writes one FP register. Under `FS == Off` this raises the illegal-instruction trap
    // that `exceptions.rs` turns into an enable, and is re-executed after it.
    unsafe {
        core::arch::asm!(
            ".option push",
            ".option arch, +d",
            "fmv.d.x f0, zero",
            ".option pop",
            options(nomem, nostack, preserves_flags),
        );
    }
}

#[cfg(any(test, feature = "system_tests"))]
impl FpState {
    /// Write [`crate::fp::register_pattern`] across all 32 registers.
    pub fn set_pattern(&mut self, seed: u64) {
        for (index, register) in self.f.iter_mut().enumerate() {
            *register = crate::fp::register_pattern(seed, index);
        }
    }

    /// Is that pattern still there, in every register?
    pub fn has_pattern(&self, seed: u64) -> bool {
        self.f
            .iter()
            .enumerate()
            .all(|(index, &value)| value == crate::fp::register_pattern(seed, index))
    }

    /// One register, so a test can name the one that went wrong.
    pub fn lane_low(&self, index: usize) -> u64 {
        self.f[index]
    }

    /// Do the registers hold [`Self::INITIAL`]'s values, i.e. has the file been scrubbed?
    pub fn is_scrubbed(&self) -> bool {
        self.f.iter().all(|&value| value == 0) && self.fcsr == 0
    }
}
