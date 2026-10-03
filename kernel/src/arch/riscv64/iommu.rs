//! The RISC-V IOMMU (v1.0.1) driver: riscv's IOMMU, in front of the PCIe bus.
//!
//! Milestone 16b (DECISIONS §20, notes/iommu.md). The ratified RISC-V IOMMU is the architectural
//! twin of the SMMUv3 next door (arch/aarch64/iommu.rs), and the structural rhyme is the point of
//! building both: a **device directory table** instead of a stream table (keyed by the PCIe
//! requester id, called `device_id`), a **device context** instead of an STE+CD (the IOMMU's copy
//! of `satp`: a first-stage `iosatp` naming an Sv39 root the portable seam built), a **command
//! queue** for invalidations, and a **fault queue** where blocked transactions are recorded.
//!
//! On QEMU's `virt` board the IOMMU is itself a PCI function (`riscv-iommu-pci`, Red Hat
//! 1b36:0014): its register file lives in BAR 0, which the kernel places like any other BAR (the
//! kernel is the firmware here; see notes/pcie.md). The portable seam does that PCI legwork and
//! hands this module the register base; everything after that is this file.
//!
//! # Default deny, and one honest asymmetry
//!
//! `init` installs an all-invalid device directory before enabling anything, so a device the
//! kernel never attached faults (`DDT_ENTRY_INVALID`) instead of reaching memory. Before `init`
//! runs, though, the reset state of `ddtp` is mode `Off`, which *blocks* all transactions on this
//! IOMMU, so the pre-init window fails closed by architecture here; the SMMU needed an explicit
//! `GBPA.ABORT` for the same posture. Sv39 single-stage translation requires the U bit on every
//! leaf (a device does not "request supervisor privilege"), which is why the seam builds domains
//! with `user_data` flags; see `paging::domain`.
//!
//! # BUGS
//!
//! - **The MSI page table is offered and deliberately not used** (milestone 317). `CAPS.MSI_FLAT`
//!   is set on QEMU's `riscv-iommu-pci` (measured: `CAPS = 0x78c2cf4f10`, bit 22 set, on QEMU
//!   11.1.1's `virt`), so every device context here is the extended 64-byte format and carries
//!   `msiptp`, `msi_addr_mask` and `msi_addr_pattern`. [`attach`] writes all four of those words
//!   **zero**, which is `msiptp.MODE = Off`. No MSI page table is allocated and no entry in one is
//!   ever programmed.
//!
//!   **This is the whole of riscv's position on interrupt confinement, and it is not a gap in the
//!   same shape as the other two.** MSI confinement lives somewhere different on each
//!   architecture: a separate IOMMU feature on `x86_64` (`ECAP.IR`, and VT-d intercepts a write to
//!   the MSI address range so DMA remapping alone does not cover it), a separate *device* on
//!   aarch64 (the `GICv3` ITS, which this tree's GICv2 machine does not have at all), and here, one
//!   mode field inside a device context this driver already writes.
//!
//!   **What `MODE = Off` means for a transaction is a reading of the spec and not a measurement**,
//!   and it is flagged as such because this tree has carried a claim from memory before. The
//!   RISC-V IOMMU specification says MSI address translation is not performed in that mode and the
//!   transaction is translated as an ordinary memory access, which would mean an MSI-shaped write
//!   from an attached device is still bounded by the `iosatp` domain rather than escaping it. **No
//!   boot here has tested that**, nothing forges an MSI, and `MSI page table offered (mode off)`
//!   in the machine description is a report of the capability bit, not of any behaviour.
//!   See design/roadmap/317-interrupt-remapping-flags.md.
//! - **Untestable on the silicon this project owns, which inverts `x86_64`'s position.** The
//!   comment below on `CAP_MSI_FLAT` notes that real silicon without MSI support reports
//!   otherwise, and that branch has run zero times: no board shipping the ratified RISC-V IOMMU
//!   exists, milestone 143 (silicon IOMMU), and the JH7110 (radon, the `VisionFive` 2) has no IOMMU at all. So this
//!   architecture is confined in the emulated case and has no second witness, where `x86_64` has
//!   xenon waiting to provide one.

use crate::arch::mmu::phys_to_virt;
use crate::sync::{IrqSafeMutex, rank};

// --- Register file (offsets into BAR 0; RISC-V IOMMU spec ch. 5, QEMU's riscv-iommu-bits.h) ---
const CAPS: u64 = 0x00; // 64-bit
const FCTL: u64 = 0x08;
const DDTP: u64 = 0x10; // 64-bit
const CQB: u64 = 0x18; // 64-bit
const CQH: u64 = 0x20;
const CQT: u64 = 0x24;
const FQB: u64 = 0x28; // 64-bit
const FQH: u64 = 0x30;
const FQT: u64 = 0x34;
const CQCSR: u64 = 0x48;
const FQCSR: u64 = 0x4c;

const CAP_SV39: u64 = 1 << 9;
const CAP_MSI_FLAT: u64 = 1 << 22;

// ddtp: mode in bits [3:0], busy at 4, PPN at [53:10]. Mode 2 = one-level device directory.
const DDTP_MODE_1LVL: u64 = 2;
const DDTP_BUSY: u64 = 1 << 4;

// cqcsr/fqcsr: enable at 0, active ("on") at 16, busy at 17; error bits 8..11 for diagnostics.
const QUEUE_EN: u32 = 1 << 0;
const QUEUE_ON: u32 = 1 << 16;
const QUEUE_BUSY: u32 = 1 << 17;

// Queue geometry: the base register's bits [4:0] hold log2(entries) - 1, PPN at [53:10]. One
// frame each: 256 commands of 16 bytes; 128 fault records of 32 bytes.
const CQ_LOG2: u32 = 8;
const CMD_BYTES: u64 = 16;
const FQ_LOG2: u32 = 7;
const FQ_BYTES: u64 = 32;

// Commands: opcode bits [6:0], function bits [9:7] of dword 0.
const CMD_IOTINVAL_VMA: u64 = 1; // func 0
const CMD_IOFENCE_C: u64 = 2; // func 0
const CMD_IODIR_INVAL_DDT: u64 = 3; // func 0
const CMD_DV: u64 = 1 << 33; // IODIR: the DID field is valid
const CMD_DID_SHIFT: u32 = 40;

// Device context fields. tc.V validates the entry; fsc = iosatp: mode Sv39 (8) in [63:60], PPN
// in [43:0]. ta.PSCID in [31:12] tags this domain's entries in the IOMMU's address translation
// cache the way an ASID tags the CPU TLB.
const DC_TC_V: u64 = 1 << 0;
const FSC_MODE_SV39: u64 = 8 << 60;
const TA_PSCID_SHIFT: u32 = 12;

/// One recorded fault, in portable terms. `code` is the spec's CAUSE (e.g. 15 = store/AMO page
/// fault, the "no mapping for that IOVA" case; 258 = invalid device-directory entry). Read today
/// only by the confinement test; a production fault handler is future work.
#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
#[derive(Debug, Clone, Copy)]
pub struct Fault {
    pub rid: u32,
    pub code: u32,
    pub addr: u64,
}

struct Iommu {
    base: u64,
    ddt: u64,
    /// Extended (64-byte) device contexts, because the QEMU device reports `MSI_FLAT`. The base
    /// format is 32 bytes; both are handled, decided once at init from the capabilities register.
    dc_bytes: u64,
    /// `CAPS.MSI_FLAT`: does this unit offer the flat MSI page table? Recorded so the machine
    /// description can say so, which is milestone 317's parity half: MSI confinement sits in a
    /// different place on each of the three architectures, and each one has to be able to report
    /// its own position from inside the guest rather than from a source comment. Kept separate
    /// from [`Self::dc_bytes`] even though one implies the other today, because `dc_bytes` is a
    /// layout and this is a capability, and a reader should not have to infer one from the other.
    msi_flat: bool,
    cq: u64,
    cq_tail: u32,
    // Read by take_fault (the confinement test); no production fault handler yet.
    #[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
    fq: u64,
    #[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
    fq_head: u32,
}

static IOMMU: IrqSafeMutex<Option<Iommu>> = IrqSafeMutex::new(rank::IOMMU, None);

fn r32(base: u64, off: u64) -> u32 {
    // SAFETY: the IOMMU's BAR lies inside the PCI window mapped device-typed by
    // mmu::map_everything; these register reads are side-effect-free.
    unsafe { core::ptr::read_volatile(phys_to_virt(base + off) as *const u32) }
}
fn w32(base: u64, off: u64, v: u32) {
    // SAFETY: as above.
    unsafe { core::ptr::write_volatile(phys_to_virt(base + off) as *mut u32, v) }
}
fn r64(base: u64, off: u64) -> u64 {
    // SAFETY: as above; the 64-bit registers are 8-byte aligned.
    unsafe { core::ptr::read_volatile(phys_to_virt(base + off) as *const u64) }
}
fn w64(base: u64, off: u64, v: u64) {
    // SAFETY: as above.
    unsafe { core::ptr::write_volatile(phys_to_virt(base + off) as *mut u64, v) }
}

fn zeroed_page_frame(what: &str) -> u64 {
    let pa = crate::memory::alloc()
        .unwrap_or_else(|| panic!("no frame for the IOMMU {what}"))
        .addr();
    // SAFETY: a fresh frame, reachable through the direct map, owned by this module from here on.
    unsafe {
        core::ptr::write_bytes(
            phys_to_virt(pa) as *mut u8,
            0,
            page_frames::FRAME_SIZE as usize,
        );
    }
    pa
}

/// A queue base register value: PPN in [53:10], log2(entries)-1 in [4:0].
fn queue_base(pa: u64, log2: u32) -> u64 {
    ((pa >> 12) << 10) | (log2 as u64 - 1)
}

/// Does device `rid` have a context in the one-level device directory, whose `dc_bytes`-long entries
/// fill one frame? This is the only thing between a requester id and a raw write, so it is a
/// function of its own: `no_device_can_reach_another_devices_context` assumes exactly what it
/// admits and proves the stride in `context_offset` agrees with it.
fn is_in_directory(rid: u32, dc_bytes: u64) -> bool {
    (rid as u64) < page_frames::FRAME_SIZE / dc_bytes
}

/// Where device `rid`'s context sits in the device directory, as a byte offset into its frame.
fn context_offset(rid: u32, dc_bytes: u64) -> u64 {
    assert!(
        is_in_directory(rid, dc_bytes),
        "device_id {rid} beyond the one-level device directory"
    );
    rid as u64 * dc_bytes
}

/// A device context's words in directory order: `tc`, `iohgatp`, `ta`, `fsc`, then the extended
/// format's four MSI words. The base (32-byte) format writes only the first four. Pure so that
/// `the_iommu_is_handed_exactly_the_domain_the_kernel_built` can read each field
/// back at the spec's bit positions.
fn device_context(root: u64, pscid: u16) -> [u64; 8] {
    [
        DC_TC_V,                          // tc: valid, faults reported (DTF clear)
        0,                                // iohgatp: Bare
        (pscid as u64) << TA_PSCID_SHIFT, // ta
        FSC_MODE_SV39 | (root >> 12),     // fsc = iosatp
        0,                                // msiptp: MODE Off (see BUGS above)
        0,                                // msi_addr_mask
        0,                                // msi_addr_pattern
        0,                                // reserved
    ]
}

/// `IODIR.INVAL_DDT` for exactly device `rid`: opcode 3, function 0, and DV set, so the IOMMU
/// honours the DID field rather than dropping every device's cached context.
fn iodir_inval_ddt(rid: u32) -> u64 {
    CMD_IODIR_INVAL_DDT | CMD_DV | ((rid as u64) << CMD_DID_SHIFT)
}

/// Enable a queue through its control/status register and wait for it to come on.
fn queue_enable(base: u64, csr: u64, what: &str) {
    w32(base, csr, QUEUE_EN);
    let mut spins = 0u32;
    loop {
        let v = r32(base, csr);
        if v & QUEUE_ON != 0 && v & QUEUE_BUSY == 0 {
            break;
        }
        spins += 1;
        assert!(
            spins < 1_000_000,
            "IOMMU {what} queue never came on ({v:#x})"
        );
    }
}

/// Bring the IOMMU up: queues live, an all-invalid device directory installed. From here every
/// device faults until `attach` writes its context.
pub fn init(base: u64) {
    let mut g = IOMMU.lock();
    assert!(g.is_none(), "IOMMU initialized twice");

    let caps = r64(base, CAPS);
    assert!(
        caps & CAP_SV39 != 0,
        "the IOMMU does not support Sv39 first-stage (caps {caps:#x}); the seam depends on it"
    );
    // MSI_FLAT widens the device context from 32 to 64 bytes (the MSI page-table words). QEMU's
    // riscv-iommu-pci reports it, so the extended format is what this driver actually runs; the
    // base format is kept because real silicon without MSI support will report otherwise.
    let msi_flat = caps & CAP_MSI_FLAT != 0;
    let dc_bytes = if msi_flat { 64 } else { 32 };

    // Little-endian, wire-signaled interrupts off (we poll both queues).
    w32(base, FCTL, 0);

    let cq = zeroed_page_frame("command queue");
    w64(base, CQB, queue_base(cq, CQ_LOG2));
    queue_enable(base, CQCSR, "command");

    let fq = zeroed_page_frame("fault queue");
    w64(base, FQB, queue_base(fq, FQ_LOG2));
    queue_enable(base, FQCSR, "fault");

    // The device directory: one page of device contexts, all invalid. Keyed by device_id, which
    // for PCIe is the requester id; one level covers ids 0..63 in the extended format, which
    // spans all of bus 0. Installing it (mode 1LVL) is what arms translation: from this write on,
    // a transaction from an unattached device is blocked with DDT_ENTRY_INVALID in the fault
    // queue rather than reaching memory.
    let ddt = zeroed_page_frame("device directory");
    w64(base, DDTP, DDTP_MODE_1LVL | ((ddt >> 12) << 10));
    let mut spins = 0u32;
    while r64(base, DDTP) & DDTP_BUSY != 0 {
        spins += 1;
        assert!(spins < 1_000_000, "IOMMU ddtp write never completed");
    }

    *g = Some(Iommu {
        base,
        ddt,
        dc_bytes,
        msi_flat,
        cq,
        cq_tail: 0,
        fq,
        fq_head: 0,
    });
}

/// Is the IOMMU up? The portable seam asks this to decide whether attaching is possible.
/// **This machine's IOMMU, for the machine description** (milestone 268).
///
/// One of the eight questions the description answers on every architecture. The vocabulary is this
/// architecture's, because that is what parity means here: the same question, answered in the terms
/// of the hardware that answers it. A machine with no IOMMU says so plainly rather than printing a
/// blank, because a blank is indistinguishable from a line nobody wrote.
// The machine description is the only caller, and it is
// `#[cfg(not(any(test, feature = "bench")))]`: a test boot exits through semihosting and a bench
// boot diverges into `bench::run`, so neither reads a bring-up transcript. Same treatment
// `memory::print_summary` already carries, and for the same reason.
#[cfg_attr(
    any(test, feature = "system_tests", feature = "bench"),
    allow(dead_code)
)]
pub fn print_summary() {
    match IOMMU.lock().as_ref() {
        Some(s) => crate::println!(
            "  iommu           : riscv-iommu at {:#018x}, device directory default-deny, \
             translating, MSI page table {} (mode off)",
            s.base,
            if s.msi_flat { "offered" } else { "absent" },
        ),
        None => crate::println!(
            "  iommu           : none (no riscv-iommu-pci function on this machine's bus)",
        ),
    }
}

pub fn is_active() -> bool {
    IOMMU.lock().is_some()
}

/// **Whether requester `rid` is behind this IOMMU** (milestone 261 (the NVMe driver leaves the kernel)'s bench rehearsal): always, when
/// it is up. This tree brings the unit up against a device tree whose `iommu-map` is an identity
/// over the whole bus, so there is no second unit for a device to belong to. VT-d is the
/// architecture where that stops being true; see `arch::x86_64::iommu::scope_of`.
pub fn scope_of(_rid: u32) -> crate::iommu::Scope {
    if is_active() {
        crate::iommu::Scope::WholeBus
    } else {
        crate::iommu::Scope::NoIommu
    }
}

/// **Firmware-reserved DMA regions for requester `rid`: none on this architecture as this tree
/// brings it up** (milestone 594 (every VT-d unit translates its own devices)). The VT-d driver reports its RMRRs here so
/// [`crate::iommu::confine`] can map them into every domain. The RISC-V IOMMU has no firmware table for this; a device tree `reserved-memory` region with `iommu-addresses` would be the place, and this tree reads none.
pub fn for_each_reserved_region(_rid: u32, _each: &mut dyn FnMut(paging::domain::DmaRegion)) {}

/// Push one 16-byte command and wait for the IOMMU to consume it (QEMU processes the queue on
/// the tail write; polling the head is the architectural contract).
fn cmd_push(s: &mut Iommu, dword0: u64, dword1: u64) {
    let entries = 1u32 << CQ_LOG2;
    let idx = (s.cq_tail % entries) as u64;
    let slot = phys_to_virt(s.cq + idx * CMD_BYTES) as *mut u64;
    // SAFETY: the command queue frame is kernel-owned; idx is masked to the queue.
    unsafe {
        core::ptr::write_volatile(slot, dword0);
        core::ptr::write_volatile(slot.add(1), dword1);
    }
    // The IOMMU reads the command from memory: publish the words before moving the tail.
    crate::arch::direct_memory_access_write_barrier();
    s.cq_tail = (s.cq_tail + 1) % (2 * entries);
    w32(s.base, CQT, s.cq_tail);

    let mut spins = 0u32;
    while r32(s.base, CQH) != s.cq_tail {
        spins += 1;
        assert!(
            spins < 1_000_000,
            "IOMMU stopped consuming commands (cqcsr {:#x})",
            r32(s.base, CQCSR),
        );
    }
}

/// Point device `rid` at the domain rooted at `root` (an Sv39 table the portable seam built),
/// tagged `pscid`, then invalidate so the IOMMU drops anything cached for the previous domain.
pub fn attach(rid: u32, root: u64, pscid: u16) {
    let mut g = IOMMU.lock();
    let s = g.as_mut().expect("IOMMU attach before init");
    let offset = context_offset(rid, s.dc_bytes);
    let words = device_context(root, pscid);

    // The device context, written back to front: address and tag words first, the valid bit
    // last, with a barrier between, so the IOMMU can never observe a valid entry with a stale
    // root. iohgatp stays Bare (no second stage; there is no guest here), and the MSI words stay
    // zero (MSI mode Off). The words themselves come from `device_context`, which is pure so
    // the harnesses below can prove what they say; this function only decides the order.
    let dc = phys_to_virt(s.ddt + offset) as *mut u64;
    // SAFETY: ddt is a kernel-owned frame; `context_offset` bounds rid so the whole
    // `dc_bytes`-long entry lies inside it.
    unsafe {
        core::ptr::write_volatile(dc.add(1), words[1]); // iohgatp: Bare
        core::ptr::write_volatile(dc.add(2), words[2]); // ta
        core::ptr::write_volatile(dc.add(3), words[3]); // fsc = iosatp
        if s.dc_bytes == 64 {
            for (i, &w) in words.iter().enumerate().skip(4) {
                core::ptr::write_volatile(dc.add(i), w); // msiptp, mask, pattern, reserved
            }
        }
    }
    crate::arch::direct_memory_access_write_barrier();
    // SAFETY: as above.
    unsafe {
        core::ptr::write_volatile(dc, words[0]); // tc: valid, faults reported (DTF clear)
    }
    crate::arch::direct_memory_access_write_barrier();

    // Invalidate the cached context and every translation, then fence: the IOMMU's IODIR covers
    // its device-context cache, IOTINVAL.VMA (no address, no PSCID: everything) its address
    // translation cache, IOFENCE.C orders both before any later transaction.
    cmd_push(s, iodir_inval_ddt(rid), 0);
    cmd_push(s, CMD_IOTINVAL_VMA, 0);
    cmd_push(s, CMD_IOFENCE_C, 0);
}

/// Pop one fault from the fault queue, if any. The confinement test drains this to prove a DMA
/// escape was stopped by the hardware, not merely absent.
#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
pub fn take_fault() -> Option<Fault> {
    let mut g = IOMMU.lock();
    let s = g.as_mut()?;
    let entries = 1u32 << FQ_LOG2;
    let tail = r32(s.base, FQT);
    if tail == s.fq_head {
        return None;
    }
    let idx = (s.fq_head % entries) as u64;
    let rec = phys_to_virt(s.fq + idx * FQ_BYTES) as *const u64;
    // SAFETY: the fault queue frame is kernel-owned; the IOMMU wrote this record before moving
    // the tail, and idx is masked to the queue.
    let (hdr, iotval) = unsafe {
        (
            core::ptr::read_volatile(rec),
            core::ptr::read_volatile(rec.add(2)),
        )
    };
    s.fq_head = (s.fq_head + 1) % (2 * entries);
    w32(s.base, FQH, s.fq_head);
    Some(Fault {
        rid: (hdr >> 40) as u32 & 0xff_ffff, // DID, bits [63:40]
        code: (hdr & 0xfff) as u32,          // CAUSE, bits [11:0]
        addr: iotval,
    })
}

/// The RISC-V IOMMU's counterpart to the SMMUv3's proofs in `arch/aarch64/iommu.rs`, from
/// milestone 432 (the RISC-V IOMMU driver has no counterpart to the SMMU's proofs).
///
/// **These run on an aarch64 host, never on riscv64.** Kani compiles for the host and no host here
/// is riscv64, so this file reaches the prover through a proof-only module in `arch/mod.rs` that
/// exists only under `cfg(all(kani, target_arch = "aarch64"))`. Two consequences a reader must
/// carry:
///
/// - **Inside that module `crate::arch` is aarch64's.** This file's `phys_to_virt` and write
///   barrier resolve to the host's, so nothing that calls through `crate::arch` is proved here, and
///   neither harness calls anything that does: `device_context`, `is_in_directory`,
///   `context_offset` and `iodir_inval_ddt` are word arithmetic on their arguments, this file's
///   constants and `page_frames::FRAME_SIZE`. `script/lint` fails if this file gains a `crate::arch` reference it has not recorded, because that is the
///   moment a future harness could start proving aarch64's code while reading as riscv64's. See
///   notes/kernel-proofs.md, stub-list item 8.
/// - **The register offsets and bit constants are not proved and cannot be**, exactly as on the
///   SMMU side: the harnesses read fields back at the positions the RISC-V IOMMU specification
///   (v1.0.1, ch. 3) gives them, and if that reading is wrong the code and the proof are wrong
///   together. The boot-time confinement test in `kernel/src/virtio.rs` is not made redundant.
///
/// **Why these properties and not the SMMU's** (milestone 432's "what it is not"). The SMMU splits
/// a 64-bit address across two 32-bit words that share a word with control bits; this device
/// context is written in whole 64-bit stores, so that hazard does not exist here. What does exist is
/// the same pair of questions in this format's shape: does the context name exactly the domain the
/// seam built (address, tag, and a mode that is still translating), and can one requester id's
/// entry, or its invalidation, land on another's.
#[cfg(kani)]
mod proofs {
    use super::*;

    /// A RISC-V physical address is at most 56 bits (a 44-bit PPN of 4 KiB pages), which is also
    /// the width of `iosatp`'s PPN field. Spelled once so the assumption and the field agree.
    const PA_BITS: u32 = 56;

    /// **The IOMMU is handed exactly the domain the kernel built, and it is still translating.**
    ///
    /// `fsc` is `iosatp`: MODE in bits [63:60], reserved zero in [59:44], the root's PPN in
    /// [43:0]. Two failures live in that word. The PPN can be wrong, in which case the IOMMU walks
    /// some other table; or MODE can stop being Sv39 (8), and MODE 0 is **Bare**, which is
    /// translation switched off rather than pointed somewhere wrong. The first half is stated for
    /// EVERY `root`, with no assumption, because "no address can turn translation off" is the
    /// claim worth having unconditionally. The rest assumes what `attach`'s caller establishes and
    /// nothing checks: the root is a page frame below 2^56.
    ///
    /// `ta.PSCID` is the tag the IOMMU's address translation cache keys on. A wrong tag is
    /// invisible to every test here: the confinement test attaches one device and `attach`
    /// invalidates the whole cache after every write, so two domains sharing a tag never both
    /// have live entries. It would matter the day invalidation is narrowed to one PSCID.
    ///
    /// Falsification: replayable `kernel/falsifications/arch.riscv64.iommu.proofs.the_iommu_is_handed_exactly_the_domain_the_kernel_built.patch`
    #[kani::proof]
    fn the_iommu_is_handed_exactly_the_domain_the_kernel_built() {
        let root: u64 = kani::any();
        let pscid: u16 = kani::any();

        let dc = device_context(root, pscid);
        assert!(
            dc[3] >> 60 == 8,
            "iosatp.MODE is Sv39 for every root: no address bit can turn translation off"
        );

        // `root` comes from `paging::domain`, so a page frame, and from RAM, so a real address.
        kani::assume(root % page_frames::FRAME_SIZE == 0);
        kani::assume(root < 1 << PA_BITS);

        // Stated on the whole address rather than on the shifted field, so the harness does not
        // repeat the implementation's `>> 12` back to itself.
        assert!(
            (dc[3] & ((1 << 44) - 1)) << 12 == root,
            "iosatp.PPN is the table root entire"
        );
        assert!(
            (dc[3] >> 44) & 0xffff == 0,
            "iosatp's reserved bits [59:44] are zero: no address bit reached them"
        );
        assert!(
            (dc[2] >> 12) & 0xf_ffff == pscid as u64 && dc[2] & !(0xf_ffff << 12) == 0,
            "ta.PSCID is this domain's tag, and nothing else in ta is set"
        );
        assert!(
            dc[0] == 1,
            "tc.V is set and nothing else: DTF clear, so faults are reported"
        );
        assert!(
            dc[1] == 0,
            "iohgatp is Bare: no second stage to translate through"
        );
        assert!(
            dc[4..].iter().all(|&w| w == 0),
            "the extended format's MSI words are zero, msiptp.MODE Off"
        );
    }

    /// **No device can reach another device's context, or invalidate the wrong one.**
    ///
    /// `context_offset` is the whole of the directory's addressing and its `assert!` is the only
    /// thing between a requester id and a raw write. The stride depends on a capability bit read
    /// at run time (64 bytes with `MSI_FLAT`, 32 without), so the bound has to agree with the
    /// stride in both formats, not only in the one QEMU happens to run; the base format has run
    /// zero times (the module's BUGS). Both formats are taken symbolically here.
    ///
    /// The invalidation is the second half of the same question. `IODIR.INVAL_DDT` names the
    /// device in a 24-bit DID field; if it named a different device, or omitted DV (which means
    /// "every device"), the old context could stay cached for the device just re-attached. The
    /// assertion is that the command decodes, at the spec's positions, to this device and no other.
    ///
    /// Falsification: replayable `kernel/falsifications/arch.riscv64.iommu.proofs.no_device_can_reach_another_devices_context.patch`
    #[kani::proof]
    fn no_device_can_reach_another_devices_context() {
        let a: u32 = kani::any();
        let b: u32 = kani::any();
        let extended: bool = kani::any();
        let dc_bytes: u64 = if extended { 64 } else { 32 };
        // A PCIe requester id is 16 bits (`pci::Bdf::requester_id`).
        kani::assume(a <= u16::MAX as u32 && b <= u16::MAX as u32);
        // Anything `context_offset` refuses panics rather than answering wrongly; restrict to what
        // it admits, asked through the same predicate rather than restated, so a bound that drifts
        // from the stride turns this red instead of being assumed away.
        kani::assume(is_in_directory(a, dc_bytes) && is_in_directory(b, dc_bytes));

        let (oa, ob) = (context_offset(a, dc_bytes), context_offset(b, dc_bytes));
        assert!(
            oa + dc_bytes <= page_frames::FRAME_SIZE,
            "every context the bound admits lies inside the directory's single frame"
        );
        assert!(
            oa % 8 == 0,
            "every context starts on the 8-byte boundary its 64-bit stores need"
        );
        assert!(
            a == b || oa + dc_bytes <= ob || ob + dc_bytes <= oa,
            "two different devices never share a byte of the directory"
        );

        let cmd = iodir_inval_ddt(a);
        assert!(
            cmd & 0x7f == 3 && (cmd >> 7) & 0b111 == 0,
            "the command is IODIR.INVAL_DDT"
        );
        assert!(
            (cmd >> 33) & 1 == 1,
            "DV is set, so the DID field is honoured"
        );
        assert!(
            cmd >> 40 == a as u64,
            "the invalidation names exactly the device just attached"
        );
    }
}
