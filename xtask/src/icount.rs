//! The instruction-count instrument (milestone 78): the two timing claims a wall clock cannot
//! make, on both ISAs. See script/icount.

use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::host::{cargo, flag_value};
use crate::suite::ArchLegs;
use crate::{RISCV_TARGET, RUNNER, TARGET};

/// **The instruction-count instrument** (milestone 78;
/// design/roadmap/0078-load-sensitive-assertions.md), on both ISAs because parity is a gate (§19).
///
/// Boots a `--features icount` kernel under `-icount shift=0,sleep=off`, where QEMU's virtual clock
/// advances by exactly one nanosecond per guest instruction retired and by nothing else. The guest
/// asserts two claims a wall-clock test cannot make (that the timer fired at the deadline the kernel
/// armed, and that the handler costs fewer than N instructions) and prints what it measured.
///
/// **This is not on the test path and that is the design.** `-icount` changes what QEMU is, and the
/// two ways that matter are not the one the milestone block gave: it is **not** measurably slower on
/// compute (measured, 2026-08-17), but it gives every vCPU **one shared virtual clock**, which forces
/// `-smp 1` and would silently retire every cross-core property the suite proves, and it makes a
/// clock-bound wait cost instructions rather than host time. So the instrument gets its own boot,
/// exactly as `script/bench` does, and the test path is untouched. See notes/instruction-clock.md.
///
/// The verdict arrives the bench boot's way rather than through semihosting: the guest prints
/// `icount: done` and parks in `wfi`, this owns the child and kills it. A panic (a violated claim)
/// prints `[PANIC]` and is a failure; so is reaching end of output with neither.
pub(crate) fn icount() -> bool {
    let legs = match flag_value("--arch").as_deref() {
        None => ArchLegs::All,
        Some("aarch64") => ArchLegs::Aarch64,
        Some("riscv64") => ArchLegs::Riscv64,
        // x86_64 has no icount leg: the instrument compares against a deadline the kernel re-armed,
        // and this port's LAPIC timer is a periodic hardware reload (`bench_x86`'s doc). The
        // userspace reason this line used to give stopped being true with milestone 161 (the x86-64
        // kernel port); found stale by the 2026-09-29 documentation audit.
        Some(other) => {
            eprintln!("icount: --arch {other} is not an architecture (aarch64 or riscv64)");
            return false;
        }
    };
    if legs.aarch64() && !icount_leg("aarch64", RUNNER, TARGET) {
        return false;
    }
    if legs.riscv64() && !icount_leg("riscv64", "helpers/qemu-runner-riscv64.sh", RISCV_TARGET) {
        return false;
    }
    true
}

/// One ISA's instrument run: build, boot, read the transcript, report.
fn icount_leg(arch: &str, runner: &str, target: &str) -> bool {
    if !cargo(&[
        "build",
        "-p",
        "kernel",
        "--features",
        "icount",
        "--target",
        target,
    ]) {
        return false;
    }

    eprintln!();
    eprintln!(
        "--- icount: {arch}, single hart, TCG + icount (one instruction = one nanosecond) ---"
    );

    let mut cmd = Command::new(runner);
    cmd.arg(format!("target/{target}/debug/kernel"));
    cmd.args(["-icount", "shift=0,sleep=off"]);
    // One hart, for the reason the bench instrument pins it and the placement probe can never move
    // here: under `-icount` all vCPUs share ONE virtual clock, and an idle secondary parked in `wfi`
    // jumps that clock forward to the next event. A timer measurement on four harts would be
    // measuring three other harts' idle jumps. See notes/benchmarks.md.
    cmd.env("NIFE_SMP", "1");
    // No accelerator: HVF has no icount at all (it runs the physical core, which is the whole point
    // of it), so a stale `NIFE_ACCEL` from the caller's shell would silently produce wall-clock
    // numbers wearing instruction units. The guest's own calibration refuses that case too; this
    // stops it happening rather than catching it.
    cmd.env_remove("NIFE_ACCEL");
    // And no devices. Every one of these adds a source of interrupts, and an interrupt that is not
    // the timer landing inside the measured window would show up as the timer handler being late.
    // The suite attaches them because its tests assert they are present; this boot drives none of
    // them, so a variable left set by an earlier `script/test` in the same shell must not reach it.
    for device in [
        "NIFE_GPU",
        "NIFE_KEYBOARD",
        "NIFE_RNG",
        "NIFE_NVME",
        "NIFE_NET",
        "NIFE_DISK",
        "NIFE_INITRD",
        "NIFE_GPU_MON",
        // The `ramfb` of milestone 243 (a machine with no serial port). Not an interrupt source
        // like the rest of this list, but it is a device the guest talks to during the boot this
        // instrument measures, and a device the instrument did not measure last time is drift.
        "NIFE_SCREEN",
        "NIFE_SCREEN_MON",
    ] {
        cmd.env_remove(device);
    }
    cmd.stdout(std::process::Stdio::piped());

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("icount: failed to start the runner: {e}");
            return false;
        }
    };

    // **A deadline, on a thread, because the guest is not obliged to say anything.**
    //
    // The read below blocks, and a QEMU that has wedged before printing (or after panicking, since
    // this kernel's panic handler halts rather than exiting) never closes the pipe. This lane leaked
    // two emulators learning that, at 80% of a core each, on a laptop already carrying four other
    // lanes' gates. AGENTS.md's rule is that every unattended QEMU run is bounded, and a bound that
    // depends on the guest reaching a marker is not a bound.
    //
    // Generous on purpose: the instrument's own work is a few seconds, so this only ever fires on a
    // machine that is not making progress at all.
    const DEADLINE_SECS: u64 = 300;
    const PANIC_GRACE_SECS: u64 = 3;
    let pid = child.id();
    let finished = std::sync::Arc::new(AtomicBool::new(false));
    let panicked = std::sync::Arc::new(AtomicBool::new(false));
    let watchdog = {
        let finished = std::sync::Arc::clone(&finished);
        let panicked = std::sync::Arc::clone(&panicked);
        std::thread::spawn(move || {
            // Woken in slices so a normal run's thread goes away promptly rather than sleeping out
            // the whole deadline after everything else is done.
            let mut grace: Option<u64> = None;
            for _ in 0..DEADLINE_SECS {
                std::thread::sleep(std::time::Duration::from_secs(1));
                if finished.load(Ordering::Relaxed) {
                    return;
                }
                // A panic shortens the fuse rather than taking a path of its own. The reader below
                // keeps printing for a few seconds, however many lines the message runs to, and then
                // this kill closes the pipe and ends the read. Counting lines instead was the first
                // attempt and it hung: the panic printed two and the reader waited forever for a
                // third that a halted guest was never going to send.
                match grace {
                    _ if !panicked.load(Ordering::Relaxed) => {}
                    None => grace = Some(PANIC_GRACE_SECS),
                    Some(0) => break,
                    Some(n) => grace = Some(n - 1),
                }
            }
            if grace.is_none() {
                eprintln!("icount: no verdict in {DEADLINE_SECS}s; killing QEMU (pid {pid})");
            }
            // `kill(1)` rather than the `Child`, which the reading thread owns.
            let _ = Command::new("kill").arg("-9").arg(pid.to_string()).status();
        })
    };

    use std::io::BufRead;
    let stdout = child.stdout.take().expect("piped stdout");
    let reader = std::io::BufReader::new(stdout);
    let mut done = false;
    for line in reader.lines() {
        let Ok(line) = line else { break };
        // The guest's own lines, verbatim: the numbers are the deliverable, not a summary of them.
        if let Some(rest) = line.strip_prefix("icount: ") {
            eprintln!("  {rest}");
            if rest == "done" {
                done = true;
                break;
            }
            continue;
        }
        // A violated claim is a panic in the guest, and its message is written to say which claim
        // moved and by how much. Print it and everything after it; the watchdog's grace period ends
        // the read, because this kernel's panic handler halts rather than exiting and the pipe would
        // otherwise never close.
        if panicked.load(Ordering::Relaxed) || line.contains("[PANIC]") {
            eprintln!("  {line}");
            panicked.store(true, Ordering::Relaxed);
        }
    }
    finished.store(true, Ordering::Relaxed);
    let _ = child.kill();
    let _ = child.wait();
    let _ = watchdog.join();
    let panicked = panicked.load(Ordering::Relaxed);

    if panicked {
        eprintln!("icount: {arch} FAILED a claim (the panic above says which)");
        return false;
    }
    if !done {
        eprintln!("icount: {arch} QEMU ended before printing `icount: done`; no verdict");
        return false;
    }
    eprintln!("icount: {arch} claims hold");
    true
}
