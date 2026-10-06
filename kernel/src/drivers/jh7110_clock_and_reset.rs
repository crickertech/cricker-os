//! **The JH7110's clock and reset generator, the volatile half** (milestone 220).
//!
//! `crates/jh7110_clock_and_reset` owns the offsets, the bit positions, the bring-up plan and the device-tree
//! query, all of it host-tested and pointer-free. This file is the twenty lines that actually
//! store to a register, and it does nothing else: it walks a plan it is handed, against a base it
//! is handed, and reports what the hardware said.
//!
//! Same rule as every driver here (DECISIONS §4): **it reaches into no kernel global.** No base
//! address of its own, no static state, no idea which device it is bringing up.
//!
//! # Why this is in the kernel, when drivers here are EL0 processes
//!
//! DECISIONS §86 (whether an NVMe driver can leave the kernel, and what capability would let it)
//! names *"the microkernel thesis that drivers are user programs"* as this project's direction of
//! travel, so a kernel-resident driver is an exception and owes an argument. It is the same
//! argument §86 settled for NVMe on 2026-09-03, and reusing it beats inventing a second one: **the kernel keeps the admin
//! plane, EL0 gets the data path, and no new syscall surface is added.**
//!
//! Three things make the clock controller a stronger case for that split than NVMe was, not a
//! weaker one:
//!
//! - **It is a shared resource, and granting it would *widen* a driver's authority rather than
//!   confine it.** The STG window is one 64 KiB block holding the clocks and resets for USB,
//!   both PCIe root ports, the DMA engine and the security block. A userspace TRNG driver holding
//!   it could gate PCIe's clock or assert USB's reset. Milestone 159's whole demonstration is that
//!   the driver holds *one page of one device's registers and two endpoints*; handing it the CRG
//!   would trade the narrowest authority in this tree for the widest.
//! - **It is one-shot setup, entirely off any hot path.** Fatal risk 6's unmeasured half is "at
//!   real speed", and nothing about three register writes at wiring time is on that path. The
//!   performance argument that makes EL0 attractive for NVMe's data path does not exist here.
//! - **It costs zero new syscall surface**, which is the property §86 called the reversible part
//!   of its own decision. `start_jh7110` already maps the device page and spawns; this adds three
//!   stores before it. Nothing two programs agree on changes, so if a later milestone wants a
//!   clock service at EL0 (a power-management story would want one), that is a decision taken on
//!   its own evidence and not one this forecloses.
//!
//! # SAFETY, and the one rule that keeps it true
//!
//! [`bring_up`] stores to whatever address it is given. `STG_BASE`'s fallback in the pure crate
//! means a `Found` can name `0x1023_0000` on a machine that has nothing there, and a store to
//! unmapped MMIO on RISC-V is a fault or a hang. **So the caller must have established that this
//! machine is a JH7110 before calling**, and `memory::jh7110_clock_and_reset` is where that is
//! established: it records a region only when the tree names either a CRG or a JH7110 TRNG, so
//! QEMU's `virt` board (which names neither) never produces one and this driver is never reached
//! there.

use jh7110_clock_and_reset::{
    CLOCK_ENABLE, Domain, MAX_RECORDED_CLOCKS, Report, Step, is_deasserted, with_parent,
};

/// How many times [`bring_up`] reads the status word before giving up on a deassert.
///
/// **An iteration count, not a duration**, which is a real limitation and is recorded as one in
/// `notes/jh7110-clock-and-reset.md`'s `BUGS`. Linux polls the same word with a 1000 µs timeout
/// (\[mainline-rst\]'s `readl_poll_timeout_atomic`); this runs before the scheduler is doing
/// anything else and has no calibrated delay to hand at that point, so it counts reads instead.
/// A million reads of an MMIO word is far longer than the microsecond the hardware needs and far
/// shorter than a boot anyone would call hung.
const POLL_LIMIT: u32 = 1_000_000;

/// **Run `plan` against the clock-and-reset window at `base`.**
///
/// `base` is a *kernel virtual* address: the direct-map address of the controller's register
/// window, mapped device-typed. `domain` says where the resets sit in it and bounds every
/// identifier the plan names, so a plan written against the wrong domain produces `rejected`
/// steps rather than stores to whatever the arithmetic landed on.
///
/// Idempotent by construction: enabling an enabled clock and deasserting a released reset are
/// both no-ops in the hardware, which matters because the firmware may already have done some of
/// this and nothing here can tell in advance.
///
/// # Safety
///
/// `base` must be a mapped, device-typed window of at least `domain`'s extent, belonging to a
/// JH7110 clock-and-reset generator. See this module's header: the caller establishes that the
/// machine is a JH7110, because the pure crate's discovery will hand back a plausible address on
/// a machine that has no such device.
pub unsafe fn bring_up(base: usize, domain: &Domain, plan: &[Step]) -> Report {
    let mut report = Report::default();

    for step in plan {
        match *step {
            Step::EnableClock(index) => {
                let Some(offset) = domain.clock_offset(index) else {
                    report.rejected += 1;
                    continue;
                };
                let reg = (base + offset as usize) as *mut u32;
                // SAFETY: `base` is a mapped device window per this function's contract, and
                // `clock_offset` returned `Some`, which bounds the offset inside the domain.
                let before = unsafe { core::ptr::read_volatile(reg) };
                // SAFETY: as above. A read-modify-write of a word this controller dedicates to
                // one clock, so there is no neighbouring field to preserve beyond what the
                // read already carries.
                unsafe { core::ptr::write_volatile(reg, before | CLOCK_ENABLE) };
                // SAFETY: as above. Read back rather than assumed: a window with nothing behind
                // it accepts the store and reads zero, and that is the case this exists to catch.
                let after = unsafe { core::ptr::read_volatile(reg) };
                if report.clocks < MAX_RECORDED_CLOCKS {
                    report.clock_before[report.clocks] = before;
                    report.clock_after[report.clocks] = after;
                    report.clocks += 1;
                } else {
                    report.truncated = true;
                }
            }
            Step::SelectParent { clock, parent } => {
                let Some(offset) = domain.clock_offset(clock) else {
                    report.rejected += 1;
                    continue;
                };
                let reg = (base + offset as usize) as *mut u32;
                // SAFETY: as `EnableClock`: a mapped device window per this function's contract,
                // and `clock_offset` bounded the offset inside the domain.
                let before = unsafe { core::ptr::read_volatile(reg) };
                // SAFETY: as above. A read-modify-write of this clock's own word, replacing only
                // the parent field (`with_parent`), so the enable bit and divider are kept.
                unsafe { core::ptr::write_volatile(reg, with_parent(before, parent)) };
                // SAFETY: as above; read back, for `EnableClock`'s reason.
                report.mux_after = unsafe { core::ptr::read_volatile(reg) };
                report.mux_before = before;
                report.had_mux = true;
            }
            Step::DeassertReset(id) => {
                let Some(bit) = domain.reset_bit(id) else {
                    report.rejected += 1;
                    continue;
                };
                let assert = (base + bit.assert_offset as usize) as *mut u32;
                let status = (base + bit.status_offset as usize) as *const u32;
                // SAFETY: `base` is a mapped device window per this function's contract, and
                // `reset_bit` returned `Some`, which bounds both offsets inside the domain.
                let before = unsafe { core::ptr::read_volatile(assert) };
                // SAFETY: as above. A read-modify-write, and it must be one: this word holds up
                // to 32 resets and clobbering it would release or hold every other line in the
                // domain. Nothing else in this kernel writes it (the only caller is the
                // boot/test thread that wires the service, the same single-writer argument
                // `entropy_service::WIRED` makes for its plain atomics), so the read and the
                // write cannot be separated by another writer.
                unsafe { core::ptr::write_volatile(assert, before & !bit.mask) };
                let mut polls = 0;
                let mut seen;
                loop {
                    // SAFETY: as above; a read of the status word inside the domain.
                    seen = unsafe { core::ptr::read_volatile(status) };
                    polls += 1;
                    if is_deasserted(seen, bit.mask) || polls >= POLL_LIMIT {
                        break;
                    }
                }
                // SAFETY: as above.
                report.reset_assert_after = unsafe { core::ptr::read_volatile(assert) };
                report.reset_assert_before = before;
                report.reset_status_after = seen;
                report.released = is_deasserted(seen, bit.mask);
                report.polls = polls;
                report.had_reset = true;
                report.resets += 1;
                report.resets_released += u32::from(report.released);
            }
        }
    }

    report
}
