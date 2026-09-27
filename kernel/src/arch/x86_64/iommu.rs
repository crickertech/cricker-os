//! **The VT-d driver: Intel's IOMMU, in front of the PCIe bus.**
//!
//! Milestone 16b's role, x86's device (milestone 161, roadmap item 6). The register file, the
//! root/context tables, and register-based invalidation live here; the translation tables
//! themselves are the portable seam's job (`paging::domain` via `crate::iommu`), built in
//! [`paging::x86_64::Vtd`] rather than the CPU's own [`paging::x86_64::Ia32e`] (that module's own
//! doc says why the two must not be confused).
//!
//! # The shape of the hardware, and how it differs from the other two
//!
//! SMMUv3 and the RISC-V IOMMU are both driven almost entirely through memory: a stream/device
//! table the driver writes and queues the driver pushes commands into. **VT-d is driven through
//! registers**, and that is the real architectural difference, not a detail: there is no command
//! queue and no fault queue here, because the *legacy* (non-scalable, non-queued) interface this
//! driver speaks does invalidation with a register write-and-poll (`CCMD_REG`, the IOTLB invalidate
//! register) and reports faults through a small bank of Fault Recording Registers instead of a
//! ring in memory. That interface is what every VT-d unit supports unconditionally; the queued
//! (`ECAP.QI`) and scalable-mode interfaces are supersets this driver does not need yet.
//!
//! Two levels of memory-resident table stand between a register write and a working translation:
//!
//! - The **root table**: one page, 256 entries (one per PCI bus), each either absent or pointing
//!   at a context table.
//! - The **context table**: one page per bus that has an attached device, 256 entries (one per
//!   PCI device/function), each either absent or naming a domain id and a second-level page-table
//!   root, the tables [`crate::iommu::confine`] built.
//!
//! Both tables are allocated **lazily and default-absent**, which is the same default-deny posture
//! `init` gives the SMMUv3 and the RISC-V IOMMU: a bus with no context table, or a context entry
//! with its present bit clear, faults every transaction rather than routing it anywhere.
//!
//! # What is built, against what QEMU's `-device intel-iommu` on `q35` presents
//!
//! **Every DRHD the DMAR names** (milestone 594 (every VT-d unit translates its own devices), provisional number), each with its own root
//! table, and every `attach` routed to the unit the DMAR says owns the device
//! (`machine_discovery::acpi::DmarUnits::owner_index`, VT-d 4.1 section 8.3). Translation
//! enable/disable through `GCMD`/`GSTS`, written the way section 11.4.4 prescribes (read `GSTS`,
//! clear the one-shot bits, change one bit), the root and context tables, register-based
//! context-cache and IOTLB invalidation (global granularity only), and fault detection through
//! `FSTS.PPF` and the first Fault Recording Register of each unit. The firmware's reserved memory
//! regions (RMRRs) are identity-mapped for the devices they name before any unit translates, and
//! stay mapped in every domain those devices are later confined to (section 3.16). Interrupt
//! remapping, queued invalidation and PASID/scalable mode are real VT-d features this driver does
//! not build; see the BUGS section for what each costs and where the next piece would go.
//!
//! # A device no unit owns
//!
//! Section 8.3 requires at least one DRHD per PCI segment and says the `INCLUDE_PCI_ALL` unit
//! takes every device no other unit names, and section 8.4 requires every device an RMRR names to
//! be under some unit. So on firmware that follows the specification, every PCI function has an
//! owner. The specification does not say what the hardware does with a requester no scope
//! covers, because by its own rules there is none. **This driver does what Linux does**: Linux's
//! `intel_iommu_probe_device` returns `-ENODEV` when `dmar_find_matched_drhd_unit` finds no unit,
//! and the device is left out of the IOMMU layer with its DMA untranslated. Here `attach` for such
//! a device writes no context entry anywhere (there is no root table it would be read from), and
//! [`scope_of`] answers `Elsewhere { owner: None, .. }`, so no caller that asks can claim the
//! device confined. The NVMe bench boot's `bypass` rehearsal is that case. Refusing the device
//! bus mastering instead was considered and refused: it would change what a non-compliant
//! machine does without making anything confined, and it would silence the very preflight that
//! reports the firmware's gap.
//!
//! # Default deny
//!
//! `init` zeroes the root table and points `RTADDR_REG` at it before setting `GCMD.TE`, so
//! translation turns on over an all-absent root: every bus faults until its context table exists,
//! and every device on a bus that does have one faults until `attach` writes its entry. There is
//! no window where translation is on and a device is unconstrained by omission, the same property
//! the other two drivers' `init` establishes for their own table shapes.
//!
//! # BUGS
//!
//! - **Only segment 0 is brought up.** A unit on another PCI segment is reported and refused,
//!   because this kernel reads only one ECAM window and so cannot resolve a scope path elsewhere.
//!   No machine this tree has met has a second segment.
//! - **A unit whose scope names only absent devices is still brought up.** Linux ignores such a
//!   unit (`init_no_remapping_devices`); this driver translates it with nothing attached, which
//!   denies nothing that exists. It differs from Linux only when the firmware names a unit whose
//!   register file is not really there, and then `init` panics on the first status poll instead
//!   of carrying on.
//! - **An RMRR that names a bridge is not mapped until the device below it is confined.** Before
//!   translation turns on, `init` pre-attaches the devices RMRR *endpoint* scopes name;
//!   a sub-hierarchy scope would mean walking every function below the bridge, and no machine
//!   this tree has met writes one. [`crate::iommu::confine`] still adds the region to any such
//!   device's domain. Until that confine, such a device's firmware DMA faults.
//! - **The graphics unit translates, and nothing here drives the GPU.** Its only mapping is the
//!   graphics RMRR, so the display engine keeps scanning out of stolen memory if and only if the
//!   firmware's RMRR covers the memory it scans. Linux translates the same unit on Skylake and Kaby
//!   Lake without a quirk (its `quirk_iommu_igfx` list stops at Broadwell), which is the evidence
//!   this is safe; xenon is the first run. notes/risk-6-bench-evening.md says what to watch.
//! - **No interrupt remapping.** `ECAP.IR` is read and *reported* since milestone 317
//!   (the interrupt-remapping flags, and where MSI confinement actually lives) by
//!   [`interrupt_remapping_available`] and the bring-up line `print_summary` writes, and that
//!   is all: this driver never sets `GCMD.IRE`, never allocates an interrupt-remapping table, and
//!   never programs an entry in one. MSI/MSI-X delivery is unaffected either way (this kernel does
//!   not remap interrupts on any architecture yet), but a future PCI MSI driver on x86 would want
//!   to know this is missing before assuming a remapping table exists to program.
//!
//!   **The unit has been offering it all along, which is a correction and not a feature.**
//!   DECISIONS §86 recorded that interrupt remapping is off in every `x86_64` boot this tree runs,
//!   reasoning from the runner attaching `-device intel-iommu` with no `intremap=on`. Reading
//!   `ECAP` from inside the guest says otherwise: QEMU's `intremap` property defaults to `auto`,
//!   which resolves ON with no in-kernel irqchip, so `ECAP.IR` reads set on the default machine
//!   (`0xf00f4a`) and clear only under an explicit `intremap=off` (`0xf42`). Nothing read the bit,
//!   so nobody noticed. `NIFE_INTREMAP=off` (provisional name) is now the way to reach a machine
//!   without the capability.
//!
//!   **What remains unexercised is the whole of the rest.** Nothing here writes an `IRTE`, forges
//!   an MSI, or proves that a remapped interrupt lands where the table says it should. That is the
//!   confinement claim notes/confinement-claims.md carries as stated nowhere, and
//!   design/roadmap/317-interrupt-remapping-flags.md says what it would take.
//! - **Invalidation is global, never domain- or device-selective.** Every `attach` invalidates the
//!   *entire* context cache and the *entire* IOTLB rather than just the entry that changed, which
//!   is correct (nothing survives that should not) and expensive on a machine with many attached
//!   devices, exactly the same trade the RISC-V driver's `IOTINVAL.VMA` with no address makes.
//! - **`RWBF` (`CAP_REG` bit 4) is honoured but has never been exercised.** QEMU's model does not
//!   set it, and neither do the 7040's two units, so the write-buffer-flush branch in
//!   [`invalidate_all`] has run zero times. It waited for `GSTS.WBFS` to be *set* until milestone
//!   594; section 11.4.4 says hardware clears it when the flush completes, which is what it now
//!   waits for (Linux's `iommu_flush_write_buffer` agrees).
//! - **Protected memory regions are switched off, never used.** A unit offering them
//!   (`CAP.PLMR`/`PHMR`, which the 7040's units both set) has `PMEN.EPM` cleared once translation
//!   is on, as Linux does, in case firmware left one enabled. QEMU's model offers none, so this has
//!   run zero times.
//! - **The fault path decodes and clears exactly one Fault Recording Register per unit.** `CAP.NFR` is read
//!   to find where the bank starts, not to size it; QEMU's model reports `NFR = 0` (one register),
//!   so a real unit with more than one is read at the same fixed offset only. A burst of faults past
//!   that one register sets `FSTS.PFO`, and the hardware records nothing new until it is cleared;
//!   [`take_fault`] clears it, so the faults lost in a burst are lost but the next one is recorded.

use machine_discovery::acpi::{DmarUnits, Drhd, MAX_DRHDS, MAX_RMRR_SCOPES, SCOPE_PCI_ENDPOINT};
use paging::PageFormat;
use paging::x86_64::Vtd;

use crate::arch::mmu::phys_to_virt;
use crate::sync::{IrqSafeMutex, rank};

// --- Register file (offsets from the DRHD's register base; Intel VT-d spec chapter 10, and
// QEMU's hw/i386/intel_iommu_internal.h, which is this driver's ground truth for what q35's
// emulation actually checks). ---
const CAP: u64 = 0x08; // 64-bit
const ECAP: u64 = 0x10; // 64-bit
const GCMD: u64 = 0x18;
const GSTS: u64 = 0x1c;
const RTADDR: u64 = 0x20; // 64-bit
const CCMD: u64 = 0x28; // 64-bit
const FSTS: u64 = 0x34;
const PMEN: u64 = 0x64; // 32-bit, section 11.4.8.1

// GCMD (write-only) / GSTS (read-only, same bit positions): translation and root-pointer control.
const GCMD_TE: u32 = 1 << 31; // Translation Enable
const GCMD_SRTP: u32 = 1 << 30; // Set Root Table Pointer
const GCMD_WBF: u32 = 1 << 27; // Write Buffer Flush
const GSTS_TES: u32 = 1 << 31;
const GSTS_RTPS: u32 = 1 << 30;
const GSTS_WBFS: u32 = 1 << 27;
// Section 11.4.4's mask for turning a GSTS read into a GCMD value: it clears the one-shot command
// bits (SRTP, SFL, WBF, SIRTP) so reissuing the status never repeats one of them.
const GSTS_ONE_SHOT_MASK: u32 = 0x96FF_FFFF;
// IRES: interrupt remapping is ENABLED. Read only by the test that asserts it is clear; this
// driver has no `GCMD_IRE` constant to pair it with, deliberately, because there is nothing here
// that should be one typo away from turning interrupt remapping on. See this module's BUGS.
#[cfg(any(test, feature = "system_tests"))]
const GSTS_IRES: u32 = 1 << 25;

// CAP fields this driver reads. SAGAW is a bitmap (bit N means "AGAW level N is supported"), not
// an index; bit 2 of the 5-bit field (so bit 10 of the register) is the 48-bit/4-level width
// `Vtd`'s four-level walk needs, the same width `AW_48BIT` below selects in a context entry.
const CAP_SAGAW_48BIT: u64 = 1 << 10;
const CAP_RWBF: u64 = 1 << 4;
const CAP_PLMR: u64 = 1 << 5; // protected low-memory region supported
const CAP_PHMR: u64 = 1 << 6; // protected high-memory region supported
const CAP_ND_MASK: u64 = 7; // number of domain ids: 2^(4 + 2 * ND)
// PMEN: EPM enables the protected memory regions, PRS reports them in force.
const PMEN_EPM: u32 = 1 << 31;
const PMEN_PRS: u32 = 1 << 0;
const CAP_FRO_SHIFT: u64 = 24; // 10-bit field, in 16-byte units
const CAP_FRO_MASK: u64 = 0x3ff;

// ECAP fields: where the IOTLB invalidate register lives, since unlike the fault-recording bank
// it is not at a fixed offset by specification (QEMU happens to put it at a fixed offset; a real
// unit is not required to).
const ECAP_IRO_SHIFT: u64 = 8; // 10-bit field, in 16-byte units
const ECAP_IRO_MASK: u64 = 0x3ff;

// ECAP.IR (bit 3): the unit supports interrupt remapping. Read to REPORT, never to act on
// (this driver does not set `GCMD.IRE`; see BUGS). It is here because milestone 317 needed the
// machine's own answer to "is interrupt remapping present" to be visible from inside the guest:
// `-device intel-iommu,intremap=on` is a host-side string, and a boot that cannot tell the two
// machines apart cannot claim to have exercised either. `print_summary` is where it surfaces.
const ECAP_IR: u64 = 1 << 3;

// CCMD_REG: context-cache invalidation, register-based (the legacy, non-queued interface every
// VT-d unit supports). ICC is set to start, cleared by hardware on completion; CIRG selects
// global granularity, the only one this driver uses.
const CCMD_ICC: u64 = 1 << 63;
const CCMD_CIRG_GLOBAL: u64 = 1 << 61;

// The IOTLB invalidate register, same shape as CCMD: IVT starts it, hardware clears it, IIRG
// selects granularity.
const IOTLB_IVT: u64 = 1 << 63;
const IOTLB_IIRG_GLOBAL: u64 = 1 << 60;

// FSTS: fault status. PPF says a record is waiting; PFO says a fault arrived with no free record, and
// while it is set the unit records nothing at all (write 1 to clear).
const FSTS_PFO: u32 = 1 << 0;
const FSTS_PPF: u32 = 1 << 1;

// Root entry (one page, 256 x 16 bytes, one per PCI bus): lower qword only, in legacy mode.
const ROOT_ENTRY_P: u64 = 1 << 0;
const ROOT_ENTRY_CTP_MASK: u64 = 0x000f_ffff_ffff_f000; // bits 63:12

// Context entry (one page, 256 x 16 bytes, one per device/function on a bus). Lower qword:
// present, translation type (00 = second-level-only, the shape every attach here writes), and the
// second-level page-table pointer. Upper qword: address width and domain id.
const CTX_ENTRY_P: u64 = 1 << 0;
const CTX_TT_MULTI_LEVEL: u64 = 0; // bits 3:2, value 0
const CTX_SLPTPTR_MASK: u64 = 0x000f_ffff_ffff_f000; // bits 63:12
const CTX_AW_48BIT: u64 = 2; // bits 2:0 of the upper qword: 010 = 48-bit AGAW, 4 levels
const CTX_DID_SHIFT: u64 = 8; // bits 23:8 of the upper qword

/// One recorded fault, in portable terms. `code` is the Fault Reason byte the spec defines (e.g.
/// `0x02` is a write past the second-level page table's write permission; `0x07` is no entry for
/// the address at all). Read today only by the confinement test; a production fault handler is
/// future work, the same posture the other two drivers take.
#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
#[derive(Debug, Clone, Copy)]
pub struct Fault {
    pub rid: u32,
    pub code: u32,
    pub addr: u64,
}

/// One VT-d unit this kernel brought up.
struct Unit {
    base: u64,
    root: u64,
    rwbf: bool,
    frcd: u64,
    /// The IOTLB invalidate register's offset, from `ECAP.IRO` (not fixed by the specification).
    iotlb: u64,
    /// `ECAP.IR`: does this unit offer interrupt remapping? Recorded, never acted on. See the
    /// constant's comment and this module's BUGS for why a read-only field earns its place.
    interrupt_remapping: bool,
    /// The next domain id to hand out, and the largest `CAP.ND` allows. See [`Unit::domain_id`].
    next_did: u32,
    last_did: u32,
    /// How many devices an RMRR had pre-attached here before translation turned on.
    reserved_devices: u32,
}

impl Unit {
    /// **A fresh domain id, inside the width this unit implements** (milestone 594). Section 9.3:
    /// a unit with fewer than 16-bit domain ids treats the unused high bits of a context entry's
    /// `DID` as reserved, so a wider value is a reserved-field fault on every DMA the device
    /// makes. The portable seam passes the requester id as the tag, which QEMU's 16-bit model
    /// accepts and the 7040's two units (`CAP.ND` = 2, 8-bit ids) would reject for any device off
    /// bus 0: the NVMe at 01:00.0 would have been domain 0x100. Zero is never used, since
    /// section 9.3 reserves it when `CAP.CM` is set and nothing is lost by skipping it always.
    fn domain_id(&mut self) -> u16 {
        assert!(
            self.next_did <= self.last_did,
            "VT-d unit {:#x} has handed out all {} of its domain ids (CAP.ND)",
            self.base,
            self.last_did,
        );
        let did = self.next_did as u16;
        self.next_did += 1;
        did
    }
}

/// A DMAR unit's slot: brought up, refused with a reason, or not described at all.
enum Slot {
    Absent,
    Up(Unit),
    Refused(&'static str),
}

/// Every unit, indexed as `machine_discovery::acpi::DmarUnits::units` lists them, and each
/// unit's context tables beside it rather than inside [`Unit`], so a unit is a few dozen bytes to
/// build on the boot stack and only this static holds the 4 KiB of table pointers per unit.
struct Units {
    slots: [Slot; MAX_DRHDS],
    /// Per unit, one context-table root per PCI bus, allocated the first time a device on that
    /// bus is attached. `None` is the default-deny state: a bus with no table here has no
    /// root-table entry pointing at one either, so every device on it faults.
    ctx: [[Option<u64>; 256]; MAX_DRHDS],
}

impl Units {
    fn iter(&self) -> core::slice::Iter<'_, Slot> {
        self.slots.iter()
    }
}

static IOMMU: IrqSafeMutex<Units> = IrqSafeMutex::new(
    rank::IOMMU,
    Units {
        slots: [const { Slot::Absent }; MAX_DRHDS],
        ctx: [[None; 256]; MAX_DRHDS],
    },
);

/// Every unit, device scope and RMRR the DMAR described, so [`attach`] can route a device to its
/// owner and [`scope_of`] can say which unit that is. Recorded by [`init`]; never held at the same
/// time as [`IOMMU`], because resolving a scope path reads configuration space.
static DMAR: IrqSafeMutex<Option<DmarUnits>> = IrqSafeMutex::new(rank::IOMMU, None);

fn r32(base: u64, off: u64) -> u32 {
    // SAFETY: the DRHD's register file lies inside the direct map (it is ordinary MMIO below the
    // 4 GiB line on every machine this driver has run against), mapped device-typed by
    // `mmu::map_everything`; these reads are side-effect-free registers.
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

/// Wait for `cond(gsts)` to hold, polling `GSTS_REG`. Every global-command write this driver makes
/// is confirmed this way, the same write-then-poll shape `CCMD`/IOTLB invalidation use on their
/// own registers below.
fn wait_gsts(base: u64, what: &str, cond: impl Fn(u32) -> bool) {
    let mut spins = 0u32;
    loop {
        let v = r32(base, GSTS);
        if cond(v) {
            return;
        }
        spins += 1;
        assert!(spins < 1_000_000, "IOMMU {what} never completed ({v:#x})");
    }
}

/// **Issue one global command, leaving every other one as it is** (VT-d 4.1 section 11.4.4).
/// `GCMD_REG` is write-only and every bit in it is a command, so writing a lone bit also writes
/// zero to every other one, and a zero `TE` turns translation off. The section's recipe is to
/// read the status, clear the one-shot bits (`0x96FF_FFFF`), then set or clear exactly one bit.
/// Until milestone 594 this driver wrote lone bits, which was harmless only because `WBF`, the
/// one command issued with translation on, never ran.
fn global_command(base: u64, bit: u32, on: bool) {
    let status = r32(base, GSTS) & GSTS_ONE_SHOT_MASK;
    w32(base, GCMD, if on { status | bit } else { status & !bit });
}

/// **Install an all-absent root table on one unit, without turning translation on.** `Err` is a
/// unit this driver will not drive, with the reason the tour prints.
fn root_up(d: &Drhd) -> Result<Unit, &'static str> {
    if d.segment != 0 {
        return Err("not on PCI segment 0, the only configuration space this kernel reads");
    }
    let base = d.register_base;
    let cap = r64(base, CAP);
    if cap & CAP_SAGAW_48BIT == 0 {
        return Err(
            "no 48-bit/4-level second-level translation (CAP.SAGAW bit 2), which Vtd needs",
        );
    }
    let ecap = r64(base, ECAP);
    // ND: 0 means 4-bit ids and each step adds two bits, up to 16 at 6; 7 is reserved.
    let nd = (cap & CAP_ND_MASK).min(6);
    let unit = Unit {
        base,
        root: zeroed_page_frame("root table"),
        rwbf: cap & CAP_RWBF != 0,
        frcd: ((cap >> CAP_FRO_SHIFT) & CAP_FRO_MASK) << 4,
        iotlb: (((ecap >> ECAP_IRO_SHIFT) & ECAP_IRO_MASK) << 4) + 8,
        interrupt_remapping: ecap & ECAP_IR != 0,
        next_did: 1,
        last_did: (1u32 << (4 + 2 * nd)) - 1,
        reserved_devices: 0,
    };

    // Firmware may hand over with translation already on (a pre-boot DMA protection setting, or
    // a kexec). Linux turns it off before reprogramming (`init_dmars`), since a root pointer the
    // kernel did not build is not one it can reason about; so does this.
    if r32(base, GSTS) & GSTS_TES != 0 {
        global_command(base, GCMD_TE, false);
        wait_gsts(base, "translation disable", |v| v & GSTS_TES == 0);
    }

    w64(base, RTADDR, unit.root);
    global_command(base, GCMD_SRTP, true);
    wait_gsts(base, "set root table pointer", |v| v & GSTS_RTPS != 0);
    invalidate_all(&unit);
    Ok(unit)
}

/// **Bring every VT-d unit up, each translating the devices it owns** (milestone 594, provisional
/// number; VT-d 4.1 sections 3.16, 8.3 and 8.4). Three steps, in an order that is the point:
///
/// 1. Every unit gets an all-absent root table, with translation still off.
/// 2. Every device an RMRR names is attached, through [`crate::iommu::confine`] with no grant of
///    its own, to a domain that is exactly its reserved regions. So a USB controller's legacy
///    buffers and the GPU's stolen memory are mapped before anything could fault them.
/// 3. Translation turns on, unit by unit, and any protected memory region firmware left enabled
///    is switched off.
///
/// From then every device a running unit owns faults until `attach` writes its context entry
/// (or, for an RMRR device, anywhere outside its reserved regions). One `vt-d` line per unit.
pub fn init(dmar: &DmarUnits) {
    *DMAR.lock() = Some(*dmar);
    {
        let mut g = IOMMU.lock();
        assert!(
            g.iter().all(|s| matches!(s, Slot::Absent)),
            "IOMMU initialized twice"
        );
        for (i, d) in dmar.units().iter().enumerate() {
            g.slots[i] = match root_up(d) {
                Ok(u) => Slot::Up(u),
                Err(why) => Slot::Refused(why),
            };
        }
    }

    // Step 2. A device named by two scopes (two RMRRs, say) is confined once, with both regions:
    // `confine` asks `for_each_reserved_region` for all of them.
    let mut done = [0u32; MAX_RMRR_SCOPES];
    let mut done_count = 0;
    for s in &dmar.rmrr_scopes[..dmar.rmrr_scope_count] {
        let region = dmar.rmrrs[s.unit as usize];
        if region.segment != 0 || s.kind != SCOPE_PCI_ENDPOINT {
            continue; // see this module's BUGS on sub-hierarchy RMRR scopes
        }
        let Some((bus, dev, func)) = s.resolve(&mut crate::pci::bridge_bus_range) else {
            continue; // the path names hardware that is not on this machine
        };
        let rid = (bus as u32) << 8 | (dev as u32) << 3 | func as u32;
        if done[..done_count].contains(&rid) {
            continue;
        }
        done[done_count] = rid;
        done_count += 1;
        if let Some(i) = owner_index(rid) {
            crate::iommu::confine(rid, &[]);
            if let Slot::Up(u) = &mut IOMMU.lock().slots[i] {
                u.reserved_devices += 1;
            }
        }
    }

    // Step 3.
    let mut g = IOMMU.lock();
    for s in g.slots.iter_mut() {
        let Slot::Up(u) = s else { continue };
        global_command(u.base, GCMD_TE, true);
        wait_gsts(u.base, "translation enable", |v| v & GSTS_TES != 0);
        if r64(u.base, CAP) & (CAP_PLMR | CAP_PHMR) != 0 {
            w32(u.base, PMEN, r32(u.base, PMEN) & !PMEN_EPM);
            let mut spins = 0u32;
            while r32(u.base, PMEN) & PMEN_PRS != 0 {
                spins += 1;
                assert!(spins < 1_000_000, "IOMMU protected-memory disable hung");
            }
        }
    }
    let n = dmar.units().len();
    for (i, d) in dmar.units().iter().enumerate() {
        let what = if d.include_pci_all {
            "the catch-all"
        } else {
            "named devices only"
        };
        match &g.slots[i] {
            Slot::Up(u) => crate::println!(
                "  vt-d        : drhd {:#x} up ({} of {n}, {what}), translation enabled (gsts.tes \
                 confirmed), {} rmrr device(s) identity-mapped",
                d.register_base,
                i + 1,
                u.reserved_devices,
            ),
            Slot::Refused(why) => crate::println!(
                "  vt-d        : drhd {:#x} NOT up ({} of {n}, {what}): {why}; the devices it owns \
                 are not translated",
                d.register_base,
                i + 1,
            ),
            Slot::Absent => {}
        }
    }
}

/// The index of the unit that owns `rid`, when that unit is one this kernel could ask. `None` is
/// a device no unit owns, or a DMAR recorded only in part. Resolving a scope path reads
/// configuration space, so this runs with neither lock held across it.
fn owner_index(rid: u32) -> Option<usize> {
    let units = (*DMAR.lock())?;
    let (bus, dev, func) = ((rid >> 8) as u8, ((rid >> 3) & 0x1f) as u8, (rid & 7) as u8);
    match units.owner_index(0, bus, dev, func, &mut crate::pci::bridge_bus_range) {
        Ok(Some((i, _))) => Some(i),
        _ => None,
    }
}

/// **Every RMRR the firmware declared for requester `rid`**, as the regions a DMA domain for it
/// must also map (VT-d 4.1 section 3.16). [`crate::iommu::confine`] adds them to every domain it
/// builds, so confining a USB controller or the GPU never takes away memory the firmware still
/// DMAs into. Name: provisional (milestone 594).
pub fn for_each_reserved_region(rid: u32, each: &mut dyn FnMut(paging::domain::DmaRegion)) {
    let Some(units) = *DMAR.lock() else {
        return;
    };
    let (bus, dev, func) = ((rid >> 8) as u8, ((rid >> 3) & 0x1f) as u8, (rid & 7) as u8);
    units.reserved_for(
        0,
        bus,
        dev,
        func,
        &mut crate::pci::bridge_bus_range,
        &mut |r| {
            each(paging::domain::DmaRegion {
                base: r.base,
                size: r.size(),
            });
        },
    );
}

/// **Which unit, if any, translates requester id `rid`, and why?** (milestone 261 (the NVMe driver leaves the kernel)'s bench
/// rehearsal; VT-d 4.1 section 8.3.) Asked after the bus is up, because a scope path is resolved
/// through the live bridges' bus-number registers.
///
/// This is fatal risk 6's first night-of condition as code. A VT-d unit translates only the
/// requesters its DMAR scope gives it, so "translation is on" and "this device is confined" are
/// different claims. Since milestone 594 every unit is brought up, so the answer is `Owned`
/// whenever the device has an owner and that owner came up; `Elsewhere` now means a unit this
/// kernel refused (`owner: Some`) or no owner at all (`owner: None`).
pub fn scope_of(rid: u32) -> crate::iommu::Scope {
    use crate::iommu::Scope;
    // Copied out rather than held, so the config-space reads that resolve a path run with no
    // lock taken.
    let mut up = [false; MAX_DRHDS];
    let mut translating = None;
    for (i, s) in IOMMU.lock().iter().enumerate() {
        if let Slot::Up(u) = s {
            up[i] = true;
            translating.get_or_insert(u.base);
        }
    }
    let Some(translating) = translating else {
        return Scope::NoIommu;
    };
    let Some(units) = *DMAR.lock() else {
        return Scope::Unknown { translating };
    };
    let (bus, dev, func) = ((rid >> 8) as u8, ((rid >> 3) & 0x1f) as u8, (rid & 7) as u8);
    match units.owner_index(0, bus, dev, func, &mut crate::pci::bridge_bus_range) {
        Ok(Some((i, how))) if up[i] => Scope::Owned {
            unit: units.drhds[i].register_base,
            how,
        },
        Ok(Some((i, _))) => Scope::Elsewhere {
            translating,
            owner: Some(units.drhds[i].register_base),
        },
        Ok(None) => Scope::Elsewhere {
            translating,
            owner: None,
        },
        Err(()) => Scope::Unknown { translating },
    }
}

/// **Does this machine's VT-d offer interrupt remapping (`ECAP.IR`) on every unit that is up?**
/// `None` when no unit is up, which is a different answer from "a unit that says no". Every unit,
/// because remapping that one unit lacks leaves that unit's devices' interrupts unremapped.
///
/// Nothing in this kernel remaps an interrupt. This exists so the question is *askable* from
/// inside the guest, which is what milestone 317 is for: without it, turning
/// `-device intel-iommu,intremap=on` on and watching the suite stay green proves only that the
/// suite does not care. See design/roadmap/317-interrupt-remapping-flags.md.
// Two callers: `print_summary` (which a bench boot skips, the same treatment that function already
// carries) and this module's own test.
#[cfg_attr(feature = "bench", allow(dead_code))]
pub fn interrupt_remapping_available() -> Option<bool> {
    let g = IOMMU.lock();
    let mut any = false;
    let mut all = true;
    for s in g.iter() {
        if let Slot::Up(u) = s {
            any = true;
            all &= u.interrupt_remapping;
        }
    }
    any.then_some(all)
}

/// **This machine's IOMMU, for the machine description** (milestone 268).
///
/// One of the eight questions the description answers on every architecture. The vocabulary is this
/// architecture's, because that is what parity means here: the same question, answered in the terms
/// of the hardware that answers it. A machine with no IOMMU says so plainly rather than printing a
/// blank, because a blank is indistinguishable from a line nobody wrote. One line per unit.
// The machine description is the only caller, and it is
// `#[cfg(not(any(test, feature = "bench")))]`: a test boot exits through semihosting and a bench
// boot diverges into `bench::run`, so neither reads a bring-up transcript. Same treatment
// `memory::print_summary` already carries, and for the same reason.
#[cfg_attr(
    any(test, feature = "system_tests", feature = "bench"),
    allow(dead_code)
)]
pub fn print_summary() {
    let g = IOMMU.lock();
    let mut any = false;
    for s in g.iter() {
        match s {
            Slot::Up(u) => {
                any = true;
                crate::println!(
                    "  iommu           : VT-d drhd at {:#018x}, root table default-deny, translating, \
                     interrupt remapping {} (unused)",
                    u.base,
                    if u.interrupt_remapping {
                        "offered"
                    } else {
                        "absent"
                    },
                );
            }
            Slot::Refused(why) => {
                any = true;
                crate::println!("  iommu           : VT-d unit refused: {why}");
            }
            Slot::Absent => {}
        }
    }
    if !any {
        crate::println!(
            "  iommu           : none (this machine's ACPI names no DMAR, or it names no DRHD)",
        );
    }
}

/// Is any VT-d unit up? The portable seam asks this to decide whether attaching is possible.
pub fn is_active() -> bool {
    IOMMU.lock().iter().any(|s| matches!(s, Slot::Up(_)))
}

/// Register-based, global invalidation: the context cache first (a stale context entry would
/// still point translation at the previous domain), then the IOTLB, each a write-and-poll on its
/// own register. Both use global granularity; see this module's BUGS for the cost.
fn invalidate_all(s: &Unit) {
    if s.rwbf {
        global_command(s.base, GCMD_WBF, true);
        wait_gsts(s.base, "write buffer flush", |v| v & GSTS_WBFS == 0);
    }

    w64(s.base, CCMD, CCMD_ICC | CCMD_CIRG_GLOBAL);
    let mut spins = 0u32;
    while r64(s.base, CCMD) & CCMD_ICC != 0 {
        spins += 1;
        assert!(spins < 1_000_000, "IOMMU context-cache invalidate hung");
    }

    w64(s.base, s.iotlb, IOTLB_IVT | IOTLB_IIRG_GLOBAL);
    let mut spins = 0u32;
    while r64(s.base, s.iotlb) & IOTLB_IVT != 0 {
        spins += 1;
        assert!(spins < 1_000_000, "IOMMU IOTLB invalidate hung");
    }
}

/// Point device `rid` (the PCIe requester id: bus in bits 15:8, device/function in bits 7:0) at
/// the domain rooted at `root` (a [`Vtd`]-format table the portable seam built), **in the unit
/// the DMAR says owns it**, then invalidate that unit so it drops anything cached for the previous
/// state of this device.
///
/// `_tag` is the portable seam's cache tag (an ASID on aarch64, a PSCID on riscv64). VT-d's tag is
/// the domain id, and this unit hands out its own, because the requester id the seam passes does
/// not fit the domain-id width a real unit implements; see [`Unit::domain_id`].
///
/// **A device no running unit owns gets no context entry**, because there is no root table its
/// DMA would be looked up in: see this module's "A device no unit owns". [`scope_of`] is how a
/// caller learns that, and every caller that claims confinement asks it.
pub fn attach(rid: u32, root: u64, _tag: u16) {
    let Some(i) = owner_index(rid) else {
        return;
    };
    let mut g = IOMMU.lock();
    let Units { slots, ctx } = &mut *g;
    let Slot::Up(s) = &mut slots[i] else {
        return;
    };
    let ctx = &mut ctx[i];
    let did = s.domain_id();

    let bus = (rid >> 8) as usize & 0xff;
    let devfn = (rid & 0xff) as usize;

    let ctp = match ctx[bus] {
        Some(ctp) => ctp,
        None => {
            let ctp = zeroed_page_frame("context table");
            // Publish the (still all-absent) context table before the root entry that makes it
            // reachable, so the IOMMU can never walk to a root entry whose context table isn't
            // there yet.
            crate::arch::direct_memory_access_write_barrier();
            let root_entry = phys_to_virt(s.root + bus as u64 * 16) as *mut u64;
            // SAFETY: `s.root` is a kernel-owned page-aligned frame; `bus` is masked to 0..256,
            // which is exactly the 256 entries a one-page root table holds.
            unsafe {
                core::ptr::write_volatile(root_entry, (ctp & ROOT_ENTRY_CTP_MASK) | ROOT_ENTRY_P);
                core::ptr::write_volatile(root_entry.add(1), 0); // upper qword: reserved, legacy mode
            }
            ctx[bus] = Some(ctp);
            ctp
        }
    };

    // The context entry, written back to front: the second-level root and the domain id first,
    // the present bit last, with a barrier between, so the IOMMU can never observe a present
    // entry with a stale second-level root. Same ordering discipline the RISC-V driver's
    // `attach` uses for its device context.
    let ctx_entry = phys_to_virt(ctp + devfn as u64 * 16) as *mut u64;
    let hi = CTX_AW_48BIT | ((did as u64) << CTX_DID_SHIFT);
    let lo = (root & CTX_SLPTPTR_MASK) | CTX_TT_MULTI_LEVEL;
    // SAFETY: `ctp` is a kernel-owned page-aligned frame; `devfn` is masked to 0..256, which is
    // exactly the 256 entries a one-page context table holds.
    unsafe {
        core::ptr::write_volatile(ctx_entry.add(1), hi);
        core::ptr::write_volatile(ctx_entry, lo);
    }
    crate::arch::direct_memory_access_write_barrier();
    // SAFETY: as above.
    unsafe {
        core::ptr::write_volatile(ctx_entry, lo | CTX_ENTRY_P);
    }
    crate::arch::direct_memory_access_write_barrier();

    invalidate_all(s);
}

/// Pop one fault, if any unit holds one: `FSTS.PPF` says at least one Fault Recording Register
/// holds an unprocessed record, and (with `CAP.NFR` reporting one register on every unit this
/// driver has met) the first is the only one read. The confinement test drains this to prove a
/// DMA escape was stopped by the hardware, not merely absent.
#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
pub fn take_fault() -> Option<Fault> {
    let g = IOMMU.lock();
    for slot in g.iter() {
        let Slot::Up(s) = slot else { continue };
        // **An overflow blinds the unit until it is cleared** (found 2026-09-27 by milestone 609 (the
        // system tests leave the kernel crate), which changed which tests share a boot). A device
        // still running a queue from an earlier registration faulted until its one record was full;
        // QEMU then set PFO and dropped every later fault, including the one the confinement test
        // provoked, so the test read a stale record. Clearing PFO here makes the next fault land.
        if r32(s.base, FSTS) & FSTS_PFO != 0 {
            w32(s.base, FSTS, FSTS_PFO);
        }
        if r32(s.base, FSTS) & FSTS_PPF == 0 {
            continue;
        }
        let lo = r64(s.base, s.frcd);
        let hi = r64(s.base, s.frcd + 8);
        if hi & (1 << 63) == 0 {
            // PPF was set but this record's own F bit is not: nothing to report from the one
            // register this driver reads. See this module's BUGS on why only one is ever read.
            continue;
        }
        // Write-1-to-clear: writing back exactly what was read clears the F bit (bit 63 of `hi`)
        // and leaves every other field, which the hardware overwrites on the next fault anyway.
        w64(s.base, s.frcd + 8, hi);
        return Some(Fault {
            rid: (hi & 0xffff) as u32,
            code: ((hi >> 32) & 0xff) as u32,
            addr: lo & !0xfff,
        });
    }
    None
}

// A compile-time check that this module and `Vtd` agree on the level count `CTX_AW_48BIT`
// promises the hardware: four levels, the same the context entry's AW field selects.
const _: () = assert!(Vtd::LEVELS == 4);

#[cfg(test)]
mod tests {
    //! Milestone 317's half of the interrupt-remapping question, which is the half a boot can
    //! answer. The other half (does a remapped interrupt land where an `IRTE` says it should)
    //! needs a driver that programs one, and this tree has none.

    use super::*;

    /// **The unit's interrupt-remapping capability is reported, and remapping is still off.**
    ///
    /// Two assertions, and the second is the one with teeth. The first says the machine answered
    /// at all: a `None` here on a runner that always attaches `-device intel-iommu` would mean the
    /// DRHD was never found, which every other VT-d test would also fail on, so it is a guard
    /// rather than a claim.
    ///
    /// The second states a limit, the shape notes/confinement-claims.md asks for: whatever
    /// `ECAP.IR` says, `GSTS.IRES` is clear, because this driver never sets `GCMD.IRE`. **That is
    /// what keeps `NIFE_INTREMAP` honest.** Booting a machine that offers remapping and watching
    /// the suite stay green would otherwise prove nothing; this fails the day somebody enables
    /// remapping without also retiring the claim in
    /// design/roadmap/317-interrupt-remapping-flags.md that nothing in this kernel remaps an
    /// interrupt.
    ///
    /// It runs identically with the flag and without it, on purpose. A test that only ran under
    /// the flag would be a test nobody runs.
    #[test_case]
    fn interrupt_remapping_is_reported_and_never_enabled() {
        let Some(offered) = interrupt_remapping_available() else {
            crate::testing::skip!("this machine's ACPI names no DMAR, so there is no unit to ask");
        };

        for slot in IOMMU.lock().iter() {
            let Slot::Up(u) = slot else { continue };
            assert_eq!(
                r32(u.base, GSTS) & GSTS_IRES,
                0,
                "GSTS.IRES is set on drhd {:#x}: something enabled interrupt remapping (ECAP.IR \
                 was {offered}), and the claim that this kernel never remaps an interrupt is now \
                 false",
                u.base,
            );
        }
    }

    /// **Every unit the DMAR names is translating** (milestone 594). Before it, one unit came up
    /// and the rest were left off by design; this fails if a unit is silently skipped again. A
    /// refused unit fails it too, which is right under QEMU, whose one unit has nothing to refuse.
    #[test_case]
    fn every_unit_the_dmar_names_is_translating() {
        let Some(units) = *DMAR.lock() else {
            crate::testing::skip!("this machine's ACPI names no DMAR, so there is no unit to ask");
        };
        let g = IOMMU.lock();
        for (i, d) in units.units().iter().enumerate() {
            match &g.slots[i] {
                Slot::Up(u) => assert_ne!(
                    r32(u.base, GSTS) & GSTS_TES,
                    0,
                    "drhd {:#x} is up but GSTS.TES reads clear",
                    u.base
                ),
                Slot::Refused(why) => panic!("drhd {:#x} was refused: {why}", d.register_base),
                Slot::Absent => panic!("drhd {:#x} was never brought up", d.register_base),
            }
        }
    }

    /// **A device no unit owns gets no context entry anywhere** (milestone 594's decision, from
    /// Linux's `-ENODEV`). Before it, `attach` wrote into the one running unit's tables whatever
    /// requester it was handed, so a domain could be "attached" in a table the device's DMA is
    /// never looked up in. Bus 0xfe is empty on every runner; on a DMAR with a catch-all it is
    /// owned all the same, and there is nothing to prove.
    #[test_case]
    fn a_device_no_unit_owns_gets_no_context_entry() {
        let rid = 0xfe << 8;
        match scope_of(rid) {
            crate::iommu::Scope::Elsewhere { owner: None, .. } => {}
            crate::iommu::Scope::NoIommu => {
                crate::testing::skip!("no VT-d unit is up on this machine")
            }
            _ => crate::testing::skip!("bus 0xfe has an owner on this DMAR (a catch-all takes it)"),
        }
        let root = zeroed_page_frame("test domain root");
        attach(rid, root, 0);
        let g = IOMMU.lock();
        for (i, slot) in g.iter().enumerate() {
            let Slot::Up(u) = slot else { continue };
            assert!(
                g.ctx[i][0xfe].is_none(),
                "drhd {:#x} grew a context table for a bus no unit owns",
                u.base
            );
        }
    }
}
