//! **radon's SD/MMC controller, the volatile half** (milestone 53 (the board's own peripherals:
//! network and storage on real silicon)).
//!
//! `crates/designware_mobile_storage` is the whole driver: the register map, the controller and
//! card sequences, block I/O, the bench step's two sequences, all host-tested against a simulated
//! controller. This file is the window those sequences run against, and nothing else: one
//! volatile load, one volatile store and a clock, over a base address it is handed. Rule 2 in its
//! smallest form. See notes/designware-mobile-storage.md.
//!
//! **The parity note this milestone must carry**: the JH7110 is a riscv64 part on one board, so
//! this module is riscv64-only and has no aarch64 or x86_64 counterpart. Rule 5's "a scope note
//! records the gap and the plan" applies: the crate is architecture-neutral, and a board with a
//! `DesignWare` Mobile Storage controller on another ISA (several Rockchip arm64 boards have one)
//! would need only this file's twenty lines and a device-tree match.

use designware_mobile_storage::host::Registers;

/// A controller's register window in the direct map, and the counter as a microsecond clock.
pub struct Window {
    base: usize,
    hz: u64,
}

impl Window {
    /// The window whose physical base the tree named.
    ///
    /// # Safety
    ///
    /// `phys` must be a `DesignWare` Mobile Storage controller's base that `mmu::init` mapped
    /// device-typed for at least `regs::WINDOW_USED` bytes (step 10, from the same
    /// `memory::jh7110_storage` answer), and nothing else may drive that controller while this
    /// window is in use.
    pub unsafe fn new(phys: u64) -> Window {
        Window {
            base: crate::arch::mmu::phys_to_virt(phys) as usize,
            hz: crate::arch::timer::frequency().max(1),
        }
    }
}

impl Registers for Window {
    fn read(&mut self, offset: u32) -> u32 {
        debug_assert!(
            offset < designware_mobile_storage::regs::WINDOW_USED && offset.is_multiple_of(4)
        );
        // SAFETY: `new`'s contract: the base is a mapped, device-typed controller window at least
        // `WINDOW_USED` bytes long, and every offset the crate passes is a register below that
        // (asserted above in debug builds), word-aligned.
        unsafe { core::ptr::read_volatile((self.base + offset as usize) as *const u32) }
    }

    fn write(&mut self, offset: u32, value: u32) {
        debug_assert!(
            offset < designware_mobile_storage::regs::WINDOW_USED && offset.is_multiple_of(4)
        );
        // SAFETY: as `read`.
        unsafe { core::ptr::write_volatile((self.base + offset as usize) as *mut u32, value) }
    }

    fn now_us(&mut self) -> u64 {
        // In 128 bits so a counter that has run for days cannot overflow the multiplication.
        (u128::from(crate::arch::timer::now()) * 1_000_000 / u128::from(self.hz)) as u64
    }
}
