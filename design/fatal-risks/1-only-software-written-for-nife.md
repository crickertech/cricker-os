---
experiment_status: RUN
experiment_run: 2026-08-31
---
# 1. Only software written for nife runs on nife

*Risk 1 of [the nine](README.md). The status vocabulary, the rule an entry meets and the running order are there.*

The claim, stated so it can fail: the platform runs hand-written Rust and nothing else, so every
piece of software anyone wants has to be rewritten. It is the most dangerous entry because it is
structural. Optimization cannot fix "nothing runs here". A system in this state is a research
demonstrator forever, which is not what DECISIONS §14 (a verified-Rust capability microkernel that
runs real workloads) claims.

**The experiment:** milestone 121 (`ripgrep`: enumeration as a capability), for its real dependency
tree, its filesystem walk and its threads.

AMBER since 2026-10-06 (GREEN on all three architectures from 2026-09-16): a stranger's program
builds and runs here unmodified, and has never yet done its job. The blocker was not what anyone
predicted. Unmodified `ripgrep` 14.1.1, forty transitive crates, zero
patches, and three byte-identical transcripts from three separately built binaries
([`notes/ripgrep-on-nife.md`](../../notes/ripgrep-on-nife.md)). What stopped it was the missing
argument vector, milestone 205 (how a foreign program is told what to do). Correction, 2026-09-27:
205 is BUILT, and what keeps `ripgrep` from the prompt now is the 256 KiB image ceiling (#1399).

Correction, 2026-10-06 (UTC), from `notes/ripgrep-on-nife.md`'s own `BUGS` and
`system_tests/src/user/ripgrep_tests.rs`: `rg` has never searched. Of the three things it was chosen
to exercise, it reached one, the dependency tree. It stopped at argument parsing, so its walk never
ran and its threads were never asked for. The walk was run instead by an in-tree `std` walker
(`crates/walk_pricing`), which is our code. Milestones 205 and 206 are BUILT, yet the harness still
starts `rg` with no arguments, so nothing has yet tried a search. When one is tried, PR #1777
measured `rg --threads 1 --no-mmap` over this tree peaking at 3.0 MiB on macOS, against a `std`
heap of about 1 MiB.

Correction, 2026-10-07 (UTC), from milestone 595 (the shell runs a `std` program)'s block and
`script/swish-check` on aarch64, riscv64 and x86_64 (lane `milestone/595-rg-at-prompt`): `rg` has
now searched, from the prompt. Unmodified `ripgrep` 14.1.1, run by its path and unvouched, was
told `rg needle docs ../hay/secret.txt` and printed `docs/n.txt:find the needle here`, the one
match in the directory the word `docs` granted. It could not open `../hay/secret.txt`, which holds
the needle too: the caretaker refused the `..`. The transcript is byte-identical on the three
architectures. The 256 KiB ceiling named above was raised to 4 MiB by milestone 595 on 2026-09-27,
and the two last steps were a manifest note linked into `rg` and a disk copy without its symbol
table. Two limits on what this shows: the lines run only where `helpers/build-ripgrep.sh` was run,
never in CI (§46 (thin primitives or whole subsystems)), and one file in one directory is not the
walk `ripgrep` was chosen to exercise. That walk is measured in the kernel harness,
`notes/ripgrep-on-nife.md`.

Added 2026-10-07 (UTC), from milestone 121 (`ripgrep`: enumeration as a capability)'s block and
`script/swish-check` on aarch64, riscv64 and x86_64 (lane `milestone/121-rg-walk`, #1845): `rg` has
now walked a real tree at the prompt. In `/search`, holding a copy of this repository's `crates/`
(661 files, 275 directories, 8,202,706 bytes, nested five deep), the gate typed
`/installed/rg BUGS crates` with no flags. On each architecture `rg` printed 349 lines from 136
files, and the gate compared them line for line with what `rg` 14.1.1 built for the host printed
over the same copy: no line missing, none extra. The walk took 14.9 s on aarch64 (HVF), 40.8 s on
riscv64 and 345.8 s on x86_64 (both TCG). The same limits as above still hold: it runs only where
`helpers/build-ripgrep.sh` was run, never in CI, and `rg` sits on the data disk rather than being
installed by `jig`. Its peak memory at the prompt is unmeasured; it finished within the `std` heap.

Ruled 2026-10-08 (UTC), on the evidence above. calef: *"I would keep it amber and we prioritize
the path to green."* The verdict stays AMBER. The path to GREEN is three gaps, each with an owner:

1. Nothing re-checked the claim. Owner: lane `milestone/121-rg-ci` (#1853). Its `swish-check` rows
   build `rg` in CI, type the walk on every pull request, and refuse to skip it there.
2. One program is one data point. A second foreign program, unrelated to `ripgrep` and ideally not
   written in Rust, is untested. Open: it waits on calef's choice of program.
3. Threads were never exercised. nife answers `available_parallelism()` with `Ok(1)`, and §105
   declines `std::thread::spawn`, so no foreign program here has run a second thread. Open: it
   waits on a §105 question to calef.

DECISIONS §105 (`std::thread::spawn` stays declined, until a customer needs it) was never reached,
and that reverses the premise. `ripgrep` asks `available_parallelism()` rather than assuming it, and
nife answers `Ok(1)` honestly. A platform answering `Unsupported` there would have failed this
program.

The caveat. The structural fear is retired. The one published argument that speaks to this says it
goes badly: clean-slate kernels have *"significantly fewer features than Linux ... impeding
adoption"*, risk 8's paper ([`notes/incremental-path.md`](../../notes/incremental-path.md)).
[Appendix](somebody-elses-software.md).

2026-10-08 (UTC): threads are to be built (§263 (threads are built), milestone 812
(`std::thread::spawn` runs real threads in one address space)), so `ripgrep`'s threads become
reachable; `fork` is refused and spawn supported (§264 (`fork` is declined for good), milestone 172
(a capability-native subprocess primitive)).

2026-10-10 (UTC): milestone 812 is built. `std::thread::spawn` runs real threads on all three
architectures, and unmodified `rayon` from crates.io runs its parallel sum on every core. That
moves Rust software that uses threads, `ripgrep`'s parallel walk among it, from refused to
reachable. It does nothing for C software, which still waits on a C library with threads (milestone
836 (a C library, stage 2: threads)), so the risk's status is unchanged.
## Benchmarks that inform this risk

As of 2026-10-08 (UTC), under §262 (nife is measured with the field's standard benchmarks).
Each is to run unmodified on nife and on Linux, none has produced a number yet, and every nife
port waits on a C library, which §262 leaves to calef.

- [Milestone 831 (SQLite's speedtest1 on nife and Linux)](../roadmap/0831-sqlite-speedtest1-on-nife-and-linux.md). Needs only a C library.
- [Milestone 833 (the same storage benchmark on nife and Linux, by porting real fio)](../roadmap/0833-the-same-storage-benchmark-on-nife-and-linux.md). Waits on threads too.
- [Milestone 834 (ioping on nife and Linux)](../roadmap/0834-ioping-on-nife-and-linux.md). Needs only a C library.
