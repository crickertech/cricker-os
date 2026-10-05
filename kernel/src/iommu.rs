//! **The portable IOMMU seam** (milestone 16b; DECISIONS §20, notes/iommu.md).
//!
//! One IOMMU confines a device by translating every address it emits through page tables the
//! kernel programs, in the CPU's own page-table format. aarch64 has an SMMUv3 that walks
//! VMSAv8-64; riscv has the ratified RISC-V IOMMU (v1.0.1) that walks Sv39. Those are the two
//! formats the `paging` crate already builds for process address spaces, so a device's DMA domain
//! is not a new kind of table (`paging::domain::build_identity_domain`). This module is the thin,
//! architecture-neutral layer that ties that portable domain builder to whichever arch IOMMU
//! driver is compiled in (`crate::arch::iommu`): pick the frame allocator and the format, build
//! the domain, hand its root to the driver's `attach`. Everything ISA-specific (the register file,
//! the stream/device table, the command and fault queues) lives under `arch/`, per rule #1.
//!
//! # What "confine" does, and why it fails closed
//!
//! `confine(rid, regions)` builds an identity map (IOVA == PA) over exactly `regions` and nothing
//! else, then attaches the device whose requester id is `rid` to it. From that point the device
//! reaches precisely the frames in `regions`; any other address it emits has no mapping, so the
//! IOMMU faults instead of touching memory. Before `confine` runs for a device, the driver's
//! `init` has already installed an all-invalid stream table / device directory, so an unattached
//! device is denied by default. The window between "the bus grants Bus-Master Enable" and "the
//! kernel confines the device" is therefore closed by the hardware, not by careful ordering.

use paging::domain::{DmaRegion, build_identity_domain};

// The fault-reporting surface, re-exported as the portable interface. `take_fault` is drained by
// the confinement test (kernel/src/virtio.rs); `Fault` is its public return type. Neither is used
// by a production fault handler yet (that routing is future work), and the test reads the fault's
// fields without ever naming the type, so both re-exports are allowed to be locally unused.
#[allow(unused_imports)]
pub use crate::arch::iommu::{Fault, take_fault};
use crate::arch::mmu::phys_to_virt;

/// Is an IOMMU present and initialized on this machine? False on a `virt` boot without
/// `iommu=smmuv3` (aarch64) or without a `riscv-iommu-pci` function (riscv), where the kernel
/// runs exactly as it did before this milestone. The confinement path checks this before
/// attaching a device, so a machine with no IOMMU keeps working (with only the software shadow
/// ring for DMA defence, now demoted to defence in depth; see notes/dma.md).
pub fn is_active() -> bool {
    crate::arch::iommu::is_active()
}

/// **Whether a requester id's DMA passes through an IOMMU this kernel programmed** (milestone
/// 261's bench rehearsal). The question `confine` cannot answer for itself: it builds a domain and
/// hands it to the unit that is up, and on a machine with more than one unit that may not be the
/// unit the device's transactions reach. Fatal risk 6's first night-of condition, as a value.
///
/// Name: provisional. calef names public items.
// Each architecture's driver constructs its own subset: VT-d the last three, the SMMUv3 and the
// RISC-V IOMMU `WholeBus`. So on every build some variant is never constructed, by design.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// No IOMMU is translating on this machine.
    NoIommu,
    /// One IOMMU fronts every requester on the bus: the SMMUv3 and the RISC-V IOMMU as this tree
    /// brings them up, where the device tree's `iommu-map` is an identity over the whole bus.
    WholeBus,
    /// VT-d: the unit at `unit` is translating and owns this requester, and `how` says why.
    Owned {
        unit: u64,
        how: machine_discovery::acpi::Ownership,
    },
    /// VT-d: **nothing this kernel brought up owns** this requester. `owner` is the unit that
    /// does, which this kernel refused to bring up, or `None` when no unit owns it at all;
    /// `translating` is some unit that is up. The device's DMA is not translated by anything this
    /// kernel set up.
    Elsewhere {
        translating: u64,
        owner: Option<u64>,
    },
    /// VT-d: the DMAR described more than this kernel records, so no answer is honest.
    Unknown { translating: u64 },
}

impl Scope {
    /// True only when this kernel's page tables are what the device's DMA is checked against.
    pub fn is_confining(&self) -> bool {
        matches!(self, Scope::WholeBus | Scope::Owned { .. })
    }
}

impl core::fmt::Display for Scope {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        use machine_discovery::acpi::Ownership;
        match *self {
            Scope::NoIommu => write!(f, "no iommu is translating on this machine"),
            Scope::WholeBus => write!(f, "the machine's one iommu fronts the whole bus"),
            Scope::Owned { unit, how } => match how {
                Ownership::Named => write!(f, "drhd {unit:#x} names it in its scope"),
                Ownership::UnderBridge(b, d, fu) => {
                    write!(
                        f,
                        "drhd {unit:#x} owns bridge {b:02x}:{d:02x}.{fu} above it"
                    )
                }
                Ownership::CatchAll => {
                    write!(
                        f,
                        "drhd {unit:#x} is the catch-all and no other unit names it"
                    )
                }
            },
            Scope::Elsewhere {
                translating,
                owner: Some(owner),
            } => write!(
                f,
                "drhd {owner:#x} owns it, but it did not come up; this kernel translates {translating:#x}"
            ),
            Scope::Elsewhere {
                translating,
                owner: None,
            } => write!(
                f,
                "no drhd owns it (no scope names it, no catch-all); {translating:#x} is up"
            ),
            Scope::Unknown { translating } => write!(
                f,
                "the dmar did not fit what this kernel records; {translating:#x} is up"
            ),
        }
    }
}

/// Ask the architecture's driver for `rid`'s [`Scope`].
pub fn scope_of(rid: u32) -> Scope {
    crate::arch::iommu::scope_of(rid)
}

/// How many regions one domain carries: a caller's grant (two today, virtio's) plus every RMRR
/// the DMAR decoder can record. Name: provisional (milestone 594 (every VT-d unit translates its own devices)).
const MAX_CONFINED_REGIONS: usize = 4 + machine_discovery::acpi::MAX_RMRRS;

/// Allocate one zeroed frame and return its physical address. The domain's root table and every
/// intermediate table come from here. These frames are owned by the IOMMU from now on; a re-attach
/// of the same device leaks the previous domain's tables, which is acceptable because `confine`
/// runs once per device per boot (see notes/iommu.md).
fn zeroed_page_frame() -> u64 {
    crate::memory::alloc_zeroed()
        .expect("no frame for an IOMMU DMA domain")
        .addr()
}

/// **Confine PCI device `rid` to exactly `regions`.** Builds a DMA domain in this architecture's
/// page-table format (the seam's whole point: one call, and `arch::iommu::build_domain` picks
/// VMSAv8-64 on aarch64, Sv39 on riscv, and VT-d's or AMD-Vi's format on `x86_64`) and attaches
/// the device to it. After this returns, the device faults on any
/// address outside `regions`.
///
/// `rid` is the PCIe requester id (bus/dev/fn), which both IOMMUs key their tables on (each `virt`
/// board's device tree gives an identity `iommu-map`). The caller must have checked [`is_active`];
/// attaching before the driver's `init` panics.
pub fn confine(rid: u32, regions: &[DmaRegion]) {
    // **The grant, plus every region the firmware reserved for this device** (milestone 594). On
    // VT-d an RMRR is memory the firmware keeps DMA-ing into (USB legacy emulation, a UMA GPU's
    // stolen memory), and VT-d 4.1 section 3.16 asks for it identity-mapped in whatever domain
    // the device uses; a domain built without it would fault the firmware the moment it was
    // attached. The other two architectures report none. A region that overlaps the grant is
    // left out rather than mapped twice, which would fail the whole build: the grant already
    // covers those pages, and a firmware region overlapping kernel-allocated memory is a firmware
    // bug the boot print names.
    let mut all = [DmaRegion { base: 0, size: 0 }; MAX_CONFINED_REGIONS];
    assert!(
        regions.len() <= all.len(),
        "a DMA grant of {} regions is more than confine carries",
        regions.len()
    );
    all[..regions.len()].copy_from_slice(regions);
    let mut n = regions.len();
    crate::arch::iommu::for_each_reserved_region(rid, &mut |r| {
        let overlaps = all[..n].iter().any(|g| {
            r.base < g.base.saturating_add(g.size) && g.base < r.base.saturating_add(r.size)
        });
        if !overlaps && n < all.len() {
            all[n] = r;
            n += 1;
        }
    });
    let regions = &all[..n];

    // The format is the architecture's to choose, and on `x86_64` the running machine's: VT-d and
    // AMD-Vi walk different tables (lane `amd-vi`).
    let root = crate::arch::iommu::build_domain(regions);

    // The domain's cache tag (ASID on aarch64, PSCID on riscv). One per device; the requester id is
    // unique per device and never zero for a real PCI function (dev >= 1), so it is a fine tag.
    crate::arch::iommu::attach(rid, root, rid as u16);
}

/// **Build an identity domain over exactly `regions` in format `F`, and return its root.** The
/// architecture's `build_domain` names `F`; this is the part that is the same everywhere: the
/// frame allocator and the pointer projection the kernel's own `Mapper` uses.
pub(crate) fn build_identity<F: paging::PageFormat>(regions: &[DmaRegion]) -> u64 {
    let root = zeroed_page_frame();
    // SAFETY: `root` is a freshly zeroed, page-aligned frame; `zeroed_page_frame` returns the same for
    // every intermediate table; `phys_to_virt` yields a pointer the Mapper contract accepts. Nobody
    // installs `root` until the driver's `attach`, which happens only after this returns.
    unsafe {
        build_identity_domain::<_, _, F>(
            root,
            || Some(zeroed_page_frame()),
            |pa| phys_to_virt(pa) as *mut paging::PageTable,
            regions,
        )
        .expect("IOMMU DMA domain build failed");
    }
    // The tables are read by the IOMMU, a separate observer: publish them before it is pointed at
    // the root. The driver's `attach` issues its own invalidation + sync after installing the STE /
    // device context, so a stale cached entry cannot survive either.
    crate::arch::direct_memory_access_write_barrier();
    root
}

/// The physical regions a virtio device's domain must cover: the driver's DMA region (its rings'
/// used half and its data buffers) and the kernel-private shadow page (the descriptor table and
/// available ring the device actually reads). Both are frame-granular. See notes/dma.md for why the
/// device reads a shadow the driver cannot write.
pub fn virtio_regions(
    direct_memory_access_base: u64,
    direct_memory_access_size: u64,
    shadow_base: u64,
) -> [DmaRegion; 2] {
    [
        DmaRegion {
            base: direct_memory_access_base,
            size: direct_memory_access_size,
        },
        DmaRegion {
            base: shadow_base,
            size: page_frames::FRAME_SIZE,
        },
    ]
}
