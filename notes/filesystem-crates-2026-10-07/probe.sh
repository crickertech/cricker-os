#!/bin/sh
# Filesystem-crate probes for notes/filesystem-crates-2026-10-07.md. Name provisional.
#
#   notes/filesystem-crates-2026-10-07/probe.sh nife [probe...]   # build for all three nife targets
#   notes/filesystem-crates-2026-10-07/probe.sh nife-tree [probe...]  # the same, configured as this tree builds
#   notes/filesystem-crates-2026-10-07/probe.sh host [probe...]   # build for the host only
#
# `nife` is script/crypto-probes' recipe unchanged: each probe is generated OUTSIDE this repository
# (so the repository's rust-toolchain.toml cannot beat RUSTUP_TOOLCHAIN for the rustc build-std
# runs; see that script's header), built with the patched farm at target/nife-farm for
# targets/{aarch64,riscv64,x86_64}-unknown-nife.json, release, panic = "abort". Run
# `cargo xtask std-src` first. A probe is a [[bin]] whose main calls the crate, so the linker is
# asked to resolve it. `host` builds the same sources for the Mac, and run-host.sh runs them
# against images. Versions are pinned with `=` so a re-run measures the same code.
#
# `nife-tree` is the configuration a program in this tree would actually be built with, and each
# addition is one this tree already carries for another crate: `entropy_backend` with
# `--cfg getrandom_backend="custom"` (script/crate-probes), the `--soft` recipe from
# script/crypto-probes (`sha2`'s `force-soft` and its sibling `--cfg`s, which x86_64-unknown-nife
# needs because it has no SSE), and two one-line patches: rust-fs-btrfs's C ABI module (`capi`) is
# gated to unix and windows, the platforms its own dependency gates `FileDevice` to, and
# btrfs-transaction's single zstd call (compress-on-write) is removed with the C library it pulls.
#
# BUGS: build and link only on the nife side; nothing here runs a probe under nife. Needs the
# network. Takes the account-wide nife-dev link through `cargo xtask std-src`.
set -eu
HERE=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
ROOT=$(CDPATH='' cd -- "$HERE/../.." && pwd)
WORK="${WORK:-${TMPDIR:-/tmp}/nife-fs-probes}"
MODE=${1:?usage: probe.sh nife|host [probe...]}
shift

all='lambutter rust-fs-btrfs btrfs-core btrfs-fs btrfs-transaction ferrosys fatfs fatfs-nochrono lamfat embedded-sdmmc hadris-fat ext4-view ext4_rs'

deps() {
    case "$1" in
    lambutter) echo 'lambutter = "=0.3.1"' ;;
    rust-fs-btrfs) echo 'rust-fs-btrfs = "=0.10.2"'; echo 'rust-fs-core = "=0.3.7"' ;;
    btrfs-core) echo 'btrfs-core = { version = "=0.1.5", default-features = false }' ;;
    btrfs-fs) echo 'btrfs-fs = "=0.13.0"'; echo 'tokio = { version = "1", features = ["rt"] }' ;;
    btrfs-transaction) echo 'btrfs-transaction = "=0.13.0"'; echo 'btrfs-disk = "=0.13.0"' ;;
    ferrosys) echo 'ferrosys = { version = "=0.6.0", default-features = false, features = ["btrfs", "fat", "exfat", "ext"] }' ;;
    fatfs) echo 'fatfs = "=0.3.6"' ;;
    fatfs-nochrono) echo 'fatfs = { version = "=0.3.6", default-features = false, features = ["std", "alloc"] }' ;;
    lamfat) echo 'lamfat = "=0.4.2"' ;;
    embedded-sdmmc) echo 'embedded-sdmmc = { version = "=0.10.0", default-features = false }' ;;
    hadris-fat)
        echo 'hadris-fat = { version = "=3.0.0-rc.1", features = ["write", "std"] }'
        echo 'hadris-fs = "=3.0.0-rc.1"'
        echo 'hadris-storage = "=3.0.0-rc.1"' ;;
    ext4-view) echo 'ext4-view = "=1.0.0"' ;;
    ext4_rs) echo 'ext4_rs = "=1.3.3"' ;;
    *) return 1 ;;
    esac
}

src() {
    case "$1" in
    fatfs-nochrono) echo fatfs ;;
    *) echo "$1" ;;
    esac
}

write_probe() {
    dir="$WORK/$1"
    rm -rf "$dir"
    mkdir -p "$dir/src"
    {
        printf '[package]\nname = "probe"\nversion = "0.1.0"\nedition = "2021"\n[workspace]\n[dependencies]\n'
        deps "$1"
        if [ "$MODE" = nife-tree ]; then
            echo "entropy_backend = { path = \"$ROOT/entropy_backend\" }"
            echo 'sha2 = { version = "0.10", default-features = false, features = ["force-soft"] }'
            if [ "$1" = rust-fs-btrfs ]; then
                patched="$WORK/.patched/rust-fs-btrfs"
                rm -rf "$patched" && mkdir -p "$WORK/.patched"
                cp -R "$(ls -d "${CARGO_HOME:-$HOME/.cargo}"/registry/src/*/rust-fs-btrfs-0.10.2 | head -1)" "$patched"
                perl -pi -e 's/^pub mod capi;/#[cfg(any(unix, windows))] pub mod capi;/' "$patched/src/lib.rs"
                printf '[patch.crates-io]\nrust-fs-btrfs = { path = "%s" }\n' "$patched"
            fi
            if [ "$1" = btrfs-transaction ]; then
                # Its one zstd call is the optional compress-on-write path, which already returns
                # None ("store it uncompressed") on failure, so this answers whether anything
                # else stands in the way.
                patched="$WORK/.patched/btrfs-transaction"
                rm -rf "$patched" && mkdir -p "$WORK/.patched"
                cp -R "$(ls -d "${CARGO_HOME:-$HOME/.cargo}"/registry/src/*/btrfs-transaction-0.13.0 | head -1)" "$patched"
                perl -0pi -e 's/\[dependencies\.zstd\]\nversion = "0\.13"\n//' "$patched/Cargo.toml"
                perl -pi -e 's/CompressionType::Zstd => zstd::bulk::compress\(data, 3\)\.ok\(\)\?,/CompressionType::Zstd => return None,/' "$patched/src/transaction.rs"
                printf '[patch.crates-io]\nbtrfs-transaction = { path = "%s" }\n' "$patched"
            fi
        fi
        printf '[profile.release]\npanic = "abort"\n'
    } >"$dir/Cargo.toml"
    if [ "$MODE" = nife-tree ]; then
        mkdir -p "$dir/.cargo"
        printf '[build]\nrustflags = ["--cfg", "getrandom_backend=\\"custom\\"", "--cfg", "polyval_force_soft", "--cfg", "poly1305_force_soft", "--cfg", "aes_force_soft"]\n' >"$dir/.cargo/config.toml"
        { echo 'use entropy_backend as _;'; cat "$HERE/probes/$(src "$1").rs"; } >"$dir/src/main.rs"
        return
    fi
    cp "$HERE/probes/$(src "$1").rs" "$dir/src/main.rs"
}

want="$*"
[ -n "$want" ] || want=$all
mkdir -p "$WORK"
for p in $want; do
    write_probe "$p"
    if [ "$MODE" = host ]; then
        log="$WORK/$p/build-host.log"
        if (cd "$WORK/$p" && CARGO_TARGET_DIR="$WORK/.target-host" cargo +nightly-2026-10-06 build --release) >"$log" 2>&1; then
            mkdir -p "$WORK/bin" && cp "$WORK/.target-host/release/probe" "$WORK/bin/$p"
            echo "host  $p PASS"
        else
            echo "host  $p FAIL $(grep -m1 -E '^error(\[|:)' "$log" || true)"
        fi
        continue
    fi
    for a in aarch64 riscv64 x86_64; do
        log="$WORK/$p/build-$a.log"
        if (cd "$WORK/$p" && RUSTUP_TOOLCHAIN="$ROOT/target/nife-farm" CARGO_TARGET_DIR="$WORK/.target-$a" \
            cargo build --release -Zjson-target-spec \
            -Zbuild-std=core,alloc,std,panic_abort -Zbuild-std-features=compiler-builtins-mem \
            --target "$ROOT/targets/$a-unknown-nife.json") >"$log" 2>&1; then
            echo "$a $p PASS"
        else
            crate=$(grep -m1 -oE 'compile `[^`]+`|failed to run custom build command for `[^`]+`' "$log" || true)
            echo "$a $p FAIL $crate | $(grep -m1 -E '^error(\[|:)' "$log" || true)"
        fi
    done
done
