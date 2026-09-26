#!/usr/bin/env bash
# Build **unmodified `ripgrep` from crates.io** for the nife custom target (milestone 121).
#
# This is an experiment's apparatus, not part of the build. Nothing in `script/test` runs it and no
# gate needs it: `xtask initrd-aarch64` packs the resulting ELF only if it is already on disk, and
# `kernel/src/user/ripgrep_tests.rs` skips when it is not. That is deliberate, and DECISIONS §46 is
# the reason: making the gate fetch `ripgrep` and its ~40 transitive crates would put a crates.io
# dependency tree in this repository's build, which is calef's call and not a lane's.
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
if [ ! -d "$SRC" ]; then
  echo "build-ripgrep: fetching ripgrep $VERSION from crates.io"
  curl -sSL --max-time 120 -o "$BUILD/ripgrep-$VERSION.crate" \
    "https://static.crates.io/crates/ripgrep/ripgrep-$VERSION.crate"
  tar xzf "$BUILD/ripgrep-$VERSION.crate" -C "$BUILD"
fi

# The patched std lives in the `nife-dev` toolchain, which `xtask std-src` builds and links.
# `RUSTUP_TOOLCHAIN` rather than `+nife-dev` for the reason `xtask::std_exerciser` records: the
# cargo proxy exports `RUSTUP_TOOLCHAIN=nightly`, which would override a `+` selector.
(cd "$ROOT" && cargo xtask std-src)

# `-Copt-level=s` and `-Cstrip=debuginfo` are not tuning: ripgrep's own release profile sets
# `debug = 1`, which produces a 25 MB ELF the initrd would carry into RAM. Overriding a profile from
# the command line is a build setting, not a change to the program.
# **The link base has to move, and that is a finding rather than a workaround.**
# `crates/user_mode_runtime/link.ld` puts every program at `0x40_0000`, and `kernel/src/user.rs` puts every program's
# stack at `0x50_0000` with a deeper std stack below that, so an image has under 896 KiB of address
# space before it collides with its own stack. ripgrep's `.text` alone is 1.37 MiB, and the loader
# refuses it with `Unmappable(AlreadyMapped)`. Relinking at 16 MiB is a link setting and touches no
# ripgrep source, but it is not something a stranger's program would know to do: see
# notes/ripgrep-on-nife.md, which argues this address map is the thing to change.
#
# Derived from `crates/user_mode_runtime/link.ld` by substitution rather than copied, so the two
# cannot drift. That is also how it keeps a manifest note (milestone 597 (a program carries its
# manifest in an ELF note), provisional): the `PT_NOTE` header and the `.note.nife` section arrive
# with the substitution. ripgrep carries no note today, so it runs as a program that asks for
# nothing but its output; a foreign program carries one by linking an object that holds it
# (`-Clink-arg=note.o`, measured by #1319), which nothing here writes yet.
mkdir -p "$OUT"
sed 's/^    \. = 0x400000;$/    . = 0x1000000;/' "$ROOT/crates/user_mode_runtime/link.ld" > "$OUT/link-high.ld"
grep -q '0x1000000' "$OUT/link-high.ld" || { echo "build-ripgrep: crates/user_mode_runtime/link.ld no longer sets 0x400000 where this script expects it"; exit 1; }

for TRIPLE in ${NIFE_RIPGREP_TRIPLES:-aarch64-unknown-nife riscv64-unknown-nife x86_64-unknown-nife}; do
  cd "$SRC"
  RUSTUP_TOOLCHAIN=nife-dev \
  RUSTFLAGS="-Clink-arg=-T$OUT/link-high.ld -Clink-arg=-u_start -Clink-arg=--build-id=none -Cstrip=debuginfo -Copt-level=s" \
    cargo build --release \
      -Zjson-target-spec \
      -Zbuild-std=core,alloc,std,panic_abort \
      -Zbuild-std-features=compiler-builtins-mem \
      --target "$ROOT/targets/$TRIPLE.json"

  mkdir -p "$OUT/$TRIPLE"
  cp "$SRC/target/$TRIPLE/release/rg" "$OUT/$TRIPLE/rg"
  echo "build-ripgrep: $OUT/$TRIPLE/rg ($(wc -c < "$OUT/$TRIPLE/rg") bytes)"
done
