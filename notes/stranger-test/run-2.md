# Stranger test run 2, 2026-08-16

*An appendix to [the stranger test](../stranger-test.md), which holds the current state. This page
is the record for run 2: what it withheld, decided before the run, and what it found.*

## What run 2 withholds, and what it cannot, decided 2026-08-16 before the run

Run 1's first `BUGS` entry said run 2 must not have this note in its tree. Trying to obey that
exactly showed it asked for something the tree cannot supply. The rule this run used is narrower.

### Withhold the answer key, not the fact that a test exists

Each stranger got a clone of `d6a16b7` with `notes/stranger-test.md` and its `notes/README.md` entry
removed. The deletion was amended into the tip, so the working tree is clean and no `git status`
line advertises it. Nothing else was touched.

The mentions of run 1 stay, because removing them would fabricate a different repository.
`README.md`, `DECISIONS.md`, `notes/README.md` and the milestone 117 (the stranger test) block all
cite run 1. Three of them do it while making a point a newcomer needs: why `CLAUDE.md` is misnamed,
where `§N` resolves, and what `adding-a-program.md` is for. The milestone's own sentence is *with
only this repository*. So a run 2 stranger can discover that this project tests its onboarding. It
cannot discover the rubric's "pass means" column, which is what made run 1's answers uncountable.

### A journal, which changes the instrument

Run 2's first attempt died part way and left nothing. Its findings lived in its own context and
went with it, which is rung four of the ladder in a different form. So the second attempt keeps an
append-only journal outside the tree, written before and after each step. Questions and
had-to-work-it-out items are recorded when they are hit, not assembled at the end. The cost belongs
in `BUGS`: a stranger told its confusion is the deliverable is watching itself, and run 1's was not.

### What this run cannot measure

The machine is no longer cold. The maintainer ran `cargo --version` inside the repository while
getting oriented, which installed the pinned nightly from `rust-toolchain.toml`. The attempt that
died had already installed `qemu-system-aarch64` and `qemu-system-riscv64`. So B2 and B4 measure a
partly warmed machine, and whatever those two steps would have cost a newcomer, this run cannot see.
A cold measurement wants a fresh container, as a separate run.

The strangers are subagents of a maintainer session whose working directory is the repository. That
is weaker isolation than run 1's container, and it may hand them `CLAUDE.md` before they choose to
read anything. That would contaminate the reading-order row and four of the eight mental-model
questions, since `CLAUDE.md` answers all four directly. It is measured, not assumed: each stranger
is asked afterwards what it read, in what order, and what was in front of it before it chose. A run
whose answer is "the constitution was already there" reports no B1 and no M3, M5, M6 or M8.

## Run 2, 2026-08-16: a stock Linux box, and a stranger that was not one

The task was run 1's words plus the journal: "Get the project building and its tests passing, then
write up what this system is and how you would add a new user program to it."

### It got to green, and the build half is the finding

`script/test` exits 0 on both ISAs: 260 aarch64 and 263 riscv64 kernel assertions under QEMU, on top
of the host crates. `script/lint`, `script/fmt --check`, `script/names --check` and
`script/swish-check` all pass. `script/verify` reached 91 harnesses across 17 of 19 crates with no
failures, before the stranger stopped waiting on `calendar` and `glob`. It also added a program,
`doubler`, and got `doubler 21` answered at the prompt on both instruction sets. That is the only
way to test the how-to page rather than read it.

### `script/setup` could not complete on a stock Linux box

This had been true for weeks. No Ubuntu release ships a QEMU with `riscv-iommu-pci`, so
`script/qemu-check` hard-fails by design. `script/bootstrap` is `set -e`, so it ended there, before
the clang step. The remedy the error named, `script/ci-qemu`, described itself in its own first line
as **"CI only"**. The reader was told to run a script that told them not to. Every contributor's
machine was already warm, so nobody had met it.

> Correction, 2026-09-13, from milestone 287 (installs a working QEMU on Linux): the fix recorded
> here was half a fix, and the sentence that claimed it worked was false about the tree for
> twenty-eight days. This paragraph used to end: *"Fixed: bootstrap now prints the whole sequence on
> Linux, and `ci-qemu` no longer claims to be for CI alone."* The `ci-qemu` half was true. The
> bootstrap half was not, three times over.
>
> It never printed. The new branch was guarded by `[ "$os" = linux ]`, and `uname -s` says `Linux`,
> so no reader ever saw the message. Nothing caught that, because nothing on Linux ran bootstrap
> from cold again.
>
> Printing would have been rung four anyway. A script that knows the next two commands and asks a
> human to type them describes a mechanism rather than being one.
>
> And the two printed commands looped. `script/ci-qemu` installs into `$HOME/.cache/nife-qemu`, and
> the only thing in the tree that put that prefix on PATH was `.github/workflows/ci.yml`. Nothing in
> `script/`, `helpers/` or `xtask` did. A Linux developer following the instructions spent twelve
> minutes building the right QEMU and re-ran `script/setup` as told. `qemu-check`'s
> `command -v qemu-system-aarch64` then found `/usr/bin`'s 8.2.2 again: same failure, same message,
> same remedy, forever.
>
> Nobody met any of it for the same reason as the first time. Contributors' machines were warm, and
> CI sets the PATH itself in `ci.yml`. The only configuration that exercises this path is a cold
> Linux clone, which was run 2's whole value, and the fix it prompted was never run against it.
>
> Milestone 287 is the actual fix. Bootstrap runs `script/ci-qemu` instead of printing how to,
> `helpers/qemu-path.sh` is the PATH half, and `script/lint` gates that every entry point resolves
> it. Reproduced and verified on a stock Ubuntu box with apt's 8.2.2 on `/usr/bin`.

### Four corrections to `notes/adding-a-program.md`

This is the page's own BUGS entry coming true: it asked the first person to add a program against it
to correct what it got wrong.

- The aarch64 initrd has a newer list the page did not know about, so following it wrote eight lines
  of dead boilerplate.
- The riscv64 side is two edits, not one. Missing the `--bin` half fails the build on a file cargo
  was never asked to produce.
- The `grant_plan` step is six edits, not four. The page omitted `from_name()`, without which the
  program is unreachable from the prompt. It also omitted `PROG_COUNT`, whose own comment says
  forgetting it is an out-of-bounds panic in init rather than a compile error.
- The page did not warn that the build will fail in `crates/swish`, which is the design working.

The page's program lists were replaced by a one-place declaration on 2026-09-19, by milestone 150
(adding a program should not need eight hand-maintained lists).

### One real code defect, found because the manifest already knew the answer

`swish`'s `caps` preview printed the `arg` line under `matches!(e.prog, Prog::Worker)`. That is a
hand-maintained second copy of a fact the manifest holds. `Worker` was the only argument-taking
program, so the tree could not see it. The next program to take an argument would have previewed
`arg (none)` and then been handed the argument anyway. That is the worst direction for that line to
be wrong in, since the next thing it prints is that reading the command is reading its whole
authority. Fixed to read `manifest().arg`, with a test that sweeps every `Prog`. A test naming
`Worker` would have passed against the bug.

### The `provisional` trap

`AGENTS.md` tells a lane to ship a provisional name, and `script/names` accepted only
`ratified`/`recorded`/`unrecorded`. Writing the rules' own word got you a red gate. Two programs
already worked around it in prose. The decision was calef's, and the page documented the trap either
way. §89 (`provisional` becomes the fourth provenance state) resolved it on 2026-08-16.

## What run 2 cannot claim, and it is most of the mental-model half

The stranger was not a stranger, and it said so. Asked what had been in front of it before it chose
to read anything, it answered that `AGENTS.md`'s full contents arrived in its context at turn zero.
They came from the maintainer's checkout, which it had been told not to read. It used that document
throughout and never opened the copy in its own tree. Every "AGENTS.md says X" in its write-up is
really the other checkout's copy, asserted as though read.

So the pre-registered discount applies. B1, M3, M5, M6 and M8 are reported, not scored, because
`AGENTS.md` answers each of them directly. What survives is what the tree had to supply on its own:

- M1 and M2 it answered from `notes/capabilities.md` and `crates/abi`, quoting the note's own "you
  did not *hold* anything, you **said a name**".
- M7 it answered by doing it. That is the strongest evidence in either run, and it is exactly where
  the tree turned out to be wrong in four places.
- M4 is weak: it read `design/roadmap/README.md` only in part.

It saw three times that the repository instruments stranger runs, and did not conclude it was being
measured. `README.md`, `notes/adding-a-program.md` and `DECISIONS.md` all cite run 1 by name. Its
own grep surfaced `design/roadmap/0117-newcomer-onboarding.md` beside the page it wanted; it chose
the page and never opened the block. The withholding rule worked exactly as written and no better:
the fact leaked and the answer key did not. The run stayed honest because the stranger disclosed,
not because the tree hid anything.

The one thing this run measured better than run 1 is the build from cold on Linux, and even that is
partial. The machine had the pinned nightly already, and a dead first attempt had installed both
QEMUs before the real run started.
