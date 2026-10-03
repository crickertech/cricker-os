# Patches carried against upstream projects

One file per patch, in `git format-patch` form, applied with `git am`. Each exists to be
upstreamed; an entry leaves this directory when the pin that needed it advances past a release
containing the fix.

- `redoxfs-no-std-vec-import.patch` — fixes redoxfs's no_std build (Vec imports the std prelude
  masked; four E0425 sites across filesystem.rs and record.rs) and adds a
  `--no-default-features` CI job so the configuration cannot bit-rot again. Written against
  master @ 99bc185 (2026-07-27); milestone 32's 0.9.1 pin carries the same fix. Submission:
  fork on gitlab.redox-os.org, `git am` this file on a branch, push, open the MR; see
  notes/redoxfs-audit.md for the audit that produced it.
- `redoxfs-no-std-create-uuid.patch` — lets a `no_std` caller create a filesystem by supplying the
  disk id, the same way `create` already takes `ctime` because the engine has no clock
  (`Header::new_with_uuid`, `FileSystem::create_reserved_with_uuid`; the `std` entry points keep
  their signatures and no existing caller changes). Written against the published 0.9.1, which is
  what milestone 32 pins, rather than against master: it applies there with zero fuzz, and rebasing
  it onto master is the submitter's first step. Same submission route as above. Milestone 57's write
  half needed it because on a bare-metal target the randomness comes from a service the caller
  holds, never from the filesystem library; see notes/nifefs.md.
- `kani-0.67.0-riscv64-target.patch`: lets Kani compile for `riscv64gc-unknown-linux-gnu` from an
  aarch64 or x86_64 host, selected by `KANI_TARGET` (name provisional), so the `kernel` row is proved
  for riscv64 without a riscv64 machine. Two commits in one file, applied together. The first is
  four files, about fifty lines: a riscv64 machine model,
  the target allowlist, riscv64gc's target features (without them rustc warns that `d` must be
  enabled, on every crate), and the triple the driver and the sysroot build use. The second, from
  milestone 635 (riscv64 proofs check against the riscv64 model), links `kani_lib.c` as a
  precompiled goto object after the Rust inputs, so CBMC checks the machine model Kani wrote rather
  than the host's; script/verify-riscv64 fails if it does not, and its upstream route is the
  follow-up promised on model-checking/kani#4913. Written against the
  `kani-0.67.0` tag, which is the version its file name pins and the only place the tree states
  it. Unlike the two above it is **applied by the tree**, not only offered: script/verify-riscv64
  clones that tag, `git am`s this file and builds Kani, and the `prove the kernel on riscv64` job in
  `.github/workflows/verify.yml` runs it. DECISIONS §218 is the ruling (carry it, and never build
  from a fork); milestone 589 is the work. Upstream: model-checking/kani#2402, which a separate lane
  answers with a `-Z` flag; this file stays the minimal shape meanwhile. Delete it when the pin
  reaches a Kani release with riscv64 target support; rebase and rename it whenever the pin moves
  before then. The patch's own header repeats all of this, and says when a fork would be worth it.
