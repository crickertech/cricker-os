//! **The MDIO bus, through the MAC's own two registers**, and the IEEE 802.3 clause 22 registers
//! every PHY has.
//!
//! The command word and the busy-bit poll are OpenBSD's `dwqe_mii_readreg` and
//! `dwqe_mii_writereg` (`sys/dev/ic/dwqe.c`, ISC license, notice in the crate root): the data
//! first on a write, then the address with the busy bit set, then wait for the controller to clear
//! it. The clause 22 register numbers and bits are the standard's.

use crate::{Hw, regs};

/// The MDC clock range field's shift in [`regs::MAC_MDIO_ADDR`].
pub const CR_SHIFT: u32 = 8;
/// **The clock range this driver always programs**: `0b0111`, "CSR clock 500 to 800 MHz",
/// which divides the CSR clock by 324.
///
/// OpenBSD reads the `stmmaceth` clock's rate and picks the matching range. This kernel has no
/// clock framework to ask, and the choice only has to keep MDC at or under the standard's 2.5 MHz:
/// the largest divider does that for every CSR clock up to 810 MHz and merely runs MDC slower
/// below, which MDIO tolerates. So the driver needs no rate at all, at the cost of slower PHY
/// register access during bring-up (tens of microseconds per access rather than a few), which is
/// off every path that repeats.
pub const CR_500_800: u32 = 7;
/// The PHY address field's shift.
pub const PA_SHIFT: u32 = 21;
/// The register address field's shift.
pub const RDA_SHIFT: u32 = 16;
/// Command: read.
pub const GOC_READ: u32 = 3 << 2;
/// Command: write.
pub const GOC_WRITE: u32 = 1 << 2;
/// Busy: set by the driver to start a command, cleared by the controller when it is done.
pub const GB: u32 = 1 << 0;

/// How long one MDIO command may take before it is a timeout: OpenBSD's 2000 polls 10 µs apart.
pub const COMMAND_LIMIT_US: u64 = 20_000;
/// The poll interval.
const POLL_US: u64 = 10;

/// Clause 22: basic mode control.
pub const BMCR: u8 = 0x00;
/// Restart autonegotiation; self-clearing.
pub const BMCR_ANRESTART: u16 = 1 << 9;
/// Autonegotiation enable.
pub const BMCR_ANENABLE: u16 = 1 << 12;
/// Software reset of the PHY; self-clearing.
pub const BMCR_RESET: u16 = 1 << 15;
/// Clause 22: basic mode status.
pub const BMSR: u8 = 0x01;
/// Link status. **Latched low**: a link that dropped since the last read reads zero once, so the
/// current state is the second of two reads.
pub const BMSR_LINK: u16 = 1 << 2;
/// Autonegotiation complete.
pub const BMSR_ANEG_COMPLETE: u16 = 1 << 5;
/// Clause 22: PHY identifier, high half.
pub const PHYSID1: u8 = 0x02;
/// Clause 22: PHY identifier, low half (the low four bits are the revision).
pub const PHYSID2: u8 = 0x03;

/// Why an MDIO command did not complete.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The busy bit stayed set for [`COMMAND_LIMIT_US`]: the MAC's CSR clock is off, or the
    /// command never started.
    Timeout {
        /// The PHY addressed.
        phy: u8,
        /// The register addressed.
        reg: u8,
    },
}

/// **An MDIO command word**: the clock range, the PHY and register addresses, the operation, and
/// the busy bit that starts it. Out-of-range addresses are reduced to the field (5 bits each)
/// rather than spilling into the neighbors.
pub const fn command(phy: u8, reg: u8, write: bool) -> u32 {
    (CR_500_800 << CR_SHIFT)
        | (((phy & 0x1f) as u32) << PA_SHIFT)
        | (((reg & 0x1f) as u32) << RDA_SHIFT)
        | if write { GOC_WRITE } else { GOC_READ }
        | GB
}

/// A clause 22 PHY register file, reached somehow. The MAC's MDIO pair is one implementation
/// ([`Bus`]); a test's simulated PHY is another.
pub trait Mdio {
    /// Read register `reg` of the PHY at `phy`.
    fn read(&mut self, phy: u8, reg: u8) -> Result<u16, Error>;
    /// Write `v` to register `reg` of the PHY at `phy`.
    fn write(&mut self, phy: u8, reg: u8, v: u16) -> Result<(), Error>;
}

/// The MAC's MDIO registers, through whatever [`Hw`] reaches the controller.
pub struct Bus<'a, H: Hw>(pub &'a mut H);

impl<H: Hw> Bus<'_, H> {
    fn wait(&mut self, phy: u8, reg: u8) -> Result<(), Error> {
        let mut waited = 0;
        loop {
            if self.0.read(regs::MAC_MDIO_ADDR) & GB == 0 {
                return Ok(());
            }
            if waited >= COMMAND_LIMIT_US {
                return Err(Error::Timeout { phy, reg });
            }
            self.0.delay_us(POLL_US);
            waited += POLL_US;
        }
    }
}

impl<H: Hw> Mdio for Bus<'_, H> {
    fn read(&mut self, phy: u8, reg: u8) -> Result<u16, Error> {
        // A command still running from someone else (firmware) would be overwritten mid-flight.
        self.wait(phy, reg)?;
        self.0.write(regs::MAC_MDIO_ADDR, command(phy, reg, false));
        self.wait(phy, reg)?;
        Ok(self.0.read(regs::MAC_MDIO_DATA) as u16)
    }

    fn write(&mut self, phy: u8, reg: u8, v: u16) -> Result<(), Error> {
        self.wait(phy, reg)?;
        self.0.write(regs::MAC_MDIO_DATA, u32::from(v));
        self.0.write(regs::MAC_MDIO_ADDR, command(phy, reg, true));
        self.wait(phy, reg)
    }
}

/// **The PHY's identifier**, `PHYSID1 << 16 | PHYSID2`, revision included.
pub fn phy_id(bus: &mut impl Mdio, phy: u8) -> Result<u32, Error> {
    let hi = bus.read(phy, PHYSID1)?;
    let lo = bus.read(phy, PHYSID2)?;
    Ok(u32::from(hi) << 16 | u32::from(lo))
}

/// **Is the link up right now?** Two reads of the latched-low status bit, so a drop since the last
/// look does not read as down forever and a link that is up now reads as up.
pub fn link_is_up(bus: &mut impl Mdio, phy: u8) -> Result<bool, Error> {
    let _ = bus.read(phy, BMSR)?;
    Ok(bus.read(phy, BMSR)? & BMSR_LINK != 0)
}

/// **Restart autonegotiation**, advertising whatever the PHY's own defaults advertise (every speed
/// on a gigabit part).
pub fn restart_autonegotiation(bus: &mut impl Mdio, phy: u8) -> Result<(), Error> {
    let bmcr = bus.read(phy, BMCR)?;
    bus.write(phy, BMCR, bmcr | BMCR_ANENABLE | BMCR_ANRESTART)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_read_command_carries_the_largest_divider_the_addresses_and_the_busy_bit() {
        let c = command(0, PHYSID1, false);
        assert_eq!(c, 7 << 8 | 2 << 16 | 3 << 2 | 1);
        let w = command(31, 31, true);
        assert_eq!(w, 7 << 8 | 31 << 21 | 31 << 16 | 1 << 2 | 1);
    }

    #[test]
    fn an_address_too_wide_for_its_field_does_not_spill_into_the_next() {
        // PHY 33 is PHY 1; it must not set bit 26, which is outside the 5-bit field.
        assert_eq!(command(33, 0, false), command(1, 0, false));
        assert_eq!(command(0, 0x3f, false) & (1 << PA_SHIFT), 0);
    }
}
