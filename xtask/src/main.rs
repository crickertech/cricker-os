//! Build orchestration for nife.
//!
//! A normal Rust binary that runs on the *host*. Building a kernel means a custom
//! target, a linker script, and driving QEMU with the right flags, none of which fits
//! neatly into `cargo build`. This beats a Makefile because it's Rust and it composes.
//! See DECISIONS §7.
//!
//!     cargo xtask run      boot the kernel (the milestone tour), print to this terminal
//!     cargo xtask shell    boot straight to the interactive shell (add --hvf for the real core)
//!     cargo xtask swish-check  boot that same shell, type at it, and check what it answered
//!     cargo xtask test     host tests (milliseconds), then the kernel under QEMU
//!                          (--hvf runs the aarch64 kernel leg on the physical core)
//!     cargo xtask gdb      boot paused, waiting for a debugger on :1234
//!     cargo xtask objdump  disassemble the kernel
//!     cargo xtask image    build the flat arm64 Image and dump its header
//!     cargo xtask board-console  read the serial console of a real board, log it, stop on a deadline
//!                                (and, under --stop, send the one byte that ends a rebooting soak)
//!     cargo xtask board-script   write the U-Boot script that boots the board without a person at its prompt
//!
//! Note that `run` and `test` do NOT invoke QEMU themselves. They just call cargo,
//! which invokes `helpers/qemu-runner-aarch64.sh` via the runner setting in
//! `.cargo/config.toml`. That script is the single source of truth for how the kernel
//! gets booted, so there is exactly one place to get the QEMU flags wrong.

use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};

mod archive;
mod bench;
mod board;
mod boot_check;
mod card_check;
/// **The rollback gate** (rung 2b of milestone 198 (a package manager, and the trivial install that makes a second customer possible)): a doomed upgrade is tried and the machine comes back on its own.
mod confirm;
mod disk;
mod disk_check;
mod disk_throughput;
mod farm;
mod host;
mod icount;
mod inbound;
mod inspect;
mod install;
mod manual;
mod measure;
mod package;
mod restamp;
mod rollback;
mod scanout;
mod screen;
mod soak;
mod stick;
mod suite;
mod swish_check;
mod uefi;

use crate::archive::{initrd_aarch64, initrd_riscv, initrd_x86};
use crate::bench::bench;
use crate::board::{board_console, board_script};
use crate::boot_check::boot_check;
use crate::disk::{mkdisk, mkredoxfs, redoxfs_server_build};
use crate::farm::{std_aborts, std_exerciser, std_inputs_stamp, std_src};
use crate::host::cargo;
use crate::icount::icount;
use crate::inspect::{gdb, image, objdump};
use crate::manual::{manual_store, tree_apropos};
use crate::soak::{job_mix_sweep, soak_test};
use crate::suite::{test, undefined_behavior_check};
use crate::swish_check::swish_check;
use crate::uefi::{uefi_boot, uefi_image_command, uefi_test};

const TARGET: &str = "aarch64-unknown-none-softfloat";
const RUNNER: &str = "helpers/qemu-runner-aarch64.sh";

/// The RISC-V target, for the second-architecture initrd (milestone 20). The kernel itself is built
/// and run through cargo + `helpers/qemu-runner-riscv64.sh` directly, not this xtask; this const exists
/// only so `initrd-riscv` builds the userspace archive for the matching target.
const RISCV_TARGET: &str = "riscv64imac-unknown-none-elf";

/// The `x86_64` target (milestone 161). The kernel is built and run through cargo +
/// `helpers/qemu-runner-x86_64.sh`, exactly as the RISC-V one is, and since item 4's hand-off this
/// const also builds the third userspace archive: `initrd-x86` compiles `user` for it and
/// [`initrd_x86`] packs the same programs RISC-V's archive carries. See notes/x86-port/userspace.md.
const X86_TARGET: &str = "x86_64-unknown-none";

/// Whether this run builds optimized binaries. Only `bench --release` sets it (a fair cross-OS
/// comparison wants an optimized kernel and userspace, not the debug default). Everything else stays
/// debug: faster builds, and the tests and the tour want debuginfo and cheap rebuilds.
static RELEASE: AtomicBool = AtomicBool::new(false);

/// `"release"` or `"debug"`: the cargo profile directory the built artifacts land in.
fn profile_dir() -> &'static str {
    if RELEASE.load(Ordering::Relaxed) {
        "release"
    } else {
        "debug"
    }
}

/// Run `cargo <args>`, adding `--release` when this is a release run. For the build commands whose
/// output profile must match `profile_dir()` (the kernel and user builds behind `bench --release`).
fn cargo_profiled(args: &[&str]) -> bool {
    let mut v = args.to_vec();
    if RELEASE.load(Ordering::Relaxed) {
        v.push("--release");
    }
    cargo(&v)
}

fn main() -> ExitCode {
    let cmd = std::env::args().nth(1).unwrap_or_default();

    let ok = match cmd.as_str() {
        "build" => build(),
        "run" => {
            maybe_hvf();
            // Build the disk and the initrd first: the kernel boots with them, and `cargo run`
            // would not rebuild them on its own (the kernel does not depend on them in cargo).
            mkdisk() && user() && cargo(&["run", "-p", "kernel", "--target", TARGET])
        }
        "shell" => {
            // Boot straight to the interactive shell (the milestone tour compiled out).
            maybe_hvf();
            eprintln!("--- booting nife to an interactive shell (type `help`, Ctrl-C to quit) ---");
            // A virtio-rng device (DECISIONS §120's 2026-08-26 amendment: "grant the QEMU-only
            // virtio-rng stopgap"), the same terms `swish_check_leg` already attaches one on: this
            // is the interactive boot itself, not the bench boot sharing its runner, so there is
            // no icount-drift reason to keep it test-leg only, and the whole point of the
            // amendment is that a person booting this way should have one.
            // SAFETY: `set_var` became unsafe in edition 2024 because it races other threads.
            // xtask is single-threaded up to this point in `main`, before `cargo(&[..])` spawns
            // its child, so there is no concurrent reader to race.
            unsafe { std::env::set_var("NIFE_RNG", "1") };
            // **The filesystem the prompt's `>` and `<` need** (milestone 50). The FS server first,
            // because `user()` packs the initrd and the boot loads it out of there by name, and the
            // RedoxFS image because the runner attaches it only when the file exists. Both are
            // rebuilt per boot, so the prompt always meets a fresh fixture rather than whatever the
            // last session wrote.
            redoxfs_server_build(TARGET)
                && mkredoxfs()
                && mkdisk()
                && user()
                && cargo(&[
                    "run",
                    "-p",
                    "kernel",
                    "--features",
                    "shell",
                    "--target",
                    TARGET,
                ])
        }
        // The aarch64 archive, standalone (2026-08-27): every other caller reaches
        // `initrd_aarch64` through `user()` as part of a boot (`build`, `run`, `shell`, ...), and
        // `initrd_aarch64` itself only packs, it does not build. `initrd_riscv` and `initrd_x86`
        // both build-then-pack in one call, so this subcommand calls `user()` (build, then pack)
        // rather than `initrd_aarch64()` alone, to give aarch64 the same self-contained entry
        // point its two siblings already have.
        "initrd-aarch64" => user(),
        "initrd-riscv" => initrd_riscv(),
        // The third archive (milestone 161). Same programs, built for x86_64.
        "initrd-x86" => initrd_x86(),
        // The bootable UEFI image (milestone 87): the entry real firmware can start, staged at
        // target/esp for a QEMU/OVMF boot or for a FAT32 stick. See notes/x86-uefi-boot.md.
        // Name: `uefi-image` and `uefi-boot` ratified 2026-08-30 (calef, in session, on milestone
        // 87's lane report). Hyphenated like every other subcommand, and each names what it
        // produces rather than the tool that produces it.
        "uefi-image" => uefi_image_command(),
        // The same image, booted under OVMF and checked. Runs inside `script/test --arch x86_64`;
        // exposed on its own because the bench procedure starts by watching this pass locally.
        "uefi-boot" => uefi_boot(),
        // Milestone 198's rung 3a, the producer half: a reviewed recipe in, one package file and
        // its digest out. Name provisional (2026-09-23).
        "package" => package::package(std::env::args().nth(2)),
        "install-boot" => install::install_boot(),
        "rollback-boot" => rollback::rollback_boot(),
        // Rung 2b's other half: a GOOD upgrade is tried, confirmed by the running system, and
        // still chosen on the next boot with no tries left. The exact negative of the line above,
        // and the one that proves the rollback is a policy rather than an accident. Name
        // provisional (2026-09-21): `stick-boot` was already the boot stick's gate.
        "confirm-boot" => confirm::confirm_boot(),
        // Milestone 195: the same firmware, the kernel's test binary instead of its tour.
        "uefi-test" => uefi_test(),
        // The stick (DECISIONS §157): every architecture's boot file, sealed, and `stick_maker`
        // built around them; then the same directory booted under all three firmwares. See
        // xtask/src/stick.rs and notes/boot-stick.md. Names provisional (2026-09-19).
        "stick" => stick::stick(),
        "stick-boot" => stick::stick_boot(),
        // **The boot tour read back off a `ramfb`**, milestone 243 (a machine with no serial port),
        // on the two architectures whose firmware never lights a screen. `uefi-boot`'s twin: same
        // decoder, same claim, a different way of getting a framebuffer. Name provisional.
        "screen-boot" => {
            let arch = std::env::args().nth(2).unwrap_or_else(|| "aarch64".into());
            screen::screen_boot(&arch)
        }
        // The documentation store (milestone 40): build it, print what it costs, and optionally
        // answer a query against it with the same reader the guest uses.
        "manual" => manual_store(std::env::args().nth(2)),
        // The tree-wide search (milestone 40, script/apropos): the same index and the same reader,
        // pointed at this repository instead of at what the image installs. See `tree_apropos`.
        "apropos" => tree_apropos(std::env::args().nth(2)),
        "std-src" => std_src(),
        // Print the farm's input stamp and exit. Exists so that "the stamp does not depend on where
        // the checkout lives" is a claim anyone can CHECK rather than one they have to believe:
        //   cargo xtask std-stamp                        # in the main checkout
        //   git worktree add /tmp/w HEAD && (cd /tmp/w && cargo xtask std-stamp)
        // The two must print the same value. If they ever diverge, something location-dependent has
        // crept back into `std_inputs_stamp`, and `nife-dev` will start being stolen again.
        "std-stamp" => {
            println!("{:016x}", std_inputs_stamp());
            true
        }
        "std-exerciser" => std_exerciser(),
        // The abort sweep (milestone 64): which std calls kill a nife process instead of refusing
        // it. Runs at the end of `std-exerciser` (and so inside `script/test`); exposed on its own
        // because re-reading the list after a nightly bump should not need a rebuild.
        "std-aborts" => std_aborts(),
        "swish-check" => swish_check(),
        // The boot ladder's gate (milestone 268, item 6): boot every architecture's default kernel
        // and fail if its self-test verdict is not green. See script/boot-check.
        "boot-check" => boot_check(),
        "test" => test(),
        "undefined-behavior-check" => undefined_behavior_check(),
        "bench" => bench(),
        // The instruction-count instrument (milestone 78): the two timing claims a wall clock
        // cannot make, on both ISAs. See script/icount.
        "icount" => icount(),
        "gdb" => gdb(),
        "objdump" => objdump(),
        "image" => image(),
        // The real board's serial console (milestone 216). Returns its own exit code rather than a
        // bool, so a bench script can tell "reached the banner" from "went quiet" from "ran out".
        "board-console" => return board_console(),
        // The sustained multicore run under QEMU (milestone 219), judged by the same recogniser
        // `board-console` points at a board. Returns its own exit code for the same reason.
        "soak-test" => return soak_test(),
        // The multi-tasking workload sweep under QEMU (milestone 168). Returns its own exit code
        // for `soak-test`'s reason: a rehearsal that cannot say *how* it failed is not a rehearsal.
        "job-mix" => return job_mix_sweep(),
        // Fatal risk 6's bench boot (milestone 261 (the NVMe driver leaves the kernel)): stage the stick image and rehearse its four
        // verdicts under OVMF. Name provisional. See xtask/src/disk_throughput.rs.
        "disk-throughput" => return disk_throughput::disk_throughput(),
        // The card's U-Boot script (milestone 218): what makes the board boot without a person at
        // its prompt. `script/board-image` calls this; it is a separate verb so the script it
        // produces can be rebuilt and read on its own.
        "board-script" => board_script(),
        // **milestone 223 (read a card and say whether its kernel and archive match).**
        // Returns its own exit code rather than a bool, because "this pair
        // would be refused" and "there is nothing here to check" want different reactions from
        // whoever ran it, and a bench script wants to tell them apart.
        "card-check" => return card_check::card_check(std::env::args().nth(2)),
        other => {
            if !other.is_empty() {
                eprintln!("unknown command: {other}\n");
            }
            eprintln!(
                "usage: cargo xtask <build|run|shell|swish-check|boot-check|initrd-aarch64|initrd-riscv|initrd-x86|uefi-image|uefi-boot|uefi-test|package|install-boot|rollback-boot|confirm-boot|stick|stick-boot|screen-boot|manual|apropos|std-src|std-stamp|std-exerciser|std-aborts|test|undefined-behavior-check|bench|icount|gdb|objdump|image|board-console|soak-test|board-script|card-check> [--hvf]"
            );
            eprintln!("       cargo xtask swish-check [--arch aarch64|riscv64]");
            eprintln!(
                "       cargo xtask install-boot   (x86_64/OVMF: a stick installs itself onto an NVMe disk, then that disk boots with the stick detached)"
            );
            eprintln!(
                "       cargo xtask rollback-boot  (x86_64/OVMF: a doomed upgrade is tried, and the machine comes back on the previous image by itself)"
            );
            eprintln!(
                "       cargo xtask confirm-boot   (x86_64/OVMF: a good upgrade is tried, confirms itself, and is still chosen with no tries left)"
            );
            eprintln!(
                "       cargo xtask card-check [<mounted card>|<stick dir>|<boot file>]   (default: target/board)"
            );
            eprintln!("       cargo xtask boot-check [--arch aarch64|riscv64|x86_64] [--inject]");
            eprintln!(
                "       cargo xtask undefined-behavior-check [extra cargo-miri-test args, e.g. -p <crate>]"
            );
            eprintln!(
                "       cargo xtask bench [--riscv | --x86] [--real] [--release] [--smp] [--check] [--save --why <reason>]"
            );
            eprintln!(
                "       cargo xtask test [--arch aarch64|riscv64|x86_64] [--cpu <qemu-cpu-model>] [--hvf] [--test <substring>]"
            );
            eprintln!("       cargo xtask icount [--arch aarch64|riscv64]");
            eprintln!(
                "       cargo xtask board-console [--port <dev>] [--replay <log>] [--log <file>] [--for <duration>] [--until spl|opensbi|uboot|handoff|banner|machine|selftest|tour|prompt|none] [--quiet-after <duration>] [--stop | --stop-after <n>]"
            );
            return ExitCode::FAILURE;
        }
    };

    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn build() -> bool {
    // The user program and the disk image first: the kernel boots with the program as an initrd
    // and reads the disk over virtio, so both have to exist before it runs.
    mkdisk() && user() && cargo(&["build", "-p", "kernel", "--target", TARGET])
}

/// Build the userspace ELF that the kernel will load at milestone 7.
///
/// It is a **separate crate with its own linker script** (linked at `address_space_map::IMAGE_BASE`, in the low half,
/// where `TTBR0` lives), so it cannot accidentally share anything with the kernel. And it stays
/// an **ELF**: the kernel's loader wants program headers, unlike the kernel itself, which QEMU
/// wants as a flat image. See notes/elf.md.
pub(crate) fn user() -> bool {
    cargo_profiled(&[
        "build",
        "-p",
        "components",
        "-p",
        "fixtures",
        "--target",
        TARGET,
    ]) && initrd_aarch64()
}

/// The packed initrd archive ([`archive::initrd_path`]) is what `helpers/qemu-runner-aarch64.sh` passes to QEMU as
/// `-initrd` (milestone 19f); the raw user ELFs ([`host::bin_elf`]) are only the input `initrd_aarch64` packs.
///
/// **Deliberately the same road Linux's initramfs travels**, now literally an archive like theirs.
/// QEMU loads the file into RAM and writes its address into `/chosen/linux,initrd-start` in the
/// device tree; the kernel finds it there (`memory::initrd_region`, built at milestone 3 for
/// exactly this). Nothing about the contents is known to the kernel at build time, which is the
/// entire point of milestone 7c.
///
/// If `--hvf` was passed, boot under Apple's Hypervisor.framework instead of TCG.
fn maybe_hvf() {
    if std::env::args().any(|a| a == "--hvf") {
        // SAFETY: `set_var`/`remove_var` became unsafe in edition 2024 because they race other
        // threads. xtask is single-threaded here: this runs on the main thread before the child
        // that reads it is spawned, and the only thread xtask ever starts (the transcript reader
        // in swish_check_leg) copies pipe bytes into a String and never touches the environment.
        unsafe { std::env::set_var("NIFE_ACCEL", "hvf") };
        eprintln!("--- on the real Apple Silicon core via Hypervisor.framework ---");
    }
}
