---
status: DECIDED
raised: 2026-09-19
decided: 2026-09-19
ratified_by: calef
---

# 158. A program is declared once: the archives read `Cargo.toml`, and the shell's table is one macro

By milestone 150 (adding a program should not need eight hand-maintained lists)'s lane on 2026-09-19, as a reversible implementation choice
inside its own milestone (AGENTS.md: a reversible decision is made by whoever is holding the
problem), and recorded here by the maintainer at merge. Two things it touches are calef's and are
**not** decided here; they are listed under "Not decided" below. *(Section number provisional until
the merge queue lands it.)*

## What was decided

Adding a program to this tree meant writing the same fact into eight hand-maintained places, and
three successive stranger-test runs named that as the highest-value defect a newcomer could fix
(milestone 150's block has the history). After this section a program is declared in one place,
its `[[bin]]` block, plus one row in `crates/grant_plan` if the shell can spawn it. Everything
else is generated from those or checked against them.

1. The archive list is the `[[bin]]` blocks. `xtask`'s `declared_programs()` reads the `[[bin]]`
   blocks of `components/` and `fixtures/`, and all three initrd builders pack that list. The
   per-architecture `entries` tables are deleted. The reader refuses a `[[bin]]` key it does not
   know rather than silently dropping a program.
2. Every archive packs every program. When the two tables were deleted they disagreed about
   three programs (`serial_driver`, `jh7110_entropy`, `pmap`) and none of the differences was a
   decision. A test that cannot run on an architecture `skip!()`s with its reason, as before.
3. The shell's program table is one `macro_rules!` declaration (`programs!`, provisional name),
   generating the `Prog` enum, `name()`, `id()`, `from_id()`, `from_name()`, `Prog::ALL` and
   `PROG_COUNT`. `manifest()` stays a hand-written match, because the compiler already demands its
   arm and the arms are the most commented code in the crate (`rustfmt` does not format inside a
   macro).
4. **Wire ids are written, never positional.** The thirteen ids shipped before 2026-09-19 are pinned
   by a test that refuses renumbering and reuse. `PROG_COUNT` is one past the highest id, so
   removing a program leaves a hole rather than renumbering its successors, which would be a
   wire-format change dressed as a deletion.
5. **The count gate is three relationship checks, not a pinned number**: a spawnable program with no
   binary, a program the tree loads by name with no binary, and a spawnable program no
   `SWISH_CHECK_SCRIPT` line runs. A pinned total would be one more hand-maintained number, failing
   on every legitimate addition.
6. `swish`'s exhaustive render match became a wildcard, backed by a test that every program
   answering in words renders its answer. Eleven of its thirteen arms were empty, so the compile
   error asked for a keystroke rather than a decision. This is rung two in place of rung one, on
   purpose, and it is the one place the milestone went down the ladder.

`xtask` now depends on `grant_plan`. That is an in-tree crate with no external dependencies, so it
is not a §46 dependency decision.

## What was considered and why each lost

- A shared crate holding a `const` table of programs. Still a second list beside `Cargo.toml`,
  gated rather than hand-copied, so adding a program stays two edits. It would earn its place only
  if the kernel needed the list at runtime, and everything that loads a program looks it up by name.
- `cargo metadata` instead of reading `Cargo.toml`. Correct by construction, but JSON, and
  `xtask` has no JSON parser; §46 refuses `serde_json` or `toml` for one list. Hand-scanning JSON is
  no less fragile than scanning the four keys this tree writes in a `[[bin]]` block.
- Per-architecture tables gated against each other. They had already diverged without anyone
  deciding it, and the shared table's own comment said not to filter by architecture.
- A derive crate that counts variants (`strum` and the like). A proc-macro dependency in a crate
  the kernel and the progenitor link, for one count, is what §46 exists to refuse.
- A `const` table of `(Prog, id, name)` beside a hand-written enum. Still two lists; a variant
  without a row is the original bug moved one line down.
- `manifest()` inside the macro. One fewer edit, but the edit it removes was never silent, and
  it would take the crate's most-read code out of `rustfmt`'s reach.
- Ids from declaration order. Refused for the wire-format reason in item 4.

## Not decided here, and whose they are

- `PROG_COUNT`'s name. It now means "one past the highest id", not a count of programs. Renaming
  it is a naming decision and calef's (design/naming.md).
- Whether a program may take both an argument and an input. Kept as today's behaviour;
  `design/roadmap/498-a-program-that-takes-an-argument-and-an-input.md` carries the options
  and recommends keeping it.
- Every name the milestone introduced (`programs!`, `Prog::ALL`, `declared_programs`,
  `declared_program_blobs`, `bin_names`, `check_declared_programs`, `PROGRAM_PACKAGES`) is
  provisional.

## Reversibility

The mechanism is code and can be rewritten in an afternoon. The one irreversible fact near it is the
wire ids, and item 4 is what keeps them where they were. Milestone 198 (the package manager) is the
consumer most likely to want this changed, since installing a program is exactly "add a
declaration"; whatever 198 decides about the package format supersedes item 1 without disturbing
item 4.

## BUGS

Recorded beside the feature in `notes/adding-a-program.md`'s `BUGS`: a removed program's
`SWISH_CHECK_SCRIPT` line is caught only by `script/swish-check`; the removal gate is a text scan
and misses names built at runtime; the wire-id pin covers only ids shipped before 2026-09-19; the
`[[bin]]` reader knows four keys; and every archive now carries `pmap`, which nothing spawns.
