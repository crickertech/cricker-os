//! An EL0 component gets the userspace linker script, and it must NOT be the kernel's.
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

use std::path::Path;

fn main() {
    link_el0(Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap()));
}

/// Point the linker at the shared EL0 script and keep the output an ELF.
///
/// Duplicated, four lines of it, in `fixtures/build.rs`: a build script is per-package and there is
/// nowhere below this that two packages can share code without a third crate existing to hold it.
/// The thing that would actually hurt to duplicate, the layout, is the one file both call.
fn link_el0(manifest_dir: &Path) {
    let script = manifest_dir.join("../crates/user_mode_runtime/link.ld");
    let script = script.display();
    println!("cargo::rerun-if-changed={script}");
    println!("cargo::rustc-link-arg=-T{script}");

    // Keep the ELF an ELF. The kernel's loader wants program headers, not a flat blob.
    println!("cargo::rustc-link-arg=--build-id=none");
}
