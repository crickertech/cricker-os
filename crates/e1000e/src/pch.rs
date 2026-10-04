//! **The I219's own bring-up, ported from FreeBSD's `e1000` shared code** (milestone 494 (a driver
//! for the network card a PC actually has), after calef's amendment of 2026-10-04 to §46 (thin
//! primitives or whole subsystems): outside the kernel and the Kani-proved crates, take or adapt
//! first).
//!
//! The I219 is not a PCIe NIC in the 82574L's sense. It is a MAC inside the platform controller hub
//! (PCH) talking to a separate PHY, shared with the Management Engine, and it carries workarounds
//! that only field exposure finds. This module holds the subset of FreeBSD's that touches **MAC
//! registers and PCI configuration space only**, as constants and pure decisions; the kernel's
//! `e1000e` module performs them. Each item names the FreeBSD function it comes from, read on
//! 2026-10-04 from `sys/dev/e1000/` on FreeBSD's `main`:
//!
//! - [`ulp`]: leaving ultra-low-power mode through the Management Engine, from
//!   `e1000_disable_ulp_lpt_lp` (`e1000_ich8lan.c`), its ME branch only.
//! - [`flush`]: the descriptor-ring flush SPT parts need before a reset, from `em_flush_desc_rings`,
//!   `em_flush_tx_ring` and `em_flush_rx_ring` (`if_em.c`).
//! - [`reset`]: the MAC-register steps around the global reset, from `e1000_reset_hw_ich8lan`
//!   (`e1000_ich8lan.c`) and `e1000_disable_pcie_master_generic` (`e1000_mac.c`).
//!
//! **What was not ported, and why:** everything that talks to the PHY. ULP exit without an ME,
//! the `LANPHYPC` toggle's follow-up, the `SMBus`-mode unforcing, `e1000_post_phy_reset_ich8lan` and
//! the PHY workarounds all go through MDIO (`MDIC`) under the software/firmware semaphore
//! (`EXTCNF_CTRL`), which is a PHY access layer this crate does not have. That is a few hundred
//! lines of FreeBSD and is proposed as its own piece in milestone 494's `Reuse` section. Because
//! of it, the reset here deliberately does **not** set `CTRL.PHY_RST`: FreeBSD resets the PHY with
//! the MAC only because it then runs the post-reset PHY workarounds, and resetting it without them
//! would be worse than leaving the PHY as firmware configured it.
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
//! - **None of it has run.** QEMU's 82574L is not a PCH part, so every step here is skipped under
//!   the gates ([`is_pch`] is false for it). xenon's bench boot is its first execution.
//! - **The transmit flush diverges from FreeBSD in one place**, said at [`flush::tail_after`].
//! - **The licence's second condition is not met by any binary this tree ships yet.** A binary
//!   redistribution must reproduce Intel's notice "in the documentation and/or other materials".
//!   The source carries it here; nothing that ships an image (the stick, a package) carries a
//!   third-party notices file, and none exists in the tree. Milestone 494's block records it.

/// Is `device` a PCH part (an I219) rather than the 82574L? Every I219 id this crate claims is in
/// FreeBSD's `e1000_pch_spt` class (`e1000_api.c`, `e1000_set_mac_type`), which is the class all
/// three ported sequences are written for.
pub fn is_pch(device: u16) -> bool {
    super::is_supported(super::VENDOR_INTEL, device) && device != 0x10d3
}

/// MAC registers the I219 adds. Offsets from FreeBSD's `e1000_regs.h` and `e1000_ich8lan.h`.
pub mod regs {
    /// Strap register, read only; FreeBSD writes it to order a config-space read before reset.
    pub const STRAP: u64 = 0x0_000c;
    /// Receive descriptor control, queue 0.
    pub const RXDCTL: u64 = 0x0_2828;
    /// Analog front end band gap transmit reference data.
    pub const KABGTXD: u64 = 0x0_3004;
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
    /// Does this machine have an ME to ask? When it does not, ULP exit is a PHY-register sequence
    /// this crate has not ported (the module header).
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
