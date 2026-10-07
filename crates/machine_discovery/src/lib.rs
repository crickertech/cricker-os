//! **What machine is this?** One record per architecture, populated once at boot, printed at boot.
//!
//! Milestone 60 (design/roadmap/0060-isa-discovery.md). Until this crate existed, the kernel ran on what the target
//! triple implied plus exactly one runtime measurement (the `satp.ASID` probe). Nothing read
//! `riscv,isa-extensions`, nothing read `mmu-type`, and nothing on aarch64 read an ID register
//! except the one `PARange` field that `TCR_EL1.IPS` needs. That is fine on an emulator, which is
//! configured to be whatever we ask for. It stops being fine on a board.
//!
//! # The three tiers, and the one RISC-V does not have
//!
//! The roadmap entry names three ways to find out what a machine is, in decreasing order of how
//! much you should like them:
//!
//! 1. **The firmware's claim.** A device tree property. Cheap, structured, and *hearsay*.
//! 2. **A targeted measurement.** Write a value, read back what stuck. Costs a little code and
//!    tells you what the silicon does rather than what somebody wrote about it.
//! 3. **Trap-and-detect.** Execute the instruction and catch the illegal-instruction fault. Last
//!    resort; needs the exception path to be up; we do not do this anywhere.
//!
//! Working on both ISAs at once exposes a tier the list is missing, and it is the reason the two
//! halves of this crate look so different. **aarch64 has a tier 0: the CPU describes itself.**
//! `MIDR_EL1` and the `ID_AA64*` registers are architected, mandatory, and read directly from the
//! part in front of you, so they are neither hearsay nor a measurement you have to design. RISC-V
//! deliberately removed that tier. `misa` exists but is coarse, is permitted to read as zero, and
//! says nothing about any extension ratified after 2015, so the architected answer is a device-tree
//! property written by firmware.
//!
//! That is why [`riscv64::Isa`] is a parser and [`aarch64::Isa`] is a decoder, and why the RISC-V
//! side keeps its tier-2 probe even though the tree now answers the same question. **The tree is a
//! claim and the probe is a measurement, and when they disagree the machine wins.** This project has
//! already been wrong about a QEMU boot register it believed the documentation about.
//!
//! # The trap this crate is written to avoid
//!
//! `if isa.has_x()` sprouting across the kernel turns a fact into a hundred branches. So the API is
//! deliberately awkward to sprinkle: the record is `Copy`, has public fields, and offers exactly two
//! verbs, [`riscv64::Isa::missing_requirements`] and [`aarch64::Isa::missing_requirements`], which
//! answer "can this machine run us at all". Everything else is for the boot print.
//!
//! # The third module, which belongs to neither
//!
//! [`cpu_list`] is milestone 100's, and it is here because both architectures answer the same
//! question the same way: `/cpus` holds a `cpu@` node per core with that core's hardware id in
//! `reg`, and nothing above it needs to know whether the id is an MPIDR affinity value or a hart id.
//! It sits beside the two arch records rather than inside either, which is the shape the question
//! has. `aarch64::Psci` is the counterpart that genuinely is arch-specific, and it lives in the
//! aarch64 module for exactly that reason.
//!
//! **There is no trait and no per-chip abstraction here**, and there should not be one until a
//! second real board says what the abstraction is (CLAUDE.md's rule against speculative
//! trait-ification). Two records that share no code is the honest shape when two ISAs answer the
//! same question by unrelated mechanisms.
//!
//! # BUGS
//!
//! - **Discovery does not make the kernel portable, it makes it honest.** Knowing an extension is
//!   missing and doing something useful about it are different milestones. Today the kernel does one
//!   thing with a missing requirement: it says so and stops.
//! - **The device tree can lie**, or firmware can describe a machine it is not. Tier 2 exists for
//!   that, and on RISC-V it currently covers exactly one fact (the ASID width).
//!
//! Name: ratified 2026-08-23 (calef, a kernel-dependency crate naming review). Renamed from
//! `isa`: collided with the ISA bus and only partially described the crate's scope. Matches
//! verified prior art (Linux ARM's `machine_desc`/`DT_MACHINE_START`, FreeBSD's `hw.machine`), both
//! of which use "machine" for this same device-tree-plus-probe scope. Introduced 2026-08-03,
//! one record per architecture for what the machine actually is.

#![no_std]
// milestone 68's ratchet is workspace-wide (§107); this crate opts out until its 54-item
// worklist (notes/doc-coverage.md) is burned down.
#![allow(missing_docs)]

// ACPI (milestone 161), and it sits here rather than in the x86_64 module for the reason
// `cpu_list` does: ACPI is not an x86 standard. Every aarch64 server that is not a device-tree
// board describes itself with the same tables, and milestone 20's own text expects the machine
// after the VisionFive 2 to be a UEFI/ACPI one. x86 is its first consumer, not its owner.
pub mod aarch64;
pub mod acpi;
pub mod cpu_list;
// Where the screen is (milestone 243), and it belongs to neither architecture for the same reason
// `cpu_list` does not: the loader asks the firmware where the linear framebuffer is and tells the
// kernel in one command-line token, and milestone 157's U-Boot handoff has the identical sentence
// to say on aarch64. x86_64 is its first consumer, not its owner.
pub mod framebuffer;
// Which Arm interrupt controller (milestone 227): aarch64's question, but a device-tree binding
// read plus a pure cross-check of raw ID-register words, so it is host logic like `plic` beside it.
pub mod gic;
pub mod interrupt_id;
pub mod plic;
pub mod riscv64;
// The third architecture's boot handoff (milestone 161). Not an ISA record like the two beside it:
// x86 answers "what CPU is this" from CPUID and needs no parser for that, but it has no device
// tree, so where RAM is and where the ACPI tables are arrive in the boot protocol's own structure.
// Same reason the RISC-V half is a parser: what the firmware says is a format somebody has to read,
// and a format read only inside a booting kernel is read by nothing that runs in milliseconds.
pub mod x86_64;
