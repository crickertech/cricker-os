//! **The reboot gate**: boot to the prompt, write a file, type `reboot`, and see the machine come
//! back to a second prompt that reads the file (milestone 805 (`reboot` at the prompt), DECISIONS
//! §251 (restarting the machine is a kernel object the progenitor hands out)).
//!
//! # What it asserts, and what a failure of each would mean
//!
//! - **The flush answered before the reset.** `reboot` prints what the progenitor's
//!   `filesystem_protocol::fs::SYNC` said, which is the block server's count of completed
//!   `blk::FLUSH`es, and that line must come before the kernel's first `reboot:` line. Missing, and
//!   the reset ran ahead of the flush §251 puts first.
//! - **The kernel took the route this architecture has.** Its `reboot:` lines name the attempt
//!   (PSCI `SYSTEM_RESET`, SBI SRST, the FADT reset register) before making it. A refusal line
//!   fails the gate.
//! - **The machine came back.** Where there is firmware between the reset and the kernel (OpenSBI
//!   on riscv64, OVMF and `uefi_loader` on `x86_64`) its line must appear after the reset; aarch64
//!   `virt` boots `-kernel` with no firmware of its own, so there the second banner is the proof.
//!   Then a second prompt banner, and a prompt.
//! - **The file survived.** `wc reboot.txt` at the second prompt answers what the first prompt
//!   wrote. Under QEMU that is weaker than it sounds, and the BUGS say why.
//!
//! The same suite on all three architectures, under QEMU only. radon's reset waits on milestone
//! 592 (radon's cold reboot dies in OpenSBI's PMIC write); no gate here can say anything about
//! silicon.
//!
//! # EXAMPLES
//!
//! ```console
//! $ cargo xtask reboot-check --arch riscv64
//! --- reboot-check (riscv64): boot, write a file, type `reboot`, and come back ---
//! reboot-check (riscv64):   reboot: filesystem flushed (device flushes completed since boot: 1); restarting
//! reboot-check (riscv64): reboot: attempt 1 of 1: SBI SRST system_reset, reset type 1 (cold reboot). ...
//! reboot-check (riscv64): PASS, back at a prompt 19s after `reboot`, and the file reads back
//! ```
//!
//! # BUGS
//!
//! - **The read-back cannot fail for want of a flush under QEMU.** A guest reset leaves the QEMU
//!   process and its host-side cache alive, so a write the device never flushed survives anyway.
//!   The flush line is the evidence the flush happened; the read-back is evidence the machine that
//!   came back is a working one on the same disk. A power cut on silicon is where an unflushed
//!   write is lost, and no gate here cuts power.
//! - **Not in `cargo xtask test`.** It boots the interactive system, as `swish-check` does, and
//!   neither is part of `script/test`'s kernel-test legs; CI runs it in the `swish-check` jobs.
//!   Milestone 805's exit criterion 1 asked for `script/test`, and its block records the change.
//!
//! Name: provisional (milestone 805).

use std::io::{Read, Write};
use std::process::{Command, ExitCode};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::archive::{initrd_path, initrd_riscv, riscv_initrd_path};
use crate::disk::{disk_path, mkdisk, mkredoxfs, redoxfs_server_build};
use crate::host::flag_value;
use crate::suite::ArchLegs;
use crate::uefi::{esp_dir, uefi_image};
use crate::{RISCV_TARGET, RUNNER, TARGET, X86_TARGET, profile_dir, user};

/// The shell's banner, printed once per boot. `swish-check` waits for the same words.
const BANNER: &str = "nife capability shell";
/// The kernel's prefix on every line its reboot method prints (`kernel::reboot::MARKER`).
const KERNEL_MARKER: &str = "reboot: the kernel was asked to restart the machine";
/// What the kernel says when every route was refused (`kernel::reboot::restart`).
const KERNEL_REFUSED: &str = "reboot: every reset route was refused";
/// The `reboot` program's report, whichever of its three successful sentences it is.
const PROGRAM_REPORT: &str = "; restarting\n";
/// The file the first boot writes and the second reads back, and what `wc` says of it:
/// `survives a reboot` and its newline are 18 bytes, 3 words, 1 line.
const WRITE_LINE: &str = "echo survives a reboot > reboot.txt";
const READ_LINE: &str = "wc reboot.txt";
const READ_ANSWER: &str = "1 3 18";
/// Bounds. A boot under TCG is the long one; `swish-check` measured its own the same way.
const BOOT_SECS: u64 = 180;
const LINE_SECS: u64 = 45;

/// **`cargo xtask reboot-check [--arch aarch64|riscv64|x86_64]`**: every architecture by default.
pub(crate) fn reboot_check() -> ExitCode {
    let legs = match flag_value("--arch").as_deref() {
        None => ArchLegs::All,
        Some("aarch64") => ArchLegs::Aarch64,
        Some("riscv64") => ArchLegs::Riscv64,
        Some("x86_64") => ArchLegs::X86_64,
        Some(other) => {
            eprintln!(
                "reboot-check: --arch {other} is not an architecture (aarch64, riscv64 or x86_64)"
            );
            return ExitCode::from(2);
        }
    };
    // TCG, as `swish-check`: nothing here is a measurement, and the time is QEMU's serial.
    // SAFETY: single-threaded here; the reader thread below starts later and never touches the
    // environment.
    unsafe { std::env::remove_var("NIFE_ACCEL") };
    for (run, arch) in [
        (legs.aarch64(), "aarch64"),
        (legs.riscv64(), "riscv64"),
        (legs.x86_64(), "x86_64"),
    ] {
        if run && !leg(arch) {
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}

/// The firmware line that must follow the reset, where the architecture has firmware of its own
/// between the reset and the kernel.
fn firmware_needle(arch: &str) -> Option<&'static str> {
    match arch {
        "riscv64" => Some("OpenSBI"),
        "x86_64" => Some("uefi_loader"),
        _ => None,
    }
}

fn build(arch: &str) -> bool {
    match arch {
        "x86_64" => redoxfs_server_build(X86_TARGET) && mkdisk() && mkredoxfs() && uefi_image(),
        "riscv64" => {
            redoxfs_server_build(RISCV_TARGET)
                && mkdisk()
                && mkredoxfs()
                && initrd_riscv()
                && crate::cargo_profiled(&[
                    "build",
                    "-p",
                    "kernel",
                    "--features",
                    "shell",
                    "--target",
                    RISCV_TARGET,
                ])
        }
        _ => {
            redoxfs_server_build(TARGET)
                && mkredoxfs()
                && mkdisk()
                && user()
                && crate::cargo_profiled(&[
                    "build",
                    "-p",
                    "kernel",
                    "--features",
                    "shell",
                    "--target",
                    TARGET,
                ])
        }
    }
}

fn leg(arch: &str) -> bool {
    let x86 = arch == "x86_64";
    let riscv = arch == "riscv64";
    eprintln!();
    eprintln!("--- reboot-check ({arch}): boot, write a file, type `reboot`, and come back ---");
    if !build(arch) {
        eprintln!("reboot-check ({arch}): the build failed");
        return false;
    }

    let mut cmd = if x86 {
        let mut c = Command::new("helpers/qemu-uefi-x86_64.sh");
        c.arg(esp_dir());
        c.env(
            "NIFE_UEFI_TIMEOUT",
            (BOOT_SECS * 2 + LINE_SECS * 4).to_string(),
        );
        c.env_remove("NIFE_NVME");
        if crate::swish_check::kvm_is_usable() {
            c.env("NIFE_ACCEL", "kvm");
        }
        c.env("NIFE_UEFI_REDOXFS", "1");
        c
    } else {
        let target = if riscv { RISCV_TARGET } else { TARGET };
        let mut c = Command::new(if riscv {
            "helpers/qemu-runner-riscv64.sh"
        } else {
            RUNNER
        });
        c.arg(format!("target/{target}/{}/kernel", profile_dir()));
        c.env(
            "NIFE_INITRD",
            if riscv {
                riscv_initrd_path()
            } else {
                initrd_path()
            },
        );
        c
    };
    cmd.env("NIFE_DISK", disk_path());
    // The one run whose proof is a reset: the x86 runners otherwise pass `-no-reboot`.
    cmd.env("NIFE_ALLOW_REBOOT", "1");
    cmd.stdin(std::process::Stdio::piped());
    cmd.stdout(std::process::Stdio::piped());
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("reboot-check ({arch}): cannot start the runner: {e}");
            return false;
        }
    };
    let mut stdin = child.stdin.take().expect("piped stdin");
    let mut stdout = child.stdout.take().expect("piped stdout");
    let seen = Arc::new(Mutex::new(String::new()));
    let collector = Arc::clone(&seen);
    let reader = std::thread::spawn(move || {
        let mut buf = [0u8; 1024];
        while let Ok(n) = stdout.read(&mut buf) {
            if n == 0 {
                return;
            }
            let text = String::from_utf8_lossy(&buf[..n]).replace('\r', "");
            collector.lock().expect("transcript lock").push_str(&text);
        }
    });

    let len = || seen.lock().expect("transcript lock").len();
    let wait_after = |from: usize, needle: &str, secs: u64| -> Option<usize> {
        let deadline = Instant::now() + Duration::from_secs(secs);
        while Instant::now() < deadline {
            if let Some(at) = seen.lock().expect("transcript lock")[from..].find(needle) {
                return Some(from + at);
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        None
    };
    // Ready means the transcript ends in the bare prompt, `swish-check`'s rule for not typing
    // ahead of the line editor.
    let wait_prompt = |secs: u64| -> bool {
        let deadline = Instant::now() + Duration::from_secs(secs);
        while Instant::now() < deadline {
            if seen.lock().expect("transcript lock").ends_with("$ ") {
                return true;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        false
    };
    let mut type_line = |line: &str| -> bool {
        writeln!(stdin, "{line}")
            .and_then(|()| stdin.flush())
            .is_ok()
    };

    let mut failed: Option<String> = None;
    let mut fail = |why: String| {
        if failed.is_none() {
            failed = Some(why);
        }
    };
    let begun = Instant::now();
    let mut reset_at = None;
    'steps: {
        let Some(first) = wait_after(0, BANNER, BOOT_SECS) else {
            fail(format!(
                "no prompt banner within {BOOT_SECS}s of the first boot"
            ));
            break 'steps;
        };
        // x86_64's kernel finishes a hand-over report (milestone 299 (the x86 port-range
        // capability)) after the shell starts; Enter after it gives a fresh prompt, exactly as
        // `swish-check` does.
        if x86 {
            if wait_after(first, "as a port capability (milestone 299).", BOOT_SECS).is_none() {
                fail("the x86_64 kernel never finished its hand-over report".to_string());
                break 'steps;
            }
            type_line("");
        }
        if !wait_prompt(LINE_SECS) {
            fail("the first boot never reached a bare prompt".to_string());
            break 'steps;
        }
        let at = len();
        type_line(WRITE_LINE);
        if wait_after(at, &format!("{WRITE_LINE}\n"), LINE_SECS).is_none()
            || !wait_prompt(LINE_SECS)
        {
            fail(format!("`{WRITE_LINE}` never came back to a prompt"));
            break 'steps;
        }
        let typed_at = len();
        type_line("reboot");
        let Some(report) = wait_after(typed_at, PROGRAM_REPORT, LINE_SECS) else {
            fail("`reboot` never reported its flush (no line ending \"; restarting\")".to_string());
            break 'steps;
        };
        let Some(kernel) = wait_after(typed_at, KERNEL_MARKER, LINE_SECS) else {
            fail("the kernel never said it was asked to restart the machine".to_string());
            break 'steps;
        };
        if report > kernel {
            fail("the kernel's reset line came before `reboot`'s flush report".to_string());
            break 'steps;
        }
        if !seen.lock().expect("transcript lock")[typed_at..report]
            .contains("reboot: filesystem flushed")
        {
            fail("`reboot` restarted without a completed flush: the line does not say \"filesystem flushed\" (the transcript below has what it said)".to_string());
            break 'steps;
        }
        reset_at = Some(Instant::now());
        let after = if let Some(fw) = firmware_needle(arch) {
            match wait_after(kernel, fw, BOOT_SECS) {
                Some(at) => at,
                None => {
                    fail(format!(
                        "no {fw:?} line after the reset: the machine did not come back through its firmware"
                    ));
                    break 'steps;
                }
            }
        } else {
            kernel
        };
        if seen.lock().expect("transcript lock")[kernel..].contains(KERNEL_REFUSED) {
            fail("the firmware refused every reset route".to_string());
            break 'steps;
        }
        let Some(second) = wait_after(after, BANNER, BOOT_SECS) else {
            fail(format!(
                "no second prompt banner within {BOOT_SECS}s of the reset"
            ));
            break 'steps;
        };
        if x86 {
            if wait_after(second, "as a port capability (milestone 299).", BOOT_SECS).is_none() {
                fail("the second x86_64 boot never finished its hand-over report".to_string());
                break 'steps;
            }
            type_line("");
        }
        if !wait_prompt(LINE_SECS) {
            fail("the second boot never reached a bare prompt".to_string());
            break 'steps;
        }
        let at = len();
        type_line(READ_LINE);
        if wait_after(at, READ_ANSWER, LINE_SECS).is_none() {
            fail(format!(
                "`{READ_LINE}` after the reset did not answer {READ_ANSWER:?}: the file the first boot wrote is not there"
            ));
        }
    }

    // Kill whatever happened. SIGTERM on x86_64 because that runner forwards it to QEMU, the
    // other two `exec` the emulator. `swish-check`'s teardown, and the QEMU rule it keeps.
    if x86 {
        let _ = Command::new("kill")
            .args(["-TERM", &child.id().to_string()])
            .status();
    } else {
        let _ = child.kill();
    }
    let _ = child.wait();
    let _ = reader.join();

    let transcript = seen.lock().expect("transcript lock").clone();
    // The whole console, kept: a PASS prints only the lines below, and a person checking what it
    // passed on wants the rest.
    let log = format!("target/reboot-check-{arch}.log");
    let _ = std::fs::write(&log, &transcript);
    eprintln!(
        "reboot-check ({arch}): {} prompt banner(s) in {log}",
        transcript.matches(BANNER).count()
    );
    for l in transcript.lines().filter(|l| l.contains(READ_ANSWER)) {
        eprintln!("reboot-check ({arch}): after the reset, `{READ_LINE}` answered:{l}");
    }
    for l in transcript.lines().filter(|l| l.contains("reboot:")) {
        eprintln!("reboot-check ({arch}): {l}");
    }
    if transcript.contains("[PANIC]") {
        fail("the kernel panicked".to_string());
    }
    match failed {
        None => {
            eprintln!(
                "reboot-check ({arch}): PASS, back at a prompt {}s after `reboot` ({}s in all), \
                 and the file reads back",
                reset_at.map_or(0, |t| t.elapsed().as_secs()),
                begun.elapsed().as_secs()
            );
            true
        }
        Some(why) => {
            eprintln!("--- reboot-check ({arch}) transcript ---");
            eprintln!("{transcript}");
            eprintln!("reboot-check ({arch}): FAIL, {why}");
            false
        }
    }
}
