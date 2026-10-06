//! **The I219's own bring-up, ported from FreeBSD's `e1000` shared code** (milestone 494 (a driver
//! for the network card a PC actually has), after calef's amendment of 2026-10-04 to §46 (thin
//! primitives or whole subsystems): outside the kernel and the Kani-proved crates, take or adapt
//! first).
//!
//! The I219 is not a PCIe NIC in the 82574L's sense. It is a MAC inside the platform controller hub
//! (PCH) talking to a separate PHY, shared with the Management Engine, and it carries workarounds
//! that only field exposure finds. FreeBSD's Intel shared code (BSD-3-Clause, notice below) is the
//! only source this tree reads for them; Linux's `e1000e` is GPL and was not consulted for any of
//! it. Each item names the FreeBSD function it comes from, read from `sys/dev/e1000/` on FreeBSD's
//! `main`, on 2026-10-04 for the first three and 2026-10-05 for the rest:
//!
//! - [`ulp`]: leaving ultra-low-power mode through the Management Engine, from
//!   `e1000_disable_ulp_lpt_lp` (`e1000_ich8lan.c`), its ME branch.
//! - [`flush`]: the descriptor-ring flush SPT parts need before a reset, from `em_flush_desc_rings`,
//!   `em_flush_tx_ring` and `em_flush_rx_ring` (`if_em.c`).
//! - [`reset`]: the MAC-register constants around the global reset, from `e1000_reset_hw_ich8lan`
//!   (`e1000_ich8lan.c`) and `e1000_disable_pcie_master_generic` (`e1000_mac.c`).
//! - [`phy`]: MDIO through `MDIC`, the software/firmware semaphore, and the PCH PHY's paged
//!   registers.
//! - [`nvm`]: reading the `GbE` region of the flash through BAR0, as Sunrise Point does it.
//! - [`sequence`]: the bring-up itself, in FreeBSD's order: the PHY workarounds at attach
//!   (`e1000_init_phy_workarounds_pchlan`, which holds the ULP exit without an ME, the
//!   `SMBus`-to-PCIe switch and the `LANPHYPC` power cycle), the global reset with the PHY
//!   (`e1000_reset_hw_ich8lan`, `e1000_post_phy_reset_ich8lan`), and the hardware and copper-link
//!   setup (`e1000_init_hw_ich8lan`, `e1000_setup_copper_link_pch_lpt`).
//!
//! Every sequence is generic over [`Hw`], so it runs on the host against a simulated I219 (this
//! module's `sim`) as well as in the kernel. QEMU's 82574L is not a PCH part and never reaches
//! [`sequence`]; it does reach [`phy::id_at`], which shares [`phy::read_mdic`] with everything
//! here, so the MDIO primitive is the one piece the QEMU gates prove.
//!
//! The constants are facts about the hardware; the sequences are FreeBSD's, adapted, and carry
//! Intel's licence below as its first condition requires.
//!
//! ```text
//! SPDX-License-Identifier: BSD-3-Clause
//!
//! Copyright (c) 2001-2020, Intel Corporation
//! All rights reserved.
//!
//! Redistribution and use in source and binary forms, with or without
//! modification, are permitted provided that the following conditions are met:
//!
//!  1. Redistributions of source code must retain the above copyright notice,
//!     this list of conditions and the following disclaimer.
//!
//!  2. Redistributions in binary form must reproduce the above copyright
//!     notice, this list of conditions and the following disclaimer in the
//!     documentation and/or other materials provided with the distribution.
//!
//!  3. Neither the name of the Intel Corporation nor the names of its
//!     contributors may be used to endorse or promote products derived from
//!     this software without specific prior written permission.
//!
//! THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
//! AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
//! IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
//! ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT OWNER OR CONTRIBUTORS BE
//! LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
//! CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
//! SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
//! INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
//! CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
//! ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
//! POSSIBILITY OF SUCH DAMAGE.
//! ```
//!
//! # BUGS
//!
//! - **None of it has run on silicon.** QEMU's 82574L is not a PCH part, so [`sequence`] is
//!   skipped under the gates ([`is_pch`] is false for it) and is proved only against `sim`, which
//!   is this crate's reading of FreeBSD and not the device. xenon's bench boot is its first
//!   execution (notes/e1000e.md).
//! - **The link-up half is not ported.** FreeBSD reconfigures the PHY each time link comes up
//!   (`e1000_check_for_copper_link_ich8lan`: the EMI receive configuration, the PLL clock gate,
//!   the 776.20 pointer gap at 1000 Mb/s, the beacon duration, LTR and OBFF), from a link-change
//!   interrupt this driver does not take. A link that comes up and then loses frames, especially
//!   at 10 or 100 Mb/s, points here first. About 250 lines of FreeBSD; proposed in milestone 494's
//!   Follow-on.
//! - **Flow control is not configured, so it is not advertised.** FreeBSD advertises symmetric
//!   pause and programs the watermarks to match (`e1000_set_fc_watermarks_generic`); this driver
//!   programs neither, and advertises neither ([`sequence::setup_copper_link`]).
//! - **The transmit flush diverges from FreeBSD in one place**, said at [`flush::tail_after`].
//! - **One read in FreeBSD's `SMBus` unforce is not mirrored.** In `e1000_phy_is_accessible_pchlan`,
//!   a failed read of `CV_SMB_CTRL` leaves the previous register's value in the variable that is
//!   then written back with one bit cleared. This port writes only when the read succeeded.
//! - **The licence's second condition is not met by any binary this tree ships yet.** A binary
//!   redistribution must reproduce Intel's notice "in the documentation and/or other materials".
//!   The source carries it here and in each adapted file; nothing that ships an image (the stick, a
//!   package) carries a third-party notices file, and none exists in the tree. Milestone 494's
//!   block records it.

pub mod nvm;
pub mod phy;
pub mod sequence;
#[cfg(test)]
mod sim;

/// **What the I219 sequences need from whoever holds BAR0.** The kernel implements it over the
/// direct map (`kernel/src/e1000e.rs`); the host tests implement it over a simulated device.
pub trait Hw {
    /// Read the 32-bit register at `off` in BAR0.
    fn read(&mut self, off: u64) -> u32;
    /// Write it.
    fn write(&mut self, off: u64, v: u32);
    /// Read the 16-bit register at `off`. Only the flash status word ([`nvm::HSFSTS`]) is read
    /// this way, as FreeBSD reads it.
    fn read16(&mut self, off: u64) -> u16;
    /// Wait at least `us` microseconds.
    fn delay_us(&mut self, us: u64);
    /// The function's PCI vendor id, read from configuration space. The read is the point: it is
    /// the delay `e1000_reset_hw_ich8lan` needs around the global reset.
    fn pci_vendor_id(&mut self) -> u16;
    /// Something the bench boot should show. Never needed for correctness.
    fn note(&mut self, n: sequence::Note);
}

fn delay_ms(hw: &mut impl Hw, ms: u64) {
    hw.delay_us(ms * 1000);
}

/// A posted-write flush (`E1000_WRITE_FLUSH`): any read of BAR0 will do; FreeBSD reads `STATUS`.
fn flush_writes(hw: &mut impl Hw) {
    let _ = hw.read(super::regs::STATUS);
}

/// Is `device` a PCH part (an I219) rather than the 82574L? Every I219 id this crate claims is in
/// FreeBSD's `e1000_pch_spt` class (`e1000_api.c`, `e1000_set_mac_type`), which is the class all
/// three ported sequences are written for.
pub fn is_pch(device: u16) -> bool {
    super::is_supported(super::VENDOR_INTEL, device) && device != 0x10d3
}

/// MAC registers the I219 adds. Offsets from FreeBSD's `e1000_regs.h` and `e1000_ich8lan.h`.
pub mod regs {
    /// Strap register, read only; FreeBSD writes it to order a config-space read before reset.
    /// Bits 5:1 also give the NVM's size ([`super::nvm::Geometry`]).
    pub const STRAP: u64 = 0x0_000c;
    /// MDI control: the MDIO transaction register ([`super::phy`]).
    pub const MDIC: u64 = 0x0_0020;
    /// Future extended NVM.
    pub const FEXTNVM: u64 = 0x0_0028;
    /// Future extended NVM 3.
    pub const FEXTNVM3: u64 = 0x0_003c;
    /// Future extended NVM 7.
    pub const FEXTNVM7: u64 = 0x0_00e4;
    /// LED control.
    pub const LEDCTL: u64 = 0x0_0e00;
    /// Extended configuration control: the semaphore, the PHY-configuration gate, and the pointer
    /// to the NVM's extended configuration region.
    pub const EXTCNF_CTRL: u64 = 0x0_0f00;
    /// Extended configuration size.
    pub const EXTCNF_SIZE: u64 = 0x0_0f08;
    /// PHY control as the MAC sees it (`E1000_PHY_CTRL`): LPLU and gigabit disable.
    pub const PHY_CTRL: u64 = 0x0_0f10;
    /// I/O side-band fabric power control; SPT's transmit errata write it.
    pub const IOSFPC: u64 = 0x0_0f28;
    /// Packet buffer allocation.
    pub const PBA: u64 = 0x0_1000;
    /// Packet buffer ECC status.
    pub const PBECCSTS: u64 = 0x0_100c;
    /// Receive descriptor control, queue 0.
    pub const RXDCTL: u64 = 0x0_2828;
    /// Analog front end band gap transmit reference data.
    pub const KABGTXD: u64 = 0x0_3004;
    /// Transmit descriptor control, queue 0.
    pub const TXDCTL0: u64 = 0x0_3828;
    /// Transmit arbitration control, queue 0.
    pub const TARC0: u64 = 0x0_3840;
    /// Transmit descriptor control, queue 1.
    pub const TXDCTL1: u64 = 0x0_3928;
    /// Transmit arbitration control, queue 1.
    pub const TARC1: u64 = 0x0_3940;
    /// Host to Management Engine.
    pub const H2ME: u64 = 0x0_5b50;
    /// Firmware semaphore (Management Engine status).
    pub const FWSM: u64 = 0x0_5b54;
    /// Future extended NVM 11.
    pub const FEXTNVM11: u64 = 0x0_5bbc;
}

/// Leaving ultra-low-power (ULP) mode through the Management Engine.
pub mod ulp {
    /// `FWSM.FW_VALID`: an ME is present and running, so it owns the ULP exit.
    pub const FWSM_FW_VALID: u32 = 0x0000_8000;
    /// `FWSM.ULP_CFG_DONE`: set while the ME's ULP configuration is still in force.
    pub const FWSM_ULP_CFG_DONE: u32 = 0x0000_0400;
    /// `H2ME.ULP`: the host's ULP indication.
    pub const H2ME_ULP: u32 = 0x0000_0800;
    /// `H2ME.ENFORCE_SETTINGS`: ask the ME to apply the host's request.
    pub const H2ME_ENFORCE_SETTINGS: u32 = 0x0000_1000;
    /// How long to wait for the ME, in milliseconds: FreeBSD's 250 polls of 10 ms.
    pub const WAIT_MS: u64 = 2500;

    /// The `H2ME` value that asks the ME to take the PHY out of ULP (the forced path).
    pub const fn request(h2me: u32) -> u32 {
        (h2me & !H2ME_ULP) | H2ME_ENFORCE_SETTINGS
    }
    /// The `H2ME` value once the ME has answered.
    pub const fn release(h2me: u32) -> u32 {
        h2me & !H2ME_ENFORCE_SETTINGS
    }
    /// Is the ME's ULP configuration finished?
    pub const fn done(fwsm: u32) -> bool {
        fwsm & FWSM_ULP_CFG_DONE == 0
    }
    /// Does this machine have an ME to ask? When it does not, ULP exit is the PHY-register
    /// sequence in [`super::sequence`].
    pub const fn has_me(fwsm: u32) -> bool {
        fwsm & FWSM_FW_VALID != 0
    }
}

/// The SPT descriptor-ring flush. FreeBSD's comment: "In I219, the descriptor rings must be
/// emptied before resetting the HW ... Failure to do this will cause the HW to enter a unit hang
/// state which can only be released by PCI reset on the device."
pub mod flush {
    /// PCI configuration offset of the descriptor ring status word.
    pub const DESC_RING_STATUS: u64 = 0xe4;
    /// The bit in it that says a flush is required.
    pub const REQUIRED: u16 = 0x100;
    /// `FEXTNVM11.DISABLE_MULR_FIX`, set before the flush.
    pub const FEXTNVM11_DISABLE_MULR_FIX: u32 = 0x0000_2000;
    /// The dummy transmit's length: FreeBSD sends 512 bytes of the ring itself.
    pub const TX_DUMMY_LEN: u16 = 512;

    /// Is a flush required, given the status word and `TDLEN`? FreeBSD flushes nothing when the
    /// transmit ring has no length.
    pub const fn required(status: u16, tdlen: u32) -> bool {
        status & REQUIRED != 0 && tdlen != 0
    }
    /// The dummy transmit descriptor's second word: insert FCS, 512 bytes, no end of packet and no
    /// status report, exactly FreeBSD's `txd_lower`.
    pub const fn tx_word() -> u64 {
        TX_DUMMY_LEN as u64 | (0x02u64 << 24)
    }
    /// **The one divergence.** FreeBSD writes the dummy at its own completion cursor and then
    /// writes that same index to `TDT`. This driver has no completion cursor at reset time, so it
    /// writes the dummy at the device's current tail and advances the tail by one, which hands the
    /// device exactly that descriptor: the effect the FreeBSD comment describes ("assign the ring
    /// itself as the data of the next descriptor").
    pub const fn tail_after(tdt: u32, entries: u16) -> u32 {
        (tdt + 1) % entries as u32
    }
    /// `RXDCTL` for the receive flush: prefetch threshold 31, host threshold 1, granularity in
    /// descriptors, the rest of the register kept.
    pub const fn rxdctl(rxdctl: u32) -> u32 {
        (rxdctl & 0xffff_c000) | 0x1f | 1 << 8 | 0x0100_0000
    }
}

/// The MAC-register steps around the global reset.
pub mod reset {
    /// `CTRL.GIO_MASTER_DISABLE`: stop new bus-master requests before the reset.
    pub const CTRL_GIO_MASTER_DISABLE: u32 = 0x0000_0004;
    /// `STATUS.GIO_MASTER_ENABLE`: set while master requests are still pending.
    pub const STATUS_GIO_MASTER_ENABLE: u32 = 0x0008_0000;
    /// How long to wait for pending master requests, in microseconds: 800 polls of 100.
    pub const MASTER_WAIT_US: u64 = 80_000;
    /// `KABGTXD.BGSQLBIAS`, set after the reset.
    pub const KABGTXD_BGSQLBIAS: u32 = 0x0005_0000;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pch_class_is_every_claimed_i219_and_not_qemus_part() {
        assert!(!is_pch(0x10d3));
        for &(id, model) in super::super::DEVICE_IDS {
            assert_eq!(is_pch(id), model.starts_with("I219"), "{id:#06x} {model}");
        }
    }

    #[test]
    fn the_ulp_request_clears_the_indication_and_enforces() {
        let h = ulp::request(ulp::H2ME_ULP | 0x1);
        assert_eq!(h & ulp::H2ME_ULP, 0);
        assert_ne!(h & ulp::H2ME_ENFORCE_SETTINGS, 0);
        assert_eq!(h & 1, 1, "other bits survive");
        assert_eq!(ulp::release(h) & ulp::H2ME_ENFORCE_SETTINGS, 0);
        assert!(!ulp::done(ulp::FWSM_ULP_CFG_DONE));
        assert!(ulp::has_me(ulp::FWSM_FW_VALID));
    }

    #[test]
    fn the_flush_runs_only_when_asked_and_the_ring_has_length() {
        assert!(flush::required(flush::REQUIRED, 256));
        assert!(!flush::required(flush::REQUIRED, 0));
        assert!(!flush::required(0, 256));
        assert_eq!(flush::tail_after(15, 16), 0);
        assert_eq!(flush::rxdctl(0xffff_ffff) & 0x3fff, 0x1f | 1 << 8);
        assert_ne!(flush::rxdctl(0) & 0x0100_0000, 0);
        // No end of packet, no report status: the dummy is never a frame anyone waits on.
        assert_eq!((flush::tx_word() >> 24) as u8 & (0x01 | 0x08), 0);
    }
}
