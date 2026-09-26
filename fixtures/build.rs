//! A fixture gets the same userspace linker script every component does, and it must NOT be the
//! kernel's.
//!
//! `cargo:rustc-link-arg` is per-package, which is the whole reason the EL0 programs are their own
//! packages rather than more binaries in `kernel/`.
//!
//! **The linker script lives in `crates/user_mode_runtime`, not here, and that is deliberate.** Both
//! `components` and `fixtures` link against the identical EL0 layout, so a copy in each would be
//! two places to be wrong about the image base with nothing comparing them; milestone 73 (name the aarch64 files aarch64) refused to
//! rename `user/link.ld` on exactly that ground ("it is genuinely shared"), and milestone 175 split
//! the package that held it. `user_mode_runtime` is the runtime that supplies `_start`'s ABI and the panic
//! handler to every program in both packages, so a program image's layout is its business.
//! `helpers/build-ripgrep.sh` derives its high-load variant from the same file by substitution.
//!
//! Since milestone 36 this also compiles the **C** half of the `c_shim` program (`c/c_seam.c`) with
//! a bare-metal clang and hands the object to the linker for that one binary only. Both C-linked
//! programs are fixtures (milestone 36's throwaway seam and milestone 23's replacement component),
//! which is why the C lives on this side of milestone 175's split and `components/build.rs` is the
//! short one. See [`compile_c_component`] and notes/c-seam.md.

use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let script = Path::new(&dir).join("../crates/user_mode_runtime/link.ld");
    let script = script.display();
    println!("cargo::rerun-if-changed={script}");
    println!("cargo::rustc-link-arg=-T{script}");

    // Keep the ELF an ELF. The kernel's loader wants program headers, not a flat blob.
    println!("cargo::rustc-link-arg=--build-id=none");

    compile_c_component(Path::new(&dir));
}

/// The C sources, and the one binary each is linked into (`(source, binary)`). One translation unit
/// per object, no archive: `rustc-link-arg-bin` puts the object straight on the linker's command
/// line for that binary, so no `ar` is involved and nothing else in the package sees it.
///
/// - `c/c_seam.c` -> `c_shim`: milestone 36's throwaway component, the seam itself.
/// - `c/c_swappable.c` -> `c_swappable`: milestone 23's replacement component. The reason the
///   hot-swap claim is about a *component* and not about a recompile.
const C_SOURCES: &[(&str, &str)] = &[("c/c_seam.c", "c_shim"), ("c/c_swappable.c", "c_swappable")];

/// **Compile the foreign component and give it to `c_shim`'s linker, or fail with instructions.**
///
/// Cost accepted deliberately: from milestone 36 on, building `user` needs a C compiler, so a fresh
/// clone runs `script/bootstrap` (which installs one) before `cargo build` works. The roadmap
/// already priced this in for Zig at milestone 29; paying it here, with a throwaway component, is
/// the whole point of doing the seam first.
fn compile_c_component(manifest_dir: &Path) {
    for (source, _) in C_SOURCES {
        println!("cargo::rerun-if-changed={source}");
    }
    println!("cargo::rerun-if-env-changed=NIFE_CC");

    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let flags = match arch.as_str() {
        // `-mgeneral-regs-only` is not a size optimization, it is an ABI requirement: the Rust
        // target is `aarch64-unknown-none-softfloat`, which uses no FP/SIMD registers, and the
        // kernel does not enable the FP unit for EL0. A clang that felt free to vectorize a byte
        // loop into `q` registers would trap on the first one.
        "aarch64" => vec!["--target=aarch64-unknown-none-elf", "-mgeneral-regs-only"],
        // Match `riscv64imac-unknown-none-elf` exactly: no F/D extension, so the soft-float `lp64`
        // ABI, not `lp64d`. A mismatch here is an ABI mismatch, and lld would either refuse the
        // object or (worse) link it and pass arguments in registers the other side never reads.
        "riscv64" => vec![
            "--target=riscv64-unknown-none-elf",
            "-march=rv64imac",
            "-mabi=lp64",
        ],
        // Match `x86_64-unknown-none` exactly (milestone 161). That Rust target sets
        // `features = "-mmx,-sse,+soft-float"` and `disable_redzone = true`, and both halves of
        // that have to be said again here or the object disagrees with every Rust object it is
        // linked beside.
        //
        // **`-mno-sse` is the aarch64 arm's `-mgeneral-regs-only` argument, one ISA over**: the
        // kernel does not enable the FP unit for ring 3, so a clang that felt free to vectorize a
        // byte loop into `xmm` registers would take `#UD` (`CR4.OSFXSR` clear) on the first one.
        // `-mno-red-zone` is the one with no counterpart on either other architecture: the red zone
        // is 128 bytes below `rsp` a leaf function may use without adjusting the pointer, and it is
        // safe only if nothing can push there behind the function's back. Ring 3 code is preempted
        // by an interrupt that switches to a *different* stack, so a user program could in fact
        // keep it; it is off anyway because the Rust half has it off, and an ABI that is off by 128
        // bytes on one side of a call is not an ABI.
        "x86_64" => vec![
            "--target=x86_64-unknown-none-elf",
            "-mno-sse",
            "-mno-mmx",
            "-mno-red-zone",
        ],
        // The host passes (`cargo clippy` over the workspace excludes `user`, but a stray host
        // build of this package would land here) have nothing to compile for and no business
        // guessing. Say so rather than emitting a broken link arg.
        other => {
            println!(
                "cargo::warning=fixtures/build.rs: no C component for target arch '{other}'; the \
                 c_shim and c_swappable binaries will not link. Build for aarch64, riscv64 \
                 or x86_64."
            );
            return;
        }
    };

    let Some(clang) = resolve_clang() else {
        // A hard error, not a warning. A silently missing C component would produce a `c_shim`
        // that fails to link with a bare "undefined symbol: c_seam_transform", which tells a
        // reader nothing about what to install.
        panic!("{}", CLANG_HELP);
    };

    for (source, bin) in C_SOURCES {
        compile_one(manifest_dir, &clang, &flags, source, bin);
    }
}

/// Compile one translation unit and put its object on one binary's linker command line.
fn compile_one(manifest_dir: &Path, clang: &Path, flags: &[&str], source: &str, bin: &str) {
    let stem = Path::new(source).file_stem().unwrap().to_string_lossy();
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join(format!("{stem}.o"));
    let src = manifest_dir.join(source);

    let mut cmd = Command::new(clang);
    cmd.args(flags)
        // Freestanding: no hosted libc is present or implied. clang still supplies its own
        // `stddef.h` / `stdint.h` out of its resource directory, which is all the component uses.
        .arg("-ffreestanding")
        // Match the Rust bare targets' `static` relocation model. A PIC object here would ask for
        // GOT indirection that a statically linked, fixed-address image has no use for.
        .arg("-fno-pic")
        // No stack protector: `__stack_chk_fail` and `__stack_chk_guard` are libc symbols, and
        // tier two of the libc question (design/roadmap/36-foreign-component.md records the three
        // tiers; notes/c-seam.md applies them) is "a handful of symbols we chose", not "whatever
        // the compiler decided to reference". Turning the feature off is honest; shimming its
        // runtime would be pretending we have one.
        .arg("-fno-stack-protector")
        .args(["-Os", "-std=c11", "-Wall", "-Wextra", "-Werror"])
        .arg("-c")
        .arg(&src)
        .arg("-o")
        .arg(&out);

    let status = cmd.status();
    match status {
        Ok(s) if s.success() => {}
        Ok(s) => panic!(
            "fixtures/build.rs: {} failed to compile {} ({s})",
            clang.display(),
            src.display()
        ),
        Err(e) => panic!(
            "fixtures/build.rs: could not run {}: {e}\n{}",
            clang.display(),
            CLANG_HELP
        ),
    }

    // Scoped to the one binary. Every other program in this package links exactly as before, which
    // is what keeps a foreign component from becoming everyone's problem.
    println!("cargo::rustc-link-arg-bin={bin}={}", out.display());
}

/// **Find a clang that can target every one of this project's bare ISAs.**
///
/// The same discipline `xtask`'s `llvm_tool` uses for `llvm-objcopy`: resolve the tool from a known
/// list rather than hoping it is on `PATH` under the right name, and fail loudly with what to
/// install. The extra step here is a *capability* check, because the most likely clang on a macOS
/// dev machine (Apple's, in the Xcode command line tools) is built with the AArch64 and X86
/// backends only and cannot emit RISC-V at all.
///
/// Every backend is required from one compiler even when only one ISA is being built. That is
/// DECISIONS §19 (parity is a gate, not an aspiration) applied to the toolchain: a machine where the
/// architectures would be compiled by different clangs is a machine where "it works on aarch64"
/// stops predicting anything about riscv64, and the failure would surface halfway through a build
/// rather than here.
///
/// **Milestone 161 made this a three-way check and did not change the rejection**, which is the
/// useful thing to notice: adding `x86_64` costs nothing here because every clang that already passed
/// (Homebrew's keg, a distro build) has the X86 backend, and the one that already failed (Apple's)
/// fails on RISC-V exactly as before.
fn resolve_clang() -> Option<PathBuf> {
    // The escape hatch **overrides** the search rather than joining it: "use this compiler" has to
    // mean that, or a typo in the path silently falls back to a different compiler and the build is
    // no longer reproducible in the way the setter thought it was. It is still capability-checked,
    // because a named compiler that cannot emit RISC-V is the same problem by a different route.
    if let Ok(explicit) = std::env::var("NIFE_CC")
        && !explicit.is_empty()
    {
        let cc = PathBuf::from(explicit);
        return has_every_backend(&cc).then_some(cc);
    }
    [
        // Homebrew's `llvm` keg, which is what `script/bootstrap` installs on macOS. It is not
        // symlinked onto `PATH` by design (it would shadow Apple's clang), so name it directly.
        "/opt/homebrew/opt/llvm/bin/clang",
        "/usr/local/opt/llvm/bin/clang",
        // A distro clang (Debian/Ubuntu build every LLVM backend in, so this is the usual CI answer).
        "clang",
    ]
    .into_iter()
    .map(PathBuf::from)
    .find(|c| has_every_backend(c))
}

/// Does this clang have the AArch64, RISC-V *and* X86 backends registered? `-print-targets` answers
/// without compiling anything, and a clang that cannot be run at all simply fails the check.
///
/// Name: provisional (milestone 161 (the `x86_64` kernel port)): it was `has_both_backends` when
/// there were two, and "both" is a word with an arity in it, which is the smallest possible version
/// of a name going stale. calef names functions (AGENTS.md, extended 2026-08-23).
fn has_every_backend(clang: &Path) -> bool {
    let Ok(out) = Command::new(clang).arg("-print-targets").output() else {
        return false;
    };
    if !out.status.success() {
        return false;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let has = |name: &str| {
        text.lines()
            .any(|l| l.split_whitespace().next() == Some(name))
    };
    has("aarch64") && has("riscv64") && has("x86-64")
}

/// What to do about a missing or unsuitable clang. Long on purpose: the failure lands on a fresh
/// clone or a machine whose only clang is Apple's, and a reader deserves to know why the compiler
/// they already have was rejected.
const CLANG_HELP: &str = "\
fixtures/build.rs: no clang found that can target aarch64, riscv64 and x86-64.

nife links a C component into the `c_shim` program (milestone 36, DECISIONS \u{a7}30), so the
build needs a bare-metal clang with ALL THREE of the AArch64, RISC-V and X86 backends. Apple's
clang, in the Xcode command line tools, has no RISC-V backend and is deliberately rejected: two
different compilers for two of the architectures would break the parity gate (DECISIONS \u{a7}19).

  macOS:   brew install llvm      (script/bootstrap does this)
  Debian:  sudo apt-get install clang
  other:   any clang whose `clang -print-targets` lists aarch64, riscv64 and x86-64

Then re-run the build. To point at a specific compiler, set NIFE_CC=/path/to/clang.";
