//! The hardware abstraction boundary.
//!
//! Everything below this module is architecture-specific. Everything above it
//! should be portable. The rest of the kernel talks to the hardware only through
//! what is re-exported here.
//!
//! This is the single most important structural rule in the codebase, and the one
//! that is easiest to erode by accident. If you find yourself writing `asm!` or
//! touching a system register outside `arch/`, that's the bug. See
//! notes/portability.md.

#[cfg(target_arch = "aarch64")]
mod aarch64;

#[cfg(target_arch = "aarch64")]
pub use aarch64::*;

// The second architecture (milestone 20, notes/riscv-port.md). Its presence here, dispatched by
// the same `cfg` and re-exported flat through the same surface, is the proof that rule #1 holds: a
// new ISA is a new directory, not a diff across the kernel.
#[cfg(target_arch = "riscv64")]
mod riscv64;

#[cfg(target_arch = "riscv64")]
pub use riscv64::*;

// **riscv64's IOMMU driver, compiled for the prover on an aarch64 host.** This is milestone 432
// (the RISC-V IOMMU driver has no counterpart to the SMMU's proofs), built as option 1 of
// design/roadmap/650-riscv64-code-the-prover-can-already-compile.md. Kani compiles for
// the host, so the `cfg` above hides every riscv64 file from it; this reaches one of them by path.
// The inline module is named `riscv64` and holds `iommu`, so the harnesses keep the module path
// they would have natively (`arch::riscv64::iommu::proofs::...`) and `script/falsifications` finds
// their patches under the name it derives from the file. No `#[path]` is needed: an inline module
// in `arch/mod.rs` resolves `mod iommu;` to `arch/riscv64/iommu.rs` on its own.
//
// **A foot gun, deliberately taken, and read notes/kernel-proofs.md before extending it.** Inside
// this module `crate::arch` is the HOST's architecture, not riscv64's: `iommu.rs`'s
// `crate::arch::mmu::phys_to_virt` resolves to aarch64's here. Only code that never calls through
// `crate::arch` is proved by a harness in this module, and `script/lint` fails if a file listed
// below gains a `crate::arch` reference it has not recorded. Why only aarch64: exactly one verify
// host should run it, and whether x86_64 resolves the same names is unmeasured. Not riscv64
// either, where the real `mod riscv64` above already exists and nothing proves it.
//
// `dead_code` is allowed because nothing on the host calls `init` or `attach`; only the pure
// functions the harnesses reach are live here.
#[cfg(all(kani, target_arch = "aarch64"))]
#[allow(dead_code)]
mod riscv64 {
    mod iommu;
}

// The third architecture (milestone 161, notes/x86-port.md), and the one that tests whether the
// split above is real or an accident of two similar RISC machines. Same `cfg`, same flat
// re-export, no change anywhere else in this file: a new ISA is a new directory.
#[cfg(target_arch = "x86_64")]
mod x86_64;

#[cfg(target_arch = "x86_64")]
pub use x86_64::*;

/// **What this architecture is called**, in the spelling the rest of the tree already uses for it:
/// `cargo xtask test --arch`, `script/swish-check --arch`, and the target directory names.
///
/// It is here rather than in each architecture's own module because it is the one fact about an
/// architecture that is not about the hardware: it is what a person types and what a log is
/// grepped for. Milestone 268's machine-description summary line prints it, so `board_console` can
/// say which of the three answered without keeping three copies of three spellings.
///
/// Name: provisional (milestone 268 (every architecture boots the same way)).
#[cfg(target_arch = "aarch64")]
// The machine description and the boot self-test are the only callers, and both are
// `#[cfg(not(any(test, feature = "bench")))]`: a test boot exits through semihosting and a bench
// boot diverges into `bench::run`, so neither reads a bring-up transcript. Same treatment
// `memory::print_summary` already carries, and for the same reason.
#[cfg_attr(
    any(test, feature = "system_tests", feature = "bench"),
    allow(dead_code)
)]
pub const NAME: &str = "aarch64";
#[cfg(target_arch = "riscv64")]
#[cfg_attr(
    any(test, feature = "system_tests", feature = "bench"),
    allow(dead_code)
)]
pub const NAME: &str = "riscv64";
#[cfg(target_arch = "x86_64")]
#[cfg_attr(
    any(test, feature = "system_tests", feature = "bench"),
    allow(dead_code)
)]
pub const NAME: &str = "x86_64";

/// Which access a user thread was attempting when it faulted.
///
/// `Fetch` is not "a read of an instruction": the two arrive through different exception classes on
/// aarch64 (`0x20` vs `0x24`) and different `scause` codes on RISC-V (12 vs 13), and a test that
/// wanted a data read would be satisfied by an instruction fetch if we collapsed them.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum UserFaultAccess {
    Read,
    Write,
    Fetch,
}

impl UserFaultAccess {
    const fn encode(self) -> u64 {
        match self {
            UserFaultAccess::Read => 0,
            UserFaultAccess::Write => 1,
            UserFaultAccess::Fetch => 2,
        }
    }

    const fn decode(n: u64) -> Self {
        match n {
            0 => UserFaultAccess::Read,
            1 => UserFaultAccess::Write,
            _ => UserFaultAccess::Fetch,
        }
    }
}

/// **What the last user fault was, in terms both instruction sets can state.**
///
/// The distinction that carries the weight is `Permission` against `Translation`. A translation
/// fault means we merely failed to map something. A permission fault means the page **is there**,
/// the hardware found it, read the permission bits, and said no: that is the privilege boundary
/// doing its job rather than an accident of an incomplete page table. A test that asserts only
/// "something faulted" cannot tell those apart, and the sloppier of the two would pass it.
///
/// The two ISAs reach this answer very differently, and the difference is worth knowing:
///
/// - **aarch64 is told.** `ESR_EL1`'s fault status code says `permission fault, level 3` or
///   `translation fault, level 2` outright, in silicon, at the instant of the fault.
/// - **RISC-V is not.** `scause` has one code for "load page fault" and stops there; the
///   architecture does not report *why* the walk refused. So the RISC-V side derives it, by walking
///   the tables the hardware just walked and asking whether a translation exists. See
///   `riscv64::exceptions::user_fault` for the caveat that derivation carries.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum UserFault {
    /// The address translated, and the permission bits refused this access.
    Permission(UserFaultAccess),
    /// The address had no translation. Nothing was mapped, so nothing was refused.
    Translation(UserFaultAccess),
    /// Not a memory access at all: an illegal instruction, a breakpoint, a misaligned access.
    Other,
}

impl UserFault {
    /// Pack into the `u64` the fault path stores, so recording a fault is one relaxed store and
    /// needs no lock on a path that is already handling a fault. 0 is reserved for "no user thread
    /// has faulted yet", which is why every real code is non-zero.
    pub(crate) const fn encode(self) -> u64 {
        match self {
            UserFault::Permission(a) => 1 + a.encode(),
            UserFault::Translation(a) => 4 + a.encode(),
            UserFault::Other => 7,
        }
    }

    /// Unpack [`encode`](Self::encode). `None` for 0: no user thread has faulted.
    pub(crate) const fn decode(code: u64) -> Option<Self> {
        match code {
            0 => None,
            1..=3 => Some(UserFault::Permission(UserFaultAccess::decode(code - 1))),
            4..=6 => Some(UserFault::Translation(UserFaultAccess::decode(code - 4))),
            _ => Some(UserFault::Other),
        }
    }
}
