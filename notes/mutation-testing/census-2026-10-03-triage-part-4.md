# The 2026-10-03 census's survivors, part four

Milestone 637 (triage the crates the 2026-09-21 mutation census measured for the first time),
provisional number, 2026-10-03. Parts [one](census-2026-10-03-triage.md),
[two](census-2026-10-03-triage-part-2.md) and [three](census-2026-10-03-triage-part-3.md) hold the
method and the first crates. Every number here is `script/mutation -p <crate>` on the lane's
worktree against the same run, [37108924347](https://github.com/nifeos/nife/actions/runs/37108924347).
Line numbers in the census list are from the run's tree and have moved since.

## swish

32 missed: 26 killed, 6 equivalent. The sweep afterwards reports 6 missed, and 14 timeouts that the
census did not list (a mutated loop step in `pieces`, `write_found`, `pad`, `completing`,
`command_position` and `split`, which hang rather than survive).

- `PATH_MAX` (4). The longest installed path, three 32-byte fields, fills the array to the last byte.
  A bound too small refuses a legal install; one too large is noticed by the length check.
- `Lister` (8). A row of exactly 78 columns stays one row and one column more wraps. Three names so
  the column carried after the second is tested, and a directory's `/` takes a column.
  An empty listing prints no newline.
- `designation` (5). The first word is the program and an option is the program's: a resolver that
  holds every name makes either mistake show as a grant. `..`, an absolute path and a quoted glob
  are refused before the directory is asked, which a resolver that records its questions proves.
- `write_say`, `write_holdings` and `HEAP_MAX_BYTES` (4). The heap cap prints as 32 KiB in both
  places and the constant is 32 KiB.
- `write_note_asks` (2). A note asking for a word of output or for none says so; bytes is the third
  wording.
- `versions::entry` (1). A comment that reads as two words does not select a program of that name.

The 6 equivalents:

- `write_duration`'s two `%` as `+`. The sum is the remainder plus one whole unit, which adds exactly
  1,000 thousandths, and each digit is taken `% 10`, so the printed fraction is the same.
- `write_refusal`'s empty `NoSuchProgram` arm. Without it the `_` arm runs, and `Prog::from_name` of
  the empty name is `None`, so nothing is written either way.
- `write_preview_rows`'s `n < 10` as `<=`. No slot printed through `cap` is 10: the native ones are
  at most 9 and the `std` ones at most 8. It becomes live if a slot constant moves to 10.
- `command_position`'s `<` or `>` arm. `is_break` counts both as breaks, so the `_` arm scans an
  empty word, which is no prefix word, and answers false as the deleted arm did.
- `Sequence::is_empty` as `false`. A line is always at least one segment, as `len` says.
