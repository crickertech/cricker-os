---
status: BUILT
raised: 2026-10-03
built: 2026-10-03
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 743. No orphaned Rust source

Raised 2026-10-03 (UTC). The number 743 is provisional until the integrator mints it at merge; the
title, the slug and the name `helpers/orphan_rust.py` are drafts that calef has not ratified.

## Why

rustc compiles only the files reachable through `mod` declarations. A file nothing declares is dead
without any error. Commit d0b1ff821 dropped `mod proofs;` from the timetable crate, and its Kani
proofs and their falsification records went stale while every gate stayed green (repaired by #1554).
A test or proof file that nothing compiles is a test that cannot fail, which is fatal risk 3's
subject (design/fatal-risks/README.md).

## What was built

`script/lint` check 14, running `helpers/orphan_rust.py`: a plain parse, no cargo. Every tracked
`.rs` file under a package's directory must be reached from a build-target root (`[lib]`, `[[bin]]`,
`[[test]]`, `[[bench]]`, `[[example]]` paths, and cargo's auto-discovered roots). The edges are
`mod x;` (2018 `x.rs` or `x/mod.rs`, nested and inline), `#[path]` (also in `cfg_attr`) and
`include!` of a `.rs` file. A `#[cfg(...)] mod` counts: the question is whether a declaration exists. An allowlist in
the helper takes one reason per entry, every entry is a finding, and an entry that stops being an
orphan fails the check.

Measured on main at 1a145fcaa: 627 `.rs` files in 106 manifests, zero orphans, about 1 second.
Mutation checked by commenting out `mod note;` in `crates/elf/src/lib.rs`: the check named
`crates/elf/src/note.rs` and exited 1. The selftest (fixtures: reached, orphan, `#[path]`,
`#[cfg(kani)] mod`, nested `x/mod.rs`, an inline `mod`, a `mod` inside a string and a comment, and a
virtual workspace manifest) runs first in lint.

## What it cannot see

- A `mod` declared by macro expansion is not parsed; a file reached only that way reads as an orphan
  and needs an allowlist entry with the reason.
- The 24 `.rs` files under no package (`bench/host/`, `helpers/kani-lint-shim/`,
  `patches/std-nife/overlay/`) are compiled by bare `rustc` or spliced into std. Nothing checks them
  here.
- `autobins = false` and its siblings are not honored; an auto-discovered path counts as a root.

## Follow-on

- **Recorded.** The macro-declared `mod`, the files under no package and `autobins = false` are
  limits of the check, written in `helpers/orphan_rust.py` where a reader meets them. Main has no
  orphans, so nothing was deleted or rewired and the allowlist is empty.

## Index row

A `.rs` file nothing declares is never compiled, so a proof or test in one cannot fail. This check
fails lint on any such file, and it would have caught the dropped `mod proofs;` of d0b1ff821.
