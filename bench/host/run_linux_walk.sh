#!/bin/sh
# Same-machine Linux run of the walk milestone 606 (a directory walk costs what it does on Linux)
# prices: `walk_pricing`, which nife's `script/bench --real --release --smp` times as `fs_walk`.
#
# It boots the SAME Alpine kernel run_linux.sh and run_linux_fs.sh use, on the SAME
# `virt,accel=hvf` machine with the SAME `-cpu host`, `-m 256M` and `-smp 4` as nife's bench boot.
# PID 1 is `walk_pricing`'s `host` example, built static for musl: it stages the priced tree and
# walks it with the very function `std_exerciser` runs on nife, then exits (and `-no-reboot` turns
# the kernel's panic at a dead init into QEMU exiting).
#
# **The tree is on the initramfs, which is tmpfs, not ext4 on virtio.** That is the one way this
# is not matched, and it is small for the figure it produces: every timed walk is warm, and a warm
# walk on Linux is answered from the dentry cache and the page cache whichever filesystem is under
# them, never from the device. A cold ext4 walk is a different number and wants run_linux_fs.sh's
# podman-made image; see notes/walk-pricing.md.
#
# Needs: rustup target add aarch64-unknown-linux-musl; qemu-system-aarch64; network once, for the
# kernel.
#
# Run: sh bench/host/run_linux_walk.sh
set -e
HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$HERE/../.." && pwd)
WORK=${WORK:-$ROOT/target/walkbench}
mkdir -p "$WORK"

KERNEL="$WORK/vmlinuz-virt"
if [ ! -f "$KERNEL" ]; then
    echo "fetching an aarch64 Linux kernel..."
    curl -sSL -o "$KERNEL" \
        "https://dl-cdn.alpinelinux.org/alpine/v3.20/releases/aarch64/netboot/vmlinuz-virt"
fi

# rust-lld and the self-contained musl objects, as run_linux_fs.sh links: macOS's `cc` cannot link
# a Linux binary.
CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=rust-lld \
CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_RUSTFLAGS="-C link-self-contained=yes -C target-feature=+crt-static" \
    cargo build -q --release --manifest-path "$ROOT/Cargo.toml" -p walk_pricing --example host \
    --target aarch64-unknown-linux-musl

rm -rf "$WORK/iroot" && mkdir "$WORK/iroot"
cp "$ROOT/target/aarch64-unknown-linux-musl/release/examples/host" "$WORK/iroot/init"
( cd "$WORK/iroot" && find . | cpio -o -H newc 2>/dev/null ) > "$WORK/initramfs.cpio"

# `helpers/qemu-bounded.sh`, never `timeout(1)` (macOS has none) or `perl -e alarm` (QEMU swallows
# SIGALRM). CLAUDE.md has the rule.
"$ROOT/helpers/qemu-bounded.sh" 120 \
    qemu-system-aarch64 -M virt,accel=hvf,gic-version=3 -cpu host -m 256M -smp 4 -no-reboot \
    -kernel "$KERNEL" -initrd "$WORK/initramfs.cpio" \
    -append "console=ttyAMA0 rdinit=/init panic=-1 quiet loglevel=0" \
    -display none -serial stdio 2>&1 | grep '^walk'
