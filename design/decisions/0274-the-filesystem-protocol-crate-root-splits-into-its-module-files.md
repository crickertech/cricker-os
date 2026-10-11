---
status: PROPOSED
raised: 2026-10-10
---

# 274. The filesystem-protocol crate root splits into its module files

Raised 2026-10-10 (UTC) as the cut that
[milestone 845 (the filesystem-protocol crate root is split along its seams)](../roadmap/0845-the-filesystem-protocol-crate-root-is-split-along-its-seams.md)
says is calef's call before a line moves. Written by lane
`milestone/845-the-filesystem-protocol-crate-root-is-split-along-its-seams`. *(Section number
provisional until the merge queue lands it.)*

What is blocked: the whole milestone. Nothing else waits on it.

## What the file is today

`crates/filesystem_protocol/src/lib.rs` is 5,744 lines on main at `396187b0b`, and the block's
table still holds line for line. Every seam is already an inline `pub mod`. Bodies with their
four-space indent removed: `blk` 103, `fs` 663, `dir` 228, `verb` 431, `dirent` 49, `statfs` 43,
`grant` 87, `nameset` 91, `xattr` 425, `fixture` 1,239. `mod tests` is 1,937 lines, 66 tests. The
234 lines between modules are their `///` docs.

## The recommended cut

Both of the block's steps, in one change. The target here is milestone 840 (the scheduler file is split along its seams)'s: no new file over
1,500 lines and the parent under 3,000. Step 1 alone leaves `lib.rs` at 3,807, which misses it, and
leaves a `tests.rs` of 1,937, which misses it too.

| file | holds | lines, about |
|---|---|---|
| `lib.rs` | lines 1 to 193, each module's `///` docs above its `pub mod x;`, `mod tests;` last | 455 |
| `blk.rs` | `blk` | 105 |
| `fs.rs` | `fs` | 665 |
| `dir.rs` | `dir`, with its three Kani proofs | 230 |
| `verb.rs` | `verb` | 430 |
| `dirent.rs`, `statfs.rs`, `grant.rs`, `nameset.rs` | one module each | 270 in all |
| `xattr.rs` | `xattr` | 425 |
| `fixture.rs` | `fixture` | 1,240 |
| `tests.rs` | the protocol tests: verbs, grants, blk, fs, dir, dirent, nameset, xattr, statfs | 1,350 |
| `tests/fixture.rs` | the tests that witness the fixture's files | 590 |

Files, not new modules: every module keeps the path it has today, so none of the 112 Rust files
outside the crate changes. The only new names are `tests.rs` and `tests/fixture.rs`, both
provisional.

The module docs stay as `///` above each `pub mod x;` in `lib.rs` rather than becoming `//!` in
each file. That keeps the generated copy the same text, and keeps the two over-cap doc blocks
listed in the comment-block ratchet in `lib.rs`, where their rows already are.

The tests stay one child of the crate root, not a `mod tests` inside each module file. Inside a
module file, the generator's truncation would no longer find them once the file is inlined. No test
touches a private item, so nothing changes visibility.

## The generator, and why this is not a plain move

`xtask/src/farm.rs`'s `std_generate_modules` copies `lib.rs` into the std farm as
`sys/pal/nife/fsproto.rs`, stripping `#![...]` lines and truncating at `#[cfg(test)]\nmod tests`.
After the split it would copy `pub mod fs;` with no file beside it, and std would not build.

The change: replace each column-0 `pub mod x;` with `pub mod x {`, the file's lines indented four
spaces, and `}`. Every non-empty body line today is indented at least four, none has trailing
whitespace, and none of the braces has a blank line inside it, so the round trip is exact.
`std_inputs_stamp` hashes every `src/*.rs` of the crate rather than `lib.rs` alone, and
`STD_SRC_PATCH_VERSION` goes from 10 to 11. The generator change is generic, for any job, not
special to this crate.

## What an architect has to rule

1. Move every module, not tests alone. Recommended. Tests alone needs no tooling, but leaves the
   parent at 3,807 and misses the target. Moving `fixture` alone and having the generator drop it
   reaches about 2,565 with no inlining, but changes what std carries, which is a separate change
   with its own reason. Once the generator inlines one module, inlining all of them costs nothing
   more. This holds at equal cost.
2. `fixture` stays in this crate, as `fixture.rs`. This is the block's recommendation and it is
   about effort, as the block said: at equal cost, the test image's names and contents would not
   live in a wire-protocol crate. Moving it changes `filesystem_protocol::fixture` at 185 sites, or
   adds a re-export that makes this crate depend on the new one. Both are a public API change and
   a separate milestone.
3. What "byte for byte" means for the std copy. rustfmt reflows four lines once the indent is
   gone: a `use super::{...}` in `xattr::store`, and `MOTD`, `WRITE_PATTERN` and `crash::INITIAL`
   in `fixture`. They wrapped only because of the indent. The generated `fsproto.rs` then differs
   from today's in whitespace on those four items and nowhere else. Recommended: accept that, and
   show the diff of the old and new `fsproto.rs` in the pull request. The other answer is
   `#[rustfmt::skip]` on four items, which keeps the copy identical and leaves four formatting
   exceptions in the source for a reason a reader cannot see.
4. The comment-block ratchet fails this split as it stands. §267 (a comment states the constraint as it is now) says a new file's blocks meet
   the 40-line cap outright. Four listed blocks would land in new files: three in `fs.rs` (52, 47
   and 70 lines) and one in `verb.rs` (52). The ratchet sees rows "gained" by a new path, and git
   does not call a one-third split a rename. The same is true of every split in this batch and of
   milestone 840's. Recommended: amend §267 so that a row moves with its block when a file is split
   along a seam, at the same ceiling, and the old file's row goes in the same change, the way a
   rename already moves rows. That is one small change to `helpers/comment_block_ratchet.py`, made
   once, in its own pull request, for every split lane. The other answer is to trim each block to
   40 lines first. That is the work §267's sweep exists to do, and doing it inside a split mixes an
   edit into a move that is meant to be checkable as a move. This recommendation is partly about
   effort and says so: at equal cost, trimming first leaves the better tree.

## What the move costs beyond moving code

- Three falsification patches under `crates/filesystem_protocol/falsifications/` patch the
  proofs in `dir`. `script/lint` runs `git apply --check` on each, so all three fail after the move.
  Each is regenerated against `src/dir.rs` without the indent, and shown to apply and to make its
  proof fail again. Their names and the `Falsification:` lines do not change.
- Two bench scripts, `bench/blk-transfer-sweep.sh` and `bench/transfer-size-sweep.sh`, `sed` a
  constant in `lib.rs` with a four-space indent. They point at `blk.rs` and `fs.rs` and lose the
  indent. Each already fails loudly if its edit does not land.
- The ratchets. `design/file-length-baseline.tsv` drops its row for this file. The seven
  comment-block rows for `lib.rs:<line>` move with their blocks: two stay in `lib.rs` at new line
  numbers, the rest follow ruling 4.
- Prose. Four notes name the path and `mutation-triage.csv` has about 27 rows citing it. The
  notes are updated; the triage rows are a historical record and stay.
- Proof. The crate reaches the kernel suite directly and through std, so `script/test` runs on
  aarch64, riscv64 and x86_64, with the crate's 66 host tests, the three Kani proofs, the std farm
  build and `script/lint`.

## What it does not decide

No wire format changes under any answer: no opcode, layout or constant moves value, and no public
path changes.
