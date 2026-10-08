//! **A thread reading the CPU's cycle counter from user mode**, milestone 229.
//!
//! Reads the counter twice through `user_mode_runtime::cycle_reading` and reports
//! [`capability_witness_protocol::CYCLE_COUNTER_WORD`], the difference of the two reads, and what
//! they count (an `abi::cycle_counter::CycleMeaning`, as its word). Until milestone 353 (the aarch64 half of 74) this program
//! held its own raw read, so that milestone 74 (cycle counters) would design the shared function rather than inherit
//! one from a test vehicle; calef's ruling B4 (2026-10-07 UTC) designed it, and this is now its
//! first caller. Holds a report endpoint (slot 0) and nothing else: the grant is **not** a capability in
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
//! **The difference is checked on the kernel side, and only partly.** Since milestone 74's aarch64
//! half the kernel starts `PMCCNTR_EL0` on every core, so the kernel's test asserts the difference
//! is positive wherever `arch::pmu` reports the counter running (and always on `x86_64`). It does not
//! assert on riscv64, where the kernel's counter and the `cycle` CSR this program reads may not be
//! the same counter. Nothing asserts on the size of the difference: under QEMU it is emulator time.
//! The meaning is checked on all three.
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

use user_mode_runtime::{cycle_reading, exit, send, yield_now};

const REPORT: u64 = 0;

#[unsafe(no_mangle)]
pub extern "C" fn _start(_arg0: u64, _arg1: u64, _arg2: u64) -> ! {
    let first = cycle_reading();
    yield_now();
    let second = cycle_reading();
    send(
        REPORT,
        capability_witness_protocol::CYCLE_COUNTER_WORD,
        second.count.wrapping_sub(first.count),
        second.meaning.to_word(),
    );
    exit()
}

user_mode_runtime::panic_handler!();
