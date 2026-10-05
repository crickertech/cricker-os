//! **The AMD-Vi driver: AMD's IOMMU, in front of the PCIe bus** (lane `amd-vi`; module name and
//! milestone number provisional).
//!
//! VT-d's counterpart on an AMD machine, behind the same five calls the portable seam makes
//! (`attach`, `scope_of`, `is_active`, `take_fault`, `for_each_reserved_region`), which
//! [`super::iommu`] hands over whenever a unit here is up. Every register, table and command
//! format below is from AMD document 48882 revision 2.62 (February 2015), read for this driver.
//! Where a choice rests on what QEMU's model does (`hw/i386/amd_iommu.c`, read at QEMU 11.1.1),
//! the comment says so, because QEMU is the only AMD-Vi this has run on and it diverges from the
//! specification in four places this driver had to know about (see BUGS).
//!
//! # The shape, and how it differs from VT-d
//!
//! VT-d is driven through registers. **AMD-Vi is driven through memory**, the way the SMMUv3 and
//! the RISC-V IOMMU are: one device table the unit reads (one 256-bit entry per 16-bit device id,
//! no per-bus level), one command ring the driver writes and the unit consumes, and one event log
//! the unit writes and the driver drains. Invalidation is a command (`INVALIDATE_DEVTAB_ENTRY`,
//! `INVALIDATE_IOMMU_PAGES`) followed by a `COMPLETION_WAIT` whose store the driver polls for, and
//! a fault is a 128-bit record in the event log rather than a fault-recording register.
//!
//! **No cache maintenance is needed, and that is the specification's answer, not an assumption.**
//! The device table read is snooped while `Control.Coherent` is 1 (its reset value, section 3.4,
//! which this driver never clears); page-table walks are snooped while a device table entry's `SD`
//! bit is 0 (Table 7), which every entry here leaves clear; and the command ring is always read
//! coherently (section 2.4). So the `clflush` VT-d's [`Unit::publish`](super::iommu) needs on a
//! unit with `ECAP.C` clear has no counterpart here.
//!
//! # Default deny, and what quarantine means here
//!
//! `init` fills every device table entry with [`blocked_dte`] before the unit is enabled: valid,
//! translating, four levels, rooted at one all-zero table, with read and write permission both
//! clear. Every DMA from a device nobody confined walks to a not-present entry and is
//! target-aborted and logged as an `IO_PAGE_FAULT`. A device id past the end of the table is
//! target-aborted by the hardware itself (section 2.2.2). So translation turns on over a
//! deny-everything table, the same property the other three drivers' `init` establishes.
//!
//! **That blocked entry is also §163 (where a confined device's IOMMU fault is delivered)'s
//! quarantine**: the entry a faulted driver's device is
//! switched to so it cannot keep issuing DMA, as VT-d clears `CTX_ENTRY_P`. [`quarantine`] writes
//! it; milestone 102 (what a confined device's fault reaches) is the caller that does not exist
//! yet on any architecture.
//!
//! **Why not the simpler "translation disabled, no permission" entry** (`Mode` 0, `IR` = `IW` =
//! 0), which the specification says blocks everything (Table 7: with `Mode` 0, "access controlled
//! by IR and IW")? Because QEMU does not implement it: its model treats every `Mode` 0 entry as
//! pass-through and ignores `IR` and `IW` (`amdvi_update_addr_translation_mode`, then
//! `enable_nodma_mode`). An entry that blocks on silicon and passes everything under QEMU would be
//! a default-deny claim no test here could check. The four-level walk to an empty table blocks on
//! both, and logs on both, which is what a test needs to see.
//!
//! # BUGS
//!
//! - **QEMU confines nothing unless the unit is created with `dma-remap=on`**, and nothing in the
//!   guest can tell. Without it QEMU routes every device's DMA around the unit whatever the device
//!   table says (`amdvi_switch_address_space`); `helpers/qemu-runner-x86_64.sh` sets it. Silicon
//!   has no such switch.
//! - **QEMU switches a device to translated DMA only when `INVALIDATE_DEVTAB_ENTRY` names it.**
//!   Until then its DMA passes through untranslated, however the table reads. So `init` invalidates
//!   every entry in the table one command at a time, which is also what Linux does
//!   (`iommu_flush_dte_all`, from memory, not re-read). A device id past the table's end is never
//!   named, and under QEMU passes through rather than being target-aborted as section 2.2.2 says.
//!   QEMU lists every device it has in the IVRS, so no device it models is in that position.
//! - **QEMU 11.1.1 writes no address into any event record.** `amdvi_setevent_bits` builds the
//!   address field's mask with a shift of 64, which is undefined in C and comes out zero in the
//!   Homebrew build this tree runs (QEMU's own trace shows the right address on the same fault).
//!   Fixed upstream by commit 4adfb431c0 (2026-08-14), which is in no release as of 2026-10-05,
//!   11.1.2 included. So [`take_fault`] reports a zero address as unknown (`Fault::addr` is
//!   `None`), and the DMA-escape tests tie such a fault to their device by requester id; the NVMe
//!   test's canary is what still proves its escape did not land. Silicon fills the field.
//! - **QEMU's `IO_PAGE_FAULT` record carries the device's `devfn` alone, not its bus**, sets the
//!   `I` (interrupt) bit on a memory fault, and leaves the domain id zero (`amdvi_page_fault`). A
//!   fault from a device off bus 0 would be reported against the wrong requester id. Every device
//!   QEMU's `q35` runner attaches is on bus 0.
//! - **There is a window at enable where a stale cached entry could still be used.** The unit
//!   reads commands only once `IommuEn` is set, so the invalidations that empty firmware's caches
//!   can only run after the unit is translating with whatever it cached. Linux has the same window.
//!   Firmware that hands over with the unit enabled is unseen so far.
//! - **Interrupt remapping is part of the base architecture and is never enabled.** Every device
//!   table entry here leaves `IV` clear, so interrupts pass through untranslated (Table 9), the
//!   same posture as VT-d's driver. Whether to remap is design/roadmap/317's question, not this
//!   driver's.
//! - **No fault interrupt.** The event log is drained by [`take_fault`], which only tests call,
//!   exactly as VT-d's fault-recording register is. Milestone 102 owns the interrupt.
//! - **An IVMD that names every device (type 20h) is added to every domain `confine` builds, but
//!   no device is pre-attached for it**, because pre-attaching all 65,536 ids would be 65,536
//!   domains. A device the firmware is still DMA-ing for stays blocked until its driver confines
//!   it. No AMD machine this tree has met publishes an IVMD of any type (QEMU writes none), so the
//!   whole IVMD path has run zero times.
//! - **Domain ids are never reused.** Each attach takes the next one, and a unit has 65,535; a
//!   boot that confined more devices than that would panic. Every change to a device's entry also
//!   flushes the domain the device leaves (`Unit::set_entry` says why), which for the blocked
//!   domain 0 empties every unconfined device's cached faults along with it; a cache miss, not a
//!   correctness cost.
//!
//! The five entries below were found by milestone 633 (an outside agent attacks the confinement
//! claim)'s second pass (2026-10-05 UTC), reading this driver as a confinement boundary with no row
//! in `notes/confinement-claims.md`. All are reasoned from the code and the specification; none has
//! been booted, because QEMU models none of the firmware state they depend on. The first three are
//! the acceptance items of design/roadmap/767-amd-vi-hardening-before-the-first-amd-boot.md.
//!
//! - **The firmware's exclusion range is never cleared.** `set_up` keeps every `Control` bit it does
//!   not explicitly clear, and nothing writes the Exclusion Base and Limit registers (`0x0020`,
//!   `0x0028`). On silicon a firmware-set range with `ExEn`, and above all with `Allow`, lets every
//!   device reach that range untranslated whatever its entry says. It is the one place a device can
//!   pass through this driver's default deny, and the guest cannot see it under QEMU.
//! - **Alias entries are shared between devices and are not quarantined.** [`attach`] writes one
//!   translating entry under both the requester id and its alias source id. Two functions behind
//!   one PCIe-to-PCI bridge share a source id, so the second attach moves the first device's aliased
//!   DMA into the second's domain, and [`quarantine`] resets only the requester id, leaving the
//!   alias translating. Every device QEMU's `q35` attaches is on bus 0 with no alias, so no boot has
//!   reached this.
//! - **Every DMA mapping is read-write.** `build_identity_domain` maps with `Flags::user_data()`, so
//!   a firmware IVMD marked read-only is writable to the device (the IVRS parser's own BUGS admits
//!   the bit is dropped), and the virtio shadow page, which is kernel-private by design, is mapped
//!   writable to the device it shadows.
//! - **Nothing revokes a device's domain in production.** [`quarantine`] is called only by this
//!   module's tests; `confine` has no inverse at the seam, so a device keeps its reach for the
//!   whole boot after its driver dies, and a re-attach leaks the previous domain's tables
//!   (`kernel/src/iommu.rs` records the leak).
//! - **The entry builders have no literal permitted-bits guard and no proof.** [`translating_dte`]
//!   masks the root with `DTE_ROOT_MASK` silently rather than asserting it fits, never narrows to
//!   the unit's real address width, and the only check is a `#[test_case]` over two concrete
//!   values that looks at bits 6:2 and 63. VT-d's equivalent, `VTD_PERMITTED_BITS`, is proved over
//!   every `u64` by `no_vtd_entry_ever_sets_a_reserved_bit`; the AMD-Vi leaf and directory masks
//!   in `crates/paging` are checked over a few addresses by host tests only.

use machine_discovery::acpi::ivrs::{IvrsUnits, MAX_IVHDS};

use crate::arch::mmu::phys_to_virt;
use crate::sync::{IrqSafeMutex, rank};

// --- MMIO register offsets (section 3.4). ---
const DEVICE_TABLE_BASE: u64 = 0x0000;
const COMMAND_BUFFER_BASE: u64 = 0x0008;
const EVENT_LOG_BASE: u64 = 0x0010;
const CONTROL: u64 = 0x0018;
const EXTENDED_FEATURES: u64 = 0x0030;
const COMMAND_HEAD: u64 = 0x2000;
const COMMAND_TAIL: u64 = 0x2008;
const EVENT_HEAD: u64 = 0x2010;
const EVENT_TAIL: u64 = 0x2018;
const STATUS: u64 = 0x2020;

/// **How much of the register file this driver maps**: 16 KiB, which covers every register above
/// (the highest is `STATUS`, at `0x2020`). A unit with performance counters decodes up to 512 KiB
/// (`0x80000`); nothing here reads them. QEMU's model is exactly 16 KiB (`AMDVI_MMIO_SIZE`).
pub const REGISTER_SIZE: u64 = 0x4000;

// Control register bits this driver sets or clears (section 3.4.3).
const CONTROL_IOMMU_EN: u64 = 1 << 0;
const CONTROL_EVENT_LOG_EN: u64 = 1 << 2;
const CONTROL_EVENT_INT_EN: u64 = 1 << 3;
const CONTROL_COM_WAIT_INT_EN: u64 = 1 << 4;
const CONTROL_CMD_BUF_EN: u64 = 1 << 12;

// Status register bits (section 3.4.x, MMIO 2020h).
const STATUS_EVENT_OVERFLOW: u64 = 1 << 0;
const STATUS_EVENT_LOG_RUN: u64 = 1 << 3;
const STATUS_CMD_BUF_RUN: u64 = 1 << 4;

// Extended Feature Register (MMIO 0030h): `IASup` is bit 6, `HATS` bits 11:10.
const EFR_IA_SUP: u64 = 1 << 6;
const EFR_HATS_SHIFT: u64 = 10;
/// `HATS` = 11b: QEMU writes it when host translation is off (`dma-translation=off`), and revision
/// 2.62 calls it reserved. Either way the unit cannot walk the four levels a domain here has.
const EFR_HATS_NONE: u64 = 0b11;

/// Both rings are one page: 256 entries of 16 bytes, the smallest length the hardware takes
/// (`ComLen` and `EventLen` = 1000b).
const RING_BYTES: u64 = 4096;
const RING_LEN_256: u64 = 0b1000 << 56;

// --- Device table entry, 256 bits as four qwords (Table 7). ---
const DTE_V: u64 = 1 << 0;
const DTE_TV: u64 = 1 << 1;
/// `Mode` (bits 11:9) = 4: a four-level host page table, the 48-bit walk `AmdVi` builds.
const DTE_MODE_4_LEVEL: u64 = 4 << 9;
const DTE_ROOT_MASK: u64 = 0x000f_ffff_ffff_f000;
const DTE_IR: u64 = 1 << 61;
const DTE_IW: u64 = 1 << 62;
/// `IV`, bit 128: the interrupt-remapping half of the entry is valid. Never set here.
#[cfg(test)]
const DTE_IV: u64 = 1 << 0;
const DTE_BYTES: u64 = 32;

// Command opcodes, bits 63:60 of the first qword (section 2.4).
const CMD_COMPLETION_WAIT: u64 = 0x1 << 60;
const CMD_INVALIDATE_DEVTAB_ENTRY: u64 = 0x2 << 60;
const CMD_INVALIDATE_IOMMU_PAGES: u64 = 0x3 << 60;
const CMD_INVALIDATE_IOMMU_ALL: u64 = 0x8 << 60;
/// `COMPLETION_WAIT`'s `s` bit: store the second qword at the address in the first.
const COMPLETION_WAIT_STORE: u64 = 1 << 0;
/// `INVALIDATE_IOMMU_PAGES` over a whole domain: `S` and `PDE` set, address `7_FFFF_FFFF_FFFFh`
/// (section 2.4.3's software note says exactly this invalidates everything for the domain).
const INVALIDATE_WHOLE_DOMAIN: u64 = 0x7fff_ffff_ffff_f000 | 0b011;

/// **The device table entry of a device that is confined**: valid, translating through the
/// four-level table at `root`, read and write allowed so the leaves decide, tagged `domain`.
/// Everything else zero: no interrupt remapping (`IV` clear), no IOTLB, events not suppressed.
const fn translating_dte(root: u64, domain: u16) -> [u64; 4] {
    [
        DTE_V | DTE_TV | DTE_MODE_4_LEVEL | (root & DTE_ROOT_MASK) | DTE_IR | DTE_IW,
        domain as u64,
        0,
        0,
    ]
}

/// **The device table entry that denies everything**: the default for every device and §163's
/// quarantine. A four-level walk to `empty_root`, an all-zero table, with read and write both
/// clear; every access target-aborts and is logged. This module's header says why it is not
/// `Mode` 0. Domain 0, which no confined device is given, because every blocked device shares the
/// one root and the specification asks that devices sharing a domain id share their tables.
const fn blocked_dte(empty_root: u64) -> [u64; 4] {
    [
        DTE_V | DTE_TV | DTE_MODE_4_LEVEL | (empty_root & DTE_ROOT_MASK),
        0,
        0,
        0,
    ]
}

/// One fault, in the portable shape VT-d's driver returns.
pub use super::iommu::Fault;

/// One AMD-Vi unit this kernel brought up.
struct Unit {
    base: u64,
    /// The device table: `entries` entries of 32 bytes, physically contiguous.
    device_table: u64,
    entries: u32,
    /// The empty top-level table every blocked entry points at.
    empty_root: u64,
    command_ring: u64,
    /// The offset of the next command slot, as the tail register holds it.
    command_tail: u64,
    event_log: u64,
    /// One 8-byte word the unit writes on `COMPLETION_WAIT`, and the value last asked for.
    semaphore: u64,
    sequence: u64,
    /// `EFR.IASup`: the unit takes `INVALIDATE_IOMMU_ALL`.
    invalidate_all: bool,
    next_domain: u32,
    /// Set once `IommuEn` is: before that the unit reads no commands, so `attach` writes the
    /// entry alone and `init`'s flush covers it.
    enabled: bool,
    /// How many devices an IVMD had attached before the unit was enabled.
    reserved_devices: u32,
}

enum Slot {
    Absent,
    Up(Unit),
    Refused(&'static str),
}

static UNITS: IrqSafeMutex<[Slot; MAX_IVHDS]> =
    IrqSafeMutex::new(rank::IOMMU, [const { Slot::Absent }; MAX_IVHDS]);

/// The IVRS as [`init`] read it, kept so `attach` can route a device to its unit. Never held at
/// the same time as [`UNITS`].
static IVRS: IrqSafeMutex<Option<IvrsUnits>> = IrqSafeMutex::new(rank::IOMMU, None);

fn r64(base: u64, off: u64) -> u64 {
    // SAFETY: the unit's register file lies inside the direct map, mapped device-typed by
    // `mmu::map_everything` from `memory::amd_vi_regions`; these are aligned 64-bit registers.
    unsafe { core::ptr::read_volatile(phys_to_virt(base + off) as *const u64) }
}
fn w64(base: u64, off: u64, v: u64) {
    // SAFETY: as above.
    unsafe { core::ptr::write_volatile(phys_to_virt(base + off) as *mut u64, v) }
}

/// Write one device table entry, upper qwords first and the one carrying `V` last, so a unit
/// that reads it between the stores sees the old first qword with new upper halves, never a new
/// first qword with stale ones.
fn write_dte(table: u64, id: u32, dte: [u64; 4]) {
    let at = phys_to_virt(table + id as u64 * DTE_BYTES) as *mut u64;
    // SAFETY: `table` is a kernel-owned contiguous run of `entries` entries and every caller
    // checks `id < entries`; the direct map reaches it.
    unsafe {
        for i in (1..4).rev() {
            core::ptr::write_volatile(at.add(i), dte[i]);
        }
        crate::arch::direct_memory_access_write_barrier();
        core::ptr::write_volatile(at, dte[0]);
    }
}

fn read_dte(table: u64, id: u32) -> [u64; 4] {
    let at = phys_to_virt(table + id as u64 * DTE_BYTES) as *const u64;
    // SAFETY: as in `write_dte`; read only.
    unsafe { core::array::from_fn(|i| core::ptr::read_volatile(at.add(i))) }
}

fn spin_until(what: &str, mut done: impl FnMut() -> bool) {
    for _ in 0..10_000_000u32 {
        if done() {
            return;
        }
        core::hint::spin_loop();
    }
    panic!("AMD-Vi {what} never completed");
}

impl Unit {
    /// Queue one command. Waits for a free slot first: the ring is full when advancing the tail
    /// would make it equal the head.
    fn command(&mut self, first: u64, second: u64) {
        let next = (self.command_tail + 16) % RING_BYTES;
        let base = self.base;
        spin_until("command ring drain", || {
            r64(base, COMMAND_HEAD) & 0x7fff0 != next
        });
        let at = phys_to_virt(self.command_ring + self.command_tail) as *mut u64;
        // SAFETY: `command_ring` is a kernel-owned page and `command_tail` a 16-byte-aligned
        // offset inside it; the unit does not read a slot until the tail register passes it.
        unsafe {
            core::ptr::write_volatile(at, first);
            core::ptr::write_volatile(at.add(1), second);
        }
        // The command is in memory before the tail write that tells the unit to read it.
        crate::arch::direct_memory_access_write_barrier();
        self.command_tail = next;
        w64(self.base, COMMAND_TAIL, next);
    }

    /// **Wait until every command queued so far has finished** (section 2.4.1): a
    /// `COMPLETION_WAIT` that stores a fresh sequence number, then a bounded poll for it.
    fn sync(&mut self) {
        self.sequence += 1;
        let seq = self.sequence;
        self.command(
            CMD_COMPLETION_WAIT | (self.semaphore & 0x000f_ffff_ffff_fff8) | COMPLETION_WAIT_STORE,
            seq,
        );
        let at = phys_to_virt(self.semaphore) as *const u64;
        let base = self.base;
        spin_until("completion wait", || {
            // SAFETY: `semaphore` is a kernel-owned word the unit writes; read only.
            unsafe { core::ptr::read_volatile(at) == seq }
        });
        // A halted command processor (an ILLEGAL_COMMAND_ERROR) never stores, so a timeout above
        // panics; this checks the opposite surprise, a store with the processor stopped.
        assert_ne!(r64(base, STATUS) & STATUS_CMD_BUF_RUN, 0);
    }

    fn invalidate_entry(&mut self, id: u32) {
        self.command(CMD_INVALIDATE_DEVTAB_ENTRY | id as u64, 0);
    }

    fn invalidate_domain(&mut self, domain: u16) {
        self.command(
            CMD_INVALIDATE_IOMMU_PAGES | ((domain as u64) << 32),
            INVALIDATE_WHOLE_DOMAIN,
        );
    }

    fn domain(&mut self) -> u16 {
        assert!(
            self.next_domain <= 0xffff,
            "AMD-Vi unit {:#x} has handed out every domain id",
            self.base
        );
        let d = self.next_domain as u16;
        self.next_domain += 1;
        d
    }

    /// Point device table entry `id` at `dte` and, once the unit reads commands, make it forget
    /// both the entry and every translation it cached under the entry's **previous** domain. The
    /// caller syncs.
    ///
    /// **The second half was found by the DMA-escape test, not by reading.** Section 2.4.2 says
    /// `INVALIDATE_DEVTAB_ENTRY` "does not invalidate translation cache entries, since they may be
    /// in use by other devices sharing the same `DomainID`", and that software should invalidate the
    /// domain when it is not shared. Without that, QEMU's model (whose IOTLB is keyed by device
    /// id and page, not by domain: `amdvi_iotlb_lookup`) kept serving a virtio disk the read and
    /// write mapping an earlier test's domain had given it for a frame the allocator then handed
    /// out as the escape test's victim: the device read the victim through the stale entry, and
    /// the only fault logged was its next access, at address 0. Silicon tags its caches by domain
    /// id, so the stale entry would be unreachable there, but the old domain's entries would sit
    /// in the cache until evicted; invalidating them is right on both.
    fn set_entry(&mut self, id: u32, dte: [u64; 4]) {
        if id >= self.entries {
            return;
        }
        let previous = read_dte(self.device_table, id)[1] as u16;
        write_dte(self.device_table, id, dte);
        if self.enabled {
            self.invalidate_entry(id);
            self.invalidate_domain(previous);
        }
    }
}

fn zeroed_frames(count: usize, what: &str) -> u64 {
    crate::memory::alloc_contiguous_zeroed(count)
        .unwrap_or_else(|| panic!("no {count} frame(s) for the AMD-Vi {what}"))
        .addr()
}

/// **Give one unit an all-blocked device table, a command ring and an event log**, with the unit
/// still disabled. `Err` is a unit this driver will not drive, with the reason the boot prints.
fn set_up(ivrs: &IvrsUnits, i: usize) -> Result<Unit, &'static str> {
    let d = ivrs.units()[i];
    if d.segment != 0 {
        return Err("not on PCI segment 0, the only configuration space this kernel reads");
    }
    let base = d.register_base;
    // The EFR is defined only when the unit says it has one (`IVinfo.EFRSup`, mirroring the
    // capability header's bit); without it, read nothing and assume the 2.62 baseline.
    let efr = if ivrs.ivinfo & 1 != 0 {
        r64(base, EXTENDED_FEATURES)
    } else {
        0
    };
    if (efr >> EFR_HATS_SHIFT) & 0b11 == EFR_HATS_NONE {
        return Err("no host address translation (EFR.HATS = 11b)");
    }

    // Firmware may hand over with the unit on. Turn it off before pointing it anywhere new, the
    // same reasoning VT-d's `root_up` gives for clearing `TE`; the base registers may not be
    // written while their rings run (sections 3.4.2 and 3.4.3).
    let control = r64(base, CONTROL);
    let running = CONTROL_IOMMU_EN
        | CONTROL_EVENT_LOG_EN
        | CONTROL_CMD_BUF_EN
        | CONTROL_EVENT_INT_EN
        | CONTROL_COM_WAIT_INT_EN;
    if control & running != 0 {
        w64(base, CONTROL, control & !running);
        spin_until("disable", || {
            r64(base, STATUS) & (STATUS_CMD_BUF_RUN | STATUS_EVENT_LOG_RUN) == 0
        });
    }

    // The device table covers every id the IVRS gives this unit, rounded up to a whole page; the
    // hardware target-aborts any id past its end (section 2.2.2).
    let ids = ivrs.last_device_id(i) as u64 + 1;
    let pages = (ids * DTE_BYTES).div_ceil(page_frames::FRAME_SIZE);
    let entries = (pages * page_frames::FRAME_SIZE / DTE_BYTES) as u32;
    let device_table = zeroed_frames(pages as usize, "device table");
    let empty_root = zeroed_frames(1, "blocked domain root");
    for id in 0..entries {
        write_dte(device_table, id, blocked_dte(empty_root));
    }
    let command_ring = zeroed_frames(1, "command ring");
    let event_log = zeroed_frames(1, "event log");
    let semaphore = zeroed_frames(1, "completion word");
    crate::arch::direct_memory_access_write_barrier();

    // `Size` is pages minus one (section 3.4.1). Writing a ring's base resets its head and tail
    // (section 2.4; QEMU does the same in `amdvi_handle_cmdbase_write`).
    w64(base, DEVICE_TABLE_BASE, device_table | (pages - 1));
    w64(base, COMMAND_BUFFER_BASE, command_ring | RING_LEN_256);
    w64(base, COMMAND_HEAD, 0);
    w64(base, COMMAND_TAIL, 0);
    w64(base, EVENT_LOG_BASE, event_log | RING_LEN_256);
    w64(base, EVENT_HEAD, 0);
    w64(base, EVENT_TAIL, 0);

    Ok(Unit {
        base,
        device_table,
        entries,
        empty_root,
        command_ring,
        command_tail: 0,
        event_log,
        semaphore,
        sequence: 0,
        invalidate_all: efr & EFR_IA_SUP != 0,
        // Domain 0 is the blocked domain every unconfined device shares.
        next_domain: 1,
        enabled: false,
        reserved_devices: 0,
    })
}

/// **Turn one unit on and empty its caches.** The command ring and the event log first, then the
/// unit, as Linux orders it (`early_enable_iommu`, from memory); the unit reads no command until
/// `IommuEn` is set, so the flush comes after.
fn enable(u: &mut Unit) {
    let control = r64(u.base, CONTROL) | CONTROL_CMD_BUF_EN | CONTROL_EVENT_LOG_EN;
    w64(u.base, CONTROL, control);
    w64(u.base, CONTROL, control | CONTROL_IOMMU_EN);
    let base = u.base;
    spin_until("enable", || {
        r64(base, STATUS) & (STATUS_CMD_BUF_RUN | STATUS_EVENT_LOG_RUN)
            == STATUS_CMD_BUF_RUN | STATUS_EVENT_LOG_RUN
    });
    u.enabled = true;

    // Every entry, by name: the specification's way to make the unit reload one, and the only
    // way QEMU's model turns a device's translation on at all (this module's BUGS).
    for id in 0..u.entries {
        u.invalidate_entry(id);
    }
    if u.invalidate_all {
        u.command(CMD_INVALIDATE_IOMMU_ALL, 0);
    } else {
        // Without `INVALIDATE_IOMMU_ALL`, every domain id firmware might have cached under.
        for domain in 0..=0xffffu16 {
            u.invalidate_domain(domain);
        }
    }
    u.sync();
}

/// **Bring every AMD-Vi unit the IVRS names up, each translating the devices it serves.** Three
/// steps, in the order VT-d's `init` takes and for the same reason:
///
/// 1. Every unit gets an all-blocked device table and its rings, still disabled.
/// 2. Every device an IVMD names (types 21h and 22h) is confined, through
///    [`crate::iommu::confine`] with no grant of its own, to exactly its reserved regions.
/// 3. Each unit is enabled and its caches emptied.
///
/// One `amd-vi` line per unit.
pub fn init(ivrs: &IvrsUnits) {
    *IVRS.lock() = Some(*ivrs);
    {
        let mut g = UNITS.lock();
        assert!(
            g.iter().all(|s| matches!(s, Slot::Absent)),
            "AMD-Vi initialized twice"
        );
        for i in 0..ivrs.units().len() {
            g[i] = match set_up(ivrs, i) {
                Ok(u) => Slot::Up(u),
                Err(why) => Slot::Refused(why),
            };
        }
    }

    // Step 2. `confine` asks `for_each_reserved_region` for every region that names the device,
    // so a device two IVMDs name is confined once with both.
    let mut done = 0u32;
    for r in ivrs.unity_regions() {
        if r.first == 0 && r.last == 0xffff {
            continue; // type 20h: see this module's BUGS
        }
        for id in r.first..=r.last {
            if done == 256 {
                break; // a bound, not a rule: see BUGS on IVMDs
            }
            if let Ok(Some((i, _))) = ivrs.owner_index(id) {
                crate::iommu::confine(id as u32, &[]);
                done += 1;
                if let Slot::Up(u) = &mut UNITS.lock()[i] {
                    u.reserved_devices += 1;
                }
            }
        }
    }

    // Step 3.
    let mut g = UNITS.lock();
    for s in g.iter_mut() {
        if let Slot::Up(u) = s {
            enable(u);
        }
    }
    let n = ivrs.units().len();
    for (i, d) in ivrs.units().iter().enumerate() {
        match &g[i] {
            Slot::Up(u) => crate::println!(
                "  amd-vi      : unit {:#x} up ({} of {n}, ivhd type {:#x}), {} device table \
                 entries all blocked, translation enabled (status confirmed), {} ivmd device(s) \
                 mapped",
                d.register_base,
                i + 1,
                d.kind,
                u.entries,
                u.reserved_devices,
            ),
            Slot::Refused(why) => crate::println!(
                "  amd-vi      : unit {:#x} NOT up ({} of {n}): {why}; the devices it serves are \
                 not translated",
                d.register_base,
                i + 1,
            ),
            Slot::Absent => {}
        }
    }
}

/// Is any AMD-Vi unit up?
pub fn is_active() -> bool {
    UNITS.lock().iter().any(|s| matches!(s, Slot::Up(_)))
}

/// The unit that serves `rid`, when it is one this kernel brought up.
fn owner_index(rid: u32) -> Option<usize> {
    let ivrs = (*IVRS.lock())?;
    match ivrs.owner_index(rid as u16) {
        Ok(Some((i, _))) => Some(i),
        _ => None,
    }
}

/// **Confine device `rid` to the domain rooted at `root`** (an [`paging::x86_64::AmdVi`] table
/// the portable seam built), in the unit the IVRS says serves it, and make that unit forget what
/// it cached. The entry written is the device's own and, when an alias entry says its
/// transactions carry another id, that id's too; Linux writes both (`clone_aliases`, from memory).
/// A device no unit serves gets no entry, as on VT-d.
pub fn attach(rid: u32, root: u64) {
    let Some(i) = owner_index(rid) else {
        return;
    };
    let source = match *IVRS.lock() {
        Some(ivrs) => ivrs.source_id(rid as u16) as u32,
        None => rid,
    };
    let mut g = UNITS.lock();
    let Slot::Up(u) = &mut g[i] else {
        return;
    };
    let domain = u.domain();
    let dte = translating_dte(root, domain);
    u.set_entry(rid, dte);
    if source != rid {
        u.set_entry(source, dte);
    }
    if u.enabled {
        u.sync();
    }
}

/// **Switch device `rid` to the blocked entry** (§163's quarantine; milestone 102 is its caller).
/// The device's next transaction is target-aborted, whatever its domain had mapped.
#[cfg_attr(not(test), allow(dead_code))]
pub fn quarantine(rid: u32) {
    let Some(i) = owner_index(rid) else {
        return;
    };
    let mut g = UNITS.lock();
    let Slot::Up(u) = &mut g[i] else {
        return;
    };
    let dte = blocked_dte(u.empty_root);
    u.set_entry(rid, dte);
    if u.enabled {
        u.sync();
    }
}

/// **Every IVMD the firmware declared for `rid`**, as regions its domain must also map.
pub fn for_each_reserved_region(rid: u32, each: &mut dyn FnMut(paging::domain::DmaRegion)) {
    let Some(ivrs) = *IVRS.lock() else {
        return;
    };
    ivrs.unity_for(rid as u16, &mut |r| {
        each(paging::domain::DmaRegion {
            base: r.base,
            size: r.size,
        });
    });
}

/// **Which unit, if any, translates `rid`, and why?** The same answer VT-d's `scope_of` gives,
/// from the IVRS's device ids rather than the DMAR's paths.
pub fn scope_of(rid: u32) -> crate::iommu::Scope {
    use crate::iommu::Scope;
    let mut up = [None; MAX_IVHDS];
    let mut translating = None;
    for (i, s) in UNITS.lock().iter().enumerate() {
        if let Slot::Up(u) = s {
            up[i] = Some(u.base);
            translating.get_or_insert(u.base);
        }
    }
    let Some(translating) = translating else {
        return Scope::NoIommu;
    };
    let Some(ivrs) = *IVRS.lock() else {
        return Scope::Unknown { translating };
    };
    match ivrs.owner_index(rid as u16) {
        Ok(Some((i, how))) => match up[i] {
            Some(unit) => Scope::Owned { unit, how },
            None => Scope::Elsewhere {
                translating,
                owner: Some(ivrs.units()[i].register_base),
            },
        },
        Ok(None) => Scope::Elsewhere {
            translating,
            owner: None,
        },
        Err(()) => Scope::Unknown { translating },
    }
}

/// **Pop one event, if any unit logged one** (section 2.5). The record's device id, its event
/// code (bits 63:60 of the first qword; 2 is `IO_PAGE_FAULT`) and its address. An overflowed log
/// records nothing until software clears `EventOverflow` and restarts logging (section 3.4,
/// status register), so that happens here first, as VT-d's `take_fault` clears `PFO`.
pub fn take_fault() -> Option<Fault> {
    let g = UNITS.lock();
    for slot in g.iter() {
        let Slot::Up(u) = slot else { continue };
        if r64(u.base, STATUS) & STATUS_EVENT_OVERFLOW != 0 {
            w64(u.base, STATUS, STATUS_EVENT_OVERFLOW);
            let control = r64(u.base, CONTROL);
            w64(u.base, CONTROL, control & !CONTROL_EVENT_LOG_EN);
            w64(u.base, CONTROL, control | CONTROL_EVENT_LOG_EN);
        }
        let head = r64(u.base, EVENT_HEAD) & 0x7fff0;
        if head == r64(u.base, EVENT_TAIL) & 0x7fff0 {
            continue;
        }
        let at = phys_to_virt(u.event_log + head) as *const u64;
        // SAFETY: `event_log` is a kernel-owned page and `head` a 16-byte offset inside it, behind
        // the tail the unit has published.
        let (first, address) = unsafe {
            (
                core::ptr::read_volatile(at),
                core::ptr::read_volatile(at.add(1)),
            )
        };
        w64(u.base, EVENT_HEAD, (head + 16) % RING_BYTES);
        return Some(Fault {
            rid: (first & 0xffff) as u32,
            code: (first >> 60) as u32,
            // A zero address is QEMU 11.1.1's model writing none (see `Fault::addr`); a real fault
            // at device address 0 is reported as unknown with it, which no test aims at.
            addr: (address != 0).then_some(address & !0xfff),
        });
    }
    None
}

/// **This machine's AMD-Vi, for the machine description.** `true` when there was a unit to
/// describe, so VT-d's `print_summary` knows not to print its own "none".
#[cfg_attr(
    any(test, feature = "system_tests", feature = "bench"),
    allow(dead_code)
)]
pub fn print_summary() -> bool {
    let g = UNITS.lock();
    let mut any = false;
    for s in g.iter() {
        match s {
            Slot::Up(u) => {
                any = true;
                crate::println!(
                    "  iommu           : AMD-Vi unit at {:#018x}, device table default-deny, \
                     translating, interrupt remapping not enabled",
                    u.base,
                );
            }
            Slot::Refused(why) => {
                any = true;
                crate::println!("  iommu           : AMD-Vi unit refused: {why}");
            }
            Slot::Absent => {}
        }
    }
    any
}

/// Bench diagnostic, the AMD-Vi side of VT-d's `print_faults`: the unit's status, the entry for
/// `rid`, and how many events are waiting. Clears nothing.
#[cfg(feature = "disk_throughput")]
pub fn print_faults(rid: u32) {
    let owner = owner_index(rid);
    let g = UNITS.lock();
    let Some(Slot::Up(u)) = owner.map(|i| &g[i]) else {
        crate::println!("diag amd-vi no running unit serves {rid:#06x}");
        return;
    };
    let head = r64(u.base, EVENT_HEAD);
    let tail = r64(u.base, EVENT_TAIL);
    crate::println!(
        "diag amd-vi {:#x} control {:#x} status {:#x} events {}",
        u.base,
        r64(u.base, CONTROL),
        r64(u.base, STATUS),
        ((tail + RING_BYTES - head) % RING_BYTES) / 16,
    );
    if rid < u.entries {
        let e = read_dte(u.device_table, rid);
        crate::println!(
            "diag amd-vi dte[{rid:#06x}] {:#x} {:#x} {:#x} {:#x}",
            e[0],
            e[1],
            e[2],
            e[3]
        );
    }
}

#[cfg(test)]
mod tests {
    //! What a boot on an AMD-Vi machine can say about the unit itself. The confinement claim (a
    //! device's DMA outside its grant is refused) is the escape tests', which run on the same
    //! machine: `virtio::tests` and `system_tests`' NVMe test.

    use super::*;

    /// **Every unit the IVRS names is translating, with both rings running**, read back from the
    /// unit's own status register rather than from what this driver wrote.
    #[test_case]
    fn every_unit_the_ivrs_names_is_translating() {
        let Some(ivrs) = *IVRS.lock() else {
            crate::testing::skip!("this machine's ACPI names no IVRS, so there is no unit to ask");
        };
        let g = UNITS.lock();
        for (i, d) in ivrs.units().iter().enumerate() {
            match &g[i] {
                Slot::Up(u) => {
                    assert_ne!(r64(u.base, CONTROL) & CONTROL_IOMMU_EN, 0);
                    let s = r64(u.base, STATUS);
                    assert_eq!(
                        s & (STATUS_CMD_BUF_RUN | STATUS_EVENT_LOG_RUN),
                        STATUS_CMD_BUF_RUN | STATUS_EVENT_LOG_RUN,
                        "unit {:#x} status {s:#x}: a ring is not running",
                        u.base
                    );
                }
                Slot::Refused(why) => panic!("unit {:#x} was refused: {why}", d.register_base),
                Slot::Absent => panic!("unit {:#x} was never brought up", d.register_base),
            }
        }
    }

    /// **A device nobody confined has the blocked entry**: the IOMMU's own function, which the
    /// IVRS names and nothing ever attaches. This is the default-deny half of the claim, read
    /// from the table the unit walks.
    #[test_case]
    fn a_device_nobody_confined_is_blocked() {
        let Some(ivrs) = *IVRS.lock() else {
            crate::testing::skip!("this machine's ACPI names no IVRS");
        };
        let g = UNITS.lock();
        for (i, d) in ivrs.units().iter().enumerate() {
            let Slot::Up(u) = &g[i] else { continue };
            let id = d.device_id as u32;
            assert!(
                id < u.entries,
                "the unit's own id {id:#x} is past its table"
            );
            assert_eq!(read_dte(u.device_table, id), blocked_dte(u.empty_root));
        }
    }

    /// **A device no unit serves gets no entry anywhere** (the VT-d test's twin). Device id
    /// `0xfe00` is on a bus QEMU's runner never populates, and past the table QEMU's IVRS sizes.
    #[test_case]
    fn a_device_no_unit_serves_gets_no_entry() {
        match scope_of(0xfe00) {
            crate::iommu::Scope::Elsewhere { owner: None, .. } => {}
            crate::iommu::Scope::NoIommu => crate::testing::skip!("no AMD-Vi unit is up"),
            _ => crate::testing::skip!("device 0xfe00 has an owner on this IVRS"),
        }
        let root = zeroed_frames(1, "test domain root");
        attach(0xfe00, root);
        for s in UNITS.lock().iter() {
            let Slot::Up(u) = s else { continue };
            if 0xfe00 < u.entries {
                assert_eq!(read_dte(u.device_table, 0xfe00), blocked_dte(u.empty_root));
            }
        }
    }

    /// **Interrupt remapping is never enabled**: no entry sets `IV`. The claim VT-d's
    /// `interrupt_remapping_is_reported_and_never_enabled` makes, on this hardware's terms.
    #[test_case]
    fn no_entry_enables_interrupt_remapping() {
        if !is_active() {
            crate::testing::skip!("no AMD-Vi unit is up");
        }
        for s in UNITS.lock().iter() {
            let Slot::Up(u) = s else { continue };
            for id in 0..u.entries {
                assert_eq!(read_dte(u.device_table, id)[2] & DTE_IV, 0, "entry {id:#x}");
            }
        }
    }

    /// **Quarantine is the blocked entry, and it takes effect**: a device is attached, then
    /// quarantined, and its entry reads as the default. Run on the IOMMU's own function, which no
    /// driver owns, so the boot loses nothing.
    #[test_case]
    fn quarantine_restores_the_blocked_entry() {
        let Some(ivrs) = *IVRS.lock() else {
            crate::testing::skip!("this machine's ACPI names no IVRS");
        };
        let id = ivrs.units()[0].device_id as u32;
        let root = zeroed_frames(1, "test domain root");
        attach(id, root);
        {
            let g = UNITS.lock();
            let Slot::Up(u) = &g[0] else { return };
            assert_eq!(read_dte(u.device_table, id)[0] & DTE_ROOT_MASK, root);
        }
        quarantine(id);
        let g = UNITS.lock();
        let Slot::Up(u) = &g[0] else { return };
        assert_eq!(read_dte(u.device_table, id), blocked_dte(u.empty_root));
    }

    /// The two entries this driver writes, spelled against Table 7's bit positions: `V` (0), `TV`
    /// (1), `Mode` (11:9) = 4, the root (51:12), `IR` (61), `IW` (62), the domain id in the second
    /// qword's low 16 bits, and nothing in the reserved bits QEMU checks (6:2 and 63).
    #[test_case]
    fn the_entries_are_what_table_7_says() {
        let t = translating_dte(0x1234_5000, 7);
        assert_eq!(t, [0x6000_0000_1234_5803, 7, 0, 0]);
        let b = blocked_dte(0x9000);
        assert_eq!(b, [0x0000_0000_0000_9803, 0, 0, 0]);
        for q in [t[0], b[0]] {
            assert_eq!(q & ((0b11111 << 2) | (1 << 63)), 0);
        }
    }
}
