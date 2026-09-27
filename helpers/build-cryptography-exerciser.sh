#!/usr/bin/env bash
# Build `cryptography_exerciser` for the three nife custom targets, for milestone 442 (a crypto provider `rustls` can use on all three bare-metal targets).

#
# **This is an experiment's apparatus, not part of the build**, and that is the same posture
# `helpers/build-ripgrep.sh` takes for the same reason. Nothing in `script/test` runs it and no gate
# needs it: `xtask` packs the resulting ELF only if it is already on disk, and
# `kernel/src/user/cryptography_tests.rs` skips when it is not.
#
# DECISIONS §46 (thin primitives or whole subsystems; we write everything in between) is why. The
# program depends on `rustls` and a crypto provider, and while §196 (nife carries TLS: `rustls` for the protocol, and a crypto provider we make work)
# ruled on `rustls`, it ruled
# explicitly **not** on a provider: that is an architect's decision and not a lane's. Keeping the
# build
# here rather than in `script/test` keeps roughly a hundred crates out of this repository's
# `Cargo.lock`, out of `deny.toml`'s reach, and out of CI, until there is a ruling to put them in.
#
# The one thing it does that `build-ripgrep.sh` does not: the package is **ours**, so it carries its
# own `.cargo/config.toml` with the `getrandom` backend selector and the four soft-implementation
# cfgs, and this script adds nothing to the command line beyond the target and build-std. If a
# build fails, the configuration is in the package where a reader will find it.
#
# Usage: helpers/build-cryptography-exerciser.sh
#        NIFE_CRYPTO_TRIPLES="x86_64-unknown-nife" helpers/build-cryptography-exerciser.sh
#
# See notes/cryptography-provider.md for the measurement that chose these crates and what is still open.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="$ROOT/cryptography_exerciser"
OUT="$ROOT/target/cryptography-exerciser"

# The patched std lives in the `nife-dev` toolchain, which `xtask std-src` builds and links.
# `RUSTUP_TOOLCHAIN` rather than `+nife-dev` for the reason `xtask::std_exerciser` records: the
# cargo proxy exports `RUSTUP_TOOLCHAIN=nightly`, which would override a `+` selector. A
# `rust-toolchain.toml` naming `nife-dev` is NOT an alternative here: milestone 442's lane measured
# it getting `aarch64-unknown-nife` wrong on an aarch64 host while the other two stayed right, and
# `script/crypto-probes`' header records the whole trap.
(cd "$ROOT" && cargo xtask std-src)

for TRIPLE in ${NIFE_CRYPTO_TRIPLES:-aarch64-unknown-nife riscv64-unknown-nife x86_64-unknown-nife}; do
  (
    cd "$SRC"
    RUSTUP_TOOLCHAIN=nife-dev cargo build --release \
      -Zjson-target-spec \
      -Zbuild-std=core,alloc,std,panic_abort \
      -Zbuild-std-features=compiler-builtins-mem \
      --target "$ROOT/targets/$TRIPLE.json"
  )
  mkdir -p "$OUT/$TRIPLE"
  cp "$SRC/target/$TRIPLE/release/cryptography_exerciser" "$OUT/$TRIPLE/cryptography_exerciser"
  echo "build-cryptography-exerciser: $OUT/$TRIPLE/cryptography_exerciser ($(wc -c <"$OUT/$TRIPLE/cryptography_exerciser") bytes)"
done
