//! **A thread reading the CPU's cycle counter from user mode**, milestone 229.
//!
//! Reads the counter twice and reports [`capability_witness_protocol::CYCLE_COUNTER_WORD`] plus both
//! reads. Holds a report endpoint (slot 0) and nothing else: the grant is **not** a capability in
//! a slot, it is a property of this thread that the context switch writes into a system register
//! before the thread runs.
//!
//! **Getting here at all is the result.** Without the grant the read is an EL0 access that
//! `PMUSERENR_EL0` (aarch64) or `scounteren` (riscv64) does not permit, which traps and kills the
//! thread, and nothing is sent. The kernel-side test runs this program both ways and asserts each
//! outcome.
//!
//! **The `yield` between the two reads is the point of there being two.** The first read proves
//! the grant reached a thread that had not yet been switched; the yield gives the scheduler a
//! chance to switch this thread out and back in, so the second read is one taken *after* the
//! context switch re-applied the grant from the thread's own field. A yield is not a guarantee
//! that a switch happened (this may be the only runnable thread on the core), so the second read
//! is evidence rather than proof, and the register-level assertions in `arch::*::timer`'s tests
//! are what prove the write itself.
//!
//! # Bugs
//!
//! **The counter values are carried here and checked on the kernel side, and only partly.** Since
//! milestone 74's aarch64 half the kernel starts `PMCCNTR_EL0` on every core, so the kernel's test
//! asserts the second read is past the first wherever `arch::pmu` reports the counter running (and
//! always on `x86_64`). It does not assert on riscv64, where the kernel's counter and the `cycle` CSR
//! this program reads may not be the same counter. Nothing asserts on the size of the difference:
//! under QEMU it is emulator time.
//!
//! Name: provisional (milestone 291). This was `hello`'s `CYCLE_COUNTER_CHILD` role, number 42.
//! Refused keeping `_child`: nothing builds this program as a child. The kernel's own test starts
//! it directly, because milestone 229 shipped the grant mechanism without a syscall method to set
//! it and there is therefore no userspace route to a granted thread; `_child` named a relationship
//! that does not exist.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

use user_mode_runtime::{exit, send, yield_now};

const REPORT: u64 = 0;

#[unsafe(no_mangle)]
pub extern "C" fn _start(_arg0: u64, _arg1: u64, _arg2: u64) -> ! {
    let first = read_cycle_counter();
    yield_now();
    let second = read_cycle_counter();
    send(
        REPORT,
        capability_witness_protocol::CYCLE_COUNTER_WORD,
        first,
        second,
    );
    exit()
}

/// Read the CPU's cycle counter from user mode: one instruction on every architecture, which is
/// the property DECISIONS 139 chose option 4 to keep.
///
/// **Deliberately not in `crates/user_mode_runtime`.** A portable userspace cycle-counter API is milestone
/// 74's deliverable, and it will want to say what the number means (a frequency, a scaling, a
/// story about what a "cycle" is on a big.LITTLE part). This is the raw read, in the one program
/// that needs it today, so that 74 designs the API rather than inheriting one from a test vehicle.
/// **Its name and promise are an architect's**, and the options are in
/// design/roadmap/proposals/the-aarch64-half-of-74.md; the aarch64 half of 74 started the counter
/// and left this read where it was on purpose.
#[cfg(target_arch = "aarch64")]
fn read_cycle_counter() -> u64 {
    let value: u64;
    // SAFETY: `mrs` from `PMCCNTR_EL0` reads a counter and touches no memory, which the options
    // state. It is UNDEFINED at EL0 unless `PMUSERENR_EL0` permits it, which is exactly what this
    // program exists to have been granted; an ungranted thread faults here, and that is the
    // negative half of the test rather than an accident.
    unsafe {
        core::arch::asm!("mrs {}, pmccntr_el0", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// **The `x86_64` half, which needs no grant at all.** `rdtsc` is ambient in ring 3 on this
/// architecture and DECISIONS 139 part 3 kept it that way, so this reads without asking.
#[cfg(target_arch = "x86_64")]
fn read_cycle_counter() -> u64 {
    let low: u32;
    let high: u32;
    // SAFETY: `rdtsc` reads the time-stamp counter into `edx:eax` and touches no memory. `CR4.TSD`
    // is clear (milestone 228 read it back rather than assuming it), so this is legal in ring 3.
    unsafe {
        core::arch::asm!("rdtsc", out("eax") low, out("edx") high, options(nomem, nostack, preserves_flags));
    }
    ((high as u64) << 32) | low as u64
}

/// The riscv64 half: the `cycle` CSR, gated by `scounteren.CY`.
#[cfg(target_arch = "riscv64")]
fn read_cycle_counter() -> u64 {
    let value: u64;
    // SAFETY: `csrr` from the `cycle` CSR reads a counter and touches no memory. It is an illegal
    // instruction in U-mode unless `scounteren.CY` permits it; see the aarch64 twin above.
    unsafe {
        core::arch::asm!("csrr {}, cycle", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

user_mode_runtime::panic_handler!();
