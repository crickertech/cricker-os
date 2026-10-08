---
status: NOT-STARTED
raised: 2026-10-08
milestone_dependencies: none
decision_dependencies: 266
machine_requirements: none
specific_machine: none
needs_person: no
---
# 847. The swish-check harness is split along its seams

*(Minted 2026-10-08 (UTC) by lane split-milestones, filed at calef's approval the same day; number
provisional until the merge queue lands it. Title and slug are drafts.)*

`xtask/src/swish_check.rs` is 4,576 lines at `0435a9aeb`. It was 1,886 lines on 2026-09-23, when it
was renamed from `shell_check.rs`. §266 (a Rust source file stays under 2,000 lines) sets the
ceiling and a 2026-12-31 goal of no file over 4,000. Milestone 840 (the scheduler file is split
along its seams) is the model, and milestone 365 (`xtask/src/main.rs` is 6,785 lines with no module
structure) is the precedent inside `xtask`.

The cut and every module name below are calef's call. A lane writes them up as a
`status: PROPOSED` file in `design/decisions/` before it moves a line, and its pull request
carries `needs-architect`.

Reuse: not applicable; this moves code and adds none.

## What is in the file

Measured on `0435a9aeb` from the file's own top-level items. Ranges are approximate at the edges.

| lines | count | what |
|---|---|---|
| 1 to 101 | 101 | module docs, imports, the feature probe |
| 102 to 334 | 233 | `swish_check`, the entry; `Line`; the keyboard and after-reboot scripts |
| 335 to 1389 | 1,055 | `SWISH_CHECK_SCRIPT`, 152 rows with 528 comment lines |
| 1390 to 1580 | 191 | per-architecture omissions, timeouts, the leg-cost verdict |
| 1581 to 2229 | 649 | reading a transcript: gauges, markers, the boot claim |
| 2230 to 2491 | 262 | the per-architecture leg and the USB keyboard boot |
| 2492 to 3519 | 1,028 | `swish_check_boot`, one function |
| 3520 to 3996 | 477 | keystrokes, the reboot phase, the graphical launch, the `rg` walk |
| 3997 to 4576 | 580 | `mod tests`, 33 `#[test]` functions |

## Two things that are not code

The script table is data: one row per typed line and the substrings its answer must hold. At
1,055 lines it is 23% of the file. It is not a case for "do not split", because it is the part
lanes edit most. Adding a check line is adding a row, and the module docs say that is the design. A
table in its own file makes that common edit touch a file of rows and nothing else.

`swish_check_boot` is 1,028 lines in one body. Moving it keeps it whole, and every file still comes
in under 2,000. Decomposing it is a refactor, and only `script/swish-check` on three architectures
would prove it. That is follow-on work, not this milestone, for the same reason as milestone 843
(the system-initializer crate root is split along its seams) gives for `boot`.

## A proposed cut

Every name here is provisional. `swish_check.rs` becomes `swish_check/mod.rs`, so `main.rs` and
every caller keep `crate::swish_check::*`.

| module, provisional | from the table | about |
|---|---|---|
| `swish_check` (`mod.rs`) | 1 to 334 | 340 |
| `swish_check::script` | 335 to 1389 | 1,060 |
| `swish_check::verdict` | 1390 to 1580 | 190 |
| `swish_check::transcript` | 1581 to 2229 | 650 |
| `swish_check::boot` | 2230 to 3519 | 1,290 |
| `swish_check::launch` | 3520 to 3996 | 480 |

Tests move beside the code they test. About half of the 33 test the transcript reading, which
takes `transcript` to under 1,000. The checks on the script table itself, such as
`every_spawnable_program_has_a_swish_check_line`, go with `script`.

## What an architect has to rule

1. Whether the script table gets its own module. The recommendation is yes, for the reason above.
2. Every module name in the table.

No public crate API, wire format or syscall changes under any answer here. `xtask` is a binary and
`swish_check` is `pub(crate)`.

## What moving the code breaks

Found with `grep` at `0435a9aeb`.

- `script/falsifications` sets `SWISH_CHECK_SOURCE = "xtask/src/swish_check.rs"` and scans that one
  file for `Falsification:` records above functions (milestone 742 (every test is falsified as
  routine)). After the split it must scan the directory, or the two records go unswept and nothing
  fails.
- The two patches those records name, `xtask/falsifications/swish_check.swish_check_boot.patch` and
  `swish_check.swish_check_leg.patch`, carry the module path `swish_check` in their file names. Their
  diffs patch `crates/system_initializer/src/lib.rs`, not this file, so milestone 843 regenerates
  them and this milestone may rename them. Whichever lands second carries both changes.
- `script/swish-check`, `script/ci-build` and `script/falsifications` name the path in comments,
  28 tracked files name it in all, and 5 cite it as `swish_check.rs:NNNN`.
- 53 first-parent merges in the 14 days to 2026-10-08 touched it, second only to milestone 843's
  file. Every lane that adds a shell check edits it. Take it when few lanes are open in it.

## Done when

1. `xtask/src/swish_check.rs` is gone, and no file under `xtask/src/swish_check/` is over 2,000
   lines.
2. No Rust file outside `xtask/src/swish_check/` changes.
3. `cargo xtask`'s host tests pass, the 33 here among them, with no test body changed.
4. `script/swish-check` passes on aarch64, riscv64 and x86_64.
5. `script/falsifications` finds both records, and both patches apply and fail their lines.
6. Each new module carries a `//! Name:` block marked provisional.

## Index row

The harness behind `script/swish-check` grew from 1,886 lines to 4,576 in two weeks. A quarter of it
is the script table lanes edit to add a check, and one function is 1,028 lines. This moves it into
modules under `xtask/src/swish_check/`, with the falsification sweep widened to find its records.
The cut and every name are an architect's call.
