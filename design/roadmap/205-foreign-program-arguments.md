---
status: NOT-STARTED
raised: 2026-08-31
milestone_dependencies: none
decision_dependencies: 170
machine_requirements: none
specific_machine: none
needs_person: no
---
# 205. How a foreign program is told what to do

Minted 2026-08-31 from milestone 121's (`ripgrep`: enumeration as a
capability) lane. *(Number provisional until the merge queue lands it.)*

§170 (how a foreign program is told what to do), the decision this block depends on, was decided
by calef on 2026-09-26. Two parts of the build are still an architect's: the
block's layout, which two programs agree on, and the spelling of the mark on a word. The lane ships
each provisionally and brings it to calef as a proposal rather than waiting.

In brief. Unmodified `ripgrep` runs on nife and stops at argument parsing, because
`std::env::args()` compiles std's `unsupported` backend and yields nothing. A stranger's program
reaches its own usage error and can go no further. Everything milestone 121 (`ripgrep` on nife)
still owes is behind this: the confined `rg`, the loud `ENUMERATE` refusal, and the walk benchmark.

## What the ruling asks this milestone to build

§170 has the ruling in full. In short:

1. An argv of plain bytes in one page, carrying no authority. The transport was priced at about 50
   lines and no new syscall in notes/foreign-program-arguments.md (#1314), which also holds the
   layout this milestone proposes.
2. A word that resolves to an existing file is granted that file as the program's manifest
   declares: read-only, read-write, or the file's directory. The manifest travels in the ELF (§197
   (a package is one archive file), M2), which is the proposal
   `design/roadmap/proposals/a-program-carries-its-manifest-in-an-elf-note.md`.
3. A manifest may declare "may create the named path" for a word that does not yet resolve.
4. An unvouched program gets every named file read-only. Read-write and create each need a mark
   on the word, spelled provisionally here.
5. The directories granted on the line bound everything else.

## What stays open

- The layout: where the page sits, `argv[0]`, bytes rather than UTF-8, the 4,080-byte ceiling.
  The note lists the choices.
- The mark's spelling, which is a naming decision.
- Environment variables and exit codes for a foreign program, which §170 does not rule.

## BUGS

- It says nothing about environment variables or exit codes, which are the same family of
  question and arrive right behind it.
- Two code comments still call §170 open, and both change when this is built: the `std_layout`
  arm of the progenitor's spawn path and `StdLayout`'s BUGS in `crates/system_initializer`, and
  `Prog::StdExerciser`'s manifest in `crates/grant_plan`.

## Index row

Minted from milestone 121's lane. Unmodified `ripgrep` runs and stops at argument parsing. The nife
ABI has no argument vector, so `std::env::args()` compiles std's `unsupported` backend and yields
nothing. §170 ruled on 2026-09-26: a byte argv in one page, and authority from the
manifest and the line's directories. Everything 121 still owes is behind this milestone.
