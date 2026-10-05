---
status: NOT-STARTED
raised: 2026-09-19
promoted_from: lint-does-not-build-the-uefi-binary
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 508. `script/lint` does not build the UEFI binary, so lint-clean code can fail `script/test`

*(Number provisional until the merge queue lands it.)* Promoted from the
proposal `lint-does-not-build-the-uefi-binary`, filed 2026-09-19, on calef's instruction of
2026-09-20 to give every proposal on `main` a number. The text below is the proposal's own, unedited
except for this paragraph: the argument is its author's and promotion is not the moment to improve
it. Found by milestone 243's lane, which re-applied its loader banner after a rebase, watched
`script/lint` pass, and then watched `script/test`'s x86_64 leg fail to compile the same code.
Reported as *"a real gap in the gate ladder, not just my mistake"*, which is the right reading: a
gate that cannot see a target cannot hold it.

It is a check and its runtime cost, both inside this tree.

## The gap, read from the script

`script/lint`'s clippy stage builds the host workspace and `xtask`, plus the two vendored-ish
workspaces (`tools/redoxfs_host`, `redoxfs_server`). It builds the kernel for each architecture in
its own stage. **Nothing in it builds `uefi_loader` for `x86_64-unknown-uefi`**, the PE/COFF binary
that a stick boots and that milestone 441's stick program writes. The five mentions of `uefi` in the
file are all commentary about mutation coverage, not a build.

So the ladder has a rung missing at exactly the place with the fewest other checks: the loader has
no host tests to speak of (its logic was deliberately lifted into testable crates, which is the
right shape), and the only thing that compiles it today is `cargo xtask uefi-boot`/`uefi-test`
inside `script/test`, which costs an emulator boot.

## Why it matters more now than it did

Three things landed on 2026-09-19 that all run through that binary. Milestone 400 (the shell on the
firmware screen) put a terminal on it. Milestone 441 (the program that makes the stick) has a host
program write that one file to a stick, per architecture. Milestone 243 (a machine with no serial
port has no way to say anything) gave the loader a pre-jump banner and a screen module of its own,
and the riscv64 payload is now produced through `crates/portable_executable`. The loader stopped
being a thin shim while nothing was watching its build.

## What to build

A lint stage that compiles the UEFI binaries for every architecture the tree produces one for, with
clippy's `-D warnings` the way the other stages do. The question worth answering with a measurement
rather than an opinion is how much it costs: `script/lint` is run before every push, so a stage
that adds a minute is a different proposition from one that adds five. Time it against a warm and a
cold target directory and say so in the block.

Two cheaper shapes, if the cost is bad: build only on the architectures whose loader files
changed (a path-triggered stage, which the tree does not do anywhere else and would be a new habit),
or move it into `script/ci-build` so CI holds it and a contributor learns at push time rather than
at commit time. The tree's own precedent argues for the plain version: `script/lint` already builds
three kernels.

## BUGS

- **This proposal does not check whether anything else is unbuilt by lint.** The same question
  applies to every non-host target in the tree, and the honest version of this work starts by
  listing what `script/lint` compiles and comparing it with what `script/test` compiles.
- A gate that only compiles proves compilation. The loader's behaviour is still proved by an
  OVMF boot, and nothing here changes that.

## Index row

`script/lint`'s clippy stage builds the host workspace and `xtask`, plus the two vendored-ish
workspaces (`tools/redoxfs_host`, `redoxfs_server`).
