---
status: NOT-STARTED
raised: 2026-10-07
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: yes
---
# 820. `extensible_host_controller_interface`: proven xHCI logic, released on its own

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
  as of today): [...] `extensible_host_controller_interface` (no offer to rust-osdev's dormant
  `xhci`, per today's only-our-dependencies rule) [...] Names are ratified now and permanent on
  first publication."* His words: *"Yes, one per crate"*.
- That rule: *"nife contributes upstream only to projects nife depends on [...] For projects nife
  does not use, nife makes no offer and maintains no proofs there; our proofs stay public under our
  license for anyone to take."* His word: *"Yes"*.
- The release rules are forks 1 to 4 and 6 of the same pull request, applied in *Release* below.

## What exists, checked 2026-10-07 (UTC)

- `crates/extensible_host_controller_interface`: 1,191 code lines (1,817 with comments), written by
  milestone 242 (USB host and HID, because on commodity hardware the keyboard is not a UART), which
  is PARTIAL. It is everything a USB host controller driver computes and nothing it touches:
  capability and operational registers, device and endpoint contexts, the rings, port status and
  HID boot reports. Class A in #1806's note `notes/releasable-crates-2026-10-07.md`, so it depends
  on no other nife crate and needs no cut.
- Three Kani harnesses, each with a falsification patch: the register window never maps a withheld
  page, a producer ring stays in bounds, and the handoff round-trips.
- Its consumers: `kernel/src/extensible_host_controller_interface.rs` (finds and confines the
  controller), `kernel/src/user.rs`, `kernel/src/user/usb_keyboard_service.rs` and
  `components/src/usb_keyboard_driver.rs` (the EL0 driver).
- The `Handoff` types encode nife's spawn ABI rather than xHCI. Whether they stay in the published
  crate or move to the driver is the lane's call, recorded in its pull request.
- crates.io answers 404 for the name (checked 2026-10-07).

**Reuse:** the alternatives were surveyed in #1806's `survey.md` appendix (looked up 2026-10-07),
and none is taken. rust-osdev's `xhci` 0.9.2 (2023-07, MIT OR Apache-2.0, 108k downloads, no proofs)
has the right scope and has been dormant since 2024-09. The first proposal on #1806 was to ask
rust-osdev to revive it, and calef ruled no offer, since nife does not depend on it. rcore-os's
`crab-usb` 0.12.1 (Apache-2.0, 28k) lives in the `tgoskits` monorepo with no proofs found. Redox's
`xhcid` is a daemon whose GitHub mirror is archived. The xHCI 1.2 specification is the reference.

## Release, by #1806's rulings

1. In the tree, `script/verify` proves the harnesses and `Cargo.toml` says `publish = false`.
2. The package is 0.1.0 with its own semver, states `rust-version` (#1806's note built it on stable
   1.94.1), and ships its harnesses and falsification patches. `cargo package --list` shows them.
3. Ready means proofs passing and the API stable. §235 (the OS is built and updated from
   packages, and the tree divides by what releases together) has milestone 610 (the interface's
   stability is measured weekly)'s per-package co-change column judges the second. When it is ready,
   calef adds it to the lint-checked publish allowlist. A
   `nifeos/extensible_host_controller_interface` repository is then created on the shared CI
   template, through reusable workflows: Kani through `kani-github-action`, lint, release. Its
   release workflow refuses to publish unless the proof job is green. No harness here reads
   `target_arch`, so stock Kani proves them. The patched Kani of §218 (carry a Kani patch so riscv64
   is proved, and send it upstream) is carried only if one comes to need riscv64. Milestone 589
   (Kani can prove riscv64 from the hosts we already have) built it.
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

A check proves it. In nife, `cargo tree --locked -i extensible_host_controller_interface` resolves
to `v0.1.x` from crates.io with a checksum in `Cargo.lock`,
`crates/extensible_host_controller_interface` no longer exists, and the tree builds for all three
targets. In `nifeos/extensible_host_controller_interface`, the run that published 0.1.0 shows the
Kani job green ahead of the publish job, and every harness's falsification patch makes its proof
fail.

## BUGS

- Steps 3 and 4 need machinery that does not exist yet: the publish allowlist lint, the shared CI
  template, the `nifeos` crates.io team, and milestone 610's per-package co-change column. #1806
  ruled their shape; none is a milestone yet.
- Milestone 242 is PARTIAL, and the crate's own `BUGS` says a device behind a hub cannot be
  addressed. A stranger meets that limit first, so the published README states it.
- Three harnesses cover the window, one ring and the handoff. The context and port decoders carry
  none; whether they want proofs before 0.1 is the lane's question, answered in its pull request.
- `needs_person` is `yes` because the exit includes a publication only calef can approve. Everything
  before step 5 can start now.

## Index row

nife's xHCI host controller logic, three Kani proofs and their falsification patches, published on
crates.io as `extensible_host_controller_interface` 0.1 from its own `nifeos` repository instead
of being offered to rust-osdev's dormant `xhci`, then consumed back by nife from that release.
