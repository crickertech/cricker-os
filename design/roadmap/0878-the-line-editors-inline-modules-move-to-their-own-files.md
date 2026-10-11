---
status: NOT-STARTED
raised: 2026-10-11
milestone_dependencies: none
decision_dependencies: 266
machine_requirements: none
specific_machine: none
needs_person: no
---
# 878. The line editor's inline modules move to their own files

*(Minted 2026-10-11 (UTC) by lane split-milestones-2, filed at calef's approval the same day; number
provisional until the merge queue lands it. Title and slug are drafts.)*

`crates/line_editor/src/lib.rs` is 2,290 lines at `396187b0b`, up from 1,476 on 2026-09-24. §266
(a Rust source file stays under 2,000 lines) sets the ceiling. Milestone 840 (the scheduler file
is split along its seams) is the model for this block.

The cut and every file name below are calef's call. A lane writes them up as a
`status: PROPOSED` file in `design/decisions/` before it moves a line, and its pull request
carries `needs-architect`.

Reuse: not applicable; this moves code and adds none. The seams are already drawn: the file holds
three public inline modules, `proto`, `component` and `handoff`. The last two came with milestone
23 (a capability-routed component OS with live replacement) on 2026-09-27, which is most of the
growth.

## What is in the file

Measured on `396187b0b` from the file's own top-level items. Ranges are approximate at the edges,
where a doc comment belongs to the item below it.

| lines | count | what |
|---|---|---|
| 1 to 144 | 144 | crate docs, `#![no_std]` |
| 145 to 330 | 186 | `pub mod proto`: the terminal contract's opcodes, flags and word packing |
| 332 to 828 | 497 | `Sink`, the limits, `Event`, `LineDisc` and its one `impl` block |
| 830 to 898 | 69 | `RawQueue` |
| 900 to 948 | 49 | `expand_output` and the cursor-move helpers |
| 950 to 1134 | 185 | `pub mod component`: the two capability declarations and their `const` checks |
| 1136 to 1310 | 175 | `pub mod handoff`: the state blob a replacement reads |
| 1312 to 2290 | 979 | `mod tests`, 54 `#[test]` functions |

Of the 54 tests, two test `proto` alone (24 lines) and six test `handoff` (182 lines). The other 46
test `LineDisc`, `RawQueue` and `expand_output`. `component` has no `#[test]`; its checks are
ten `const` assertions inside it, which travel with it.

## The verdict: split along the three modules, and keep the editor's tests beside it

Two axes were weighed.

- Tests only, as in milestone 846 (the file-server core moves its tests out). `lib.rs` would end
  near 1,310 and `tests.rs` near 980. It works, but it leaves three unrelated modules in one file
  and sends their tests away from them.
- By module. `proto`, `component` and `handoff` already have their own `use` lines, doc headers and
  provisional names. Each becomes a file, and `handoff` takes its six tests along. A child module
  reaches its parent's private items, so `handoff` still reads `LineDisc`'s fields and nothing in
  the shipped code widens. `lib.rs` keeps `LineDisc` and its 46 tests and ends near 1,540.

The recommendation is by module. It is the cut the code already made, and it keeps each piece's
tests beside it. `LineDisc` is one type and is not cut. Its tests could leave as well, which would
take `lib.rs` to about 770. Nothing needs that, and it can wait for the file to grow again.

`RawQueue` stays in `lib.rs`. `handoff` saves and restores its private fields, and a sibling module
could not see them without `pub(crate)`.

## A proposed layout

Every name here is provisional. The three modules stay `pub mod` in `lib.rs`, so
`line_editor::proto`, `line_editor::component` and `line_editor::handoff` resolve as today.

| file, provisional | from the table | about |
|---|---|---|
| `lib.rs` | 1 to 144, 332 to 948, the 46 editor tests | 1,540 |
| `proto.rs` | 145 to 330, with its 2 tests | 215 |
| `component.rs` | 950 to 1134 | 185 |
| `handoff.rs` | 1136 to 1310, with its 6 tests | 365 |

Two of the six handoff tests drive a `LineDisc` on the tests' `Screen` model. `Screen` and
`feed_all` become `pub(super)` inside `#[cfg(test)] mod tests`, so `handoff`'s own test module can
reach them. That widens test code only.

## What an architect has to rule

1. By module, or tests only. The recommendation is by module, for the reasons above. They hold at
   equal cost, so the recommendation is not about effort.
2. Whether the editor's 46 tests stay in `lib.rs` or move to `tests.rs`. The recommendation is that
   they stay, since `lib.rs` ends 460 lines under the ceiling.
3. Every file name in the table. The module names are unchanged.

No public crate API, wire format or syscall changes under any answer here.

## What moving the code breaks

Found with `grep` at `396187b0b`. A lane has to carry each one.

- 40 Rust files outside the crate name `line_editor::`, most of them through `proto`. None
  changes, because the module paths do not.
- One falsification patch mentions `line_editor`:
  `system_tests/falsifications/user.raw_mode_tests.a_badged_copy_of_the_terminal_reads_keystrokes_and_cannot_type_them.patch`.
  It diffs the program `components/src/line_editor.rs`, not this crate, and is unaffected. The
  crate has no `falsifications` directory.
- No Kani harness is in the crate, and `kani-reach.yml` does not name it. No `#[path]`,
  `include!` or `include_str!` reaches into it, and `xtask` neither depends on it nor copies it.
- `notes/project-metrics/mutation-triage.csv` keys 12 rows on `handoff::` names, such as
  `handoff::RawQueue::save`. The video terminal's rows for its `script.rs` carry no `script::`
  prefix, which suggests cargo-mutants keys an inline module and not a file module. If so, all 12
  keys lose their prefix once `handoff` is a file, and the scheduled run reports them as
  untriaged. That is inferred from the ledger, not read in cargo-mutants. A lane checks with
  `cargo mutants --list` and rewrites the rows in the same change.
- The module doc comments link by bare name: `component`'s links `proto` and `handoff`'s links
  `handoff::LAYOUT`. As `//!` comments inside the new files they resolve from the module, so each
  needs a `crate::` path.
- `design/file-length-baseline.tsv` lists the file at 2,290. The row goes in the same change.
  `design/comment-block-baseline.tsv` lists only `lib.rs:1`, the crate docs (142 lines), which stay.
- Six tracked files name the path. §227 (how Tab reaches the shell) cites the crate docs. Milestone
  668 (scrollback from the keyboard) points at `csi_final`. `notes/terminal-contract.md` links
  `LineDisc`. All three point at code that stays in `lib.rs`. No
  `lib.rs:NNNN` citation of this file exists.
- 7 first-parent merges in the 14 days to 2026-10-11 touched it.

## Done when

1. `crates/line_editor/src/lib.rs` is under 2,000 lines, and no new file is over 1,500.
2. The crate's 54 host tests and its doc tests pass with no test body changed.
3. No Rust file outside `crates/line_editor/src/` changes, and no shipped item's visibility widens.
4. `script/lint` passes with the file-length row gone, and the mutation ledger's keys match
   `cargo mutants --list`.
5. `cargo doc` for the crate reports no broken intra-doc link.
6. `script/test` passes on aarch64, riscv64 and x86_64.

## Index row

The line editor's file is 2,290 lines, and most of its growth is two modules added inline for live
replacement. This moves `proto`, `component` and `handoff` into their own files, each with its tests,
behind unchanged public paths. `LineDisc` stays whole with its tests in `lib.rs`, near 1,540
lines. The layout and every file name are an architect's call.
