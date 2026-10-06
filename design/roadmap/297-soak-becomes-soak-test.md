---
status: BUILT
raised: 2026-09-14
built: 2026-09-14
---
# 297. `soak` becomes `soak-test`

Built 2026-09-14. *(Number provisional until the merge queue lands it.)*

calef was shown `script/soak` for ratification and ruled `soak-test`. The maintainer had
recommended keeping `soak`, and this block records that argument because it was a real one and the
next person will want it: `soak` is the field's term **as a noun**, so it already names the activity
where `stranger` alone names a person, which is why `stranger-test` needs its suffix; it composes
into a sibling cargo feature (`reboot_soak = ["soak", "board"]`) that a suffix makes longer; and the
rename reaches 36 `cfg` sites, three console markers and everything that reads them. calef ruled the
other way and the ruling accepts that cost. This milestone performs it; it does not re-argue it.

## The measurement, re-derived at the base commit

`e5bc598c`, by `git show` against the base rather than against the working tree:

| | count |
|---|---|
| `feature = "soak"` sites | **36** (`sched.rs` 19, `main.rs` 9, `thread.rs` 6, `user.rs` 2) |
| `feature = "reboot_soak"` sites | **16** (`soak.rs` 9, `ns16550.rs` 3, `console.rs` 2, `semihosting.rs` 2) |
| files mentioning the word at all | **88** |
| console markers | **3** (`soak:`, `soak-census:`, `soak-reboot:`) |
| code that *parses* a marker | **2** (`board_console`'s `progress.rs` and `lottery.rs`) |
| fixture files carrying a marker | **3** (two captures, one synthetic) |

Two of those differ from the brief's figures and are worth stating because the brief said to
re-derive rather than trust them. **88 files, not 30**: the smaller number was a grep over a subset
of directories. And **the marker readers are two, not four**: `kernel/src/soak.rs` *writes* the
markers rather than reading them, and `xtask/src/main.rs` has 21 `soak: ` strings that are its own
tool-diagnostic prefix (the shape `board-console:` and `job-mix:` already use) and parses
nothing. All four files needed editing; only two of them are recognizers, and the difference decides
what a test can prove.

The `reboot_soak` row is new: the brief counted only the 36. The rename touches **52** feature-gate
sites, not 36.

## The four domains, and one word that had to be decided

| domain | was | is | rule |
|---|---|---|---|
| `script/` entry point | `soak` | `soak-test` | hyphens |
| `cargo xtask` subcommand | `soak` | `soak-test` | mirrors the entry point, as `job-mix` already does |
| cargo feature | `soak` | `soak_test` | a Rust identifier, so `snake_case` |
| composed feature | `reboot_soak` | `reboot_soak_test` | ditto |
| console markers | `soak:`, `soak-census:`, `soak-reboot:` | `soak-test:`, `soak-test-census:`, `soak-test-reboot:` | the command's spelling |

**`reboot_soak_test`, because the head noun is `soak` and not `reboot`.** The feature names *a soak
that reboots*, so renaming the head is the whole change and nothing else about the compound moves.
`soak_test_reboot` was refused for making `reboot` the head, which would name a reboot test that
happens to soak. The hazard its own `[features]` entry insists the name carry (an operator is about
to write a card that makes their board reset itself on a timer) survives: `reboot` is still the
first word read.

The markers moved because calef ratified a rule hours earlier, with `job-mix:` /
`job-mix-census:`: a console marker takes the spelling of the command a reader typed to produce
it, not of the module that implements it, because the reader's path to the string runs through the
command. `soak` was that rule's cited precedent, by accident rather than by argument, since one word
has no seam to show. `soak-reboot:` was not in the brief and follows for the same reason.

## What deliberately did not move

Everything below names **the workload**, and the workload is still a soak. Each is calef's own call
and none was put to him, so this lane left them and proposes the sweep rather than taking it.

- `crates/soak_page`, ratified in its own right on 2026-09-13.
- the program `fixtures/src/soaker.rs`, the kernel module `kernel/src/soak.rs`.
- `notes/soak.md`. Its title is *The workload that does not stop*; it covers `soak.rs`, `soaker`,
  `soak_page` and `Stage::Soak`, of which the command was one line. Renaming it would also break the
  pointer in eleven files under `design/` that a lane may not edit (two closed decisions, eight
  milestone blocks and one proposal, counted rather than estimated), which is the deciding half.
- `board_console`'s `Stage::Soak`, its `--until soak` token, and `BootProgress::soak`.
- `script/board-image --soak`.

The seam this leaves is real and is named in `BUGS`.

## The captures: the brief's premise was false, and correcting it is the useful part

The brief said both QEMU captures can be re-taken, unlike milestone 295's silicon capture, and
freed this lane from teaching the recognizer two spellings. Read rather than assumed, that is wrong,
and `watch.rs` says so in the tree already: *"A fixture is a record of what a machine said, so it is
the one thing here a later mechanism must not update."*

- `qemu-2026-09-01-riscv64-soak.log` is pre-221. It has no `wakes=` field and its
  `crossings=21, wakes=0` is asserted on purpose, as the tripwire for milestone 219's finding that a
  saturated workload does not migrate. A capture taken today has the tick route in it and cannot
  carry that fact.
- `qemu-2026-09-03-riscv64-soak-census.log` is one draw of the placement lottery, whose
  one-clean-core count and 18,963/s `notes/soak.md` cites. A re-take draws a different lottery.

Both are unrepeatable in time rather than in hardware, which is a weaker claim than 295's and a
claim about two files rather than about anything a board will print tomorrow. So:

- The captures are left byte-identical. Rewriting a marker inside `captured/` would be the
  fabricated transcript `design/naming/rename-where-names-hide.md` records.
- The respelling happens at read time, in one function with the argument beside it,
  `board_console::respell_pre_297_markers`, which replaces the marker prefix and nothing else.
- The parser keeps one spelling, unlike 295's, because a permanent second match would pay
  forever for a fact belonging to two files.
- A third capture was taken instead. `qemu-2026-09-14-riscv64-soak-test.log` is
  `script/soak-test --arch riscv64 --for 30s` in this lane's worktree, unedited, green, exit 0. It
  is what keeps the live vocabulary proved against a machine rather than against text this project
  wrote, which is the standard `tests/fixtures/README.md` holds every other marker to. Two new tests
  assert on it, one per recognizer.
- The synthetic fixture moved, because `synthetic/` is hand-written by definition and is not a
  record of anything.

## What it prints

`script/soak-test --arch riscv64 --for 30s`, exit 0:

```
soak-test: started 4 groups of one responder, 3 callers, 1 grinder and 1 tick waiter ...
soak-test-census: where the kernel placed each worker at spawn: R=responder, C=caller, G=grinder, W=tick waiter ...
soak-test-census: core=0 threads=5 W0 W1 W2 G3 W3
soak-test: t=25s beat=5 rounds=274744 rate=11014/s wakes=10032 wakerate=400/s workers=24 refused=0 mismatch=0 stalled=0 drifted=0 crossings=2717 remote=6336 steals=4 deferred=0
soak-test: 274744 round trips in 25s (11014 /s at the last beat), 2717 cross-core handoffs
soak-test: refused=0 mismatch=0 stalled=0 (each must be 0)
soak-test: a clean run is a number to compare against, NOT evidence that the concurrency is correct.
```

Those magnitudes are a Linux container under TCG and are not comparable with anything in
`notes/soak.md`; the run is here to show the vocabulary, not a number.

## BUGS

- Six things still spell `soak` where the command no longer does, listed under "what did not
  move" above, and the seam a reader meets is `script/board-image --soak` producing a card whose
  console says `soak-test:`, and `script/board-console --until soak` matching a line that says
  `soak-test: started`. Each is a name calef has not been asked about, and three of the six
  (`soak_page`, and arguably the module and the program) are defensible on their own terms rather
  than merely unasked. Proposed as its own piece of work:
  `design/roadmap/proposals/the-soak-that-is-not-the-soak-test.md` is **not** written, because a
  developer lane does not add files under `design/` outside its own block; the proposal is in this
  lane's report instead, which is rung four and says so.
- Four files under `design/roadmap/proposals/` still name `soak` and `reboot_soak` as live cargo
  features, and one lists them as a build command that will now fail:
  `board-only-features-nothing-compiles.md` (`board,soak,reboot_soak`),
  `select-the-padding-at-boot-not-at-compile-time.md`, `nothing-in-ci-boots-the-riscv-tour.md`
  (the first two are `design/roadmap/373-board-only-features-nothing-compiles.md` and
  `design/roadmap/379-select-the-padding-at-boot-not-at-compile-time.md` since milestone 433
  numbered them, and both still carry the old spellings in their bodies)
  (`script/soak --arch riscv64`, `--features soak`) and
  `board-console-cannot-speak-to-the-board.md` (`soak-reboot: DISARMED`). All four are `PROPOSED`,
  so by `design/naming.md`'s status table they should have moved. They did not because a
  developer lane edits its own milestone's roadmap block and nothing else under `design/`, which is
  the same exception `design/naming.md`'s own `BUGS` records for milestone 63. The integrator or a
  later lane owns them.
- Nothing in CI compiles a `soak_test` or `reboot_soak_test` card, which is not new
  (`notes/footprint-perturbation.md` records it) but is newly load-bearing: the feature names in
  `script/board-image` are shell strings, so a typo there fails at a bench rather than in a gate.
  `script/lint`'s per-feature clippy loop covers `soak_test` and does not cover `reboot_soak_test`,
  which is riscv64-only and has 16 `cfg` sites. This lane built it by hand
  (`cargo check -p kernel --features reboot_soak_test --target riscv64imac-unknown-none-elf`,
  clean) and that is rung zero. The finding has a home already:
  `design/roadmap/373-board-only-features-nothing-compiles.md` proposes exactly this gate, and
  its example command is one of the four this rename made stale, so whoever picks it up must read
  the new spellings out of `kernel/Cargo.toml` rather than out of that file.
- The rename was run on QEMU and on no board. No VisionFive 2 was at this lane's bench, so
  every claim about what radon prints under the new markers is read from the source.

## Follow-on

- **Recorded.** The six names that still spell the *workload* rather than the command
  (`crates/soak_page`, `fixtures/src/soaker.rs`, `kernel/src/soak.rs`, `notes/soak.md`,
  `board_console`'s `Stage::Soak` and `--until soak`, and `script/board-image --soak`). Each is a
  name calef has not been asked about and this lane refused to decide for him; the seam a reader
  meets is in this block's `BUGS`. This bullet read `**Proposed.**` and named no file, because a
  developer lane does not add files under `design/` outside its own block and the ask sat in the
  lane's report for an integrator to file. Milestone 433 drained the proposal directory to zero on
  2026-09-19, so filing one now would reopen the pile for an item that is a naming decision rather
  than a milestone's worth of work. It is recorded here instead and put to calef directly, which is
  where a naming decision belongs; the six paths above are the whole of it.
- **Recorded.** `design/roadmap/297-soak-becomes-soak-test.md`: four `PROPOSED` proposals still name
  `soak` and `reboot_soak` as live cargo features, and one carries a build command
  (`board,soak,reboot_soak`) that will now fail. By `design/naming.md`'s status table they should
  have moved; a developer lane may not edit them.
- **Milestone 373.** `design/roadmap/373-board-only-features-nothing-compiles.md`, which already
  owns this: nothing in CI compiles a `soak_test` or `reboot_soak_test` card, so the latter's 16
  riscv64-only `cfg` sites are held up by a hand-run `cargo check` and by nothing repeatable. That
  proposal's own example command is one of the four this rename made stale.
- **Done.** The console markers, by calef's 2026-09-14 ruling that a marker takes the command's
  spelling. `crates/job_mix`'s block cites `soak`'s markers as that rule's precedent and has not
  landed yet, so it will cite a spelling that changed under it; noted for whoever lands it.

## Index row

calef was shown `script/soak` and ruled `soak-test`, against a maintainer recommendation to keep it; the losing argument is recorded on the script because it was a real one (`soak` is the field's term as a noun, it composed into `reboot_soak`, and the rename costs 52 feature-gate sites). Four domains moved together, each by its own rule: `script/soak-test`, `cargo xtask soak-test`, the features `soak_test` and `reboot_soak_test`, and the three console markers `soak-test:` / `soak-test-census:` / `soak-test-reboot:`, the last following calef's same-day ruling that a marker takes the spelling of the command a reader typed. `reboot_soak_test` because the head noun is `soak`. The brief's premise about the captures was false and the correction is the useful half: both pre-rename QEMU captures are unrepeatable in time rather than in hardware (one is pre-221 and carries the `crossings=21, wakes=0` tripwire; the other is one draw of the placement lottery), so they are left byte-identical and respelled at read time by one documented function, the parser keeps a single spelling unlike milestone 295's, and a third capture was taken under the new command so the live vocabulary is proved against a machine. Six names that spell the *workload* rather than the command were deliberately left (`soak_page`, `soaker`, `kernel/src/soak.rs`, `notes/soak.md`, `Stage::Soak`, `--soak`) and the seam is in `BUGS`.
