//! PCIe **enumeration and virtio-pci bring-up**. Not a driver.
//!
//! The PCIe analog of virtio.rs's discovery half, and the same division of labor: the kernel
//! walks the bus (a bootstrap role), finds the block device, brings its register blocks up, and
//! hands a userspace driver a confined transport capability. The decode logic (ECAM addressing,
//! BAR sizing, capability parsing) lives in the host-tested `pci` crate; this module supplies
//! only the volatile accessors into the mapped ECAM window and the policy: which device to pick,
//! where BARs go, which command bits to set. See notes/pcie.md and notes/pcie-transport-scope.md.
//!
//! Portable: everything here except the `arch::mmu` policy/irq constants is architecture-
//! neutral, and every window this module reads comes through one seam, `memory::pci_regions()`.
//! Both `virt` boards fill it from the `pci-host-ecam-generic` node their device tree states;
//! `x86_64` fills the same static from ACPI's MCFG (`arch::x86_64::machine::enable_pcie_ecam`,
//! called once from `main.rs`), since it has no device tree to read a node from. A machine that
//! names no window at all (the JH7110's tree, or a machine with no MCFG) answers every probe
//! here with "nobody home", no MMIO touched.
//!
//! **How a function's interrupt reaches a driver is the machine's answer, not this module's**
//! (milestone 215, `x86_64` PCI interrupt routing). `arch::irq::alloc_msi_vector` is asked first: on
//! `x86_64` it reserves a local APIC vector and this module programs it into the function's MSI-X
//! table, because a legacy pin on `q35` goes through a PIRQ router only ACPI's `_PRT` describes
//! and `_PRT` is AML. Both `virt` boards answer `None` and keep the INTx swizzle they have always
//! used: the PLIC (sources 32..35) on riscv, GIC SPIs (INTIDs 35..38) on aarch64. Each arch's
//! constants say so, and host-run witnesses hold the device-tree architectures' swizzle against
//! the machine's own tree.

use core::sync::atomic::{AtomicU16, AtomicU64, Ordering};

use paging::PAGE_SIZE;
use pci::{Bar, Bdf, VirtioCap};

use crate::arch::mmu::{self, PCI_BAR_MAPPED, PCI_ECAM_BUSES, PCI_IRQ_BASE};

/// One bus of ECAM configuration space: 32 devices of 8 functions of 4 KiB.
const BUS_BYTES: u64 = 0x10_0000;

/// **How many buses of configuration space this kernel has mapped, and may therefore read.** Zero
/// means no survey has run and [`ecam_buses`] answers the architecture's compiled-in default.
///
/// It is a runtime value rather than a constant because the answer is the machine's (milestone
/// 320). `arch::mmu::PCI_ECAM_BUSES` is 1 on all three architectures and was right for a year,
/// because QEMU hangs every device off the root complex on every board this tree boots. A machine
/// whose firmware put a controller behind a PCIe root port needs the bus behind that port mapped
/// before anything can read it, and nothing compiled in can know which bus that is.
static ECAM_BUSES: AtomicU16 = AtomicU16::new(0);

/// The ECAM window's physical base, cached from `memory::pci_regions` by
/// [`is_host_bridge_present`]. Zero means "not cached yet, or no bridge", and cannot collide with a
/// real value: no machine puts config space at physical zero (that is RAM or a vector table
/// everywhere this runs).
static ECAM_BASE: AtomicU64 = AtomicU64::new(0);

/// The shared bump cursor for kernel-assigned BARs, starting at the discovered 32-bit memory
/// window's base. With `-bios default` the kernel is the PCI firmware (OpenSBI does no PCI
/// setup), so every BAR arrives zero and the kernel places it. More than one function needs
/// placing: the virtio disk, and on riscv the IOMMU (itself a PCI function, milestone 16b). Two
/// independent bump allocators would hand out the same address twice, so the cursor is one shared
/// value that only ever advances. Enumeration runs on the boot hart alone; the atomics here are
/// for correctness of the shared state, not contention.
static BAR_NEXT: AtomicU64 = AtomicU64::new(0);

/// One past the last byte a BAR may occupy: the discovered window base plus the mapped slice
/// (`mmu::map_everything` maps the same `PCI_BAR_MAPPED`-capped slice, so this limit is exactly
/// what is addressable).
static BAR_LIMIT: AtomicU64 = AtomicU64::new(0);

/// **The exclusive bus-number bound every walk in this module passes to `pci::walk`**: how many
/// buses of configuration space are mapped, so how many can be read without faulting.
///
/// The compiled-in `PCI_ECAM_BUSES` until [`survey`] has run, and what the survey found afterwards.
/// On both `virt` boards no survey runs and this is 1 forever, which is the truth there: their
/// device trees describe a flat bus and QEMU puts nothing behind a bridge.
pub fn ecam_buses() -> u16 {
    match ECAM_BUSES.load(Ordering::Relaxed) {
        0 => PCI_ECAM_BUSES,
        n => n,
    }
}

/// **Bytes of ECAM the direct map must cover**, for each architecture's `map_everything`.
///
/// One seam rather than three copies of `PCI_ECAM_MAPPED.min(ecam_size)`, so that the day a
/// device-tree machine grows a root port there is one place that already knows the answer. The
/// caller still clamps against the window the machine actually described: a survey cannot report a
/// bus outside it, but `PCI_ECAM_BUSES` on a machine that describes less than one bus could.
///
/// Name: provisional (milestone 320 (every PCI bus the machine has)), every name listed here: this,
/// [`ecam_buses`], [`survey`] and `NIFE_PCIE_ROOT_PORT` in `helpers/qemu-runner-x86_64.sh`. calef
/// names public items.
pub fn ecam_bytes() -> u64 {
    u64::from(ecam_buses()) * BUS_BYTES
}

/// **Print every function on every bus the machine has, and record how many buses that is**
/// (milestone 320). Returns the number of functions found.
///
/// # Why this exists, and why it is a print rather than a count
///
/// The kernel mapped one megabyte of configuration space and enumerated bus 0, while ACPI's MCFG
/// described a hundred and twenty-eight buses. Both facts were printed by every `x86_64` boot, two
/// lines apart, for a year. On QEMU's `q35` the two agree in effect, because everything is on bus
/// 0. On xenon the NVMe controller is behind a PCIe root port, so [`find_nvme_device`] searched a
/// bus the disk was never on and `non_volatile_memory_express_tests` **skipped** with QEMU's explanation for an absence
/// that had an entirely different cause.
///
/// A count would not have caught that and did not: `bar_census` printed "15 function(s) on the bus"
/// and fifteen functions is a perfectly ordinary number for a machine whose disk is missing. What
/// was needed was the list, because the finding is in *which* functions those were.
///
/// # When it must run
///
/// **Before the fine map replaces the boot map**, which is the whole reason this is possible
/// without lazy mapping machinery. On `x86_64` the boot tables cover the low 4 GiB
/// indiscriminately, so at this point in boot every bus the MCFG describes is already readable and
/// the survey costs no mapping at all; `arch::mmu::map_everything` then maps exactly the buses the
/// survey found. Called later it would read its way straight off the end of the mapped window.
///
/// # What it cannot do, and this is a real limit rather than a caveat
///
/// It maps **bus 0 through the highest bus it found, contiguously**, not the set of buses it found.
/// A machine whose firmware numbers a root port's subtree 0x60 pays sixty-one buses of page tables
/// for two buses of devices. Nothing measured yet does that (see `BUGS` in notes/pcie.md), and the
/// cost is bounded by what a flat map of the whole MCFG window would have cost anyway, so the set
/// version is not built until a machine asks for it.
#[cfg_attr(not(target_arch = "x86_64"), allow(dead_code))]
pub fn survey() -> usize {
    if !is_host_bridge_present() {
        return 0;
    }
    // The whole window the machine described, which is what is readable right now and is never
    // what stays mapped. `pci::walk` clamps this to 256 itself.
    let described =
        crate::memory::pci_regions().map_or(0, |((_, size), _)| (size / BUS_BYTES).min(256) as u16);

    let (mut functions, mut highest) = (0usize, 0u8);
    pci::walk(described, &mut |b, o| cfg_read32(b, o), &mut |f| {
        functions += 1;
        highest = highest.max(f.bdf.bus);
        match f.bridge {
            Some(b) => {
                highest = highest.max(b.subordinate);
                crate::println!(
                    "                {:02x}:{:02x}.{} {:04x}:{:04x} class {:06x} \
                     bridge to buses {:02x}..={:02x}",
                    f.bdf.bus,
                    f.bdf.dev,
                    f.bdf.func,
                    f.vendor,
                    f.device,
                    f.class,
                    b.secondary,
                    b.subordinate,
                );
            }
            None => crate::println!(
                "                {:02x}:{:02x}.{} {:04x}:{:04x} class {:06x}",
                f.bdf.bus,
                f.bdf.dev,
                f.bdf.func,
                f.vendor,
                f.device,
                f.class,
            ),
        }
    });

    // `highest` takes a bridge's SUBORDINATE bus and not only the buses that answered, because a
    // subtree firmware left room in is a subtree a hotplugged device appears in, and because a
    // bridge whose secondary bus is empty still decodes it.
    let buses = u16::from(highest) + 1;
    ECAM_BUSES.store(buses.min(described.max(1)), Ordering::Relaxed);
    crate::println!(
        "                {functions} function(s) over {} bus(es) of the {described} described; \
         mapping {} KiB of config space",
        buses,
        ecam_bytes() / 1024,
    );
    functions
}

/// True when `memory::pci_regions()` named a host bridge, caching its windows on first call.
/// Every public entry point checks this before touching config space, so on a machine with
/// nothing recorded there (the JH7110's tree has no generic-ECAM node; an x86 machine's ACPI has
/// no MCFG) every probe reports nobody home without a single MMIO access, the same degradation
/// as an absent virtio-mmio device. Single-hart boot-path code (see `BAR_NEXT`); the store order
/// (cursor and limit before the base that stands for "present") is documentation, not
/// synchronization.
fn is_host_bridge_present() -> bool {
    if ECAM_BASE.load(Ordering::Relaxed) != 0 {
        return true;
    }
    let Some(((ecam, _), (bar, bar_size))) = crate::memory::pci_regions() else {
        return false;
    };
    BAR_NEXT.store(bar, Ordering::Relaxed);
    BAR_LIMIT.store(bar + PCI_BAR_MAPPED.min(bar_size), Ordering::Relaxed);
    ECAM_BASE.store(ecam, Ordering::Relaxed);
    true
}

/// Place every unassigned BAR of `bdf` at a size-aligned address drawn from the shared cursor,
/// writing the config-space BAR registers. Returns false (after saying so) if the window is
/// exhausted. A BAR that already carries an address **inside the window `mmu::map_everything`
/// actually mapped** is left alone; one that carries an address the machine chose and that is not
/// memory is **adopted** where it stands ([`adopt`]); only a BAR that is unassigned, or assigned
/// somewhere this kernel cannot honour, is moved.
///
/// **The adoption arm is milestone 256's, and it exists because relocating a real machine's whole
/// bus is the wrong answer rather than a slow one.** QEMU hands this kernel BARs that are
/// unassigned, so for a year every BAR it met wanted placing and the code that placed them was
/// never wrong. Real firmware hands over BARs it has already put somewhere, having read the host
/// bridge's `_CRS` that this kernel cannot: on xenon's second boot the census read **13 of 15
/// functions** carrying a BAR outside the kernel's window, and moving thirteen functions into a
/// 2 MiB window would have exhausted it after the first one that wanted a megabyte. Firmware's
/// placement is better evidence about where the bus decodes than anything this kernel can derive,
/// so where it is honourable it is honoured.
///
/// **The "leave a nonzero BAR alone" half of that rule is not enough on its own**, found here
/// 2026-08-25 (decisions §86's VT-d/NVMe data point). The comment this replaced assumed a nonzero
/// BAR means "firmware (or a previous boot stage) already placed it," and therefore already
/// mapped: true on the two device-tree architectures, where nothing runs before this kernel to
/// place one. It is false on `x86_64`'s PVH boot: nothing here runs any firmware either, yet
/// QEMU's `-device nvme`, attached directly to the root complex, resets with a live, working BAR0
/// already assigned (`0xfebd4000` on this boot) that has no relationship to the kernel's own
/// mapped window. Trusting it produced a controller whose registers page
/// fault on first touch, because nothing in `mmu::map_everything` maps wherever QEMU chose. The
/// same gap would bite a real UEFI machine too (milestone 87): firmware there also picks its own
/// addresses, unrelated to this kernel's hardcoded window, so "nonzero" was never sufficient
/// evidence of "mapped" on this architecture. Checking against the window this kernel actually
/// mapped, rather than against zero, is correct in both cases and a no-op on the other two
/// architectures (their BARs still always arrive at 0).
fn place_bars(bdf: Bdf, bars: &mut [Option<Bar>; 6]) -> bool {
    // The window `mmu::map_everything` actually mapped: `memory::pci_regions()`'s mem32 half for
    // the lower bound (immutable once `is_host_bridge_present` caches it), `BAR_LIMIT` for the
    // upper (the same `PCI_BAR_MAPPED.min(bar_size)` clamp that call computed, so this always
    // agrees with what is really mapped even on the two architectures whose device tree describes a
    // PCIe memory window larger than `PCI_BAR_MAPPED`).
    let mapped_lo = crate::memory::pci_regions().map(|(_, (base, _))| base);
    let mapped_hi = BAR_LIMIT.load(Ordering::Relaxed);

    for (i, bar) in bars.iter_mut().enumerate() {
        let Some(bar) = bar.as_mut() else { continue };
        if bar.base != 0
            && let Some(lo) = mapped_lo
            && bar.base >= lo
            && bar.base + bar.size <= mapped_hi
        {
            continue; // already placed, and inside memory this kernel actually mapped
        }
        // **Adopt what the machine placed, when the machine placed it somewhere that is not
        // memory** (milestone 256). Mapping it is what makes adoption real: the fine map was built
        // before this ran and covers the kernel's own window, not wherever firmware put this one.
        if bar.base != 0 && adopt(bdf, i, bar) {
            continue;
        }
        // A BAR's address must be aligned to its size (the writable-bits mask encodes that). Reserve
        // size-aligned space from the shared cursor.
        let align = bar.size.max(0x10);
        let base = loop {
            let cur = BAR_NEXT.load(Ordering::Relaxed);
            let base = cur.next_multiple_of(align);
            if base + bar.size > BAR_LIMIT.load(Ordering::Relaxed) {
                crate::println!("  pci: BAR window exhausted; cannot place BAR{i}");
                return false;
            }
            if BAR_NEXT
                .compare_exchange(cur, base + bar.size, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                break base;
            }
        };
        let off = pci::BAR0 + i as u64 * 4;
        cfg_write32(bdf, off, base as u32 | if bar.is_64 { 0b100 } else { 0 });
        if bar.is_64 {
            cfg_write32(bdf, off + 4, (base >> 32) as u32);
        }
        bar.base = base;
    }
    true
}

/// **Does `lo..hi` overlap any region the machine called usable RAM?**
///
/// The one safety question adoption turns on, and deliberately the weakest one that is sufficient:
/// a device window this kernel maps over RAM is a device answering at addresses the frame
/// allocator hands out, which is the corruption milestone 256 exists to prevent. Whether the
/// address *also* decodes to the bus is the machine's claim to make and it has already made it by
/// writing the BAR.
fn overlaps_ram(lo: u64, hi: u64) -> bool {
    crate::memory::ram_regions().any(|(start, size)| lo < start + size && hi > start)
}

/// **Take a BAR the machine already placed, and map it where it stands** (milestone 256). True
/// when it was adopted, false when the caller must place it instead.
///
/// Refused, and the BAR moved instead, when the address the machine chose overlaps RAM: that is
/// not a window firmware meant this kernel to drive through, whatever wrote it, and mapping device
/// registers over memory the allocator owns is the failure this whole milestone is about. It is
/// also refused when a page of it cannot be mapped for any reason other than being mapped
/// already, since a half-mapped BAR faults on first touch and is worse than a moved one.
///
/// **`AlreadyMapped` is success here and is not treated as success anywhere else in this tree.**
/// Two BARs of the same function routinely share a page, and a BAR may sit inside the window
/// `mmu::map_everything` mapped up front; both are the same page mapped device-typed twice with
/// the same flags, which is the one shape where the second mapping asks for nothing new.
///
/// On the two device-tree architectures this is unreachable: every BAR there arrives zero, so the
/// caller never asks.
fn adopt(bdf: Bdf, index: usize, bar: &pci::Bar) -> bool {
    let lo = bar.base & !(PAGE_SIZE - 1);
    let hi = (bar.base + bar.size).next_multiple_of(PAGE_SIZE);
    if overlaps_ram(lo, hi) {
        crate::println!(
            "  pci: {:02x}:{:02x}.{} BAR{index} at {:#x} overlaps RAM; moving it",
            bdf.bus,
            bdf.dev,
            bdf.func,
            bar.base,
        );
        return false;
    }
    let mut pa = lo;
    while pa < hi {
        match mmu::map_page(mmu::phys_to_virt(pa), pa, paging::Flags::device()) {
            Ok(()) | Err(paging::MapError::AlreadyMapped) => {}
            Err(e) => {
                crate::println!(
                    "  pci: {:02x}:{:02x}.{} BAR{index} at {:#x} could not be mapped ({e:?}); \
                     moving it",
                    bdf.bus,
                    bdf.dev,
                    bdf.func,
                    bar.base,
                );
                return false;
            }
        }
        pa += PAGE_SIZE;
    }
    true
}

/// Callers sit behind [`is_host_bridge_present`], which is what makes the cached base nonzero and
/// the window below it device-mapped.
fn cfg_read32(bdf: Bdf, off: u64) -> u32 {
    let va = mmu::phys_to_virt(ECAM_BASE.load(Ordering::Relaxed) + bdf.ecam_offset() + (off & !3));
    // SAFETY: the ECAM window for the buses we enumerate is device-mapped (mmu::map_everything's
    // PCIe step maps it whenever the tree described one), and ECAM config reads are
    // side-effect-free.
    unsafe { core::ptr::read_volatile(va as *const u32) }
}

/// **A bridge's `(secondary, subordinate)` bus numbers**, or `None` when `bus:dev.func` is absent
/// or is not a bridge. The live-bus half of resolving a DMAR device-scope path, which is firmware
/// naming a device by the route to it rather than by its number (milestone 261 (the NVMe driver leaves the kernel)'s bench rehearsal;
/// `machine_discovery::acpi::DmarUnits::owner`).
#[cfg_attr(not(target_arch = "x86_64"), allow(dead_code))]
pub fn bridge_bus_range(bus: u8, dev: u8, func: u8) -> Option<(u8, u8)> {
    if !is_host_bridge_present() || dev > 31 || func > 7 {
        return None;
    }
    let bdf = Bdf { bus, dev, func };
    if cfg_read32(bdf, pci::VENDOR_ID) & 0xffff == 0xffff {
        return None;
    }
    let header = (cfg_read32(bdf, pci::HEADER_TYPE & !3) >> ((pci::HEADER_TYPE & 3) * 8)) as u8;
    if header & 0x7f != pci::HEADER_TYPE_BRIDGE {
        return None;
    }
    let b = pci::bridge_buses(bdf, &mut |b, o| cfg_read32(b, o));
    Some((b.secondary, b.subordinate))
}

fn cfg_write32(bdf: Bdf, off: u64, v: u32) {
    let va = mmu::phys_to_virt(ECAM_BASE.load(Ordering::Relaxed) + bdf.ecam_offset() + (off & !3));
    // SAFETY: as above; config writes go to the one function this bdf names.
    unsafe { core::ptr::write_volatile(va as *mut u32, v) }
}

/// A modern virtio device on the PCI bus, brought up: every register block resolved to a
/// physical address, memory decoding and bus mastering enabled, its INTx line known. Device-neutral
/// (a disk and a NIC differ only in the driver above this), so both `find_block_device` and
/// `find_net_device` return it.
#[derive(Debug, Clone, Copy)]
pub struct PciVirtioDevice {
    /// Which function this is, for the boot tours' prints; every other build (tests, the shell
    /// and bench boots) drives the device without ever naming it, so the lint is quieted rather
    /// than chased through four cfg combinations.
    #[allow(dead_code)]
    pub bdf: Bdf,
    /// The virtio common-config block (queue setup, status, features), physical.
    pub common: u64,
    /// The notify region base and the per-queue multiplier; queue N's doorbell is at
    /// `notify_base + queue_notify_off(N) * notify_mult`.
    pub notify_base: u64,
    pub notify_mult: u32,
    /// The ISR byte (read-to-ack), physical.
    pub isr: u64,
    /// **The interrupt a driver binds and waits on**, whichever way this machine delivers it: the
    /// PLIC/GIC input the INTx pin swizzles to (see `pci::intx_irq`) on the two device-tree
    /// boards, and the MSI-X vector `arch::irq::alloc_msi_vector` reserved on `x86_64`. Which of the
    /// two happened is [`msix_vector`](Self::msix_vector), and nothing above this module needs to
    /// know: an `Irq` capability names an intid and the arch layer resolves it.
    pub intid: u32,
    /// **Which MSI-X table entry [`intid`](Self::intid) was programmed into**, or
    /// `pci::VIRTIO_MSIX_NO_VECTOR` when this function delivers INTx instead (both `virt` boards).
    ///
    /// It has to travel with the transport rather than being consumed here, because a virtio queue
    /// is told which vector to raise **at queue-setup time, with that queue selected**, and the
    /// queue is set up later, by `virtio::setup_queue`, on the driver's request. A device reset
    /// (which every driver does on the way in) puts the field back to `NO_VECTOR`, so it cannot be
    /// programmed once here and left.
    pub msix_vector: u16,
    /// The PCIe requester id (`bus:8 | dev:5 | fn:3`), the id the IOMMU keys its per-device tables
    /// on (milestone 16b). Carried through to `virtio::register` so the device is confined to its
    /// DMA region in hardware.
    pub rid: u32,
    /// The **virtio device type** (2 = block, 1 = net, 16 = gpu), recovered from the PCI device id
    /// (`0x1040 + type`). PCI config space has no `DeviceID` register in the virtio-mmio sense, so
    /// the transport answers a driver's `DeviceID` read from this instead of a hardcoded value; a
    /// GPU driver that sanity-checks what it is talking to then gets the truth on either bus.
    pub device_type: u32,
}

/// Find the first modern virtio-blk function on the bus and bring it up. `None` if there is no
/// PCI disk (an empty bus reads all-ones and enumerates nothing).
pub fn find_block_device() -> Option<PciVirtioDevice> {
    let bdf = find_virtio_bdf(
        pci::VIRTIO_BLK_MODERN,
        Some(pci::VIRTIO_BLK_TRANSITIONAL),
        "virtio-blk",
    )?;
    bring_up(bdf, pci::VIRTIO_TYPE_BLOCK)
}

/// **Find the `n`-th (0-based) modern virtio-blk function on the bus and bring it up**, in
/// bus:device.function order. `None` if the bus has no such function.
///
/// The PCI half of [`crate::virtio::find_block_device_n`]'s ordering, and the reason it exists is
/// `q35`: that machine has no virtio-mmio bus, so every block device a wiring can ask for on `x86_64`
/// is a PCI function and "the second disk" has to be answerable here (milestone 303). On both
/// `virt` boards the mmio half answers every ordinal any wiring asks for and this is never reached.
///
/// [`find_block_device`] is `n = 0` and is kept rather than folded into this, because the PCIe
/// transport tests (`virtio_service::start_role_pci`) want "the PCI disk" rather than an ordinal
/// into an ordering that spans a bus they are deliberately not on.
#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))] // the mmio half answers first on both virt boards
pub fn find_block_device_n(n: usize) -> Option<PciVirtioDevice> {
    if !is_host_bridge_present() {
        return None;
    }
    let mut seen = 0;
    let mut found: Option<Bdf> = None;
    pci::enumerate(
        ecam_buses(),
        &mut |b, o| cfg_read32(b, o),
        &mut |bdf, vendor, device| {
            if found.is_none() && vendor == pci::VIRTIO_VENDOR && device == pci::VIRTIO_BLK_MODERN {
                if seen == n {
                    found = Some(bdf);
                }
                seen += 1;
            }
        },
    );
    bring_up(found?, pci::VIRTIO_TYPE_BLOCK)
}

/// Find the first modern virtio-net function on the bus and bring it up. `None` if there is no
/// PCI NIC. Same bring-up as the disk; the transport seam and the DMA confinement do not know or
/// care that a NIC sits behind them (milestone 30).
pub fn find_net_device() -> Option<PciVirtioDevice> {
    let bdf = find_virtio_bdf(
        pci::VIRTIO_NET_MODERN,
        Some(pci::VIRTIO_NET_TRANSITIONAL),
        "virtio-net",
    )?;
    bring_up(bdf, pci::VIRTIO_TYPE_NET)
}

/// Find the first modern virtio-gpu function on the bus and bring it up (milestone 29, the display
/// ladder's first rung). `None` if there is no PCI GPU.
///
/// **PCIe only, on purpose and on both boards.** A virtio-gpu is not on either `virt` machine's
/// virtio-mmio bus in any configuration we attach, so unlike the disk and the NIC there is no mmio
/// twin to prove parity over; the parity that matters here is aarch64 `virt` and riscv `virt`, both
/// of which carry `virtio-gpu-pci` over the §18 transport. Same bring-up as the disk, and the same
/// hardware confinement: the device is entered behind the IOMMU at `virtio::register`.
pub fn find_gpu_device() -> Option<PciVirtioDevice> {
    // No transitional twin exists for virtio-gpu (the legacy id range predates the device), so this
    // asks for the modern id and nothing else rather than warning about a legacy device that cannot
    // be on the bus.
    let bdf = find_virtio_bdf(pci::VIRTIO_GPU_MODERN, None, "virtio-gpu")?;
    bring_up(bdf, pci::VIRTIO_TYPE_GPU)
}

/// Find the first modern virtio-input function on the bus and bring it up (milestone 29's keyboard).
/// `None` if there is no PCI input device.
///
/// PCIe only, and here that **is** a choice rather than a constraint: unlike the GPU, both `virt`
/// machines do offer a `virtio-keyboard-device` on the virtio-mmio bus. The keyboard rides PCIe
/// anyway so it sits behind the same IOMMU domain the GPU does and the display's two devices are
/// confined the same way. A keyboard is the one device whose DMA a user would least like
/// unconfined: its buffers are where every keystroke lands.
pub fn find_input_device() -> Option<PciVirtioDevice> {
    let bdf = find_virtio_bdf(pci::VIRTIO_INPUT_MODERN, None, "virtio-input")?;
    bring_up(bdf, pci::VIRTIO_TYPE_INPUT)
}

/// Find the first modern virtio-rng function on the bus and bring it up (milestone 56's entropy
/// source). `None` if there is no PCI RNG.
///
/// The mmio twin is [`crate::virtio::find_entropy_device`], and both exist because both machines
/// offer both: the entropy service is wired over whichever one the wiring asked for, and the
/// milestone-56 test runs it over each in turn. An RNG behind the IOMMU is worth having for the
/// same reason a keyboard is: its buffer is the one place in memory whose contents must not be
/// guessable, and an unconfined device could write it anywhere and read the rest.
#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))] // entropy_service is the caller, and the m56 tests drive it
pub fn find_rng_device() -> Option<PciVirtioDevice> {
    let bdf = find_virtio_bdf(
        pci::VIRTIO_RNG_MODERN,
        Some(pci::VIRTIO_RNG_TRANSITIONAL),
        "virtio-rng",
    )?;
    bring_up(bdf, pci::VIRTIO_TYPE_ENTROPY)
}

/// How many modern virtio-blk functions are on the bus (milestone 57's roster).
///
/// **Counting is not bringing up.** [`find_block_device`] resolves a transport, which means sizing
/// and assigning BARs and enabling memory decoding: side effects a *listing* has no business
/// causing, and ones that would disturb whichever driver already owns the function. This reads
/// config space and nothing else, which is all a roster is entitled to know
/// (`crates/block_roster`).
pub fn count_block_devices() -> usize {
    if !is_host_bridge_present() {
        return 0;
    }
    let mut n = 0;
    pci::enumerate(
        ecam_buses(),
        &mut |b, o| cfg_read32(b, o),
        &mut |_, vendor, device| {
            if vendor == pci::VIRTIO_VENDOR && device == pci::VIRTIO_BLK_MODERN {
                n += 1;
            }
        },
    );
    n
}

/// **A read-only census of what the machine left on the bus**, provisional name (milestone 165).
///
/// Returns `(functions, stranded)`: how many functions answer at all, and how many carry at least
/// one memory BAR this kernel can **neither use where it is nor adopt** (milestone 256), which is
/// the pair of numbers that says whether the BAR window this port derived is adequate on the
/// machine it is running on.
///
/// **The second number changed meaning on 2026-09-04 and got smaller for a good reason.** It used
/// to count every BAR outside the kernel's own window, which was the right question while
/// [`place_bars`] moved all of them: each one was a relocation, and a relocation onto a hardcoded
/// window nobody had checked against this machine was the risk being measured. Since that window
/// is derived from the machine and a BAR the machine placed outside it is adopted rather than
/// moved, "outside the window" stopped being a problem and "outside the window **and** on top of
/// RAM" became the whole of it. That is the count here, and **zero is the passing answer**: it
/// means every function on the bus is reachable exactly where the machine left it or in a window
/// the machine agreed to.
///
/// **Why this is worth a boot line rather than a comment.** [`place_bars`] leaves a nonzero BAR
/// alone only when it already lies inside that window, and **moves it** otherwise. That arm is
/// exercised and correct (its own doc records the `-device nvme` case that found it), so the open
/// question is not whether the move works but **how much of the machine it moves and whether the
/// place it moves things to is free.** That window was a hardcoded `0xc000_0000` until milestone
/// 256, checked once against QEMU's `info mtree` and against nothing else, because the window a
/// real machine wants BARs in is in its host bridge's `_CRS` and `_CRS` is AML.
///
/// **What the old number measured, kept because it is the evidence for the change.** Under the
/// relocate-everything rule it read 5 of 8 under PVH and 3 of 6 under OVMF on the bare tour
/// machine (2026-09-02), and on xenon's second boot, on real firmware, **13 of 15**. That last one
/// is the measurement milestone 256 was minted from: thirteen functions of fifteen were about to
/// be moved out of windows the machine's own firmware had chosen, into a 2 MiB hardcoded window
/// that was RAM on that machine.
///
/// Counting only, like [`count_block_devices`] and for the same reason: no sizing writes, no
/// command-register changes, nothing a function's existing owner could notice.
// The x86 boot tour is the only caller, for the reason its own comment gives: on both `virt`
// boards every BAR arrives zero and there would be nothing to report. The function itself is
// arch-neutral and stays here rather than in `arch/`, so this allow is the cost of that.
#[cfg_attr(not(target_arch = "x86_64"), allow(dead_code))]
pub fn bar_census() -> (usize, usize) {
    if !is_host_bridge_present() {
        return (0, 0);
    }
    let lo = crate::memory::pci_regions().map_or(0, |(_, (base, _))| base);
    let hi = BAR_LIMIT.load(Ordering::Relaxed);
    let (mut functions, mut stranded) = (0usize, 0usize);
    pci::enumerate(
        ecam_buses(),
        &mut |b, o| cfg_read32(b, o),
        &mut |bdf, _, _| {
            functions += 1;
            // The raw BAR registers, not `pci::read_bars`, which sizes them by writing all-ones:
            // this runs on every function on the bus, including ones nothing will ever drive, and
            // a census that wrote to them would not be a census. Counting only, like
            // `count_block_devices` and for the same reason.
            //
            // A BAR is stranded when it is nonzero, is a memory BAR (bit 0 clear), sits outside
            // the window this kernel places into, and is on top of RAM, which is what stops
            // `place_bars` adopting it. Its length is unknown without sizing, so this asks about
            // its first page only; a BAR that starts clear of RAM and runs into it is
            // `place_bars`' problem and is refused there, on a span it has sized.
            let stuck = (0..6).any(|i| {
                let raw = cfg_read32(bdf, pci::BAR0 + i * 4);
                let base = u64::from(raw & !0xf);
                raw & 1 == 0
                    && base != 0
                    && !(lo..hi).contains(&base)
                    && overlaps_ram(base, base + PAGE_SIZE)
            });
            if stuck {
                stranded += 1;
            }
        },
    );
    (functions, stranded)
}

/// Enumerate the bus for the first function matching `modern`, warning (once) if only a
/// `transitional` (legacy) twin is present, since we drive modern only. `kind` names the device for
/// that warning. `transitional` is `None` for a device type that has no legacy id at all
/// (virtio-gpu): there is then no such warning to give, and inventing an id to compare against
/// would be a fact nobody checked.
fn find_virtio_bdf(modern: u16, transitional: Option<u16>, kind: &str) -> Option<Bdf> {
    if !is_host_bridge_present() {
        return None;
    }
    let mut found: Option<Bdf> = None;
    pci::enumerate(
        ecam_buses(),
        &mut |b, o| cfg_read32(b, o),
        &mut |bdf, vendor, device| {
            if found.is_none() && vendor == pci::VIRTIO_VENDOR {
                if device == modern {
                    found = Some(bdf);
                } else if Some(device) == transitional {
                    crate::println!(
                        "  pci: {kind} at {:02x}:{:02x}.{} is transitional (legacy); \
                         we drive modern only",
                        bdf.bus,
                        bdf.dev,
                        bdf.func,
                    );
                }
            }
        },
    );
    found
}

/// Bring an enumerated virtio function up and resolve its transport. Bring-up order matters and is
/// deliberate:
/// 1. size BARs while memory decoding is off (the sizing dance writes the BARs);
/// 2. assign addresses to unassigned BARs (with `-bios default`, OpenSBI has done no PCI setup,
///    so every BAR arrives zero and the kernel is the firmware here);
/// 3. parse the virtio vendor capabilities and resolve them against the assigned BARs;
/// 4. only then set Memory-Space Enable, and Bus-Master last: DMA permission is granted at the
///    final moment, after the transport the confinement layer owns is fully described.
fn bring_up(bdf: Bdf, device_type: u32) -> Option<PciVirtioDevice> {
    // Size every BAR, then place the unassigned ones from the shared cursor (the IOMMU function
    // draws from the same cursor on riscv, so the two cannot overlap).
    let mut bars = pci::read_bars(bdf, &mut |b, o| cfg_read32(b, o), &mut |b, o, v| {
        cfg_write32(b, o, v);
    });
    if !place_bars(bdf, &mut bars) {
        return None;
    }

    // The virtio vendor capabilities name (bar, offset) pairs; resolve them to physical
    // addresses against the now-assigned BARs.
    let resolve = |bars: &[Option<Bar>; 6], cap: &VirtioCap| -> Option<u64> {
        let bar = bars.get(cap.bar as usize)?.as_ref()?;
        (u64::from(cap.offset) + u64::from(cap.length) <= bar.size)
            .then(|| bar.base + u64::from(cap.offset))
    };
    let (mut common, mut notify_base, mut notify_mult, mut isr) = (None, None, 0u32, None);
    pci::virtio_caps(
        bdf,
        &mut |b, o| cfg_read32(b, o),
        &mut |cap| match cap.cfg_type {
            pci::VIRTIO_CAP_COMMON if common.is_none() => common = resolve(&bars, &cap),
            pci::VIRTIO_CAP_NOTIFY if notify_base.is_none() => {
                notify_base = resolve(&bars, &cap);
                notify_mult = cap.notify_off_multiplier;
            }
            pci::VIRTIO_CAP_ISR if isr.is_none() => isr = resolve(&bars, &cap),
            _ => {}
        },
    );
    let (common, notify_base, isr) = (common?, notify_base?, isr?);

    // Memory-Space Enable so the BARs decode; Bus-Master Enable so the device may DMA at all.
    // The upper half of this dword is the STATUS register, whose error bits are write-1-to-clear,
    // so it is written as zero: clearing nothing, changing nothing.
    let cmd = cfg_read32(bdf, pci::COMMAND) as u16;
    cfg_write32(
        bdf,
        pci::COMMAND,
        (cmd | pci::CMD_MEMORY_SPACE | pci::CMD_BUS_MASTER) as u32,
    );

    let (intid, msix_vector) = function_interrupt(bdf, &bars, false)?;

    Some(PciVirtioDevice {
        bdf,
        common,
        notify_base,
        notify_mult,
        isr,
        intid,
        msix_vector,
        rid: bdf.requester_id(),
        device_type,
    })
}

/// **How this function's interrupt reaches a driver**: `(intid, msix_vector)`, and the machine
/// decides rather than this module. `arch::irq::alloc_msi_vector` answers `Some` only where the
/// machine wants message-signalled interrupts; both `virt` boards answer `None` and fall through to
/// the INTx swizzle they have always used. See milestone 215 (a PCI function's interrupt on
/// x86_64) for why x86_64 differs.
///
/// `msi_fallback` lets a function with **MSI and no MSI-X** take plain MSI (milestone 242 (USB
/// host and HID)): the xHCI on an Intel PCH, xenon's USB controller, is one. The virtio functions
/// pass `false`, because every one of them has MSI-X and a virtio queue names its vector by MSI-X
/// table index, which MSI has no equivalent of. `msix_vector` is
/// `pci::VIRTIO_MSIX_NO_VECTOR` whenever the answer was not MSI-X.
fn function_interrupt(bdf: Bdf, bars: &[Option<Bar>; 6], msi_fallback: bool) -> Option<(u32, u16)> {
    match crate::arch::irq::alloc_msi_vector() {
        Some((intid, target)) => {
            // **A refusal rather than a fallback to INTx**, and the reason is the bug milestone
            // 215 exists to fix. A machine that answered `Some` has no other way to deliver a PCI
            // interrupt, so falling back to `intx_intid` here would compute
            // `intx_irq(PCI_IRQ_BASE, ..)` with a base that is 0 and admits it, hand back an
            // intid that resolves to the PIT's line, and produce a driver armed on the timer with
            // nothing anywhere saying so. That is precisely what milestone 164's lane measured.
            if msi_fallback
                && pci::msix_cap(bdf, &mut |b, o| cfg_read32(b, o)).is_none()
                && program_msi(bdf, target)
            {
                return Some((intid, pci::VIRTIO_MSIX_NO_VECTOR));
            }
            let vector = program_msix(bdf, bars, target)?;
            Some((intid, vector))
        }
        None => Some((intx_intid(bdf)?, pci::VIRTIO_MSIX_NO_VECTOR)),
    }
}

/// **Point this function's one MSI vector at `target` and enable it** (milestone 242 (USB host and
/// HID)). `false`, having touched nothing, when the function has no MSI capability.
///
/// Address and data first, enable last, so no half-written message is ever live; one vector
/// (Multiple Message Enable left 0), because one is what every driver here asks for.
fn program_msi(bdf: Bdf, target: pci::MsiTarget) -> bool {
    let Some(cap) = pci::msi_cap(bdf, &mut |b, o| cfg_read32(b, o)) else {
        return false;
    };
    cfg_write32(bdf, cap.address_low(), target.address as u32);
    if cap.is_64 {
        cfg_write32(bdf, cap.address_low() + 4, (target.address >> 32) as u32);
    }
    // The data register is 16 bits, and the dword holding it has the (64-bit) layout's reserved
    // half or the next register above it; a read-modify-write keeps whichever it is.
    let old = cfg_read32(bdf, cap.data());
    cfg_write32(
        bdf,
        cap.data(),
        (old & 0xffff_0000) | (target.data & 0xffff),
    );
    let head = cfg_read32(bdf, cap.cap_offset);
    let control = ((head >> 16) as u16 & !(0x7 << 4)) | pci::MSI_ENABLE;
    cfg_write32(
        bdf,
        cap.cap_offset,
        (head & 0xffff) | (control as u32) << 16,
    );
    true
}

/// The INTx path: read the function's interrupt pin and swizzle it onto this board's controller.
///
/// `None`, loudly, for a function that declares no pin at all, because on a board that routes INTx
/// a device with no pin has no way to interrupt anybody and a driver bound to whatever
/// `intx_irq(base, dev, 0)` returned would wait forever on a line nothing drives.
fn intx_intid(bdf: Bdf) -> Option<u32> {
    // config 0x3d; 1=INTA..4=INTD, 0=none. The dtb fixture test holds the swizzle against the
    // machine's own interrupt-map.
    let pin = ((cfg_read32(bdf, 0x3c) >> 8) & 0xff) as u8;
    if pin == 0 {
        crate::println!(
            "  pci: the virtio function at {:02x} declares no INTx pin",
            bdf.dev
        );
        return None;
    }
    Some(pci::intx_irq(PCI_IRQ_BASE, bdf.dev, pin))
}

/// **Point one of this function's MSI-X vectors at `target` and arm it** (milestone 215). Returns
/// the table index the device must be told to raise, or `None` (loudly) when the function has no
/// usable MSI-X table.
///
/// # Why this and not the legacy pin, on the machine that needed a choice
///
/// `x86_64` had no working answer at all: `arch::mmu::PCI_IRQ_BASE` was `0` and said so, because a
/// function's INTx pin goes through a PIRQ router that only ACPI's `_PRT` describes, and `_PRT` is
/// AML. The two ways out were an AML interpreter (a project) or a hardcoded swizzle that matches
/// QEMU's `q35` and may not match a Dell `OptiPlex`, which is the failure milestone 87 would have to
/// discover at a null modem. **MSI-X has no board-specific routing to be wrong about**: the device
/// is handed the address and the value, so the only thing that has to be right is the interrupt
/// controller's own message format, which this kernel already programs directly. See milestone 215
/// for the full refusal.
///
/// # What is written, in what order
///
/// The entry is written **before** MSI-X Enable is set, and the per-entry mask is cleared last of
/// the four words, so no configuration is ever live half-written. The Function Mask bit is cleared
/// explicitly rather than assumed clear, because it is set at reset on some parts and a
/// function-masked device is one that never interrupts with nothing anywhere saying why.
///
/// **The entry lives in a BAR, not in config space**, which is why this takes the placed `bars`:
/// the table's BIR names one of them, and until `place_bars` has run there is no address to write
/// through. A table that names an unplaced or out-of-range BAR is refused rather than written to a
/// guessed address.
fn program_msix(bdf: Bdf, bars: &[Option<Bar>; 6], target: pci::MsiTarget) -> Option<u16> {
    let Some(cap) = pci::msix_cap(bdf, &mut |b, o| cfg_read32(b, o)) else {
        crate::println!(
            "  pci: the function at {:02x}:{:02x}.{} declares no MSI-X, and this machine has no \
             other route for a PCI interrupt",
            bdf.bus,
            bdf.dev,
            bdf.func,
        );
        return None;
    };

    // Entry 0. One vector per function is all any driver here asks for: every device this tree
    // attaches uses a single queue's completion, and handing out more would be table entries
    // nothing raises.
    let bar = bars.get(cap.table_bar as usize)?.as_ref()?;
    let entry = u64::from(cap.table_offset);
    if entry + pci::MSIX_ENTRY_BYTES > bar.size {
        crate::println!(
            "  pci: the MSI-X table of the function at {:02x} does not fit its own BAR{}",
            bdf.dev,
            cap.table_bar,
        );
        return None;
    }
    let at = mmu::phys_to_virt(bar.base + entry);

    // SAFETY: `at` is inside BAR `cap.table_bar`, which `place_bars` put inside the window
    // `mmu::map_everything` maps as device memory, and the bounds check above keeps all sixteen
    // bytes inside it. The four words are the specified layout of one MSI-X table entry.
    unsafe {
        core::ptr::write_volatile(at as *mut u32, target.address as u32);
        core::ptr::write_volatile((at + 4) as *mut u32, (target.address >> 32) as u32);
        core::ptr::write_volatile((at + 8) as *mut u32, target.data);
        core::ptr::write_volatile((at + 12) as *mut u32, 0); // unmasked, last
    }

    // Message Control is the upper half of the capability's first dword, so it is written back
    // with the (read-only) id and next-pointer bytes underneath it, which is what a 32-bit config
    // write to a capability header means.
    let head = cfg_read32(bdf, cap.cap_offset);
    let control = ((head >> 16) as u16 | pci::MSIX_ENABLE) & !pci::MSIX_FUNCTION_MASK;
    cfg_write32(
        bdf,
        cap.cap_offset,
        (head & 0xffff) | ((control as u32) << 16),
    );

    Some(0)
}

/// An enumerated, brought-up NVMe controller: its register file (BAR0) placed and decoding, bus
/// mastering enabled, requester id known so the caller can confine its DMA before enabling the
/// controller. No INTx line, on purpose: the milestone-53 driver completes by polling the phase
/// tag (`kernel/src/non_volatile_memory_express.rs`), so wiring an interrupt here would record a fact nothing checks.
#[derive(Debug, Clone, Copy)]
pub struct PciNvmeDevice {
    /// The register file's physical base (BAR0; the doorbells live in it too).
    pub bar0: u64,
    /// The PCIe requester id, the key the IOMMU confines DMA by (as `PciVirtioDevice::rid`).
    pub rid: u32,
}

/// Find the first NVMe controller on the bus and bring its transport up: size and place its BARs,
/// resolve BAR0, and enable memory decoding and bus mastering. `None` if no function on the bus
/// carries the NVMe class code.
///
/// Same bring-up order as [`bring_up`], for the same reasons; what it does not share is the virtio
/// capability walk (NVMe's register layout is fixed by its own spec, not described by vendor
/// capabilities) and the INTx resolution (the driver polls). Bus-Master last, as everywhere: DMA
/// permission is granted at the final moment, and on a machine with an IOMMU the device still
/// cannot reach a byte until `iommu::confine` maps its region (denied by default, milestone 16b).
pub fn find_nvme_device() -> Option<PciNvmeDevice> {
    if !is_host_bridge_present() {
        return None;
    }
    let mut found: Option<Bdf> = None;
    pci::enumerate(
        ecam_buses(),
        &mut |b, o| cfg_read32(b, o),
        &mut |bdf, _, _| {
            if found.is_none() && cfg_read32(bdf, pci::CLASS_REVISION) >> 8 == pci::CLASS_NVME {
                found = Some(bdf);
            }
        },
    );
    let bdf = found?;

    let mut bars = pci::read_bars(bdf, &mut |b, o| cfg_read32(b, o), &mut |b, o, v| {
        cfg_write32(b, o, v);
    });
    if !place_bars(bdf, &mut bars) {
        return None;
    }
    let bar0 = bars[0].as_ref().map(|b| b.base)?;

    let cmd = cfg_read32(bdf, pci::COMMAND) as u16;
    cfg_write32(
        bdf,
        pci::COMMAND,
        (cmd | pci::CMD_MEMORY_SPACE | pci::CMD_BUS_MASTER) as u32,
    );

    Some(PciNvmeDevice {
        bar0,
        rid: bdf.requester_id(),
    })
}

/// **An enumerated, brought-up xHCI controller** (milestone 242 (USB host and HID)): its register
/// file placed and decoding, bus mastering on, its interrupt resolved, and the byte ranges of BAR0
/// its MSI-X structures occupy, which the driver must never be mapped.
#[derive(Debug, Clone, Copy)]
pub struct PciXhciDevice {
    /// The register file's physical base (BAR0).
    pub bar0: u64,
    /// BAR0's size in bytes.
    pub bar_bytes: u64,
    /// The PCIe requester id the IOMMU confines DMA by.
    pub rid: u32,
    /// The interrupt a driver binds and waits on, as [`PciVirtioDevice::intid`].
    pub intid: u32,
    /// The MSI-X table and pending-bit array, as byte ranges into BAR0, `(0, 0)` where one is
    /// absent or lives in another BAR. Withheld from the driver's mapping.
    pub withheld: [(u64, u64); 2],
}

/// **Find the first xHCI controller on the bus and bring its transport up**, the
/// [`find_nvme_device`] shape plus an interrupt: size and place the BARs, enable memory decoding and
/// bus mastering, resolve how its interrupt arrives. `None` if no function carries the xHCI class
/// code, or one does and has no BAR0 or no interrupt this machine can route.
///
/// Bus-Master before confinement, as [`find_nvme_device`] does and for its reason: on a machine
/// with an IOMMU the controller cannot reach a byte until `iommu::confine` maps its region.
pub fn find_xhci_device() -> Option<PciXhciDevice> {
    if !is_host_bridge_present() {
        return None;
    }
    let mut found: Option<Bdf> = None;
    pci::enumerate(
        ecam_buses(),
        &mut |b, o| cfg_read32(b, o),
        &mut |bdf, _, _| {
            if found.is_none() && cfg_read32(bdf, pci::CLASS_REVISION) >> 8 == pci::CLASS_XHCI {
                found = Some(bdf);
            }
        },
    );
    let bdf = found?;

    let mut bars = pci::read_bars(bdf, &mut |b, o| cfg_read32(b, o), &mut |b, o, v| {
        cfg_write32(b, o, v);
    });
    if !place_bars(bdf, &mut bars) {
        return None;
    }
    let bar = bars[0].as_ref()?;
    let (bar0, bar_bytes) = (bar.base, bar.size);

    let cmd = cfg_read32(bdf, pci::COMMAND) as u16;
    cfg_write32(
        bdf,
        pci::COMMAND,
        (cmd | pci::CMD_MEMORY_SPACE | pci::CMD_BUS_MASTER) as u32,
    );

    let (intid, _) = function_interrupt(bdf, &bars, true)?;

    // The MSI-X table's and pending-bit array's byte ranges in BAR0, whether or not this machine
    // uses MSI-X: a driver that could write the table could enable and aim it, whatever this
    // kernel chose.
    let mut withheld = [(0, 0); 2];
    if let Some(cap) = pci::msix_cap(bdf, &mut |b, o| cfg_read32(b, o)) {
        if cap.table_bar == 0 {
            let at = u64::from(cap.table_offset);
            withheld[0] = (at, at + u64::from(cap.table_size) * pci::MSIX_ENTRY_BYTES);
        }
        if cap.pba_bar == 0 {
            let at = u64::from(cap.pba_offset);
            // One bit per entry, in whole quadwords.
            withheld[1] = (at, at + u64::from(cap.table_size).div_ceil(64) * 8);
        }
    }

    Some(PciXhciDevice {
        bar0,
        bar_bytes,
        rid: bdf.requester_id(),
        intid,
        withheld,
    })
}

/// The QEMU `riscv-iommu-pci` function's PCI identity (Red Hat vendor, RISC-V IOMMU device id).
/// riscv-only: aarch64's SMMUv3 is a device-tree platform node, not a PCI function, and x86's
/// IOMMUs are discovered through ACPI, so a PCI-function IOMMU is a RISC-V shape today.
#[cfg(target_arch = "riscv64")]
const IOMMU_VENDOR: u16 = 0x1b36;
#[cfg(target_arch = "riscv64")]
const IOMMU_DEVICE: u16 = 0x0014;

/// **Bring up the RISC-V IOMMU, if the machine has one** (milestone 16b). Unlike the aarch64
/// SMMUv3 (a platform device the kernel finds in the device tree), the ratified RISC-V IOMMU is
/// itself a PCI function: the kernel enumerates it, places its BAR0 (the register file) from the
/// same shared cursor as every other BAR, enables memory-space decoding, and hands the base to the
/// arch driver's `init`. No such function on the bus (a plain `virt` boot, or aarch64), no-op.
///
/// Its own registers are reached by the CPU as MMIO, so it needs Memory-Space Enable but not
/// Bus-Master: the IOMMU is not a DMA initiator, it is the thing that polices them.
#[cfg(target_arch = "riscv64")]
pub fn init_iommu() {
    if !is_host_bridge_present() {
        return;
    }
    let mut found: Option<Bdf> = None;
    pci::enumerate(
        ecam_buses(),
        &mut |b, o| cfg_read32(b, o),
        &mut |bdf, vendor, device| {
            if found.is_none() && vendor == IOMMU_VENDOR && device == IOMMU_DEVICE {
                found = Some(bdf);
            }
        },
    );
    let Some(bdf) = found else { return };

    let mut bars = pci::read_bars(bdf, &mut |b, o| cfg_read32(b, o), &mut |b, o, v| {
        cfg_write32(b, o, v);
    });
    if !place_bars(bdf, &mut bars) {
        crate::println!("  pci: could not place the IOMMU's BAR; leaving the IOMMU off");
        return;
    }
    let Some(base) = bars[0].as_ref().map(|b| b.base) else {
        crate::println!("  pci: the IOMMU function has no BAR0; leaving the IOMMU off");
        return;
    };

    // Memory-Space Enable so BAR0 decodes and the register reads below land. No Bus-Master: the
    // IOMMU does not itself DMA.
    let cmd = cfg_read32(bdf, pci::COMMAND) as u16;
    cfg_write32(bdf, pci::COMMAND, (cmd | pci::CMD_MEMORY_SPACE) as u32);

    crate::arch::iommu::init(base);
}

#[cfg(test)]
mod tests {
    #[cfg(target_arch = "x86_64")]
    use super::*;

    /// **The discovered PCIe windows are the machine's own, and on this machine they equal the
    /// constants they replaced.** A fresh parse of the live DTB must agree with what
    /// `memory::init` recorded, which proves the boot-path read end to end; and on QEMU `virt`
    /// (the machine every merge boots) the values must be the old `PCI_ECAM_BASE` and
    /// `PCI_BAR_BASE` hardcodes, which is the whole regression claim: same machine, same
    /// windows, different provenance. The JH7110's different answer (no generic-ECAM node at
    /// all, so `pci_regions()` is None and no window is mapped) is witnessed on the host, where
    /// that tree exists (`crates/pci/tests/qemu_virt_dtb.rs`). Same shape as the PLIC-context
    /// test in arch/riscv64/irq.rs, for the same §43 reason.
    ///
    /// **Not on `x86_64`**: there is no device tree to cross-check against (`crate::DTB` stays zero
    /// on this architecture; see [`acpi_mcfg_wires_a_real_ecam_window_that_finds_the_host_bridge`]
    /// for the ACPI-sourced equivalent).
    #[cfg(not(target_arch = "x86_64"))]
    #[test_case]
    fn the_discovered_pci_windows_are_the_machines_own_and_match_the_old_constants() {
        let Some((ecam, mem32)) = crate::memory::pci_regions() else {
            // The JH7110 has no generic-ECAM node at all (comment above); milestone 145 gives
            // this the same treatment as non_volatile_memory_express.rs rather than a panic the doc comment already
            // predicted.
            crate::testing::skip!(
                "no pci-host-ecam-generic bridge in the device tree (expected on the JH7110)"
            );
        };

        let dt = crate::device_tree().expect("device tree is unreadable");
        let mut regs = [device_tree_blob::Region { start: 0, size: 0 }; 1];
        let n = dt
            .node_reg_compatible(b"pci-host-ecam-generic", &mut regs)
            .expect("the bridge's reg parses");
        assert_eq!(n, 1, "one host bridge, one register window");
        assert_eq!(
            (regs[0].start, regs[0].size),
            ecam,
            "the recorded ECAM window is not the tree's"
        );
        let ranges = dt
            .node_prop_compatible(b"pci-host-ecam-generic", b"ranges")
            .expect("the bridge's ranges parses")
            .expect("the bridge states its ranges");
        assert_eq!(
            ::pci::mem32_window(ranges),
            Some(mem32),
            "the recorded BAR window is not the tree's"
        );

        #[cfg(target_arch = "riscv64")]
        {
            assert_eq!(ecam, (0x3000_0000, 0x1000_0000), "was PCI_ECAM_BASE");
            assert_eq!(mem32, (0x4000_0000, 0x4000_0000), "was PCI_BAR_BASE");
        }
        #[cfg(target_arch = "aarch64")]
        {
            assert_eq!(ecam, (0x40_1000_0000, 0x1000_0000), "was PCI_ECAM_BASE");
            assert_eq!(mem32, (0x1000_0000, 0x2eff_0000), "was PCI_BAR_BASE");
        }
    }

    /// **`x86_64`'s version of the same claim, over ACPI instead of a device tree.** There is no
    /// `_CRS` reader here (no AML interpreter), so this cannot cross-check a second parse the way
    /// the device-tree test does; what it can prove, and does, is that the window ACPI's MCFG
    /// named actually decodes real PCI configuration space once
    /// `arch::x86_64::machine::enable_pcie_ecam` has run.
    ///
    /// The host bridge (bus 0, device 0, function 0) is the chipset's own function and is on the
    /// bus on every x86_64 machine, emulated or not, regardless of what `-device` flags
    /// `helpers/qemu-runner-x86_64.sh` does or does not pass, which is what makes this a real
    /// read rather than a hope: before
    /// `enable_pcie_ecam` runs, this exact physical address **faults** (measured against QEMU's
    /// monitor, 2026-08-24: `xp` answers "Cannot access memory") rather than reading the
    /// all-ones an absent *device*'s config space would, so a wrong address or a skipped enable
    /// step would show up here as "nothing found," not as a false pass.
    #[cfg(target_arch = "x86_64")]
    #[test_case]
    fn acpi_mcfg_wires_a_real_ecam_window_that_finds_the_host_bridge() {
        assert!(
            is_host_bridge_present(),
            "ACPI's MCFG did not leave a usable window in memory::pci_regions()"
        );

        let mut found_host_bridge = false;
        let mut count = 0usize;
        pci::enumerate(
            ecam_buses(),
            &mut |b, o| cfg_read32(b, o),
            &mut |bdf, vendor, device| {
                count += 1;
                if bdf.bus == 0 && bdf.dev == 0 && bdf.func == 0 {
                    // **The ids are a diagnostic, not a pass condition**, and this is the one
                    // thing xenon changed about this test. An earlier version asserted
                    // `0x29c0`, q35's host bridge, which is a claim about which chipset is
                    // being *emulated* rather than about whether ECAM reads reach config
                    // space. It panicked on the first real machine it ever met (2026-09-17,
                    // an OptiPlex 7050, whose Kaby Lake bridge answers a different id), and
                    // it halted the suite before the NVMe test the boot was for.
                    //
                    // What this test claims is in its own doc comment: the window MCFG named
                    // decodes real configuration space. The evidence for that is that 0:0.0
                    // answers *something a device answers*, because an address outside a
                    // decoded window reads back all-ones and one that is not decoded at all
                    // faults. A specific id adds nothing to it.
                    assert_ne!(
                        vendor, 0xffff,
                        "bus 0 device 0 function 0 read back all-ones, which is what an absent \
                         device looks like; the ECAM window is not decoding"
                    );
                    assert_ne!(
                        vendor, 0x0000,
                        "bus 0 device 0 function 0 read back zero, which is neither a device nor \
                         the absent pattern; the window is reading memory rather than config space"
                    );
                    crate::println!("    host bridge: vendor {vendor:#06x}, device {device:#06x}");
                    found_host_bridge = true;
                }
            },
        );
        assert!(
            found_host_bridge,
            "bus 0 device 0 function 0 (the host bridge) was not found; ECAM reads are not \
             reaching real config space"
        );
        assert!(
            count >= 1,
            "enumeration walked the bus and found nothing at all"
        );
    }

    /// **A controller behind a PCIe root port is found on the bus behind it** (milestone 320).
    ///
    /// The bug this pins is the one xenon exposed: the kernel mapped one megabyte of configuration
    /// space, enumerated bus 0, and reported a machine whose NVMe was behind a root port as having
    /// no NVMe at all. What made it expensive to find is that nothing failed. `non_volatile_memory_express_tests`
    /// **skipped**, with QEMU's explanation ("`NIFE_NVME` not set on this leg?") for an absence that
    /// had an entirely different cause, and a skip reads like a fact about the run.
    ///
    /// So this test asserts the whole chain rather than the endpoint: the survey recorded more than
    /// one bus, a bridge on bus 0 states a subtree, and the controller is on the bus that bridge
    /// named. Asserting only "an NVMe was found" would pass on the flat machine too.
    ///
    /// **It does not bring the controller up, and that is a property of the machine rather than a
    /// gap in the test.** A PCI-to-PCI bridge forwards memory only inside the window its own
    /// base/limit registers describe, and this machine boots PVH with no firmware to have written
    /// them, so the controller's BAR does not decode however correctly it is placed. On xenon
    /// firmware wrote both the bus numbers and the windows and `place_bars` adopts what it finds
    /// (milestone 256). See `design/roadmap/proposals/a-bridge-window-the-kernel-programs-itself.md`.
    #[cfg(target_arch = "x86_64")]
    #[test_case]
    fn a_controller_behind_a_bridge_is_found_on_the_bus_behind_it() {
        assert!(is_host_bridge_present(), "no ECAM window on this machine");

        let (mut bridged_to, mut controller_on) = (None, None);
        pci::walk(ecam_buses(), &mut |b, o| cfg_read32(b, o), &mut |f| {
            if let Some(behind) = f.bridge
                && behind.secondary != 0
                && bridged_to.is_none()
            {
                bridged_to = Some(behind.secondary);
            }
            if f.class == pci::CLASS_NVME && controller_on.is_none() {
                controller_on = Some(f.bdf.bus);
            }
        });

        let Some(secondary) = bridged_to else {
            // The flat `q35` every other leg boots, which is the honest answer here and names the
            // knob rather than leaving a reader to guess, the way the skip this milestone came from
            // did not.
            crate::testing::skip!(
                "no configured PCI bridge on this machine: q35 is a flat root complex unless \
                 NIFE_PCIE_ROOT_PORT=1 puts the NVMe controller behind a root port, which \
                 `cargo xtask test --arch x86_64` does on its own leg"
            );
        };

        assert!(
            ecam_buses() >= 2,
            "a bridge states a subtree at bus {secondary} but the survey mapped {} bus(es)",
            ecam_buses(),
        );
        assert_eq!(
            controller_on,
            Some(secondary),
            "the NVMe controller was not on the bus the bridge named; before milestone 320 this \
             read as None, because the walk never left bus 0"
        );
    }
}
