#!/bin/sh
# Run the 2026-10-09 lint census's clippy passes and keep their JSON. From the repository root:
#
#     OUT=/some/dir sh notes/lint-census-2026-10-09/clippy-runs.sh
#     python3 notes/lint-census-2026-10-09/clippy_counts.py /some/dir
#
# Five configurations, none with --all-targets, so cfg(test) code is never compiled: the host
# workspace (script/lint's host package selection), the kernel for each of the three
# architectures (aarch64 also takes components and user_mode_runtime), and redoxfs_server's own
# workspace. Default features only, so the kernel's build-mode features are not in the count.
# The x86_64 kernel takes -A dead_code for the reason script/lint's x86_64 pass gives.
#
# A scratch clippy.toml (the tree's, plus two thresholds at 0) makes too_many_lines and
# cognitive_complexity report every function, which is how the distributions are read. NESTING=4
# instead runs only excessive_nesting at that threshold, on the host and aarch64 configurations.
# A separate CARGO_TARGET_DIR (default $OUT/target) keeps target/ untouched. Read-only against the tree.
set -eu
OUT=${OUT:?set OUT to a directory for the JSON}
mkdir -p "$OUT/conf"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$OUT/target}" CLIPPY_CONF_DIR="$OUT/conf"
cp clippy.toml "$OUT/conf/clippy.toml"
HOST="--workspace --exclude kernel --exclude system_tests --exclude components --exclude fixtures
      --exclude user_mode_runtime --exclude swap_protocol --exclude virtio
      --exclude supervision_protocol --exclude system_initializer"

if [ -n "${NESTING:-}" ]; then
    echo "excessive-nesting-threshold = $NESTING" >> "$OUT/conf/clippy.toml"
    # shellcheck disable=SC2086
    cargo clippy --message-format=json $HOST -- -W clippy::excessive_nesting > "$OUT/nest-host.json"
    cargo clippy --message-format=json -p kernel -p components -p user_mode_runtime \
        --target aarch64-unknown-none-softfloat -- -W clippy::excessive_nesting > "$OUT/nest-a64.json"
    exit 0
fi

printf 'too-many-lines-threshold = 0\ncognitive-complexity-threshold = 0\n' >> "$OUT/conf/clippy.toml"
L="-W clippy::unwrap_used -W clippy::expect_used -W clippy::panic -W clippy::unreachable
   -W clippy::todo -W clippy::unimplemented -W clippy::indexing_slicing
   -W clippy::cast_possible_truncation -W clippy::cast_sign_loss -W clippy::cast_possible_wrap
   -W clippy::arithmetic_side_effects -W clippy::too_many_lines -W clippy::cognitive_complexity
   -W clippy::allow_attributes -W clippy::allow_attributes_without_reason -W static_mut_refs"
# shellcheck disable=SC2086
cargo clippy --message-format=json $HOST -- $L > "$OUT/host.json"
# shellcheck disable=SC2086
cargo clippy --message-format=json -p kernel -p components -p user_mode_runtime \
    --target aarch64-unknown-none-softfloat -- $L > "$OUT/a64.json"
# shellcheck disable=SC2086
cargo clippy --message-format=json -p kernel --target riscv64imac-unknown-none-elf -- $L > "$OUT/rv.json"
# shellcheck disable=SC2086
cargo clippy --message-format=json -p kernel --target x86_64-unknown-none -- $L -A dead_code > "$OUT/x86.json"
# shellcheck disable=SC2086
cargo clippy --message-format=json --manifest-path redoxfs_server/Cargo.toml -- $L > "$OUT/redox.json"
