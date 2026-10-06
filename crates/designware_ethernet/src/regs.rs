//! **The controller's register map**: offsets from the 64 KiB window's base, and the bits this
//! driver writes or reads.
//!
//! Transcribed from OpenBSD's `sys/dev/ic/dwqereg.h` (revision of 2026-07-31, commit
//! `59ec3ab4d107`, ISC license, notice in the crate root), which is the header OpenBSD's JH7110
//! driver is built on, and cross-checked against Linux's `stmmac/dwmac4.h` for the one bit
//! OpenBSD does not name ([`MAC_CONF_LM`]) and U-Boot's `dwc_eth_qos.h` for the receive buffer
//! size field OpenBSD never writes ([`CHAN_RX_CONTROL_RBSZ_SHIFT`]). Only what this driver uses
//! is here; the header has more.
//!
//! # Where the pages fall, which decides what a process can be given
//!
//! | page | holds |
//! |---|---|
//! | `0x0000` | MAC: configuration, packet filter, flow control, interrupt enables, version, hardware features, MDIO, the station address |
//! | `0x0c00` (still page 0) | MTL: the queue operation modes |
//! | `0x1000` | DMA: the bus mode (with **software reset**), the system bus mode, and channel 0's control, ring bases, ring lengths, **tail pointers** and status |
//!
//! So the channel's tail pointers share a page with its ring bases and with the bit that resets
//! the whole controller. A process that can move a tail can repoint a ring and reset the MAC, and
//! the JH7110 has no IOMMU in front of this device. That is the whole confinement story, and
//! `kernel/src/user/designware_ethernet_service.rs`'s header carries it.

/// MAC configuration.
pub const MAC_CONF: u32 = 0x0000;
/// Receiver enable.
pub const MAC_CONF_RE: u32 = 1 << 0;
/// Transmitter enable.
pub const MAC_CONF_TE: u32 = 1 << 1;
/// Disable carrier sense during transmission.
pub const MAC_CONF_DCRS: u32 = 1 << 9;
/// **Loopback mode**: the MAC returns what it transmits to its own receive path (Linux
/// `GMAC_CONFIG_LM`, `dwmac4.h`). The coherence probe uses it so the measurement does not depend
/// on a cable or a link partner.
pub const MAC_CONF_LM: u32 = 1 << 12;
/// Full duplex.
pub const MAC_CONF_DM: u32 = 1 << 13;
/// Speed: with [`MAC_CONF_PS`], 100 Mbit/s when set and 10 when clear.
pub const MAC_CONF_FES: u32 = 1 << 14;
/// Port select: set for 10 or 100 Mbit/s (MII), clear for 1000 (GMII/RGMII).
pub const MAC_CONF_PS: u32 = 1 << 15;
/// Jabber disable.
pub const MAC_CONF_JD: u32 = 1 << 17;
/// Packet burst enable.
pub const MAC_CONF_BE: u32 = 1 << 18;

/// The receive packet filter. Zero after reset: the station address and broadcast pass,
/// multicast does not.
pub const MAC_PACKET_FILTER: u32 = 0x0008;
/// Transmit flow control for queue 0.
pub const MAC_Q0_TX_FLOW_CTRL: u32 = 0x0070;
/// Pause time field shift in [`MAC_Q0_TX_FLOW_CTRL`].
pub const MAC_Q0_TX_FLOW_CTRL_PT_SHIFT: u32 = 16;
/// Transmit flow control enable.
pub const MAC_Q0_TX_FLOW_CTRL_TFE: u32 = 1 << 0;
/// Receive flow control.
pub const MAC_RX_FLOW_CTRL: u32 = 0x0090;
/// Receive flow control enable.
pub const MAC_RX_FLOW_CTRL_RFE: u32 = 1 << 0;
/// Receive queue control 0: which receive queues are enabled.
pub const MAC_RXQ_CTRL0: u32 = 0x00a0;
/// Receive queue 0 enabled for generic (DCB) traffic. **Without this the MAC receives nothing**
/// on the 4.x and 5.x controllers, whatever the DMA is doing.
pub const MAC_RXQ_CTRL0_Q0_DCB: u32 = 2;
/// The MAC interrupt enable register.
pub const MAC_INT_EN: u32 = 0x00b4;
/// Synopsys version register; the low byte is the core version (`0x52` for 5.20).
pub const MAC_VERSION: u32 = 0x0110;
/// Hardware feature register `i`, for `i` in `0..4`.
pub const fn mac_hw_feature(i: u32) -> u32 {
    0x011c + 4 * (i % 4)
}
/// MDIO address and command.
pub const MAC_MDIO_ADDR: u32 = 0x0200;
/// MDIO data.
pub const MAC_MDIO_DATA: u32 = 0x0204;
/// Station address 0, high half (bytes 4 and 5).
pub const MAC_ADDR0_HI: u32 = 0x0300;
/// Station address 0, low half (bytes 0 to 3).
pub const MAC_ADDR0_LO: u32 = 0x0304;
/// The MMC receive counters' interrupt mask: all ones masks every counter interrupt.
pub const MMC_RX_INT_MASK: u32 = 0x070c;
/// The MMC transmit counters' interrupt mask.
pub const MMC_TX_INT_MASK: u32 = 0x0710;

/// MTL transmit queue 0 operation mode.
pub const MTL_TXQ0_OP_MODE: u32 = 0x0d00;
/// Flush the transmit queue; self-clearing.
pub const MTL_TXQ_OP_MODE_FTQ: u32 = 1 << 0;
/// Transmit store and forward.
pub const MTL_TXQ_OP_MODE_TSF: u32 = 1 << 1;
/// Transmit queue enable field.
pub const MTL_TXQ_OP_MODE_TXQEN_MASK: u32 = 0x3 << 2;
/// Transmit queue enabled.
pub const MTL_TXQ_OP_MODE_TXQEN: u32 = 2 << 2;
/// Transmit threshold control field.
pub const MTL_TXQ_OP_MODE_TTC_MASK: u32 = 0x7 << 4;
/// Transmit threshold of 512 bytes, what OpenBSD picks in threshold mode.
pub const MTL_TXQ_OP_MODE_TTC_512: u32 = 7 << 4;
/// Transmit queue size field.
pub const MTL_TXQ_OP_MODE_TQS_MASK: u32 = 0x1ff << 16;
/// Transmit queue size shift: the field holds `bytes / 256 - 1`.
pub const MTL_TXQ_OP_MODE_TQS_SHIFT: u32 = 16;
/// MTL receive queue 0 operation mode.
pub const MTL_RXQ0_OP_MODE: u32 = 0x0d30;
/// Receive threshold control field.
pub const MTL_RXQ_OP_MODE_RTC_MASK: u32 = 0x3 << 3;
/// Receive threshold of 128 bytes.
pub const MTL_RXQ_OP_MODE_RTC_128: u32 = 3 << 3;
/// Receive store and forward.
pub const MTL_RXQ_OP_MODE_RSF: u32 = 1 << 5;
/// Receive queue size field.
pub const MTL_RXQ_OP_MODE_RQS_MASK: u32 = 0x3ff << 20;
/// Receive queue size shift: the field holds `bytes / 256 - 1`.
pub const MTL_RXQ_OP_MODE_RQS_SHIFT: u32 = 20;

/// The DMA bus mode.
pub const DMA_MODE: u32 = 0x1000;
/// **Software reset** of the whole controller (MAC, MTL and DMA); self-clearing. The bit that
/// makes the DMA page an administrative page as well as a data one.
pub const DMA_MODE_SWR: u32 = 1 << 0;
/// The DMA system bus (AXI) mode.
pub const DMA_SYSBUS_MODE: u32 = 0x1004;
/// Fixed burst.
pub const DMA_SYSBUS_MODE_FB: u32 = 1 << 0;
/// AXI burst length 32 permitted.
pub const DMA_SYSBUS_MODE_BLEN_32: u32 = 1 << 4;
/// AXI burst length 64 permitted.
pub const DMA_SYSBUS_MODE_BLEN_64: u32 = 1 << 5;
/// AXI burst length 128 permitted.
pub const DMA_SYSBUS_MODE_BLEN_128: u32 = 1 << 6;
/// AXI burst length 256 permitted.
pub const DMA_SYSBUS_MODE_BLEN_256: u32 = 1 << 7;
/// Enhanced address mode: descriptor addresses wider than 32 bits.
pub const DMA_SYSBUS_MODE_EAME: u32 = 1 << 11;
/// Outstanding read requests limit field.
pub const DMA_SYSBUS_MODE_RD_OSR_LMT_SHIFT: u32 = 16;
/// Outstanding write requests limit field.
pub const DMA_SYSBUS_MODE_WR_OSR_LMT_SHIFT: u32 = 24;
/// Both limit fields' width.
pub const DMA_SYSBUS_MODE_OSR_LMT_MASK: u32 = 0xf;

/// DMA channel 0 control.
pub const CHAN_CONTROL: u32 = 0x1100;
/// Multiply both programmable burst lengths by eight. Both JH7110 trees say `snps,no-pbl-x8`.
pub const CHAN_CONTROL_PBLX8: u32 = 1 << 16;
/// Channel 0 transmit control.
pub const CHAN_TX_CONTROL: u32 = 0x1104;
/// Start transmit DMA.
pub const CHAN_TX_CONTROL_ST: u32 = 1 << 0;
/// Operate on a second frame: fetch the next descriptor before the status of this one is written.
pub const CHAN_TX_CONTROL_OSP: u32 = 1 << 4;
/// Transmit programmable burst length shift.
pub const CHAN_TX_CONTROL_PBL_SHIFT: u32 = 16;
/// Both programmable burst length fields' mask, after the shift.
pub const CHAN_PBL_MASK: u32 = 0x3f;
/// Channel 0 receive control.
pub const CHAN_RX_CONTROL: u32 = 0x1108;
/// Start receive DMA.
pub const CHAN_RX_CONTROL_SR: u32 = 1 << 0;
/// The receive buffer size field, in bytes, bits 14:1 (U-Boot's `EQOS_DMA_CH0_RX_CONTROL_RBSZ_*`,
/// Linux's `DMA_RBSZ_MASK`). OpenBSD never writes it and relies on the reset value; this driver
/// writes it because a field nobody wrote is a fact about whoever ran last.
pub const CHAN_RX_CONTROL_RBSZ_SHIFT: u32 = 1;
/// The receive buffer size field's mask, after the shift.
pub const CHAN_RX_CONTROL_RBSZ_MASK: u32 = 0x3fff;
/// Receive programmable burst length shift.
pub const CHAN_RX_CONTROL_PBL_SHIFT: u32 = 16;
/// Transmit descriptor ring base, high 32 bits.
pub const CHAN_TX_BASE_HI: u32 = 0x1110;
/// Transmit descriptor ring base, low 32 bits.
pub const CHAN_TX_BASE: u32 = 0x1114;
/// Receive descriptor ring base, high 32 bits.
pub const CHAN_RX_BASE_HI: u32 = 0x1118;
/// Receive descriptor ring base, low 32 bits.
pub const CHAN_RX_BASE: u32 = 0x111c;
/// **Transmit tail pointer**: the address one past the last descriptor handed to the device.
pub const CHAN_TX_TAIL: u32 = 0x1120;
/// **Receive tail pointer**, the same for the receive ring.
pub const CHAN_RX_TAIL: u32 = 0x1128;
/// Transmit ring length: the descriptor count minus one.
pub const CHAN_TX_RING_LEN: u32 = 0x112c;
/// Receive ring length: the descriptor count minus one.
pub const CHAN_RX_RING_LEN: u32 = 0x1130;
/// Channel 0 interrupt enable. This driver polls and writes zero.
pub const CHAN_INTR_ENA: u32 = 0x1134;
/// The descriptor the transmit DMA is on, as the device sees it.
pub const CHAN_CUR_TX_DESC: u32 = 0x1144;
/// The descriptor the receive DMA is on, as the device sees it.
pub const CHAN_CUR_RX_DESC: u32 = 0x114c;
/// Channel 0 status. Write one to clear.
pub const CHAN_STATUS: u32 = 0x1160;
/// Transmit complete.
pub const CHAN_STATUS_TI: u32 = 1 << 0;
/// Transmit process stopped.
pub const CHAN_STATUS_TPS: u32 = 1 << 1;
/// **Transmit buffer unavailable**: the DMA fetched a descriptor it does not own and suspended.
pub const CHAN_STATUS_TBU: u32 = 1 << 2;
/// Receive complete.
pub const CHAN_STATUS_RI: u32 = 1 << 6;
/// **Receive buffer unavailable**: the DMA fetched a receive descriptor it does not own.
pub const CHAN_STATUS_RBU: u32 = 1 << 7;
/// Receive process stopped.
pub const CHAN_STATUS_RPS: u32 = 1 << 8;
/// Fatal bus error: the DMA's own access to memory failed.
pub const CHAN_STATUS_FBE: u32 = 1 << 12;

/// The page of the window holding channel 0's registers, which is the page the data plane is
/// mapped. Every `CHAN_*` and `DMA_*` offset above is inside it.
pub const DMA_PAGE: u32 = 0x1000;
/// The window's size, `reg`'s `0x10000` in both JH7110 trees.
pub const WINDOW: u32 = 0x1_0000;

#[cfg(test)]
mod tests {
    use super::*;

    /// Every register the data plane writes or reads is on the DMA page, which is the one page
    /// the process is mapped; a register that moved off it would be a store into nothing.
    #[test]
    fn the_data_plane_registers_are_all_on_the_dma_page() {
        for r in [
            CHAN_TX_TAIL,
            CHAN_RX_TAIL,
            CHAN_STATUS,
            CHAN_CUR_RX_DESC,
            DMA_MODE,
        ] {
            assert_eq!(r & !0xfff, DMA_PAGE, "{r:#x}");
        }
    }

    /// The MAC's administrative registers are not on it: the process cannot reach the MDIO bus
    /// or the station address through the page it is given.
    #[test]
    fn the_mdio_bus_and_the_station_address_are_not_on_the_dma_page() {
        for r in [
            MAC_MDIO_ADDR,
            MAC_MDIO_DATA,
            MAC_ADDR0_HI,
            MAC_PACKET_FILTER,
            MAC_CONF,
        ] {
            assert_ne!(r & !0xfff, DMA_PAGE, "{r:#x}");
        }
    }
}
