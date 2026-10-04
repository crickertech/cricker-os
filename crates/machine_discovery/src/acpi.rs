//! **ACPI: what x86 has instead of a device tree.**
//!
//! Milestone 161. This module sits beside the arch records rather than inside the `x86_64` one, for
//! the reason [`cpu_list`](crate::cpu_list) does: ACPI is not an x86 standard. Every aarch64 server
//! that is not a device-tree board describes itself with exactly these tables, and milestone 20's
//! own roadmap text says the machine after the VisionFive 2 "should probably be a UEFI/ACPI machine
//! rather than another Device Tree board". x86 is simply its first consumer here.
//!
//! # The shape, and how it differs from a device tree
//!
//! A device tree is one blob with one root and a tree of nodes, and `crates/device_tree_blob` walks
//! it. ACPI is a **linked structure of independent tables**, each with its own signature and
//! checksum, reached from a root pointer that is not itself a table:
//!
//! ```text
//!   RSDP  ("RSD PTR ")            found by scanning low memory, or handed over by the loader
//!    |
//!    +-- RSDT (32-bit pointers)   ACPI 1.0
//!    +-- XSDT (64-bit pointers)   ACPI 2.0+, and what to prefer when both exist
//!         |
//!         +-- APIC ("MADT")       the local APICs, the IO APICs, and the interrupt rewiring
//!         +-- MCFG                where the PCIe ECAM window is
//!         +-- DMAR                where the IOMMU is
//!         +-- FACP, HPET, WAET, ...
//! ```
//!
//! **Every table is checksummed and this module checks**, which a device tree has no equivalent of.
//! That is not politeness: the RSDP is found by *scanning memory for a string*, so without the
//! checksum any sixteen bytes that happen to spell `RSD PTR ` would be believed.
//!
//! # What is here and what is deliberately not
//!
//! Here: the RSDP, the root table walk, the SDT header, and the two tables the kernel needs first
//! (MADT and MCFG). Not here, and it is the big one: **AML**, the bytecode in the DSDT that
//! describes everything ACPI does not have a fixed table for, including PCI interrupt routing
//! (`_PRT`) and every power-management method. AML needs an interpreter, which is a project rather
//! than a parser, and nothing in this kernel needs it yet.
//!
//! # BUGS
//!
//! - **Nothing validates that two tables do not overlap**, or that a table's length is sane
//!   relative to where it sits in memory. The caller supplies the bytes, so the caller is where a
//!   physical-address sanity check belongs.
//! - **The MADT's `flags` bit 0 (`PCAT_COMPAT`) is reported and not acted on.** It means the machine
//!   also has 8259 PICs that must be masked before the APICs are used. Whoever brings the APIC up
//!   has to mask them; this only says whether they are there.
//! - **A DMAR's PCI device scopes and RMRRs are decoded into fixed arrays** ([`DmarUnits`],
//!   milestone 261 (the NVMe driver leaves the kernel)'s bench rehearsal, RMRRs since milestone 594 (every VT-d unit translates its own devices)) and anything past them sets
//!   [`DmarUnits::truncated`], which turns an ownership question into "unknown" rather than a
//!   guess. IOAPIC, HPET and ACPI-namespace scopes are skipped, and ATSR, RHSA, ANDD, SATC and
//!   SIDP are not decoded at all. An ANDD names an ACPI-namespace device (an I2C or serial
//!   controller with no PCI identity) that can issue DMA; this kernel drives none, so none is
//!   confined, and a machine where one matters needs ANDD and the type-5 scope decoded.
//! - **`Dmar::flags`' `INTR_REMAP` bit is read and never used.** Interrupt remapping is a real VT-d
//!   feature this parser can report the presence of and this kernel does not build.
//! - **An MCFG window whose `end_bus` precedes its `start_bus` is reported as written.** Nothing in
//!   the encoding forbids it and [`mcfg_entry`] does not refuse it, because the bus numbers are the
//!   firmware's claim and this module reports claims. [`McfgEntry::size`] answers zero for such a
//!   window, which is what "no bus is in this range" means; a caller that reads `start_bus` and
//!   `end_bus` itself, as the boot print does, sees the inverted pair.

/// The eight bytes that begin an RSDP. Note the trailing space; it is part of the signature.
pub const RSDP_SIGNATURE: &[u8; 8] = b"RSD PTR ";

/// The length of the ACPI 1.0 RSDP, which is what its checksum covers.
const RSDP_V1_LEN: usize = 20;
/// The length of the ACPI 2.0 RSDP, which carries a second checksum over the whole thing.
const RSDP_V2_LEN: usize = 36;

/// Every system descriptor table begins with this many bytes of common header.
pub const SDT_HEADER_LEN: usize = 36;

/// Why an ACPI structure could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcpiError {
    /// The signature is not the one expected.
    BadSignature,
    /// The bytes end before the structure does.
    Truncated,
    /// The bytes sum to something other than zero. **The most important error here**, because the
    /// RSDP is found by scanning for a string and this is the only thing that separates a real one
    /// from a coincidence.
    BadChecksum,
    /// A length field says something impossible (shorter than its own header).
    BadLength(u32),
}

/// Do these bytes sum to zero in eight-bit arithmetic? That is ACPI's checksum for every structure
/// it defines.
pub fn is_checksum_ok(bytes: &[u8]) -> bool {
    bytes.iter().fold(0u8, |acc, b| acc.wrapping_add(*b)) == 0
}

/// The root pointer, decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rsdp {
    /// 0 for ACPI 1.0 (RSDT only), 2 or more for ACPI 2.0+ (XSDT present).
    pub revision: u8,
    /// The 32-bit root table's physical address. Always present.
    pub rsdt: u32,
    /// The 64-bit root table's physical address, or 0 on a revision-0 RSDP.
    pub xsdt: u64,
}

impl Rsdp {
    /// **Which root table to walk.** The XSDT when there is one, because a machine with memory
    /// above 4 GiB can have tables the RSDT's 32-bit pointers cannot name, and a firmware that
    /// publishes both is not required to list the same tables in each. Returns
    /// `(physical_address, entries_are_64_bit)`.
    pub const fn root_table(&self) -> (u64, bool) {
        if self.revision >= 2 && self.xsdt != 0 {
            (self.xsdt, true)
        } else {
            (self.rsdt as u64, false)
        }
    }
}

/// Decode an RSDP from bytes beginning at its signature.
///
/// Both checksums are checked when the revision says there are two. The first covers the first 20
/// bytes (which is the whole ACPI 1.0 structure, and is why the field order was never allowed to
/// change); the second covers all 36.
pub fn parse_rsdp(bytes: &[u8]) -> Result<Rsdp, AcpiError> {
    if bytes.len() < RSDP_V1_LEN {
        return Err(AcpiError::Truncated);
    }
    if &bytes[0..8] != RSDP_SIGNATURE {
        return Err(AcpiError::BadSignature);
    }
    if !is_checksum_ok(&bytes[..RSDP_V1_LEN]) {
        return Err(AcpiError::BadChecksum);
    }
    let revision = bytes[15];
    let rsdt = u32(bytes, 16);

    if revision < 2 {
        return Ok(Rsdp {
            revision,
            rsdt,
            xsdt: 0,
        });
    }

    if bytes.len() < RSDP_V2_LEN {
        return Err(AcpiError::Truncated);
    }
    // The extended structure's own length field, at offset 20. Trusted only as far as the second
    // checksum, which is computed over exactly what it claims.
    let length = u32(bytes, 20) as usize;
    if !(RSDP_V2_LEN..=bytes.len()).contains(&length) {
        return Err(AcpiError::BadLength(length as u32));
    }
    if !is_checksum_ok(&bytes[..length]) {
        return Err(AcpiError::BadChecksum);
    }
    Ok(Rsdp {
        revision,
        rsdt,
        xsdt: u64(bytes, 24),
    })
}

/// The header every system descriptor table begins with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SdtHeader {
    /// Four ASCII characters: `APIC`, `MCFG`, `DMAR`, `FACP`.
    pub signature: [u8; 4],
    /// The table's total length **including** this header.
    pub length: u32,
    pub revision: u8,
    /// Six ASCII characters naming the firmware vendor. Read only for the boot print, and worth
    /// printing: it is the one place a firmware identifies itself before anything else runs.
    pub oem_id: [u8; 6],
}

impl SdtHeader {
    /// The signature as a string, for printing. `None` if it is not ASCII, which would mean the
    /// bytes are not a table.
    pub fn signature_str(&self) -> Option<&str> {
        core::str::from_utf8(&self.signature).ok()
    }

    /// How many bytes follow this header in the table.
    pub const fn body_len(&self) -> usize {
        self.length as usize - SDT_HEADER_LEN
    }
}

/// Decode a table header, without checking the checksum (which needs the whole table, and the
/// caller has to read `length` bytes before it can have them).
pub fn parse_sdt_header(bytes: &[u8]) -> Result<SdtHeader, AcpiError> {
    if bytes.len() < SDT_HEADER_LEN {
        return Err(AcpiError::Truncated);
    }
    let length = u32(bytes, 4);
    if (length as usize) < SDT_HEADER_LEN {
        return Err(AcpiError::BadLength(length));
    }
    Ok(SdtHeader {
        signature: [bytes[0], bytes[1], bytes[2], bytes[3]],
        length,
        revision: bytes[8],
        // Offset 10, not 9: byte 8 is the revision and byte 9 is the table's own checksum. Getting
        // this off by one reads the checksum as the first letter of the vendor's name, which prints
        // as garbage and is the kind of mistake that survives review because nothing depends on it.
        oem_id: [
            bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
        ],
    })
}

/// How many table pointers a root table of `length` bytes holds.
pub const fn root_entry_count(length: u32, entries_are_64_bit: bool) -> usize {
    let body = (length as usize).saturating_sub(SDT_HEADER_LEN);
    if entries_are_64_bit {
        body / 8
    } else {
        body / 4
    }
}

/// The physical address of root-table entry `index`. `body` begins **after** the SDT header.
///
/// Split from the header this way because the kernel reads the header first to learn the length,
/// then reads the body; handing this the whole table would mean the caller doing the same offset
/// arithmetic twice, in two places, with one chance to get it wrong.
pub fn root_entry(body: &[u8], index: usize, entries_are_64_bit: bool) -> Option<u64> {
    let width = if entries_are_64_bit { 8 } else { 4 };
    let at = index.checked_mul(width)?;
    if body.len() < at.checked_add(width)? {
        return None;
    }
    Some(if entries_are_64_bit {
        u64(body, at)
    } else {
        u32(body, at) as u64
    })
}

// ---------------------------------------------------------------------------------------------
// The MADT (signature "APIC"): where the interrupt controllers are.
// ---------------------------------------------------------------------------------------------

/// The MADT's fixed part, which precedes its variable-length entry list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Madt {
    /// The local APIC's physical address as the 32-bit field states it. A
    /// [`MadtEntry::LocalApicAddressOverride`] later in the list replaces it, which is why this is
    /// not the final answer on every machine.
    pub local_apic: u32,
    /// Bit 0 (`PCAT_COMPAT`) means the machine also has 8259 PICs. See this module's BUGS.
    pub flags: u32,
}

/// True when the machine has legacy 8259 PICs that must be masked before the APICs are used.
pub const MADT_PCAT_COMPAT: u32 = 1 << 0;

/// The eight bytes of the MADT that precede its entries.
const MADT_FIXED_LEN: usize = 8;

/// Decode the MADT's fixed part. `body` begins after the SDT header.
pub fn parse_madt(body: &[u8]) -> Result<Madt, AcpiError> {
    if body.len() < MADT_FIXED_LEN {
        return Err(AcpiError::Truncated);
    }
    Ok(Madt {
        local_apic: u32(body, 0),
        flags: u32(body, 4),
    })
}

/// One entry of the MADT's list. Only the kinds this tree acts on are decoded, four for x86 and
/// three for aarch64; the rest keep their type byte so a boot print can say what it skipped rather
/// than pretending the list was shorter than it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MadtEntry {
    /// A CPU, named by its local APIC id. **`enabled` is what decides whether it can be started**:
    /// a processor entry exists for sockets that are empty and for cores the firmware disabled.
    LocalApic {
        processor_id: u8,
        apic_id: u8,
        /// The CPU is usable now.
        enabled: bool,
        /// The CPU is not enabled but could be brought online later (hot-plug). Not the same as
        /// `enabled`, and starting one that is merely online-capable is a different operation.
        online_capable: bool,
    },
    /// An IO APIC: where device interrupts arrive. `gsi_base` is the first global system interrupt
    /// this one owns, which is how a machine with several of them divides the space.
    IoApic { id: u8, address: u32, gsi_base: u32 },
    /// **A legacy IRQ has been rewired.** The ISA interrupt `source` actually arrives as global
    /// system interrupt `gsi`, with `flags` giving polarity and trigger mode. This is the entry that
    /// makes "COM1 is IRQ 4" not mean "COM1 is IO APIC input 4": on almost every machine the timer's
    /// IRQ 0 is remapped, and reading this wrong arms the wrong line.
    InterruptSourceOverride {
        bus: u8,
        source: u8,
        gsi: u32,
        flags: u16,
    },
    /// The local APIC is not at the 32-bit address the fixed part gave; it is here.
    LocalApicAddressOverride(u64),
    /// **An aarch64 core** (type 11, "GIC CPU Interface"), which is this architecture's
    /// [`MadtEntry::LocalApic`]: the per-core entry, one per processor, carrying the id a bring-up
    /// call has to name. There is no local APIC on an Arm machine, so a decoder that knew only the
    /// x86 types read every core of an aarch64 server as [`MadtEntry::Other`] and found none.
    GenericInterruptController {
        /// `MPIDR_EL1[39:0]`, the affinity value PSCI's `CPU_ON` takes. The device tree spells the
        /// same number in a `cpu@` node's `reg`, which is why `cpu_list::Cpu::hwid` needs no second
        /// meaning for this path.
        mpidr: u64,
        /// The processor's ACPI UID, which is what the DSDT's `Processor` objects refer to. Kept
        /// because it is the only handle AML has on this core, not because anything reads it yet.
        uid: u32,
        /// The core is usable now. Same meaning, and same consequence, as
        /// [`MadtEntry::LocalApic`]'s.
        enabled: bool,
        /// Not enabled, but able to be brought online later. See [`MadtEntry::LocalApic`].
        online_capable: bool,
        /// The GICv2 CPU interface (`GICC`) for this core, zero on a GICv3 machine where the CPU
        /// interface is system registers rather than memory.
        cpu_interface: u64,
        /// This core's GICv3 redistributor frame, zero when the machine instead states its
        /// redistributors as [`MadtEntry::GenericRedistributor`] ranges. Both spellings are legal
        /// and QEMU uses the second.
        redistributor: u64,
    },
    /// **The GIC distributor** (type 12), one per machine, and the entry that says which GIC
    /// architecture version this is. A device tree says the same thing through `compatible`, which
    /// is why `version` is the field a tree writer needs.
    GenericDistributor {
        /// `GICD`, physical.
        address: u64,
        /// 0 unspecified, 1 `GICv1`, 2 `GICv2`, 3 `GICv3`, 4 `GICv4`. **Zero is common and means "work it
        /// out"**, which a caller can only do from the other entries.
        version: u8,
    },
    /// **A range of GICv3 redistributor frames** (type 14), the contiguous array `GICR_TYPER` is
    /// then walked over. The alternative spelling is a per-core `redistributor` in
    /// [`MadtEntry::GenericInterruptController`]; a machine states one or the other.
    GenericRedistributor {
        /// The first frame, physical.
        address: u64,
        /// How many bytes of frames, all cores' together.
        length: u32,
    },
    /// A kind this decoder does not act on, with its type byte.
    Other(u8),
}

/// Walks a MADT's entry list. Each entry is `[type, length, ...]`, so the list is self-describing
/// and a length of zero would loop forever; that is refused by ending the walk.
pub struct MadtEntries<'a> {
    body: &'a [u8],
    at: usize,
}

/// Iterate the MADT's entries. `body` begins after the SDT header.
pub fn madt_entries(body: &[u8]) -> MadtEntries<'_> {
    MadtEntries {
        body,
        at: MADT_FIXED_LEN,
    }
}

impl Iterator for MadtEntries<'_> {
    type Item = MadtEntry;

    fn next(&mut self) -> Option<MadtEntry> {
        if self.at + 2 > self.body.len() {
            return None;
        }
        let kind = self.body[self.at];
        let len = self.body[self.at + 1] as usize;
        // A zero (or absurdly short) length is a malformed table, and continuing would either loop
        // forever or read the next entry from the middle of this one. Stop, which reports the
        // entries read so far rather than inventing more.
        if len < 2 || self.at + len > self.body.len() {
            return None;
        }
        let e = &self.body[self.at..self.at + len];
        self.at += len;

        Some(match kind {
            0 if len >= 8 => {
                let flags = u32(e, 4);
                MadtEntry::LocalApic {
                    processor_id: e[2],
                    apic_id: e[3],
                    enabled: flags & 1 != 0,
                    online_capable: flags & 2 != 0,
                }
            }
            1 if len >= 12 => MadtEntry::IoApic {
                id: e[2],
                address: u32(e, 4),
                gsi_base: u32(e, 8),
            },
            2 if len >= 10 => MadtEntry::InterruptSourceOverride {
                bus: e[2],
                source: e[3],
                gsi: u32(e, 4),
                flags: u16(e, 8),
            },
            5 if len >= 12 => MadtEntry::LocalApicAddressOverride(u64(e, 4)),
            // The three aarch64 kinds. Their lengths have grown across ACPI revisions (a GICC
            // entry was 40 bytes in 5.0, 76 in 5.1, 80 in 6.0 and 82 in 6.3), so every field is
            // taken behind a length check for the offset it sits at rather than behind one check
            // of the whole entry: a firmware writing an older, shorter revision still yields the
            // fields it does carry instead of falling off the list as `Other`.
            11 if len >= 16 => MadtEntry::GenericInterruptController {
                uid: u32(e, 8),
                enabled: u32(e, 12) & 1 != 0,
                online_capable: u32(e, 12) & 2 != 0,
                cpu_interface: if len >= 40 { u64(e, 32) } else { 0 },
                redistributor: if len >= 68 { u64(e, 60) } else { 0 },
                mpidr: if len >= 76 { u64(e, 68) } else { 0 },
            },
            12 if len >= 24 => MadtEntry::GenericDistributor {
                address: u64(e, 8),
                version: e[20],
            },
            14 if len >= 16 => MadtEntry::GenericRedistributor {
                address: u64(e, 4),
                length: u32(e, 12),
            },
            other => MadtEntry::Other(other),
        })
    }
}

// ---------------------------------------------------------------------------------------------
// Legacy IRQ numbers, and the overrides that make them a lie.
// ---------------------------------------------------------------------------------------------

/// **The polarity field of an MPS INTI flags word**, bits 1:0. `00` means "whatever this bus
/// specifies", `01` active high, `11` active low. `10` is reserved.
pub const INTI_POLARITY_MASK: u16 = 0b11;
/// Polarity `11`: the line is asserted low.
const INTI_POLARITY_ACTIVE_LOW: u16 = 0b11;

/// **The trigger-mode field**, bits 3:2. `00` means "whatever this bus specifies", `01` edge, `11`
/// level. `10` is reserved.
pub const INTI_TRIGGER_MASK: u16 = 0b11 << 2;
/// Trigger `11`: the line stays asserted until the device is serviced.
const INTI_TRIGGER_LEVEL: u16 = 0b11 << 2;

/// The bus number an [`MadtEntry::InterruptSourceOverride`] uses for the ISA bus. It is the only
/// value ACPI defines for that field, which is why the overrides are exactly the legacy IRQs.
pub const ISA_BUS: u8 = 0;

/// How many legacy ISA IRQs there are: two cascaded 8259s, eight lines each.
pub const ISA_IRQ_COUNT: usize = 16;

/// **How one ISA interrupt actually reaches an IO APIC.**
///
/// The whole point of this type is that `gsi` is very often *not* the IRQ number it was looked up
/// by. On essentially every PC the timer's IRQ 0 arrives as global system interrupt 2, because the
/// PIT is wired to the IO APIC's pin 2 while pin 0 carries the 8259 cascade. A kernel that armed
/// redirection entry 0 for "the timer" would arm a line nothing drives, and would see no
/// interrupts and no error.
///
/// Name: provisional (milestone 161 (the kernel port)), along with the two fields and
/// [`isa_irq_table`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IsaIrqRouting {
    /// The global system interrupt this IRQ arrives on.
    pub gsi: u32,
    /// The line is asserted low rather than high.
    pub active_low: bool,
    /// The line is level triggered rather than edge triggered.
    pub level_triggered: bool,
}

impl IsaIrqRouting {
    /// **What an ISA IRQ is when nothing overrides it**: identity-mapped onto the global interrupt
    /// space, active high, edge triggered. That is the ISA bus's own convention, which is what the
    /// `00` ("conforms to the specifications of the bus") encoding in an override's flags means.
    pub const fn isa_default(irq: u8) -> Self {
        Self {
            gsi: irq as u32,
            active_low: false,
            level_triggered: false,
        }
    }

    /// Apply an override's MPS INTI flags word. `00` in either field means "conforms to the bus",
    /// which for the ISA bus is what [`isa_default`](Self::isa_default) already set, so those bits
    /// deliberately change nothing.
    const fn with_flags(mut self, flags: u16) -> Self {
        if flags & INTI_POLARITY_MASK == INTI_POLARITY_ACTIVE_LOW {
            self.active_low = true;
        }
        if flags & INTI_TRIGGER_MASK == INTI_TRIGGER_LEVEL {
            self.level_triggered = true;
        }
        self
    }
}

/// **Resolve all sixteen legacy ISA IRQs through the MADT's interrupt source overrides.** `body`
/// begins after the SDT header.
///
/// Returned as a whole table rather than one lookup at a time because the overrides are a list that
/// has to be walked to answer any single question, and the caller (an interrupt controller being
/// brought up) wants the answers to outlive the table's bytes.
///
/// An override naming a source outside 0..16 is ignored: the field is a legacy IRQ number and there
/// are sixteen of those, so a larger one is a malformed table rather than a seventeenth IRQ.
pub fn isa_irq_table(body: &[u8]) -> [IsaIrqRouting; ISA_IRQ_COUNT] {
    let mut table = core::array::from_fn(|irq| IsaIrqRouting::isa_default(irq as u8));
    for entry in madt_entries(body) {
        if let MadtEntry::InterruptSourceOverride {
            bus,
            source,
            gsi,
            flags,
        } = entry
            && bus == ISA_BUS
            && (source as usize) < ISA_IRQ_COUNT
        {
            table[source as usize] = IsaIrqRouting {
                gsi,
                active_low: false,
                level_triggered: false,
            }
            .with_flags(flags);
        }
    }
    table
}

// ---------------------------------------------------------------------------------------------
// The MCFG: where the PCIe ECAM window is.
// ---------------------------------------------------------------------------------------------

/// One ECAM window: a range of buses on one PCI segment, and the physical address their
/// configuration space is memory-mapped at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct McfgEntry {
    pub base: u64,
    pub segment: u16,
    pub start_bus: u8,
    pub end_bus: u8,
}

impl McfgEntry {
    /// How many bytes of configuration space this window covers. One bus is 1 MiB (32 devices x 8
    /// functions x 4 KiB), and the range is inclusive at both ends.
    ///
    /// **A window that ends before it begins covers nothing, and says so rather than panicking.**
    /// Nothing in the MCFG's encoding stops firmware writing `start_bus = 255, end_bus = 0`, and
    /// the subtraction underflowed on exactly that until milestone 319's
    /// `verification::an_ecam_windows_size_is_total_and_counts_one_mebibyte_per_bus` found it.
    /// Zero is the honest answer: there is no bus in the range, so there is no configuration space
    /// to map.
    pub const fn size(&self) -> u64 {
        if self.end_bus < self.start_bus {
            return 0;
        }
        (self.end_bus as u64 - self.start_bus as u64 + 1) * 0x10_0000
    }
}

/// The MCFG's eight reserved bytes before its entry list.
const MCFG_FIXED_LEN: usize = 8;
/// The size of one MCFG allocation entry.
const MCFG_ENTRY_LEN: usize = 16;

/// Decode MCFG entry `index`. `body` begins after the SDT header.
pub fn mcfg_entry(body: &[u8], index: usize) -> Option<McfgEntry> {
    let at = MCFG_FIXED_LEN + index.checked_mul(MCFG_ENTRY_LEN)?;
    if body.len() < at.checked_add(MCFG_ENTRY_LEN)? {
        return None;
    }
    Some(McfgEntry {
        base: u64(body, at),
        segment: u16(body, at + 8),
        start_bus: body[at + 10],
        end_bus: body[at + 11],
    })
}

// ---------------------------------------------------------------------------------------------
// The DMAR: where VT-d is.
// ---------------------------------------------------------------------------------------------

/// The DMAR's fixed part, before its list of remapping structures: the machine's physical
/// address width and a flags byte, then ten reserved bytes. Verified against QEMU's own table
/// builder (`build_dmar_q35` in `hw/i386/acpi-build.c`), which is the ground truth for what this
/// parser has to read, the same way the MADT and MCFG sections above were checked against q35.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dmar {
    /// The machine's physical address width in bits, decoded from the table's
    /// `HostAddressWidth - 1` field (so a table saying 38 means a 39-bit width).
    ///
    /// **A `u16` for a value no machine puts above 64**, because the byte the table holds ranges
    /// over `0..=255` and the width it names therefore ranges over `1..=256`. Narrower than the
    /// field plus one is not a smaller type, it is an addition that can overflow, and firmware
    /// writing `0xff` here panicked this parser on the boot path until milestone 319's
    /// `verification::the_dmar_fixed_part_decodes_without_arithmetic_overflow` found it. Widening
    /// makes the wrong state unrepresentable rather than guarded.
    pub host_address_width: u16,
    /// Bit 0 is `INTR_REMAP`: the platform also supports interrupt remapping. Reported and not
    /// acted on, the same posture the MADT's `PCAT_COMPAT` bit takes; interrupt remapping is not
    /// built here (milestone 161 roadmap item 6 names it a follow-on, not this item).
    pub flags: u8,
}

/// One byte of host address width, one byte of flags, ten reserved bytes.
const DMAR_FIXED_LEN: usize = 12;

/// Decode the DMAR's fixed part. `body` begins after the SDT header.
pub fn parse_dmar(body: &[u8]) -> Result<Dmar, AcpiError> {
    if body.len() < DMAR_FIXED_LEN {
        return Err(AcpiError::Truncated);
    }
    Ok(Dmar {
        host_address_width: body[0] as u16 + 1,
        flags: body[1],
    })
}

/// **One DRHD: one VT-d hardware unit's register file.** A machine can have more than one (one
/// per PCI segment, sometimes one per root port group, and on a client Intel part one for the
/// integrated graphics beside the catch-all). The kernel brings up every one [`DmarUnits`]
/// records (milestone 594, provisional number).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Drhd {
    /// Remapping-structure flags bit 0: this unit is the catch-all for every PCI device on its
    /// segment that no other DRHD's device-scope list names. [`DmarUnits::owner`] reads it.
    pub include_pci_all: bool,
    /// The PCI segment group this unit covers. Always 0 on a single-segment machine, which QEMU's
    /// `q35` is.
    pub segment: u16,
    /// The physical address of this unit's MMIO register file (`kernel/src/arch/x86_64/iommu.rs`
    /// reads it as the SMMUv3 driver reads its device-tree base and the RISC-V driver reads its
    /// BAR).
    pub register_base: u64,
    /// **How many bytes the register file spans**, from the DRHD's `Size` byte (VT-d 4.1 section
    /// 8.3: bits 3:0 are N, and the set is 2^N 4 KiB pages). Firmware written before that field
    /// existed left the byte reserved and zero, which decodes to one page, the size every unit
    /// had then. Name: provisional (milestone 594).
    pub register_size: u64,
}

/// The register-file size a DRHD's `Size` byte (offset 5) names: 4 KiB shifted by its low four
/// bits. At most 128 MiB, so the shift cannot overflow.
const fn drhd_register_size(size_byte: u8) -> u64 {
    4096u64 << (size_byte & 0xf)
}

/// How many bytes a DRHD's fixed part occupies before its (unparsed) device-scope list: the
/// 4-byte type/length header shared by every remapping structure, plus flags, reserved, segment
/// and register base.
const DRHD_FIXED_LEN: usize = 16;

/// One entry of the DMAR's remapping-structure list. Only DRHD (type 0, the piece this kernel
/// drives) is decoded; the rest keep their type code so a boot print can say what it skipped
/// rather than pretending the list was shorter than it is. The same shape as [`MadtEntry`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmarEntry {
    Drhd(Drhd),
    /// A remapping-structure type this decoder does not act on (RMRR, ATSR, RHSA, ANDD, SATC,
    /// ...), with its type code.
    Other(u16),
}

/// Walks a DMAR's remapping-structure list. Each entry is `[type: u16, length: u16, ...]`, the
/// same self-describing shape the MADT's entries use, and a length of zero is refused the same
/// way: it would never advance and the walk would loop forever.
pub struct DmarStructures<'a> {
    body: &'a [u8],
    at: usize,
}

/// Iterate the DMAR's remapping structures. `body` begins after the SDT header.
pub fn dmar_structures(body: &[u8]) -> DmarStructures<'_> {
    DmarStructures {
        body,
        at: DMAR_FIXED_LEN,
    }
}

impl Iterator for DmarStructures<'_> {
    type Item = DmarEntry;

    fn next(&mut self) -> Option<DmarEntry> {
        if self.at + 4 > self.body.len() {
            return None;
        }
        let kind = u16(self.body, self.at);
        let len = u16(self.body, self.at + 2) as usize;
        if len < 4 || self.at + len > self.body.len() {
            return None;
        }
        let e = &self.body[self.at..self.at + len];
        self.at += len;

        Some(match kind {
            0 if len >= DRHD_FIXED_LEN => DmarEntry::Drhd(Drhd {
                include_pci_all: e[4] & 1 != 0,
                segment: u16(e, 6),
                register_base: u64(e, 8),
                register_size: drhd_register_size(e[5]),
            }),
            other => DmarEntry::Other(other),
        })
    }
}

/// The first DRHD in the list, if any. **No longer the unit the kernel brings up**: since milestone
/// 594 (every VT-d unit translates its own devices) it brings up every unit [`DmarUnits`] records. Kept because in-tree
/// records cite it and its tests pin the table order a client Intel machine uses.
pub fn first_drhd(body: &[u8]) -> Option<Drhd> {
    dmar_structures(body).find_map(|e| match e {
        DmarEntry::Drhd(d) => Some(d),
        DmarEntry::Other(_) => None,
    })
}

// ---------------------------------------------------------------------------------------------
// Which unit owns which device, and which firmware memory each device still needs: the
// device-scope lists and the RMRRs, decoded (milestone 261's bench rehearsal, milestone 594).
// ---------------------------------------------------------------------------------------------

/// How many DRHDs [`DmarUnits`] records. A client Intel part has two (one for the integrated
/// graphics, one catch-all); a two-socket server has one per root complex, a handful. A table with
/// more than this sets [`DmarUnits::truncated`] rather than dropping the rest silently.
pub const MAX_DRHDS: usize = 8;
/// How many PCI device-scope entries [`DmarUnits`] records across every DRHD. Only the two PCI
/// types are kept (endpoint and sub-hierarchy); IOAPIC, HPET and ACPI-namespace entries name no
/// requester id this kernel confines and are skipped.
pub const MAX_SCOPES: usize = 32;
/// The longest path a recorded scope can carry: the device itself plus three bridges above it.
/// Deeper than any machine this tree has met, and a deeper one sets [`DmarUnits::truncated`].
pub const MAX_SCOPE_PATH: usize = 4;
/// How many RMRRs [`DmarUnits`] records. The `OptiPlex` 7040 publishes two (USB, graphics). A
/// ninth sets [`DmarUnits::truncated`]. Name: provisional (milestone 594).
pub const MAX_RMRRS: usize = 8;
/// How many PCI device-scope entries [`DmarUnits`] records across every RMRR. Name: provisional
/// (milestone 594).
pub const MAX_RMRR_SCOPES: usize = 16;

/// Device-scope type 1: a PCI endpoint, named by its path from the start bus.
pub const SCOPE_PCI_ENDPOINT: u8 = 1;
/// Device-scope type 2: a PCI-PCI bridge, and with it **every device below it**.
pub const SCOPE_PCI_SUBHIERARCHY: u8 = 2;

/// The fixed part of a device-scope entry before its path: type, length, flags, reserved,
/// enumeration id, start bus. VT-d 3.x section 8.3.1; QEMU's `insert_scope` writes the same six.
const SCOPE_FIXED_LEN: usize = 6;

/// An RMRR's fixed part before its scope list: the 4-byte header, two reserved bytes, the segment,
/// then the region's base and its last byte (VT-d 4.1 section 8.4).
const RMRR_FIXED_LEN: usize = 24;

/// **One PCI device-scope entry, as the firmware wrote it**: a start bus and a path of
/// (device, function) pairs, each hop but the last a bridge whose secondary bus is where the next
/// hop lives. Resolving the path therefore needs the bus's live bridge registers, which is why
/// [`DmarUnits::owner`] takes a reader rather than doing it here.
///
/// Name: provisional (milestone 261's bench rehearsal). calef names public items.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DeviceScope {
    /// Index of the structure this entry belongs to: into [`DmarUnits::drhds`] for an entry in
    /// [`DmarUnits::scopes`], into [`DmarUnits::rmrrs`] for one in [`DmarUnits::rmrr_scopes`].
    pub unit: u8,
    /// [`SCOPE_PCI_ENDPOINT`] or [`SCOPE_PCI_SUBHIERARCHY`].
    pub kind: u8,
    pub start_bus: u8,
    /// `(device, function)` per hop, the first on `start_bus`. Only `path[..path_len]` is real.
    pub path: [(u8, u8); MAX_SCOPE_PATH],
    pub path_len: u8,
}

impl DeviceScope {
    /// **The function this entry's path ends at, as `(bus, device, function)` on this machine**,
    /// or `None` when a hop that should be a bridge is not one here (the path describes hardware
    /// that is not present). Every hop but the last is a bridge, and its secondary bus is where the
    /// next hop sits. Name: provisional (milestone 594).
    pub fn resolve(
        &self,
        bridge: &mut dyn FnMut(u8, u8, u8) -> Option<(u8, u8)>,
    ) -> Option<(u8, u8, u8)> {
        let n = self.path_len as usize;
        // `parse` never records an empty or over-long path; the fields are public, so say so
        // here rather than let a hand-built scope underflow `n - 1`.
        if n == 0 || n > MAX_SCOPE_PATH {
            return None;
        }
        let mut at_bus = self.start_bus;
        for &(d, f) in &self.path[..n - 1] {
            at_bus = bridge(at_bus, d, f)?.0;
        }
        let (d, f) = self.path[n - 1];
        Some((at_bus, d, f))
    }

    /// **Does this entry cover `bus:dev.func`, and how?** An endpoint covers the function it
    /// resolves to; a sub-hierarchy covers its bridge and every bus in the bridge's
    /// secondary..=subordinate range (VT-d 4.1 section 8.3.1: "the specified bridge device and all
    /// its downstream devices").
    fn covers(
        &self,
        bus: u8,
        dev: u8,
        func: u8,
        bridge: &mut dyn FnMut(u8, u8, u8) -> Option<(u8, u8)>,
    ) -> Option<Ownership> {
        let (at_bus, d, f) = self.resolve(bridge)?;
        if (at_bus, d, f) == (bus, dev, func) {
            return Some(Ownership::Named);
        }
        if self.kind == SCOPE_PCI_SUBHIERARCHY
            && let Some((secondary, subordinate)) = bridge(at_bus, d, f)
            && (secondary..=subordinate).contains(&bus)
        {
            return Some(Ownership::UnderBridge(at_bus, d, f));
        }
        None
    }
}

/// **How a unit came to own a device**, which is the half of the answer a bench reader needs to
/// believe the other half. The three ways the VT-d specification allows (section 8.3).
///
/// Name: provisional (milestone 261's bench rehearsal).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ownership {
    /// The unit's scope names this function as an endpoint.
    Named,
    /// The unit's scope names a bridge, and this function's bus is inside the bridge's
    /// secondary..=subordinate range. The bridge is carried as (bus, device, function).
    UnderBridge(u8, u8, u8),
    /// Nothing names the function, and this unit carries `INCLUDE_PCI_ALL` for its segment.
    CatchAll,
}

/// **One RMRR: memory the firmware keeps DMA-ing into after it hands over** (VT-d 4.1 sections
/// 3.16 and 8.4). USB legacy keyboard emulation under SMM and a UMA graphics controller's stolen
/// memory are the two the specification names, and the two the `OptiPlex` 7040 publishes. The
/// specification's instruction is that system software identity-map `base..=limit`, read and
/// write, for every device the region's scope names, before it turns translation on.
///
/// Its devices are the entries of [`DmarUnits::rmrr_scopes`] whose `unit` is this region's index.
///
/// Name: provisional (milestone 594). calef names public items.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ReservedRegion {
    pub segment: u16,
    /// First byte, 4 KiB aligned.
    pub base: u64,
    /// Last byte (inclusive, as the table writes it), so `limit + 1` is 4 KiB aligned.
    pub limit: u64,
}

impl ReservedRegion {
    /// The region's length in bytes. Total: [`DmarUnits::parse`] records only regions whose
    /// `limit` exceeds `base`.
    pub const fn size(&self) -> u64 {
        self.limit - self.base + 1
    }

    /// The VT-d specification's well-formedness rule for an RMRR (section 8.4: a 4 KiB-aligned
    /// base, a size that is a whole number of 4 KiB pages, a limit above the base), which is also
    /// the check Linux's `rmrr_sanity_check` makes before it trusts one.
    const fn is_well_formed(&self) -> bool {
        self.base.is_multiple_of(4096)
            && self.limit > self.base
            && self.limit != u64::MAX
            && (self.limit + 1).is_multiple_of(4096)
    }
}

/// **Every DRHD, every PCI device scope and every RMRR in one DMAR**, decoded into fixed arrays so
/// the kernel can keep it after the firmware's bytes are out of reach (it is read at boot, before
/// the fine map, and asked after it). `Copy` and free of lifetimes for exactly that reason.
///
/// This exists because [`first_drhd`] answered a question nobody on real hardware asks. xenon's
/// DMAR is 204 bytes and its first DRHD is `0xfed90000`; on the Skylake `OptiPlex` 7040, the same
/// family and the same register addresses, Linux reports that unit with flags `0x0` and a second
/// at `0xfed91000` with flags `0x1` (`INCLUDE_PCI_ALL`), which is the integrated-graphics unit
/// followed by the catch-all. See notes/risk-6-bench-evening.md for the source.
///
/// Name: provisional (milestone 261's bench rehearsal).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DmarUnits {
    pub drhds: [Drhd; MAX_DRHDS],
    pub drhd_count: usize,
    pub scopes: [DeviceScope; MAX_SCOPES],
    pub scope_count: usize,
    pub rmrrs: [ReservedRegion; MAX_RMRRS],
    pub rmrr_count: usize,
    pub rmrr_scopes: [DeviceScope; MAX_RMRR_SCOPES],
    pub rmrr_scope_count: usize,
    /// RMRRs the table held and this decoder refused because they break section 8.4's
    /// well-formedness rule. Linux skips such a region with a firmware-bug warning; so does this,
    /// and the count is what lets the boot print say so. Name: provisional (milestone 594).
    pub rmrrs_refused: usize,
    /// Something in the table did not fit the arrays above: a ninth DRHD or RMRR, a scope past its
    /// array, a path deeper than [`MAX_SCOPE_PATH`], or a scope whose length disagrees with its
    /// path. **When set, a device no recorded scope names is answered "unknown" rather than handed
    /// to the catch-all**, because the entry that names it may be the one that was dropped.
    pub truncated: bool,
}

impl Default for DmarUnits {
    fn default() -> Self {
        Self {
            drhds: [Drhd {
                include_pci_all: false,
                segment: 0,
                register_base: 0,
                register_size: 0,
            }; MAX_DRHDS],
            drhd_count: 0,
            scopes: [DeviceScope::default(); MAX_SCOPES],
            scope_count: 0,
            rmrrs: [ReservedRegion::default(); MAX_RMRRS],
            rmrr_count: 0,
            rmrr_scopes: [DeviceScope::default(); MAX_RMRR_SCOPES],
            rmrr_scope_count: 0,
            rmrrs_refused: 0,
            truncated: false,
        }
    }
}

/// Decode one scope list into `out`, tagging each entry with `owner`. Total for any input: a
/// malformed list stops and sets `truncated`, and an entry past `out` sets it too.
fn read_scope_list(
    mut list: &[u8],
    owner: u8,
    out: &mut [DeviceScope],
    count: &mut usize,
    truncated: &mut bool,
) {
    while list.len() >= 2 {
        let kind = list[0];
        let len = list[1] as usize;
        if len < SCOPE_FIXED_LEN || len > list.len() || !(len - SCOPE_FIXED_LEN).is_multiple_of(2) {
            *truncated = true;
            return;
        }
        let entry = &list[..len];
        list = &list[len..];
        if kind != SCOPE_PCI_ENDPOINT && kind != SCOPE_PCI_SUBHIERARCHY {
            continue;
        }
        let hops = (len - SCOPE_FIXED_LEN) / 2;
        if hops == 0 || hops > MAX_SCOPE_PATH || *count == out.len() {
            *truncated = true;
            continue;
        }
        let mut s = DeviceScope {
            unit: owner,
            kind,
            start_bus: entry[5],
            path_len: hops as u8,
            ..DeviceScope::default()
        };
        for h in 0..hops {
            s.path[h] = (
                entry[SCOPE_FIXED_LEN + 2 * h],
                entry[SCOPE_FIXED_LEN + 2 * h + 1],
            );
        }
        out[*count] = s;
        *count += 1;
    }
}

impl DmarUnits {
    /// Decode every DRHD, every RMRR and their PCI scopes. `body` begins after the SDT header.
    /// Total for any input: a malformed scope list stops that structure's scopes and sets
    /// [`DmarUnits::truncated`].
    pub fn parse(body: &[u8]) -> DmarUnits {
        let mut u = DmarUnits::default();
        let mut at = DMAR_FIXED_LEN;
        while at + 4 <= body.len() {
            let kind = u16(body, at);
            let len = u16(body, at + 2) as usize;
            if len < 4 || at + len > body.len() {
                break;
            }
            let e = &body[at..at + len];
            at += len;
            match kind {
                0 if len >= DRHD_FIXED_LEN => {
                    if u.drhd_count == MAX_DRHDS {
                        u.truncated = true;
                        continue;
                    }
                    let unit = u.drhd_count;
                    u.drhds[unit] = Drhd {
                        include_pci_all: e[4] & 1 != 0,
                        segment: u16(e, 6),
                        register_base: u64(e, 8),
                        register_size: drhd_register_size(e[5]),
                    };
                    u.drhd_count += 1;
                    read_scope_list(
                        &e[DRHD_FIXED_LEN..],
                        unit as u8,
                        &mut u.scopes,
                        &mut u.scope_count,
                        &mut u.truncated,
                    );
                }
                1 if len >= RMRR_FIXED_LEN => {
                    let r = ReservedRegion {
                        segment: u16(e, 6),
                        base: u64(e, 8),
                        limit: u64(e, 16),
                    };
                    if !r.is_well_formed() {
                        u.rmrrs_refused += 1;
                        continue;
                    }
                    if u.rmrr_count == MAX_RMRRS {
                        u.truncated = true;
                        continue;
                    }
                    let index = u.rmrr_count;
                    u.rmrrs[index] = r;
                    u.rmrr_count += 1;
                    read_scope_list(
                        &e[RMRR_FIXED_LEN..],
                        index as u8,
                        &mut u.rmrr_scopes,
                        &mut u.rmrr_scope_count,
                        &mut u.truncated,
                    );
                }
                _ => {}
            }
        }
        u
    }

    /// The recorded DRHDs, in table order.
    pub fn units(&self) -> &[Drhd] {
        &self.drhds[..self.drhd_count]
    }

    /// The recorded RMRRs, in table order. Name: provisional (milestone 594).
    pub fn reserved_regions(&self) -> &[ReservedRegion] {
        &self.rmrrs[..self.rmrr_count]
    }

    /// **Which unit owns PCI function `bus:dev.func` on `segment`**, by the VT-d specification's
    /// rule (section 8.3): an explicit scope anywhere wins, then the segment's `INCLUDE_PCI_ALL`
    /// unit. `Ok(None)` is a machine where nothing translates this device's DMA. `Err(())` is a
    /// table this decoder recorded only part of, where the missing part could be the answer.
    ///
    /// Linux's `dmar_find_matched_drhd_unit` walks the units in table order and takes the
    /// catch-all the moment it reaches it; the specification requires the catch-all to be listed
    /// last on its segment, so the two agree on every table that obeys it, and this one also
    /// agrees on a table that does not.
    ///
    /// `bridge` returns a bridge's `(secondary, subordinate)` bus numbers, or `None` when
    /// `bus:dev.func` is not a bridge. It is the live bus, which is the only thing that can turn a
    /// firmware path into a bus number.
    #[allow(clippy::result_unit_err)]
    pub fn owner(
        &self,
        segment: u16,
        bus: u8,
        dev: u8,
        func: u8,
        bridge: &mut dyn FnMut(u8, u8, u8) -> Option<(u8, u8)>,
    ) -> Result<Option<(Drhd, Ownership)>, ()> {
        Ok(self
            .owner_index(segment, bus, dev, func, bridge)?
            .map(|(i, how)| (self.drhds[i], how)))
    }

    /// [`DmarUnits::owner`], answered as an index into [`DmarUnits::units`]: the unit whose root
    /// table a context entry for this function belongs in. Name: provisional (milestone 594).
    #[allow(clippy::result_unit_err)]
    pub fn owner_index(
        &self,
        segment: u16,
        bus: u8,
        dev: u8,
        func: u8,
        bridge: &mut dyn FnMut(u8, u8, u8) -> Option<(u8, u8)>,
    ) -> Result<Option<(usize, Ownership)>, ()> {
        for s in &self.scopes[..self.scope_count] {
            let unit = s.unit as usize;
            if self.drhds[unit].segment != segment {
                continue;
            }
            if let Some(how) = s.covers(bus, dev, func, bridge) {
                return Ok(Some((unit, how)));
            }
        }
        if self.truncated {
            return Err(());
        }
        Ok(self
            .units()
            .iter()
            .position(|d| d.include_pci_all && d.segment == segment)
            .map(|i| (i, Ownership::CatchAll)))
    }

    /// **Every RMRR the firmware declared for `bus:dev.func` on `segment`**, each passed to `each`
    /// once. A region names a device by an endpoint scope, or by a sub-hierarchy scope above it,
    /// which is the same test Linux's `intel_iommu_get_resv_regions` makes
    /// (`is_downstream_to_pci_bridge`). The kernel adds every one to the device's DMA domain,
    /// because section 3.16 asks for the identity map in *the* translation the device uses, and a
    /// domain built without them would fault the firmware's own DMA the moment it was attached.
    /// Name: provisional (milestone 594).
    pub fn reserved_for(
        &self,
        segment: u16,
        bus: u8,
        dev: u8,
        func: u8,
        bridge: &mut dyn FnMut(u8, u8, u8) -> Option<(u8, u8)>,
        each: &mut dyn FnMut(ReservedRegion),
    ) {
        for (i, r) in self.reserved_regions().iter().enumerate() {
            if r.segment != segment {
                continue;
            }
            let named = self.rmrr_scopes[..self.rmrr_scope_count]
                .iter()
                .filter(|s| s.unit as usize == i)
                .any(|s| s.covers(bus, dev, func, bridge).is_some());
            if named {
                each(*r);
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The tables an Arm machine has that an x86 one does not.
// ---------------------------------------------------------------------------------------------

/// **The Generic Timer Description Table** (signature `GTDT`), which is where an Arm machine states
/// the architected timer's interrupts.
///
/// x86 has no counterpart: its timers are the HPET and the local APIC, both described elsewhere. On
/// a device-tree machine the same facts are the `arm,armv8-timer` node's `interrupts` property, and
/// the two describe the same four timers in the same order, which is what makes a tree writer able
/// to carry one into the other.
///
/// **A GSIV is an absolute INTID**, not a bank-relative number: the virtual timer's 27 here is the
/// same 27 the GIC delivers, where the device tree spells it `<1 11 ...>` (PPI 11, and PPIs start
/// at 16). Converting between the two is the writer's job, not this parser's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gtdt {
    /// The secure EL1 timer's interrupt, and its flags. A kernel at EL1 non-secure never takes it.
    pub secure_el1: (u32, u32),
    /// The non-secure EL1 physical timer (`CNTP_*`).
    pub non_secure_el1: (u32, u32),
    /// **The virtual timer** (`CNTV_*`), which is the one this kernel arms.
    pub virtual_el1: (u32, u32),
    /// The EL2 physical timer (`CNTHP_*`).
    pub el2: (u32, u32),
}

/// The GTDT flag bit that says the line is edge-triggered rather than level-triggered.
pub const GTDT_EDGE_TRIGGERED: u32 = 1 << 0;
/// The GTDT flag bit that says the line is asserted low rather than high.
pub const GTDT_ACTIVE_LOW: u32 = 1 << 1;

/// The bytes of a GTDT body this decoder needs, which is everything up to the platform-timer list.
const GTDT_FIXED_LEN: usize = 60;

/// Decode the GTDT's four architected timers. `body` begins after the SDT header.
///
/// The platform timers past the fixed part (the memory-mapped `CNTCTLBase` blocks and the watchdog
/// entries) are **not** decoded: nothing here drives them, and the device tree binding this feeds
/// has no place to put them.
pub fn parse_gtdt(body: &[u8]) -> Result<Gtdt, AcpiError> {
    if body.len() < GTDT_FIXED_LEN {
        return Err(AcpiError::Truncated);
    }
    Ok(Gtdt {
        secure_el1: (u32(body, 12), u32(body, 16)),
        non_secure_el1: (u32(body, 20), u32(body, 24)),
        virtual_el1: (u32(body, 28), u32(body, 32)),
        el2: (u32(body, 36), u32(body, 40)),
    })
}

/// **The Serial Port Console Redirection table** (signature `SPCR`): where the firmware's console
/// is and which interrupt it raises.
///
/// It is the only fixed table that names a UART, and on an ACPI-only machine it is therefore the
/// only place that fact exists outside AML. SBBR requires it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spcr {
    /// The interface type byte: see [`SPCR_PL011`] and [`SPCR_SBSA_UART`].
    pub interface: u8,
    /// The register block's address, in whatever space `address_space` names.
    pub address: u64,
    /// 0 for system memory, 1 for system I/O. An Arm machine says 0.
    pub address_space: u8,
    /// The GSIV the port raises, absolute like every GSIV, or `None` when the table states only a
    /// PC-AT IRQ (an x86 spelling an Arm machine does not use).
    pub interrupt: Option<u32>,
}

/// [`Spcr::interface`] for a PL011, which is what QEMU `virt` and most Arm machines present.
pub const SPCR_PL011: u8 = 0x03;
/// [`Spcr::interface`] for the SBSA generic UART, a PL011 subset with no DMA and no modem lines.
pub const SPCR_SBSA_UART: u8 = 0x0e;

/// The bit of SPCR's interrupt-type byte that says a GSIV is stated rather than a PC-AT IRQ.
const SPCR_INTERRUPT_TYPE_GSIV: u8 = 1 << 3;

/// The bytes of an SPCR body this decoder reads.
const SPCR_FIXED_LEN: usize = 22;

/// Decode the SPCR. `body` begins after the SDT header.
pub fn parse_spcr(body: &[u8]) -> Result<Spcr, AcpiError> {
    if body.len() < SPCR_FIXED_LEN {
        return Err(AcpiError::Truncated);
    }
    // The base address is a Generic Address Structure at offset 4: space id, width, offset, access
    // size, then the 64-bit address. Only the space id and the address decide anything here.
    let gsiv = u32(body, 18);
    Ok(Spcr {
        interface: body[0],
        address_space: body[4],
        address: u64(body, 8),
        // A GSIV of zero is how a table that has no interrupt to state spells it; interrupt 0 is
        // an SGI and no UART raises one, so zero is unambiguous rather than a value to defend.
        interrupt: (body[16] & SPCR_INTERRUPT_TYPE_GSIV != 0 && gsiv != 0).then_some(gsiv),
    })
}

/// **The two facts the FADT holds that only an Arm machine has**: whether firmware implements PSCI,
/// and which instruction calls it.
///
/// They are two bits of the "ARM Boot Architecture Flags" word, which ACPI 5.1 added at a fixed
/// offset and which is meaningless on every other architecture. A device tree states the same pair
/// as `/psci`'s `compatible` and `method`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArmBoot {
    /// Firmware implements the PSCI interface, so cores can be started with `CPU_ON`.
    pub psci: bool,
    /// The call is `hvc` rather than `smc`. Meaningless when [`ArmBoot::psci`] is false.
    pub hvc: bool,
}

/// Where the ARM Boot Architecture Flags sit **in the FADT body**, which is 36 bytes after the
/// offset ACPI's own table states (129).
const FADT_ARM_BOOT_AT: usize = 129 - SDT_HEADER_LEN;

/// Decode the FADT's Arm boot flags. `body` begins after the SDT header.
///
/// A body too short to hold the word is an ACPI 5.0-or-earlier FADT, which predates PSCI's
/// existence in this table; that is [`AcpiError::Truncated`] rather than "no PSCI", because the two
/// are different claims and only the caller knows what to do with the first.
pub fn parse_arm_boot(body: &[u8]) -> Result<ArmBoot, AcpiError> {
    if body.len() < FADT_ARM_BOOT_AT + 2 {
        return Err(AcpiError::Truncated);
    }
    let flags = u16(body, FADT_ARM_BOOT_AT);
    Ok(ArmBoot {
        psci: flags & 1 != 0,
        hvc: flags & 2 != 0,
    })
}

/// **Where the FADT says to write to reset the machine** (milestone 249 (the boot lottery is sampled by a person walking to the board)'s `x86_64` half).
///
/// ACPI 2.0 added a Generic Address Structure (`RESET_REG`) and a byte (`RESET_VALUE`) to the FADT:
/// write the byte to the register and the platform performs a reset. It is the route firmware
/// vouches for, which is why the kernel tries it before the two legacy ports it falls back to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResetRegister {
    /// Which address space [`ResetRegister::address`] is in.
    pub space: ResetSpace,
    /// The register's width in bits, as the table states it. ACPI requires 8 for this register;
    /// kept rather than assumed so a caller can refuse anything else out loud.
    pub bit_width: u8,
    /// The register's address in [`ResetRegister::space`]. For [`ResetSpace::PciConfig`] this is
    /// ACPI's packed form: device in bits 32..48, function in 16..32, register offset in 0..16, on
    /// bus 0.
    pub address: u64,
    /// The byte to write.
    pub value: u8,
}

/// The address spaces a reset register can be in. ACPI permits exactly three for this one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResetSpace {
    /// System memory: an MMIO store.
    Memory,
    /// System I/O: an `out` to a port. Every PC chipset this project has seen puts it at `0xCF9`.
    Io,
    /// PCI configuration space on bus 0.
    PciConfig,
    /// Anything else, which ACPI does not permit here; carried so the caller can say what it saw.
    Other(u8),
}

/// The FADT flag bit that says the reset register is implemented (`RESET_REG_SUP`, bit 10).
const FADT_RESET_REG_SUP: u32 = 1 << 10;
/// Where the FADT's `Flags` word sits **in the body** (ACPI's own offset is 112).
const FADT_FLAGS_AT: usize = 112 - SDT_HEADER_LEN;
/// Where `RESET_REG` sits in the body (ACPI offset 116); a 12-byte Generic Address Structure.
const FADT_RESET_REG_AT: usize = 116 - SDT_HEADER_LEN;
/// Where `RESET_VALUE` sits in the body (ACPI offset 128).
const FADT_RESET_VALUE_AT: usize = 128 - SDT_HEADER_LEN;

/// Decode the FADT's reset register. `body` begins after the SDT header.
///
/// `None` is one answer to three questions, deliberately: a body too short to hold the field (an
/// ACPI 1.0 FADT, which predates it), a table whose `RESET_REG_SUP` flag is clear, and an address of
/// zero. Each means "firmware does not offer this route", and the caller's response to all three is
/// the same: fall back to the legacy ports. Unlike [`parse_arm_boot`], a short body is not an error
/// here, because on `x86_64` a FADT without the field is a machine the kernel still has to reset.
///
/// Name: provisional (milestone 249): calef names public items.
pub fn parse_fadt_reset(body: &[u8]) -> Option<ResetRegister> {
    if body.len() <= FADT_RESET_VALUE_AT {
        return None;
    }
    if u32(body, FADT_FLAGS_AT) & FADT_RESET_REG_SUP == 0 {
        return None;
    }
    let gas = &body[FADT_RESET_REG_AT..FADT_RESET_REG_AT + 12];
    let address = u64(gas, 4);
    if address == 0 {
        return None;
    }
    Some(ResetRegister {
        space: match gas[0] {
            0 => ResetSpace::Memory,
            1 => ResetSpace::Io,
            2 => ResetSpace::PciConfig,
            other => ResetSpace::Other(other),
        },
        bit_width: gas[1],
        address,
        value: body[FADT_RESET_VALUE_AT],
    })
}

fn u16(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

fn u32(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

fn u64(bytes: &[u8], at: usize) -> u64 {
    let mut w = [0u8; 8];
    w.copy_from_slice(&bytes[at..at + 8]);
    u64::from_le_bytes(w)
}

/// Machine-checked proofs over the ACPI tables (DECISIONS §14, milestone 319).
///
/// These tables are the x86 boot path's untrusted input, and on 2026-09-17 they stopped being
/// hypothetical: xenon, a Dell workstation, booted nife and this code parsed a real MADT, MCFG
/// and DMAR written by firmware that had never heard of it (`bench/xenon-2026-09-17/`). Until then
/// every table this parser had seen was one QEMU wrote for it.
///
/// **Two of the harnesses below were false when they were written**, which is the answer to why a
/// crate with 41 hand-written tests wanted a prover: both defects are in arithmetic over a field a
/// test would have had to think to write down, and neither is reachable from any table QEMU emits.
/// [`the_dmar_fixed_part_decodes_without_arithmetic_overflow`] and
/// [`an_ecam_windows_size_is_total_and_counts_one_mebibyte_per_bus`] have the details.
///
/// **What is deliberately not here**: the whole-table walk from the RSDP down through the XSDT,
/// which needs a symbolic pointer into memory this crate never holds, and is the same wall
/// `crates/device_tree_blob` records for the structure-block token loop. The leaves and the two
/// self-describing entry walks are what bounded model checking can reach, so they are what is
/// proved.
///
/// Names: provisional (milestone 319 (the crate that parses firmware had no proofs)). An architect
/// names things.
#[cfg(kani)]
mod verification {
    use super::*;

    /// Long enough to hold a v2 RSDP, a table header, and a few entries of either walk. Every
    /// harness here is linear in this, so it is the whole cost knob.
    const N: usize = 36;

    /// **An RSDP is accepted only when its bytes sum to zero**, for every 36-byte input rather than
    /// for the handful of corruptions a test can write down.
    ///
    /// This is the crate's one genuinely security-shaped claim, and the module header says why: the
    /// RSDP is found by **scanning memory for an eight-byte string**, so the checksum is the only
    /// thing between a coincidence and a physical address the kernel will follow. The hand-written
    /// tests flip one byte in the short range and one in the extended range; this quantifies over
    /// every byte pattern, including the ones where a second error cancels the first.
    ///
    /// Could plausibly have been false: checking the checksum over `bytes.len()` rather than over
    /// `RSDP_V1_LEN`, or after the revision branch instead of before it, both pass every test in
    /// this file and both let a 20-byte structure through unchecked.
    /// Falsification: replayable `crates/machine_discovery/falsifications/acpi.verification.an_rsdp_is_accepted_only_when_its_bytes_sum_to_zero.patch`
    #[kani::proof]
    fn an_rsdp_is_accepted_only_when_its_bytes_sum_to_zero() {
        let bytes: [u8; N] = kani::any();
        if let Ok(r) = parse_rsdp(&bytes) {
            assert!(is_checksum_ok(&bytes[..RSDP_V1_LEN]));
            if r.revision >= 2 {
                // The extended checksum covers the structure's own length field, which is the part
                // a caller cannot compute for itself.
                let length = u32(&bytes, 20) as usize;
                assert!((RSDP_V1_LEN..=N).contains(&length));
                assert!(is_checksum_ok(&bytes[..length]));
            }
            // Not vacuous: an accepted RSDP exists.
            kani::cover!(true, "some byte pattern is a valid RSDP");
        }
    }

    /// **A header this parser accepts can always be asked how long its body is.**
    /// `SdtHeader::body_len` is `length - SDT_HEADER_LEN` with no guard of its own, so the guard in
    /// [`parse_sdt_header`] is the only thing standing between a table claiming `length = 10` and a
    /// `usize` underflow on the boot path. This proves the two are exactly matched: every header the
    /// parser returns has a body length, and it is the length less the header.
    ///
    /// Could plausibly have been false: `<=` in place of `<` is fine here, but `length > 0`, or
    /// dropping the check because "firmware would not do that", is the shape of a change a reader
    /// makes while tidying, and the crate's own tests only exercise `length = 10` and `length = 60`.
    /// Falsification: replayable `crates/machine_discovery/falsifications/acpi.verification.a_table_header_this_parser_accepts_always_has_a_body_length.patch`
    #[kani::proof]
    fn a_table_header_this_parser_accepts_always_has_a_body_length() {
        let bytes: [u8; N] = kani::any();
        if let Ok(h) = parse_sdt_header(&bytes) {
            assert_eq!(h.length as usize, SDT_HEADER_LEN + h.body_len());
            kani::cover!(h.body_len() > 0, "a table with a body is accepted");
        }
    }

    /// **The MADT walk terminates and never reads outside the body**, for every byte pattern.
    ///
    /// The entry list is self-describing: each entry carries its own length byte, which is firmware
    /// telling the parser how far to advance. A length of zero never advances and a length past the
    /// end reads the next entry out of the middle of this one, so this loop is a hostile input away
    /// from hanging the boot. The unwind bound is what proves termination: a walk that could fail to
    /// advance cannot be unrolled to a fixed depth.
    ///
    /// Could plausibly have been false: `len < 1` instead of `len < 2` (which accepts the
    /// zero-advance case as soon as a `type` byte is followed by a `1`), or `>=` in place of `>` in
    /// the bound check. Both keep every test in this file green.
    /// Falsification: replayable `crates/machine_discovery/falsifications/acpi.verification.the_madt_walk_terminates_and_stays_inside_the_body.patch`
    #[kani::proof]
    #[kani::unwind(20)]
    fn the_madt_walk_terminates_and_stays_inside_the_body() {
        let body: [u8; N] = kani::any();
        let mut it = madt_entries(&body);
        let mut seen = 0usize;
        let mut last = it.at;
        while it.next().is_some() {
            // Every step consumes at least the two bytes of its own header, which is what makes the
            // walk finite; without it this loop would not unroll.
            assert!(it.at >= last + 2);
            assert!(it.at <= body.len());
            last = it.at;
            seen += 1;
        }
        assert!(seen <= N / 2);
    }

    /// **The DMAR walk terminates and never reads outside the body**, the same claim one table over.
    ///
    /// Not a restatement of the MADT harness above and not shareable with it: these are two separate
    /// `Iterator` implementations over two different encodings (a one-byte length with a minimum of
    /// two against a little-endian `u16` length with a minimum of four), written months apart. A
    /// property that held for one and silently not the other is precisely the parity failure
    /// AGENTS.md rule 5 names, and the only way to know is to state it twice.
    ///
    /// Could plausibly have been false: `len < 2` copied across from the MADT, which is the minimum
    /// for the *other* encoding and lets a four-byte header with a claimed length of two re-read its
    /// own first half forever.
    /// Falsification: replayable `crates/machine_discovery/falsifications/acpi.verification.the_dmar_walk_terminates_and_stays_inside_the_body.patch`
    #[kani::proof]
    #[kani::unwind(12)]
    fn the_dmar_walk_terminates_and_stays_inside_the_body() {
        let body: [u8; N] = kani::any();
        let mut it = dmar_structures(&body);
        let mut seen = 0usize;
        let mut last = it.at;
        while it.next().is_some() {
            assert!(it.at >= last + 4);
            assert!(it.at <= body.len());
            last = it.at;
            seen += 1;
        }
        assert!(seen <= N / 4);
    }

    /// **No interrupt source override can write outside the sixteen legacy IRQs.**
    ///
    /// [`isa_irq_table`] indexes a fixed sixteen-entry array with `source`, a byte straight out of
    /// the table, and the only thing between firmware writing `source = 200` and an out-of-bounds
    /// store is one `<` in a chained `if let`. This proves it holds for every body.
    ///
    /// Could plausibly have been false: the guard sits in the middle of a four-clause `let`-chain
    /// added in one commit, and a reader rewriting that chain as a `match` on the entry has to carry
    /// it across by hand. The existing test covers `source = 200` and nothing else.
    /// Falsification: replayable `crates/machine_discovery/falsifications/acpi.verification.no_override_writes_outside_the_sixteen_legacy_irqs.patch`
    #[kani::proof]
    #[kani::unwind(20)]
    fn no_override_writes_outside_the_sixteen_legacy_irqs() {
        let body: [u8; N] = kani::any();
        let table = isa_irq_table(&body);
        // An IRQ nothing overrode keeps the ISA bus's own convention, so the table is never left
        // holding an uninitialised-looking entry.
        kani::cover!(
            table[0] != IsaIrqRouting::isa_default(0),
            "some body overrides IRQ 0, which is the case every PC exercises"
        );
    }

    /// **The DMAR's fixed part decodes without arithmetic overflow.**
    ///
    /// **This harness was false when it was written, and the defect was on the boot path.**
    /// `host_address_width` was `body[0] + 1` into a `u8`, so a DMAR whose `HostAddressWidth` byte
    /// is `0xff` panicked `read_dmar` in `kernel/src/arch/x86_64/machine.rs`, which calls
    /// [`parse_dmar`] directly on firmware bytes. It is the same defect `device_tree_blob::be32`'s
    /// unchecked `at + 4` was, one table over: a field widened by one with no room for the
    /// widening. Fixed by making [`Dmar::host_address_width`] a `u16`, so the addition cannot
    /// overflow at all rather than being guarded against.
    ///
    /// Could plausibly have been false, and was: every DMAR this parser had ever seen came from
    /// QEMU's `build_dmar_q35`, which writes 38. Nothing in the encoding stops a vendor writing
    /// anything else, and xenon is the reminder that vendors do.
    /// Falsification: replayable `crates/machine_discovery/falsifications/acpi.verification.the_dmar_fixed_part_decodes_without_arithmetic_overflow.patch`
    #[kani::proof]
    fn the_dmar_fixed_part_decodes_without_arithmetic_overflow() {
        let body: [u8; N] = kani::any();
        if let Ok(d) = parse_dmar(&body) {
            assert_eq!(d.host_address_width, body[0] as u16 + 1);
        }
    }

    /// **An ECAM window's size is total, and counts exactly one mebibyte per bus it covers.**
    ///
    /// **This harness was also false when it was written.** [`McfgEntry::size`] computed
    /// `end_bus - start_bus + 1` in `u64`, which underflows for any window whose end precedes its
    /// start. Nothing in the MCFG's encoding forbids that pair, and `mcfg_entry` reads both bytes
    /// straight out of the table without comparing them, so a malformed or merely creative MCFG
    /// panicked whoever asked how big the window was. Fixed by answering zero, which is what "no bus
    /// is in this range" means; the limitation is recorded in this module's BUGS.
    ///
    /// The second clause is what stops this being a totality tautology: it pins the arithmetic to
    /// 32 devices x 8 functions x 4 KiB per bus, inclusive at both ends, which an off-by-one in
    /// either direction breaks.
    /// Falsification: replayable `crates/machine_discovery/falsifications/acpi.verification.an_ecam_windows_size_is_total_and_counts_one_mebibyte_per_bus.patch`
    #[kani::proof]
    fn an_ecam_windows_size_is_total_and_counts_one_mebibyte_per_bus() {
        let start_bus: u8 = kani::any();
        let end_bus: u8 = kani::any();
        let e = McfgEntry {
            base: 0,
            segment: 0,
            start_bus,
            end_bus,
        };
        let size = e.size();
        if start_bus <= end_bus {
            let buses = end_bus as u64 - start_bus as u64 + 1;
            assert_eq!(size, buses * 0x10_0000);
        } else {
            assert_eq!(size, 0);
        }
    }

    /// **The root table's entry count and its entry reader agree, exactly.**
    ///
    /// These are two functions doing the same arithmetic in two directions:
    /// [`root_entry_count`] divides the body length by the pointer width to say how many entries
    /// there are, and [`root_entry`] multiplies an index by that width to reach one. The kernel
    /// trusts the first to bound a loop over the second, so a disagreement at the last entry is
    /// either a table silently half-walked or a `None` the caller reads as the end of the list.
    /// This proves the boundary is in the same place in both, for every length and index, at both
    /// pointer widths.
    ///
    /// Could plausibly have been false: `root_entry_count`'s `saturating_sub` and `root_entry`'s
    /// `checked_add` were written for different reasons at different times, and the `+ 1` that
    /// makes an inclusive count out of an exclusive bound has to be absent from exactly one of them.
    /// Falsification: replayable `crates/machine_discovery/falsifications/acpi.verification.the_root_tables_entry_count_and_its_entry_reader_agree.patch`
    #[kani::proof]
    fn the_root_tables_entry_count_and_its_entry_reader_agree() {
        let body: [u8; N] = kani::any();
        let body_len: usize = kani::any();
        kani::assume(body_len <= N);
        let body = &body[..body_len];
        let entries_are_64_bit: bool = kani::any();
        let index: usize = kani::any();

        let count = root_entry_count((body_len + SDT_HEADER_LEN) as u32, entries_are_64_bit);
        assert_eq!(
            root_entry(body, index, entries_are_64_bit).is_some(),
            index < count,
            "the count bounds the reader with nothing left over and nothing missing"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Set the checksum byte at `at` so `bytes[..len]` sums to zero, the way firmware does when it
    /// writes a table.
    fn seal(bytes: &mut [u8], at: usize, len: usize) {
        bytes[at] = 0;
        let sum = bytes[..len].iter().fold(0u8, |a, b| a.wrapping_add(*b));
        bytes[at] = sum.wrapping_neg();
    }

    fn rsdp_v2() -> [u8; RSDP_V2_LEN] {
        let mut b = [0u8; RSDP_V2_LEN];
        b[0..8].copy_from_slice(RSDP_SIGNATURE);
        b[9..15].copy_from_slice(b"BOCHS ");
        b[15] = 2; // revision
        b[16..20].copy_from_slice(&0x7ffe_1a40u32.to_le_bytes()); // rsdt
        b[20..24].copy_from_slice(&(RSDP_V2_LEN as u32).to_le_bytes());
        b[24..32].copy_from_slice(&0x7ffe_1b00u64.to_le_bytes()); // xsdt
        seal(&mut b, 8, RSDP_V1_LEN);
        seal(&mut b, 32, RSDP_V2_LEN);
        b
    }

    /// **The checksum is the whole defence.** The RSDP is found by scanning memory for an
    /// eight-byte string, so without this any sixteen bytes spelling `RSD PTR ` would be believed
    /// and the kernel would follow a pointer out of somebody's data.
    #[test]
    fn a_string_that_spells_the_signature_but_does_not_checksum_is_refused() {
        let mut b = rsdp_v2();
        b[17] ^= 0xff; // corrupt the RSDT pointer without fixing the checksum
        assert_eq!(parse_rsdp(&b), Err(AcpiError::BadChecksum));
    }

    /// A revision-2 RSDP has a second checksum over all 36 bytes, and it is checked too.
    #[test]
    fn the_extended_checksum_is_checked_as_well_as_the_short_one() {
        let mut b = rsdp_v2();
        b[30] ^= 0xff; // inside the extended part only; the 20-byte checksum still passes
        assert!(
            is_checksum_ok(&b[..RSDP_V1_LEN]),
            "the short one still passes"
        );
        assert_eq!(parse_rsdp(&b), Err(AcpiError::BadChecksum));
    }

    /// A wrong signature is refused before anything else is read.
    #[test]
    fn a_wrong_signature_is_refused() {
        let mut b = rsdp_v2();
        b[7] = b'!';
        assert_eq!(parse_rsdp(&b), Err(AcpiError::BadSignature));
    }

    /// **The XSDT wins when there is one.** A machine with memory above 4 GiB can have tables the
    /// RSDT's 32-bit pointers cannot name, and firmware is not required to list the same tables in
    /// both.
    #[test]
    fn the_xsdt_is_preferred_and_its_entries_are_64_bit() {
        let r = parse_rsdp(&rsdp_v2()).expect("well-formed");
        assert_eq!(r.root_table(), (0x7ffe_1b00, true));
    }

    /// A revision-0 RSDP has no XSDT, so the RSDT is the only answer and its entries are 32-bit.
    /// The extended fields are not read at all, because on a 20-byte structure they do not exist.
    #[test]
    fn a_revision_0_rsdp_uses_the_rsdt_and_never_reads_the_extended_fields() {
        let mut b = rsdp_v2();
        b[15] = 0;
        // Leave garbage where the XSDT would be, to prove it is not read.
        b[24..32].copy_from_slice(&0xdead_beef_dead_beefu64.to_le_bytes());
        seal(&mut b, 8, RSDP_V1_LEN);
        let r = parse_rsdp(&b).expect("a 20-byte RSDP is still an RSDP");
        assert_eq!(r.xsdt, 0);
        assert_eq!(r.root_table(), (0x7ffe_1a40, false));
    }

    /// **An aarch64 MADT is read as cores and a GIC, where an x86-only decoder saw nothing.**
    ///
    /// The body is QEMU `virt`'s shape with `acpi=on`: a GIC distributor stating version 3, a
    /// redistributor range, and two GIC CPU interface entries carrying their MPIDRs. Before this
    /// decoder knew the three types, every one of them came back as [`MadtEntry::Other`], which is
    /// the same answer a machine with no cores and no interrupt controller would give.
    #[test]
    fn an_aarch64_madt_names_its_gic_and_its_cpus() {
        let mut body = [0u8; 128];
        body[0..4].copy_from_slice(&0u32.to_le_bytes()); // no local APIC address
        body[4..8].copy_from_slice(&0u32.to_le_bytes()); // no PCAT_COMPAT
        // GIC distributor (type 12, 24 bytes).
        let d = &mut body[8..32];
        d[0] = 12;
        d[1] = 24;
        d[8..16].copy_from_slice(&0x0800_0000u64.to_le_bytes());
        d[20] = 3;
        // GIC redistributor (type 14, 16 bytes).
        let r = &mut body[32..48];
        r[0] = 14;
        r[1] = 16;
        r[4..12].copy_from_slice(&0x080a_0000u64.to_le_bytes());
        r[12..16].copy_from_slice(&0x00f6_0000u32.to_le_bytes());
        // Two GIC CPU interfaces at ACPI 5.0's length, which is 40 bytes and stops short of the
        // MPIDR field that 5.1 added. A firmware writing this revision is exactly the case the
        // per-offset length guards exist for.
        for (i, at) in [48usize, 88].into_iter().enumerate() {
            let c = &mut body[at..at + 40];
            c[0] = 11;
            c[1] = 40;
            c[8..12].copy_from_slice(&(i as u32).to_le_bytes()); // uid
            c[12..16].copy_from_slice(&1u32.to_le_bytes()); // enabled
        }

        let mut entries = [MadtEntry::Other(0); 8];
        let mut count = 0;
        for e in madt_entries(&body) {
            entries[count] = e;
            count += 1;
        }
        assert_eq!(count, 4, "two controller entries and two cores");
        assert_eq!(
            entries[0],
            MadtEntry::GenericDistributor {
                address: 0x0800_0000,
                version: 3
            }
        );
        assert_eq!(
            entries[1],
            MadtEntry::GenericRedistributor {
                address: 0x080a_0000,
                length: 0x00f6_0000
            }
        );
        for entry in &entries[2..4] {
            let MadtEntry::GenericInterruptController { enabled, mpidr, .. } = entry else {
                panic!("a GIC CPU interface entry, got {entry:?}");
            };
            assert!(enabled, "both cores are enabled");
            // A 40-byte entry is too short to carry the MPIDR field, and the decoder answers zero
            // for the fields the firmware's revision does not have rather than dropping the entry.
            assert_eq!(*mpidr, 0);
        }
    }

    /// A full-length (ACPI 6.0, 80-byte) GIC CPU interface entry yields the two addresses and the
    /// MPIDR, which is what a tree writer needs and what the shorter revisions cannot give.
    #[test]
    fn a_full_length_gic_cpu_interface_entry_yields_its_mpidr_and_frames() {
        let mut body = [0u8; 8 + 80];
        let c = &mut body[8..];
        c[0] = 11;
        c[1] = 80;
        c[12..16].copy_from_slice(&1u32.to_le_bytes());
        c[32..40].copy_from_slice(&0x0801_0000u64.to_le_bytes()); // GICC
        c[60..68].copy_from_slice(&0x080a_0000u64.to_le_bytes()); // GICR
        c[68..76].copy_from_slice(&0x0000_0001_0000_0003u64.to_le_bytes()); // MPIDR with Aff2 set
        assert_eq!(
            madt_entries(&body).next(),
            Some(MadtEntry::GenericInterruptController {
                uid: 0,
                enabled: true,
                online_capable: false,
                cpu_interface: 0x0801_0000,
                redistributor: 0x080a_0000,
                mpidr: 0x0000_0001_0000_0003,
            })
        );
    }

    /// The GTDT's four timers, in the order the device tree binding also states them. The numbers
    /// are QEMU `virt`'s: secure 29, non-secure 30, virtual 27, EL2 26, all level-triggered.
    #[test]
    fn the_gtdt_names_four_timers_and_their_trigger_modes() {
        let mut body = [0u8; GTDT_FIXED_LEN];
        for (at, gsiv) in [(12usize, 29u32), (20, 30), (28, 27), (36, 26)] {
            body[at..at + 4].copy_from_slice(&gsiv.to_le_bytes());
        }
        // The virtual timer, and only it, marked edge-triggered and active low, so a decoder that
        // read one flags word for all four would fail here.
        body[32..36].copy_from_slice(&(GTDT_EDGE_TRIGGERED | GTDT_ACTIVE_LOW).to_le_bytes());
        let g = parse_gtdt(&body).expect("a full-length GTDT");
        assert_eq!(g.secure_el1, (29, 0));
        assert_eq!(g.non_secure_el1, (30, 0));
        assert_eq!(g.virtual_el1, (27, GTDT_EDGE_TRIGGERED | GTDT_ACTIVE_LOW));
        assert_eq!(g.el2, (26, 0));
        every_short_prefix_is_refused(&body, GTDT_FIXED_LEN, parse_gtdt);
    }

    /// The SPCR names a PL011 at QEMU `virt`'s address on GSIV 33, which is the INTID the kernel
    /// has hardcoded since milestone 19 (run a real workload) and now reads instead.
    #[test]
    fn the_spcr_names_a_pl011_and_its_global_interrupt() {
        let mut body = [0u8; SPCR_FIXED_LEN];
        body[0] = SPCR_PL011;
        body[4] = 0; // system memory
        body[8..16].copy_from_slice(&0x0900_0000u64.to_le_bytes());
        body[16] = SPCR_INTERRUPT_TYPE_GSIV;
        body[18..22].copy_from_slice(&33u32.to_le_bytes());
        assert_eq!(
            parse_spcr(&body).expect("a full-length SPCR"),
            Spcr {
                interface: SPCR_PL011,
                address: 0x0900_0000,
                address_space: 0,
                interrupt: Some(33),
            }
        );
        every_short_prefix_is_refused(&body, SPCR_FIXED_LEN, parse_spcr);
    }

    /// **A PC-AT IRQ is not a GSIV**, and a table that states only the former answers `None` rather
    /// than handing back an ISA interrupt number as though it were a GIC INTID. That is the same
    /// class of mistake `interrupt_id`'s bank bases exist to prevent, one table over.
    #[test]
    fn an_spcr_that_states_only_a_pc_at_irq_names_no_global_interrupt() {
        let mut body = [0u8; SPCR_FIXED_LEN];
        body[0] = SPCR_PL011;
        body[16] = 1; // PC-AT compatible, not a GSIV
        body[17] = 4;
        body[18..22].copy_from_slice(&33u32.to_le_bytes());
        assert_eq!(parse_spcr(&body).expect("well formed").interrupt, None);
    }

    /// The FADT's Arm boot flags, which are two bits at a fixed offset and the only place ACPI says
    /// how to start a second core.
    #[test]
    fn the_fadt_says_whether_psci_is_there_and_which_instruction_calls_it() {
        let mut body = [0u8; FADT_ARM_BOOT_AT + 2];
        body[FADT_ARM_BOOT_AT..][..2].copy_from_slice(&0b11u16.to_le_bytes());
        assert_eq!(
            parse_arm_boot(&body).expect("a long enough FADT"),
            ArmBoot {
                psci: true,
                hvc: true
            }
        );
        body[FADT_ARM_BOOT_AT..][..2].copy_from_slice(&0b01u16.to_le_bytes());
        assert_eq!(
            parse_arm_boot(&body).expect("a long enough FADT"),
            ArmBoot {
                psci: true,
                hvc: false
            }
        );
        every_short_prefix_is_refused(&body, FADT_ARM_BOOT_AT + 2, parse_arm_boot);
    }

    /// **The reset register is read where ACPI puts it, and only when the flag says it is there.**
    /// The values are QEMU `q35`'s, as this kernel read them from its FADT on 2026-09-24 (`fadt reset
    /// register: Io 0xcf9 (8 bits) <- 0x0f`, `script/soak-test --reboot --arch x86_64`): system
    /// I/O, eight bits wide, port `0xCF9`, and the value `0x0F`.
    #[test]
    fn the_fadt_reset_register_is_read_only_when_the_flag_says_so() {
        let mut body = [0u8; FADT_RESET_VALUE_AT + 1];
        body[FADT_FLAGS_AT..][..4].copy_from_slice(&FADT_RESET_REG_SUP.to_le_bytes());
        body[FADT_RESET_REG_AT] = 1; // system I/O
        body[FADT_RESET_REG_AT + 1] = 8;
        body[FADT_RESET_REG_AT + 4..][..8].copy_from_slice(&0xcf9u64.to_le_bytes());
        body[FADT_RESET_VALUE_AT] = 0x0f;
        assert_eq!(
            parse_fadt_reset(&body),
            Some(ResetRegister {
                space: ResetSpace::Io,
                bit_width: 8,
                address: 0xcf9,
                value: 0x0f,
            })
        );

        // The same bytes with RESET_REG_SUP clear: firmware has not vouched for them.
        let mut unflagged = body;
        unflagged[FADT_FLAGS_AT..][..4].copy_from_slice(&0u32.to_le_bytes());
        assert_eq!(parse_fadt_reset(&unflagged), None);

        // Flagged but at address zero, which is how a table that has nothing to say spells it.
        let mut zero = body;
        zero[FADT_RESET_REG_AT + 4..][..8].copy_from_slice(&0u64.to_le_bytes());
        assert_eq!(parse_fadt_reset(&zero), None);

        // An ACPI 1.0 FADT ends before the field, at every length short of it, without panicking.
        for len in 0..body.len() {
            assert_eq!(parse_fadt_reset(&body[..len]), None, "length {len}");
        }
    }

    /// Build a well-formed table: signature, length, revision, a sealed checksum at offset 9, and
    /// a vendor name at offset 10.
    fn sdt(signature: &[u8; 4], body: &[u8]) -> [u8; 128] {
        let mut t = [0u8; 128];
        let len = SDT_HEADER_LEN + body.len();
        assert!(len <= 128);
        t[0..4].copy_from_slice(signature);
        t[4..8].copy_from_slice(&(len as u32).to_le_bytes());
        t[8] = 1;
        t[10..16].copy_from_slice(b"BOCHS ");
        t[SDT_HEADER_LEN..len].copy_from_slice(body);
        seal(&mut t, 9, len);
        t
    }

    /// **Every prefix shorter than a structure's fixed part is refused, and the shortest one that
    /// is long enough is accepted.** One loop per parser, over every length from nothing to one
    /// byte short.
    ///
    /// Both halves are the property, and the second is the one an "it returns an error" test
    /// leaves out. A guard written `<=` rather than `<` refuses a structure that is exactly long
    /// enough, and no malformed input can expose that: only the boundary length can. These four
    /// parsers read firmware bytes on the x86 boot path, where a guard one byte too loose reads
    /// past the buffer and a guard one byte too tight loses a table the machine really has.
    fn every_short_prefix_is_refused<T: core::fmt::Debug + PartialEq>(
        bytes: &[u8],
        fixed_len: usize,
        parse: impl Fn(&[u8]) -> Result<T, AcpiError>,
    ) {
        for len in 0..fixed_len {
            assert_eq!(
                parse(&bytes[..len]),
                Err(AcpiError::Truncated),
                "{len} bytes is short of the {fixed_len} this structure needs",
            );
        }
        assert!(
            parse(&bytes[..fixed_len]).is_ok(),
            "{fixed_len} bytes is exactly enough and must not be refused",
        );
    }

    /// A revision-0 RSDP, which is the whole 20-byte structure and has no extended half.
    fn rsdp_v1() -> [u8; RSDP_V1_LEN] {
        let mut b = [0u8; RSDP_V1_LEN];
        b[0..8].copy_from_slice(RSDP_SIGNATURE);
        b[9..15].copy_from_slice(b"BOCHS ");
        b[15] = 0; // revision 0: RSDT only
        b[16..20].copy_from_slice(&0x7ffe_1a40u32.to_le_bytes());
        seal(&mut b, 8, RSDP_V1_LEN);
        b
    }

    #[test]
    fn no_parser_reads_past_a_table_that_ends_early() {
        every_short_prefix_is_refused(&rsdp_v1(), RSDP_V1_LEN, parse_rsdp);
        every_short_prefix_is_refused(&rsdp_v2(), RSDP_V2_LEN, parse_rsdp);
        // An empty body, so the header's own length field is exactly the header length: the case
        // that separates `length < SDT_HEADER_LEN` from `length <= SDT_HEADER_LEN`.
        every_short_prefix_is_refused(&sdt(b"APIC", &[]), SDT_HEADER_LEN, parse_sdt_header);
        every_short_prefix_is_refused(&q35_madt_body(), MADT_FIXED_LEN, parse_madt);
        every_short_prefix_is_refused(&q35_dmar_body(), DMAR_FIXED_LEN, parse_dmar);
    }

    /// A table header's length must at least cover its own header, or the body length underflows.
    #[test]
    fn a_length_shorter_than_the_header_is_refused() {
        let mut t = sdt(b"APIC", &[]);
        t[4..8].copy_from_slice(&10u32.to_le_bytes());
        assert_eq!(parse_sdt_header(&t), Err(AcpiError::BadLength(10)));
    }

    /// The header's fields land where the specification puts them.
    #[test]
    fn a_table_header_decodes() {
        let t = sdt(b"MCFG", &[0u8; 24]);
        let h = parse_sdt_header(&t).expect("well-formed");
        assert_eq!(h.signature_str(), Some("MCFG"));
        assert_eq!(h.length as usize, SDT_HEADER_LEN + 24);
        assert_eq!(h.body_len(), 24);
        assert_eq!(
            &h.oem_id, b"BOCHS ",
            "the vendor name starts at offset 10, after the checksum byte"
        );
        assert!(
            is_checksum_ok(&t[..h.length as usize]),
            "a sealed table sums to zero"
        );
    }

    /// Root-table entries are 4 bytes in an RSDT and 8 in an XSDT, and running off the end is
    /// `None` rather than a garbage pointer.
    #[test]
    fn root_entries_are_four_or_eight_bytes_and_the_end_is_none() {
        let body32 = [0x40u8, 0x1a, 0xfe, 0x7f, 0x00, 0x1b, 0xfe, 0x7f];
        assert_eq!(root_entry(&body32, 0, false), Some(0x7ffe_1a40));
        assert_eq!(root_entry(&body32, 1, false), Some(0x7ffe_1b00));
        assert_eq!(root_entry(&body32, 2, false), None);
        assert_eq!(root_entry_count(SDT_HEADER_LEN as u32 + 8, false), 2);
        assert_eq!(root_entry_count(SDT_HEADER_LEN as u32 + 8, true), 1);
    }

    /// The MADT QEMU's `q35` produces for one CPU: a processor, an IO APIC, and the two interrupt
    /// source overrides every PC has.
    fn q35_madt_body() -> [u8; 8 + 8 + 12 + 10 + 10] {
        let mut b = [0u8; 8 + 8 + 12 + 10 + 10];
        b[0..4].copy_from_slice(&0xfee0_0000u32.to_le_bytes());
        b[4..8].copy_from_slice(&MADT_PCAT_COMPAT.to_le_bytes());
        // Local APIC: processor 0, apic id 0, enabled.
        b[8] = 0;
        b[9] = 8;
        b[10] = 0;
        b[11] = 0;
        b[12..16].copy_from_slice(&1u32.to_le_bytes());
        // IO APIC: id 0 at 0xfec00000, gsi base 0.
        b[16] = 1;
        b[17] = 12;
        b[18] = 0;
        b[20..24].copy_from_slice(&0xfec0_0000u32.to_le_bytes());
        b[24..28].copy_from_slice(&0u32.to_le_bytes());
        // Interrupt source override: ISA IRQ 0 arrives as GSI 2.
        b[28] = 2;
        b[29] = 10;
        b[30] = 0; // bus 0 = ISA
        b[31] = 0; // source: IRQ 0
        b[32..36].copy_from_slice(&2u32.to_le_bytes());
        b[36..38].copy_from_slice(&0u16.to_le_bytes());
        // Interrupt source override: ISA IRQ 5, active high, level triggered.
        b[38] = 2;
        b[39] = 10;
        b[40] = 0;
        b[41] = 5;
        b[42..46].copy_from_slice(&5u32.to_le_bytes());
        b[46..48].copy_from_slice(&0x000du16.to_le_bytes());
        b
    }

    /// The fixed part, and the PICs the machine still has.
    #[test]
    fn the_madt_reports_the_local_apic_and_whether_the_8259s_are_there() {
        let m = parse_madt(&q35_madt_body()).expect("well-formed");
        assert_eq!(m.local_apic, 0xfee0_0000);
        assert_ne!(
            m.flags & MADT_PCAT_COMPAT,
            0,
            "a PC still has 8259s and they must be masked before the APICs are used"
        );
    }

    /// **The entry list decodes, including the overrides that make legacy IRQ numbers a lie.** On
    /// almost every machine the timer's IRQ 0 arrives as GSI 2, and a kernel that armed input 0
    /// would arm nothing.
    #[test]
    fn the_madt_entries_decode_and_the_overrides_rewire_legacy_irqs() {
        let body = q35_madt_body();
        let mut it = madt_entries(&body);
        assert_eq!(
            it.next(),
            Some(MadtEntry::LocalApic {
                processor_id: 0,
                apic_id: 0,
                enabled: true,
                online_capable: false,
            })
        );
        assert_eq!(
            it.next(),
            Some(MadtEntry::IoApic {
                id: 0,
                address: 0xfec0_0000,
                gsi_base: 0,
            })
        );
        assert_eq!(
            it.next(),
            Some(MadtEntry::InterruptSourceOverride {
                bus: 0,
                source: 0,
                gsi: 2,
                flags: 0,
            }),
            "IRQ 0 is remapped on essentially every PC"
        );
        assert_eq!(
            it.next(),
            Some(MadtEntry::InterruptSourceOverride {
                bus: 0,
                source: 5,
                gsi: 5,
                flags: 0x000d,
            })
        );
        assert_eq!(it.next(), None);
    }

    /// **A malformed entry length ends the walk rather than looping forever.** A length of zero
    /// would never advance, and a length past the end would read the next entry from the middle of
    /// this one.
    #[test]
    fn a_zero_or_overlong_entry_length_ends_the_walk() {
        let mut body = q35_madt_body();
        body[9] = 0; // the first entry claims zero length
        assert_eq!(madt_entries(&body).count(), 0);

        let mut body = q35_madt_body();
        body[17] = 200; // the second entry claims to run past the table
        assert_eq!(
            madt_entries(&body).count(),
            1,
            "the entries before the bad one are still reported"
        );
    }

    /// A CPU that is present but not enabled is not startable, and online-capable is a third state
    /// rather than a synonym for enabled.
    #[test]
    fn a_disabled_processor_is_reported_as_disabled() {
        let mut body = q35_madt_body();
        body[12..16].copy_from_slice(&2u32.to_le_bytes()); // online-capable, not enabled
        let first = madt_entries(&body).next().expect("an entry");
        assert_eq!(
            first,
            MadtEntry::LocalApic {
                processor_id: 0,
                apic_id: 0,
                enabled: false,
                online_capable: true,
            }
        );
    }

    /// **The trap, resolved.** IRQ 0 is the timer and it is not IO APIC input 0; the override says
    /// it arrives as GSI 2. A kernel that armed redirection entry 0 would arm the 8259 cascade and
    /// see nothing at all, with no error anywhere to say why.
    #[test]
    fn the_timers_irq_0_resolves_to_gsi_2() {
        let table = isa_irq_table(&q35_madt_body());
        assert_eq!(
            table[0],
            IsaIrqRouting {
                gsi: 2,
                active_low: false,
                level_triggered: false,
            },
            "IRQ 0 is remapped on essentially every PC"
        );
    }

    /// An IRQ with no override keeps its number and the ISA bus's own polarity and trigger mode.
    #[test]
    fn an_irq_with_no_override_is_identity_mapped_edge_triggered_and_active_high() {
        let table = isa_irq_table(&q35_madt_body());
        for irq in [1usize, 4, 8, 15] {
            assert_eq!(
                table[irq],
                IsaIrqRouting::isa_default(irq as u8),
                "IRQ {irq} has no override in this MADT"
            );
        }
        assert_eq!(table[4].gsi, 4, "COM1 really is GSI 4 on a PC");
    }

    /// **The flags word is two two-bit fields, and reading it as two one-bit flags gets both
    /// wrong.** `0x000d` is `0b1101`: bits 1:0 are `01`, active *high*, and bits 3:2 are `11`,
    /// level triggered. A decoder that tested bit 1 for polarity and bit 3 for trigger would answer
    /// "active low, level" here, which is a redirection entry that never fires.
    #[test]
    fn the_inti_flags_are_two_two_bit_fields_not_two_bits() {
        let table = isa_irq_table(&q35_madt_body());
        assert_eq!(
            table[5],
            IsaIrqRouting {
                gsi: 5,
                active_low: false,
                level_triggered: true,
            },
        );
    }

    /// Active low is `11` in bits 1:0, and it is the encoding a PCI line uses.
    #[test]
    fn active_low_is_the_11_encoding() {
        let mut body = q35_madt_body();
        body[46..48].copy_from_slice(&0b1111u16.to_le_bytes());
        let table = isa_irq_table(&body);
        assert!(table[5].active_low);
        assert!(table[5].level_triggered);
    }

    /// **A `00` field means "conforms to the bus", not "active high, edge" by accident.** The two
    /// happen to coincide on the ISA bus, and the test exists so that a future reader changing the
    /// default cannot do it in only one of the two places.
    #[test]
    fn a_conforms_to_the_bus_override_keeps_the_isa_defaults_but_takes_the_new_gsi() {
        let mut body = q35_madt_body();
        // Rewrite the second override: IRQ 9 -> GSI 9, flags 0 (conforms).
        body[41] = 9;
        body[42..46].copy_from_slice(&9u32.to_le_bytes());
        body[46..48].copy_from_slice(&0u16.to_le_bytes());
        let table = isa_irq_table(&body);
        assert_eq!(table[9], IsaIrqRouting::isa_default(9));
        assert_eq!(
            table[5],
            IsaIrqRouting::isa_default(5),
            "IRQ 5 is back to itself"
        );
    }

    /// An override naming a source outside the sixteen legacy IRQs is a malformed table, not a
    /// seventeenth IRQ, and must not index past the end of the table.
    #[test]
    fn an_override_for_a_source_above_15_is_ignored() {
        let mut body = q35_madt_body();
        body[41] = 200;
        let table = isa_irq_table(&body);
        for (irq, routing) in table.iter().enumerate().skip(1) {
            assert_eq!(*routing, IsaIrqRouting::isa_default(irq as u8));
        }
        assert_eq!(table[0].gsi, 2, "the well-formed override still applied");
    }

    /// **A revision-2 RSDP whose XSDT pointer is zero still uses the RSDT.**
    ///
    /// Both halves of the choice have to hold: firmware that claims ACPI 2.0 and then writes no
    /// extended pointer has published exactly one root table, and following the zero would send
    /// the kernel to physical address 0. Every other fixture here has both fields set, where the
    /// revision alone decides and the second half of the condition is never asked.
    #[test]
    fn a_revision_2_rsdp_with_no_xsdt_falls_back_to_the_rsdt() {
        let mut b = rsdp_v2();
        b[24..32].copy_from_slice(&0u64.to_le_bytes());
        seal(&mut b, 32, RSDP_V2_LEN);
        let r = parse_rsdp(&b).expect("well-formed, just without an XSDT");
        assert_eq!(r.revision, 2);
        assert_eq!(r.xsdt, 0);
        assert_eq!(
            r.root_table(),
            (0x7ffe_1a40, false),
            "a null XSDT is not a root table, whatever the revision claims",
        );
    }

    /// **The shortest entry the MADT's list can hold is still read.**
    ///
    /// Two bytes, a type and a length, and the list ends exactly where the body does. Every other
    /// MADT here has entries this decoder acts on, which are eight bytes and up, so the walk's own
    /// bounds have only ever been asked about lengths with room to spare either side.
    #[test]
    fn the_shortest_entry_the_list_can_hold_is_read_to_the_last_byte() {
        let mut body = [0u8; MADT_FIXED_LEN + 2];
        body[MADT_FIXED_LEN] = 9; // a type this decoder does not act on
        body[MADT_FIXED_LEN + 1] = 2; // the shortest self-describing length there is

        let mut it = madt_entries(&body);
        assert_eq!(it.next(), Some(MadtEntry::Other(9)));
        assert_eq!(it.next(), None);
    }

    /// **An entry of a kind this decoder reads, too short to hold that kind's fields, is reported
    /// by its type rather than decoded.**
    ///
    /// The four guards are the only thing between a malformed table and a read past the entry: a
    /// four-byte processor entry has no flags word, and decoding one anyway reads whatever follows
    /// it in the list. Firmware writes these lengths; nothing else checks them.
    #[test]
    fn an_entry_too_short_for_its_own_kind_is_not_decoded() {
        for kind in [0u8, 1, 2, 5] {
            let mut body = [0u8; MADT_FIXED_LEN + 4];
            body[MADT_FIXED_LEN] = kind;
            body[MADT_FIXED_LEN + 1] = 4;

            let mut it = madt_entries(&body);
            assert_eq!(
                it.next(),
                Some(MadtEntry::Other(kind)),
                "a four-byte entry of type {kind} holds none of that type's fields",
            );
            assert_eq!(it.next(), None);
        }
    }

    /// **The local APIC address override decodes**, which is the one entry kind nothing else here
    /// reads. A machine whose local APIC sits above 4 GiB can say so only in this entry, because
    /// the fixed part's field is 32 bits wide; a decoder that reported it as an unrecognised type
    /// would use the 32-bit address and touch memory that is not the APIC.
    #[test]
    fn a_local_apic_address_override_carries_a_64_bit_address() {
        let mut body = [0u8; MADT_FIXED_LEN + 12];
        body[MADT_FIXED_LEN] = 5;
        body[MADT_FIXED_LEN + 1] = 12;
        body[MADT_FIXED_LEN + 4..MADT_FIXED_LEN + 12]
            .copy_from_slice(&0x0000_0001_fee0_0000u64.to_le_bytes());

        let mut it = madt_entries(&body);
        assert_eq!(
            it.next(),
            Some(MadtEntry::LocalApicAddressOverride(0x1_fee0_0000)),
        );
        assert_eq!(it.next(), None);
    }

    /// **An override naming source 16 is ignored**, and 16 is the number that has to be exact.
    /// There are sixteen legacy IRQs, so 16 is the first value that is not one of them, and a
    /// bound read as inclusive writes past the sixteen-entry table. The existing refusal test uses
    /// 200, which any reading of the bound turns away.
    #[test]
    fn an_override_for_the_first_source_past_the_legacy_sixteen_is_ignored() {
        let mut body = q35_madt_body();
        body[41] = 16;
        let table = isa_irq_table(&body);
        for (irq, routing) in table.iter().enumerate().skip(1) {
            assert_eq!(*routing, IsaIrqRouting::isa_default(irq as u8));
        }
        assert_eq!(table[0].gsi, 2, "the well-formed override still applied");
    }

    /// **A window's size counts the buses between its ends, inclusive at both.**
    ///
    /// Three cases, and the single-bus one is the point: equal bus numbers are a window of one
    /// mebibyte rather than an empty one, which is what separates "ends before it begins" from
    /// "ends where it begins". The four-bus case starts away from bus zero, where a sum and a
    /// difference stop agreeing.
    #[test]
    fn an_ecam_window_counts_the_buses_between_its_ends_inclusive() {
        let one = McfgEntry {
            base: 0,
            segment: 0,
            start_bus: 7,
            end_bus: 7,
        };
        assert_eq!(one.size(), 0x10_0000, "one bus is one mebibyte");

        let four = McfgEntry {
            base: 0,
            segment: 0,
            start_bus: 2,
            end_bus: 5,
        };
        assert_eq!(four.size(), 4 * 0x10_0000, "buses 2, 3, 4 and 5");

        let backwards = McfgEntry {
            base: 0,
            segment: 0,
            start_bus: 8,
            end_bus: 7,
        };
        assert_eq!(
            backwards.size(),
            0,
            "nothing in the MCFG's encoding stops firmware writing this",
        );
    }

    /// **A second window, on a segment that is not zero, read from its own offsets.**
    ///
    /// One window at bus 0 on segment 0 hides every offset inside the entry, because each byte a
    /// wrong offset would reach is also zero: the segment can be read from the base's low half and
    /// still answer 0. Distinct values in every field are what make the offsets load-bearing.
    #[test]
    fn a_second_ecam_window_is_read_from_its_own_offsets() {
        let mut body = [0u8; MCFG_FIXED_LEN + 2 * MCFG_ENTRY_LEN];
        body[8..16].copy_from_slice(&0xb000_0000u64.to_le_bytes());
        body[16..18].copy_from_slice(&0x0102u16.to_le_bytes());
        body[18] = 0;
        body[19] = 255;
        let at = MCFG_FIXED_LEN + MCFG_ENTRY_LEN;
        body[at..at + 8].copy_from_slice(&0xc000_0000u64.to_le_bytes());
        body[at + 8..at + 10].copy_from_slice(&0x0304u16.to_le_bytes());
        body[at + 10] = 16;
        body[at + 11] = 31;

        let first = mcfg_entry(&body, 0).expect("two windows");
        assert_eq!(
            first.segment, 0x0102,
            "the segment is its own field, not the base's low half",
        );

        let second = mcfg_entry(&body, 1).expect("two windows");
        assert_eq!(second.base, 0xc000_0000);
        assert_eq!(second.segment, 0x0304);
        assert_eq!(second.start_bus, 16);
        assert_eq!(second.end_bus, 31);
        assert_eq!(second.size(), 16 * 0x10_0000);
        assert_eq!(mcfg_entry(&body, 2), None);
    }

    /// The MCFG entry q35 produces: bus 0 through 255 at 0xb0000000.
    #[test]
    fn the_mcfg_gives_the_ecam_window_and_its_size() {
        let mut body = [0u8; MCFG_FIXED_LEN + MCFG_ENTRY_LEN];
        body[8..16].copy_from_slice(&0xb000_0000u64.to_le_bytes());
        body[16..18].copy_from_slice(&0u16.to_le_bytes());
        body[18] = 0;
        body[19] = 255;
        let e = mcfg_entry(&body, 0).expect("one window");
        assert_eq!(e.base, 0xb000_0000);
        assert_eq!(e.start_bus, 0);
        assert_eq!(e.end_bus, 255);
        assert_eq!(e.size(), 256 * 0x10_0000, "256 buses of 1 MiB each");
        assert_eq!(mcfg_entry(&body, 1), None);
    }

    /// The DMAR `-device intel-iommu` produces on `q35`: 39-bit host address width, interrupt
    /// remapping off, one DRHD (not the catch-all, QEMU names devices explicitly) at the address
    /// `Q35_HOST_BRIDGE_IOMMU_ADDR` names, `0xfed90000`.
    fn q35_dmar_body() -> [u8; DMAR_FIXED_LEN + DRHD_FIXED_LEN] {
        let mut b = [0u8; DMAR_FIXED_LEN + DRHD_FIXED_LEN];
        b[0] = 38; // host address width - 1: 39-bit
        b[1] = 0; // flags: no interrupt remapping
        // reserved[2..12] stays zero
        let d = DMAR_FIXED_LEN;
        b[d..d + 2].copy_from_slice(&0u16.to_le_bytes()); // type 0: DRHD
        b[d + 2..d + 4].copy_from_slice(&(DRHD_FIXED_LEN as u16).to_le_bytes());
        b[d + 4] = 0; // flags: not INCLUDE_PCI_ALL
        b[d + 5] = 0; // reserved
        b[d + 6..d + 8].copy_from_slice(&0u16.to_le_bytes()); // segment 0
        b[d + 8..d + 16].copy_from_slice(&0xfed9_0000u64.to_le_bytes());
        b
    }

    /// The fixed part decodes: the address width is the table's field plus one, and the flags
    /// byte round-trips unexamined.
    #[test]
    fn the_dmar_reports_the_address_width_and_flags() {
        let d = parse_dmar(&q35_dmar_body()).expect("well-formed");
        assert_eq!(
            d.host_address_width, 39,
            "the table stores width - 1; q35 reports 38 for a 39-bit width"
        );
        assert_eq!(d.flags, 0, "no interrupt remapping on this machine");
    }

    /// **The whole reason this table gets read**: a DRHD names where VT-d's register file is,
    /// and `first_drhd` is what `kernel/src/arch/x86_64/machine.rs` calls to find it.
    #[test]
    fn the_drhd_gives_the_register_base() {
        let body = q35_dmar_body();
        let d = first_drhd(&body).expect("one DRHD in this table");
        assert_eq!(d.register_base, 0xfed9_0000);
        assert_eq!(d.segment, 0);
        assert!(
            !d.include_pci_all,
            "q35 names its devices explicitly rather than using the catch-all"
        );
    }

    /// The iterator sees the DRHD by type code, alongside whatever else the list might hold, the
    /// same shape [`madt_entries`] proves for the MADT.
    #[test]
    fn dmar_structures_decodes_a_drhd_and_stops_at_the_end() {
        let body = q35_dmar_body();
        let mut it = dmar_structures(&body);
        assert_eq!(
            it.next(),
            Some(DmarEntry::Drhd(Drhd {
                include_pci_all: false,
                segment: 0,
                register_base: 0xfed9_0000,
                register_size: 4096,
            }))
        );
        assert_eq!(it.next(), None);
    }

    /// **Two of the shortest structures the list can hold, both read.**
    ///
    /// Four bytes each, a type and a length and nothing else, and the second one is what proves
    /// the walk advances by the length it read rather than jumping to the end. Every other DMAR
    /// here holds a single sixteen-byte DRHD, where a cursor that advanced wrongly and a walk that
    /// stopped early are indistinguishable from the right answer.
    #[test]
    fn two_shortest_remapping_structures_are_both_read() {
        let mut body = [0u8; DMAR_FIXED_LEN + 8];
        let d = DMAR_FIXED_LEN;
        body[d..d + 2].copy_from_slice(&7u16.to_le_bytes());
        body[d + 2..d + 4].copy_from_slice(&4u16.to_le_bytes());
        body[d + 4..d + 6].copy_from_slice(&8u16.to_le_bytes());
        body[d + 6..d + 8].copy_from_slice(&4u16.to_le_bytes());

        let mut it = dmar_structures(&body);
        assert_eq!(it.next(), Some(DmarEntry::Other(7)));
        assert_eq!(it.next(), Some(DmarEntry::Other(8)));
        assert_eq!(it.next(), None);
    }

    /// **A DRHD too short to hold its own register base is reported by its type**, not decoded.
    /// The guard is the only thing between a malformed table and reading eight bytes that are not
    /// there, and what a decoded one would yield is an MMIO address the IOMMU driver then writes
    /// to.
    #[test]
    fn a_drhd_too_short_for_its_own_fields_is_reported_by_type() {
        let mut body = [0u8; DMAR_FIXED_LEN + 8];
        let d = DMAR_FIXED_LEN;
        body[d..d + 2].copy_from_slice(&0u16.to_le_bytes()); // type 0: DRHD
        body[d + 2..d + 4].copy_from_slice(&8u16.to_le_bytes());

        let mut it = dmar_structures(&body);
        assert_eq!(it.next(), Some(DmarEntry::Other(0)));
        assert_eq!(it.next(), None);
    }

    /// A remapping-structure type this decoder does not act on is reported by its type code
    /// rather than silently dropped, so a boot print can say what it skipped.
    #[test]
    fn an_unrecognized_remapping_structure_is_reported_by_type() {
        let mut body = [0u8; DMAR_FIXED_LEN + 8].to_vec();
        body[DMAR_FIXED_LEN..DMAR_FIXED_LEN + 2].copy_from_slice(&1u16.to_le_bytes()); // RMRR
        body[DMAR_FIXED_LEN + 2..DMAR_FIXED_LEN + 4].copy_from_slice(&8u16.to_le_bytes());
        let mut it = dmar_structures(&body);
        assert_eq!(it.next(), Some(DmarEntry::Other(1)));
        assert_eq!(it.next(), None);
    }

    /// **A malformed structure length ends the walk rather than looping forever**, the same
    /// refusal [`madt_entries`] makes for the same reason: a zero length would never advance.
    #[test]
    fn a_zero_length_remapping_structure_ends_the_walk() {
        let mut body = q35_dmar_body().to_vec();
        body[DMAR_FIXED_LEN + 2] = 0;
        body[DMAR_FIXED_LEN + 3] = 0;
        assert_eq!(dmar_structures(&body).count(), 0);
    }

    extern crate std;

    /// A DRHD with its scope list, for the device-scope tests below.
    fn drhd(flags: u8, base: u64, scopes: &[&[u8]]) -> std::vec::Vec<u8> {
        let len = DRHD_FIXED_LEN + scopes.iter().map(|s| s.len()).sum::<usize>();
        let mut e = std::vec![0u8; DRHD_FIXED_LEN];
        e[0..2].copy_from_slice(&0u16.to_le_bytes());
        e[2..4].copy_from_slice(&(len as u16).to_le_bytes());
        e[4] = flags;
        e[8..16].copy_from_slice(&base.to_le_bytes());
        for s in scopes {
            e.extend_from_slice(s);
        }
        e
    }

    /// One device-scope entry: type, start bus, and a path of (device, function) hops.
    fn scope(kind: u8, start_bus: u8, path: &[(u8, u8)]) -> std::vec::Vec<u8> {
        let mut s = std::vec![
            kind,
            (SCOPE_FIXED_LEN + 2 * path.len()) as u8,
            0,
            0,
            0,
            start_bus
        ];
        for &(d, f) in path {
            s.push(d);
            s.push(f);
        }
        s
    }

    fn dmar(units: &[std::vec::Vec<u8>]) -> std::vec::Vec<u8> {
        let mut b = std::vec![0u8; DMAR_FIXED_LEN];
        b[0] = 38;
        for u in units {
            b.extend_from_slice(u);
        }
        b
    }

    /// The bus xenon is expected to have: a root port at 00:1d.0 with the NVMe on bus 1 below it.
    fn xenon_bus(bus: u8, dev: u8, func: u8) -> Option<(u8, u8)> {
        ((bus, dev, func) == (0, 0x1d, 0)).then_some((1, 1))
    }

    /// **The layout a client Intel machine publishes, and the reason this decoder exists.** The
    /// integrated-graphics unit first, naming 00:02.0 and nothing else, then the catch-all with its
    /// IOAPIC and HPET scopes. This is the `OptiPlex` 7040's as Linux prints it (flags 0x0 at
    /// 0xfed90000, flags 0x1 at 0xfed91000); xenon is the 7050, the next generation of the same
    /// board. `first_drhd` picks the graphics unit, and the NVMe is not behind it.
    #[test]
    fn on_a_client_intel_layout_the_nvme_belongs_to_the_catch_all_not_the_first_unit() {
        let body = dmar(&[
            drhd(0, 0xfed9_0000, &[&scope(SCOPE_PCI_ENDPOINT, 0, &[(2, 0)])]),
            drhd(
                1,
                0xfed9_1000,
                &[&scope(3, 0xf0, &[(0x1f, 0)]), &scope(4, 0, &[(0x1f, 0)])],
            ),
        ]);
        let u = DmarUnits::parse(&body);
        assert!(
            !u.truncated,
            "IOAPIC and HPET scopes are skipped, not errors"
        );
        assert_eq!(u.units().len(), 2);
        assert_eq!(first_drhd(&body).unwrap().register_base, 0xfed9_0000);
        let (unit, how) = u.owner(0, 1, 0, 0, &mut xenon_bus).unwrap().unwrap();
        assert_eq!(
            (unit.register_base, how),
            (0xfed9_1000, Ownership::CatchAll)
        );
        let (unit, how) = u.owner(0, 0, 2, 0, &mut xenon_bus).unwrap().unwrap();
        assert_eq!((unit.register_base, how), (0xfed9_0000, Ownership::Named));
    }

    /// **A bridge named as a sub-hierarchy owns everything below it**, and a device no scope names
    /// on a machine with no catch-all is owned by nobody. The second half is the verdict the bench
    /// must be able to print, and it is the shape QEMU writes when a host bridge bypasses the IOMMU.
    #[test]
    fn a_subhierarchy_scope_owns_its_buses_and_an_unnamed_device_has_no_owner() {
        let body = dmar(&[drhd(
            0,
            0xfed9_0000,
            &[&scope(SCOPE_PCI_SUBHIERARCHY, 0, &[(0x1d, 0)])],
        )]);
        let u = DmarUnits::parse(&body);
        assert_eq!(
            u.owner(0, 1, 0, 0, &mut xenon_bus).unwrap().unwrap().1,
            Ownership::UnderBridge(0, 0x1d, 0)
        );
        assert_eq!(u.owner(0, 0, 3, 0, &mut xenon_bus), Ok(None));
        assert_eq!(
            u.owner(1, 1, 0, 0, &mut xenon_bus),
            Ok(None),
            "another segment"
        );
    }

    /// **A two-hop path is resolved through the live bridge**: the first hop is 00:1d.0, its
    /// secondary bus is 1, and the endpoint is 01:00.0. A path whose first hop is not a bridge on
    /// this machine names nothing, rather than being read as a bus-0 device.
    #[test]
    fn a_multi_hop_endpoint_path_is_resolved_through_the_bridge_registers() {
        let body = dmar(&[drhd(
            0,
            0xfed9_2000,
            &[&scope(SCOPE_PCI_ENDPOINT, 0, &[(0x1d, 0), (0, 0)])],
        )]);
        let u = DmarUnits::parse(&body);
        assert_eq!(
            u.owner(0, 1, 0, 0, &mut xenon_bus).unwrap().unwrap().1,
            Ownership::Named
        );
        assert_eq!(u.owner(0, 1, 0, 0, &mut |_, _, _| None), Ok(None));
    }

    /// **A table that did not fit answers "unknown", never "the catch-all"**, because the dropped
    /// entry could have been the one naming the device.
    #[test]
    fn a_truncated_table_refuses_to_guess() {
        let many: std::vec::Vec<std::vec::Vec<u8>> = (0..MAX_SCOPES as u8 + 1)
            .map(|i| scope(SCOPE_PCI_ENDPOINT, 0, &[(i, 0)]))
            .collect();
        let refs: std::vec::Vec<&[u8]> = many.iter().map(|s| s.as_slice()).collect();
        let u = DmarUnits::parse(&dmar(&[drhd(1, 0xfed9_1000, &refs)]));
        assert!(u.truncated);
        assert_eq!(u.owner(0, 5, 0, 0, &mut xenon_bus), Err(()));
        assert_eq!(
            u.owner(0, 0, 3, 0, &mut xenon_bus).unwrap().unwrap().1,
            Ownership::Named,
            "a recorded scope still answers"
        );
    }

    /// Hostile scope lengths (zero, odd, past the end) stop the list and mark it, without a panic.
    #[test]
    fn a_malformed_scope_list_is_marked_rather_than_trusted() {
        for bad in [
            &[1u8, 0][..],
            &[1, 7, 0, 0, 0, 0, 0][..],
            &[1, 40, 0, 0, 0, 0, 1, 0][..],
        ] {
            let u = DmarUnits::parse(&dmar(&[drhd(1, 0xfed9_1000, &[bad])]));
            assert!(u.truncated, "{bad:?}");
            assert_eq!(u.units().len(), 1);
        }
        assert_eq!(DmarUnits::parse(&[]).units().len(), 0);
    }

    /// An RMRR with its scope list: base, last byte, then the scopes.
    fn rmrr(base: u64, limit: u64, scopes: &[&[u8]]) -> std::vec::Vec<u8> {
        let len = RMRR_FIXED_LEN + scopes.iter().map(|s| s.len()).sum::<usize>();
        let mut e = std::vec![0u8; RMRR_FIXED_LEN];
        e[0..2].copy_from_slice(&1u16.to_le_bytes());
        e[2..4].copy_from_slice(&(len as u16).to_le_bytes());
        e[8..16].copy_from_slice(&base.to_le_bytes());
        e[16..24].copy_from_slice(&limit.to_le_bytes());
        for s in scopes {
            e.extend_from_slice(s);
        }
        e
    }

    /// **The `OptiPlex` 7040's whole DMAR, rebuilt from what Linux printed about it** (milestone
    /// 594): the graphics unit naming 00:02.0, the catch-all naming its IOAPIC and HPET, an RMRR at
    /// `0xdb5d2000..=0xdb5f1fff` and one at `0xdd800000..=0xdfffffff`. Which device each RMRR names
    /// is not in that log; USB (00:14.0) and graphics (00:02.0) are what section 8.4 says RMRRs are
    /// for and what the two sizes fit (128 KiB of USB buffers, 40 MiB of stolen memory). The
    /// reconstruction is checked by length: the 7040's table header says `0xA8` bytes, and this is
    /// that many, with nothing to spare for a structure the reading left out.
    fn optiplex_7040_dmar() -> std::vec::Vec<u8> {
        dmar(&[
            drhd(0, 0xfed9_0000, &[&scope(SCOPE_PCI_ENDPOINT, 0, &[(2, 0)])]),
            drhd(
                1,
                0xfed9_1000,
                &[&scope(3, 0xf0, &[(0x1f, 0)]), &scope(4, 0, &[(0x1f, 0)])],
            ),
            rmrr(
                0xdb5d_2000,
                0xdb5f_1fff,
                &[&scope(SCOPE_PCI_ENDPOINT, 0, &[(0x14, 0)])],
            ),
            rmrr(
                0xdd80_0000,
                0xdfff_ffff,
                &[&scope(SCOPE_PCI_ENDPOINT, 0, &[(2, 0)])],
            ),
        ])
    }

    fn reserved(u: &DmarUnits, bus: u8, dev: u8, func: u8) -> std::vec::Vec<(u64, u64)> {
        let mut v = std::vec::Vec::new();
        u.reserved_for(0, bus, dev, func, &mut xenon_bus, &mut |r| {
            v.push((r.base, r.limit));
        });
        v
    }

    /// **Every device lands in its own unit's root table** (milestone 594, the proposal's
    /// two-unit host test). `owner_index` is the decision the kernel's `attach` routes on, so this
    /// is the routing proved without the registers: the GPU to the graphics unit (index 0), and
    /// the NVMe behind its root port, the USB controller, the root port itself and the Ethernet
    /// function to the catch-all (index 1). QEMU presents one unit, so this is the only place the
    /// two-unit route runs before xenon.
    #[test]
    fn on_the_7040s_table_each_device_routes_to_the_unit_that_owns_it() {
        let body = optiplex_7040_dmar();
        assert_eq!(SDT_HEADER_LEN + body.len(), 0xa8, "the 7040's DMAR length");
        let u = DmarUnits::parse(&body);
        assert!(!u.truncated);
        assert_eq!(u.units().len(), 2);
        for (bdf, want) in [
            ((0, 2, 0), (0, Ownership::Named)),
            ((1, 0, 0), (1, Ownership::CatchAll)),
            ((0, 0x14, 0), (1, Ownership::CatchAll)),
            ((0, 0x1d, 0), (1, Ownership::CatchAll)),
            ((0, 0x1f, 6), (1, Ownership::CatchAll)),
        ] {
            let (b, d, f) = bdf;
            assert_eq!(
                u.owner_index(0, b, d, f, &mut xenon_bus),
                Ok(Some(want)),
                "{bdf:x?}"
            );
        }
    }

    /// **Each RMRR is handed to exactly the device it names**, and to nothing else: the USB
    /// controller gets its 128 KiB, the GPU its 40 MiB, the NVMe none. The GPU's region is the one
    /// whose absence would blank xenon's screen the instant its unit translated, which is why the
    /// kernel maps it before it sets `GCMD.TE`.
    #[test]
    fn on_the_7040s_table_each_rmrr_goes_to_the_device_it_names() {
        let u = DmarUnits::parse(&optiplex_7040_dmar());
        assert_eq!(u.reserved_regions().len(), 2);
        assert_eq!(u.rmrrs_refused, 0);
        assert_eq!(reserved(&u, 0, 0x14, 0), [(0xdb5d_2000, 0xdb5f_1fff)]);
        assert_eq!(reserved(&u, 0, 2, 0), [(0xdd80_0000, 0xdfff_ffff)]);
        assert_eq!(u.reserved_regions()[1].size(), 40 << 20);
        assert!(reserved(&u, 1, 0, 0).is_empty());
        assert!(reserved(&u, 0, 0x14, 1).is_empty(), "another function");
    }

    /// An RMRR naming a bridge as a sub-hierarchy covers the functions below it, the case Linux's
    /// `is_downstream_to_pci_bridge` handles.
    #[test]
    fn an_rmrr_naming_a_bridge_covers_what_is_below_it() {
        let u = DmarUnits::parse(&dmar(&[
            drhd(1, 0xfed9_1000, &[]),
            rmrr(
                0x7000_0000,
                0x7000_ffff,
                &[&scope(SCOPE_PCI_SUBHIERARCHY, 0, &[(0x1d, 0)])],
            ),
        ]));
        assert_eq!(reserved(&u, 1, 0, 0), [(0x7000_0000, 0x7000_ffff)]);
        assert!(reserved(&u, 0, 2, 0).is_empty());
    }

    /// **An RMRR that breaks section 8.4's shape is counted and dropped**, never mapped: an
    /// unaligned base, a size that is not whole pages, a limit at or below the base, and a limit
    /// of the top address (whose `+ 1` would wrap). Mapping any of them would widen a domain by
    /// memory the firmware never declared.
    #[test]
    fn a_malformed_rmrr_is_refused_rather_than_mapped() {
        let s = scope(SCOPE_PCI_ENDPOINT, 0, &[(2, 0)]);
        for (base, limit) in [
            (0x1000_0800, 0x1000_ffff),
            (0x1000_0000, 0x1000_07ff),
            (0x1000_0000, 0x1000_0000),
            (0x2000_0000, 0x1000_0fff),
            (0xffff_ffff_ffff_f000, u64::MAX),
        ] {
            let u = DmarUnits::parse(&dmar(&[
                drhd(1, 0xfed9_1000, &[]),
                rmrr(base, limit, &[&s]),
            ]));
            assert_eq!(u.rmrrs_refused, 1, "{base:#x}..={limit:#x}");
            assert!(u.reserved_regions().is_empty());
            assert!(!u.truncated, "a refused region is not a truncated table");
        }
    }

    /// A ninth RMRR marks the table rather than vanishing, the same rule a ninth DRHD follows.
    #[test]
    fn a_ninth_rmrr_marks_the_table_truncated() {
        let s = scope(SCOPE_PCI_ENDPOINT, 0, &[(2, 0)]);
        let mut parts = std::vec![drhd(1, 0xfed9_1000, &[])];
        for i in 0..=MAX_RMRRS as u64 {
            parts.push(rmrr(
                0x1000_0000 + i * 0x1000,
                0x1000_0fff + i * 0x1000,
                &[&s],
            ));
        }
        let u = DmarUnits::parse(&dmar(&parts));
        assert_eq!(u.reserved_regions().len(), MAX_RMRRS);
        assert!(u.truncated);
    }

    /// **The register file is as big as the DRHD's `Size` byte says**: zero is one page (firmware
    /// that predates the field), three is eight pages. The kernel maps this much, not one page.
    #[test]
    fn a_drhds_size_byte_gives_its_register_span() {
        let mut body = dmar(&[drhd(1, 0xfed9_0000, &[])]);
        assert_eq!(DmarUnits::parse(&body).units()[0].register_size, 4096);
        body[DMAR_FIXED_LEN + 5] = 3;
        assert_eq!(DmarUnits::parse(&body).units()[0].register_size, 8 * 4096);
        body[DMAR_FIXED_LEN + 5] = 0xf3; // bits 7:4 are reserved and ignored
        assert_eq!(DmarUnits::parse(&body).units()[0].register_size, 8 * 4096);
    }

    // ---- survivor triage of the 2026-10-03 census ---------------------------------------------

    /// A structure of the smallest legal length (4: a type and a length) is skipped, and what comes
    /// after it is still read. A structure that claims more bytes than the table has ends the walk
    /// instead of slicing past it.
    #[test]
    fn dmar_walks_past_a_four_byte_structure_and_stops_at_one_that_overruns() {
        let tiny = [5u8, 0, 4, 0];
        let body = dmar(&[tiny.to_vec(), drhd(1, 0xfed9_1000, &[])]);
        let u = DmarUnits::parse(&body);
        assert_eq!(u.units().len(), 1);
        assert_eq!(u.units()[0].register_base, 0xfed9_1000);

        let mut body = dmar(&[drhd(1, 0xfed9_1000, &[])]);
        body.extend_from_slice(&[5, 0, 100, 0]);
        let u = DmarUnits::parse(&body);
        assert_eq!(u.units().len(), 1);
    }

    /// A DRHD or RMRR too short to hold its fixed part is not read as one. Reading it would index
    /// past what the table gave.
    #[test]
    fn a_dmar_structure_shorter_than_its_fixed_part_is_ignored() {
        let body = dmar(&[
            [0u8, 0, 4, 0].to_vec(),
            [1u8, 0, 4, 0].to_vec(),
            drhd(1, 0xfed9_1000, &[]),
        ]);
        let u = DmarUnits::parse(&body);
        assert_eq!(u.units().len(), 1);
        assert_eq!(u.reserved_regions().len(), 0);
    }

    /// The same for the MADT's three ARM kinds: a GICC, a distributor and a redistributor that stop
    /// after their header come back as `Other`.
    #[test]
    fn a_short_arm_madt_entry_is_other_and_not_read() {
        for kind in [11u8, 12, 14] {
            let mut body = std::vec![0u8; MADT_FIXED_LEN];
            body.extend_from_slice(&[kind, 4, 0, 0]);
            let got: std::vec::Vec<_> = madt_entries(&body).collect();
            assert_eq!(got, std::vec![MadtEntry::Other(kind)], "kind {kind}");
        }
    }

    /// A GICC's flags word: bit 0 is enabled and bit 1 is online-capable, and each is its own.
    #[test]
    fn a_gicc_reads_enabled_and_online_capable_separately() {
        let mut body = std::vec![0u8; MADT_FIXED_LEN];
        let mut e = std::vec![0u8; 16];
        e[0] = 11;
        e[1] = 16;
        e[12] = 2;
        body.extend_from_slice(&e);
        match madt_entries(&body).next() {
            Some(MadtEntry::GenericInterruptController {
                enabled,
                online_capable,
                ..
            }) => assert!(!enabled && online_capable),
            other => panic!("{other:?}"),
        }
    }

    /// A scope's path is bounded on both sides: none is nothing, four is the longest, and what
    /// `parse` records and `resolve` reads agree on where the limit is.
    #[test]
    fn a_scope_path_resolves_up_to_its_limit_and_no_further() {
        let mut s = DeviceScope {
            start_bus: 3,
            ..DeviceScope::default()
        };
        let direct = &mut |bus: u8, _: u8, _: u8| Some((bus + 1, 0));
        assert_eq!(s.resolve(direct), None, "an empty path names nothing");
        s.path_len = MAX_SCOPE_PATH as u8;
        for (i, hop) in s.path.iter_mut().enumerate() {
            *hop = (i as u8, 0);
        }
        // Three bridges are crossed (3 -> 4 -> 5 -> 6) and the last hop is the device on bus 6.
        assert_eq!(s.resolve(direct), Some((6, MAX_SCOPE_PATH as u8 - 1, 0)));
        s.path_len = MAX_SCOPE_PATH as u8 + 1;
        assert_eq!(s.resolve(direct), None);
    }

    /// A scope list is read hop by hop and keeps everything it was given: the starting bus, the
    /// path at every depth, and a path exactly as long as the limit.
    #[test]
    fn a_scope_keeps_its_start_bus_and_every_hop() {
        let path = [(0x1d, 3), (5, 7), (9, 11), (13, 15)];
        let u = DmarUnits::parse(&dmar(&[drhd(
            1,
            0xfed9_1000,
            &[&scope(SCOPE_PCI_ENDPOINT, 0x09, &path)],
        )]));
        assert!(!u.truncated);
        assert_eq!(u.scope_count, 1);
        assert_eq!(u.scopes[0].start_bus, 0x09);
        assert_eq!(u.scopes[0].path_len as usize, MAX_SCOPE_PATH);
        assert_eq!(u.scopes[0].path, path);
    }

    /// A scope with no hops, or with one more than the limit, is marked and not recorded, and the
    /// entries after it are still read.
    #[test]
    fn a_scope_with_no_hops_or_too_many_is_marked_and_the_list_goes_on() {
        let good = scope(SCOPE_PCI_ENDPOINT, 0, &[(2, 0)]);
        for bad in [
            scope(SCOPE_PCI_ENDPOINT, 0, &[]),
            scope(
                SCOPE_PCI_ENDPOINT,
                0,
                &[(1, 0), (2, 0), (3, 0), (4, 0), (5, 0)],
            ),
        ] {
            let u = DmarUnits::parse(&dmar(&[drhd(1, 0xfed9_1000, &[&bad, &good])]));
            assert!(u.truncated);
            assert_eq!(u.scope_count, 1, "only the good one is kept");
            assert_eq!(u.scopes[0].path[0], (2, 0));
        }
    }

    /// The FADT's ARM boot flags and its reset register sit at the offsets ACPI states (129, 112,
    /// 116 and 128), less the 36-byte header the body does not carry. PSCI and HVC are separate bits.
    #[test]
    fn the_fadt_fields_are_read_from_the_offsets_acpi_states() {
        let mut body = std::vec![0u8; 129 - SDT_HEADER_LEN + 2];
        body[129 - SDT_HEADER_LEN] = 0b10;
        let boot = parse_arm_boot(&body).unwrap();
        assert!(!boot.psci && boot.hvc, "flags 0b10 is HVC without PSCI");
        body[129 - SDT_HEADER_LEN] = 0b01;
        let boot = parse_arm_boot(&body).unwrap();
        assert!(boot.psci && !boot.hvc);

        let mut fadt = std::vec![0u8; 129 - SDT_HEADER_LEN];
        let flags = 112 - SDT_HEADER_LEN;
        fadt[flags..flags + 4].copy_from_slice(&(1u32 << 10).to_le_bytes());
        let reg = 116 - SDT_HEADER_LEN;
        fadt[reg] = 1;
        fadt[reg + 1] = 8;
        fadt[reg + 4..reg + 12].copy_from_slice(&0xcf9u64.to_le_bytes());
        fadt[128 - SDT_HEADER_LEN] = 0x06;
        let r = parse_fadt_reset(&fadt).expect("RESET_REG_SUP is set");
        assert_eq!(
            (r.space, r.bit_width, r.address, r.value),
            (ResetSpace::Io, 8, 0xcf9, 0x06)
        );
    }

    /// The GTDT interrupt flags are the bits ACPI numbers: bit 0 edge, bit 1 active low.
    #[test]
    fn the_gtdt_flag_bits_are_the_ones_acpi_numbers() {
        assert_eq!(GTDT_EDGE_TRIGGERED, 0b01);
        assert_eq!(GTDT_ACTIVE_LOW, 0b10);
    }
}
