---
status: NOT-STARTED
raised: 2026-10-07
milestone_dependencies: 597, 809
decision_dependencies: 151, 195, 219, 250
machine_requirements: none
specific_machine: none
needs_person: no
---
# 810. `ripgrep` is packaged in basalt and installed with `jig`

*(Minted 2026-10-07 (UTC) by lane `package-runtime-requirements` from fork 4 of the proposal
[milestone 849 (a package declares what it needs at run time)](0849-a-package-declares-what-it-needs-at-run-time.md),
pull request #1797. The number is provisional until the merge queue lands it; the title and slug are
drafts.)*

The purpose: a customer runs `rg` at the swish prompt after `jig install ripgrep`. `ripgrep` is a
customer-facing `optional` package that any customer installs from basalt's index. What is excluded
is baking it into the base image. calef, 2026-10-07 (UTC): *"I don't know why ripgrep would be
restricted to test images. We want ripgrep usable by a customer."*

calef ruled on 2026-10-07 (UTC) that packaging `ripgrep` and making it work are two milestones:
*"Should we have a milestone for packaging ripgrep and a milestone for a functional ripgrep, which
seems like what the ripgrep milestone should be."* then *"Yes"*. Then where: *"To be clear, we
should package ripgrep in basalt. That's what it is for."* And how it arrives: *"With a functional
jig, ripgrep should be installed via jig and not part of the base image. It isn't base."*

This is the packaging half. Milestone 121 (`ripgrep` on nife) is the functional half, and it
depends on this one only for running `rg` at the swish prompt.

## What exists, checked 2026-10-07 (UTC)

In nife:

- `helpers/build-ripgrep.sh` downloads `ripgrep-14.1.1.crate` from `static.crates.io` with `curl`
  and checks no hash. It builds for three targets with `-Zbuild-std` against the `nife-dev` farm
  that `cargo xtask std-src` builds, into `target/ripgrep/<triple>/rg`.
- `xtask` packs that file into each architecture's initrd whenever it exists
  (`xtask/src/archive.rs:354`, `:525`, `:653`), so a base image can carry it by accident.
- Only `system_tests/src/user/ripgrep_tests.rs` runs it, by hand-placed capabilities.
- It carries no manifest note, so it is in no package and no recipe.

In `nifeos/basalt`, read through `gh api` on 2026-10-07: `README.md`, `pins.toml` with one entry
pinning `nife` by full commit id, `helpers/pins.py`, and two workflows. `gate.yml` runs the pinned
commit's own `script/ci-build test`. `pin-bump.yml` proposes a new pin daily. The README says
*"basalt has no toolchain of its own. It builds each component at the toolchain pin that component's
commit carries."* There is no recipe, no package build, no checksum check and no published index.

## The work

1. A recipe in `nifeos/basalt`, not in this tree, naming the crate (`ripgrep`), the version
   (`14.1.1`) and a SHA-256 of the `.crate` file. The build refuses a download that does not match.
   crates.io's index carries a `cksum` per version, which the recipe can be checked against
   (unverified). The recipe format is basalt's to choose; nife's `packages/*.recipe.toml` (§195 (a
   reviewed recipe vouches for a package)) is the shape to start from.
2. A build job in basalt. It checks out nife at the commit `pins.toml` names, runs `cargo xtask
   std-src` there for that commit's `nife-dev` farm, and builds `rg` for each target with the flags
   `helpers/build-ripgrep.sh` uses. One build for all three architectures, per §19 (architectural
   parity).
3. The manifest note, written at build time: `runner = elf`, `runtime = Std`,
   `arg = Words(ReadOnly)` and `output = Bytes`, in milestone 597 (a program carries its manifest in
   an ELF note)'s encoding, linked as an object (`-Clink-arg=note.o`, measured by #1319). Nothing
   writes a note for a foreign program yet, so the writer is part of this milestone. It belongs in a
   form basalt can call at the pinned commit, not a copy.
4. A package archive per architecture (§197 (a package is one archive file)), kind `optional`,
   vouched by the recipe's digest line (§195).
5. Publication to basalt's package index at `basalt.nifeos.org` (§250 (an image names its
   distribution's package index)). Basalt publishes nothing today, so the first publish job is part
   of this milestone, or of whichever milestone builds the index first.
6. Installation by `jig install ripgrep` (milestone 809 (the package client becomes a program)), to
   where swish finds an installed program (§219 (how the shell names an installed program to the
   spawner)). It is vouched through the activation set, so it gets its note's grants.
7. Stop packing `rg` into the base image by accident: `xtask` packs it only where a test asks, with
   a comment citing this milestone. That is not a restriction to tests. In the interim before
   milestone 809 works, an image carrying `rg` is a test image and says so; after it, customers get
   `rg` from `jig install`.

## Exit

On aarch64, riscv64 and x86_64: a nife at basalt's pinned commit runs `jig install ripgrep` against
basalt's index, the archive's digest matches the recipe, and `rg --version` at the swish prompt
prints ripgrep's version. A recipe whose checksum disagrees with the download fails basalt's build.
Searching with `rg pattern dir` is milestone 121's exit, not this one's.

Reuse: the build reuses `helpers/build-ripgrep.sh`'s flags and `cargo xtask std-src` unchanged,
and the package format is §197's. `cargo install` and `cargo-binstall` were considered for the fetch.
Neither builds against a custom target's patched `std`, and neither emits a §197 archive, so they
would replace only the download (unverified for `cargo-binstall`'s custom-target support).

## BUGS

- A search may not fit a `std` program's heap. PR #1777 measured `rg --threads 1 --no-mmap` over
  this tree at a 3.0 MiB peak on macOS, against about 1 MiB of `std` heap. That is milestone 121's
  to solve, but `rg --version` passing here says nothing about it.
- basalt's gate is one job for three architectures (its README's BUGS), so a red riscv64 build can
  hide behind a red aarch64 one.

## Index row

The first third-party program nife ships the way every later one will. A recipe in the
distribution names its source by checksum, basalt builds it against a pinned nife, a manifest note
says what it may hold, and `jig` installs it from an index. `ripgrep` already builds and runs on all three
architectures, so what this proves is the path a customer takes, not the port. It is optional, not base.
