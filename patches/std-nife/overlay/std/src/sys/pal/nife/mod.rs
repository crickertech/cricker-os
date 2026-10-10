//! Platform Abstraction Layer for nife (milestone 27): std over the native capability ABI.
//!
//! This is the Hermit shape (a `sys` backend implemented directly on a non-POSIX ABI), not the
//! Redox shape (a libc first): there is no errno, no fd table, no `open`, no `fork` under here,
//! because the OS does not have them and std does not actually need them. What a nife
//! process *does* have is a capability table populated by its parent, and this PAL binds std to the two
//! slots the std runtime contract names (see `rt`): an untyped budget that pays for the heap,
//! and an endpoint that stdout/stderr SEND to.
//!
//! `net` and `fs` are bound to their capability-granted servers (phase two: net_stack's socket contract
//! and the FS service), and each answers `Unsupported` when the program was not granted the
//! capability that reaches its server. Everything the OS cannot honestly do yet (`process`,
//! `thread::spawn`, ...) keeps the shared dispatch fallback rather than pretending; those PALs bind
//! here when the servers behind them exist.
//!
//! This file lives in the nife repo (patches/std-nife) and is materialized into a
//! patched rust-src by `cargo xtask std-src`; see notes/std.md for how the sysroot is built.

#![allow(unsafe_op_in_unsafe_fn)]

pub(crate) mod abi;
// The net_stack socket-contract wire format (opcodes, frame offsets), generated verbatim from
// `user/src/netproto.rs` by `cargo xtask std-src` so the numbers cannot drift from the server.
// The net PAL (`sys/net`) is a client of it.
pub(crate) mod netproto;
// The FS-service wire format (opcodes, request packing, the errno convention), generated verbatim
// from `crates/filesystem_protocol/src/lib.rs` by the same xtask step. The fs PAL (`sys/fs`) is a client of
// it. `blk`/`fixture` in there belong to the other two parties, hence the allow.
#[allow(dead_code)]
pub(crate) mod fsproto;
// The wall-clock contract (DECISIONS §43): the clock page's layout and its seqlock, the propose
// protocol, and the policy, generated verbatim from `crates/clock_protocol/src/lib.rs` by the same
// xtask step. The time PAL (`sys/time`) is a *reader* of the page and never a writer, so the
// publish half and the propose half belong to the other parties; hence the allow.
#[allow(dead_code)]
pub(crate) mod clockproto;
// The entropy contract (DECISIONS §44): the request packing and the reply's byte count, generated
// verbatim from `crates/entropy_protocol/src/lib.rs` by the same xtask step. The random PAL
// (`sys/random`) is a client of it; `READY` belongs to the service and its spawner, hence the allow.
#[allow(dead_code)]
pub(crate) mod entropyproto;
// The byte-sink contract (milestone 50, notes/sink-protocol.md): the one framing every "write
// these bytes there" destination speaks, generated verbatim from `crates/byte_sink_protocol/src/lib.rs` by
// the same xtask step. `sys/stdio` is a *writer* of it, so the receiving half (`unpack`, `Msg`)
// belongs to the sinks; hence the allow.
#[allow(dead_code)]
pub(crate) mod sinkproto;
// The inert-configuration contract (milestone 47's environment-variable fork, DECISIONS §111):
// the config page's layout and the closed, validated domains TZ/LANG/TERM are checked against,
// generated verbatim from `crates/environment_protocol/src/lib.rs` by the same xtask step. `sys/env` is a
// *reader* of the page; `PageBuilder` and the domain tables belong to whoever assembles one
// (init, or today's kernel test harness standing in for it), hence the allow.
#[allow(dead_code)]
pub(crate) mod envproto;
// The argument page (milestone 205 (how a foreign program is told what to do), DECISIONS §170):
// the byte argv's layout, generated verbatim from `crates/argument_protocol/src/lib.rs` by the same
// xtask step. `sys/args` is a *reader*; `PageBuilder` belongs to whoever assembles a page (the
// shell), hence the allow.
#[allow(dead_code)]
pub(crate) mod argproto;
// The name resolver's client wire (milestone 384 (in a capability system the resolver is a grant)),
// generated verbatim from `crates/name_resolution_protocol/src/wire.rs` by the same xtask step.
// `sys/net`'s `lookup_host` is a client of it; the grant words belong to the spawner, hence the allow.
#[allow(dead_code)]
pub(crate) mod resolveproto;
// The std runtime contract itself (milestone 595 (provisional)): the fixed slots and the shared pages'
// addresses, generated verbatim from `crates/std_runtime_protocol/src/lib.rs` by the
// same xtask step, so the progenitor that builds a std child and this PAL that reads one cannot
// drift. `rt` re-exports what it reads; `SLOTS` and `STACK_PAGES` are the loader's, hence the allow.
#[allow(dead_code)]
pub(crate) mod runtimeproto;
// The timebase page (milestone 184; riscv64 joined 2026-09-21): where `rt::cntfrq` reads the rate
// on the two architectures that have no register stating it. Generated verbatim from
// `crates/counter_frequency_protocol/src/lib.rs` by the same xtask step, so the page's address and
// layout cannot drift from the kernel that writes it. Compiled only where it is read; aarch64
// states its rate in `CNTFRQ_EL0`. `build_page` is the kernel's half.
#[cfg(any(target_arch = "x86_64", target_arch = "riscv64"))]
#[allow(dead_code)]
pub(crate) mod counterfreqproto;
pub(crate) mod rt;
// The user address-space map and the current-CPU page (milestone 812 (`std::thread::spawn` runs real
// threads in one address space)): a thread's allowance is read from its page, whose layout is
// `current_cpu_protocol`'s, generated here like every contract above and pointed at `addressmap`
// for the one constant it takes from the map.
#[allow(dead_code)]
pub(crate) mod addressmap;
#[allow(dead_code)]
pub(crate) mod currentcpuproto;

use crate::io;

/// The process entry point. The kernel (or a userspace loader) enters the ELF here per the
/// native ABI (notes/abi.md §3): three free argument registers, a mapped stack, a populated
/// capability table, no libc and no argv. rustc's generated C `main` wraps the user's `fn main` in
/// `lang_start`, which runs std's rt init and catches the exit code; all this shim does is call
/// it and turn the return into `SYS_EXIT`.
///
/// The linker finds this symbol in std's rlib because the link script's `ENTRY(_start)` makes it
/// an undefined reference, the same way a libc's crt0.o gets pulled in.
#[cfg(all(not(test), not(target_arch = "x86_64")))]
#[unsafe(no_mangle)]
pub extern "C" fn _start(_a0: u64, _a1: u64, _a2: u64) -> ! {
    run_main()
}

/// **`x86_64` enters through one `call`, because the SysV stack contract is about the instruction
/// before the function, not the function** (milestone 184).
///
/// The kernel drops a new thread to ring 3 with `rsp` at the top of its stack page, which is a page
/// boundary and so `rsp % 16 == 0`. The x86-64 psABI promises a function that `rsp % 16 == 8` on
/// entry, because it assumes a `call` just pushed an 8-byte return address. aarch64 and RISC-V have
/// no such offset (neither pushes on call), which is why only this architecture needs a shim. The
/// cheapest correct fix is to make the assumption true: `call` the Rust entry, which pushes exactly
/// the eight bytes the ABI expects. The argument registers pass through untouched.
///
/// Without SSE nothing in this target *faults* on a misaligned stack (there is no `movaps` to trip),
/// so getting this wrong would pass every test and stay latent until someone compiled with vector
/// features. That is the reason to do it properly rather than to rely on the soft-float target
/// being forgiving. The `ud2` is unreachable: the Rust entry diverges.
#[cfg(all(not(test), target_arch = "x86_64"))]
core::arch::global_asm!(
    ".pushsection .text._start, \"ax\", @progbits",
    ".globl _start",
    "_start:",
    "call {entry}",
    "ud2",
    ".popsection",
    entry = sym x86_entry,
);

#[cfg(all(not(test), target_arch = "x86_64"))]
extern "C" fn x86_entry(_a0: u64, _a1: u64, _a2: u64) -> ! {
    run_main()
}

#[cfg(not(test))]
fn run_main() -> ! {
    unsafe extern "C" {
        fn main(argc: isize, argv: *const *const u8, sigpipe: u8) -> i32;
    }
    // No argv on this ABI (arguments are three registers, unused by std programs for now), and
    // sigpipe is a Unix concept passed as 0, the same as every non-Unix port.
    let code = unsafe { main(0, core::ptr::null(), 0) };
    rt::exit(code as i64)
}

// SAFETY: must be called only once during runtime initialization.
pub unsafe fn init(_argc: isize, _argv: *const *const u8, _sigpipe: u8) {
    // The heap wires itself lazily to the untyped in slot 0 on first allocation, and there is no
    // signal machinery and no TLS to set up. **The environment is seeded here, once, before
    // `main` runs** (milestone 47's environment-variable fork, DECISIONS §111): a program
    // granted an inert-configuration page gets `TZ`/`LANG`/`TERM` in its `std::env` table from
    // the first line of `main` onward, and a program granted none is seeded with nothing, which
    // is the honest-absence answer this platform gives everywhere else.
    crate::sys::env::seed();
}

// SAFETY: must be called only once during runtime cleanup.
// NOTE: this is not guaranteed to run, for example when the program aborts.
//
// **Close the output stream** (milestone 50). std's `rt::cleanup` flushes stdout and then calls
// this, so an end-of-stream message here is the last thing on the wire, after every byte the
// program printed. A sink acts on it: a file sink closes its handle, and the reader of a pipe stops
// reading instead of blocking forever on a writer that has exited. See notes/sink-protocol.md.
//
// It does not run when the program aborts, which is correct rather than a gap: a process the kernel
// killed did not finish its output, and saying so would be a lie. The reader learns that death the
// other way, from the supervisor that holds its fault endpoint (DECISIONS §26).
pub unsafe fn cleanup() {
    crate::sys::stdio::end_of_stream();
}

pub fn unsupported<T>() -> io::Result<T> {
    Err(unsupported_err())
}

pub fn unsupported_err() -> io::Error {
    io::Error::UNSUPPORTED_PLATFORM
}

/// Abort: take a breakpoint fault. The kernel kills the process and reports the fault on the
/// console, which is this ABI's honest version of `abort()`: loud, attributable, and no
/// unwinding (the target is panic=abort; unwinding machinery is never even linked).
pub fn abort_internal() -> ! {
    rt::abort()
}
