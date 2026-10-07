//! E3's footprint-perturbation experiment (milestone 134,
//! design/roadmap/0134-the-measurements-that-decide.md): pad the IPC fastpath's call graph with a
//! large, reachable-but-never-taken function, so the fastpath's *measured* footprint
//! (`script/fastpath-footprint`) roughly doubles while the *executed* work on a benchmarked round
//! trip stays exactly what it was. Liedtke's claim is that footprint costs cache, not that any
//! particular byte is slow; padding tests that without a cache counter, by making the tool that
//! already measures footprint (milestone 132) disagree or agree with a real latency change.
//!
//! Gated on the `fastpath_pad` Cargo feature (kernel/Cargo.toml): this module, the call into it,
//! and the per-arch nop sleds it calls do not exist in an ordinary build, so nothing here costs a
//! normal boot anything. `crate::sched::ipc_send` calls [`maybe_pad`] unconditionally when the
//! feature is on; whether that call actually reaches the padding is decided at runtime by
//! [`core::hint::black_box`], not by a flag anyone sets, which is the property that makes the
//! padding *resident* (present in `.text`, walked by `script/fastpath-footprint`'s static closure)
//! while staying *dead* (never executed, so a benchmark run with the feature on does exactly the
//! IPC work a run without it does, plus this one guard).
//!
//! The guard itself is not free: `black_box(false)` still costs a real compare-and-branch on every
//! `ipc_send`, in both the padded and the un-padded-by-flag sense, because the feature being *on*
//! is what adds it, whether or not the branch is ever taken. That is a real, if small, confound on
//! E3's comparison and it is named in this milestone's roadmap doc rather than hidden: a purer
//! design would need the compiler to see "always false" at compile time, which is exactly the
//! condition LLVM is licensed to delete, undoing the whole point. `black_box` is the standard way
//! to keep a dead branch reachable without also making it free; the residual cost is on the order
//! of a nanosecond, which the observed IPC round trip (low microseconds under HVF) comfortably
//! dwarfs. See this file's callers for where the guard sits.
//!
//! # The layout control (2026-09-19, milestone 370)
//!
//! **Why a boolean was not enough.** On radon on 2026-09-04 the padded kernel was 1.49% slower on
//! `call_reply` and 3.01% *faster* on `ipc_rtt_el0`. The sled is never executed, so nothing it adds
//! can make a round trip faster; what it does is move other code. A shift changes which L1i sets
//! each hot line lands in, where 64-byte line boundaries fall, and whether a branch target is
//! 8-byte aligned, which the U74's BTB needs for a zero-bubble prediction (`SiFive`'s U74-MC Core
//! Complex Manual 21G3.02.00, §4.2.6). One padded kernel against one un-padded kernel therefore
//! reports footprint and layout summed.
//!
//! **Two numbers, one perturbation, both read at build time** by kernel/build.rs:
//!
//! - `NIFE_FASTPATH_PAD=<units>`, [`PAD_UNITS`]: the sled's length in units of what this feature
//!   has always linked (5,092 bytes on riscv64, 5,796 on aarch64). Unset is 1, the sled the
//!   2026-09-04 session booted. 0 is the dose-response's zero: the guard and a `ret`, so every rung
//!   executes the same instructions and the guard stops being a difference between conditions.
//! - `NIFE_FASTPATH_SHIFT=<bytes>`, [`SHIFT_BYTES`]: zero bytes appended after the sled's `ret`,
//!   under their own symbol (`fastpath_layout_shift`) that nothing references. Zeros rather than
//!   `nop`s because both ISAs decode zero as an illegal instruction: a block that were ever
//!   executed would trap rather than run.
//!
//! **The linker script places this module's section first in `.text`** (kernel/link-riscv64.ld,
//! kernel/link-aarch64.ld, right after the boot stub), and that is what makes the two numbers mean
//! the same thing: a sled of N bytes and a block of N bytes both move **the whole kernel text** by
//! N, and nothing else in the image changes. Before it was pinned, where the linker dropped the
//! section was redrawn every commit: on 2026-09-04 it landed ahead of the entire trap path, and by
//! 2026-09-19 it landed past all but 5% of the fastpath's bytes, so how much of the hot path a pad
//! perturbed was an accident. An ordinary build has no such section, so the line matches nothing
//! and the default link order is unchanged, checked by building the default boot image on both
//! ISAs across the change and comparing bytes.
//!
//! **What that buys E3, and it is the whole point.** A pad and a shift of the same displacement
//! produce **identical binaries** apart from the sled's own bytes: `PAD=1` and `SHIFT=5088` place
//! all 1,047 riscv64 text symbols at the same addresses. So the two ladders differ in exactly one
//! thing, whether the bytes are inside the footprint `script/fastpath-footprint` counts, which is
//! the variable E3 claims to be testing and the only one it can honestly isolate.
//!
//! The images E3 boots, their sizes and why, and how to read them are in
//! notes/footprint-perturbation.md, "The next radon evening".
//! `script/fastpath-footprint --layout` is the proof that every one of them executes the same
//! instructions: it hashes the hot path with addresses normalised away. Names are provisional.

/// The sled's length, in units of `UNIT_NOPS` (each arch's `fastpath_pad.rs`).
pub const PAD_UNITS: usize = parse(env!("NIFE_FASTPATH_PAD"));

/// Unreferenced zero bytes after the sled; non-zero only in a layout variant.
pub const SHIFT_BYTES: usize = parse(env!("NIFE_FASTPATH_SHIFT"));

/// The two numbers as data, for the bench boot to print (`bench-probe: fastpath_pad ...`), so a
/// card can say which E3 image it is. Data rather than immediates on purpose: on riscv64 `li a0, 0`
/// is two bytes and `li a0, 3280` is eight, so printing constants would change the length of the
/// printing function and move everything linked after it, which is the variable being controlled.
#[cfg(feature = "bench")]
pub static BUILD: [usize; 2] = [PAD_UNITS, SHIFT_BYTES];

/// Decimal, checked by kernel/build.rs before it gets here; a const fn because `str::parse` is not.
const fn parse(s: &str) -> usize {
    let b = s.as_bytes();
    let mut i = 0;
    let mut n = 0;
    while i < b.len() {
        n = n * 10 + (b[i] - b'0') as usize;
        i += 1;
    }
    n
}

/// Call from a root the fastpath-footprint closure walks (today: [`crate::sched::ipc_send`]).
/// Branches into the arch's nop sled only if [`core::hint::black_box`] fails to prove its input
/// false, which it never does, so this costs one untaken branch and nothing else at runtime.
#[inline(never)]
// In the pinned hot section, because every `ipc_send` in a padded build runs it: milestone 796
// (pin the hot trap path's placement).
// The sled it guards stays outside: never executed, it is what moves the rest of the text.
#[cfg_attr(
    target_os = "none",
    unsafe(link_section = ".text.hot.fastpath_pad.maybe_pad")
)]
pub fn maybe_pad() {
    if core::hint::black_box(false) {
        // SAFETY: the arch nop sled is a leaf function (no memory access, no stack frame beyond
        // its own `ret`) reached with the platform's ordinary call ABI; it touches nothing this
        // caller owns. It is also never actually called (see the module doc), so its body has no
        // runtime effect to be unsound about; this `unsafe` covers the FFI call shape only.
        unsafe { crate::arch::fastpath_pad_body() };
    }
}
