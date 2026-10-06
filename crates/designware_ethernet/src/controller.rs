//! **The controller's sequences**: identify, reset, configure, start, stop.
//!
//! OpenBSD's `dwqe_attach`, `dwqe_reset`, `dwqe_up`, `dwqe_down` and `dwqe_mii_statchg`
//! (`sys/dev/ic/dwqe.c`, ISC license, notice in the crate root), in their order, with three
//! changes each argued at the place it is made: the receive buffer size is written rather than
//! inherited, jumbo frames are not enabled, and nothing is allocated (the DMA region is the
//! caller's, laid out by [`crate::layout`]). Every sequence runs against [`Hw`], so the host tests
//! run them against `sim`'s controller.

use crate::motorcomm::{Link, Speed};
use crate::{BUFFER_SIZE, Hw, RX_ENTRIES, TX_ENTRIES, layout, regs};

/// Why the controller was not brought up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The software reset did not self-clear in [`RESET_LIMIT_US`]. On this controller the usual
    /// cause is a missing receive clock from the PHY, which the reset needs to complete.
    ResetTimeout,
    /// The DMA region is not somewhere this controller can address: past its address width
    /// ([`Identity::dma_bits`]), or straddling a 4 GiB boundary, which the 32-bit tail pointer
    /// registers cannot express.
    RegionOutOfReach {
        /// The controller's address width.
        dma_bits: u8,
    },
    /// The transmit queue did not finish flushing on [`stop`].
    FlushTimeout,
}

/// How long the software reset may take: OpenBSD's 30000 polls 10 µs apart.
pub const RESET_LIMIT_US: u64 = 300_000;

/// What the controller says it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Identity {
    /// [`regs::MAC_VERSION`].
    pub version: u32,
    /// The four hardware feature registers.
    pub features: [u32; 4],
}

impl Identity {
    /// Read the version and feature registers.
    pub fn read(hw: &mut impl Hw) -> Identity {
        let mut features = [0; 4];
        for (i, f) in features.iter_mut().enumerate() {
            *f = hw.read(regs::mac_hw_feature(i as u32));
        }
        Identity {
            version: hw.read(regs::MAC_VERSION),
            features,
        }
    }

    /// The Synopsys core version, `0x52` for the 5.20 both trees name.
    pub const fn core_version(&self) -> u8 {
        self.version as u8
    }

    /// The DMA's address width from feature register 1's `ADDR64` field: 32, 40 or 48 bits.
    pub const fn dma_bits(&self) -> u8 {
        match (self.features[1] >> 14) & 0x3 {
            1 => 40,
            2 => 48,
            _ => 32,
        }
    }

    /// The transmit FIFO's size in bytes, `128 << TXFIFOSIZE`.
    pub const fn tx_fifo_bytes(&self) -> u32 {
        fifo_bytes((self.features[1] >> 6) & 0x1f)
    }

    /// The receive FIFO's size in bytes, `128 << RXFIFOSIZE`.
    pub const fn rx_fifo_bytes(&self) -> u32 {
        fifo_bytes(self.features[1] & 0x1f)
    }
}

/// `128 << field`, with a field past the largest FIFO the databook encodes (256 KiB, field 11)
/// saturating there rather than shifting a bit off the top of the word.
const fn fifo_bytes(field: u32) -> u32 {
    let f = if field > 11 { 11 } else { field };
    128 << f
}

/// **Reset the whole controller** and wait for the reset to finish. Returns how long it took.
pub fn soft_reset(hw: &mut impl Hw) -> Result<u64, Error> {
    let mode = hw.read(regs::DMA_MODE);
    hw.write(regs::DMA_MODE, mode | regs::DMA_MODE_SWR);
    let mut waited = 0;
    loop {
        if hw.read(regs::DMA_MODE) & regs::DMA_MODE_SWR == 0 {
            return Ok(waited);
        }
        if waited >= RESET_LIMIT_US {
            return Err(Error::ResetTimeout);
        }
        hw.delay_us(10);
        waited += 10;
    }
}

/// **The station address in the two address registers**, OpenBSD's `dwqe_lladdr_read` byte order,
/// or `None` when they hold nothing a station could own: all zero or all ones (the reset value),
/// or a multicast address.
pub fn station_address(hi: u32, lo: u32) -> Option<[u8; 6]> {
    let [a, b, c, d] = lo.to_le_bytes();
    let [e, f, _, _] = hi.to_le_bytes();
    let mac = [a, b, c, d, e, f];
    if mac == [0; 6] || mac == [0xff; 6] || a & 1 != 0 {
        return None;
    }
    Some(mac)
}

/// The two address register words for `mac`: `(high, low)`, OpenBSD's `dwqe_lladdr_write`.
pub const fn address_words(mac: [u8; 6]) -> (u32, u32) {
    (
        u32::from_le_bytes([mac[4], mac[5], 0, 0]),
        u32::from_le_bytes([mac[0], mac[1], mac[2], mac[3]]),
    )
}

/// **How the DMA meets the bus**: the `snps,*` properties of a board's node and its
/// `snps,axi-config`. [`crate::jh7110::BUS`] is radon's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bus {
    /// `snps,fixed-burst`.
    pub fixed_burst: bool,
    /// Not `snps,no-pbl-x8`.
    pub pbl_x8: bool,
    /// `snps,txpbl`.
    pub tx_pbl: u32,
    /// `snps,rxpbl`.
    pub rx_pbl: u32,
    /// `snps,force_thresh_dma_mode`: threshold mode rather than store and forward.
    pub threshold_mode: bool,
    /// The [`regs::DMA_SYSBUS_MODE`] burst-length bits `snps,blen` names.
    pub burst_lengths: u32,
    /// `snps,rd_osr_lmt`.
    pub read_outstanding: u32,
    /// `snps,wr_osr_lmt`.
    pub write_outstanding: u32,
}

/// The receive operation mode word for a FIFO of `fifo` bytes, OpenBSD's arithmetic: the queue
/// size in 256-byte units less one, threshold or store-and-forward, and hardware flow control with
/// its thresholds once the FIFO is large enough for them to mean anything.
pub fn rx_queue_mode(current: u32, bus: &Bus, fifo: u32) -> u32 {
    let mut mode = current;
    if bus.threshold_mode {
        mode &= !(regs::MTL_RXQ_OP_MODE_RSF | regs::MTL_RXQ_OP_MODE_RTC_MASK);
        mode |= regs::MTL_RXQ_OP_MODE_RTC_128;
    } else {
        mode |= regs::MTL_RXQ_OP_MODE_RSF;
    }
    let rqs = (fifo / 256).saturating_sub(1);
    mode &= !regs::MTL_RXQ_OP_MODE_RQS_MASK;
    mode |= (rqs << regs::MTL_RXQ_OP_MODE_RQS_SHIFT) & regs::MTL_RXQ_OP_MODE_RQS_MASK;
    if fifo >= 4096 {
        // EHFC, then RFD = 3 and RFA = 1: OpenBSD's flow control thresholds.
        mode |= 1 << 7;
        mode &= !((0x3f << 14) | (0x3f << 8));
        mode |= (0x3 << 14) | (0x1 << 8);
    }
    mode
}

/// The transmit operation mode word for a FIFO of `fifo` bytes, OpenBSD's arithmetic.
pub fn tx_queue_mode(current: u32, bus: &Bus, fifo: u32) -> u32 {
    let mut mode = current;
    if bus.threshold_mode {
        mode &= !(regs::MTL_TXQ_OP_MODE_TSF | regs::MTL_TXQ_OP_MODE_TTC_MASK);
        mode |= regs::MTL_TXQ_OP_MODE_TTC_512;
    } else {
        mode |= regs::MTL_TXQ_OP_MODE_TSF;
    }
    mode &= !regs::MTL_TXQ_OP_MODE_TXQEN_MASK;
    mode |= regs::MTL_TXQ_OP_MODE_TXQEN;
    let tqs = (fifo / 256).saturating_sub(1);
    mode &= !regs::MTL_TXQ_OP_MODE_TQS_MASK;
    mode |= (tqs << regs::MTL_TXQ_OP_MODE_TQS_SHIFT) & regs::MTL_TXQ_OP_MODE_TQS_MASK;
    mode
}

/// Can a controller of `dma_bits` address a region of `bytes` at `phys`, with tail pointers
/// that only carry the low 32 bits?
pub const fn region_in_reach(phys: u64, bytes: u64, dma_bits: u8) -> bool {
    let Some(end) = phys.checked_add(bytes) else {
        return false;
    };
    let last = end - 1;
    last >> dma_bits == 0 && phys >> 32 == last >> 32
}

/// **Configure a controller just reset**, with nothing started: the DMA's bus mode, channel 0's
/// burst lengths, buffer size, both rings at `region` (heads and tails at the ring bases, so
/// nothing is available until the data plane posts), both queues' modes, flow control, the
/// receive queue enable, the station address and filter, and every interrupt masked.
///
/// The FIFO sizes are the controller's own ([`Identity`]), not the tree's: radon's vendor tree
/// says `rx-fifo-depth = <262144>` and mainline says `<2048>` for the same silicon, and a queue
/// size computed from the first would overflow the field into the next. The hardware's statement
/// is the one fact about the FIFO both trees are describing.
pub fn configure(
    hw: &mut impl Hw,
    id: &Identity,
    bus: &Bus,
    region: u64,
    mac: [u8; 6],
) -> Result<(), Error> {
    if !region_in_reach(region, layout::BYTES, id.dma_bits()) {
        return Err(Error::RegionOutOfReach {
            dma_bits: id.dma_bits(),
        });
    }

    let mut mode = hw.read(regs::DMA_SYSBUS_MODE);
    if bus.fixed_burst {
        mode |= regs::DMA_SYSBUS_MODE_FB;
    }
    if id.dma_bits() > 32 {
        mode |= regs::DMA_SYSBUS_MODE_EAME;
    }
    let osr = regs::DMA_SYSBUS_MODE_OSR_LMT_MASK;
    mode &= !(osr << regs::DMA_SYSBUS_MODE_RD_OSR_LMT_SHIFT
        | osr << regs::DMA_SYSBUS_MODE_WR_OSR_LMT_SHIFT);
    mode |= (bus.read_outstanding & osr) << regs::DMA_SYSBUS_MODE_RD_OSR_LMT_SHIFT
        | (bus.write_outstanding & osr) << regs::DMA_SYSBUS_MODE_WR_OSR_LMT_SHIFT
        | bus.burst_lengths;
    hw.write(regs::DMA_SYSBUS_MODE, mode);

    let control = hw.read(regs::CHAN_CONTROL);
    hw.write(
        regs::CHAN_CONTROL,
        if bus.pbl_x8 {
            control | regs::CHAN_CONTROL_PBLX8
        } else {
            control & !regs::CHAN_CONTROL_PBLX8
        },
    );
    let pbl = regs::CHAN_PBL_MASK;
    let tx = hw.read(regs::CHAN_TX_CONTROL) & !(pbl << regs::CHAN_TX_CONTROL_PBL_SHIFT);
    hw.write(
        regs::CHAN_TX_CONTROL,
        tx | (bus.tx_pbl & pbl) << regs::CHAN_TX_CONTROL_PBL_SHIFT | regs::CHAN_TX_CONTROL_OSP,
    );
    // The buffer size is written, not inherited (OpenBSD inherits it): a size left by firmware is
    // a fact about whoever ran last, and the device writes a frame up to it.
    let rbsz = regs::CHAN_RX_CONTROL_RBSZ_MASK << regs::CHAN_RX_CONTROL_RBSZ_SHIFT;
    let rx = hw.read(regs::CHAN_RX_CONTROL) & !(pbl << regs::CHAN_RX_CONTROL_PBL_SHIFT | rbsz);
    hw.write(
        regs::CHAN_RX_CONTROL,
        rx | (bus.rx_pbl & pbl) << regs::CHAN_RX_CONTROL_PBL_SHIFT
            | (BUFFER_SIZE as u32) << regs::CHAN_RX_CONTROL_RBSZ_SHIFT,
    );

    let tx_ring = region + layout::TX_RING;
    let rx_ring = region + layout::RX_RING;
    hw.write(regs::CHAN_TX_BASE_HI, (tx_ring >> 32) as u32);
    hw.write(regs::CHAN_TX_BASE, tx_ring as u32);
    hw.write(regs::CHAN_TX_RING_LEN, u32::from(TX_ENTRIES) - 1);
    hw.write(regs::CHAN_TX_TAIL, tx_ring as u32);
    hw.write(regs::CHAN_RX_BASE_HI, (rx_ring >> 32) as u32);
    hw.write(regs::CHAN_RX_BASE, rx_ring as u32);
    hw.write(regs::CHAN_RX_RING_LEN, u32::from(RX_ENTRIES) - 1);
    hw.write(regs::CHAN_RX_TAIL, rx_ring as u32);

    let rx_mode = rx_queue_mode(hw.read(regs::MTL_RXQ0_OP_MODE), bus, id.rx_fifo_bytes());
    hw.write(regs::MTL_RXQ0_OP_MODE, rx_mode);
    let tx_mode = tx_queue_mode(hw.read(regs::MTL_TXQ0_OP_MODE), bus, id.tx_fifo_bytes());
    hw.write(regs::MTL_TXQ0_OP_MODE, tx_mode);

    let flow = hw.read(regs::MAC_Q0_TX_FLOW_CTRL);
    hw.write(
        regs::MAC_Q0_TX_FLOW_CTRL,
        flow | 0xffff << regs::MAC_Q0_TX_FLOW_CTRL_PT_SHIFT | regs::MAC_Q0_TX_FLOW_CTRL_TFE,
    );
    let flow = hw.read(regs::MAC_RX_FLOW_CTRL);
    hw.write(regs::MAC_RX_FLOW_CTRL, flow | regs::MAC_RX_FLOW_CTRL_RFE);
    hw.write(regs::MAC_RXQ_CTRL0, regs::MAC_RXQ_CTRL0_Q0_DCB);

    let (hi, lo) = address_words(mac);
    hw.write(regs::MAC_ADDR0_HI, hi);
    hw.write(regs::MAC_ADDR0_LO, lo);
    // Own unicast address and broadcast; no multicast, no promiscuous receive.
    hw.write(regs::MAC_PACKET_FILTER, 0);

    hw.write(regs::MAC_INT_EN, 0);
    hw.write(regs::CHAN_INTR_ENA, 0);
    hw.write(regs::MMC_RX_INT_MASK, u32::MAX);
    hw.write(regs::MMC_TX_INT_MASK, u32::MAX);
    hw.write(regs::CHAN_STATUS, u32::MAX);
    Ok(())
}

/// **The MAC configuration word for a link**, OpenBSD's `dwqe_mii_statchg` and `dwqe_up`: the
/// port and speed select for the negotiated speed, duplex, carrier sense off, jabber off, burst
/// on, transmitter and receiver on. `loopback` turns the MAC's own loopback on for the coherence
/// probe.
///
/// OpenBSD also sets `JE` (jumbo frames). This does not: a jumbo frame cannot fit a 2048-byte
/// buffer, would span descriptors, and would be dropped by [`crate::rx_verdict`] anyway, so
/// accepting it into the FIFO only spends ring slots on frames that are thrown away.
pub const fn mac_conf(current: u32, link: Link, loopback: bool) -> u32 {
    let mut conf =
        current & !(regs::MAC_CONF_PS | regs::MAC_CONF_FES | regs::MAC_CONF_DM | regs::MAC_CONF_LM);
    conf |= match link.speed {
        Speed::Thousand => 0,
        Speed::Hundred => regs::MAC_CONF_PS | regs::MAC_CONF_FES,
        Speed::Ten => regs::MAC_CONF_PS,
    };
    if link.full_duplex {
        conf |= regs::MAC_CONF_DM;
    }
    if loopback {
        conf |= regs::MAC_CONF_LM;
    }
    conf | regs::MAC_CONF_BE
        | regs::MAC_CONF_JD
        | regs::MAC_CONF_DCRS
        | regs::MAC_CONF_TE
        | regs::MAC_CONF_RE
}

/// **Start both DMA channels and the MAC** at `link`, OpenBSD's order: receive DMA, transmit DMA,
/// then the MAC. Nothing moves until the data plane hands the device descriptors.
pub fn start(hw: &mut impl Hw, link: Link, loopback: bool) {
    let rx = hw.read(regs::CHAN_RX_CONTROL);
    hw.write(regs::CHAN_RX_CONTROL, rx | regs::CHAN_RX_CONTROL_SR);
    let tx = hw.read(regs::CHAN_TX_CONTROL);
    hw.write(regs::CHAN_TX_CONTROL, tx | regs::CHAN_TX_CONTROL_ST);
    let conf = hw.read(regs::MAC_CONF);
    hw.write(regs::MAC_CONF, mac_conf(conf, link, loopback));
}

/// **Stop the MAC and both DMA channels**, OpenBSD's `dwqe_down` order: receiver off, receive DMA
/// off, transmit DMA off, flush the transmit queue, transmitter off.
pub fn stop(hw: &mut impl Hw) -> Result<(), Error> {
    let conf = hw.read(regs::MAC_CONF);
    hw.write(regs::MAC_CONF, conf & !regs::MAC_CONF_RE);
    let rx = hw.read(regs::CHAN_RX_CONTROL);
    hw.write(regs::CHAN_RX_CONTROL, rx & !regs::CHAN_RX_CONTROL_SR);
    let tx = hw.read(regs::CHAN_TX_CONTROL);
    hw.write(regs::CHAN_TX_CONTROL, tx & !regs::CHAN_TX_CONTROL_ST);
    let q = hw.read(regs::MTL_TXQ0_OP_MODE);
    hw.write(regs::MTL_TXQ0_OP_MODE, q | regs::MTL_TXQ_OP_MODE_FTQ);
    let mut flushed = false;
    for _ in 0..10_000 {
        if hw.read(regs::MTL_TXQ0_OP_MODE) & regs::MTL_TXQ_OP_MODE_FTQ == 0 {
            flushed = true;
            break;
        }
        hw.delay_us(1);
    }
    let conf = hw.read(regs::MAC_CONF);
    hw.write(regs::MAC_CONF, conf & !regs::MAC_CONF_TE);
    hw.write(regs::CHAN_INTR_ENA, 0);
    if flushed {
        Ok(())
    } else {
        Err(Error::FlushTimeout)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jh7110::BUS;
    use crate::sim::{Memory, Nic};

    const PHYS: u64 = 0x4800_0000;
    const MAC: [u8; 6] = [0x6c, 0xcf, 0x39, 0x00, 0x47, 0x2c];

    #[test]
    fn a_reset_that_completes_reports_how_long_it_took() {
        let mut nic = Nic::new(PHYS, Memory::coherent(PHYS));
        // The fifth read sees the bit clear, after four 10 µs waits.
        nic.set_reset_polls(5);
        let waited = soft_reset(&mut nic).unwrap();
        assert_eq!(waited, 40);
        assert_eq!(nic.reg(regs::DMA_MODE) & regs::DMA_MODE_SWR, 0);
    }

    #[test]
    fn a_reset_with_no_receive_clock_times_out_rather_than_hanging() {
        let mut nic = Nic::new(PHYS, Memory::coherent(PHYS));
        nic.stop_receive_clock();
        assert_eq!(soft_reset(&mut nic), Err(Error::ResetTimeout));
    }

    #[test]
    fn the_station_address_round_trips_through_its_registers() {
        let (hi, lo) = address_words(MAC);
        assert_eq!(hi, 0x2c47);
        assert_eq!(lo, 0x0039_cf6c);
        assert_eq!(station_address(hi, lo), Some(MAC));
        assert_eq!(station_address(0xffff_ffff, 0xffff_ffff), None);
        assert_eq!(station_address(0, 0), None);
        assert_eq!(station_address(0, 1), None, "multicast");
    }

    #[test]
    fn identity_decodes_the_address_width_and_fifo_sizes() {
        let id = Identity {
            version: 0x1052,
            features: [0, 1 << 14 | 4 << 6 | 4, 0, 0],
        };
        assert_eq!(id.core_version(), 0x52);
        assert_eq!(id.dma_bits(), 40);
        assert_eq!(id.tx_fifo_bytes(), 2048);
        assert_eq!(id.rx_fifo_bytes(), 2048);
    }

    #[test]
    fn a_region_past_the_address_width_or_across_4_gib_is_refused() {
        let bytes = layout::BYTES;
        assert!(region_in_reach(0x4000_0000, bytes, 32));
        assert!(!region_in_reach(0x1_0000_0000, bytes, 32));
        assert!(region_in_reach(0x1_0000_0000, bytes, 40));
        assert!(
            !region_in_reach(0x1_0000_0000 - 4096, bytes, 40),
            "straddles"
        );
        assert!(!region_in_reach(u64::MAX - 10, bytes, 48));
    }

    #[test]
    fn configure_programs_the_rings_with_nothing_available_and_every_interrupt_masked() {
        let mut nic = Nic::new(PHYS, Memory::coherent(PHYS));
        let id = Identity::read(&mut nic);
        configure(&mut nic, &id, &BUS, PHYS, MAC).unwrap();
        assert_eq!(nic.reg(regs::CHAN_RX_BASE), PHYS as u32);
        assert_eq!(nic.reg(regs::CHAN_TX_BASE), (PHYS + layout::TX_RING) as u32);
        assert_eq!(nic.reg(regs::CHAN_RX_TAIL), nic.reg(regs::CHAN_RX_BASE));
        assert_eq!(nic.reg(regs::CHAN_TX_TAIL), nic.reg(regs::CHAN_TX_BASE));
        assert_eq!(nic.reg(regs::CHAN_RX_RING_LEN), 15);
        assert_eq!(
            nic.reg(regs::CHAN_RX_CONTROL) >> 1 & regs::CHAN_RX_CONTROL_RBSZ_MASK,
            2048
        );
        assert_eq!(nic.reg(regs::CHAN_INTR_ENA), 0);
        assert_eq!(nic.reg(regs::MAC_INT_EN), 0);
        assert_eq!(nic.reg(regs::MAC_RXQ_CTRL0), regs::MAC_RXQ_CTRL0_Q0_DCB);
        assert_eq!(
            station_address(nic.reg(regs::MAC_ADDR0_HI), nic.reg(regs::MAC_ADDR0_LO)),
            Some(MAC)
        );
        // radon's bus: fixed burst, no PBLx8, 16-beat bursts, threshold mode.
        assert_ne!(nic.reg(regs::DMA_SYSBUS_MODE) & regs::DMA_SYSBUS_MODE_FB, 0);
        assert_eq!(nic.reg(regs::CHAN_CONTROL) & regs::CHAN_CONTROL_PBLX8, 0);
        assert_eq!(nic.reg(regs::CHAN_TX_CONTROL) >> 16 & 0x3f, 16);
        assert_eq!(
            nic.reg(regs::MTL_RXQ0_OP_MODE) & regs::MTL_RXQ_OP_MODE_RSF,
            0,
            "threshold mode"
        );
    }

    #[test]
    fn the_queue_size_comes_from_the_hardware_not_the_vendor_trees_number() {
        // 262144 bytes, the vendor tree's rx-fifo-depth, is 1023 in a 10-bit field: it would fit,
        // and it would be a lie. The hardware's 2048 is 7.
        let mode = rx_queue_mode(0, &BUS, 2048);
        assert_eq!(mode >> regs::MTL_RXQ_OP_MODE_RQS_SHIFT & 0x3ff, 7);
        assert_eq!(mode & 1 << 7, 0, "no flow control thresholds below 4 KiB");
        let mode = tx_queue_mode(0, &BUS, 2048);
        assert_eq!(mode >> regs::MTL_TXQ_OP_MODE_TQS_SHIFT & 0x1ff, 7);
        assert_eq!(
            mode & regs::MTL_TXQ_OP_MODE_TXQEN_MASK,
            regs::MTL_TXQ_OP_MODE_TXQEN
        );
    }

    #[test]
    fn the_mac_word_selects_speed_and_duplex_and_never_jumbo() {
        let gig = Link {
            speed: Speed::Thousand,
            full_duplex: true,
        };
        let c = mac_conf(regs::MAC_CONF_PS | regs::MAC_CONF_LM, gig, false);
        assert_eq!(c & (regs::MAC_CONF_PS | regs::MAC_CONF_FES), 0);
        assert_ne!(c & regs::MAC_CONF_DM, 0);
        assert_eq!(c & regs::MAC_CONF_LM, 0, "loopback cleared");
        assert_eq!(c & (1 << 16), 0, "JE");
        let c = mac_conf(
            0,
            Link {
                speed: Speed::Hundred,
                full_duplex: false,
            },
            true,
        );
        assert_eq!(
            c & (regs::MAC_CONF_PS | regs::MAC_CONF_FES | regs::MAC_CONF_DM),
            regs::MAC_CONF_PS | regs::MAC_CONF_FES
        );
        assert_ne!(c & regs::MAC_CONF_LM, 0);
    }

    #[test]
    fn stop_quiesces_and_reports_a_flush_that_never_finishes() {
        let mut nic = Nic::new(PHYS, Memory::coherent(PHYS));
        let id = Identity::read(&mut nic);
        configure(&mut nic, &id, &BUS, PHYS, MAC).unwrap();
        start(
            &mut nic,
            Link {
                speed: Speed::Thousand,
                full_duplex: true,
            },
            false,
        );
        assert!(stop(&mut nic).is_ok());
        assert_eq!(nic.reg(regs::CHAN_RX_CONTROL) & regs::CHAN_RX_CONTROL_SR, 0);
        assert_eq!(nic.reg(regs::MAC_CONF) & regs::MAC_CONF_TE, 0);
        nic.wedge_transmit_flush();
        assert_eq!(stop(&mut nic), Err(Error::FlushTimeout));
    }
}
