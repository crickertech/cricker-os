//! The std runtime contract, and the syscall glue that meets it.
//!
//! This is the PAL's twin of `crates/user_mode_runtime`: the same `svc #0` / `ecall` / `syscall` instructions, the
//! same register convention, deliberately re-stated here because std cannot depend on an
//! out-of-tree crate. The ABI *constants* are not re-stated: `abi.rs` next door is generated
//! verbatim from `crates/abi/src/lib.rs` by `cargo xtask std-src`, so the numbers cannot drift.
//! Only these few asm wrappers are hand-copied; if `user_mode_runtime`'s change, change these.
//!
//! # The std slot convention
//!
//! A std program's loader owes it, per the out-of-band-contract rule of notes/abi.md §4:
//!
//! - **slot 0**: an untyped budget. The global allocator draws heap pages from it lazily via
//!   `memory_region::MAP`, at [`HEAP_BASE`], capped at [`HEAP_MAX`].
//! - **slot 1**: an endpoint with WRITE. `stdout` and `stderr` SEND here, 16 bytes per message
//!   (w0 = byte count, w1|w2 = the bytes, little-endian). Interleaving of out and err is the
//!   phase-one price of one endpoint; milestone 28's terminal contract owns fixing it.
//!
//! And two more, granted only to a std program that is given the network (milestone 27 phase two;
//! the net PAL in `sys/net` binds them, DECISIONS §25):
//!
//! - **slot 2**: the `Stack` endpoint with WRITE. `std::net` speaks the net_stack socket contract
//!   (`netproto`) over it: `CALL`s carry a socket id and control words, `SEND_CAP` delegates a
//!   per-socket shared frame. A program not given the network leaves this slot empty, and every
//!   `TcpStream`/`UdpSocket` operation returns `Unsupported` rather than blocking.
//! - **slot 3**: an untyped budget the net PAL mints and maps each socket's shared frame from.
//!
//! And one more, granted only to a std program that is given a **directory** (milestone 27 phase
//! two, the FS-service half; the fs PAL in `sys/fs` binds it, DECISIONS §27):
//!
//! - **slot 4**: the FS-service endpoint with WRITE. That endpoint **is** the directory
//!   capability: the server it reaches is bound to one directory node, and every name `std::fs`
//!   sends is resolved under that directory. There is no global namespace to reach past it, so a
//!   path that tries to leave the directory is refused before it ever reaches the wire. The grant
//!   comes with one page the loader maps at [`FS_PAGE`], the contract's unit of transfer. A
//!   program left this slot empty gets `Unsupported` from every `std::fs` call, which is what "no
//!   ambient filesystem" feels like from inside a process.
//!
//! And one more, granted only to a std program that is given a **wall clock** (milestone 51; the
//! time PAL in `sys/time` binds it, DECISIONS §43):
//!
//! - **slot 5**: a `PageFrame` capability naming the clock page, with `READ`. The loader maps that
//!   same page **read-only** at [`CLOCK_PAGE`], and `SystemTime::now()` is then the ambient
//!   monotonic counter plus the offset the clock service published there: two loads and an add,
//!   no server round trip, and nothing this program can write. A program left this slot empty
//!   does not know what time it is and `SystemTime::now()` says so loudly rather than reporting
//!   1970 plus uptime, which is what it used to do.
//!
//! And one more, granted only to a std program that is given **entropy** (milestone 56; the random
//! PAL in `sys/random` binds it, DECISIONS §44):
//!
//! - **slot 6**: the entropy service's request endpoint with WRITE. It means *"you may obtain
//!   randomness"*, and it names no device: the service holds the virtio-rng transport and this
//!   program cannot reach it. `std::random::SystemRng` (which promises bytes "suitable for
//!   cryptographic purposes") is a `CALL` on this endpoint, and a program left this slot empty gets
//!   a **panic** rather than a predictable stand-in, for the same reason `SystemTime::now()` does:
//!   the function has no error channel and the alternative is a lie. `HashMap`'s seed is the one
//!   caller std itself treats as best-effort, and it keeps working either way; see `sys/random`.
//!
//! And one more, granted only to a std program that is given **inert configuration** (milestone
//! 47's environment-variable fork, DECISIONS §111; `sys/env` binds it):
//!
//! - **slot 7**: a `PageFrame` capability naming the inert-configuration page, with `READ`. The
//!   loader maps that same page **read-only** at [`CONFIG_PAGE`], and `sys/env`'s `seed`
//!   populates `std::env`'s `TZ`, `LANG` and `TERM` from it once, at process startup, before
//!   `main` runs (`pal::nife::init`). A program left this slot empty is seeded with nothing,
//!   which is the same honest-absence shape every other slot here uses: `env::var("TZ")`
//!   answers `Err` because nobody granted this program a timezone, not because the lookup
//!   failed. Unlike the clock page, this one has exactly one writer and it finishes before the
//!   page has a second reader, so there is no seqlock to it (see `environment_protocol`'s own docs).
//!
//! Programs that never allocate, print, open a socket, or open a file never touch the slots they
//! do not use.

// **The numbers themselves live in `crates/std_runtime_protocol`** (milestone 595 (provisional)),
// generated into this PAL as `runtimeproto` by `cargo xtask std-src`. They were written here, and
// again twice in the kernel test harness, until the progenitor became a fourth place that had to
// agree; now all of them read one file. Every address in it is a row of the user address-space map
// (`crates/address_space_map`, milestone 206): the three pages are runtime windows, above the net
// PAL's per-socket frames (0x1000_0000 upward, one page per socket id) and below the initrd window
// (0x2000_0000), and the heap is the map's heap band. That crate's tests pin each number to its band.
pub use super::runtimeproto::{
    CLOCK_PAGE, CLOCK_SLOT, CONFIG_PAGE, CONFIG_SLOT, ENTROPY_SLOT, FS_DIR_SLOT, FS_PAGE, HEAP_BASE,
    HEAP_MAX, MEMORY_REGION_SLOT, NET_MEMORY_REGION_SLOT, STACK_SLOT, STDOUT_SLOT,
};

use super::abi;

/// Invoke a capability. See `crates/user_mode_runtime::invoke`, of which this is a verbatim twin.
#[cfg(target_arch = "aarch64")]
pub unsafe fn invoke(cap: u64, method: u64, a0: u64, a1: u64, a2: u64) -> i64 {
    let ret: i64;
    unsafe {
        core::arch::asm!(
            "svc #0",
            in("x8") abi::SYS_INVOKE,
            inlateout("x0") cap => ret,
            in("x1") method,
            in("x2") a0,
            in("x3") a1,
            in("x4") a2,
            options(nostack),
        );
    }
    ret
}

/// Invoke a capability (RISC-V): `ecall`, number in `a7`, args in `a0..a4`, result in `a0`.
#[cfg(target_arch = "riscv64")]
pub unsafe fn invoke(cap: u64, method: u64, a0: u64, a1: u64, a2: u64) -> i64 {
    let ret: i64;
    unsafe {
        core::arch::asm!(
            "ecall",
            in("a7") abi::SYS_INVOKE,
            inlateout("a0") cap => ret,
            in("a1") method,
            in("a2") a0,
            in("a3") a1,
            in("a4") a2,
            options(nostack),
        );
    }
    ret
}

/// Invoke a capability (`x86_64`): `syscall`, number in `rax`, args in `rdi, rsi, rdx, r10, r8`,
/// result in `rdi` (DECISIONS §124). A twin of `user_mode_runtime::invoke5`'s register list, clobbers
/// included: `syscall` itself overwrites `rcx` (return address) and `r11` (RFLAGS), and the kernel
/// writes message words back into the argument registers, so every one of them is `inlateout`
/// rather than `in`. Declaring an argument register as `in` here would promise LLVM the kernel
/// preserves it, which a RECV-shaped reply does not.
#[cfg(target_arch = "x86_64")]
pub unsafe fn invoke(cap: u64, method: u64, a0: u64, a1: u64, a2: u64) -> i64 {
    let ret: u64;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") abi::SYS_INVOKE,
            inlateout("rdi") cap => ret,
            inlateout("rsi") method => _,
            inlateout("rdx") a0 => _,
            inlateout("r10") a1 => _,
            inlateout("r8") a2 => _,
            lateout("rcx") _,
            lateout("r11") _,
            options(nostack),
        );
    }
    ret as i64
}

/// SEND three words on the endpoint in `slot`. Blocks until a receiver takes them.
pub fn send(slot: u64, w0: u64, w1: u64, w2: u64) -> i64 {
    unsafe { invoke(slot, abi::rendezvous::SEND, w0, w1, w2) }
}

/// `CALL` the endpoint in `slot`: send two words and block until the server replies through the
/// one-shot Reply capability the kernel mints. Returns the two reply words. A verbatim twin of
/// `user_mode_runtime::call`; the net PAL (`sys/net`) drives the socket contract with it.
///
/// On a syscall-level failure (an empty slot, wrong rights) the kernel returns a negative value
/// in the first result register, which a caller distinguishes from a server reply by reading it
/// as `i64` (the net server never replies a negative word).
#[cfg(target_arch = "aarch64")]
pub fn call(slot: u64, w0: u64, w1: u64) -> (u64, u64) {
    let (mut r0, mut r1): (u64, u64);
    // SAFETY: `svc`. CALL returns the two reply words in x0/x1.
    unsafe {
        core::arch::asm!(
            "svc #0",
            in("x8") abi::SYS_INVOKE,
            inlateout("x0") slot => r0,
            in("x1") abi::rendezvous::CALL,
            lateout("x1") r1,
            in("x2") w0,
            in("x3") w1,
            in("x4") 0u64,
            options(nostack),
        );
    }
    (r0, r1)
}

/// `CALL` (RISC-V). See the aarch64 twin; `ecall`, the two reply words in `a0`/`a1`.
#[cfg(target_arch = "riscv64")]
pub fn call(slot: u64, w0: u64, w1: u64) -> (u64, u64) {
    let (mut r0, mut r1): (u64, u64);
    // SAFETY: `ecall`. CALL returns the two reply words in a0/a1.
    unsafe {
        core::arch::asm!(
            "ecall",
            in("a7") abi::SYS_INVOKE,
            inlateout("a0") slot => r0,
            inlateout("a1") abi::rendezvous::CALL => r1,
            in("a2") w0,
            in("a3") w1,
            in("a4") 0u64,
            options(nostack),
        );
    }
    (r0, r1)
}

/// `CALL` (`x86_64`). See the aarch64 twin; `syscall`, the two reply words in `rdi`/`rsi`, and the
/// same clobber list as [`invoke`] for the same reasons.
#[cfg(target_arch = "x86_64")]
pub fn call(slot: u64, w0: u64, w1: u64) -> (u64, u64) {
    let (r0, r1): (u64, u64);
    // SAFETY: `syscall`. CALL returns the two reply words in rdi/rsi.
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") abi::SYS_INVOKE,
            inlateout("rdi") slot => r0,
            inlateout("rsi") abi::rendezvous::CALL => r1,
            inlateout("rdx") w0 => _,
            inlateout("r10") w1 => _,
            inlateout("r8") 0u64 => _,
            lateout("rcx") _,
            lateout("r11") _,
            options(nostack),
        );
    }
    (r0, r1)
}

/// Give up the CPU (`SYS_YIELD`); the timed sleep loop is built on this.
pub fn yield_now() {
    #[cfg(target_arch = "aarch64")]
    unsafe {
        core::arch::asm!("svc #0", in("x8") abi::SYS_YIELD, options(nostack, nomem));
    }
    #[cfg(target_arch = "riscv64")]
    unsafe {
        core::arch::asm!("ecall", in("a7") abi::SYS_YIELD, options(nostack, nomem));
    }
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") abi::SYS_YIELD,
            lateout("rcx") _,
            lateout("r11") _,
            options(nostack, nomem),
        );
    }
}

/// Terminate this process (`SYS_EXIT`). The kernel reaps the thread and frees the address space.
pub fn exit(code: i64) -> ! {
    #[cfg(target_arch = "aarch64")]
    unsafe {
        core::arch::asm!(
            "svc #0",
            in("x8") abi::SYS_EXIT,
            in("x0") code as u64,
            options(nostack, nomem),
        );
    }
    #[cfg(target_arch = "riscv64")]
    unsafe {
        core::arch::asm!(
            "ecall",
            in("a7") abi::SYS_EXIT,
            in("a0") code as u64,
            options(nostack, nomem),
        );
    }
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") abi::SYS_EXIT,
            in("rdi") code as u64,
            lateout("rcx") _,
            lateout("r11") _,
            options(nostack, nomem),
        );
    }
    loop {
        core::hint::spin_loop();
    }
}

/// Fault on purpose (`brk` / `ebreak` / `ud2`): the kernel kills the process and reports where. This is
/// `abort()` on an OS whose failure story is "a fault the kernel attributes", and it is what
/// `panic!` reaches after printing, since the target is panic=abort.
pub fn abort() -> ! {
    #[cfg(target_arch = "aarch64")]
    unsafe {
        core::arch::asm!("brk #0", options(nostack, nomem));
    }
    #[cfg(target_arch = "riscv64")]
    unsafe {
        core::arch::asm!("ebreak", options(nostack, nomem));
    }
    // `ud2`, not `int3`, for the reason `user_mode_runtime::trap` measured: this kernel's IDT gates
    // are all DPL 0, so `int3` from ring 3 is refused as a #GP that names neither the instruction
    // nor the reason, where `ud2` is a fault the CPU raises with no gate involved.
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::asm!("ud2", options(nostack, nomem));
    }
    loop {
        core::hint::spin_loop();
    }
}

/// The monotonic tick count: the one ambient readable this ABI grants (notes/abi.md, "the one
/// ambient thing"). aarch64 `CNTVCT_EL0`; RISC-V `rdtime`; `x86_64` `rdtsc`, which ring 3 may execute
/// because the kernel leaves `CR4.TSD` clear (see `user_mode_runtime::now`'s `x86_64` arm for what that
/// costs).
pub fn now() -> u64 {
    #[cfg(target_arch = "aarch64")]
    {
        let t: u64;
        unsafe {
            core::arch::asm!("mrs {}, cntvct_el0", out(reg) t, options(nomem, nostack));
        }
        t
    }
    #[cfg(target_arch = "riscv64")]
    {
        let t: u64;
        unsafe {
            core::arch::asm!("rdtime {}", out(reg) t, options(nomem, nostack));
        }
        t
    }
    #[cfg(target_arch = "x86_64")]
    {
        let (lo, hi): (u32, u32);
        unsafe {
            core::arch::asm!("rdtsc", out("eax") lo, out("edx") hi, options(nomem, nostack));
        }
        ((hi as u64) << 32) | (lo as u64)
    }
}

/// Ticks per second. aarch64 reports it in `CNTFRQ_EL0`, the one architecture where the machine
/// states its own rate. Neither other one has such a register: RISC-V states the timebase in the
/// device tree and `x86_64`'s TSC rate is measured or read from `CPUID`, and in both cases only the
/// kernel can ever know it. So the kernel learns it once at boot and maps the answer read-only at
/// `counter_frequency_protocol::PAGE_VA` into every process it builds; this reads that page, the
/// same way `user_mode_runtime::cntfrq` does, which is what keeps a `std` program and a `no_std`
/// program on one machine agreeing about what a second is.
///
/// **RISC-V returned a hardcoded 10 MHz here until 2026-09-21**, QEMU `virt`'s rate, copied from the
/// same gap `user_mode_runtime::cntfrq` carried. radon (the VisionFive 2) runs at 4 MHz, so every
/// `Instant` duration and every `thread::sleep` in a std program on that board was out by 2.5x and
/// said nothing. calef's ruling: only accurate numbers, never hardcoded ones.
///
/// # Panics
///
/// If the rate is unknown, which is a zeroed or unrecognized page. **It used to fall back to 1 GHz**,
/// and that fallback is gone for the reason above: a std program whose `Instant` is silently scaled
/// wrong reports durations nobody can tell from real ones. Dying is the honest outcome, and it
/// matches `user_mode_runtime::cntfrq` exactly, which is the property this function exists to keep.
pub fn cntfrq() -> u64 {
    #[cfg(target_arch = "aarch64")]
    {
        let f: u64;
        unsafe {
            core::arch::asm!("mrs {}, cntfrq_el0", out(reg) f, options(nomem, nostack));
        }
        f
    }
    #[cfg(any(target_arch = "riscv64", target_arch = "x86_64"))]
    {
        // SAFETY: every path that builds a process on these architectures maps a page (real, or
        // zeroed when the rate was never learned) read-only at `PAGE_VA` before it runs.
        let page = unsafe {
            super::counterfreqproto::TimebasePage::new(super::counterfreqproto::PAGE_VA)
        };
        page.hz().expect(
            "the counter frequency is unknown: this process's timebase page is zeroed or \
             unrecognized",
        )
    }
}
