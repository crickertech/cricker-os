//! **Asking the host to terminate, `x86_64`.** The test harness's exit path.
//!
//! The module keeps the aarch64 name (`arch::semihosting::exit`) because it is the arch contract the
//! test harness calls, exactly as the RISC-V one does, and like that one the mechanism has nothing
//! to do with ARM semihosting. Here it is QEMU's `isa-debug-exit` device: a write of any width to
//! its I/O port terminates the guest with status `(value << 1) | 1`.
//!
//! **That encoding is why the codes below look odd, and it is a real constraint rather than a
//! quirk to route around.** Every exit status this device can produce is ODD, so exit code 0 is
//! unreachable: "success" has to be some agreed non-zero number, and the runner is what maps it
//! back. `0` written to the port yields host status 1; `1` yields 3. This module therefore reports
//! **3** for success and anything else for failure, and `helpers/qemu-runner-x86_64.sh` carries the
//! matching translation. Getting this backwards produces a suite that passes when it fails, which
//! is the one failure mode a test harness must not have, so both halves name the number.
//!
//! # BUGS
//!
//! - **A real machine has no `isa-debug-exit`.** Milestone 87's `OptiPlex` will need the same answer
//!   the VisionFive 2 needed: print a fixed marker line the serial harness can read, then power
//!   down (ACPI S5 rather than SBI SRST). Nothing here does that yet, so a test build run on real
//!   x86 hardware would write to a port nothing answers and fall through to the halt loop.

use core::arch::asm;

// Everything below is reachable only from the test harness and the test-mode panic arm, both
// `cfg(test)`, exactly as on the other two architectures.

/// The harness's success code. **Three, not zero**, because `isa-debug-exit` can only produce odd
/// statuses; see the module header.
#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
pub const EXIT_SUCCESS: u32 = 3;
/// The harness's failure code (any status that is not [`EXIT_SUCCESS`]).
#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
pub const EXIT_FAILURE: u32 = 1;

/// The `isa-debug-exit` device's I/O port, as `helpers/qemu-runner-x86_64.sh` places it
/// (`iobase=0xf4`). Not a fixed address in the machine: it is where we put the device, and the two
/// files have to agree.
#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
const DEBUG_EXIT_PORT: u16 = 0xf4;

/// Terminate the QEMU guest with `code`. [`EXIT_SUCCESS`] is a clean exit; anything else fails the
/// run.
///
/// The value written is `code >> 1` so that the status QEMU reports is `code` itself: the device
/// computes `(written << 1) | 1`, so writing 1 yields 3 and writing 0 yields 1. That arithmetic
/// lives here rather than at the call sites, which name only the two constants above.
#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
pub fn exit(code: u32) -> ! {
    // SAFETY: a write to the port `helpers/qemu-runner-x86_64.sh` attached `isa-debug-exit` to. The
    // write terminates the guest, so nothing after it runs.
    unsafe { super::port::out8(DEBUG_EXIT_PORT, (code >> 1) as u8) };

    // The device terminates the guest; if it somehow does not (a real machine, or a runner that did
    // not attach it), stop rather than run on.
    loop {
        // SAFETY: halt until the next interrupt is always safe.
        unsafe { asm!("hlt", options(nomem, nostack)) };
    }
}
