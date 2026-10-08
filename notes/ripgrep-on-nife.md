# `ripgrep` on nife: what a stranger's program actually hits

*(Milestone 121 (ripgrep), and `design/fatal-risks/README.md` risk 1's decisive experiment. First run
2026-08-31 against `ripgrep` 14.1.1 from crates.io; the search, the refusal and the measurement
2026-10-07 (UTC) by lane `lane/121-ripgrep`.)*

Risk 1 says the platform *"can run hand-written Rust and nothing else, so every piece of software
anyone wants has to be rewritten."* It is the most dangerous of the nine because no amount of kernel
work fixes "nothing runs here". `ripgrep` was chosen as the falsifier because it is not a toy: forty
transitive crates, a filesystem walk, gitignore semantics, memory maps, and threads.

## The answer, in one paragraph

Unmodified `ripgrep` searches a directory it was granted, on all three architectures, with zero
source changes. Told `rg --threads 1 --no-mmap --stats 'walk entry'` over milestone 121's priced
tree, it finds the 138 matches in 141 files that are there, reads all 333,984 bytes, and prints
3,813 bytes that are byte for byte what the same `rg` prints on macOS. Through a grant that lacks
`ENUMERATE` it says the capability does not carry the right, searches nothing, and prints no match.
On the same HVF machine the search takes about 1.5 times what it takes on Linux. It never reaches
DECISIONS §105 (threads), it needs no `--no-mmap`, and it fits in the 256-page heap the harness gives
it with room to spare.

```
`rg --threads 1 --no-mmap --stats 'walk entry'` printed 3813 bytes and spent 65 heap pages
    | 138 matches
    | 138 matched lines
    | 138 files contained matches
    | 141 files searched
    | 3644 bytes printed
    | 333984 bytes searched
```

## How it runs, and where it does not yet

Through the kernel harness, not the prompt. `system_tests/src/user/ripgrep_tests.rs` spawns `rg`
with `fs_service::start_std_narrowed`: a heap budget at slot 0, an output endpoint at slot 1, a
caretaker narrowing `walk/` to the rights under test at slot 4, and the argv at slot 8. The argv is
built from a line typed as it would be at the prompt, by `grant_plan::argv`, the function `swish`
uses, and placed as the progenitor places it: a `READ` frame mapped read-only at `ARGS_PAGE`
(milestone 205 (how a foreign program is told what to do)). The image loads at its own link address
since milestone 206 (a program image has under 896 KiB) moved the ceiling to 496 MiB.

`rg` reaches that archive only after somebody runs `helpers/build-ripgrep.sh`, and no gate does,
because fetching its crate tree is a DECISIONS §46 (thin primitives or whole subsystems) decision. It
is never part of the base image: calef ruled on 2026-10-07 (UTC) that `rg` is installed with `jig`
from a package in nifeos/basalt. `swish` cannot name it, since its program table has no `rg`. So
`rg pattern dir` by name waits on the packaging milestone pull request #1797 mints and on milestone
809 (the package client becomes a program, `jig`) to install it.

By its path it already runs (milestone 595 (the shell runs a `std` program), 2026-10-07 (UTC)).
`helpers/build-ripgrep.sh` links a manifest note into `rg` (`cargo xtask foreign-note`), and
`script/swish-check` copies it to its disk as `installed/rg` without its symbol table, which put it
over the 4 MiB image cap. Typed `/installed/rg needle docs ../hay/secret.txt`, it finds the needle in
the granted `docs` and cannot open the path outside it, on all three architectures. The lines skip
wherever `rg` was not built, which is CI.

## Parity (DECISIONS §19)

| Target | ELF | the search | the refusal | the control |
|---|---|---|---|---|
| `aarch64-unknown-nife` | 5.0 MB | 138 / 141 / 333,984 | refused, 0 searched | found |
| `riscv64-unknown-nife` | 11.5 MB | the same | the same | the same |
| `x86_64-unknown-nife` | 4.4 MB | the same | the same | the same |

Run 2026-10-07 with `script/test --arch <arch> --test ripgrep`, debug kernel under TCG, after
`helpers/build-ripgrep.sh`. All eight tests in the file pass on each. x86_64's OVMF leg has no
RedoxFS disk and skips them, as it skips the walk tests.

The riscv64 ELF is larger from file padding and section alignment rather than code (`.text` 1.23
MiB against aarch64's 1.37). `PT_RISCV_ATTRIBUTES` at virtual address 0 is not a `PT_LOAD`, so the
loader ignores it, as it does for `std_exerciser`.

## Building it

`helpers/build-ripgrep.sh` downloads the published crate and builds every target in one pass, with
no patch, no vendored copy and no fork. What differs from a Linux build is on the command line: the
target spec (`panic=abort`, `singlethread`), `-Zbuild-std` against the patched `std` (notes/std.md),
the link script, and `-Copt-level=s -Cstrip=debuginfo`, because ripgrep's own release profile sets
`debug = 1` and a 25 MB ELF rides into RAM in the initrd. About 25 seconds a target from cold.

Every crate compiles, including the four milestone 64 (enough `std` to run somebody else's crate) flagged (`ignore`, `crossbeam-deque`,
`memmap2`, `walkdir`). Two resolve to non-Unix implementations without being asked: `memmap2`
compiles its `stub.rs`, and `std::sys::args` compiles nife's own `args/nife.rs`.

## Threads: it never asks for one

`ripgrep` does not assume parallelism, it asks. `crates/core/flags/hiargs.rs` sets its default
thread count to `available_parallelism().map_or(1, |n| n.get()).min(12)`, nife's PAL answers
`Ok(1)` (*"the process model is one thread today, and that is an answer, not an error"*), and
`ripgrep` takes its serial walker and searcher. `thread::spawn` is never called. A platform that
answered `Unsupported` there would have failed this program, which is an argument for the
honest-answer posture generally. `--threads 1` is on the measured line anyway, to pin Linux.

## The refusal, through `rg`

```
rg: ./: IO error for operation on ./: this directory capability does not carry the right that verb needs
rg: No files were searched, which means ripgrep probably applied a filter you didn't expect.
...
0 files searched
```

The words after `./:` are the PAL's, for the FS server's `EPERM`: §47 (a directory capability
carries six rights) chose a refusal over an empty listing, and it survives the caretaker, std's
`read_dir` and `ignore`'s walker to reach a person as an error. The control holds the same
`READ | DESCEND` grant and is told `rg 'walk entry' narrow/n000`, and finds its line, so the
refusal is about enumeration alone.

## The heap and `--no-mmap`, decided by measurement

- The 256-page heap is enough. Every `rg` run retyped 65 pages of its budget, on all three
  architectures, whether it searched 141 files, one, or none. std's heap grows geometrically (8, 8,
  16, 32), so `rg`'s heap peaked between 128 and 256 KiB. The next doubling fits and the one
  after it reaches the budget. Most of that
  is startup (argument parsing, the regex compile), not the walk: the one-file control spends the
  same. A tree with very large directories costs more, since `read_dir` reads a listing whole.
- `--no-mmap` need not be a default. The control's line omits it and names one file, which is
  exactly where `ripgrep` tries a memory map. `memmap2`'s stub fails, `grep-searcher` reads instead,
  and the line is found. The flag stays on the measured line only so Linux cannot map either.

## The search against Linux

`rg_search` in `script/bench --real --release --smp` (release kernel under HVF, four cores) runs the
line above six times through the caretaker grant and keeps the median of the last five.
`bench/host/run_linux_rg.sh` boots the Alpine `virt` kernel on the same `virt,accel=hvf` machine,
`-cpu host -m 256M -smp 4`, with `walk_pricing`'s `rg_host` example as PID 1. It stages the same
tree, splits the same line with `grant_plan::each_word`, and runs a static Linux `rg` built from
the same source with the same flags, six times, keeping the last five.

| 2026-10-07, M-series host | `rg`'s own clock, median of 5 | spawn to exit |
|---|---:|---:|
| nife, four boots | 1.24 to 1.33 ms | 1.91 to 2.07 ms |
| Linux (Alpine 3.20 `virt`), four boots | 0.82 to 0.87 ms | 1.23 to 1.44 ms (one boot) |
| macOS, APFS, native, reference only | 4.5 to 9.0 ms | 7.5 to 14 ms |
| `fs_walk` on the same nife boots, for calibration | 0.52 to 0.54 ms | |

About 1.5x Linux. Read it with these caveats:

- The figure is ripgrep's own `--stats` time, from after argument parsing to after the last file,
  so process creation and image loading are out of it on both sides. Spawn to exit includes them
  and is less comparable: nife's is the kernel harness spawning a caretaker and `rg`; Linux's is
  `fork` and `exec` from PID 1.
- Warm on both sides. The first run is untimed and fills nife's FS server memo; Linux's tree is
  on tmpfs.
- Not the same allocator. Linux's `rg` drops upstream's musl-only `jemalloc`, which needs a C
  cross-compiler this machine lacks, and uses musl's malloc. Upstream added `jemalloc` because
  musl's is slow for `ripgrep`, so Linux's figure is if anything pessimistic.
- The machine was loaded. Load averages ran 14 to 76 from other lanes. Two further boots on
  each side landed in a slower mode across every figure at once (nife's `fs_walk` read 0.87 ms
  instead of 0.52, Linux 1.7 ms) and are left out, as notes/walk-cost.md leaves out the same mode.
  Inside that mode the ratio was 1.46.
- Wall time only. The block asked for instruction counts too, and this does not take them.
- One tree, one microkernel. This prices this system's file service, not microkernels.

The walk alone is about 0.1 ms slower than Linux's (`fs_walk` 0.52 ms against 0.41 ms in
notes/walk-cost.md), so most of the 0.45 ms gap is in what `rg` does beyond `walk_pricing`'s walk.
Not measured yet: each directory costs `ripgrep` failed opens for `.gitignore`, `.ignore` and
`.rgignore`, and each is a round trip through the caretaker. The bound grant, which skips the
caretaker, made the walk 0.35 ms, and nothing here runs `rg` through it.

```
helpers/build-ripgrep.sh              # fetch ripgrep 14.1.1, build for all three nife targets
script/test --arch aarch64 --test ripgrep
script/bench --real --release --smp   # rg_search, beside fs_walk
sh bench/host/run_linux_rg.sh         # needs rustup target add aarch64-unknown-linux-musl
```

## BUGS

- **No gate runs any of this.** CI skips every `rg` test because nothing builds `rg` there. The
  numbers above are from one machine on one evening, and the tests are proven only where somebody
  built the binary. A pinned `rg` from the packaging milestone is what would change that.
- **The gap to Linux is not attributed.** The failed ignore-file opens are a guess, not a
  measurement; a request count like the one milestone 606 (a directory walk costs what it does on Linux) took
  with `walk_model` would settle it.
- **Gitignore semantics are untested.** The fixture holds no ignore files, so `ignore`'s matching
  has not been observed here, only its walking.
- **One version, one program.** `ripgrep` 14.1.1 is evidence about this platform, not a survey of
  crates.io.
- **The build script checks no hash.** It pins a version and fetches over the network. It is an
  experiment's apparatus and on no trust path (the initrd's measurement table digests whatever it
  packs), but it is not a supply-chain-safe way to obtain software. Packaging replaces it.
- **The harness's argv page is the kernel's, not the progenitor's.** It writes the same bytes with
  the same function and maps them the same way, but the path a command line takes from `swish`
  through the progenitor is milestone 205's tests, not these.
