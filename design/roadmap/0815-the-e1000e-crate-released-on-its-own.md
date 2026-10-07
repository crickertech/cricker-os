---
status: NOT-STARTED
raised: 2026-10-07
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: yes
---
# 815. `e1000e`: proven Intel e1000e ring logic, released on its own

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

- `crates/e1000e`: 2,151 code lines (3,210 with comments), written by milestone 494 (a driver for
  the network card a PC actually has), which is PARTIAL. It claims the 82574L that QEMU's
  `-device e1000e` presents and the Sunrise Point I219 rows, which is xenon's I219-LM5. Later PCH
  generations are refused until a bench proves one; the crate's `BUGS` says why.
- Four Kani harnesses, each with a falsification patch. A delivered frame always fits the buffer
  and the room. The handoff round-trips and accepts only what `pack` can make. Every ring offset is
  inside the region, and a paged access selects the offset's own page and register.
- Class B in #1806's note `notes/releasable-crates-2026-10-07.md`. **The cut**: inline
  `address_space_map::pair_page`, its one use (`src/lib.rs`, in the handoff). That function places
  nife's virtual addresses, so the handoff it serves is nife's spawn ABI; whether the handoff stays
  in the published crate or moves to the driver is the lane's call, recorded in its pull request.
- Its consumers: `kernel/src/e1000e.rs` (reset, MAC address, ring bases), `kernel/src/pci.rs`,
  `kernel/src/user/e1000e_service.rs`, `kernel/src/network_bench.rs`, and in `components/`,
  `e1000e_transport.rs` and `net_stack.rs` (descriptors and tails).
- crates.io answers 404 for the name (checked 2026-10-07).

**Reuse:** the alternatives were surveyed in #1806's `survey.md` appendix (looked up 2026-10-07),
and none is taken, because calef ruled this crate "publish ours". rcore-os's `eth-intel` 0.2.4
(MIT, 10.8k downloads, no proofs found) is a young driver inside the `tgoskits` monorepo.
elliott10's `e1000-driver` 0.1.0 (2023-02) is GPL-2.0 and drives the older e1000. Redox's `e1000d`
is a daemon whose GitHub mirror is archived. Linux's `e1000e` and FreeBSD's `em` stay the references
for behavior and are read, never copied. No upstream offer goes anywhere: calef ruled on #1806 that
nife contributes only to projects nife depends on.

## Release, by #1806's rulings

1. Make the cut above. `cargo tree -p e1000e` then lists no nife crate.
2. In the tree, `script/verify` proves the harnesses and `Cargo.toml` says `publish = false`.
3. The package is 0.1.0 with its own semver, states `rust-version` (#1806's note built it on stable
   1.94.1), and ships its harnesses and falsification patches. `cargo package --list` shows them.
4. Ready means proofs passing and the API stable. §235 (the OS is built and updated from
   packages, and the tree divides by what releases together) has milestone 610 (the interface's
   stability is measured weekly)'s per-package co-change column judges the second. When it is ready,
   calef adds it to the lint-checked publish allowlist. A `nifeos/e1000e` repository is then created
   on the shared CI template, through reusable workflows: Kani through `kani-github-action`, lint,
   release. Its release workflow refuses to publish unless the proof job is green. No harness here
   reads `target_arch`, so stock Kani proves them. The patched Kani of §218 (carry a Kani patch so
   riscv64 is proved, and send it upstream) is carried only if one comes to need riscv64. Milestone
   589 (Kani can prove riscv64 from the hosts we already have) built it.
5. crates.io trusted publishing from that workflow, owned by a `nifeos` team, with no stored token.
6. calef approves this publication, as he does every one.
7. nife consumes the crates.io release, pinned with its checksum in `Cargo.lock`. A `[patch]` to a
   checkout is allowed only during development, and a lint fails one that reaches `main`. The copy
   under `crates/` goes.

## Parity

Pure logic, tested on the host against a simulated device. This lane built it on 2026-10-07 with
the pinned nightly for `aarch64-unknown-none-softfloat`, `riscv64imac-unknown-none-elf`,
`x86_64-unknown-none` and the host, all exit 0. The release workflow builds the same four.

## Exit

A check proves it. In nife, `cargo tree --locked -i e1000e` resolves to `v0.1.x` from crates.io
with a checksum in `Cargo.lock`, `crates/e1000e` no longer exists, and the tree builds for all three
targets. In `nifeos/e1000e`, the run that published 0.1.0 shows the Kani job green ahead of the
publish job, and every harness's falsification patch makes its proof fail.

## BUGS

- Steps 4 and 5 need machinery that does not exist yet: the publish allowlist lint, the shared CI
  template, the `nifeos` crates.io team, and milestone 610's per-package co-change column. #1806
  ruled their shape; none is a milestone yet.
- Milestone 494 is still PARTIAL, so the API may still move. Semver 0.x allows that; publishing
  before 494 lands costs only version churn.
- `needs_person` is `yes` because the exit includes a publication only calef can approve. Everything
  before step 6 can start now.

## Index row

nife's Intel e1000e ring and register logic, with four Kani proofs and their falsification patches,
cut free of nife's address map. It is published on crates.io as `e1000e` 0.1 from its own `nifeos`
repository, and nife consumes that release.
