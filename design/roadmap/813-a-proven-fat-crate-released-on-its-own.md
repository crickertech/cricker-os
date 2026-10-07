---
status: NOT-STARTED
raised: 2026-10-07
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 813. `file_allocation_table`: a proven FAT crate, released on its own

*(Minted 2026-10-07 (UTC) by lane/file-allocation-table-milestone from calef's rulings on pull
requests #1803 and #1806. The number is provisional until the merge queue lands it; the title and
slug are drafts. On the customer path: milestone 140 (mount a drive this system did not create)
needs it for its FAT32 stratum.)*

calef's rulings, quoted from the maintainer comments of 2026-10-07 (UTC):

- #1803, fork 2: *"take no FAT dependency. nife writes its own FAT12/16/32 crate (about 5,000 lines
  by the probed crates' sizes), proven with Kani because it parses hostile input from foreign
  sticks, with rust-fatfs and this probe's round-trip images as its test oracle; and it is built,
  proven and released on its own, independently of nife."* His words: *"I'm really questioning
  taking a dependency on something that isn't updating."* and *"It seems like something we should
  build and prove and release on its own independently of nife."*
- #1806, scope: *"FAT12, FAT16 and FAT32, read and write, with long file names (VFAT), in the first
  release; exFAT later, as its own milestone or sibling crate. Proven: hostile images never panic or
  read out of bounds, cluster chains cannot loop, written data reads back; oracle rust-fatfs and
  `fsck_msdos` on #1803's images."* His word: *"Yes"*.
- #1806, name: *"`file_allocation_table` (crates.io treats it and `file-allocation-table` as one
  name; free as of today). Permanent once first published."* His word: *"Yes"*.
- #1806, milestone: *"mint a new milestone, "`file_allocation_table`: a proven FAT crate, released
  on its own" (write it, prove it, oracle tests, its own nifeos repository on the shared CI template
  at first publication, trusted publishing); milestone 140's FAT32 stratum depends on it and adds
  the filesystem-contract server. On the customer path."* His word: *"Yes"*.

## What exists, checked 2026-10-07 (UTC)

- `crates/file_allocation_table` already exists: 794 lines, no Kani harnesses, written by milestone
  198 (a package manager, and the trivial install that makes a second customer possible) rung 2a.
  It is a `mkfs` that lays out one FAT32 EFI system partition holding one 8.3-named file, as pure
  computation with no I/O. `components/src/system_installer.rs` is its one caller. It reads nothing.
  This milestone grows that crate rather than starting a second one, since the ratified name is
  already its name; the installer's call keeps working throughout. Its module doc says it "cannot
  open, read, extend, delete or rename anything, and it never will", and points the read half at
  milestone 140. This milestone retires that sentence. calef has since ruled that the crate is the
  format and 140 is the server. No scope conflict: the `mkfs` becomes the crate's format path.
- Milestone 560 (a long file name, or riscv64 cannot be installed) is that `mkfs` crate's missing
  long-name entry. The long-name writer here covers it.
- `notes/filesystem-crates-2026-10-07.md` (#1803) measured five read-write FAT crates on nife's
  three targets and built two FAT32 images that macOS formatted and wrote, one bare and one inside
  an MBR, with every file's hash in `expected.sha256`. `make-images.sh` rebuilds them on macOS.
- `crates.io` answers 404 for `file_allocation_table` (checked 2026-10-07).

**Reuse:** written here, not taken, by calef's ruling on #1803 fork 2. Three reasons, all from that
ruling. It parses hostile input (a stranger's USB stick), and §46 (thin primitives or whole
subsystems) has nife write the crates Kani proves, because a model checker needs code we can
restructure. Proofs are the point. And the main-line candidate, rust-fatfs, has not released since
`fatfs` 0.3.6 (2023-01-17); that release writes subdirectories `fsck_msdos` rejects, the fix landed
upstream on 2024-10-23 unreleased, and a release request has been open since 2023 (rafalh/rust-fatfs
#81).

## References and oracles

Read for behavior and test against; copy no code unless its license allows it and the copy is
recorded at the copied item with its source, commit and license.

| crate | license | use here |
|---|---|---|
| rust-fatfs (main line) | MIT | the oracle: host tests read what this crate wrote, and this crate reads what it wrote |
| lamfat 0.4.2 | MIT | rust-fatfs's main line republished; the way to pin it as a dev-dependency if a git pin will not do |
| hadris-fat 3.0.0-rc.1 | MIT | a second reader, and the exFAT reference for the later crate |
| embedded-sdmmc 0.10.0 | MIT or Apache-2.0 | a reference for FAT16/32 without allocation. Its project bans AI-generated contributions, so per #1806 fork 8 it gets bug reports only |
| simple-fatfs 0.1.0-alpha.2 | MIT | a `no_std` reference; alpha, last updated 2025-09-14 |

The specification is Microsoft's "FAT: General Overview of On-Disk Format" (version 1.03), which
the existing crate already cites. A dev-dependency is still a dependency under §46, so the lane names
the oracle's pin in its pull request for calef's ruling.

## Scope

- FAT12, FAT16 and FAT32, read and write: mount, walk directories, read, create, extend, truncate,
  delete, rename, make and remove directories, and format (the existing `mkfs` folds in here).
- Long file names (VFAT): read and write, with the short-name alias and checksum generated and
  checked.
- An MBR-partitioned stick and a bare volume, since #1803's images are one of each.
- `no_std`, with no allocation in the parser if that proves feasible. The crate stays sans-I/O, the
  rule `globally_unique_identifier_partition_table` keeps: the caller supplies sectors through a
  small interface the lane chooses, and the crate never names a device.

Non-goals for the first release:

- exFAT. calef ruled it later, as its own milestone or a sibling crate. hadris-fat is its reference.
- A journal or crash-consistency claim. FAT has none. Milestone 140 already says the difference
  belongs where a user meets it; this crate's `BUGS` says it too.
- The filesystem-contract server. That is milestone 140's FAT32 stratum, which consumes this crate.

## What is proven

Kani harnesses, each with a falsification patch at
`crates/file_allocation_table/falsifications/<module.path>.<harness>.patch` as §134 (a harness carries a machine-replayable falsification record) and
`script/falsifications` require:

1. No panic and no out-of-bounds read on any image. Every parse step (boot sector, BPB, FSInfo, FAT
   entry, directory entry, long-name run) takes a symbolic sector and returns a value or an error.
   Kani bounds its inputs, so whole-image coverage comes from composing per-sector proofs, with
   `script/fuzz` over whole images as the backstop. The block states that bound rather than
   claiming more.
2. Cluster chains cannot loop. Walking a chain from any start over any FAT contents ends within the
   volume's cluster count, either at an end-of-chain marker or with an error.
3. Write then read round-trips. On a small symbolic volume, what a write puts down, a read returns,
   byte for byte, and the FAT and directory stay consistent.
4. Directory entries and long names are well formed. Every entry the writer emits has a legal
   short name, every long-name run carries the matching checksum and ordinals in sequence, and the
   reader refuses a run that does not.

## Oracle tests

- Read #1803's two macOS images and check every file against `expected.sha256`.
- Write a tree (long names, nested directories, a file spanning many clusters, a delete, a rename)
  and check it two ways: rust-fatfs reads back the same names and bytes, and `fsck_msdos` exits 0.
  Where CI runs Linux, `fsck.fat` from dosfstools stands in, run as a tool and never linked.
- FAT12 and FAT16 volumes made by `newfs_msdos` or `mkfs.fat`, since #1803's images are FAT32 only.
  No test image is made by this crate.

## Parity

The crate is pure logic and its tests run on the host. It builds for
`aarch64-unknown-none-softfloat`, `riscv64imac-unknown-none-elf` and `x86_64-unknown-none` in the
tree's build, so nothing about it can work on one target and not another.

## Release, by #1806's rulings

1. In the tree, `script/verify` runs the harnesses, and `Cargo.toml` says `publish = false`.
2. The package is version 0.1.0 with its own semver, states its minimum Rust version in
   `rust-version`, and ships its harnesses and falsification patches. `cargo package --list` shows
   them.
3. At first publication, and not before, calef adds it to the lint-checked publish allowlist. A
   `nifeos/file_allocation_table` repository is then created on the shared CI template (Kani
   through `kani-github-action`, lint, release). Its release workflow refuses to publish unless the
   proof job is green.
4. crates.io trusted publishing from that workflow, owned by a `nifeos` team, with no stored token.
5. calef approves this publication, as he does every one.
6. nife then consumes the crates.io release, pinned with its checksum in `Cargo.lock`, and the copy
   in this tree goes.

## Exit

`cargo test -p file_allocation_table` fails if any piece is missing. It reads both #1803 images to
their recorded hashes, writes the tree above on FAT12, FAT16 and FAT32, and has rust-fatfs and
`fsck_msdos` (or `fsck.fat`) accept each. It formats a FAT32 EFI system partition holding
`\EFI\BOOT\BOOTRISCV64.EFI`, a name the `mkfs` refuses today. `script/verify` proves the four
properties, `script/falsifications` reports every harness `replayable`, and the crate builds for all
three targets. Publication is the last step and not the exit, since it waits on calef.

## BUGS

- Steps 3 and 4 of the release need machinery that does not exist yet: the publish allowlist lint,
  the shared CI template and the `nifeos` crates.io team. #1806 ruled their shape; none is a
  milestone yet.
- The "about 5,000 lines" figure is the probed crates' size, not an estimate of this one.
- Long-name patent status was not checked here. Read it and record it before first publication.

## Index row

nife's own FAT12/16/32 read-write crate with long file names, proven with Kani because a USB stick
is hostile input, checked against rust-fatfs and `fsck_msdos`, and released on crates.io from its own
`nifeos` repository. Milestone 140's FAT32 stratum is built on it. exFAT comes later.
