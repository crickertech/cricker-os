//! Running host commands, and finding what the build put where.
//!
//! The few helpers every command needs: `cargo`, a bare `run`, a captured `run`, the LLVM
//! tools out of the toolchain's sysroot, and the paths to the workspace and its binaries.

use std::process::Command;

use crate::archive::initrd_path;
use crate::disk::disk_path;
use crate::{TARGET, profile_dir};

/// The ELF path of a named binary the `user` package builds (milestone 19f.2+): `hello`, `least_authority_demo`,
/// `console`, and so on. `initrd_aarch64` packs each into the archive under that same name (milestone
/// 266 retired the one exception, which packed `hello` under the entry `init` on aarch64).
///
/// **The path is ABSOLUTE, and that is not fussiness.** Cargo runs the runner script with the
/// working directory set to the **package** dir for `cargo test` and the workspace root for
/// `cargo run`. A relative path therefore resolved under `cargo run` and silently did not under
/// `cargo test`, so the tests booted with no initrd at all and the one that noticed was the one
/// that panicked.
///
/// That lesson was written on a `user_elf()` helper that computed this same path for `hello`
/// alone, and was `bin_elf("hello")` in every respect but the comment. Milestone 130 folded it in
/// when `initrd_aarch64` stopped needing a special case for `init`; the warning belongs here, where
/// every caller reads it, rather than on the one caller that happened to earn it.
pub(crate) fn bin_elf(name: &str) -> String {
    workspace_root()
        .join(format!("target/{TARGET}/{}/{name}", profile_dir()))
        .display()
        .to_string()
}

/// The repo root, from the *compile-time* location of this crate, so it does not depend on
/// whatever directory cargo happens to hand us.
pub(crate) fn workspace_root() -> std::path::PathBuf {
    // Runtime, not env!: the compile-time form bakes the absolute path into the binary, and a
    // cached xtask built before the checkout moved (the 2026-08-15 cricker-os -> nife rename)
    // then aims every path it computes, the farm, the initrds, the images, at a directory that
    // no longer exists. Cargo sets the variable at run time for every cargo-invoked binary, and
    // that one is always the live path. The render test in crates/documentation had the same bug the
    // same day; if a third place grows this pattern, it is worth a lint.
    let manifest =
        std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR for xtask");
    std::path::Path::new(&manifest)
        .parent()
        .expect("xtask has no parent directory")
        .to_path_buf()
}

/// The value of a `--name value` or `--name=value` flag on our own command line, if it is there.
///
/// Both spellings, because a caller who has typed `--cpu=sifive-u54` once should not have to learn
/// that this particular tool only accepts one of them. Returns `None` when the flag is absent and
/// when it is present with nothing after it, which the callers treat as "not given"; a flag whose
/// value went missing is a typo, and defaulting is friendlier than a panic in a build tool.
pub(crate) fn flag_value(name: &str) -> Option<String> {
    let mut args = std::env::args();
    let eq = format!("{name}=");
    while let Some(a) = args.next() {
        if a == name {
            return args.next();
        }
        if let Some(v) = a.strip_prefix(&eq) {
            return Some(v.to_string());
        }
    }
    None
}

pub(crate) fn llvm_tool(name: &str) -> Option<String> {
    let sysroot = capture("rustc", &["--print", "sysroot"])?;
    let verbose = capture("rustc", &["-vV"])?;
    let host = verbose
        .lines()
        .find_map(|l| l.strip_prefix("host: "))?
        .trim();

    let path = format!("{}/lib/rustlib/{host}/bin/{name}", sysroot.trim());
    if std::path::Path::new(&path).exists() {
        Some(path)
    } else {
        eprintln!("cannot find {name} at {path}");
        eprintln!("the llvm-tools rustup component should provide it (see rust-toolchain.toml)");
        None
    }
}

pub(crate) fn capture(program: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(program).args(args).output().ok()?;
    String::from_utf8(out.stdout).ok()
}

pub(crate) fn kernel_elf() -> String {
    format!("target/{TARGET}/{}/kernel", profile_dir())
}

pub(crate) fn cargo(args: &[&str]) -> bool {
    runner_env();
    run("cargo", args)
}

/// The environment [`cargo`] gives every runner, set without running anything: for a caller that
/// spawns the child itself, such as `time_record::cargo_test` timing a test image.
pub(crate) fn runner_env() {
    // The runner needs to know where the initrd is. Set it for every cargo invocation; the
    // script ignores it when the file is not there (which is any build before `user` exists).
    // SAFETY: `set_var`/`remove_var` became unsafe in edition 2024 because they race other
    // threads. xtask is single-threaded here: this runs on the main thread before the child
    // that reads it is spawned, and the only thread xtask ever starts (the transcript reader
    // in swish_check_leg) copies pipe bytes into a String and never touches the environment.
    unsafe { std::env::set_var("NIFE_INITRD", initrd_path()) };
    // SAFETY: `set_var`/`remove_var` became unsafe in edition 2024 because they race other
    // threads. xtask is single-threaded here: this runs on the main thread before the child
    // that reads it is spawned, and the only thread xtask ever starts (the transcript reader
    // in swish_check_leg) copies pipe bytes into a String and never touches the environment.
    unsafe { std::env::set_var("NIFE_DISK", disk_path()) };
    // Attach a virtio-net NIC too (milestone 30): slirp needs no host file, so it is always on for
    // tests, and the net driver's DHCP round-trip test exercises it.
    // SAFETY: `set_var`/`remove_var` became unsafe in edition 2024 because they race other
    // threads. xtask is single-threaded here: this runs on the main thread before the child
    // that reads it is spawned, and the only thread xtask ever starts (the transcript reader
    // in swish_check_leg) copies pipe bytes into a String and never touches the environment.
    unsafe { std::env::set_var("NIFE_NET", "1") };
}

pub(crate) fn run(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .status()
        .map(|s| s.success())
        .unwrap_or_else(|e| {
            eprintln!("failed to run {program}: {e}");
            false
        })
}

/// How many tests a transcript line says the harness selected: `K` in `running K of N tests`,
/// which `kernel/src/testing.rs`'s runner prints once per image under a filter.
pub(crate) fn selected_by(line: &str) -> Option<usize> {
    let rest = line.trim_start().strip_prefix("running ")?;
    let (count, rest) = rest.split_once(' ')?;
    rest.starts_with("of ").then(|| count.parse().ok())?
}

/// [`run`] for `cargo test` under `--test`: the child's stdout passes through line by line, and the
/// return is whether it succeeded and how many tests its harness selected (milestone 609 (the
/// system tests leave the kernel crate)). A leg boots two test images now and a filter may match
/// in only one, so the "selected nothing" verdict is the caller's, over both.
pub(crate) fn cargo_test_counting_selected(args: &[&str]) -> (bool, usize) {
    use std::io::BufRead;
    let mut child = match Command::new("cargo")
        .args(args)
        .stdout(std::process::Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("failed to run cargo: {e}");
            return (false, 0);
        }
    };
    let mut selected = 0;
    if let Some(stdout) = child.stdout.take() {
        for line in std::io::BufReader::new(stdout).lines() {
            let Ok(line) = line else { break };
            println!("{line}");
            selected += selected_by(&line).unwrap_or(0);
        }
    }
    let ok = child.wait().is_ok_and(|s| s.success());
    (ok, selected)
}

#[cfg(test)]
mod selected_by_tests {
    use super::selected_by;

    /// The filtered runner's line is the only one that counts, and it counts as its first number.
    /// An unfiltered run prints `running N tests` with no `of`, and a leg that summed that as a
    /// selection would call a whole suite "one test selected" (milestone 609 (the system tests
    /// leave the kernel crate)).
    #[test]
    fn only_the_filtered_runner_line_is_a_selection() {
        assert_eq!(selected_by("running 1 of 363 tests (filter: bss)"), Some(1));
        assert_eq!(
            selected_by("running 0 of 205 tests (filter: surveys)"),
            Some(0)
        );
        assert_eq!(selected_by("running 363 tests"), None);
        assert_eq!(
            selected_by("  cycles      : PMCCNTR_EL0 running on 4 of 4 cores"),
            None
        );
    }
}
