//! The GICv3 CPU interface: system registers, `ICC_*_EL1`.
//!
//! Milestone 227. On a GICv2 the CPU interface is memory (`GICC_*`, `drivers/gic.rs`); on a GICv3 it
//! is a set of system registers, so the instructions that reach it are `msr` and `mrs` and they
//! live here by DECISIONS §4 rule 1. The memory-mapped half of the same controller (distributor and
//! redistributors) is `drivers/gicv3.rs`, and `irq.rs` beside this file is the one place that holds
//! both. See design/roadmap/0227-gicv3-driver.md for why the split falls here.
//!
//! # The registers, and what each costs to get wrong
//!
//! | register | what it does | the failure if skipped |
//! |---|---|---|
//! | `ICC_SRE_EL1.SRE` | selects the system-register interface over the legacy MMIO one | every other `ICC_*` access is UNDEFINED or silently goes nowhere |
//! | `ICC_PMR_EL1` | priority mask, `0xff` lets everything through | a mask of 0 (a common reset value) delivers nothing |
//! | `ICC_BPR1_EL1` | binary point; 0 restores the reset grouping | firmware can leave it wide enough to stop preemption between priorities |
//! | `ICC_CTLR_EL1.EOImode` | 0: one EOI write drops priority AND deactivates | mode 1 without `ICC_DIR_EL1` leaves every interrupt active forever |
//! | `ICC_AP1R0_EL1` | active-priority bits, for firmware that left some set | a stale bit masks everything at or below its priority |
//! | `ICC_IGRPEN1_EL1` | Group 1 on, at this core | the distributor signals, the core never listens: milestone 222's silence |
//!
//! # The barriers, read from Linux (v6.16) rather than recalled
//!
//! A system-register write is not a memory access, so neither program order nor a `dmb` orders it
//! against memory; an `isb` is what makes its effect visible to the instructions after it. The
//! sequence here is Linux's, and each site cites the function it comes from:
//!
//! - **`ICC_SRE_EL1` write, then `isb`, then read it back** (`gic_enable_sre`,
//!   `include/linux/irqchip/arm-gic-v3.h`). If the bit did not stick, EL2 has the interface disabled
//!   and nothing after this can work; Linux prints "panic ahead", this kernel panics now.
//! - **`isb` before `ICC_IGRPEN1_EL1`, and after it** (`gic_cpu_sys_reg_init` and
//!   `gic_write_grpen1`, `arch/arm64/include/asm/arch_gicv3.h`): the mask and the control word must
//!   be in effect before the group is switched on.
//! - **`dsb sy` after reading `ICC_IAR1_EL1`** (`gic_read_iar_common`): the acknowledge must
//!   complete before the handler's own device accesses, which a system-register read does not
//!   otherwise guarantee.
//! - **`isb` after `ICC_EOIR1_EL1`** (`gic_eoi_irq`).
//! - **`dsb ishst` before `ICC_SGI1R_EL1`, `isb` after** (`gic_ipi_send_mask`). The first is the one
//!   that matters for correctness here: the reschedule SGI tells another core to read an inbox this
//!   core just wrote, and without the `dsb` the SGI can overtake the stores. The GICv2 path has the
//!   same question with a Device store in place of the `msr`, and does not answer it; that is
//!   recorded in `drivers/gic.rs`'s BUGS rather than changed here.
//!
//! The register names below are the assembler's own (`icc_pmr_el1`, ...); LLVM knows them without
//! a target feature, so no `S3_0_C12_...` encodings appear.
//!
//! # BUGS
//!
//! - **Only `ICC_AP1R0_EL1` is cleared.** Linux clears `AP1R1`-`AP1R3` too when the core implements
//!   more than five priority bits. This kernel boots from firmware that took no interrupts, and the
//!   higher registers only matter to a core with more than 32 active priority levels, so they are
//!   left alone rather than written on the strength of a `PRIbits` read nobody has tested.
//! - **Aff0 above 15 needs `ICC_CTLR_EL1.RSS`**, the range selector. [`send_sgi`] refuses one; no
//!   core this kernel seats has one, because `smp.rs` seats cores by their Aff0 below `MAX_CPUS`.
//!
//! Name: provisional (`gic_cpu_interface`), named for the GIC architecture's own term for this half
//! of the controller.

use core::arch::asm;

use super::instructions;

/// This core's affinity, packed the way `GICR_TYPER[63:32]` states a redistributor's owner:
/// Aff3.Aff2.Aff1.Aff0, one byte each.
pub fn redistributor_affinity() -> u32 {
    let mpidr = instructions::read_mpidr();
    ((((mpidr >> 32) & 0xff) << 24) | (mpidr & 0xff_ffff)) as u32
}

/// **Bring this core's CPU interface up.** Run on every core, after its redistributor is awake.
///
/// # Panics
/// If `ICC_SRE_EL1.SRE` will not set, which means an EL2 above us has the system-register interface
/// disabled (`ICC_SRE_EL2.Enable` clear). `boot.s`'s EL2 drop sets it when this kernel was entered
/// at EL2; a hypervisor we did not write might not.
pub fn init_this_cpu() {
    let sre: u64;
    // SAFETY: the adapter calls this only on a machine whose tree names a GICv3 and whose
    // distributor confirmed it. ICC_SRE_EL1 then exists, or its access is UNDEFINED and traps into
    // the exception handler, which panics: loud either way. `ID_AA64PFR0_EL1.GIC` is deliberately
    // not the guard, because under HVF it reads zero while the registers work (see
    // `machine_discovery::gic`). Setting SRE only changes how the ICC registers below are reached;
    // `isb` (inside the write) so the read-back sees the write.
    unsafe {
        instructions::write_icc_sre_synchronized(instructions::read_icc_sre() | 1);
        sre = instructions::read_icc_sre();
    }
    assert!(
        sre & 1 != 0,
        "gic: ICC_SRE_EL1.SRE will not set; an EL2 above this kernel has the GICv3 system-register \
         interface disabled, and no interrupt can be delivered"
    );

    // SAFETY: the system-register interface is enabled (asserted above). Each write configures only
    // this core's CPU interface. The order and the barriers are Linux's `gic_cpu_sys_reg_init`:
    // mask, binary point, control, active priorities, `isb`, then the group enable and its `isb`.
    unsafe {
        asm!(
            "msr icc_pmr_el1, {pmr}",
            "msr icc_bpr1_el1, xzr",
            "msr icc_ctlr_el1, xzr",
            "msr icc_ap1r0_el1, xzr",
            "isb",
            "msr icc_igrpen1_el1, {one}",
            "isb",
            pmr = in(reg) 0xffu64,
            one = in(reg) 1u64,
            options(nostack, preserves_flags),
        );
    }
}

/// Take the interrupt: read `ICC_IAR1_EL1`, which acknowledges it, and return its INTID. The four
/// special INTIDs (1020-1023) are returned as read; the adapter treats them as spurious.
#[inline]
pub fn acknowledge() -> u32 {
    let iar: u64;
    // SAFETY: the interface is enabled (`init_this_cpu` ran on this core before interrupts were
    // unmasked). The read has a side effect on the GIC, which is the point, and `dsb sy` completes
    // it before the handler touches any device (Linux `gic_read_iar_common`).
    unsafe {
        asm!(
            "mrs {}, icc_iar1_el1",
            "dsb sy",
            out(reg) iar,
            options(nostack, preserves_flags),
        );
    }
    (iar & 0xff_ffff) as u32
}

/// "Finished with this one": drop the running priority and deactivate (`EOImode` 0).
#[inline]
pub fn end_of_interrupt(intid: u32) {
    // SAFETY: the interface is enabled; the write retires one interrupt this core acknowledged.
    // `isb` per Linux `gic_eoi_irq`.
    unsafe {
        asm!(
            "msr icc_eoir1_el1, {}",
            "isb",
            in(reg) u64::from(intid),
            options(nostack, preserves_flags),
        );
    }
}

/// Raise SGI `intid` (0-15) on the core whose `MPIDR_EL1` is `mpidr`.
///
/// `ICC_SGI1R_EL1` names a target as a cluster (Aff3.Aff2.Aff1) plus a 16-bit list of Aff0 values
/// in it, the encoding Linux's `gic_send_sgi` builds: Aff3 at bit 48, Aff2 at 32, INTID at 24, Aff1
/// at 16, the target list in bits 15:0.
pub fn send_sgi(intid: u32, mpidr: u64) {
    assert!(intid < 16, "SGIs are INTID 0-15");
    let aff0 = mpidr & 0xff;
    assert!(
        aff0 < 16,
        "gic: Aff0 {aff0} needs ICC_CTLR_EL1.RSS, which this kernel does not use"
    );
    let aff1 = (mpidr >> 8) & 0xff;
    let aff2 = (mpidr >> 16) & 0xff;
    let aff3 = (mpidr >> 32) & 0xff;
    let value = (aff3 << 48) | (aff2 << 32) | (u64::from(intid) << 24) | (aff1 << 16) | (1 << aff0);
    // SAFETY: the interface is enabled. `dsb ishst` makes this core's earlier stores (an inbox the
    // target is about to read) visible before the SGI can arrive; `isb` issues the write before
    // anything after it (Linux `gic_ipi_send_mask`). The `dsb` orders memory, so no `nomem`.
    unsafe {
        asm!(
            "dsb ishst",
            "msr icc_sgi1r_el1, {}",
            "isb",
            in(reg) value,
            options(nostack, preserves_flags),
        );
    }
}
