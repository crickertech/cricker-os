//! Console **input**, at EL0: the raw receive driver (milestone 19f.4, raw since milestone 28).
//!
//! The receive half of a terminal, reduced to what a driver actually is: own the UART's receive
//! side and its interrupt, and forward every byte that arrives. It assembles nothing, echoes
//! nothing, and interprets nothing; the editing, echo, and line assembly that used to live here
//! moved to the line discipline component (`line_editor`), where Unix keeps them too (a UART driver
//! feeds the tty layer; it is not the tty layer). What remains is the irreducible driver loop:
//! WAIT on the interrupt, drain the FIFO, hand the bytes on, ACK.
//!
//! Bytes travel packed in the words of an `OP_BYTES` CALL (up to 8 per message, the terminal
//! contract's driver half; see notes/terminal-contract.md). A keystroke is one byte and control
//! flow, not bulk data, so the words-in-registers path fits §10's rule; a paste drains in
//! 8-byte messages, and the CALL's rendezvous is the flow control that keeps a fast sender from
//! outrunning the discipline.
//!
//! Its whole authority: WRITE on the terminal endpoint (slot 0), and the device the machine gives
//! it. On aarch64/riscv64 that is the RX interrupt capability (slot 1) plus the UART registers
//! mapped device-typed; **on x86 (milestone 299) it is a `PortRange` capability for COM1's ports**,
//! polled rather than waited on, because COM1's receive line is not yet routed to a userspace waiter
//! there. It cannot print, spawn, or read what anyone else typed. No role selector; the syscall
//! runtime comes from `user_mode_runtime`.
//!
//! The arch-specific parts are the UART register layout and how a byte's arrival is learned, in the
//! `uart` module and the two `_start` arms below (aarch64 PL011, RISC-V NS16550, both
//! interrupt-driven; x86 16550 by port I/O, polled).
//!
//! Name: ratified 2026-07-30 (calef, DECISIONS §39), among the names recorded there as always
//! right.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

use line_editor::proto;
use user_mode_runtime::call;
#[cfg(not(target_arch = "x86_64"))]
use user_mode_runtime::{irq_ack, irq_wait};

// Unused on x86_64: there is no page for it to name (`user::UART_PHYS` is zero, DECISIONS §121),
// so the arm below traps instead of reading. Kept unconditional rather than cfg'd out because the
// address is the wiring's fact, agreed with the progenitor, and hiding it on one architecture would make the
// two sides of that agreement look like two different constants.
#[cfg_attr(target_arch = "x86_64", allow(dead_code))]
const UART_VA: u64 = address_space_map::pair_page(0x0000_0000_00a0_0000);

const TERM: u64 = 0; // CALL: forward raw wire bytes to the line discipline
#[cfg(not(target_arch = "x86_64"))]
const IRQ: u64 = 1; // WAIT / ACK the receive interrupt

/// The UART, the one arch-specific part of an input driver. aarch64's `virt` has a PL011 (32-bit
/// registers, RX-FIFO-empty and TX-FIFO-full flags in the Flag Register, a maskable RX interrupt in
/// IMSC, an explicit interrupt-clear register ICR). RISC-V's has an NS16550 (byte registers,
/// data-ready and transmit-holding-empty in the Line Status Register, RX interrupt enabled in IER,
/// and the RX interrupt is cleared by *reading* the received byte, so there is nothing to clear).
// Migrated onto `tock_registers` (milestone 139 round 5): every register offset checked at
// compile time instead of asserted by a hand-written comment, matching
// `kernel/src/drivers/pl011.rs`'s own idiom for the identical hardware. The RISC-V module below
// stays on plain volatile access, for the reason `kernel/src/drivers/ns16550.rs`'s own module doc
// gives: the NS16550's register *stride* is a runtime value (QEMU spaces its emulated registers
// one byte apart; the JH7110's real hardware spaces them four bytes apart, `reg-shift = <2>`),
// which `register_structs!`'s compile-time-fixed layout cannot express. The PL011 has no such
// knob, so it is the one half of this file the macro actually fits.
#[cfg(target_arch = "aarch64")]
mod uart {
    use tock_registers::interfaces::{ReadWriteable, Readable, Writeable};
    use tock_registers::registers::{ReadOnly, ReadWrite, WriteOnly};
    use tock_registers::{register_bitfields, register_structs};

    use super::UART_VA;

    register_bitfields! {
        u32,
        /// Flag register.
        FR [
            /// Receive FIFO empty.
            RXFE OFFSET(4) NUMBITS(1) [],
        ],
        /// Interrupt mask set/clear register.
        IMSC [
            /// Receive interrupt mask.
            RXIM OFFSET(4) NUMBITS(1) [],
        ],
    }

    register_structs! {
        /// The PL011's memory-mapped register block, the same layout
        /// `kernel/src/drivers/pl011.rs` verifies at compile time for the identical hardware,
        /// extended with IMSC and ICR (the receive-interrupt registers the kernel's own driver
        /// does not touch, but this driver does).
        #[allow(non_snake_case)]
        RegisterBlock {
            (0x00 => DR: ReadOnly<u32>),
            (0x04 => _reserved0),
            (0x18 => FR: ReadOnly<u32, FR::Register>),
            (0x1c => _reserved1),
            (0x38 => IMSC: ReadWrite<u32, IMSC::Register>),
            (0x3c => _reserved2),
            (0x44 => ICR: WriteOnly<u32>),
            (0x48 => @END),
        }
    }

    fn regs() -> &'static RegisterBlock {
        // SAFETY: UART_VA is our device mapping of the PL011, handed to us at spawn, for the
        // whole lifetime of this process. This is the same invariant the hand-written rd/wr calls
        // used to assert by comment; register_structs! now checks every offset above at compile
        // time instead.
        unsafe { &*(UART_VA as *const RegisterBlock) }
    }

    /// Name: ratified 2026-09-24 (calef, #1255 review). Refused `rx_pending` and `is_rx_pending`
    /// (`rx` is a decoder for "receive").
    pub fn is_byte_waiting() -> bool {
        !regs().FR.is_set(FR::RXFE)
    }
    /// Name: provisional, flagged 2026-09-25 by the lane that re-derived the x86 port
    /// falsifications (design/naming/boolean-predicates-worklist.md, "`rx` and `tx`"). calef asked
    /// what `rx` stands for in his #1255 review; recommended `read_byte`, because it reads the byte
    /// `is_byte_waiting` reports.
    pub fn rx_get() -> u8 {
        regs().DR.get() as u8
    }
    /// Name: provisional, flagged 2026-09-25 by the lane that re-derived the x86 port
    /// falsifications (design/naming/boolean-predicates-worklist.md, "`rx` and `tx`"). calef asked
    /// what `rx` stands for in his #1255 review; recommended `arm_receive_interrupt`, the word the
    /// NS16550 and PL011 manuals spell out.
    pub fn arm_rx_interrupt() {
        regs().IMSC.modify(IMSC::RXIM::SET);
    }
    pub fn clear_interrupt() {
        regs().ICR.set(0x7ff);
    }
}

#[cfg(target_arch = "riscv64")]
mod uart {
    use super::UART_VA;
    const RBR: u64 = 0x00; // receive buffer (read)
    const IER: u64 = 0x01; // interrupt enable
    const LSR: u64 = 0x05; // line status
    const IER_ERBFI: u8 = 1 << 0; // enable received-data-available interrupt
    const LSR_DR: u8 = 1 << 0; // data ready

    fn rd(off: u64) -> u8 {
        // SAFETY: UART_VA is our device mapping of the NS16550.
        unsafe { core::ptr::read_volatile((UART_VA + off) as *const u8) }
    }
    fn wr(off: u64, v: u8) {
        // SAFETY: as above.
        unsafe { core::ptr::write_volatile((UART_VA + off) as *mut u8, v) }
    }

    /// Name: ratified 2026-09-24 (calef, #1255 review). Refused `rx_pending` and `is_rx_pending`
    /// (`rx` is a decoder for "receive").
    pub fn is_byte_waiting() -> bool {
        rd(LSR) & LSR_DR != 0
    }
    /// Name: provisional, flagged 2026-09-25 by the lane that re-derived the x86 port
    /// falsifications (design/naming/boolean-predicates-worklist.md, "`rx` and `tx`"). calef asked
    /// what `rx` stands for in his #1255 review; recommended `read_byte`, because it reads the byte
    /// `is_byte_waiting` reports.
    pub fn rx_get() -> u8 {
        rd(RBR) // reading clears the receive interrupt; that is why clear_interrupt is a no-op
    }
    /// Name: provisional, flagged 2026-09-25 by the lane that re-derived the x86 port
    /// falsifications (design/naming/boolean-predicates-worklist.md, "`rx` and `tx`"). calef asked
    /// what `rx` stands for in his #1255 review; recommended `arm_receive_interrupt`, the word the
    /// NS16550 and PL011 manuals spell out.
    pub fn arm_rx_interrupt() {
        wr(IER, IER_ERBFI);
    }
    pub fn clear_interrupt() {
        // The NS16550 clears the receive interrupt when the byte is read (rx_get, in drain).
    }
}

/// **The x86 twin: COM1's receive side by port I/O** (milestone 299, DECISIONS §121 reversed
/// 2026-09-15). The two arms above differ in a register layout; this one differs in kind. COM1's
/// 16550 lives at I/O ports `0x3F8..=0x3FF`, reached only by `in`/`out`, which ring 3 may execute
/// only for a port it holds a capability to. This driver holds the `(0x3F8, 8)` port range the
/// progenitor delegated it, so the kernel's TSS I/O bitmap permits these eight ports and no others.
///
/// **The interrupt arm is a poll on x86** (see [`_start`]), so there is nothing to arm or
/// acknowledge: COM1's receive line (legacy IRQ 4) is not yet routed to a userspace
/// waiter on this architecture (the kernel delivers only self-directed vectors to a driver today,
/// `kernel/src/arch/x86_64/exceptions.rs`), so the driver reads the port it holds and yields between
/// reads rather than blocking on an interrupt it would never see. Interrupt-driven x86 input is a
/// follow-up; the port capability this milestone adds is what the poll reads through, and the
/// register layout below is exactly what the interrupt driver will use once the line is wired.
#[cfg(target_arch = "x86_64")]
mod uart {
    use user_mode_runtime::inb;
    const RBR: u16 = 0x3F8; // receive buffer (COM1 base)
    const LSR: u16 = 0x3FD; // line status register (base + 5)
    const LSR_DR: u8 = 1 << 0; // data ready

    /// Name: ratified 2026-09-24 (calef, #1255 review). Refused `rx_pending` and `is_rx_pending`
    /// (`rx` is a decoder for "receive").
    pub fn is_byte_waiting() -> bool {
        inb(LSR) & LSR_DR != 0
    }
    /// Name: provisional, flagged 2026-09-25 by the lane that re-derived the x86 port
    /// falsifications (design/naming/boolean-predicates-worklist.md, "`rx` and `tx`"). calef asked
    /// what `rx` stands for in his #1255 review; recommended `read_byte`, because it reads the byte
    /// `is_byte_waiting` reports.
    pub fn rx_get() -> u8 {
        inb(RBR) // reading clears the receive condition, as on the NS16550
    }
    // No `arm_rx_interrupt`/`clear_interrupt` here: this arm polls (see `_start`), so it never arms
    // or acknowledges a line. The interrupt-driven follow-up adds them, driving IER (`base + 1`) the
    // way the riscv64 NS16550 arm above drives it.
}

/// Forward wire bytes forever. No arguments: a standalone binary.
///
/// **Interrupt-driven on aarch64 and riscv64**: WAIT on the receive interrupt, drain the FIFO, hand
/// the bytes on, ACK. The one arch-specific fact besides the register layout is on x86, which polls
/// instead; see the `_start` twin below.
#[cfg(not(target_arch = "x86_64"))]
#[unsafe(no_mangle)]
pub extern "C" fn _start(_x0: u64, _x1: u64, _x2: u64) -> ! {
    // Drain anything already in the FIFO by POLLING, before arming the interrupt: input piped in at
    // boot is sitting in the FIFO already, and the first interrupt after arming can race with it.
    // This narrows the window but does NOT close it. A line burst-piped into QEMU during the few
    // instructions between the driver starting and this drain running can still lose its leading
    // character. A real user typing after the prompt never hits it, and every line after the first is
    // interrupt-driven and intact. Fully closing it needs the driver armed before any input arrives.
    drain();
    uart::arm_rx_interrupt();

    loop {
        irq_wait(IRQ);
        drain();
        uart::clear_interrupt(); // quiet the device (PL011: ICR; NS16550: reading RBR already did)
        irq_ack(IRQ); // re-enable the line at the controller now that the device is quiet
    }
}

/// **The x86 twin: poll the port, do not wait on an interrupt** (milestone 299). COM1's receive line
/// is not yet routed to a userspace waiter on this architecture (see the `uart` module doc), so this
/// driver reads the port range it holds and yields between reads, forwarding whatever arrived. It
/// burns cycles a blocked driver would not, which is why interrupt-driven x86 input is a named
/// follow-up rather than the end state; functionally it reaches the same prompt and forwards the
/// same keystrokes, through the same port capability. `yield_now` hands the CPU to the shell and the
/// line discipline between polls, so a cooperative or preempted schedule interleaves all three.
///
/// **What the poll costs, measured by milestone 182 (2026-09-19).** A thread that always yields is
/// always runnable, so the scheduler's run queue is never empty and its idle loop never runs again
/// once this driver starts. Two consequences follow. The core never halts: QEMU sat at 99 to 100%
/// of a host core at an `x86_64` prompt with nothing typed, 76 CPU-seconds in 78 wall-seconds. And
/// anything the idle loop does stops, `kernel::cap::report_peak` included, so the capability-slot
/// gauge reports the mark at the hand-over (5 of 24) instead of the peak (17 of 24, read by a
/// temporary instrument). Both go away with the interrupt-driven follow-up.
#[cfg(target_arch = "x86_64")]
#[unsafe(no_mangle)]
pub extern "C" fn _start(_x0: u64, _x1: u64, _x2: u64) -> ! {
    loop {
        drain();
        user_mode_runtime::yield_now();
    }
}

/// Read everything in the FIFO and forward it, up to 8 bytes per `OP_BYTES` message, packed
/// little-endian in the second word. The CALL blocks until the discipline has taken the bytes;
/// the FIFO fills while we wait, and we drain what accumulated on return.
fn drain() {
    loop {
        let mut word: u64 = 0;
        let mut n: u64 = 0;
        while n < 8 && uart::is_byte_waiting() {
            word |= (uart::rx_get() as u64) << (8 * n);
            n += 1;
        }
        if n == 0 {
            return;
        }
        call(TERM, proto::req(proto::OP_BYTES, n), word);
    }
}

user_mode_runtime::panic_handler!();
