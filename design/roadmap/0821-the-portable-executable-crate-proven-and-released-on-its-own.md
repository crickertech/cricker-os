---
status: NOT-STARTED
raised: 2026-10-07
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: yes
---
# 821. `portable_executable`: an ELF-to-PE converter, proven, then released on its own

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
  as of today): [...] and `portable_executable` (its milestone writes its proofs first). Names are
  ratified now and permanent on first publication."* His words: *"Yes, one per crate"*.
- The release rules are forks 1 to 4 and 6 of the same pull request, applied in *Release* below.

## What exists, checked 2026-10-07 (UTC)

- `crates/portable_executable`: 778 code lines (978 with comments), written by milestone 441 (the
  program that makes the stick: one download per host, a boot for every architecture). rustc has no
  riscv64 UEFI target, so the riscv64 loader is linked as a static-PIE ELF. `from_elf` converts it
  into the PE32+ image firmware loads: one section per `PT_LOAD` at the ELF's own addresses, and
  one `DIR64` fixup per `RELATIVE` relocation. Class A in #1806's note
  `notes/releasable-crates-2026-10-07.md`: no dependencies, `no_std` with `alloc`.
- **No Kani harnesses and no falsification patches.** Its one consumer is `xtask/src/stick.rs`.
- **Its `BUGS` says it is not hardened against a hostile ELF**, from the 2026-09-24 security audit.
  `phoff` near `u64::MAX` overflows before the bounds check, and a `PT_DYNAMIC` offset is never
  bounded. A `memsz` of 2^44 with `filesz` 0 sizes the image `Vec`, and `DT_RELAENT = 0` loops until
  memory runs out. That was safe while the only input was an ELF `xtask` linked seconds earlier. A
  published crate takes a stranger's ELF, so the hardening is now owed, and the proofs are what
  show it is done. Milestone 326 (nobody has been assigned to turn a mutation score upward) already
  fixed one wrong-accept here, a relocation bounded by `memsz` where `filesz` was meant.
- crates.io answers 404 for the name (checked 2026-10-07).

**Reuse:** the alternatives were surveyed in #1806's `survey.md` appendix (looked up 2026-10-07),
and none is taken, because calef ruled this crate "publish ours". No ELF-to-PE converter crate
exists on crates.io. systemd's `tools/elf2efi.py` (LGPL-2.1+, Python) takes the same approach and is
the prior art. gimli-rs's `object` 0.40.0 has a PE writer but no converter. Building on it would put
a large dependency inside a crate whose point is that a model checker can reach all of it, which §46
(thin primitives or whole subsystems; we write everything in between) refuses. `goblin` parses both
formats and converts neither.

## What is proven

Kani harnesses, each with a falsification patch at
`crates/portable_executable/falsifications/<module.path>.<harness>.patch` as §134 (a harness
carries a machine-replayable falsification record) and `script/falsifications` require. The first
four are the audit's holes, closed:

1. No panic and no overflow on any input. Each structure read (the file header, a program header, a
   dynamic entry, an `Elf64_Rela`) takes symbolic bytes and returns a value or an error, with every
   offset sum checked. Kani bounds its inputs, so whole-file coverage comes from composing
   per-structure proofs, with `script/fuzz` over whole files as the backstop. The block states that
   bound rather than claiming more.
2. The image size is capped. No accepted ELF sizes the output beyond a stated limit, whatever
   `memsz` says.
3. The relocation walk terminates. A zero or odd `DT_RELAENT` is refused, and the walk visits at
   most `DT_RELASZ / DT_RELAENT` entries.
4. Every fixup lands inside the image. Each `.reloc` entry names an 8-byte word inside a section,
   and the word holds the relocation's addend.
5. The layout is the ELF's. Each section's RVA equals its segment's virtual address and its raw
   bytes are the segment's file bytes, zero-filled to `memsz`.

## Release, by #1806's rulings

1. Close the four holes in `BUGS`, write the five proofs above with their falsification patches, and
   rewrite `BUGS` to say what the crate now refuses.
2. In the tree, `script/verify` proves the harnesses and `Cargo.toml` says `publish = false`.
3. The package is 0.1.0 with its own semver, states `rust-version` (#1806's note built it on stable
   1.94.1), and ships its harnesses and falsification patches. `cargo package --list` shows them.
4. Ready means proofs passing and the API stable. §235 (the OS is built and updated from
   packages, and the tree divides by what releases together) has milestone 610 (the interface's
   stability is measured weekly)'s per-package co-change column judges the second. When it is ready,
   calef adds it to the lint-checked publish allowlist. A `nifeos/portable_executable` repository is
   then created on the shared CI template, through reusable workflows: Kani through
   `kani-github-action`, lint, release. Its release workflow refuses to publish unless the proof job
   is green. Nothing here reads `target_arch`; the ELF's machine is data, not the build target. So
   stock Kani serves. The patched Kani of §218 (carry a Kani patch so riscv64 is proved, and send it
   upstream) is carried only if a harness comes to need riscv64. Milestone 589 (Kani can prove
   riscv64 from the hosts we already have) built it.
5. crates.io trusted publishing from that workflow, owned by a `nifeos` team, with no stored token.
6. calef approves this publication, as he does every one.
7. nife consumes the crates.io release, pinned with its checksum in `Cargo.lock`. A `[patch]` to a
   checkout is allowed only during development, and a lint fails one that reaches `main`. The copy
   under `crates/` goes.

## Parity

A host tool in nife, but `no_std` with `alloc`, so it builds anywhere. This lane built it on
2026-10-07 with the pinned nightly for `aarch64-unknown-none-softfloat`,
`riscv64imac-unknown-none-elf`, `x86_64-unknown-none` and the host, all exit 0. The release
workflow builds the same four. Its tests convert riscv64 PIEs; an aarch64 or x86_64 PIE is outside
the first release, since rustc emits PE for those directly.

## Exit

A check proves it. `script/verify` proves the five properties, `script/falsifications` reports every
harness `replayable`, and each of the four ELFs the audit described is refused with an error by a
host test. In nife, `cargo tree --locked -i portable_executable` resolves to `v0.1.x` from crates.io
with a checksum in `Cargo.lock`, `crates/portable_executable` no longer exists, and the riscv64
stick still boots in EDK2 under the existing UEFI gate. In `nifeos/portable_executable`, the run
that published 0.1.0 shows the Kani job green ahead of the publish job.

## BUGS

- Steps 4 and 5 need machinery that does not exist yet: the publish allowlist lint, the shared CI
  template, the `nifeos` crates.io team, and milestone 610's per-package co-change column. #1806
  ruled their shape; none is a milestone yet.
- The five properties are this lane's proposal, read from the code and its audit, not a ruling.
- `needs_person` is `yes` because the exit includes a publication only calef can approve. Everything
  before step 6 can start now.

## Index row

nife's ELF-to-PE/COFF converter, the only one on crates.io, hardened against a hostile ELF and given
its first Kani proofs, then published as `portable_executable` 0.1 from its own `nifeos` repository
and consumed back by `cargo xtask` from that release.
