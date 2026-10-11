---
status: PROPOSED
raised: 2026-10-10
---

# 272. The grant-plan crate root splits into private modules

Raised 2026-10-10 (UTC) as the cut that
[milestone 842 (the grant-plan crate root is split along its seams)](../roadmap/0842-the-grant-plan-crate-root-is-split-along-its-seams.md)
says is calef's call before a line moves. Written by lane
`milestone/842-the-grant-plan-crate-root-is-split-along-its-seams`. *(Section number provisional
until the merge queue lands it.)*

What is blocked: the whole milestone. Nothing else waits on it.

## What the file is today

`crates/grant_plan/src/lib.rs` is 7,034 lines on main at `396187b0b`. Since the block measured it,
the image manifest rules moved out to `image.rs`, a `verbs` module and `Prog::verbs` arrived, and
the Jig program took a row. Measured from the file's own items:

| lines | count | what |
|---|---|---|
| 1 to 93 | 93 | crate docs (69 lines), module declarations, imports |
| 94 to 249 | 156 | the `programs!` macro and the four `const fn` checks its expansion names |
| 250 to 584 | 335 | the `programs!` table, 22 rows |
| 586 to 1325 | 740 | `impl Prog`: `verbs` and `manifest`, one arm per program |
| 1326 to 2140 | 815 | output and input specs, flags, slots, `ArgSpec` through `DirSpec`, `Manifest`, `Runtime`, region pages |
| 2141 to 2682 | 542 | `Command`, `RunSpec`, `Endowment`, the file and directory grants, `Holdings` |
| 2683 to 2972 | 290 | `Refusal` and its sentences |
| 2973 to 3325 | 353 | `tokenize` through `parse_run` |
| 3326 to 3898 | 573 | `plan` and its variants, through `unplaceable` |
| 3899 to 3960 | 62 | byte helpers |
| 3961 to 4052 | 92 | interrupt escalation, §24 (interrupting the foreground process) |
| 4053 to 7034 | 2,982 | `mod tests`, 119 tests |

## The recommended cut

Private modules, children of the crate root, with `lib.rs` re-exporting each public item by name.
Every path a caller uses today still resolves, so none of the 78 Rust files outside the crate
changes. Every module name is provisional.

| file | holds | lines, about |
|---|---|---|
| `lib.rs` | crate docs, module lines, named re-exports, byte helpers | 260 |
| `program.rs` | the `programs!` macro, its four checks, the table | 500 |
| `program/manifests.rs` | `impl Prog`: `verbs` and `manifest` | 745 |
| `manifest.rs` | specs, flags, slots, `Manifest`, `Runtime`, region pages | 820 |
| `command.rs` | `Command`, `Endowment`, the grants, `Holdings` | 470 |
| `refusal.rs` | `Refusal`, `CapKind` | 295 |
| `parsing.rs` | `tokenize` through `parse_run`, and `RunSpec` | 430 |
| `planning.rs` | `plan` through `unplaceable` | 575 |
| `escalation.rs` | §24's two-tier policy | 95 |

Tests go beside the module they test, as a `tests.rs` child, which sees that module's private
items. Planning's 1,600 lines of tests are three files:

| file | lines, about |
|---|---|
| `test_support.rs` (crate level, `#[cfg(test)]`) | 270 |
| `planning/tests/designation.rs` | 530 |
| `planning/tests/streams.rs` (operators, `2>`, the draining lane) | 510 |
| `planning/tests/operands.rs` (`rm`, globbing, words) | 560 |
| `parsing/tests.rs` | 540 |
| the image tests, into `image.rs`'s own `tests` | 210 |
| `manifest/tests.rs`, `program/tests.rs`, `command/tests.rs`, `escalation/tests.rs` | 110, 110, 90, 50 |

No file is over 1,500, and `lib.rs` is far under the block's 2,000.

## What an architect has to rule

1. Private modules with named re-exports. Recommended, as the block said: the public API does
   not change at all, and `helpers/interface_stability.py`, which fingerprints this crate as a
   contract, records no churn. Public modules would add a second path to every item, a public API
   change and a fork of its own. Named re-exports rather than `pub use m::*`, so the surface stays
   a list a reviewer can read. Both hold at equal cost.
2. The program table and its manifests stay one module, in two files. They are one edit when a
   program is added (`notes/adding-a-program.md`), which is the block's case for keeping them
   together. Together they are 1,250 lines, under 1,500, but each new program adds 35 to 45, so
   there is room for about six more. `program.rs` holds the macro and table, and the `impl Prog`
   arms go to a child file, `program/manifests.rs`. An inherent `impl` can live in a child
   module, so this is one module to a reader and two files on disk. Recommended now rather than
   at the seventh program. Not about effort.
3. `RunSpec` moves to `parsing`. Its only constructor is `parse_run`, which fills six private
   fields with a struct literal. In `command` with the other types, those six fields become
   `pub(crate)`. In `parsing`, nothing widens. Recommended; it holds at equal cost.
4. Every module name. The block's, with `parsing` and `planning` not `parse` and `plan` because
   those are the crate's two public functions. New here: `program/manifests.rs`, `test_support`, and
   the three planning test files.

If the answer to 1 is "public modules", the lane writes that up as its own proposal and stops.

## What the move costs beyond moving code

- Visibility. One item: `nav_refusal`, already `pub(crate)` and called as `crate::nav_refusal`
  from `expand.rs`, gets a `pub(crate) use` in `lib.rs`. With `RunSpec` in `parsing`, nothing else
  in production code widens. The three program-table checks the survivor-triage tests use are private
  to `program`, so those tests sit under `program`.
- Shared test helpers. `plan`, `plan_against`, `WITH_DIR`, `plan_line` and `refused` are used
  across test groups. The test-local `plan` and `plan_against` shadow the crate's public ones under
  `use super::*`, and glob-importing both from two places is an ambiguity error. Each test file
  imports helpers from `test_support` by name. Test names gain a module segment; nothing filters on
  a `grant_plan::tests::` path.
- The `programs!` macro moves with its table and the four `const fn`s its expansion names, macro
  first. It is not exported and nothing outside the file uses it.
- Intra-doc links. About 120 bare-name links point into another module. Those not covered by a
  `use` the code already needs get a `crate::` path, not a doc-only import, which would trip
  `unused_imports`. The 39-line doc block on `check_chain` must stay at 40 lines or fewer after
  any rewrap.
- The ratchets. `design/file-length-baseline.tsv` drops its row. The crate doc is the file's only
  comment block over 40 lines and stays at line 1 at 69 lines, so the comment-block row holds and
  no over-cap block lands in a new file. Each new `//! Name:` block stays under the cap.
- Notes. `notes/adding-a-program.md` tells a newcomer to add a row to `programs!` in `lib.rs`; it
  is updated to `program.rs`. The other 15 or so mentions are prose, and the `lib.rs:NNNN`
  citations among them were stale already.
- Nothing else keys on the file. The crate's one falsification patch targets `job_windows.rs`.
  No Kani harness, symbol-keyed script or test-name filter names it.
- Proof. The kernel calls `grant_plan::argv`, and the shell and other components depend on the
  crate, so `script/test` runs on aarch64, riscv64 and x86_64, with `script/swish-check`, the 119
  host tests, `cargo doc` with no broken link, and `script/lint`.

## What it does not decide

The wire ids in the table move unchanged, rows in the same order;
`the_wire_ids_already_shipped_never_move` says so. No public path, wire format or syscall changes.
