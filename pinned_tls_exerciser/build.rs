//! The same linker contract as `std_exerciser` and `cryptography_exerciser`: the shared
//! `crates/user_mode_runtime/link.ld`, as is.

use std::env;
use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let shared = manifest.join("../crates/user_mode_runtime/link.ld");
    println!("cargo::rerun-if-changed=../crates/user_mode_runtime/link.ld");

    println!("cargo::rustc-link-arg=-T{}", shared.display());
    println!("cargo::rustc-link-arg=-u_start");
    println!("cargo::rustc-link-arg=--build-id=none");
}
