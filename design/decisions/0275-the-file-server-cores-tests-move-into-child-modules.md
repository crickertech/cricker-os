---
status: PROPOSED
raised: 2026-10-10
---

# 275. The file server core's tests move into child modules by topic

Raised 2026-10-10 (UTC) as the cut that
[milestone 846 (the file server core moves its tests out)](../roadmap/0846-the-file-server-core-moves-its-tests-out.md)
says is calef's call before a line moves. Written by lane
`milestone/846-the-file-server-core-moves-its-tests-out`. *(Section number provisional until the
merge queue lands it.)*

What is blocked: the whole milestone. Nothing else waits on it.

## What the file is today

`redoxfs_server/src/lib.rs` is 4,785 lines on main at `396187b0b`. Code is lines 1 to 1,738.
`mod tests` is lines 1,739 to 4,785: 3,047 lines, 77 tests.

The block's table is right about the code and wrong in two places about the tests:

- Lines 4,600 to 4,785 are not extended attributes. They are the `CachedDisk` tests
  (`CountingDisk` and 6 tests), which have a doc comment and no banner.
- Three helpers, `image_with_tree`, `server_with_tree` and `list`, sit inside the directory
  capability range but are used by the sync, mtime, rename, statfs and xattr tests. `CountingDisk`
  is used by a directory capability test. They belong in the shared file. Left in a child, `list`
  would also resolve to the crate root's private `fn list` under `use super::*` and fail to build.

## The recommended cut

Tests only. The code stays in `lib.rs` whole. Every file is a child of the existing `tests` module,
so each reaches the crate root's private items (`Server::fs`, `find_store`, `CachedDisk::inner`)
through `use super::*` exactly as today. No item's visibility changes. A scratch crate with the
same module layout confirmed it.

| file | holds | lines, about |
|---|---|---|
| `src/lib.rs` | lines 1 to 1,738, plus `#[cfg(test)] mod tests;` | 1,740 |
| `src/tests.rs` | shared helpers, open, read, write, sync, mtime, the block layer and `CachedDisk` | 690 |
| `src/tests/directory_capability.rs` | the directory capability, milestone 47 (navigation and naming) | 1,140 |
| `src/tests/entries.rs` | rename, statfs, create and truncate | 690 |
| `src/tests/xattr.rs` | extended attributes | 530 |

Submodules of `tests`, not siblings. Siblings at the crate root would each need their own
`#[cfg(test)]` and would reach the shared helpers through `crate::tests::`, which means making the
helpers `pub(crate)`. That is a widening, and the milestone forbids one.

## What an architect has to rule

1. Move the tests and leave the code whole. Recommended, as the block argues: the code is 1,738
   lines, one `impl Server` of 1,086, and one read sees it. Splitting the block cache out to
   `block.rs` is possible and buys nothing the reader needs. This holds at equal cost, so it is not
   about effort.
2. The file names. All provisional. The block proposed `names.rs` for rename, statfs, create
   and truncate. That collides in reading, not in compiling, with the test helper `fn names` used
   by the directory capability tests, so this proposes `entries.rs`. `directory_capability.rs` and
   `xattr.rs` are the block's.

If the answer to 1 is no, the lane writes the code cut up as a second proposal and stops.

## What the move costs beyond moving code

- Test names change. `tests::a_rename_survives_a_reopen` becomes
  `tests::entries::a_rename_survives_a_reopen`, and likewise under `directory_capability` and
  `xattr`. Nothing filters on a module path: `xtask/src/suite.rs` runs the crate's tests unfiltered,
  and the five notes and decisions that name a test use the bare function name, which still works
  as a `cargo test` filter.
- The four `Falsification:` records move with their tests. All are attested with no patch, and
  no tool reads them; their "replay by hand" lines use a substring filter that still matches.
- The ratchet row goes. `design/file-length-baseline.tsv` lists the file at 4,785.
  `helpers/file_length_ratchet.py` fails a row whose file is at or under 2,000 lines, so the row is
  removed in the same change.
- Two notes need the new path: `notes/benchmarks/read-path-block-contract-and-metadata-cache.md`
  and `notes/shell-navigation.md`. The other nine mentions cite code that does not move.
  `design/comment-block-baseline.tsv`'s two rows for this file name lines 1 and 1,619, which do not
  move.
- Proof. The tests are `cfg(test)` and never reach the image, so the release ELF should hash the
  same before and after; the lane records both. Host `cargo test`, clippy and `script/lint` cover
  the rest. The block's done-when also asks for `script/test` on three architectures, which the
  lane runs.

## What it does not decide

No public crate API, wire format or syscall changes under any answer here.
