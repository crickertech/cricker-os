//! The sustained runs under QEMU: the multicore soak (milestone 219) and the multi-tasking
//! job mix (milestone 168).
//!
//! Both return their own exit codes rather than a bool, because a rehearsal that cannot say
//! *how* it failed is not a rehearsal.

use std::path::PathBuf;
use std::process::{Command, ExitCode};
use std::sync::atomic::Ordering;

use crate::archive::{initrd_path, initrd_riscv, initrd_x86, riscv_initrd_path, x86_initrd_path};
use crate::board::{Tee, parse_duration};
use crate::disk::mkdisk;
use crate::host::workspace_root;
use crate::{
    RELEASE, RISCV_TARGET, TARGET, X86_TARGET, cargo_profiled, maybe_hvf, profile_dir, user,
};

/// **The QEMU rehearsal of milestone 168's multi-tasking workload sweep.** Boot a
/// `--features job_mix` kernel, echo its lines, stop when it says it is done, and kill it.
///
/// **This is a rehearsal and not the measurement**, and the distinction is the milestone's whole
/// gate. Under TCG the magnitudes are fiction (no caches are modelled) and under HVF the host
/// scheduler is underneath every guest thread; the number DECISIONS §96 is waiting for is taken on
/// radon, by `notes/job-mix.md`'s procedure. What this proves is that the workload runs, that the
/// sweep completes, and that the output is the shape the bench evening will read.
///
/// **The judging is `board_console`'s**, not this function's (milestone 324 part 2), which is the
/// same move `soak_test` below already made and for the same reason: a second reader drifts from
/// the bench-side one the first time either changes, and the cheapest way for two things to agree
/// is for there to be one of them. What stood here instead was a loop over `starts_with("job-mix")`
/// with no timeout at all, so a sweep that wedged mid-subrun hung this command forever and a
/// finished sweep and a dead one shared an exit status. That is the limitation milestone 168's lane
/// filed and milestone 324's part 2 names.
///
/// The kernel parks in `wfi` rather than exiting, so the host side owns the process and tears it
/// down; that half is `run_bench`'s and is unchanged. See `script/job-mix`.
pub(crate) fn job_mix_sweep() -> ExitCode {
    use std::io::Write;
    use std::time::Duration;

    use board_console::progress::Stage;
    use board_console::watch::{Policy, watch};

    let args: Vec<String> = std::env::args().skip(2).collect();
    let mut arch = "aarch64".to_string();
    let mut smp: Option<String> = None;
    let mut log: Option<PathBuf> = None;
    let mut policy = Policy {
        // Ten minutes. The whole sweep is eighteen subruns and took well under a minute on the
        // machine this was written on; the cap is for the run that never finishes, and a cap that
        // is too generous costs a slow failure where one that is too tight costs a wrong answer.
        total: Duration::from_secs(600),
        until: Some(Stage::SweepDone),
        // **Sized against a subrun, not against a heartbeat**, which is the one way a sweep is
        // harder to watch than a soak. `kernel/src/soak.rs` beats on the wall clock every five
        // seconds whatever it is doing, so fifteen is three missed beats. A sweep speaks only when
        // a subrun ends, and the longest is the top of `job_mix::TASK_SWEEP`: measured at
        // 249,234,771 ticks on a 62.5 MHz counter, which is 4.0 seconds, in the capture at
        // `crates/board_console/tests/fixtures/captured/qemu-2026-09-19-aarch64-job-mix-medians.log`.
        // Sixty seconds is fifteen times that, which is headroom for a slower host and still names
        // a wedge inside a minute. It was twenty times a 2.6-second subrun until milestone 168
        // took twenty-one repeats of a seven-kind mix instead of three of a five-kind one, which
        // is the margin being spent by a change nowhere near this line. Overridable, because the
        // number is a default rather than an agreement.
        quiet_after: Some(Duration::from_secs(60)),
        // Nothing this kernel prints after `job-mix: done` can change the verdict: it halts. The
        // settle window exists for the measured-boot refusal that arrives *after* the awaited rung,
        // and a sweep that has printed its last point is past every such gate.
        settle: Duration::from_secs(0),
        // An emulator has no firmware prologue to climb; see `boot_check`'s note.
        board: &board_console::board::XENON,
    };
    // `--hvf` and `--release` (added 2026-09-19 for the HVF cross-check in notes/job-mix.md): the
    // two flags the tree already spells this way, `run`'s and `bench`'s, rather than new ones. HVF
    // is aarch64 on an Apple host only, and release is what `script/board-image` builds for radon,
    // so the cross-check runs the optimisation level the board does.
    let mut hvf = false;
    // `--lock-wait` (2026-10-04, fatal risk 4; provisional): build `--features lock_wait`, the job
    // mix plus kernel/src/lock_wait.rs's count of contended lock acquisitions, which prints
    // `job-mix-lock:` lines after each point. The kernel feature's name, hyphenated the way this
    // script's flags are.
    let mut lock_wait = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--hvf" => {
                hvf = true;
                i += 1;
                continue;
            }
            "--lock-wait" => {
                lock_wait = true;
                i += 1;
                continue;
            }
            "--release" => {
                RELEASE.store(true, Ordering::Relaxed);
                i += 1;
                continue;
            }
            _ => {}
        }
        let value = |i: usize| -> Result<&str, ExitCode> {
            args.get(i + 1).map(String::as_str).ok_or_else(|| {
                eprintln!("job-mix: {} wants a value", args[i]);
                ExitCode::from(4)
            })
        };
        match args[i].as_str() {
            "--arch" => match value(i) {
                Ok(v) => arch = v.to_string(),
                Err(code) => return code,
            },
            "--smp" => match value(i) {
                Ok(v) => smp = Some(v.to_string()),
                Err(code) => return code,
            },
            "--log" => match value(i) {
                Ok(v) => log = Some(PathBuf::from(v)),
                Err(code) => return code,
            },
            "--for" | "--timeout" => match value(i).map(parse_duration) {
                Ok(Some(d)) => policy.total = d,
                Ok(None) => {
                    eprintln!("job-mix: --for wants a duration like 90, 90s, 30m or 2h");
                    return ExitCode::from(4);
                }
                Err(code) => return code,
            },
            "--quiet-after" => match value(i).map(parse_duration) {
                // Zero disables it, for the operator who knows their board is slower than any
                // number written here and would rather wait out the cap than be told it wedged.
                Ok(Some(d)) => policy.quiet_after = if d.is_zero() { None } else { Some(d) },
                Ok(None) => {
                    eprintln!("job-mix: --quiet-after wants a duration, or 0 to disable");
                    return ExitCode::from(4);
                }
                Err(code) => return code,
            },
            other => {
                eprintln!("job-mix: unknown argument {other}");
                eprintln!(
                    "usage: cargo xtask job-mix [--arch aarch64|riscv64|x86_64] [--smp <n>] \
                     [--hvf] [--release] [--for <duration>] [--quiet-after <duration>] \
                     [--log <file>]"
                );
                return ExitCode::from(4);
            }
        }
        i += 2;
    }

    if hvf && arch != "aarch64" {
        eprintln!("job-mix: --hvf runs the Apple core, so it is aarch64 only");
        return ExitCode::from(4);
    }
    if hvf {
        maybe_hvf();
    }

    let (target, runner, initrd) = match arch.as_str() {
        "aarch64" => {
            if !(mkdisk() && user()) {
                return ExitCode::from(4);
            }
            (TARGET, "helpers/qemu-runner-aarch64.sh", initrd_path())
        }
        // **`mkdisk` here too, not only on aarch64** (found 2026-09-19 by the lane that closed
        // milestone 168's sampling hole): the riscv64 runner refuses a `NIFE_DISK` naming a missing
        // file, so on a fresh worktree `--arch riscv64` died before the kernel printed a line, and
        // it only ever passed on a checkout where an aarch64 run had made the image first. The
        // sweep reads no disk; the runner's own check is what needs it.
        "riscv64" => {
            if !(mkdisk() && initrd_riscv()) {
                return ExitCode::from(4);
            }
            (
                RISCV_TARGET,
                "helpers/qemu-runner-riscv64.sh",
                riscv_initrd_path(),
            )
        }
        "x86_64" => {
            if !initrd_x86() {
                return ExitCode::from(4);
            }
            (
                X86_TARGET,
                "helpers/qemu-runner-x86_64.sh",
                x86_initrd_path(),
            )
        }
        other => {
            eprintln!("job-mix: unknown architecture {other} (aarch64, riscv64 or x86_64)");
            return ExitCode::from(4);
        }
    };

    if !cargo_profiled(&[
        "build",
        "-p",
        "kernel",
        "--features",
        if lock_wait { "lock_wait" } else { "job_mix" },
        "--target",
        target,
    ]) {
        return ExitCode::from(4);
    }

    // The log is not optional, for `soak-test`'s reason: a console session whose evidence exists
    // only in a terminal that has since scrolled is the failure this tree keeps writing down, and a
    // capture can be re-read with `script/board-console --replay`.
    let log_path = log.unwrap_or_else(|| {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        PathBuf::from(format!("target/job-mix-{arch}-{stamp}.log"))
    });
    if let Some(parent) = log_path.parent()
        && let Err(e) = std::fs::create_dir_all(parent)
    {
        eprintln!("job-mix: cannot create {}: {e}", parent.display());
        return ExitCode::from(4);
    }
    let file = match std::fs::File::create(&log_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("job-mix: cannot write {}: {e}", log_path.display());
            return ExitCode::from(4);
        }
    };
    let mut sink = Tee {
        file,
        terminal: std::io::stdout(),
    };

    let mut cmd = Command::new(runner);
    cmd.arg(format!(
        "{}/target/{target}/{}/kernel",
        workspace_root().display(),
        profile_dir()
    ));
    cmd.env("NIFE_INITRD", &initrd);
    // **No disk on `x86_64`**, `boot_check`'s fix applied here (found 2026-09-25 by milestone 593,
    // provisional number, in a fresh worktree). `host::cargo` exports a `NIFE_DISK` naming a file
    // only `mkdisk` writes, only the aarch64 leg above calls `mkdisk`, and the `x86_64` runner treats
    // a named but missing disk as fatal. So `--arch x86_64` passed only in a checkout where some
    // earlier command had left the image behind. The soak reads no disk.
    if arch == "x86_64" {
        cmd.env_remove("NIFE_DISK");
    }
    if let Some(n) = &smp {
        cmd.env("NIFE_SMP", n);
    }
    cmd.stdout(std::process::Stdio::piped());
    // The runner's diagnostics stay on this terminal rather than joining the captured stream, so
    // that a replay of the log sees what a serial cable would have seen. `soak_test`'s reason.
    cmd.stderr(std::process::Stdio::inherit());

    eprintln!(
        "--- job-mix: {arch}, a rehearsal; the number is taken on radon (notes/job-mix.md), \
         logging to {} ---",
        log_path.display()
    );
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("job-mix: cannot start {runner}: {e}");
            return ExitCode::from(4);
        }
    };
    let runner_pid = child.id();
    let Some(stdout) = child.stdout.take() else {
        eprintln!("job-mix: the runner gave us no stdout to read");
        let _ = child.kill();
        return ExitCode::from(4);
    };

    // `false`, not `true`: a pipe from a process really does end when that process dies, which a
    // serial port never does.
    let session = watch(stdout, &mut sink, &policy, false);

    // The children first and then the wrapper, `run_bench`'s order and for its reason: the x86
    // runner does not `exec`, so killing the wrapper alone orphans the emulator.
    let _ = Command::new("pkill")
        .args(["-9", "-P", &runner_pid.to_string()])
        .status();
    let _ = child.kill();
    let _ = child.wait();
    let _ = sink.flush();

    let session = match session {
        Ok(s) => s,
        Err(e) => {
            eprintln!("job-mix: {e}");
            eprintln!("job-mix: log at {}", log_path.display());
            return ExitCode::from(4);
        }
    };

    eprintln!();
    eprintln!("job-mix: {}", session.summary());
    match session.progress.sweep_point() {
        // The spread and not the median alone: the kernel prints the two ends so a figure is
        // never quoted without them, and a summary that dropped them here would undo that at the
        // one line a person reads instead of the log.
        Some(point) => eprintln!(
            "job-mix: last point tasks={} jobs={} repeats={} ticks_min={} ticks_median={} \
             ticks_max={} jpm_median={}",
            point.tasks,
            point.jobs,
            point.repeats,
            point.ticks_min,
            point.ticks_median,
            point.ticks_max,
            point.jpm_median
        ),
        // Said out loud rather than left as an absence, because an empty tail is exactly what a
        // kernel that refused and a kernel that wedged before its first point both look like.
        None => eprintln!("job-mix: no point of the sweep was measured"),
    }
    eprintln!("job-mix: log at {}", log_path.display());
    // Said on every clean run, on purpose, for `soak_test`'s reason: this is the sentence that gets
    // dropped when a number is quoted.
    eprintln!(
        "job-mix: these magnitudes are NOT the measurement. TCG models no cache and HVF puts a \
         host scheduler under every guest thread; DECISIONS \u{a7}96's number is taken on radon."
    );
    // **The same five statuses `script/board-console` returns**, computed by the same code. `2` is
    // new here and it is part 2's whole point: a sweep that spoke and then stopped is a hang, where
    // before this it was indistinguishable from one that finished.
    ExitCode::from(u8::try_from(session.exit_code()).unwrap_or(4))
}

/// **The QEMU half of milestone 219's sustained run.** Boot a `--features soak_test` kernel, watch it
/// with the same recogniser and the same policy `script/board-console` points at a real board, and
/// return the same exit statuses.
///
/// One recogniser, two sources, is the whole design. The alternative was a QEMU-side checker of its
/// own, and it would have drifted from the board-side one the first time either changed; milestone
/// 219's block is explicit that the workload and the console must agree about what a hang is, and
/// the cheapest way for two things to agree is for there to be one of them.
///
/// What this adds over `board_console` is only what a board does not need: building the kernel and
/// the archive, starting QEMU, and killing it afterwards. See `script/soak-test`.
///
/// Named for the command rather than for the workload (milestone 297): this function *is*
/// `script/soak-test`, where `BootProgress::soak` one crate over reports on the workload, which is
/// still a soak and keeps that spelling.
pub(crate) fn soak_test() -> ExitCode {
    use std::io::Write;
    use std::time::Duration;

    use board_console::watch::{Outcome, Policy, watch};

    let args: Vec<String> = std::env::args().skip(2).collect();
    let mut arch = "aarch64".to_string();
    let mut smp: Option<String> = None;
    let mut log: Option<PathBuf> = None;
    // Milestone 249 (the boot lottery is sampled by a person walking to the board)'s QEMU proof: build the rebooting soak and pass only when a second boot starts
    // soaking after the first asked for a reset. See [`second_boot`].
    let mut reboot = false;
    // Milestone 593 (a wedged kernel resets itself)'s two QEMU proofs (number provisional), x86_64
    // only: a healthy watchdog soak is never reset over a bounded window, and a wedged one is. See
    // [`watchdog_verdict`].
    let mut watchdog = false;
    let mut wedge = false;
    let mut duration_given = false;
    // A minute by default: long enough that the beat, the rate and the cross-core counters are all
    // real numbers rather than a first sample, and short enough that nobody is tempted to skip it.
    // The runs that matter are hours long and happen on a board.
    let mut policy = Policy {
        total: Duration::from_secs(60),
        until: None,
        quiet_after: Some(Duration::from_secs(15)),
        settle: Duration::from_secs(0),
        // An emulator has no firmware prologue to climb; see `boot_check`'s note above.
        board: &board_console::board::XENON,
    };

    let mut i = 0;
    while i < args.len() {
        let value = |i: usize| -> Result<&str, ExitCode> {
            args.get(i + 1).map(String::as_str).ok_or_else(|| {
                eprintln!("soak-test: {} wants a value", args[i]);
                ExitCode::from(4)
            })
        };
        match args[i].as_str() {
            "--arch" => match value(i) {
                Ok(v) => arch = v.to_string(),
                Err(code) => return code,
            },
            "--smp" => match value(i) {
                Ok(v) => smp = Some(v.to_string()),
                Err(code) => return code,
            },
            "--log" => match value(i) {
                Ok(v) => log = Some(PathBuf::from(v)),
                Err(code) => return code,
            },
            "--reboot" => {
                reboot = true;
                i += 1;
                continue;
            }
            "--watchdog" => {
                watchdog = true;
                i += 1;
                continue;
            }
            "--wedge" => {
                wedge = true;
                i += 1;
                continue;
            }
            "--for" | "--timeout" => match value(i).map(parse_duration) {
                Ok(Some(d)) => {
                    policy.total = d;
                    duration_given = true;
                }
                Ok(None) => {
                    eprintln!("soak-test: --for wants a duration like 90, 90s, 30m or 2h");
                    return ExitCode::from(4);
                }
                Err(code) => return code,
            },
            "--quiet-after" => match value(i).map(parse_duration) {
                Ok(Some(d)) => policy.quiet_after = if d.is_zero() { None } else { Some(d) },
                Ok(None) => {
                    eprintln!("soak-test: --quiet-after wants a duration, or 0 to disable");
                    return ExitCode::from(4);
                }
                Err(code) => return code,
            },
            other => {
                eprintln!("soak-test: unknown argument {other}");
                eprintln!(
                    "usage: cargo xtask soak-test [--arch aarch64|riscv64|x86_64] [--for <duration>] \
                     [--smp <n>] [--quiet-after <duration>] [--log <file>] [--reboot | --watchdog | --wedge]"
                );
                return ExitCode::from(4);
            }
        }
        i += 2;
    }

    if u8::from(reboot) + u8::from(watchdog) + u8::from(wedge) > 1 {
        eprintln!("soak-test: --reboot, --watchdog and --wedge are three different runs; pick one");
        return ExitCode::from(4);
    }
    if (watchdog || wedge) && arch != "x86_64" {
        eprintln!(
            "soak-test: --watchdog and --wedge are x86_64-only: the watchdog is the Intel TCO, which \
             QEMU's q35 emulates and neither virt board has (milestone 593's block)"
        );
        return ExitCode::from(4);
    }

    let (target, runner, initrd) = match arch.as_str() {
        "aarch64" => {
            if !(mkdisk() && user()) {
                return ExitCode::from(4);
            }
            (TARGET, "helpers/qemu-runner-aarch64.sh", initrd_path())
        }
        // `mkdisk` for job-mix's reason above (found again 2026-09-25 by milestone 592 (radon's cold reboot dies in OpenSBI's PMIC write)'s
        // lane,
        // whose fresh worktree failed `--reboot --arch riscv64` with `starts=0` before the kernel
        // printed a line): the riscv64 runner refuses a `NIFE_DISK` naming a missing file.
        "riscv64" => {
            if !(mkdisk() && initrd_riscv()) {
                return ExitCode::from(4);
            }
            (
                RISCV_TARGET,
                "helpers/qemu-runner-riscv64.sh",
                riscv_initrd_path(),
            )
        }
        "x86_64" => {
            if !initrd_x86() {
                return ExitCode::from(4);
            }
            (
                X86_TARGET,
                "helpers/qemu-runner-x86_64.sh",
                x86_initrd_path(),
            )
        }
        other => {
            eprintln!("soak-test: unknown architecture {other} (aarch64, riscv64 or x86_64)");
            return ExitCode::from(4);
        }
    };

    // One window of `REBOOT_AFTER_SECONDS` (120s), the five-second grace, the reset attempts and two
    // boots fit in six minutes under TCG with room to spare.
    if reboot && !duration_given {
        policy.total = Duration::from_secs(360);
    }
    // The wedge comes 30s into the soak and the reset 60s after the last pet; two boots and that
    // fit in six minutes for the same reason as above. The healthy run's window is counted from the
    // moment the watchdog is armed, and three minutes is three full resets' worth of time.
    if !duration_given {
        if wedge {
            policy.total = Duration::from_secs(360);
        } else if watchdog {
            policy.total = Duration::from_secs(180);
        }
    }
    if !cargo_profiled(&[
        "build",
        "-p",
        "kernel",
        "--features",
        if reboot {
            "reboot_soak_test"
        } else if wedge {
            "wedge_soak_test"
        } else if watchdog {
            "watchdog_soak_test"
        } else {
            "soak_test"
        },
        "--target",
        target,
    ]) {
        return ExitCode::from(4);
    }

    let log_path = log.unwrap_or_else(|| {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        PathBuf::from(format!("target/soak-test-{arch}-{stamp}.log"))
    });
    if let Some(parent) = log_path.parent()
        && let Err(e) = std::fs::create_dir_all(parent)
    {
        eprintln!("soak-test: cannot create {}: {e}", parent.display());
        return ExitCode::from(4);
    }
    let file = match std::fs::File::create(&log_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("soak-test: cannot write {}: {e}", log_path.display());
            return ExitCode::from(4);
        }
    };
    let mut sink = Tee {
        file,
        terminal: std::io::stdout(),
    };

    let mut cmd = Command::new(runner);
    cmd.arg(format!(
        "{}/target/{target}/{}/kernel",
        workspace_root().display(),
        profile_dir()
    ));
    cmd.env("NIFE_INITRD", &initrd);
    // **No disk on `x86_64`**, `boot_check`'s fix applied here (found 2026-09-25 by milestone 593,
    // provisional number, in a fresh worktree). `host::cargo` exports a `NIFE_DISK` naming a file
    // only `mkdisk` writes, only the aarch64 leg above calls `mkdisk`, and the `x86_64` runner treats
    // a named but missing disk as fatal. So `--arch x86_64` passed only in a checkout where some
    // earlier command had left the image behind. The soak reads no disk.
    if arch == "x86_64" {
        cmd.env_remove("NIFE_DISK");
    }
    if let Some(n) = &smp {
        cmd.env("NIFE_SMP", n);
    }
    if reboot || watchdog || wedge {
        // The x86 runner passes `-no-reboot` so a triple fault exits instead of looping; this run
        // is the one where a reset must reset. The other two runners never pass it.
        cmd.env("NIFE_ALLOW_REBOOT", "1");
        // Nothing on stdin, so nothing can reach the guest's console and disarm the loop: the
        // escape is the byte-on-the-console the kernel polls for, and a terminal's stray keypress
        // would turn this proof into a run that never resets.
        cmd.stdin(std::process::Stdio::null());
    }
    cmd.stdout(std::process::Stdio::piped());
    // The runner's own diagnostics stay on this terminal rather than joining the captured stream:
    // the log is meant to be the guest's console and nothing else, so that a replay through
    // `script/board-console --replay` sees what a serial cable would have seen.
    cmd.stderr(std::process::Stdio::inherit());

    eprintln!(
        "--- soak-test: {arch}, up to {:?}, logging to {} ---",
        policy.total,
        log_path.display()
    );
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("soak-test: cannot start {runner}: {e}");
            return ExitCode::from(4);
        }
    };
    let runner_pid = child.id();
    let Some(stdout) = child.stdout.take() else {
        eprintln!("soak-test: the runner gave us no stdout to read");
        let _ = child.kill();
        return ExitCode::from(4);
    };

    if reboot {
        let verdict = second_boot(stdout, &mut sink, policy.total);
        let _ = Command::new("pkill")
            .args(["-9", "-P", &runner_pid.to_string()])
            .status();
        let _ = child.kill();
        let _ = child.wait();
        let _ = sink.flush();
        eprintln!("soak-test: log at {}", log_path.display());
        return verdict;
    }
    if watchdog || wedge {
        let verdict = watchdog_verdict(stdout, &mut sink, policy.total, wedge);
        let _ = Command::new("pkill")
            .args(["-9", "-P", &runner_pid.to_string()])
            .status();
        let _ = child.kill();
        let _ = child.wait();
        let _ = sink.flush();
        eprintln!("soak-test: log at {}", log_path.display());
        return verdict;
    }

    // `false`, not `true`: a pipe from a process really does end when that process dies, which a
    // serial port never does. Getting this bit wrong turns a QEMU that died into a board that has
    // not spoken yet.
    let session = watch(stdout, &mut sink, &policy, false);

    // Kill it whatever happened. A nife kernel that has finished its work sits in `wfi` forever and
    // QEMU with it, and this one never even finishes: leaving it running is the leak `AGENTS.md`
    // spends a whole section on.
    //
    // **The children first, then the wrapper**, the same order and for the same reason as
    // `run_bench` above: `helpers/qemu-runner-x86_64.sh` runs QEMU as a plain foreground child
    // rather than `exec`-ing into it, so killing the wrapper alone orphans the emulator. Found on
    // 2026-09-01 by an x86 soak that returned 0 and left a `qemu-system-x86_64` behind holding this
    // pipe. The other two runners `exec`, so `pkill -P` finds nothing there and costs one process.
    let _ = Command::new("pkill")
        .args(["-9", "-P", &runner_pid.to_string()])
        .status();
    let _ = child.kill();
    let _ = child.wait();
    let _ = sink.flush();

    let session = match session {
        Ok(s) => s,
        Err(e) => {
            eprintln!("soak-test: {e}");
            eprintln!("soak-test: log at {}", log_path.display());
            return ExitCode::from(4);
        }
    };

    eprintln!();
    eprintln!("soak-test: {}", session.summary());
    match session.progress.soak() {
        Some(beat) => {
            eprintln!(
                "soak-test: {} round trips in {}s ({} /s at the last beat), {} cross-core handoffs",
                beat.rounds, beat.seconds, beat.rate, beat.crossings
            );
            eprintln!(
                "soak-test: refused={} mismatch={} stalled={} (each must be 0)",
                beat.refused, beat.mismatches, beat.stalled
            );
            // Said on every clean run, on purpose, because this is the sentence the milestone's own
            // BUGS section says will otherwise be dropped when the number is quoted.
            eprintln!(
                "soak-test: a clean run is a number to compare against, NOT evidence that the \
                 concurrency is correct."
            );
            // **What the crossings are, said on every run that has any** (milestone 221). The
            // kernel prints this at the start of a soak, but a reader quoting this summary never
            // saw that, and the wrong reading is available and flattering: that the IPC workload
            // itself is migrating. It is not, and only a periodic rebalancer would make it, which
            // DECISIONS 138 (how a saturated workload is made to hand threads across cores)
            // declines.
            if beat.wakes > 0 {
                eprintln!(
                    "soak-test: the {} crossings are tick waiters being placed by the wake protocol \
                     ({} tick wakes drove them), NOT the IPC pairs migrating. See notes/soak.md.",
                    beat.crossings, beat.wakes
                );
            }
            // The gap milestone 219 measured rather than assumed, said on every run that shows it
            // because a reader who does not know it will read the round-trip total as covering
            // more than it does. See notes/soak.md.
            if beat.crossings < beat.rounds / 1000 {
                if beat.wakes == 0 {
                    eprintln!(
                        "soak-test: and it barely crossed cores ({} handoffs against {} round trips): \
                         this scheduler does not rebalance, so a saturated workload stays where it \
                         was placed. See notes/soak.md.",
                        beat.crossings, beat.rounds
                    );
                } else {
                    // The tick route was live and the machine still did not cross, which on one
                    // core is the only possible answer and on several is a finding. Both are named
                    // rather than one being assumed, because this summary cannot see the core
                    // count and the kernel's own banner can.
                    eprintln!(
                        "soak-test: and it barely crossed cores ({} handoffs against {} tick wakes), \
                         which on a single-core run is arithmetic and on a multicore one is a \
                         finding: check the core count in the soak's own start line.",
                        beat.crossings, beat.wakes
                    );
                }
            }
        }
        None => eprintln!("soak-test: no heartbeat was seen; the workload never started"),
    }
    eprintln!("soak-test: log at {}", log_path.display());

    // The one judgement that is this driver's rather than the watcher's, because it is about a
    // process and not about a board. A serial port cannot end; a pipe can, and QEMU exiting before
    // the deadline means the guest is gone. `board_console` scores `Ended` as success when nothing
    // was being waited for, which is right for a replayed capture and wrong here.
    if session.outcome == Outcome::Ended && session.elapsed + Duration::from_secs(1) < policy.total
    {
        eprintln!(
            "soak-test: QEMU exited after {:?}, before the deadline",
            session.elapsed
        );
        return ExitCode::from(3);
    }
    if session.progress.soak().is_none() {
        return ExitCode::from(3);
    }
    ExitCode::from(u8::try_from(session.exit_code()).unwrap_or(4))
}

/// **Did the machine actually come back?** Milestone 249's proof under QEMU, for `soak-test --reboot`.
///
/// The rebooting soak's claim is not that `arch::reboot` was called, or even that it did not
/// return: it is that the machine reset and booted this kernel again. So the only passing outcome is
/// the soak's own start line appearing **a second time, after** a `rebooting now` line, which QEMU
/// can only produce by resetting the machine and loading `-kernel` again. A reset that hung in
/// firmware, a guest that printed its reset line and stopped, and a QEMU that exited on the reset
/// all fail here, which is the difference this test exists to draw.
///
/// It reads the stream itself rather than going through `board_console::watch`, because that
/// recogniser judges one boot and treats a second banner as the story starting over; teaching it
/// multi-boot runs is `board_console::lottery`'s job on a real capture, not this proof's.
///
/// Exit statuses follow `soak-test`'s: 0 came back, 1 the kernel said the reset failed or panicked,
/// 2 the deadline passed first, 3 QEMU's output ended.
fn second_boot(
    stdout: std::process::ChildStdout,
    sink: &mut impl std::io::Write,
    deadline: std::time::Duration,
) -> ExitCode {
    use std::sync::mpsc;
    use std::time::Instant;

    // The soak's own start line and the reboot loop's prefix, as `kernel/src/soak.rs` spells them.
    const STARTED: &str = "soak-test: started";
    const REBOOT: &str = "soak-test-reboot:";

    let rx = console_lines(stdout);

    let begun = Instant::now();
    let mut starts = 0u32;
    let mut rebooting = false;
    let mut last_attempt: Option<String> = None;
    loop {
        let Some(left) = deadline.checked_sub(begun.elapsed()) else {
            eprintln!(
                "soak-test: FAIL, {}s passed without a second boot (starts={starts}, reset asked \
                 for: {rebooting})",
                deadline.as_secs()
            );
            return ExitCode::from(2);
        };
        let line = match rx.recv_timeout(left) {
            Ok(line) => line,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                eprintln!(
                    "soak-test: FAIL, QEMU's output ended (starts={starts}, reset asked for: \
                     {rebooting}); a QEMU that exits on a reset has not rebooted anything"
                );
                return ExitCode::from(3);
            }
        };
        let _ = writeln!(sink, "{line}");
        if line.contains("[PANIC]") {
            eprintln!("soak-test: FAIL, the kernel panicked: {line}");
            return ExitCode::from(1);
        }
        if let Some(rest) = line.split_once(REBOOT).map(|(_, rest)| rest.trim()) {
            if rest.starts_with("rebooting now") {
                rebooting = true;
            } else if rest.starts_with("attempt") {
                last_attempt = Some(rest.to_string());
            } else if rest.starts_with("FAILED") || rest.starts_with("DISARMED") {
                eprintln!("soak-test: FAIL, the reboot loop stopped: {rest}");
                return ExitCode::from(1);
            }
        }
        if line.contains(STARTED) {
            starts += 1;
            if rebooting && starts >= 2 {
                eprintln!();
                eprintln!(
                    "soak-test: PASS, the machine reset and this kernel booted and began soaking \
                     again, {}s in",
                    begun.elapsed().as_secs()
                );
                if let Some(attempt) = last_attempt {
                    eprintln!("soak-test: the route that reset it: {attempt}");
                }
                return ExitCode::SUCCESS;
            }
        }
    }
}

/// The guest's console as a channel of lines, read on a thread of its own so the caller can wait on
/// it with a deadline. Shared by [`second_boot`] and [`watchdog_verdict`].
fn console_lines(stdout: std::process::ChildStdout) -> std::sync::mpsc::Receiver<String> {
    use std::io::BufRead;

    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        for line in std::io::BufReader::new(stdout).split(b'\n') {
            let Ok(line) = line else { break };
            if tx
                .send(String::from_utf8_lossy(&line).into_owned())
                .is_err()
            {
                break;
            }
        }
    });
    rx
}

/// **Milestone 593's two proofs under QEMU q35**: the watchdog resets a wedged kernel, and it never
/// resets a healthy one. Provisional number.
///
/// **`--wedge`** passes on three facts in order. The watchdog armed. The kernel announced its
/// deliberate wedge. Then the soak's start line appeared a second time, milestone 249's "it came
/// back" criterion, and that second boot reported `SECOND_TO_STS` set. The last fact is what makes
/// it this watchdog's reset rather than any reset: the chipset sets that bit only on a second
/// timeout, and nothing else in QEMU's reset path touches it.
///
/// **`--watchdog`** passes when `window` has elapsed since the watchdog armed with one boot and no
/// failure, and at least one pet reported a count below its reload value. That last condition is
/// the difference between a watchdog that was petted and one that was never counting: a halted TCO
/// reads its reload value forever, and a run that passed on it would prove nothing.
///
/// Exit statuses follow `soak-test`'s: 0 proven, 1 the kernel said something that disproves it, 2
/// the deadline passed first, 3 QEMU's output ended.
fn watchdog_verdict(
    stdout: std::process::ChildStdout,
    sink: &mut impl std::io::Write,
    window: std::time::Duration,
    wedge: bool,
) -> ExitCode {
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    // As `kernel/src/soak.rs` spells them.
    const STARTED: &str = "soak-test: started";
    const FAILED: &str = "soak-test: FAILED";
    const WATCHDOG: &str = "soak-test-watchdog:";

    // A healthy run's deadline is the window after arming plus the boot before it; two minutes
    // covers a TCG boot to the soak with a wide margin. A wedge run's window is already the whole
    // run.
    let deadline = if wedge {
        window
    } else {
        window + Duration::from_secs(120)
    };
    let rx = console_lines(stdout);
    let begun = Instant::now();
    let mut starts = 0u32;
    let mut armed_at: Option<Instant> = None;
    let mut counting = false;
    let mut wedged_at: Option<Instant> = None;
    loop {
        // The healthy run's pass condition is a quiet one: the window ran out and nothing bad
        // happened. So it is checked before waiting, not only when a line arrives.
        if !wedge
            && let Some(at) = armed_at
            && at.elapsed() >= window
        {
            if !counting {
                eprintln!(
                    "soak-test: FAIL, the watchdog armed but no pet ever found its count below the \
                     reload value: it was never counting, so surviving the window proves nothing"
                );
                return ExitCode::from(1);
            }
            eprintln!();
            eprintln!(
                "soak-test: PASS, the watchdog was armed and counting for {}s and never reset this \
                 healthy soak ({} boot)",
                window.as_secs(),
                starts
            );
            return ExitCode::SUCCESS;
        }
        let wait_until = match (wedge, armed_at) {
            (false, Some(at)) => (at + window).min(begun + deadline),
            _ => begun + deadline,
        };
        let Some(left) = wait_until.checked_duration_since(Instant::now()) else {
            eprintln!(
                "soak-test: FAIL, {}s passed first (starts={starts}, armed: {}, wedged: {})",
                deadline.as_secs(),
                armed_at.is_some(),
                wedged_at.is_some()
            );
            return ExitCode::from(2);
        };
        let line = match rx.recv_timeout(left) {
            Ok(line) => line,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                eprintln!(
                    "soak-test: FAIL, QEMU's output ended (starts={starts}, wedged: {}); a QEMU \
                     that exits on a reset has not rebooted anything",
                    wedged_at.is_some()
                );
                return ExitCode::from(3);
            }
        };
        let _ = writeln!(sink, "{line}");
        if line.contains("[PANIC]") || line.contains(FAILED) {
            eprintln!("soak-test: FAIL, the soak itself failed: {line}");
            return ExitCode::from(1);
        }
        if line.contains(STARTED) {
            starts += 1;
            if starts >= 2 && wedged_at.is_none() {
                eprintln!(
                    "soak-test: FAIL, the machine reset and booted again without being wedged: a \
                     healthy soak was reset"
                );
                return ExitCode::from(1);
            }
        }
        let Some(rest) = line.split_once(WATCHDOG).map(|(_, rest)| rest.trim()) else {
            continue;
        };
        if rest.starts_with("NOT ARMED") {
            eprintln!("soak-test: FAIL, the watchdog did not arm: {rest}");
            return ExitCode::from(1);
        }
        if rest.starts_with("ARMED.") && armed_at.is_none() {
            armed_at = Some(Instant::now());
        }
        if let Some(counts) = rest
            .split_once("the count had ")
            .and_then(|(_, tail)| tail.split_once(" ticks"))
            .map(|(counts, _)| counts)
            && let Some((left, reload)) = counts.split_once(" of ")
            && let (Ok(left), Ok(reload)) = (left.parse::<u32>(), reload.parse::<u32>())
            && left < reload
        {
            counting = true;
        }
        if rest.starts_with("WEDGING NOW") {
            wedged_at = Some(Instant::now());
        }
        if !wedge || starts < 2 {
            continue;
        }
        if rest.starts_with("the previous boot was ended by this watchdog") {
            eprintln!();
            eprintln!(
                "soak-test: PASS, the wedged kernel was reset by the TCO watchdog and booted \
                 again, {}s after the wedge; the second boot read SECOND_TO_STS set",
                wedged_at.map_or(0, |at| at.elapsed().as_secs())
            );
            return ExitCode::SUCCESS;
        }
        if rest.starts_with("the previous boot was not ended by this watchdog") {
            eprintln!(
                "soak-test: FAIL, the machine came back but SECOND_TO_STS was clear: something \
                 other than the watchdog reset it"
            );
            return ExitCode::from(1);
        }
    }
}
