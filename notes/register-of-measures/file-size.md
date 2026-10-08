# The file-size series

*(Appendix of [the register](../register-of-measures.md). Stem provisional, minted by milestone 841
(a ratchet on Rust file length, and its dashboard row) on 2026-10-08.)*

One row per ISO week, read from that week's commit by `script/metrics`, restated like every other
series. Six columns in `notes/project-metrics/file-size.csv`:

| column | what it is |
|---|---|
| `files_over_2000` | tracked `.rs` files outside `vendor/` over 2,000 lines |
| `lines_share_over_2000_pct` | those files' share of all Rust lines |
| `largest_file`, `largest_file_lines` | the largest Rust file, and its line count |
| `file_lines_p95` | the 95th percentile of file length, nearest rank |
| `lines_share_over_1000_pct` | the share of all Rust lines in files over 1,000 |

The chart on the deck plots the share over 2,000, because one split moves it. The count moves only
when a file crosses the line, and the percentile barely moves at all.

## The measure counts comments on purpose

A file's size is its physical line count, `wc -l`. Comments, blank lines and inline `mod tests` are
counted. That is §266 (a Rust source file stays under 2,000 lines) section 1's own ruling: a reader
loads every line of a file, and comments are 41% of the non-blank Rust here. A measure that skipped
them would hide most of what the reader pays.

That choice creates an incentive, and §266 names it where the rule is. A file at 2,050 lines can
pass by deleting 50 lines of comments, which would make the tree worse to buy a number. No gate can
tell a stale comment from a load-bearing one, so that is a review question. The ratchet's failure
message says the remedy: split the file along a seam.

## The ratchet beside it

The ceiling is held by a different instrument: `helpers/file_length_ratchet.py` in `script/lint`,
against `design/file-length-baseline.tsv`. Its entries are ceilings that only fall, which calef
ruled on 2026-10-08 as §266 section 3's option (b). The dashboard reads the tree, never the list,
so an entry left above its file's real size reaches no number anyone quotes.

## BUGS

- `largest_file` is a path, so a rename rewrites the cell without moving the size. Nothing joins
  on it.
- The 95th percentile is read over every Rust file, so a week that adds many small files moves it
  down without any file shrinking.
