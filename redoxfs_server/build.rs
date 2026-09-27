//! The FS-server EL0 binary is an ordinary nife ELF, so it links against the SAME linker
//! script `components`, `fixtures` and `std_exerciser` use, `crates/user_mode_runtime/link.ld` (linked at
//! `address_space_map::IMAGE_BASE`, explicit W^X PHDRS). The
//! link args are scoped to the two EL0 bins (`rustc-link-arg-bin`), never the lib or the host
//! test binary, which are plain host artifacts and must link the host way.
//!
//! **`mkfs` was not in that scope until 2026-09-26**, and nothing noticed: it linked with lld's own
//! default layout, at `0x20_0000`, with whatever program headers lld chose. That worked only because
//! nothing else happened to be mapped there. Milestone 206 (a program image has under 896 KiB) made
//! the loader refuse an image outside the address-space map's image band, and the first CI run
//! refused `mkfs` by name. Every EL0 bin this package builds is listed below.

fn main() {
    let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("cargo::rerun-if-changed=../crates/user_mode_runtime/link.ld");
    for bin in ["redoxfs_server", "mkfs"] {
        println!("cargo::rustc-link-arg-bin={bin}=-T{dir}/../crates/user_mode_runtime/link.ld");
        // `_start` lives in each bin, but forcing it undefined is the same belt-and-braces
        // std_exerciser uses so the entry survives whatever the linker's ENTRY timing.
        println!("cargo::rustc-link-arg-bin={bin}=-u_start");
        // Keep the ELF an ELF: the kernel's loader wants program headers, not a build-id note.
        println!("cargo::rustc-link-arg-bin={bin}=--build-id=none");
    }
}
