#!/bin/sh
# Does a Rust TUF client build for nife's targets? Cross-compile only. Name provisional.
#
#   design/roadmap/0858-lab-machines-update-themselves/tuf-probe.sh nife|host [probe...]
#
# The recipe is notes/filesystem-crates-2026-10-07/probe.sh's (PR #1803), itself script/crypto-probes':
# each probe is generated OUTSIDE the repository, so rust-toolchain.toml cannot beat
# RUSTUP_TOOLCHAIN, and built release with panic = "abort" for targets/*-unknown-nife.json with the
# patched farm. FARM defaults to this checkout's target/nife-farm; `cargo xtask std-src` makes it.
# Pointing FARM at another checkout's farm avoids retaking the account-wide nife-dev link.
#
# A probe's main verifies nothing real: it names a type and a call, so the linker has to resolve
# the crate and its crypto. Versions are pinned so a re-run measures the same code.
#
# BUGS: build and link only; nothing runs under nife. Needs the network.
set -eu
HERE=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
ROOT=$(git -C "$HERE" rev-parse --show-toplevel)
FARM="${FARM:-$ROOT/target/nife-farm}"
WORK="${WORK:-${TMPDIR:-/tmp}/nife-tuf-probes}"
MODE=${1:?usage: tuf-probe.sh nife|host [probe...]}
shift

deps() {
    case "$1" in
    tough) echo 'tough = "=0.24.0"'; echo 'serde_json = "1"' ;;
    tuf-crates) echo 'tuf = "=0.3.0-beta9"'; echo 'serde_json = "1"' ;;
    tuf-main) echo 'tuf = { git = "https://github.com/theupdateframework/rust-tuf", rev = "REV" }'; echo 'serde_json = "1"' ;;
    *) return 1 ;;
    esac
}

main_rs() {
    case "$1" in
    tough) cat <<'RS'
fn main() {
    let r: Result<tough::schema::Signed<tough::schema::Root>, _> = serde_json::from_slice(b"{}");
    println!("{}", r.is_ok());
}
RS
    ;;
    tuf-*) cat <<'RS'
fn main() {
    let k = tuf::crypto::PublicKey::from_ed25519(vec![0u8; 32]);
    println!("{}", k.is_ok());
}
RS
    ;;
    esac
}

want="$*"
[ -n "$want" ] || want='tough tuf-crates tuf-main'
mkdir -p "$WORK"
for p in $want; do
    dir="$WORK/$p"
    rm -rf "$dir" && mkdir -p "$dir/src"
    { printf '[package]\nname = "probe"\nversion = "0.1.0"\nedition = "2021"\n[workspace]\n[dependencies]\n'
      deps "$p" | sed "s/REV/${TUF_REV:-219ca7d05818d5dd44ec4e32ab47c5ce64a7bf44}/"
      printf '[profile.release]\npanic = "abort"\n'; } >"$dir/Cargo.toml"
    main_rs "$p" >"$dir/src/main.rs"
    if [ "$MODE" = host ]; then
        log="$dir/build-host.log"
        if (cd "$dir" && CARGO_TARGET_DIR="$WORK/.target-host" cargo +nightly-2026-10-06 build --release) >"$log" 2>&1
        then echo "host $p PASS"; else echo "host $p FAIL $(grep -m1 -E '^error(\[|:)' "$log" || true)"; fi
        continue
    fi
    for a in aarch64 riscv64 x86_64; do
        log="$dir/build-$a.log"
        if (cd "$dir" && RUSTUP_TOOLCHAIN="$FARM" CARGO_TARGET_DIR="$WORK/.target-$a" \
            cargo build --release -Zjson-target-spec \
            -Zbuild-std=core,alloc,std,panic_abort -Zbuild-std-features=compiler-builtins-mem \
            --target "$ROOT/targets/$a-unknown-nife.json") >"$log" 2>&1
        then echo "$a $p PASS"
        else
            crate=$(grep -m1 -oE 'compile `[^`]+`|failed to run custom build command for `[^`]+`' "$log" || true)
            echo "$a $p FAIL $crate | $(grep -m1 -E '^error(\[|:)' "$log" || true)"
        fi
    done
done
