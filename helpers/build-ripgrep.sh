#!/usr/bin/env bash
# Build **unmodified `ripgrep` from crates.io** for the nife custom target (milestone 121).
#
# This is an experiment's apparatus, not part of the build. Nothing in `script/test` runs it and no
# gate needs it: `xtask initrd-aarch64` packs the resulting ELF only if it is already on disk, and
# `system_tests/src/user/ripgrep_tests.rs` skips when it is not. That is deliberate, and §46 (thin
# primitives or whole subsystems) is the reason: making the gate fetch `ripgrep` and its ~40
# transitive crates would put a crates.io dependency tree in this repository's build, which is
# an architect's call and not a lane's.
#
# The whole point of milestone 121 is that the source is somebody else's and is untouched. There is
# no patch, no vendored copy, and no fork. What differs from a Linux build is entirely on the
# command line below: the target spec, `-Zbuild-std` against the patched `nife-dev` toolchain, and
# the three link arguments `std_exerciser/build.rs` supplies for a program built in-tree (the shared
# linker script, `-u_start`, and no build id).
#
# All three architectures, because DECISIONS §19 makes parity a gate rather than an aspiration: a
# capability ships on every supported target or a scope note records the gap and the plan. x86_64
# joined at milestone 184, which built `x86_64-unknown-nife` and its `std` farm; before that there
# was no `std` on x86_64 and therefore no `ripgrep`.
#
# Usage: helpers/build-ripgrep.sh [version]     (default 14.1.1)
#        NIFE_RIPGREP_TRIPLES="x86_64-unknown-nife" helpers/build-ripgrep.sh   (one target only)
#
# See notes/ripgrep-on-nife.md for what it does and does not do once it is running.
set -euo pipefail

VERSION="${1:-14.1.1}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# The source tree is unpacked OUTSIDE this repository on purpose. `ripgrep` carries no
# `[workspace]` table, so cargo walks up and finds this repo's root manifest, then refuses to
# build a package that "believes it's in a workspace when it's not". Unpacking under `target/`
# hits that too. The alternative (a `workspace.exclude` entry) would put ripgrep in this
# repository's manifest, which is exactly the coupling this experiment is meant not to have.
BUILD="${TMPDIR:-/tmp}/nife-ripgrep"
OUT="$ROOT/target/ripgrep"
SRC="$BUILD/ripgrep-$VERSION"

mkdir -p "$BUILD"
if [ ! -f "$SRC/Cargo.toml" ]; then  # a half-unpacked tree from an interrupted run has no manifest
  echo "build-ripgrep: fetching ripgrep $VERSION from crates.io"
  curl -sSL --max-time 120 -o "$BUILD/ripgrep-$VERSION.crate" \
    "https://static.crates.io/crates/ripgrep/ripgrep-$VERSION.crate"
  tar xzf "$BUILD/ripgrep-$VERSION.crate" -C "$BUILD"
fi

# The patched std lives in the `nife-dev` toolchain, which `xtask std-src` builds and links.
# `RUSTUP_TOOLCHAIN` rather than `+nife-dev` for the reason `xtask::std_exerciser` records: the
# cargo proxy exports `RUSTUP_TOOLCHAIN=nightly`, which would override a `+` selector. And by
# path rather than by name (2026-09-30): `nife-dev` is one symlink for the whole user account,
# so a lane gating beside this build can steal it mid-run and the name then resolves another
# worktree's farm with no diagnostic. `std-src` above has just built "$ROOT/target/nife-farm",
# so the path is the farm this checkout chose.
(cd "$ROOT" && cargo xtask std-src)

# `-Copt-level=s` and `-Cstrip=debuginfo` are not tuning: ripgrep's own release profile sets
# `debug = 1`, which produces a 25 MB ELF the initrd would carry into RAM. Overriding a profile from
# the command line is a build setting, not a change to the program.
# The link script is the shared one, unchanged. **This used to relink at 16 MiB**, by substituting
# `crates/user_mode_runtime/link.ld`'s base, because every program was linked at `0x40_0000` with its
# stack at `0x50_0000`, and ripgrep's 1.37 MiB of `.text` did not fit in the 896 KiB between them.
# Milestone 206 (a program image has under 896 KiB) drew the address-space map (`crates/address_space_map`, DECISIONS §171 (where a program image starts) option D),
# which gives an image 496 MiB at the shared base, so ripgrep links like every other program.
#
# The shared script is also how it keeps a manifest note (milestone 597 (a program carries its
# manifest in an ELF note), provisional): the `PT_NOTE` header and the `.note.nife` section come with
# it. A foreign program carries one by linking an object that holds it (`-Clink-arg=note.o`,
# measured by #1319), and `cargo xtask foreign-note` writes that object (milestone 595 (the shell runs
# a `std` program), 2026-10-07): `grant_plan::UNVOUCHED_STD_MANIFEST`, a `std` program that hears its
# words and reads what they name, read-only. Without it the shell ran `rg` by its path as a native
# program that hears nothing. The source is still untouched; the note is one more link argument.
mkdir -p "$OUT"

for TRIPLE in ${NIFE_RIPGREP_TRIPLES:-aarch64-unknown-nife riscv64-unknown-nife x86_64-unknown-nife}; do
  mkdir -p "$OUT/$TRIPLE"
  (cd "$ROOT" && cargo xtask foreign-note "$TRIPLE" "$OUT/$TRIPLE/note.o")
  cd "$SRC"
  # Overflow checks as every nife release profile carries them (notes/overflow-checks.md); ripgrep's
  # own profile is upstream's, so the setting comes from the environment, and reaches its `std` too.
  CARGO_PROFILE_RELEASE_OVERFLOW_CHECKS=true \
  RUSTUP_TOOLCHAIN="$ROOT/target/nife-farm" \
  RUSTFLAGS="-Clink-arg=-T$ROOT/crates/user_mode_runtime/link.ld -Clink-arg=-u_start -Clink-arg=--build-id=none -Clink-arg=$OUT/$TRIPLE/note.o -Cstrip=debuginfo -Copt-level=s" \
    cargo build --release \
      -Zjson-target-spec \
      -Zbuild-std=core,alloc,std,panic_abort \
      -Zbuild-std-features=compiler-builtins-mem \
      --target "$ROOT/targets/$TRIPLE.json"

  cp "$SRC/target/$TRIPLE/release/rg" "$OUT/$TRIPLE/rg"
  echo "build-ripgrep: $OUT/$TRIPLE/rg ($(wc -c < "$OUT/$TRIPLE/rg") bytes)"
done

# The same source for the host, for the expected answer (milestone 121 (`ripgrep`: enumeration as a
# capability), 2026-10-07). `script/swish-check` types `rg` over a copy of `crates/` at the prompt
# and compares what it prints against what this `rg` prints over the same copy, so the two must be
# one version: a Homebrew `rg` is whatever Homebrew last shipped, and its output need not be
# 14.1.1's. Upstream's own profile and the host's default toolchain, because this one is a
# reference and not a nife program. `NIFE_RIPGREP_HOST=0` skips it.
if [ "${NIFE_RIPGREP_HOST:-1}" != 0 ]; then
  cd "$SRC"
  cargo build --release --target-dir "$SRC/target/host"
  mkdir -p "$OUT/host"
  cp "$SRC/target/host/release/rg" "$OUT/host/rg"
  echo "build-ripgrep: $OUT/host/rg ($("$OUT/host/rg" --version | head -1))"
fi
