---
status: NOT-STARTED
raised: 2026-10-07
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: yes
---
# 814. `non_volatile_memory_express`: proven NVMe queue logic, released on its own

*(Minted 2026-10-07 (UTC) by lane/publish-ours-milestones from calef's rulings on pull request
#1806. The number is provisional until the merge queue lands it; the title and slug are drafts.
The FAT crate's block in #1811 is the shape this follows.)*

calef's rulings, quoted from the maintainer comments on #1806 of 2026-10-07 (UTC):

- Policy: *"Each publish-ours crate (NVMe, e1000e, the DesignWare pair, the JH7110 pair, xHCI, and
  `portable_executable` once it carries proofs) is published when ready (proofs passing, API
  stable), each through its own milestone at its review."* His words: *"Since the consumer would be
  other hobby OSes, I think it makes sense to publish them. That's a way to engage with the broader
  community of os developers. I can't think of who else might want these crates again if nife fails
  a nice legacy is a bunch of hobby oses that use its parts."*
- Crates: *"one milestone per crate, minted now, each keeping its tree name (all free on crates.io
  as of today) [...] Names are ratified now and permanent on first publication."* His words: *"Yes,
  one per crate"*.
- The release rules are forks 1 to 4 and 6 of the same pull request, applied in *Release* below.

## What exists, checked 2026-10-07 (UTC)

- `crates/non_volatile_memory_express`: 733 code lines (1,335 with comments, one file), class A in
  #1806's note `notes/releasable-crates-2026-10-07.md`, so it depends on no other nife crate and
  needs no cut.
- Eight Kani harnesses, each with a falsification patch in `falsifications/`. Submission pushes and
  completion pops stay in bounds, and the phase flips only at the wrap. Doorbells never collide.
  The PRP pair is total and page-disciplined, and the identify parse is total and bounds its shift.
  The spawn handoff round-trips and keeps its doorbells inside the mapped page. A transfer command
  is only built for a block the namespace has.
- Its consumers: `kernel/src/non_volatile_memory_express.rs` (the volatile shell),
  `kernel/src/user/non_volatile_memory_express_service.rs`, `kernel/src/user/install_service.rs`,
  `kernel/src/disk_throughput.rs`, `components/src/non_volatile_memory_express.rs` (the EL0
  driver) and `system_tests/src/user/non_volatile_memory_express_tests.rs`.
- The `Handoff` types encode nife's spawn ABI rather than NVMe. Whether they stay in the published
  crate or move to the driver is the lane's call, recorded in its pull request, since a stranger's
  kernel has no use for them.
- crates.io answers 404 for the name (checked 2026-10-07).

**Reuse:** the alternatives were surveyed in #1806's `survey.md` appendix (looked up 2026-10-07),
and none is taken, because calef ruled this crate "publish ours". rcore-os's `nvme-driver` 0.8.2
(MIT, 4.9k downloads, one reverse dependency, no proofs found) lives in the `tgoskits` monorepo.
lihanrui2913's `nvme` is a 0.0.0 from 2025-04. Redox's `nvmed` is a daemon, not a library, and its
GitHub mirror is archived. SPDK (C) stays the reference for behavior. None is a sans-IO layer a
model checker can reach, and the proofs are the point. No upstream offer goes anywhere: calef ruled
on #1806 that nife contributes only to projects nife depends on.

## Release, by #1806's rulings

1. In the tree, `script/verify` proves the harnesses and `Cargo.toml` says `publish = false`.
2. The package is 0.1.0 with its own semver, states `rust-version` (#1806's note built it on stable
   1.94.1), and ships its harnesses and falsification patches. `cargo package --list` shows them.
3. Ready means proofs passing and the API stable. §235 (the OS is built and updated from
   packages, and the tree divides by what releases together) has milestone 610 (the interface's
   stability is measured weekly)'s per-package co-change column judges the second. When it is ready,
   calef adds it to the lint-checked publish allowlist. A `nifeos/non_volatile_memory_express`
   repository is then created on the shared CI template, through reusable workflows: Kani through
   `kani-github-action`, lint, release. Its release workflow refuses to publish unless the proof job
   is green. No harness here reads `target_arch`, so stock Kani proves them. The patched Kani of
   §218 (carry a Kani patch so riscv64 is proved, and send it upstream) is carried only if one comes
   to need riscv64. Milestone 589 (Kani can prove riscv64 from the hosts we already have) built it.
4. crates.io trusted publishing from that workflow, owned by a `nifeos` team, with no stored token.
5. calef approves this publication, as he does every one.
6. nife consumes the crates.io release, pinned with its checksum in `Cargo.lock`. A `[patch]` to a
   checkout is allowed only during development, and a lint fails one that reaches `main`. The copy
   under `crates/` goes.

## Parity

Pure logic, tested on the host. This lane built it on 2026-10-07 with the pinned nightly for
`aarch64-unknown-none-softfloat`, `riscv64imac-unknown-none-elf`, `x86_64-unknown-none` and the
host, all exit 0. The release workflow builds the same four.

## Exit

A check proves it. In nife, `cargo tree --locked -i non_volatile_memory_express` resolves to
`v0.1.x` from crates.io with a checksum in `Cargo.lock`, `crates/non_volatile_memory_express` no
longer exists, and the tree builds for all three targets. In `nifeos/non_volatile_memory_express`,
the run that published 0.1.0 shows the Kani job green ahead of the publish job, and every harness's
falsification patch makes its proof fail.

## BUGS

- Steps 3 and 4 need machinery that does not exist yet: the publish allowlist lint, the shared CI
  template, the `nifeos` crates.io team, and milestone 610's per-package co-change column. #1806
  ruled their shape; none is a milestone yet.
- `needs_person` is `yes` because the exit includes a publication only calef can approve. Everything
  before step 5 can start now.

## Index row

nife's NVMe queue arithmetic, with eight Kani proofs and their falsification patches, published on
crates.io as `non_volatile_memory_express` 0.1 from its own `nifeos` repository. Another hobby OS
can then drive NVMe on proven queue logic, and nife consumes the same release.
