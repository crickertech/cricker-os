---
status: DECIDED
raised: 2026-09-27
decided: 2026-09-27
ratified_by: calef
---

# 240. §46 (thin primitives or whole subsystems): `toml` in `xtask`

*Section number provisional until the merge queue lands it.*

Raised on pull request #1405, milestone 611 (every program and crate belongs to a package, and
every package has a home). It is built as the follow-up to #1396 that moves `packages/*.package`,
`packages/homes` and `packages/*.recipe` to TOML. It needs a §46 dependency, which the maintainer
mints here, per the ruling of §239 (four package kinds, and TOML for package declarations and
recipes) that package declarations and recipes are written in TOML.

## The dependency

`toml` 1.1.6, in `xtask` only, `default-features = false`, features `std`, `parse` and `serde` (the
last brings in `serde_core` for `toml::Table`, not the derive macro or a proc-macro dependency).
The crate is host-only and never ships: `xtask` is the build tool, not part of any image, so this
never reaches the shipping graph §46 is written to gate.

Built dependency graph, six crates: `toml`, `serde_core`, `serde_spanned`, `toml_datetime`,
`toml_parser`, `winnow`. `cargo deny check licenses bans sources` passes.

## Why (§46's test)

§46 (thin primitives or whole subsystems) asks whether correctness here is won by reading the spec
or by exposure. A TOML parser is the second case. TOML's grammar has enough edge cases (multiline
strings, datetime literals, table/array-of-tables nesting) that a hand-rolled parser earns
correctness the slow way, one bug report at a time, where `toml` has already taken that exposure
across its ecosystem. The rule is the same one §196 (nife carries TLS) used for `rustls`: write the
calendar, take the crypto; here, take the parser for the format calef chose.

Two clauses keep this from being the general case §46 warns against:

- **It is host-only.** Nothing in `xtask` runs on a nife image, so this dependency never has to
  satisfy the kernel's `no_std`, its allocator discipline, or a proof obligation. Rule 6 in
  CLAUDE.md's codebase-rules section ("thin primitives... we write everything in between") is about
  the shipping graph, and a build-time-only tool is not on it.
- The format itself was a decision, not a convenience (§239). TOML won against JSON and YAML on its
  merits, so taking a well-exercised parser for it is not the same move as reaching for a crate to
  avoid writing code that belongs to the project.

The dependency and its reason are recorded a second time, beside the dependency itself in
`xtask/Cargo.toml`, and in `notes/packages.md`.

## Checked

- Local: `script/lint` and `script/fmt --check` exit 0; `cargo test -p xtask package::` passes.
- The package archive is unchanged: `cargo xtask package` gives the same digest for `uptime` from
  the old `.recipe` and the new `.recipe.toml`. The recipe's digest covers the built archive, not
  the recipe file, so the format change does not touch what ships.
- No device code reads a recipe; only `image_catalogue`, on the host, does.

## Reversibility

Host-only tooling dependencies are the cheap end of §46's ladder: nothing outside `xtask` depends
on `toml`'s presence, and swapping parsers would touch one module. What is not cheap is the format
choice underneath it (§239), which this dependency merely serves.
