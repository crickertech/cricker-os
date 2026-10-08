---
status: NOT-STARTED
raised: 2026-10-08
milestone_dependencies: none
decision_dependencies: 266
machine_requirements: none
specific_machine: none
needs_person: no
---
# 844. The shell program is split along its seams

*(Minted 2026-10-08 (UTC) by lane split-milestones, filed at calef's approval the same day; number
provisional until the merge queue lands it. Title and slug are drafts.)*

`components/src/swish.rs` is 5,980 lines at `0435a9aeb`. Milestone 70 (`swish`'s remaining logic
in a crate) measured it at 2,625 on 2026-08-02, and it was 3,899 on 2026-09-24. §266 (a Rust source
file stays under 2,000 lines) sets the ceiling and a 2026-12-31 goal of no file over 4,000.
Milestone 840 (the scheduler file is split along its seams) is the model.

The cut and every module name below are calef's call. A lane writes them up as a
`status: PROPOSED` file in `design/decisions/` before it moves a line, and its pull request
carries `needs-architect`.

Reuse: not applicable; this moves code and adds none.

## What is in the file

Measured on `0435a9aeb` from the file's own top-level items. Ranges are approximate at the edges.

| lines | count | what |
|---|---|---|
| 1 to 313 | 313 | docs, the capped heap and `mod heap`, slots, the shell's own clock, `holdings` |
| 314 to 998 | 685 | `Nav`, the navigation builtins, and pattern expansion |
| 999 to 1221 | 223 | `echo`, `apropos`, page and print helpers, `read_line` |
| 1222 to 1447 | 226 | the shell editing its own line, and Tab completion |
| 1448 to 1527 | 80 | the six witness roles and `_start` |
| 1528 to 1703 | 176 | the pipeline, redirection and timing witnesses |
| 1704 to 2205 | 502 | `interactive`, `builtin`, `dispatch`, `xargs`, `user`, `run` |
| 2206 to 3039 | 834 | argv pages, the words grant, `run_image`, the manifest note, `package`, `dir_grant` |
| 3040 to 3424 | 385 | `spawn`, draining, the second stream, the screen signal, `outcome` |
| 3425 to 3726 | 302 | `caps`, `caps_image`, the live generation table, `bare` |
| 3727 to 4713 | 987 | the operators `>`, `<` and `|`, and the files this shell serves behind them |
| 4714 to 5075 | 362 | interruptible jobs: `spawn_interruptible`, `watch`, `forcible`, `reclaim` |
| 5076 to 5980 | 905 | the navigating, two-tree and globbing witnesses |

The largest functions are `navigate` (303 lines), `run_pipeline` (297), `run_image` (190) and
`spawn` (166). Every seam is between items, so this is a move.

The witnesses are 1,081 lines, 18% of the file. They are test roles in the shipping binary, and
`system_tests` spawns the real shell in each role: `shell_navigation_tests`, `glob_grant_tests`,
`multi_dir_namespace_tests` and `pipeline_service`. They need the shell's own `Nav` and spawn
path, so they cannot move to a fixture binary.

## Not the milestone-70 lever

Milestone 70 lifted host-testable logic into `crates/swish`, and the obvious move is to do it again.
It is not available as the main lever. `crates/swish/src/lib.rs` is 3,885 lines and would cross
4,000. And milestone 70 recorded why the rest stayed: `builtin`, `dispatch_one`, `run`, `spawn`
and `pipeline` are capability movement, not logic.

## A proposed cut

Every name here is provisional.

| module, provisional | from the table | about |
|---|---|---|
| `main.rs` (crate root) | 1 to 313, 1448 to 1527, 1704 to 2205 | 900 |
| `navigation` | 314 to 998 | 690 |
| `prompt` | 999 to 1447 | 450 |
| `running` | 2206 to 3424 | 1,220 |
| `caps` | 3425 to 3726 | 300 |
| `pipeline` | 3727 to 4713 | 990 |
| `jobs` | 4714 to 5075 | 360 |
| `witness` | 1528 to 1703, 5076 to 5980 | 1,080 |

A `[[bin]]` crate root resolves `mod x;` beside itself, which for this file is `components/src/`,
where 59 other programs live. So the program moves to `components/src/swish/main.rs`, and
`components/Cargo.toml`'s `path` changes with it. No other program in `components/` has a
directory yet, so this is the first. `#[path]` would avoid the move and is refused: CLAUDE.md's
rule 7 and `script/lint` check 5 are about shared modules, but a `#[path]` per module is the habit
that rule exists to stop.

## What an architect has to rule

1. Whether the witnesses get their own module, or stay beside the code each one exercises. The
   recommendation is their own module: a reader of the shipping shell then never loads them, and
   their role numbers are already one table.
2. Whether `components/` gets its first program directory, or this waits for a convention that
   covers every program. The recommendation is to take it now, since the next program over 2,000
   lines needs the same answer.
3. Every module name in the table.

No public crate API, wire format or syscall changes under any answer here. The role numbers
`system_tests` passes at spawn do not move.

## What moving the code breaks

Found with `grep` at `0435a9aeb`.

- `script/lint` holds that `extern crate alloc` is declared once, inside `mod heap`, so the rest of
  the shell cannot name `Vec::push` (calef's 2026-09-26 ruling under milestone 47 (navigation and naming)). It reads the one
  file. An `extern crate` is legal in any module, so after the split a submodule could declare it
  and the gate would not see it. The check has to scan the directory. This is the one gate the move
  weakens if a lane forgets it.
- `components/Cargo.toml` and `crates/swish/Cargo.toml` name the path in comments, and 70 tracked
  files name it in prose; 20 cite it as `swish.rs:NNNN`.
- The program has no host tests (`test = false`). Its proof is the `system_tests` modules above,
  `script/swish-check`, and `script/test` on three architectures.
- Codegen units follow modules, so a split can change what is inlined and the binary's size. That
  is from memory, not read; measure the ELF before and after.
- 36 first-parent merges in the 14 days to 2026-10-08 touched it.

## Done when

1. `components/src/swish.rs` is gone, and no file under `components/src/swish/` is over 2,000
   lines.
2. No Rust file outside `components/src/swish/` changes.
3. `script/lint`'s alloc check covers the directory, and a falsification shows it fails on an
   `extern crate alloc` in a submodule.
4. `script/test` passes on aarch64, riscv64 and x86_64, and so does `script/swish-check`.
5. The shell's ELF size is recorded before and after, and any growth is explained.
6. Each new module carries a `//! Name:` block marked provisional.

## Index row

The shell program grew from 2,625 lines in August to 5,980, and 18% of it is witness roles that
only tests run. This moves it into a program directory of modules, with the alloc gate widened to
match. The cut, the directory and every name are an architect's call.
