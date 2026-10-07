//! **Which Arm interrupt controller is this, and does the hardware agree?** The GIC's device tree
//! node, read by its binding, and a cross-check of what the silicon says about itself.
//!
//! Milestone 227 (design/roadmap/0227-gicv3-driver.md). Until this existed the kernel found its GIC
//! by the node-name prefix `intc@` and took the first two `reg` blocks as "distributor, CPU
//! interface". That is true of a GICv2 and false of a GICv3, whose second block is the
//! **redistributor** array. Milestone 222 measured what followed: a kernel that booted the whole
//! tour, brought four cores up, printed `interrupts ON`, and then took zero interrupts with nothing
//! faulting, because GICv2 CPU-interface writes landed in a redistributor frame and did nothing.
//!
//! So this module answers two questions, and the second is the one that makes the first safe:
//!
//! 1. [`discover`]: **what the firmware claims.** The node is found by `compatible`, never by its
//!    label, and the version is read from which binding matched. A node the old prefix would have
//!    matched but whose binding this kernel does not drive is a [`Refusal`], not a guess.
//! 2. [`Gic::confirm`]: **what the hardware says.** The architecture revision the controller
//!    reports about itself, read by the kernel and handed over as a raw word (this crate reads no
//!    registers, the same split as [`crate::aarch64::Isa::decode`]): `GICD_PIDR2.ArchRev` for a
//!    GICv3, `GICC_IIDR.ArchitectureVersion` for a GICv2, the latter read from the very block the
//!    tree calls the CPU interface. A tree that says one version while the hardware is another
//!    fails here, loudly, at boot, which is the whole point: the failure milestone 222 measured is
//!    impossible to reach silently once this runs.
//!
//! # Why `ID_AA64PFR0_EL1.GIC` is not one of the facts
//!
//! It was, in this module's first draft, and **the machine overruled it** (2026-09-19, the first
//! HVF boot): QEMU under Hypervisor.framework on an Apple core reports `ID_AA64PFR0_EL1` as
//! `0x1101000010110011`, whose `GIC` field (bits 27:24) is **zero**, while QEMU emulates the GICv3
//! system registers perfectly well behind it. The field describes the physical core, which has
//! Apple's AIC rather than a GIC, and the hypervisor does not rewrite it. Linux does not require it
//! either: its GICv3 driver sets `ICC_SRE_EL1.SRE` and reads it back, which is a measurement rather
//! than a claim, and the kernel's CPU-interface bring-up does exactly that and panics if the bit
//! will not stick. On a core that truly has no interface the first `ICC_*` access is UNDEFINED and
//! traps, which is loud too. The field is still the right guard where Linux uses it, at EL2
//! (`boot.s`), because there the question is whether the registers exist to be touched at all.
//!
//! # The two bindings, as QEMU 11.1.1 writes them (`dumpdtb`, both versions of `virt`)
//!
//! | | `gic-version=2` | `gic-version=3` |
//! |---|---|---|
//! | `compatible` | `arm,cortex-a15-gic` | `arm,gic-v3` |
//! | `reg[0]` | GICD, `0x8000000 + 0x10000` | GICD, `0x8000000 + 0x10000` |
//! | `reg[1]` | GICC, `0x8010000 + 0x10000` | GICR, `0x80a0000 + 0xf60000` |
//! | MSI child | `v2m@8020000` | `its@8080000` under TCG, **`v2m@8020000` under HVF** |
//!
//! The fixtures in `tests/fixtures/` are those dumps, round-tripped through `dtc`.
//!
//! # BUGS
//!
//! - **One redistributor region.** The binding allows several (`#redistributor-regions`), for
//!   machines whose redistributor frames are not contiguous; QEMU `virt` needs a second only past
//!   123 cores, and this kernel's `cpu::MAX_CPUS` is eight. A tree stating more than one is a
//!   [`Refusal::SeveralRedistributorRegions`] rather than a partial answer, so the day a board
//!   needs it the boot says so instead of losing the cores in the second region.
//! - **The GICv2 binding list is the three strings an aarch64 GICv2 actually carries**
//!   (`arm,gic-400`, `arm,cortex-a15-gic`, `arm,cortex-a7-gic`), checked against Linux's
//!   `drivers/irqchip/irq-gic.c` `IRQCHIP_DECLARE` table at v6.16. The older version-1 parts that
//!   table also lists (`arm,cortex-a9-gic`, `arm,pl390`, the 11MPCore family) exist only on 32-bit
//!   machines this kernel cannot boot, so they are left out rather than claimed.
//! - **The ITS is not read.** A GICv3's MSI translation service is a child node and a separate
//!   device; milestone 317 records why interrupt remapping wants it and nothing drives it yet.
//! - **This is aarch64's question.** On RISC-V a node named `interrupt-controller@...` is the PLIC
//!   and a child of every `cpu@` node is a `riscv,cpu-intc`; the kernel asks this only on aarch64.
//!
//! Name: provisional (`gic` for the module, `Gic` for the record, `Refusal` and `Mismatch` for the
//! two ways it says no), named for the controller whose binding it reads, the same pattern as
//! [`crate::plic`].

use device_tree_blob::{DeviceTreeBlob, Error, Region};

/// The binding a GICv3 states. GICv4 parts state it too (a GICv4 is a GICv3 with virtual LPIs).
pub const GICV3_COMPATIBLE: &[&[u8]] = &[b"arm,gic-v3"];

/// The bindings an aarch64 GICv2 states. See this module's BUGS for why the list stops here.
pub const GICV2_COMPATIBLE: &[&[u8]] =
    &[b"arm,gic-400", b"arm,cortex-a15-gic", b"arm,cortex-a7-gic"];

/// **The interrupt controller the tree describes**, with its register blocks in the roles its
/// binding gives them. Both addresses physical.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Gic {
    /// Memory-mapped distributor and a banked, memory-mapped CPU interface.
    V2 {
        /// `GICD`, one per machine.
        distributor: Region,
        /// `GICC`, one address, banked per core by the hardware.
        cpu_interface: Region,
    },
    /// Memory-mapped distributor, a redistributor frame per core, and a CPU interface that is
    /// **system registers** (`ICC_*`) rather than memory, so it has no region here at all.
    V3 {
        /// `GICD`, one per machine.
        distributor: Region,
        /// The redistributor frames, every core's, contiguous. Which frame belongs to which core
        /// is read from each frame's own `GICR_TYPER`, never computed from an index.
        redistributors: Region,
    },
}

/// Why [`discover`] would not name a controller.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Refusal<'a> {
    /// The blob could not be walked.
    Unreadable(Error),
    /// An `intc@` node exists, which is where the kernel used to look, but its `compatible` names
    /// no GIC this kernel drives. The bytes are the property verbatim (NUL-separated), so the
    /// boot line can say what the machine claimed rather than that it claimed something.
    UnknownController {
        /// The node's `compatible`, verbatim.
        compatible: &'a [u8],
    },
    /// A recognised binding with fewer `reg` blocks than it requires (two, for both versions).
    TooFewRegions {
        /// The version whose binding matched.
        version: u8,
        /// How many blocks the node had.
        found: usize,
    },
    /// A GICv3 whose redistributors are split across regions. See this module's BUGS.
    SeveralRedistributorRegions(u32),
}

/// **Find the GIC by its binding.** `Ok(None)` when the tree describes no interrupt controller
/// this module recognises and none at the old `intc@` label either, which is an honest "this
/// machine has no GIC" (RISC-V asks the PLIC instead, and x86 has no tree).
///
/// GICv3 is asked first. The order would only matter for a node stating both bindings, which no
/// tree does, but the newer binding is the more specific claim and so is the one to believe.
pub fn discover<'a>(dtb: &DeviceTreeBlob<'a>) -> Result<Option<Gic>, Refusal<'a>> {
    let mut regs = [Region { start: 0, size: 0 }; 4];

    for compat in GICV3_COMPATIBLE {
        let n = dtb
            .node_reg_compatible(compat, &mut regs)
            .map_err(Refusal::Unreadable)?;
        if n == 0 {
            continue;
        }
        if n < 2 {
            return Err(Refusal::TooFewRegions {
                version: 3,
                found: n,
            });
        }
        let regions = dtb
            .node_prop_compatible(compat, b"#redistributor-regions")
            .map_err(Refusal::Unreadable)?
            .and_then(|v| v.get(..4))
            .map_or(1, |b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]));
        if regions != 1 {
            return Err(Refusal::SeveralRedistributorRegions(regions));
        }
        return Ok(Some(Gic::V3 {
            distributor: regs[0],
            redistributors: regs[1],
        }));
    }

    for compat in GICV2_COMPATIBLE {
        let n = dtb
            .node_reg_compatible(compat, &mut regs)
            .map_err(Refusal::Unreadable)?;
        if n == 0 {
            continue;
        }
        if n < 2 {
            return Err(Refusal::TooFewRegions {
                version: 2,
                found: n,
            });
        }
        return Ok(Some(Gic::V2 {
            distributor: regs[0],
            cpu_interface: regs[1],
        }));
    }

    // Nothing recognised. If the old label is present, the machine HAS an interrupt controller and
    // we do not know how to drive it; guessing is exactly what milestone 222 measured the cost of.
    match dtb.node_prop(b"intc@", b"compatible") {
        Ok(Some(compatible)) => Err(Refusal::UnknownController { compatible }),
        Ok(None) => Ok(None),
        Err(e) => Err(Refusal::Unreadable(e)),
    }
}

/// Why [`Gic::confirm`] disbelieved the tree: the block it names reported an architecture
/// revision its binding does not allow. `field` is the revision as read (0 when nothing answered,
/// which is what a register block that is not the one the tree claims usually says).
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Mismatch {
    /// The version the tree claimed.
    pub claimed: u8,
    /// The revision field the hardware reported.
    pub field: u8,
}

impl Gic {
    /// The GIC architecture version the tree claimed.
    pub fn version(&self) -> u8 {
        match self {
            Gic::V2 { .. } => 2,
            Gic::V3 { .. } => 3,
        }
    }

    /// The distributor, which both versions have in the same place in their binding.
    pub fn distributor(&self) -> Region {
        match *self {
            Gic::V2 { distributor, .. } | Gic::V3 { distributor, .. } => distributor,
        }
    }

    /// The second block: the CPU interface of a GICv2, the redistributor array of a GICv3. Named
    /// for what the two have in common, which is that the kernel maps both as device memory.
    pub fn second_region(&self) -> Region {
        match *self {
            Gic::V2 { cpu_interface, .. } => cpu_interface,
            Gic::V3 { redistributors, .. } => redistributors,
        }
    }

    /// **Does the hardware agree with the tree?** `Ok` when it does.
    ///
    /// `identification` is, for a GICv3, `GICD_PIDR2` (distributor offset `0xffe8`), whose
    /// `ArchRev` (bits 7:4) is 3 or 4; for a GICv2, `GICC_IIDR` (CPU interface offset `0xfc`), whose
    /// `ArchitectureVersion` (bits 19:16) is 2 on a GIC-400 and on QEMU's model. A GICv3 in its
    /// legacy mode can present a GICv2-shaped CPU interface, so a GICv2 claim is also accepted from
    /// a block reporting 3 or 4. The converse is not: a GICv3 claim needs a distributor that says
    /// it is one.
    pub fn confirm(&self, identification: u32) -> Result<(), Mismatch> {
        let (claimed, field, allowed) = match self {
            Gic::V3 { .. } => (3, ((identification >> 4) & 0xf) as u8, 3..=4),
            Gic::V2 { .. } => (2, ((identification >> 16) & 0xf) as u8, 2..=4),
        };
        if allowed.contains(&field) {
            Ok(())
        } else {
            Err(Mismatch { claimed, field })
        }
    }
}
