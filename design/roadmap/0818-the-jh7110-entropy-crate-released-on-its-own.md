---
status: NOT-STARTED
raised: 2026-10-07
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: yes
---
# 818. `jh7110_entropy`: proven JH7110 TRNG logic, released on its own

*(Minted 2026-10-07 (UTC) by lane/publish-ours-milestones from calef's rulings on pull request
#1806. The number is provisional until the merge queue lands it; the title and slug are drafts.
The FAT crate's block in #1811 is the shape this follows.)*

calef's rulings, quoted from the maintainer comments on #1806 of 2026-10-07 (UTC):

- Policy: *"Each publish-ours crate (NVMe, e1000e, the DesignWare pair, the JH7110 pair, xHCI, and
  `portable_executable` once it carries proofs) is published when ready (proofs passing, API
  stable), each through its own milestone at its review."* His words: *"Since the consumer would be
  other hobby OSes, I think it makes sense to publish them. [...] a nice legacy is a bunch of hobby
  oses that use its parts."*
- Crates: *"one milestone per crate, minted now, each keeping its tree name (all free on crates.io
  as of today) [...] Names are ratified now and permanent on first publication."* His words: *"Yes,
  one per crate"*.
- The release rules are forks 1 to 4 and 6 of the same pull request, applied in *Release* below.

## What exists, checked 2026-10-07 (UTC)

- `crates/jh7110_entropy`: 516 code lines (1,247 with comments), written by milestone 159 (a real
  hardware entropy source: the JH7110's TRNG), which is BUILT. It holds the StarFive JH7110 true
  random number generator's register layout, the decision whether a generation attempt succeeded,
  failed or is still running, and the device-tree query that finds it.
- Four Kani harnesses, each with a falsification patch: a lockup bit is never overridden, ready
  requires `RAND_RDY` and carries the words untouched, neither bit set is always not ready, and an
  unseeded core never yields bytes.
- Class B in #1806's note `notes/releasable-crates-2026-10-07.md`. **The cut**, which is codebase
  rule 2 (a driver gets what it needs passed in): `discover` takes a `DeviceTreeBlob`; the caller
  passes the register region instead, and the query moves to the kernel side, which already depends
  on `device_tree_blob`. The tests also `include_bytes!` a fixture from
  `../../device_tree_blob/tests/fixtures/`, outside the package.
- Its consumers: `kernel/src/user/entropy_service.rs`, `kernel/src/memory.rs`, `kernel/src/lib.rs`
  and `components/src/jh7110_entropy.rs`.
- crates.io answers 404 for the name (checked 2026-10-07).

**Reuse:** the alternatives were surveyed in #1806's `survey.md` appendix (looked up 2026-10-07),
and none is taken, because calef ruled this crate "publish ours". The only Rust JH7110 crates found,
weathered-steel's `jh71xx-pac` 0.11.1 and `jh71xx-hal` 0.7.2 (Codeberg, 2025-03), are register maps
under GPL-3.0-only, which nife cannot take under its license and which carry no TRNG logic. No Rust
crate for the TRNG was found. Linux's `jh7110-trng` driver stays the reference and is read, never
copied. No upstream offer goes anywhere: calef ruled on #1806 that nife contributes only to
projects nife depends on.

## Release, by #1806's rulings

1. Make the cut above. `cargo tree -p jh7110_entropy` then lists no nife crate, and the tests read
   only fixtures inside the package.
2. In the tree, `script/verify` proves the harnesses and `Cargo.toml` says `publish = false`.
3. The package is 0.1.0 with its own semver, states `rust-version` (#1806's note built it on stable
   1.94.1), and ships its harnesses and falsification patches. `cargo package --list` shows them.
4. Ready means proofs passing and the API stable. §235 (the OS is built and updated from
   packages, and the tree divides by what releases together) has milestone 610 (the interface's
   stability is measured weekly)'s per-package co-change column judges the second. When it is ready,
   calef adds it to the lint-checked publish allowlist. A `nifeos/jh7110_entropy` repository is then
   created on the shared CI template, through reusable workflows: Kani through `kani-github-action`,
   lint, release. Its release workflow refuses to publish unless the proof job is green. No harness
   here reads `target_arch`, so stock Kani proves them. The patched Kani of §218 (carry a Kani patch
   so riscv64 is proved, and send it upstream) is carried only if one comes to need riscv64.
   Milestone 589 (Kani can prove riscv64 from the hosts we already have) built it.
5. crates.io trusted publishing from that workflow, owned by a `nifeos` team, with no stored token.
6. calef approves this publication, as he does every one.
7. nife consumes the crates.io release, pinned with its checksum in `Cargo.lock`. A `[patch]` to a
   checkout is allowed only during development, and a lint fails one that reaches `main`. The copy
   under `crates/` goes.

## Parity

Pure logic, tested on the host. This lane built it on 2026-10-07 with the pinned nightly for
`aarch64-unknown-none-softfloat`, `riscv64imac-unknown-none-elf`, `x86_64-unknown-none` and the
host, all exit 0. The release workflow builds the same four. The silicon is riscv64 only; the
crate is not.

## Exit

A check proves it. In nife, `cargo tree --locked -i jh7110_entropy` resolves to `v0.1.x` from
crates.io with a checksum in `Cargo.lock`, `crates/jh7110_entropy` no longer exists, and the tree
builds for all three targets. In `nifeos/jh7110_entropy`, the run that published 0.1.0 shows the
Kani job green ahead of the publish job, and every harness's falsification patch makes its proof
fail.

## BUGS

- Steps 4 and 5 need machinery that does not exist yet: the publish allowlist lint, the shared CI
  template, the `nifeos` crates.io team, and milestone 610's per-package co-change column. #1806
  ruled their shape; none is a milestone yet.
- The TRNG needs its clocks and reset released first, which is `jh7110_clock_and_reset`'s plan
  (milestone 819 (JH7110 clock and reset logic, proven, then released on its own)). A stranger using
  this crate alone must do that bring-up some other way; the crate's documentation says so before it
  is published.
- The device-tree fixtures (`tests/fixtures/*.dts`) are excerpts of the mainline and vendor JH7110
  trees. Their licenses were not checked here; record each before `cargo package` ships them.
- `needs_person` is `yes` because the exit includes a publication only calef can approve. Everything
  before step 6 can start now.

## Index row

nife's StarFive JH7110 true random number generator logic, with four Kani proofs and their
falsification patches, cut free of nife's device tree. It is published on crates.io as
`jh7110_entropy` 0.1 from its own `nifeos` repository, and nife consumes that release.
