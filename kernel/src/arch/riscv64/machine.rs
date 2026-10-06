//! **The facts about a RISC-V machine that the kernel needs before it can read the device tree.**
//!
//! Exactly two, both about the console: where its registers are, and what the tree calls its node.
//! Everything else (RAM, the PLIC, the timebase, the harts) comes from the tree, and should. The
//! console cannot, because the tree parser is the code most likely to need debugging and `println!`
//! is how it gets debugged (the long version is at `console::UART_BASE`). So a new machine whose
//! UART is somewhere new needs a compile-time answer here, and this file is the one place that
//! answer lives.
//!
//! Until milestone 89 (Scaleway EM-RV1) the riscv64 address was written three times: in
//! `console.rs`, in `user.rs` (the progenitor's device capability) and in `mmu.rs` (the mapping). They
//! agreed because QEMU `virt` and the JH7110 both put UART0 at `0x1000_0000`. The TH1520 does not,
//! and a second machine would have meant editing three files, two of them outside `arch/`. Now it is
//! one cfg arm here. That move is counted as a seam repair in milestone 89's risk 9 tally.
//!
//! The machine is chosen by Cargo feature, the same mechanism `board` already is: one kernel image
//! per machine.
//!
//! Name: provisional, milestone 89 (Scaleway EM-RV1)'s lane, 2026-10-06 (UTC). Covers the module and the `board_th1520` feature.

/// The console UART's **physical** address.
///
/// QEMU `virt` and radon's JH7110 both put a 16550 here. The JH7110's is a `DesignWare` part with a
/// different register stride, which `console::configure_from_dtb` learns from the tree.
#[cfg(not(feature = "board_th1520"))]
pub const CONSOLE_UART_PHYS: u64 = 0x1000_0000;
/// The T-Head TH1520's UART0 (Scaleway's EM-RV1, the Lichee Pi 4A). 40 bits up, past what a
/// `pa + KERNEL_VA_BASE` direct map can name, so it is reached through `mmu`'s device window. From
/// Linux's `th1520.dtsi` (`serial@ffe7014000`); not yet read off the machine's own tree.
#[cfg(feature = "board_th1520")]
pub const CONSOLE_UART_PHYS: u64 = 0xff_e701_4000;

/// The console UART's node name in the device tree, carrying the address in its unit suffix.
/// `console::configure_from_dtb` reads the register shape from it and `memory::init` the
/// interrupt line. Witnessed by the fixture tests in `crates/machine_discovery/tests/`.
#[cfg(not(feature = "board_th1520"))]
pub const CONSOLE_UART_NODE: &[u8] = b"serial@10000000";
/// The TH1520's spelling, from `th1520.dtsi` and the fixture modeled on it.
#[cfg(feature = "board_th1520")]
pub const CONSOLE_UART_NODE: &[u8] = b"serial@ffe7014000";
