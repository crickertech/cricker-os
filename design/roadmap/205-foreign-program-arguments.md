---
status: PARTIAL
raised: 2026-08-31
milestone_dependencies: none
decision_dependencies: 170
machine_requirements: none
specific_machine: none
needs_person: no
---
# 205. How a foreign program is told what to do

Minted 2026-08-31 from milestone 121's (`ripgrep`: enumeration as a
capability) lane. *(Number provisional until the merge queue lands it.)*

§170 (how a foreign program is told what to do), the decision this block depends on, was decided
by calef on 2026-09-26. Two parts of the build are still an architect's: the
block's layout, which two programs agree on, and the spelling of the mark on a word. The lane ships
each provisionally and brings it to calef as a proposal rather than waiting.

In brief. Unmodified `ripgrep` ran on nife and stopped at argument parsing, because the native ABI
had no argument vector and `std::env::args()` compiled std's `unsupported` backend and yielded
nothing. The argv is now built,
so a stranger's program hears its line. What it may reach with those words is the half still to
build. Everything milestone 121 (`ripgrep` on nife) still owes is behind that half: the confined
`rg`, the loud `ENUMERATE` refusal, and the walk benchmark.

## What the ruling asks this milestone to build

§170 has the ruling in full. In short:

1. An argv of plain bytes in one page, carrying no authority. The transport was priced at about 50
   lines and no new syscall in notes/foreign-program-arguments.md (#1314), which also holds the
   layout this milestone proposes.
2. A word that resolves to an existing file is granted that file as the program's manifest
   declares: read-only, read-write, or the file's directory. The manifest travels in the ELF (§197
   (a package is one archive file), M2), which is the proposal
   `design/roadmap/proposals/a-program-carries-its-manifest-in-an-elf-note.md`.
3. A manifest may declare "may create the named path" for a word that does not yet resolve.
4. An unvouched program gets every named file read-only. Read-write and create each need a mark
   on the word, spelled provisionally here.
5. The directories granted on the line bound everything else.

## What is built (2026-09-26, lane `milestone/205-foreign-program-arguments`)

Clause 1, the transport, on all three architectures. A `std` program run from the prompt hears the
line's words through `std::env::args()`, and nothing it hears is authority.

- `crates/argument_protocol` (provisional) is the page: a magic, a count, a total, then each word
  as a length and its bytes. A page that does not parse whole reads as no arguments.
- `std_runtime_protocol` gains slot 8 and `ARGS_PAGE` (`0x1400_0000`), and the `std` PAL's
  `sys/args/nife.rs` reads the page from there, zero-copy. `cargo xtask std-src` generates the
  layout into the PAL and routes `sys/args` to it.
- `grant_plan::ArgSpec::Words` (provisional) is a manifest saying "my line is my argv".
  `std_exerciser` declares it. The planner then classifies nothing on the line: `-i` and
  `--color=never` are words, not refusals. `grant_plan::argv` writes the page. `check_words` is
  the same walk without a page, run at plan time, so a bad line is refused before anything spawns.
- The shell writes the page into a frame of its own, for a plain line and for each pipeline stage.
  It sends the frame `READ`-only under `spawnproto::ARGS_BIT` (provisional). The progenitor copies
  the frame into a page from the child's region and places it at slot 8. `caps` prints that slot.
- An unquoted pattern on such a line is refused (`Refusal::PatternInArguments`). The words carry
  no authority, so expanding one would grant names and nothing they name. Quoting passes it through.

Proven by `script/swish-check` on every architecture. `std_exerciser one 'two words'` prints
`args ["std_exerciser", "one", "two words"]`. `std_exerciser piped | wc` counts the piped
transcript. `std_exerciser *.rs` is refused with nothing spawned. `caps std_exerciser` shows
`cap 8  frame     args`. Host tests in `argument_protocol` show that every byte round-trips and
that no damage to the header or a length yields a partial argv. `grant_plan`'s tests show that
every word arrives unclassified, past the sixteen tokens `parse_run` keeps.

## What is left

Clauses 2 to 5, the designation half: turning the words that resolve into grants. Nothing below is
built. A `std` program at the prompt still holds no directory, so `rg pattern` hears its pattern and
has nothing to search. The design is
[`proposals/designating-a-foreign-programs-words.md`](proposals/designating-a-foreign-programs-words.md).
It has one fork that is calef's (what a line with no path grants) and one that is a mechanism
choice (how a single `std` directory slot holds words from several places).

## What stays open

- The layout: where the page sits, `argv[0]`, bytes rather than UTF-8, the 4,080-byte ceiling.
  Built as [`proposals/the-argument-page-layout.md`](proposals/the-argument-page-layout.md)
  proposes, and provisional until a ruling.
- The mark's spelling, which is a naming decision. Nothing spells it yet, because it marks a word
  for clause 4, which is not built.
- Environment variables and exit codes for a foreign program, which §170 does not rule.

## Provisional names

`argument_protocol`, `ArgSpec::Words`, `argv`, `check_words`, `ARGS_BIT`, `ARGS_SLOT`,
`ARGS_PAGE`, the magic `nifeargv`, `Refusal::PatternInArguments` and `Refusal::ArgumentsTooLong`.

## BUGS

- It says nothing about environment variables or exit codes, which are the same family of
  question and arrive right behind it.
- **The shell's own words win on a foreign line.** `--mem N` is the shell's grant on any line, so
  `rg --mem 5` is refused as a memory grant the program does not take. `time` and `xargs` go
  down their own paths and send no argv, so `xargs std_exerciser` runs it with none. Quoting
  passes `"--mem"` as a word.
- **A backslash is a pattern byte here**, as it is to `glob`, so a regex such as `\d+` has to be
  quoted. That matches what another shell would do to it unquoted, but it is not what a person
  typing a regex expects.
- **An image run by path hears nothing.** `run_image` sends no argv, because every image runs
  with a native manifest (`INSTALLED_MANIFEST_OF` or `UNVOUCHED_MANIFEST`). A `std` program
  installed as a package cannot be told what to do until a manifest travels in its ELF note
  (`proposals/a-program-carries-its-manifest-in-an-elf-note.md`).
- **Each spawn costs a scratch page or two in the progenitor**: one to read the shell's frame
  and one to fill the child's copy. They come from `supervision_protocol`'s never-reused
  scratch cursor, which milestone 206 (a program image has under 896 KiB)'s block records as unbounded and a draft pull request
  (#1384) proposes to bound.

## Follow-on

- **Proposed.** Clauses 2 to 5, the designation half, in
  `design/roadmap/proposals/designating-a-foreign-programs-words.md`, with calef's one fork (what a
  line naming no file grants) and the recommended mechanism.
- **Proposed.** The page's layout, in `design/roadmap/proposals/the-argument-page-layout.md`.
- **Outstanding.** The mark's spelling (clause 4). Checked 2026-09-26: nothing in the tree spells
  one, because nothing grants a word read-write yet; it arrives with the designation half.
- **Recorded.** Environment variables and exit codes for a foreign program, the `--mem`, `xargs` and image-path
  gaps, and the scratch-page cost, in this block's `BUGS` above
  (`design/roadmap/205-foreign-program-arguments.md`).

## Index row

Minted from milestone 121's lane. §170 ruled on 2026-09-26: a byte argv in one page, and
authority from the manifest and the line's directories. The argv is built: a `std` program at the
prompt hears its line through `std::env::args()`, in a pipeline too. Still to build: the
designation half, which turns a word that names a file into a grant. That half is proposed, with
one fork for calef.
