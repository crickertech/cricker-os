# The 2026-10-03 census's survivors, part three

Milestone 637 (triage the crates the 2026-09-21 mutation census measured for the first time),
provisional number, 2026-10-03. Parts [one](census-2026-10-03-triage.md) and
[two](census-2026-10-03-triage-part-2.md) hold the method and the first crates. Every number here is
`script/mutation -p <crate>` on the lane's worktree against the same run,
[37108924347](https://github.com/nifeos/nife/actions/runs/37108924347).

## grant_plan

48 missed: 38 killed, 10 equivalent. The sweep afterwards reports 10 missed.

- The const-evaluated checks (13). `all_distinct_ids`, `all_distinct_names` and `same_bytes` guard
  the program table at compile time and had never been run on a case they refuse. Duplicates are now
  placed at each pair of positions, and strings are compared at equal length with one byte
  different.
- `Flags::try_new` (3). Exactly sixteen letters are allowed, seventeen are not, and a repeated
  letter is refused wherever it sits.
- `image_can_carry` (3). A `std` image is carriable at the baseline and refused for each of
  non-byte output, a memory grant, a domain and the network, one at a time. A mutated `&&` turns
  one refusal into an acceptance only when the others hold, which is why each variant removes just
  one allowance.
- Sizes and sentinels (9). `STD_REGION_PAGES`, `image_region_pages`, `SHELL_BUDGET_PAGES`, the
  spawn sentinels (distinct, and all within eight of `u64::MAX`), and `UNVOUCHED_STD_MANIFEST`'s
  runtime field.
- `check_words` (4). A line that fills the argument page exactly is taken and one byte more is not.
  Two 1,100-byte words fit, which separates the cost of a word (bytes plus four) from four times
  its bytes.
- `is_quoted` at the last slot, `SecondDir::mount`, and a directory kept with its name when a
  smaller name is sorted in front of it (`keep_smallest`) (3).
- `Designation::is_here`, and `NameSet::last` of an empty set (2).

The 10 equivalents:

- `|` as `^` in `WordGrant::rights` (5): the rights bits are disjoint. `1 << 0` as `1 >> 0` (2).
- `while i < ids.len()` as `<=` in the two distinctness checks (2): at `i == len` the inner loop has
  nothing to compare, so the extra pass does no work.
- `keep_smallest`'s `>` as `>=` (1): it differs only for two equal names, and a directory lists a
  name once.

## paging

45 missed: 2 killed, 43 equivalent. The sweep afterwards reports 43 missed.

- Killed (2): `Ia32e::leaf_flags`'s `entry & SW_KERNEL_EXEC != 0`, read as an OR or an XOR, would make
  every supervisor leaf executable. A leaf without the software bit is now checked to be data or
  read-only data, and the same leaf with it to be code.
- Equivalent, `|` as `^` (37): every page-table entry is built by OR-ing an address (masked to its
  own bit range) with flag bits that each sit in a position of their own. This covers the aarch64,
  Sv39, `Ia32e` and VT-d entries and the capability-flag constructors, and no pair shares a bit.
- Equivalent, a shift of zero (6): `1 << 0` as `1 >> 0` and `0b00 << 6` as `0b00 >> 6` are the same
  number.

The OR group is the largest single block of equivalents in the census. Each would stop being
equivalent if the flag words were built from a type that refused overlap, which `Flags` is not
(it holds a raw capability word). Nothing in this tree needs that, so it is a note and not a
proposal.
