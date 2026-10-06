//! **The reboot object: the authority to restart the machine, as a capability** (milestone 805
//! (`reboot` at the prompt), DECISIONS §251 (restarting the machine is a kernel object the
//! progenitor hands out)).
//!
//! PSCI and SBI calls are made from EL1 and S-mode, so on two of three architectures a user program
//! cannot reset the machine at all, and the kernel has to do it on somebody's behalf. Whose behalf
//! is the whole question, and §251's answer is: whoever holds this object. It has no payload, names
//! no device and has one method, [`abi::reboot::REBOOT`]. The kernel mints exactly one at boot and
//! grants it to the progenitor, which endows it only to a program whose manifest declares
//! `grant_plan::Manifest::reboot`.
//!
//! # What the method does, and what it deliberately does not
//!
//! - It calls [`prepare_the_reset_route`] (a no-op everywhere but a JH7110) and then
//!   `arch::reboot`. On success neither returns.
//! - If every route this architecture has was refused, `arch::reboot` prints the firmware's raw
//!   answer and returns its portable reason (`abi::reboot::Refusal`), and the method answers that
//!   reason's error: calef's ruling on §251's amendment, item 3 (2026-10-06 UTC).
//! - **It syncs nothing.** The kernel knows no filesystem, and a microkernel that did would be the
//!   bug. The `reboot` program sends `filesystem_protocol::fs::SYNC` first. A holder that skips it
//!   loses whatever the device had not flushed: a foot gun, recorded in §251 and in the program's
//!   `BUGS`, and the mitigation is that exactly one program is endowed.
//! - **It does not quiesce devices.** A DMA transfer in flight is cut off with everything else.
//!
//! # BUGS
//!
//! - **A firmware that accepts the call and hangs is indistinguishable from a slow reset**, from
//!   inside the machine. radon did exactly this on 2026-09-04; milestone 592 (radon's cold reboot
//!   dies in OpenSBI's PMIC write) holds the fix, which has not run on the board.
//! - **The caller gets the reason, not the number.** Four portable reasons cover every firmware;
//!   the raw code (PSCI's or SBI's own) is on the kernel's line just above.
//!
//! Name: provisional (milestone 805). The module, the object, its method and [`MARKER`] are
//! calef's to name; §251 calls them "the reboot object" and `REBOOT` for want of anything better.

use crate::{arch, println};

/// **The prefix on every line the reboot method prints** (milestone 805), so the gate that types
/// `reboot` and a person reading a bench capture can find what the kernel said about the reset in
/// one grep. Distinct from the soak's `soak-test-reboot:` on purpose: a capture that has both is a
/// capture of two different callers.
///
/// Name: provisional, milestone 805's lane, 2026-10-06 (UTC).
pub const MARKER: &str = "reboot:";

/// **`REBOOT`: restart the machine, or say why not** (DECISIONS §251, "The method").
///
/// Returns only when the firmware refused, with the refusal already printed; a successful reset
/// does not come back. The caller is `syscall::invoke`'s arm for `Object::Reboot`, and the
/// authority check is the capability itself: §251 asks for no rights bit beyond holding it.
pub fn restart() -> abi::Error {
    // Out of the ring first: once the reset starts, the drainer never runs again.
    crate::console::enter_reset();
    println!("{MARKER} the kernel was asked to restart the machine");
    prepare_the_reset_route(MARKER);
    let refusal = arch::reboot(MARKER);
    println!(
        "{MARKER} every reset route was refused ({refusal:?}; the lines above say how); the \
         machine keeps running"
    );
    refusal.error()
}

/// **Put back what the firmware's reset needs and U-Boot took away** (milestone 592 (radon's cold reboot dies in OpenSBI's PMIC write),
/// provisional).
///
/// On a JH7110 board, OpenSBI performs SBI SRST as an I2C write to the AXP15060 PMIC on I2C5, and
/// radon's U-Boot removes its I2C driver at `Starting kernel`, which gates the bus's clock and
/// asserts its reset. Radon's OpenSBI re-enables a clock, but it computes which one from the bus
/// node's name and U-Boot's tree names it `i2c@12050000`, so it ungates UART4's core clock
/// instead; and it never releases a reset. So the kernel ungates and releases I2C5 itself, from
/// the plan `memory::init` read out of the device tree, and prints every word it saw.
///
/// A no-op with no output on every machine that is not a JH7110 (`memory::jh7110_pmic_bus` is
/// `None` there), which is every machine CI boots. Two callers since milestone 805: the rebooting
/// soak and [`restart`], the two SBI resets nife makes on purpose. Moved here from `soak.rs` so the
/// second one could reach it without the soak's feature. The board test exit's shutdown takes the
/// same road and is recorded as a `BUGS` entry in milestone 592 rather than changed here.
pub fn prepare_the_reset_route(marker: &str) {
    // Only a JH7110 has anything to prepare, and only riscv64 compiles the branch that reads it.
    #[cfg(not(target_arch = "riscv64"))]
    let _ = marker;
    #[cfg(target_arch = "riscv64")]
    if let Some((sys, bus)) = crate::memory::jh7110_pmic_bus() {
        use jh7110_clock_and_reset::Step;
        println!(
            "{marker} JH7110: bringing the PMIC's I2C bus back up first, because OpenSBI \
             resets this board with an I2C write to the AXP15060 (milestone 592). SYS CRG at \
             {:#x} ({}); plan {} ({} specifier(s) skipped{}).",
            sys.base,
            if sys.from_tree {
                "named by this machine's device tree"
            } else {
                "NOT named by this machine's tree: the constant mainline and the vendor agree on"
            },
            if bus.from_tree {
                "from the tree's own clocks and resets of the PMIC's bus"
            } else {
                "is the constant I2C5 plan (clock 143, reset 81), NOT read from this tree"
            },
            bus.skipped,
            if bus.truncated { ", TRUNCATED" } else { "" },
        );
        // SAFETY: `memory::init` recorded this window only for a machine whose tree names a
        // JH7110, and `mmu::map_everything` mapped exactly it, device-typed, in the direct map.
        // The plan's identifiers were bounded by `SYS` when it was built, and `bring_up` bounds
        // them again.
        let report = unsafe {
            crate::drivers::jh7110_clock_and_reset::bring_up(
                crate::arch::mmu::phys_to_virt(sys.base) as usize,
                &jh7110_clock_and_reset::SYS,
                bus.plan(),
            )
        };
        let clocks = bus.plan().iter().filter_map(|s| match s {
            Step::EnableClock(i) => Some(*i),
            Step::DeassertReset(_) | Step::SelectParent { .. } => None,
        });
        for (n, index) in clocks.enumerate().take(report.clocks) {
            println!(
                "{marker} JH7110: clock {index} {:#010x} -> {:#010x} ({})",
                report.clock_before[n],
                report.clock_after[n],
                if jh7110_clock_and_reset::is_clock_enabled(report.clock_after[n]) {
                    "running"
                } else {
                    "NOT running: the enable bit did not read back"
                },
            );
        }
        let reset = bus.plan().iter().rev().find_map(|s| match s {
            Step::DeassertReset(id) => Some(*id),
            Step::EnableClock(_) | Step::SelectParent { .. } => None,
        });
        if let Some(id) = reset {
            println!(
                "{marker} JH7110: reset {id} assert {:#010x} -> {:#010x}, status {:#010x} \
                 ({}, {} polls){}",
                report.reset_assert_before,
                report.reset_assert_after,
                report.reset_status_after,
                if report.released {
                    "released"
                } else {
                    "STILL HELD"
                },
                report.polls,
                if report.was_already_up() {
                    "; the bus was already up, so U-Boot's handover is NOT why the reset hangs"
                } else {
                    ""
                },
            );
        }
        if report.rejected > 0 {
            println!(
                "{marker} JH7110: {} step(s) REJECTED by the SYS domain's bounds: the plan \
                 and the domain disagree, which is a bug in this kernel, not the board",
                report.rejected
            );
        }
    }
}
