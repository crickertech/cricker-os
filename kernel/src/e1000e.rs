//! **The `e1000e` NIC's control plane** (milestone 494 (a driver for the network card a PC actually
//! has); notes/e1000e.md).
//!
//! The kernel half of milestone 261 (the NVMe driver leaves the kernel)'s shape applied to a second
//! device. This file finds the controller, confines its DMA to one region, resets it, reads its MAC
//! address, programs both rings' base registers and turns the receiver and transmitter on; then
//! `kernel/src/user/e1000e_service.rs` hands a process the two queue pages of BAR0 and the region,
//! and that process (`net_stack`, through `components/src/e1000e_transport.rs`) fills descriptors
//! and moves tails. Every offset, bit and layout here comes from `crates/e1000e`.
//!
//! What stays here is what the process must not be able to do: reset the device, change the MAC
//! address or the receive filter, turn on promiscuous receive, or reach the PHY. All of it is on
//! BAR0's pages 0 and 5, which the process is never mapped. **What does not stay here is ring
//! placement**: the ring base registers share a page with the tails (`crates/e1000e`'s header), so
//! a process that drives the data plane can repoint a ring, and the IOMMU domain built below is the
//! whole of what bounds the device's DMA.
//!
//! # BUGS
//!
//! - **The reset sequence is the 82574L's, plus FreeBSD's MAC-register steps for the I219**
//!   (`crates/e1000e/src/pch.rs`: ULP exit through the Management Engine, the SPT ring flush, bus
//!   master disable, the STRAP and KABGTXD writes). QEMU proves the first half only; the I219
//!   steps first run on xenon. The PHY-register half of FreeBSD's bring-up is not ported.
//! - **Waiting for link is a bounded spin at bring-up**, up to [`LINK_WAIT_MS`]. QEMU's link is
//!   up at once; a real copper link negotiates in seconds, and a bring-up that times out still
//!   hands the process a working device whose first DHCP DISCOVER may simply be lost (smoltcp
//!   retries). The spin holds a core, which is acceptable once per wiring and not more often.
//! - **The DMA region is allocated once per boot and never returned**, like the NVMe region, so a
//!   boot that wires the NIC carries [`::e1000e::layout::PAGES`] pages for its whole life.

use core::sync::atomic::{AtomicU64, Ordering};

use ::e1000e::{layout, regs};

use crate::arch::mmu;

/// The longest bring-up waits for `CTRL.RST` to clear.
const RESET_WAIT_MS: u64 = 1000;
/// The longest bring-up waits for `STATUS.LU`.
pub const LINK_WAIT_MS: u64 = 3000;

/// Why a controller that is on the bus was not brought up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// `CTRL.RST` did not self-clear.
    ResetTimeout,
    /// Receive-address entry 0 was not marked valid after reset: the NVM load did not happen.
    NoMacAddress,
}

/// Why there is no NIC to hand out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Absent {
    /// No function on the bus is one `crates/e1000e` claims.
    NoController,
    /// One is, and bring-up refused it. Never a skip.
    Refused {
        /// The controller's requester id.
        rid: u32,
        /// What went wrong.
        why: Error,
    },
}

/// A controller reset, programmed, and ready for a data plane.
#[derive(Debug, Clone, Copy)]
pub struct Found {
    /// BAR0's physical base. The process is mapped `bar0 + RX_QUEUE_PAGE` and `+ TX_QUEUE_PAGE`.
    pub bar0: u64,
    /// The requester id the IOMMU confines.
    pub rid: u32,
    /// The PCI device id.
    pub device: u16,
    /// What the process is told: the MAC address and the region's physical base.
    pub handoff: ::e1000e::Handoff,
    /// Whether `STATUS.LU` was set before [`LINK_WAIT_MS`] ran out.
    pub link_up: bool,
}

/// The region's physical base, once allocated: zero until the first bring-up.
static REGION: AtomicU64 = AtomicU64::new(0);

/// **Find, confine, reset and program the NIC.** Called once per data plane; a second call resets
/// the device under whatever process held the first, so the service that calls this refuses while
/// that process lives (`e1000e_service`'s `BUGS`).
pub fn bring_up() -> Result<Found, Absent> {
    let dev = crate::pci::find_e1000e_device().ok_or(Absent::NoController)?;
    let region = region_for(dev.rid);
    let c = Controller(mmu::phys_to_virt(dev.bar0));
    let refused = |why| Absent::Refused { rid: dev.rid, why };
    // An I219 behind a PCH wants FreeBSD's MAC-register steps around the reset
    // (`crates/e1000e/src/pch.rs`, which has the sources and the licence). QEMU's 82574L does not,
    // and none of this runs under the gates.
    let pch = ::e1000e::pch::is_pch(dev.device);
    if pch {
        leave_ultra_low_power(&c);
        flush_descriptor_rings(&c, &dev, region);
        disable_bus_mastering(&c);
    }

    // Quiesce: no interrupts, receiver and transmitter off, then a posted-write flush.
    c.w(regs::IMC, u32::MAX);
    c.w(regs::RCTL, 0);
    c.w(regs::TCTL, 0);
    let _ = c.r(regs::STATUS);
    spin_ms(10);

    // Global reset. Self-clearing; the registers read garbage while it runs, so wait first. On a
    // PCH part FreeBSD writes the vendor-id dword into the read-only STRAP register on each side
    // of it, because the configuration-space read that produces the value is the delay the
    // hardware needs ("Read from EXTCNF_CTRL ... may occur during global reset and cause system
    // hang. Configuration space access creates the needed delay.").
    let strap = || {
        if pch {
            c.w(::e1000e::pch::regs::STRAP, dev.config_read32(0) & 0xffff);
        }
    };
    strap();
    c.w(regs::CTRL, c.r(regs::CTRL) | ::e1000e::CTRL_RST);
    spin_ms(20);
    strap();
    if !wait_ms(RESET_WAIT_MS, || c.r(regs::CTRL) & ::e1000e::CTRL_RST == 0) {
        return Err(refused(Error::ResetTimeout));
    }
    // Reset re-enabled nothing we want; mask again and clear anything pending.
    c.w(regs::IMC, u32::MAX);
    let _ = c.r(regs::ICR);
    if pch {
        let k = ::e1000e::pch::regs::KABGTXD;
        c.w(k, c.r(k) | ::e1000e::pch::reset::KABGTXD_BGSQLBIAS);
    }
    zero(region);

    // Tell management firmware a driver has the device, and bring the link up.
    c.w(
        regs::CTRL_EXT,
        c.r(regs::CTRL_EXT) | ::e1000e::CTRL_EXT_DRV_LOAD,
    );
    c.w(
        regs::CTRL,
        c.r(regs::CTRL) | ::e1000e::CTRL_SLU | ::e1000e::CTRL_ASDE,
    );

    let mac = ::e1000e::mac_from_receive_address(c.r(regs::RAL0), c.r(regs::RAH0))
        .ok_or(refused(Error::NoMacAddress))?;

    // No multicast groups: the table is unspecified after reset.
    for k in 0..regs::MTA_DWORDS {
        c.w(regs::MTA + 4 * k, 0);
    }
    // Legacy receive descriptors, the only format `crates/e1000e` reads.
    c.w(regs::RFCTL, c.r(regs::RFCTL) & !::e1000e::RFCTL_EXTEN);

    // The rings. Heads and tails at zero: `DataPlane::start` posts the receive buffers by moving
    // the receive tail, which is the process's first act.
    let rx = region + layout::RX_RING;
    let tx = region + layout::TX_RING;
    c.w(regs::RDBAL, rx as u32);
    c.w(regs::RDBAH, (rx >> 32) as u32);
    c.w(regs::RDLEN, u32::from(::e1000e::RX_ENTRIES) * 16);
    c.w(regs::RDH, 0);
    c.w(regs::RDT, 0);
    c.w(regs::TDBAL, tx as u32);
    c.w(regs::TDBAH, (tx >> 32) as u32);
    c.w(regs::TDLEN, u32::from(::e1000e::TX_ENTRIES) * 16);
    c.w(regs::TDH, 0);
    c.w(regs::TDT, 0);

    c.w(regs::TIPG, ::e1000e::tipg());
    c.w(regs::TCTL, ::e1000e::tctl());
    c.w(regs::RCTL, ::e1000e::rctl());

    let link_up = wait_ms(LINK_WAIT_MS, || {
        c.r(regs::STATUS) & ::e1000e::STATUS_LU != 0
    });

    Ok(Found {
        bar0: dev.bar0,
        rid: dev.rid,
        device: dev.device,
        handoff: ::e1000e::Handoff {
            mac,
            data_plane_phys: region,
        },
        link_up,
    })
}

/// The DMA region: allocated and confined on the first call, the same region on every later one.
fn region_for(rid: u32) -> u64 {
    let base = REGION.load(Ordering::Acquire);
    if base != 0 {
        return base;
    }
    let base = crate::memory::alloc_contiguous_zeroed(layout::PAGES as usize)
        .expect("no DMA region for the e1000e NIC")
        .addr();
    if crate::iommu::is_active() {
        crate::iommu::confine(
            rid,
            &[paging::domain::DmaRegion {
                base,
                size: layout::PAGES * page_frames::FRAME_SIZE,
                writable: true,
            }],
        );
    }
    REGION.store(base, Ordering::Release);
    base
}

/// Zero the region, after the reset and before the rings are programmed. A stale descriptor status
/// from a previous wiring would otherwise read as a received frame.
fn zero(region: u64) {
    // SAFETY: `region` is the run `region_for` allocated, `layout::BYTES` long, in the direct map.
    // The device was reset just before this, so it is not DMA-ing into it, and the service refuses
    // a new wiring while the previous process lives, so no process has it mapped.
    unsafe {
        core::ptr::write_bytes(
            mmu::phys_to_virt(region) as *mut u8,
            0,
            layout::BYTES as usize,
        );
    }
}

/// **Leave ultra-low-power mode through the Management Engine**, FreeBSD's
/// `e1000_disable_ulp_lpt_lp` with `force`, its ME branch. Without an ME the exit is a PHY-register
/// sequence that is not ported (`crates/e1000e/src/pch.rs`), and the boot says so rather than
/// guessing.
fn leave_ultra_low_power(c: &Controller) {
    use ::e1000e::pch::{regs as pch, ulp};
    if !ulp::has_me(c.r(pch::FWSM)) {
        crate::println!(
            "  e1000e: no Management Engine; leaving ULP by PHY registers is not ported, so a PHY \
             firmware left in ULP stays there"
        );
        return;
    }
    c.w(pch::H2ME, ulp::request(c.r(pch::H2ME)));
    if !wait_ms(ulp::WAIT_MS, || ulp::done(c.r(pch::FWSM))) {
        crate::println!("  e1000e: the Management Engine did not finish leaving ULP in time");
    }
    c.w(pch::H2ME, ulp::release(c.r(pch::H2ME)));
}

/// **Empty the descriptor rings before the reset, if the hardware says it must**, FreeBSD's
/// `em_flush_desc_rings`. Skipped when the transmit ring the device points at is not this
/// driver's region: the dummy descriptor has to go somewhere both the device may read and this
/// kernel owns, and a ring firmware left behind is neither.
fn flush_descriptor_rings(c: &Controller, dev: &crate::pci::PciE1000eDevice, region: u64) {
    use ::e1000e::pch::{flush, regs as pch};
    c.w(
        pch::FEXTNVM11,
        c.r(pch::FEXTNVM11) | flush::FEXTNVM11_DISABLE_MULR_FIX,
    );
    let status = || dev.config_read32(flush::DESC_RING_STATUS) as u16;
    if !flush::required(status(), c.r(regs::TDLEN)) {
        return;
    }
    let ring = region + layout::TX_RING;
    let tdba = u64::from(c.r(regs::TDBAL)) | u64::from(c.r(regs::TDBAH)) << 32;
    if tdba != ring {
        crate::println!(
            "  e1000e: the rings need flushing and the transmit ring at {tdba:#x} is not this \
             driver's; skipped, so the reset may hang the device"
        );
        return;
    }
    c.w(regs::TCTL, c.r(regs::TCTL) | 0x2);
    let tdt = c.r(regs::TDT) % u32::from(::e1000e::TX_ENTRIES);
    let d = mmu::phys_to_virt(region + layout::tx_descriptor(tdt as u16)) as *mut u64;
    // SAFETY: `d` is a 16-byte descriptor inside the transmit ring page of this driver's region
    // (`layout::tx_descriptor` reduces its index), in the direct map. The device reads it only
    // after the tail write below, which the barrier orders after these stores.
    unsafe {
        core::ptr::write_volatile(d, ring);
        core::ptr::write_volatile(d.add(1), flush::tx_word());
    }
    crate::arch::direct_memory_access_write_barrier();
    c.w(regs::TDT, flush::tail_after(tdt, ::e1000e::TX_ENTRIES));
    spin_us(250);
    if status() & flush::REQUIRED == 0 {
        return;
    }
    let rctl = c.r(regs::RCTL);
    c.w(regs::RCTL, rctl & !0x2);
    let _ = c.r(regs::STATUS);
    spin_us(150);
    c.w(pch::RXDCTL, flush::rxdctl(c.r(pch::RXDCTL)));
    c.w(regs::RCTL, rctl | 0x2);
    let _ = c.r(regs::STATUS);
    spin_us(150);
    c.w(regs::RCTL, rctl & !0x2);
}

/// **Stop new bus-master requests and wait for pending ones**, FreeBSD's
/// `e1000_disable_pcie_master_generic`, so the PCIe link does not stick across the reset.
fn disable_bus_mastering(c: &Controller) {
    use ::e1000e::pch::reset;
    c.w(regs::CTRL, c.r(regs::CTRL) | reset::CTRL_GIO_MASTER_DISABLE);
    let deadline = reset::MASTER_WAIT_US.div_ceil(1000);
    if !wait_ms(deadline, || {
        c.r(regs::STATUS) & reset::STATUS_GIO_MASTER_ENABLE == 0
    }) {
        crate::println!("  e1000e: master requests still pending after 80 ms; resetting anyway");
    }
}

/// BAR0, through the direct map.
struct Controller(u64);

impl Controller {
    fn r(&self, off: u64) -> u32 {
        // SAFETY: BAR0 is device-mapped inside the PCI BAR window `mmu::map_everything` maps, and
        // every `off` is a `crates/e1000e::regs` constant, inside the 82574L's 128 KiB BAR.
        unsafe { core::ptr::read_volatile((self.0 + off) as *const u32) }
    }
    fn w(&self, off: u64, v: u32) {
        // SAFETY: as `r`.
        unsafe { core::ptr::write_volatile((self.0 + off) as *mut u32, v) }
    }
}

fn spin_us(us: u64) {
    crate::arch::timer::spin_for(us * crate::arch::timer::frequency() / 1_000_000);
}

fn spin_ms(ms: u64) {
    crate::arch::timer::spin_for(ms * crate::arch::timer::frequency() / 1000);
}

/// Poll `done` until it holds or `ms` pass. True when it held.
fn wait_ms(ms: u64, mut done: impl FnMut() -> bool) -> bool {
    let deadline = crate::arch::timer::now() + ms * crate::arch::timer::frequency() / 1000;
    loop {
        if done() {
            return true;
        }
        if crate::arch::timer::now() >= deadline {
            return done();
        }
        core::hint::spin_loop();
    }
}
