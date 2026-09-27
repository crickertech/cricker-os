//! **The host side of milestone 121 (`ripgrep` on nife: enumeration as a capability)'s walk
//! pricing**: stage the fixture tree in a scratch
//! directory and price a walk over it, with the very function nife's `std_exerciser` runs through
//! a granted directory.
//!
//! On macOS this is a reference point, not a competitor: APFS on the real NVMe device with its
//! dentry and page caches, against nife under an emulator or a hypervisor. Build it in release,
//! as every comparison does:
//!
//! ```text
//! cargo run --release -p walk_pricing --example host [DIR]
//! ```
//!
//! `DIR` defaults to a fresh directory under the host's temp dir, and is removed afterwards.
//!
//! # BUGS
//!
//! - **The matched-tier Linux run stages on tmpfs.** `bench/host/run_linux_walk.sh` (milestone
//!   606, a directory walk costs what it does on Linux) builds this example static for musl and
//!   boots it as PID 1 under QEMU-HVF on the machine nife's bench boot uses, so the tree lives on
//!   the initramfs rather than ext4 on virtio. For a warm walk that does not matter; for a cold
//!   one it does, and notes/walk-cost.md says so.

fn main() {
    let dir = std::env::args_os()
        .nth(1)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join(format!("walk-{}", std::process::id())));
    let _ = std::fs::remove_dir_all(&dir);
    walk_pricing::stage(&dir).expect("could not stage the priced tree");
    let price = walk_pricing::price(&dir).expect("the walk failed");
    for line in walk_pricing::report(&price) {
        println!("{line}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}
