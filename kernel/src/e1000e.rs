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
//! - **The reset sequence is the 82574L's.** It is what QEMU proves. An I219 behind a PCH is
//!   believed to want more (`crates/e1000e`'s `BUGS`), and xenon's bench step is the test.
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

    // Quiesce: no interrupts, receiver and transmitter off, then a posted-write flush.
    c.w(regs::IMC, u32::MAX);
    c.w(regs::RCTL, 0);
    c.w(regs::TCTL, 0);
    let _ = c.r(regs::STATUS);
    spin_ms(10);

    // Global reset. Self-clearing; the registers read garbage while it runs, so wait first.
    c.w(regs::CTRL, c.r(regs::CTRL) | ::e1000e::CTRL_RST);
    spin_ms(20);
    if !wait_ms(RESET_WAIT_MS, || c.r(regs::CTRL) & ::e1000e::CTRL_RST == 0) {
        return Err(refused(Error::ResetTimeout));
    }
    // Reset re-enabled nothing we want; mask again and clear anything pending.
    c.w(regs::IMC, u32::MAX);
    let _ = c.r(regs::ICR);
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
