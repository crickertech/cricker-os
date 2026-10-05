//! The std runtime contract, and the syscall glue that meets it.
//!
//! This is the PAL's twin of `crates/user_mode_runtime`: the same `svc #0` / `ecall` / `syscall` instructions, the
//! same register convention, deliberately re-stated here because std cannot depend on an
//! out-of-tree crate. The ABI *constants* are not re-stated: `abi.rs` next door is generated
//! verbatim from `crates/abi/src/lib.rs` by `cargo xtask std-src`, so the numbers cannot drift.
//! Only the one trap, `trap5`, is hand-copied; if `user_mode_runtime`'s changes, change this.
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
    ARGS_PAGE, ARGS_SLOT, CLOCK_PAGE, CLOCK_SLOT, CONFIG_PAGE, CONFIG_SLOT, ENTROPY_SLOT, FS_DIR_SLOT, FS_PAGE, HEAP_BASE,
    HEAP_MAX, MEMORY_REGION_SLOT, NET_MEMORY_REGION_SLOT, STACK_SLOT, STDOUT_SLOT,
};

use super::abi;

/// **The one trap** in this PAL, a twin of `crates/user_mode_runtime`'s `trap5`, whose doc has
/// the contract: the number in `x8`/`a7`/`rax`, five words in and five back in `x0..x4`,
/// `a0..a4`, or `rdi`, `rsi`, `rdx`, `r10`, `r8`.
///
/// **Every syscall comes through here**, and the reason is a defect found on 2026-10-05
/// (notes/job-mix/spawn-destroy-gone.md). The kernel writes its result into the first register on
/// the way out of every syscall, and some methods write the next four. A wrapper that declares
/// fewer outputs than that tells the compiler a register survives the trap when it does not, and
/// the compiler is entitled to keep a live value there. `user_mode_runtime::yield_now` did, and an
/// optimised loop sent `DESTROY`'s arguments to the wrong capability after a yield. This file had
/// the same declarations (`yield_now` with no output, `invoke` with `x1..x4` input-only); no std
/// program is known to have been miscompiled by them, which is luck rather than a property.
///
/// No `nomem`: a `CALL` is how a shared page changes hands (`sys/fs`'s `Page` relies on that),
/// and a yield is when other threads write memory this one shares with them.
#[inline(always)]
unsafe fn trap5(nr: u64, a: [u64; 5]) -> [u64; 5] {
    let mut w = a;
    #[cfg(target_arch = "aarch64")]
    unsafe {
        core::arch::asm!(
            "svc #0",
            in("x8") nr,
            inlateout("x0") w[0],
            inlateout("x1") w[1],
            inlateout("x2") w[2],
            inlateout("x3") w[3],
            inlateout("x4") w[4],
            options(nostack),
        );
    }
    #[cfg(target_arch = "riscv64")]
    unsafe {
        core::arch::asm!(
            "ecall",
            in("a7") nr,
            inlateout("a0") w[0],
            inlateout("a1") w[1],
            inlateout("a2") w[2],
            inlateout("a3") w[3],
            inlateout("a4") w[4],
            options(nostack),
        );
    }
    // `rcx` and `r11` are the `syscall` instruction's own: it writes the return address and
    // `RFLAGS` there unconditionally.
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") nr,
            inlateout("rdi") w[0],
            inlateout("rsi") w[1],
            inlateout("rdx") w[2],
            inlateout("r10") w[3],
            inlateout("r8") w[4],
            lateout("rcx") _,
            lateout("r11") _,
            options(nostack),
        );
    }
    w
}

/// Invoke a capability. See `crates/user_mode_runtime::invoke`, of which this is a twin.
pub unsafe fn invoke(cap: u64, method: u64, a0: u64, a1: u64, a2: u64) -> i64 {
    unsafe { trap5(abi::SYS_INVOKE, [cap, method, a0, a1, a2])[0] as i64 }
}

/// SEND three words on the endpoint in `slot`. Blocks until a receiver takes them.
pub fn send(slot: u64, w0: u64, w1: u64, w2: u64) -> i64 {
    unsafe { invoke(slot, abi::rendezvous::SEND, w0, w1, w2) }
}

/// `CALL` the endpoint in `slot`: send two words and block until the server replies through the
/// one-shot Reply capability the kernel mints. Returns the two reply words. A twin of
/// `user_mode_runtime::call`; the net PAL (`sys/net`) drives the socket contract with it.
///
/// On a syscall-level failure (an empty slot, wrong rights) the kernel returns a negative value
/// in the first result register, which a caller distinguishes from a server reply by reading it
/// as `i64` (the net server never replies a negative word).
pub fn call(slot: u64, w0: u64, w1: u64) -> (u64, u64) {
    // SAFETY: CALL returns the two reply words in the first two registers.
    let w = unsafe { trap5(abi::SYS_INVOKE, [slot, abi::rendezvous::CALL, w0, w1, 0]) };
    (w[0], w[1])
}

/// Give up the CPU (`SYS_YIELD`); the timed sleep loop is built on this.
pub fn yield_now() {
    let _ = unsafe { trap5(abi::SYS_YIELD, [0; 5]) };
}

/// Terminate this process (`SYS_EXIT`). The kernel reaps the thread and frees the address space.
pub fn exit(code: i64) -> ! {
    let _ = unsafe { trap5(abi::SYS_EXIT, [code as u64, 0, 0, 0, 0]) };
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
