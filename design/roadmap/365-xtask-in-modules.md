---
status: BUILT
raised: 2026-09-03
built: 2026-09-19
---
# 365. `xtask/src/main.rs` is 6,785 lines with no module structure

Built 2026-09-19, on a board thin enough to take it: no other lane running and two pull
requests armed. Filed as a proposal on 2026-09-03 by the milestone 247 sweep, from milestone 130's
block; promoted by milestone 433 on 2026-09-19 and taken the same day. The file was 11,124 lines
when the lane cut its branch, not the 6,785 the title names and not the 10,680 the promotion
measured that morning: it grew 4% in the hours between, which is the argument for the timing rather
than against it.

No decision was owed, and what it needed was a scheduled slot rather than a permission:
`xtask/src/main.rs` was one of the three merge hotspots every lane wires its test into, so a
wholesale restructure conflicts with every branch in flight and had to run when the board was thin.

In brief. Split `xtask/src/main.rs` into modules along the seams it already has, which are the
commands. The compiler verifies the split completely, so the edit is mechanical and its failure
mode is a build error rather than a subtle one. The work is not the thinking, it is the timing.

## What was built

Nineteen modules plus `main.rs`, which keeps the dispatcher and nothing else. The largest file in
`xtask/src/` went from 11,124 lines to 1,866, and `main.rs` itself to 294.

| module | lines | what it is |
| --- | --- | --- |
| `main.rs` | 294 | the dispatcher, the target triples, `build`, `user`, `--hvf` |
| `shell_check.rs` | 1,866 | `shell-check`: the script table, the boot claims, both legs |
| `board.rs` | 1,193 | the real board's console and its U-Boot script |
| `farm.rs` | 1,078 | `std` on the native ABI: the farm, the overlay, the abort sweep |
| `scanout.rs` | 1,059 | the pixels reached the device: the checks and the referee |
| `suite.rs` | 904 | `test`: the host pass, each ISA's kernel leg, the Miri run |
| `archive.rs` | 654 | the three initrds and the declared-program check |
| `uefi.rs` | 594 | the bootable image, and booting it under OVMF |
| `manual.rs` | 551 | the documentation store and the tree-wide search on it |
| `soak.rs` | 509 | the sustained QEMU runs: the soak and the job mix |
| `disk_check.rs` | 485 | the host-side checks against what the guest wrote |
| `bench.rs` | 478 | `bench`, and the comparison against the recorded baselines |
| `stick.rs` | 445 | unchanged; milestone 157's boot stick |
| `disk.rs` | 398 | the images a boot is given |
| `inbound.rs` | 373 | the host-side probe of the guest's listener |
| `boot_check.rs` | 235 | the boot ladder's gate |
| `inspect.rs` | 197 | `gdb`, `objdump`, `image` |
| `icount.rs` | 194 | the instruction-count instrument |
| `measure.rs` | 141 | the measured-boot digest handed to the kernel build |
| `host.rs` | 130 | running host commands, and where the build put things |

The table is the split as measured on 2026-09-19 and keeps the names of that day. `shell_check.rs`
is `swish_check.rs` since 2026-09-23, with the command it serves; the line count is unchanged by the
rename.

### The boundary, and what lost

The commands are the seam, and taking them literally gets most of the file placed. The decisions
worth recording are the four places where something else won.

**Shared machinery got a module when it is a thing, not when it is leftovers.** `scanout.rs`,
`inbound.rs` and `measure.rs` are not commands; they are instruments several commands use, and each
one names what it does. The alternative was a `common.rs`, which is the failure this split exists to
prevent: a grab bag of everything shared is a second hotspot wearing a new name, and every lane
would wire into it exactly as they wire into `main.rs` today.

`host.rs` is the one module named for a category rather than a subject, and it is deliberately
small. `cargo`, `run`, `capture`, `llvm_tool`, `workspace_root`, `bin_elf`, `flag_value`: 130
lines with no state and no policy. It is the residue a split always leaves, and the way to keep it
from growing into a `common.rs` is that it is named for what it holds (running a host command and
finding a build output) rather than for the fact of being shared. If something lands there that is
neither, that is the tell.

The soak and the job mix left the board. Both read as board work because they share a
recogniser with `board-console`, but they run under QEMU and never touch a board, so they are
`soak.rs`. The board module keeps the console and the U-Boot script, which are the two things that
require the hardware.

`initrd_aarch64` moved, and it is the only thing that did. It sat between the UEFI image and the
disks, a hundred lines from the two packers it is a sibling of. Everything else is in its original
order inside its module.

The tests went with the code they test, rather than into a `tests.rs`, which would have been a
fourth hotspot. The 21 host tests are the same 21: the 20 that lived in `main.rs` are
now under five `mod tests` blocks beside the code they exercise, and `stick.rs` keeps the one it
already had.

### The evidence that nothing changed

Behaviour was held fixed and checked rather than asserted:

- The move is provably a move. Every non-blank line of the original `main.rs`, ignoring `use`
  lines and the `pub(crate)` prefixes the split forced, appears in the new files: zero lines
  missing, and the only additions are the module doc comments and the `mod tests` wrappers.
- **`script/fastpath-footprint` is byte-identical on all three ISAs**, before and after, down to
  the per-symbol totals.
- **`cargo xtask` with an unknown command prints the same usage text byte for byte**, which is the
  one output that names every subcommand and flag.
- **The 21 host tests are the same 21 by name**, and `script/*` was untouched apart from one gate
  described below.

Milestone 130's refused `kernel_main` split is the reason that list exists rather than a claim that
it was not needed: that one came out byte-identical and still broke the build, because `cfg`-gated
blocks park early in `arch::halt()`. `xtask` is a host binary built for one target with no such
pattern, and the whole file compiles in one configuration, so the compiler really does verify this
split completely. That is a different situation and not a better instinct.

### Three things the split broke, all of them loudly

The failure mode was the predicted one. All three were build errors, none was subtle:

- **`script/lint`'s host-pass check read `xtask/src/main.rs` by name** for the bare-metal exclusion
  list, which now lives in `suite.rs`. It reads every `xtask/src/*.rs` instead, which is what the
  check always meant.
- Three intra-doc links stopped resolving. A ``[`run`]`` that resolves inside one file does not
  resolve across two. Qualified.
- **A section banner lifted from `//` to `//!` became documentation**, and clippy's `doc_markdown`
  had an opinion about `NIFE_GPU_MON` that it had no standing to have while the same text was an
  ordinary comment.

## Why this matters

The honest answer is that most of this is a readability win, and a readability win in a build tool
is worth less than one in the kernel. Two things make it more than tidiness.

The first is the merge cost, which cuts both ways and is the reason to do it deliberately rather
than never. Every lane that adds a command or wires a test edited the same file, and AGENTS.md's own
measurement of the merge queue found that lane collisions scale through files rather than through
numbers, with this file named as one of the three hotspots. Modules turn most of those edits into
edits of different files. The restructure itself is one large conflict, paid once, in exchange for
the recurring ones.

The second is that a file this size stops being read. Milestone 130 was a duplication sweep and
found its material by reading; the next such sweep pays the same cost again.

## Where it came from

Milestone 130's Follow-on: *"Split `xtask/src/main.rs`, 6,785 lines with no module structure, into
modules. The compiler verifies the split completely so the edit is mechanical; what it needs is a
scheduled slot, because that file is one of the three merge hotspots every lane wires its test into
and a wholesale restructure conflicts with every branch in flight."*

The same block refused a related split for a measured reason worth carrying: splitting
`kernel_main` came out byte-identical and still broke the build, because four `cfg`-gated blocks
park early in `arch::halt()` and one divergent function absorbs the unreachable tail where two do
not. Nothing like that was expected here, since `xtask` builds for the host and has no early-park
pattern, and the reason to run the split as its own commit was that precedent. It is why the move
and the fixups the compiler then demanded are two commits: the first one does not build on its own,
by construction, so a reviewer can read it as "these lines went there" with no visibility change or
import line in the way.

## BUGS

**The module names are provisional** (2026-09-19). A module name is a name, so it is an architect's,
and a lane ships one and says so. Nineteen of them landed in one commit: `archive`, `bench`,
`board`, `boot_check`, `disk`, `disk_check`, `farm`, `host`, `icount`, `inbound`, `inspect`,
`manual`, `measure`, `scanout`, `shell_check`, `soak`, `suite`, `uefi`, and the existing `stick`.
Two are worth a second look. `suite` is the `cargo xtask test` command, named around the fact that
`test.rs` holding a `mod tests` reads badly. `host` is the residue module described above, and its
name is doing more work than the others' because it is what keeps the module from becoming a
`common`.

**`shell_check.rs` is still 1,866 lines**, which is smaller than the file it came out of by a factor
of six and larger than anything else here. It is one command whose script table is 390 lines and
whose two legs are 500 more. It could be split again along the table / legs / claims seam, and that
is a different milestone with a different argument; nothing about this one required it.

The merge cost this avoids in future is paid once here, in full. Any branch in flight that
touches `xtask/src/main.rs` conflicts with this wholesale, and the resolution is to move the change
into the module the code now lives in rather than to merge line by line.

## Follow-on

- **Recorded.** `design/roadmap/365-xtask-in-modules.md`'s BUGS section keeps that the nineteen
  module names are provisional and want calef's ratification. It is not a milestone of its own:
  `script/names --unratified` is a worklist, and an unratified name blocks nobody's build.
- **Recorded.** `design/roadmap/365-xtask-in-modules.md`'s BUGS section keeps that
  `shell_check.rs` is still 1,866 lines, which is the one module a second split could reach. It is
  one command, its script table and its two legs; nothing here required cutting it further and the
  argument for doing so is not this milestone's.
- **Recorded.** Seven citations of `xtask/src/main.rs` survive in `notes/documentation-audit.md`,
  `notes/smb.md`, `notes/architecture-list-sweep.md` and `script/audits`, deliberately: they are a
  captured tool transcript, a struck-through closed row, an SMB prober that no longer exists in
  xtask, and two observations about which file the commit ranking used to put on top. Eighteen that
  direct a reader somewhere were repointed at the module that now holds the symbol.
- **Done.** Milestone 130's Follow-on item, which named this split and said it needed a slot.
  Its sibling refusal, the `kernel_main` split, stands: this one is the case where the compiler
  really does verify the whole thing, and the difference is that `xtask` compiles in one
  configuration with no `cfg`-gated early park.

## Index row

`xtask/src/main.rs` was one file with no module structure, 6,785 lines when this was filed on
2026-09-03 and 11,124 when the lane took it on 2026-09-19. It is now 19 modules plus a 294-line
dispatcher, split along the seams the file already had, which are the commands, with instruments
that several commands share (the scanout referee, the inbound probe, the measured-boot digest) named
for what they are rather than swept into a `common`. The move is provably a move: every line of the
original appears in the new files, `script/fastpath-footprint` is byte-identical on all three ISAs,
the usage text is byte-identical, and the 21 host tests are the same 21 beside the code they test.
Three things broke and all three were build errors, which is the failure mode milestone 130's
refused `kernel_main` split is the cautionary precedent for.
