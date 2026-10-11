---
status: PROPOSED
raised: 2026-10-10
---

# 273. The shell program becomes a directory of modules

Raised 2026-10-10 (UTC) as the cut that
[milestone 844 (the shell program is split along its seams)](../roadmap/0844-the-shell-program-is-split-along-its-seams.md)
says is calef's call before a line moves. Written by lane
`milestone/844-the-shell-program-is-split-along-its-seams`. *(Section number provisional until the
merge queue lands it.)*

What is blocked: the whole milestone. Nothing else waits on it.

## What the file is today

`components/src/swish.rs` is 5,920 lines on main at `396187b0b`, 60 fewer than the block measured,
all from the `package` builtin leaving in `7b9d12565`. Every seam after the argv section sits about
60 lines earlier than the block's table. Every seam is between items, so this is a move.

## The recommended cut

The program moves to `components/src/swish/main.rs`, with child modules beside it.
`components/Cargo.toml`'s `[[bin]]` path follows. Every module is a child of the crate root, so
"submodules or siblings" has one answer here. Every module name is provisional.

| file | holds | lines, about |
|---|---|---|
| `main.rs` | header, the capped heap and `mod heap`, clock, `holdings`, the role table and `_start`, `interactive` through `run`, and the helpers four or more modules share | 1,060 |
| `navigation.rs` | `Nav`, the navigation builtins, pattern expansion | 650 |
| `prompt.rs` | `echo`, `apropos`, `read_line`, the shell editing its own line, Tab completion | 420 |
| `running.rs` | argv pages, the words grant, `run_image`, the manifest note, `dir_grant`, `spawn`, draining, `outcome` | 1,160 |
| `caps.rs` | `caps`, `caps_image`, the live generation table, `bare` | 290 |
| `pipeline.rs` | the operators `>`, `<` and `|`, and the files the shell serves behind them | 990 |
| `jobs.rs` | `spawn_interruptible`, `watch`, `forcible`, `reclaim` | 350 |
| `witness.rs` | every witness role and its helpers | 1,090 |

No file is over 1,500, and the parent is well under 3,000. The block asked for every file under
2,000, which this meets.

`crates/swish/src/lib.rs` (3,850 lines) is outside this milestone. The block says so, and no
roadmap block covers it yet. The lane proposes it as a milestone of its own rather than widening
this one.

## What an architect has to rule

1. The witnesses get their own module. Recommended, as the block said. They are one table of
   roles dispatched by `_start`, with no `#[cfg]` anywhere, and the helpers at the end of the file
   (`run_line`, `pwd_is`, `Text`) serve only them. Three `Nav` constructors and methods
   (`rooted`, `rooted_twice`, `name_call`) are witness-only too, and this proposes moving them into a
   second `impl Nav` in `witness.rs`, so the shipping `navigation` module carries no test
   constructors. The reason holds at equal cost.
2. `components/` gets its first program directory. Recommended, as the block said: a bin crate
   root resolves `mod x;` beside itself, which for this file is the directory 59 other programs
   share. The next program over 2,000 lines needs the same answer, and `#[path]` per module is the
   habit rule 7 exists to stop. Nothing else in the tree enumerates `components/src/*.rs`:
   `components/build.rs` does not, and xtask finds the binary by name.
3. Helpers that four or more modules use live in `main.rs`. The status words (`record`,
   `refused`, `failed`, `current`, `last`), the output helpers (`print`, `stage`, `print_num`,
   `put_page`, `get_page`), the file-system window constants, `PAGE`, `memory_region_split`,
   `delegate`, and the window-disjointness assertion, which names windows from three modules.
   Children see the parent's private items, so this saves about 20 visibility edits and makes
   `main.rs` the program's shared vocabulary. Leaving them where they sit today would make
   `navigation` and `prompt` read as libraries the rest of the shell depends on. The reason holds
   at equal cost.
4. Every module name in the table.

If the answer to 2 is no, the program stays one file and the milestone waits for a convention that
covers every program in `components/`.

## What the move costs beyond moving code

- Visibility. About 70 top-level items, 22 `Nav` methods and 17 fields used across module
  lines become `pub(super)`, which at this depth is the same as `pub(crate)`. §271 (the scheduler
  file splits into submodules of `sched`) used `pub(super)`, so this does too. That is about 110
  one-word edits; the shell is a program, so none of it is a public API.
- The alloc gate. `script/lint` reads `components/src/swish.rs` for exactly one
  `extern crate alloc`, four spaces in, inside `mod heap {`. It is rewritten to grep the whole
  directory, require the one hit to be in `main.rs`, and run the same check on it. A missing
  directory gives zero hits and fails closed. A falsification shows an `extern crate alloc` in a
  submodule fails it, as the block asks.
- The comment-block ratchet. The header is one 100-line block, listed at `swish.rs:1`. Git will
  not call a 1,060-line `main.rs` a rename of a 5,920-line file, so the row cannot just move.
  Recommended: the header's paragraphs go to the modules they describe, as each module's `//!`, and
  `main.rs` keeps the part about the program as a whole, the heap and the clock. If what stays is
  still over 40 lines, it falls under the same ratchet question §274 (the filesystem-protocol crate
  root splits into its module files) raises for every split in this batch. No other block in the
  file is over 40.
- The file-length ratchet drops its row for `swish.rs`.
- unsafe. 15 `unsafe` blocks with 15 `SAFETY` comments, all inside items that move whole.
- ELF size. Release builds use 16 codegen units without LTO, and units follow modules, so the
  split can change inlining and size. The shell's ELF is measured before and after on all three
  architectures, and any change is explained.
- Prose. 80 files name the path, 12 with `swish.rs:NNNN` citations. Notes are updated. The 18
  Rust files whose comments name the path cannot change under done-when 2; they are listed for a
  follow-up sweep.
- Proof. The program has no host tests. `script/test` on aarch64, riscv64 and x86_64 runs the
  `system_tests` modules that spawn the shell in each witness role, and `script/swish-check` runs
  too.

## What it does not decide

No public crate API, wire format or syscall changes under any answer here. The role numbers
`system_tests` passes at spawn do not move.
