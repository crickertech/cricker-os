---
status: PARTIAL
raised: 2026-08-13
milestone_dependencies: 205, 206, 595
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 121. `ripgrep` on nife: enumeration as a capability, and what the walk costs

Minted 2026-08-13 by calef.

Checked 2026-09-26: what is left is `rg`
itself doing the three things this block calls the point, and three milestones stand between.

- It cannot be told a pattern. Milestone 205 (how a foreign program is told what to do) builds the
  answer to §170 (how a foreign program is told what to do). The maintainer reports calef ruled on
  2026-09-26; the ruling was not in the tree when this was written, so this cites the section only.
- Its image does not fit. `rg`'s 1.37 MiB `.text` is over the 896 KiB ceiling, and only the kernel
  harness relinks it at `0x100_0000`. Milestone 206 (a program image has under 896 KiB) is building
  option D of §171 (where a program image starts), an address-space map, and the relink folds into
  it.
- It cannot be started from the prompt, where the confined demonstration has to be typed. That is
  milestone 595 (the shell runs a `std` program).

None of the three is waiting on calef.

Built 2026-08-31 (lane `milestone/121-ripgrep`; notes/ripgrep-on-nife.md). Unmodified
`ripgrep` 14.1.1 builds for all three `*-unknown-nife` triples with zero source changes, loads,
resolves its own directory through a granted capability, and stops at its own "requires at least one
pattern", because `std::env::args()` yields nothing. It never reaches DECISIONS §105 (threads):
`available_parallelism()` answers 1 and it picks its serial walker. Since milestone 303 (a disk the
x86_64 FS service can find) the 62-byte transcript is identical on all three architectures.

Built 2026-09-26, without `rg` (the same lane, second pass; notes/walk-pricing.md). The two
parts of this block that do not need an argument vector, on all three architectures:

- The refusal. `ripgrep_tests::a_walk_without_enumerate_is_refused_rather_than_empty` grants
  `std_exerciser` the priced tree with `READ | DESCEND` behind a caretaker. `read_dir` is refused at
  the root and one level down, a recursive walker in `walkdir`'s shape returns `PermissionDenied`
  rather than zero entries, and a file named two levels down still opens (the control).
- The price. `ripgrep_tests::a_walk_through_a_confined_grant_is_priced` grants the same tree
  with `ENUMERATE | READ | DESCEND`, asserts the walk visits exactly the fixture's 153 entries and
  366 components, and prints the walk split per component, per entry and per KiB. Under HVF with a
  debug kernel: about 42 ms for the whole walk, 15 to 51 us per component, 43 to 85 us per entry
  listed, against 1.85 ms on macOS natively. Path components are not where the time goes; listing
  is, because the server reads every child's node to set `IS_DIR` (recorded in
  `redoxfs_server`'s `read_dir` BUGS).

The walker is `crates/walk_pricing` (provisional name), plain `std::fs` code that runs unchanged on
a host, so the nife walk and the host walk are the same function. The harness gained
`fs_service::start_std_narrowed` (provisional), a `std` program behind a caretaker.

A third "somebody else's real application"
target beside milestone 66's Vaultwarden and milestone 99's git, chosen for a reason neither of those
has: it is the workload that pushes on enumeration, which is the one authority this system treats
as dangerous.

## Why this workload rather than another

Enumeration is authority, and nothing in the tree currently walks. Milestone 40 (documentation as
a system service) met this a week
ago and wrote it down: its search index is built on the host because there is no `readdir` for a
viewer to call, and adding one "would hand a viewer the power to discover what it was not given". A
program that can list a directory can learn what exists, which is a different and larger power than
being able to read a file it was handed.

That is not a wall, it is a right. `ENUMERATE` is one of §47's six directory rights, it is
implemented in `crates/fs_proto`, and its absence answers `EPERM` rather than an empty listing, which
is milestone 42's truthfulness rule applied to the one refusal most easily faked. `rm -r` already
composes it (`REMOVE_TREE = ENUMERATE | DESCEND | REMOVE`).

This block said `rm -r` was the only thing composing `ENUMERATE`, and that was already false when it
was minted (corrected 2026-08-17 by the status-accuracy sweep; the status itself is right and did not
move). At least four other things compose it, and one of them predates this block by ten days: the
shell's glob expander (`components/src/swish.rs:518`, driving `crates/grant_plan/src/expand.rs`, landed
2026-08-03 against this block's 2026-08-13), the SMB server
(`user/src/smb_server.rs:327`), the `std` PAL's `read_dir`
(`patches/std-nife/overlay/std/src/sys/fs/nife.rs:891`), and the survey viewer's kernel-level
`Rights::ENUMERATE` (`crates/system_initializer/src/lib.rs:958`). Things walk, too:
`user/src/smb_server.rs:331` walks to a directory one `OPENDIR` per component, `components/src/rm.rs:160` is
a real recursive stack walk, and `swish` walks components to navigate.

The honest claim is narrower, and it is the one this milestone actually rests on: no recursive walk
is reachable from a `std` program, because the PAL retains no directory handle. That is milestone 122,
which is this block's gate, and the 2026-08-13 correction section below already states it correctly.
Nothing composes `ENUMERATE` to walk a tree it was handed, from somebody else's program, and that
is where the interesting costs and the interesting confinement both live.

## What is already in place, measured rather than assumed

| Piece | State | Where |
|---|---|---|
| the `ENUMERATE` right, and `EPERM` when withheld | built | §47, `crates/fs_proto` |
| rights that cannot be widened by a child | built | §47 |
| `read_dir` bound in the `std` PAL, one level only | built, see the correction below | milestone 64 (enough `std` to run somebody else's crate) |
| `regex`, `walkdir`, `ignore`, `crossbeam-channel`, `memchr` | built with no change | milestone 64's probe of 50 crates |

That last row is the surprising one. Every significant dependency in ripgrep's tree is in 64's
"Built with no change (35)" list, including `ignore`, which is the crate that does the gitignore
parsing and the directory walking and which 64's table annotates "fs plus threads". So this milestone
is closer to a port than to a construction, which is what §46's boundary predicts: the reuse boundary
is the TCB boundary, and userspace should actively prefer porting.

### Correction (2026-08-13), since resolved

This section once said a recursive walk could not work: the `std` PAL held no directory handle, so
a program could list `sub` and not open `sub/name`. `walkdir` and `ignore` compiled and could not
walk. Milestone 122 (a directory handle `std` can hold) closed that on 2026-08-18 on top of
`OPENDIR`, which `fs_proto` already had. The lesson stays: "built with no change" meant compiles,
and only running it found the gap.

## The demonstration

`rg pattern src/`, where the process holds a directory capability over `src/` carrying
`ENUMERATE | READ | DESCEND` and cannot see, name, or read anything outside it. Not by policy and
not by a check the program performs on itself, but because the authority it holds does not reach.
Milestone 48's shell already rebinds what it holds, so the grant is the shell's ordinary act rather
than a special case built for this.

The negative half is the load-bearing one and should be a test, in the shape milestone 108 used: the
same command, run against a directory capability that lacks `ENUMERATE`, must be refused loudly
rather than returning nothing. A search that silently finds no matches because it could not look is
the worst possible failure for this tool, and `fs_proto` already chose `EPERM` over an empty listing
for exactly that reason.

This is a better first demonstration than git for one reason worth stating plainly: everybody has
run grep. The confinement claim needs no explanation to anyone who has ever typed a search.

## The benchmark, which is the part worth the lane

Every `read_dir` is IPC to the filesystem server. A recursive walk is therefore the workload that
most directly prices this system's central architectural bet, and the roadmap does not currently have
that number. ripgrep is a good instrument for it because it is famous for speed, its performance is
publicly compared against other tools, and the comparison against Linux runs the same Rust logic on
both sides rather than two different programs.

What to measure: wall time and instruction count over a fixed tree, and the per-entry cost of the
walk separated from the per-byte cost of the search, because those two exercise different halves of
the system and a single number hides which one is expensive.

The caveat that makes it honest, and it is not small. This kernel has no threads sharing an
address space (§105, `std::thread::spawn` stays declined), so a process is single-threaded, and `ignore`'s parallel walker and
`crossbeam-channel` will build but cannot be used as designed. ripgrep must run with one thread here,
so the Linux side must be `rg --threads 1` or the comparison is a lie. State it next to the number,
the way the map "tie" and the spawn caveat are stated.

An honest loss is a result. §14's framing is that a recorded loss is worth more than an overclaimed
win, and a microkernel paying IPC per directory entry may well lose this one. That is worth knowing
precisely, and it is the input to whether the FS contract wants a batched or streaming listing.

## Prior art

The three questions, against `notes/prior-art.md`'s ecosystems.

Code to use: ripgrep and its dependency tree, unmodified. This is the case §46 (thin primitives or whole subsystems) rule 2 and the
prior-art note's TCB boundary both point at, and 64 already measured that the tree builds.

A design to copy: Fuchsia is the closest existing system, because its directory handles carry
rights and enumeration is one of them rather than an ambient consequence of having a path. Worth
reading for how it handles rights reduction when a handle is re-opened, which is the same question
§47 answers with "a child can never exceed its parent".

A mistake to avoid: the general shape of a Unix `readdir`, where the ability to list follows from
being able to name the directory at all. Redox is the neighbour that keeps Unix ergonomics, and the
thing to take from it is the ergonomics without the ambient reach. Getting this wrong looks like
ripgrep working beautifully and confinement being decorative.

## BUGS

- `ReadDir` reads a listing whole rather than streaming it, which milestone 64's PAL documents as
  a deliberate choice. A directory with very many entries therefore costs memory proportional to its
  size, and a recursive walk meets many directories. Unmeasured, and this milestone is what would
  measure it.
- Single-threaded only, per §105. Any published number that does not say so is dishonest, and any
  comparison that does not pin the other side to one thread is worse.
- ripgrep memory-maps large files by default and this system has no `mmap`, which milestone 99 (`git` on
  nife)'s block also names as a gap. `--no-mmap` is the workaround and its cost is unmeasured.
- The benchmark measures this tree, not a class of systems. One microkernel's IPC cost is not
  "microkernels are slow at walks", and the note that records it must say so.
- `ignore` building is not `ignore` behaving. 64's probe proved it compiles. Whether its
  metadata-heavy paths and gitignore semantics behave identically here is a separate question that
  only running it answers. The correction above is the sharp version of this: it compiles, and it
  cannot walk past one level, and no probe that only builds a crate would have found that.
- A walker's cost is per component, and on 2026-09-26 it stopped being invisible: 15 to 51 us
  per component under HVF with a debug kernel, smaller than a single operation's fixed cost and
  than a listed entry's (notes/walk-pricing.md). `ripgrep` on Linux pays neither.
## Follow-on

- **Milestone 205.** The ABI has no argument vector, which is what stops `rg` after it loads and
  resolves its own directory. `design/roadmap/205-foreign-program-arguments.md` was minted from
  this lane on 2026-08-31 and carries the wire-format fork.
- **Milestone 206.** The 896 KiB image ceiling this lane found became
  `design/roadmap/206-user-image-ceiling.md`, which also owns the mapping error that names an
  overlap rather than a size.
- **Outstanding.** Gated on milestones 205, 206 and 595. The confined demonstration: `rg pattern src/`
  at the prompt, holding `ENUMERATE | READ | DESCEND` over `src/` and nothing else.
  `kernel/src/user/ripgrep_tests.rs`'s `rg` test still asserts only its usage text. Checked
  2026-09-26.
- **Outstanding.** Gated on milestones 205, 206 and 595. The same refusal with `rg` as the walker. The
  property is proven with `walk_pricing::walk` since 2026-09-26; what is missing is `rg` meeting it.
- **Outstanding.** Gated on milestone 205. `rg --threads 1 --no-mmap` against Linux, the search
  half of the benchmark. The harness can run `rg` past the image ceiling, so this needs arguments
  and not the prompt.
- **Done.** 2026-09-26: the walk priced, per component, per entry and per KiB: notes/walk-pricing.md.
- **Recorded.** The walk pricing's own gaps (a debug kernel, no matched-tier Linux run, three samples
  per slope) are in notes/walk-pricing.md's BUGS. The per-entry listing cost is in `redoxfs_server`'s
  `read_dir` BUGS.
- **Milestone 303.** The x86_64 run, done 2026-09-16. Milestone 184 closed the `std` half of this
  gap on 2026-09-14 and `ripgrep` built there; 303 gave the FS service a disk it can find on `q35`,
  and the transcript is the same 62 bytes as the other two architectures.
- **Outstanding.** Directory reading still reads a listing whole rather than streaming it, and the
  memory cost of a deep walk over large directories is unmeasured. Checked 2026-09-03 against the
  filesystem shim under `patches/std-nife/overlay/std/src/sys/`.
- **Recorded.** `mmap` is absent, `memmap2` compiles its stub, and the searcher falls back to reads
  on its own, so the no-mmap cost stays unmeasured until a search actually runs.
- **Recorded.** A process is single-threaded, so any published number must say so and pin the Linux
  side to one thread. `ripgrep` never reaches DECISIONS §105 because the parallelism query answers
  one and it picks its own serial walker.

## Index row

Enumeration is authority: `ENUMERATE` is a §47 (a directory capability carries six rights) right,
and `rg pattern src/` that provably cannot see outside its grant is the confinement claim anyone who
has typed a search understands. Unmodified `ripgrep` 14.1.1 builds and runs on all three
architectures with zero source changes and stops at "requires at least one pattern", because the
ABI has no argument vector. Without `rg`, the refusal is proven (a walk through a grant lacking
`ENUMERATE` is an error, never empty) and the walk is priced: about 42 ms for 153 entries under HVF,
with listing rather than path components as the largest per-unit cost (notes/walk-pricing.md). What
is left is `rg` doing both, gated on milestones 205 (arguments), 206 (the image ceiling) and 595
(the prompt).
