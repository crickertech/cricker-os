#!/usr/bin/env bash
# Build pinned_tls_exerciser for the three nife targets, where xtask's archive build looks for it:
# target/pinned-tls-exerciser/<triple>/pinned_tls_exerciser. Milestone 501 (a TLS client that
# speaks to one pinned peer).
#
# helpers/build-cryptography-exerciser.sh's posture and shape, for its reason: this fetches `rustls`
# and the provider's RustCrypto primitives from crates.io, which no gate does yet, so the program
# rides in the archive only when somebody ran this, and system_tests/src/user/pinned_tls_tests.rs
# skips otherwise. `cargo xtask std-src` first, because it builds the `std` farm `-Zbuild-std`
# compiles against; that also relinks the machine-wide `nife-dev` toolchain (notes/std.md).
#
#     helpers/build-pinned-tls-exerciser.sh
#     NIFE_CRYPTO_TRIPLES=x86_64-unknown-nife helpers/build-pinned-tls-exerciser.sh
#
# Name: provisional 2026-10-06 (UTC), the sibling of build-cryptography-exerciser.sh.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="$ROOT/pinned_tls_exerciser"
OUT="$ROOT/target/pinned-tls-exerciser"

(cd "$ROOT" && cargo xtask std-src)

for TRIPLE in ${NIFE_CRYPTO_TRIPLES:-aarch64-unknown-nife riscv64-unknown-nife x86_64-unknown-nife}; do
  (
    cd "$SRC"
    RUSTUP_TOOLCHAIN="$ROOT/target/nife-farm" cargo build --release \
      -Zjson-target-spec \
      -Zbuild-std=core,alloc,std,panic_abort \
      -Zbuild-std-features=compiler-builtins-mem \
      --target "$ROOT/targets/$TRIPLE.json"
  )
  mkdir -p "$OUT/$TRIPLE"
  cp "$SRC/target/$TRIPLE/release/pinned_tls_exerciser" "$OUT/$TRIPLE/pinned_tls_exerciser"
  echo "build-pinned-tls-exerciser: $OUT/$TRIPLE/pinned_tls_exerciser ($(wc -c <"$OUT/$TRIPLE/pinned_tls_exerciser") bytes)"
done
