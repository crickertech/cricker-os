//! The riscv64 half of E3's padding (see `kernel/src/fastpath_pad.rs`): one function, nothing but
//! `nop`, sized to roughly double `bench/fastpath-riscv64.txt`'s recorded `ipc_fastpath` (5,088
//! bytes at the time this was written). `.option norvc` forces the *uncompressed* 4-byte `nop`
//! encoding (`addi x0, x0, 0`) rather than the assembler's default 2-byte `c.nop`, so the byte
//! arithmetic matches aarch64's sled and stays easy to reason about: 1,272 `nop`s at 4 bytes each
//! is 5,088 bytes plus one `ret`. Re-measure with `script/fastpath-footprint --features
//! fastpath_pad --arch riscv64` after touching this file or the fastpath it is attached to, since
//! the target is the *current* baseline, not a constant.
//!
//! **This half exists for footprint symmetry only.** `script/fastpath-footprint` is a static
//! objdump measurement and runs on both ISAs with no QEMU involved, so a riscv64 padded build is
//! as meaningful to measure as the aarch64 one. E3's *second* half, the latency comparison, is
//! `--real`-only (there is no cache to perturb under TCG), and this tree has no riscv64
//! accelerator equivalent to HVF: `cargo xtask bench --riscv` always runs under TCG. So E3's
//! latency reading is aarch64-only today; see design/roadmap/0134-the-measurements-that-decide.md's
//! BUGS for the honest statement of that gap and notes/riscv-port.md for the general shape of it.
use core::arch::global_asm;

/// One sled unit: `fastpath_pad` alone links exactly this many `nop`s, as it did before the
/// ladder (milestone 134's layout control) made the count a multiple. See
/// `crate::fastpath_pad::PAD_UNITS` and, for the zero block after the sled,
/// `crate::fastpath_pad::SHIFT_BYTES`.
const UNIT_NOPS: usize = 1272;

unsafe extern "C" {
    /// The nop sled defined below. Never actually called (see `crate::fastpath_pad::maybe_pad`);
    /// declared so the reachable-but-dead call exists in the compiled binary for
    /// `script/fastpath-footprint`'s static closure walk to find.
    pub fn fastpath_pad_body();
}

global_asm!(
    "
    .pushsection .text.fastpath_pad_body,\"ax\",%progbits
    .global fastpath_pad_body
    .type fastpath_pad_body, %function
fastpath_pad_body:
    .option push
    .option norvc
    .rept {nops}
    nop
    .endr
    ret
    .option pop
    .if {shift}
    .global fastpath_layout_shift
    .type fastpath_layout_shift, %object
fastpath_layout_shift:
    .zero {shift}
    .size fastpath_layout_shift, {shift}
    .endif
    .popsection
    ",
    nops = const UNIT_NOPS * crate::fastpath_pad::PAD_UNITS,
    shift = const crate::fastpath_pad::SHIFT_BYTES,
);
