//! The system-test image links with the kernel's own linker script, and declares `cfg(initrd)`.
//!
//! `cargo::rustc-link-arg` is per package: the kernel's `build.rs` passes `-T link-<arch>.ld` for
//! the kernel's own targets and for nobody who depends on it. This image is the same kernel, so it
//! has to ask for the same layout itself. `cfg(initrd)` is declared the way `kernel/build.rs`
//! declares it (`declare_initrd_cfg`), because the suite's modules are gated on it and a build
//! script's `rustc-cfg` reaches only its own package too.

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let link_script = match arch.as_str() {
        "aarch64" => "link-aarch64.ld",
        "riscv64" => "link-riscv64.ld",
        "x86_64" => "link-x86_64.ld",
        other => panic!("system_tests: no kernel linker script for {other}"),
    };
    println!("cargo::rerun-if-changed=../kernel/{link_script}");
    println!("cargo::rustc-link-arg=-T{manifest_dir}/../kernel/{link_script}");
    println!("cargo::rustc-check-cfg=cfg(initrd)");
    if matches!(arch.as_str(), "aarch64" | "riscv64" | "x86_64") {
        println!("cargo::rustc-cfg=initrd");
    }
}
