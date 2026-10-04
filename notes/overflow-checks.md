# Overflow checks: what we build, what they find, what they cost

*Provisional name. Milestone 744 (provisional), a measurement asked for by calef on 2026-10-04
(UTC). It changes no profile. The options at the end are calef's to choose between.*

Rust integer arithmetic panics on overflow or wraps silently depending on `-C overflow-checks`,
which Cargo turns on in `dev` and off in `release`. Neither is undefined behaviour (RFC 560), but an
unmeant wrap is a quiet wrong answer.

## What we build today

Measured on `main` at `1a145fcaa`. No `[profile.dev]` or `[profile.release]` table exists in the
root `Cargo.toml` (only per-package `opt-level` overrides), `.cargo/config.toml` passes no
`-C overflow-checks`, and nothing in `script/`, `xtask/` or `.github/workflows/` sets `RUSTFLAGS` or
`CARGO_PROFILE_*` to change it. So every build gets Cargo's default for its profile, and the profile
is what decides.

| Build | Command | Profile | Overflow checks |
|---|---|---|---|
| Kernel and workspace programs, kernel tests, three ISAs | `script/test` (`xtask test`) | dev | on |
| Interactive shell gate, three ISAs | `script/swish-check`, `script/boot-check` | dev | on |
| icount tripwire and bench, three ISAs | `script/icount`, `script/bench --check` | dev | on |
| Host tests, coverage | `cargo test --workspace` | dev (test) | on |
| Kani proofs | `script/verify` | Kani's own | checked by the prover (Kani treats overflow as a failed property) |
| Fuzz targets | `script/fuzz` | release, overridden | on (`fuzz/Cargo.toml`, the one explicit setting) |
| Fast-path footprint gate | `script/fastpath-footprint` | release | off |
| Bench for cross-OS magnitudes | `script/bench --release` (HVF, never in CI) | release | off |
| `swish-check --release` (not in CI) | | release | off |
| What a customer installs | `xtask stick` (always release, calef 2026-10-03), the UEFI loader built around it | release | off |
| `redoxfs_server`, `mkfs` (in every image, in the test boots too) | `redoxfs_server_build`, always `--release` | release (own workspace) | off, even under `script/test` |
| `std_exerciser` and its `std` (in every image, test boots too) | `xtask std-exerciser`, always `--release` | release (own workspace) | off, even under `script/test` |
| `cryptography_exerciser`, `rg` (packed only when a helper ran; not in CI) | `helpers/build-*.sh`, `--release` | release | off |

Three facts fall out of that table.

1. **The premise that checks are off almost everywhere is false.** Every gate that runs the kernel
   runs it with checks on, on all three ISAs, because every gate builds `dev`. The kernel's test
   suite has always been an overflow-checked run.
2. **What ships is the opposite build.** The stick is release, so the kernel a customer boots wraps
   silently where every test of it would have panicked.
3. **Three image programs are never checked anywhere.** The filesystem server and the `std`
   programs build release unconditionally, so even the test boots run them unchecked. The
   filesystem server parses an on-disk format, which is the bug class this flag exists for.

### What an overflow panic does

- Kernel, `kernel/src/panic.rs`: prints `[PANIC] <message and file:line>`, warns if the stack
  canary was smashed, then `arch::halt()` (`wfi` forever). Under `cfg(test)` or
  `feature = "system_tests"` it exits QEMU with a failure status instead. One overflow anywhere in
  the kernel stops the machine, so a checked overflow is a denial of service by construction and a
  wrapped one is whatever the wrong number does next.
- A `no_std` program: `user_mode_runtime::panic_handler!`, from
  milestone 130 (one trap instruction, forty-eight sites), calls `trap()`, a `brk`, `ebreak` or
  `ud2` the kernel turns into a faulting death. No message; the program's supervisor sees a fault.
- A `std` program: `panic = "abort"`, so std's panic hook runs and then `rt::abort()` (the
  nife std port, `patches/std-nife/.../pal/nife/mod.rs`). That std's default hook prints the
  message to stderr before aborting is from memory, not checked here.

## Would it find anything

**History says yes, and it already has, four times.** Every one of these was a real bug that the
dev-profile check (or Kani, which checks the same thing) turned into a loud failure:

| Where | Found by | What release would have done | Record |
|---|---|---|---|
| `argon2` `Params::new`, `p_cost * 8` | host test, dev | wrapped; a later bound check happened to save it | notes/credentials.md |
| compositor `Rect::right`, `x + w` from a client page | host test, dev | wrapped into a wrong clip | notes/compositor-claim-25.md |
| `dtb::Region::end`, `start + size` from the blob | fuzzer (checks on) | handed the frame allocator a quietly wrong memory map | notes/fuzzing.md |
| ACPI DMAR `body[0] + 1` in a `u8` | Kani proof | wrong IOMMU address width | notes/falsification.md |

The `dtb` case is the one to remember: notes/fuzzing.md says in so many words that in release the
wrap "is worse" than the panic.

This run. The dev gates are already checked and `main` is green, so the only code this experiment
newly checks is the three release-only image programs and their `std`. The throwaway branch
`lane/overflow-checks-on` (deleted) set `[profile.release] overflow-checks = true` in the root and
the three standalone workspaces, and ran the full CI gate twice:

| Run | Job | Result | What it was |
|---|---|---|---|
| 37167649858 | `test` | failed | Not an overflow. aarch64's frame ledger kept 26,670 frames against a budget of 26,668; every `std` and `redoxfs_server` test had passed. The job stops at the first leg, so riscv64 and x86_64 never ran. |
| 37167649858, 37167651310 | `clippy` | failed | Not an overflow. The throwaway `Cargo.toml` comment cited this milestone before its block existed, and `script/roadmap` refused it. The experiment's own mistake. |
| 37167649858, 37168626690 | `fastpath-footprint` | failed | Expected: the cost below. |
| 37168626690 (budget +64, throwaway) | everything else | passed | The full kernel suite on all three ISAs, `swish-check` on all three, `boot-check`, host tests, fuzz, coverage. |

**No overflow panicked anywhere.** The filesystem server, `mkfs`, `std_exerciser` and the `std`
under them ran their whole test coverage on three ISAs with checks on and none fired.

One side effect: the deterministic frame ledger read 26,670 on aarch64 and 26,645 on riscv64 with
the checked (larger) programs, two over a budget with a 32-frame margin. Adopting checks for them
means re-deriving `SUITE_PAGE_FRAME_BUDGET` in the same change.

## What it costs

All on the same commit (`1a145fcaa`), checks off against checks on, nothing else changed.

### Release kernel: size and the fast path

`script/fastpath-footprint`, run on patagonia, the stock gate against
`CARGO_PROFILE_RELEASE_OVERFLOW_CHECKS=true`. Bytes of instructions an IPC round trip can touch:

| ISA | `ipc_call_reply` off | on | growth | `syscall_entry` off | on | growth |
|---|---|---|---|---|---|---|
| aarch64 | 7,044 | 7,332 | +4.1% | 1,612 | 1,724 | +6.9% |
| riscv64 | 6,106 | 6,632 | +8.6% | 1,976 | 2,244 | +13.6% |
| x86_64 | 8,227 | 8,927 | +8.5% | 1,797 | 1,797 | 0 |

**The gate fails on all three ISAs with checks on** (its 5% drift bound; CI's footprint job on the
experiment branch failed the same way). Every ISA stays inside its 16 KiB ceiling and none crosses
an L1i line it did not already cross: the worst, x86_64's round trip, goes from 25.1% to 27.2% of
radon's 32 KB L1i. riscv64 pays most because it has no flags register, so each check is a compare
and branch rather than a branch on a flag the add already set.

Whole release kernel, sum of function symbol sizes (`llvm-nm -S`), and the number of call sites to
`core::panicking::panic_const_*_overflow`:

| ISA | code off | code on | growth | overflow sites |
|---|---|---|---|---|
| aarch64 | 264,688 B | 268,936 B | +1.6% | 584 |
| riscv64 | 230,896 B | 251,190 B | +8.8% | 1,020 |
| x86_64 | 270,606 B | 299,673 B | +10.7% | 703 |

`.rodata` grows 8 to 12 KiB on each (one `Location` per site). These are upper bounds on what the
kernel would carry, not what a path executes.

### Userspace in the image

The 104 `components` and `fixtures` programs, built release for each ISA, sum of `.text`:

| ISA | off | on | growth |
|---|---|---|---|
| aarch64 | 1,101,408 B | 1,136,356 B | +3.2% |
| riscv64 | 901,524 B | 951,918 B | +5.6% |
| x86_64 | 1,142,886 B | 1,173,941 B | +2.7% |

The three standalone programs were not size-measured; they need the `nife-dev` toolchain link,
which this lane did not take.

### Instructions on the hot paths (the icount tripwire)

The icount bench cannot run a release kernel: `xtask bench` refuses, because release changes
counts, so it is HVF-only and never gates. The instruction cost was therefore measured the other
way round: throwaway
branch `lane/overflow-checks-off` sets `[profile.dev] overflow-checks = false`, and CI's `bench` job
reports each headline against the committed baselines, which are checks-on dev builds.

CI runs 37167649858 (checks on, the stock dev build) and 37167651310 (checks off), same commit,
ticks per iteration from `script/bench --check` on each ISA. A positive cost is what checks add:

| Benchmark | aarch64 | riscv64 | x86_64 |
|---|---|---|---|
| `null_syscall` | 0.0% | 0.0% | |
| `yield_switch` | +0.4% | +1.1% | +0.4% |
| `ctx_switch` | +0.3% | +1.0% | |
| `ipc_rtt` | +0.3% | 0.0% | -2.6% |
| `call_reply` | +0.4% | 0.0% | +0.4% |
| `relay_rtt` | +0.3% | +0.3% | +0.5% |
| `broker_rtt` | +0.3% | +0.6% | +0.4% |
| `ipc_rtt_el0` | -0.9% | +0.8% | |
| `map_new` | +4.0% | +2.6% | +5.8% |
| `map_el0` | +1.2% | +1.6% | |
| `spawn_reap` | +3.8% | +2.6% | +4.4% |
| `spawn_el0` | +2.4% | +1.6% | |
| `coremark` | +5.6% | +5.5% | +7.2% |

Blank cells are benchmarks that ISA does not run. **The IPC and switch paths cost under 1.1%.** The
paths that do arithmetic on addresses and sizes (`map_*`, `spawn_*`) cost 2 to 6%, and CoreMark, a
userspace integer workload, 5.5 to 7.2%. All of it is inside the bench's 10% tripwire, which is
why the checks-off run passed. riscv64's ticks are coarse (`null_syscall` is 3), so a 0% there means
under one tick. Two rows went down with checks on (aarch64 `ipc_rtt_el0`, x86_64 `ipc_rtt`), which
added instructions alone cannot explain; tick quantisation and code layout can, and they set the
noise floor of this table at about 3%. The checks-off branch turns checks off for the kernel and
the workspace programs together, so an `_el0` row carries both.

Not apples-to-apples: these are opt-level 0 kernels, where every check survives. An optimiser
removes checks it can prove redundant, so the release instruction cost is at most this.

### Cycles on the release build (HVF, patagonia)

`script/bench --release` on patagonia's own core under HVF, 2026-10-04 (UTC), at `1a145fcaa`'s code
(this branch changes only documentation). Checks off is the stock build; checks on is the same
commit in a second, unpushed worktree with `CARGO_PROFILE_RELEASE_OVERFLOW_CHECKS=true` (its release
kernel carries 610 overflow sites, the stock one 0). Seven runs of each, interleaved off and on, on
a quiet machine. Median ns per iteration, with the range over the seven:

| Benchmark | off | on | median change |
|---|---|---|---|
| `null_syscall` | 27 [27-27] | 27 [27-27] | 0 |
| `yield_switch` | 31 [29-33] | 29 [29-31] | noise |
| `ctx_switch` | 121 [119-122] | 121 [118-122] | 0 |
| `ipc_rtt` | 49 [46-50] | 48 [48-51] | noise |
| `call_reply` | 69 [63-70] | 68 [65-84] | noise |
| `relay_rtt` | 112 [104-112] | 106 [105-118] | noise |
| `broker_rtt` | 137 [126-139] | 139 [130-142] | noise |
| `ipc_rtt_el0` | 402 [394-421] | 424 [395-431] | +5.5%, ranges overlap |
| `map_new` | 492 [449-572] | 495 [472-632] | noise |
| `map_el0` | 92 [85-92] | 93 [92-117] | noise |
| `spawn_reap` | 2133 [1746-2757] | 1917 [1858-2460] | noise |
| `spawn_el0` | 6260 [5900-6496] | 6217 [5903-7483] | noise |
| `coremark` | 8486 [8463-8538] | 8697 [8691-8778] | **+2.5%, ranges disjoint** |

"Noise" means the two ranges overlap and the medians differ by less than the spread of either. The
one cost the release build clearly shows is CoreMark's 2.5%, a userspace integer workload. On the
kernel's IPC, switch and syscall paths no cost is distinguishable from run-to-run variation at seven
runs. `ipc_rtt_el0` is the one to rerun if A lands: its median moved 5.5% but its ranges overlap.
This is magnitude, not path length: one machine, one core, real caches.

## Prior art

Read from source on 2026-10-04 (UTC) unless marked.

- Rust. The reference: integer operators "will panic when they overflow when compiled in debug
  mode"; Cargo's `dev` profile has `overflow-checks = true`, `release` has `false`. RFC 560 makes
  overflow "a program error (but not undefined behavior)" and lets an implementation check "at any
  time", justifying the release default by "final code will not pay a performance penalty".
- Linux, C. The top-level Makefile builds with `-fno-strict-overflow`, so signed overflow in a
  shipped kernel is defined wrapping. `UBSAN_SIGNED_WRAP` is gone; its successor
  `UBSAN_INTEGER_WRAP` `depends on BROKEN`, "very experimental", and limited to `size_t`. LWN
  (2024-01-26) records that wraparound checking was dropped in 5.12 for false positives and that
  Kees Cook's series adds `add_would_overflow()`/`add_wrap()` and `__signed_wrap` annotations to say
  which wraps are meant; Torvalds objected to the cost in generated code.
- Rust-for-Linux. `CONFIG_RUST_OVERFLOW_CHECKS`, `default y`: "a Rust panic will occur on
  overflow. Note that this will apply to all Rust code, including `core`. If unsure, say Y." That
  the panic ends in `BUG()` is from memory.
- Hubris (Oxide): `-C overflow-checks=y` in every firmware build, because "correctness is
  important, and panicking on integer overflow is better than wrapping and being wrong."
- Android: "Overflow checking is on by default in Android for Rust." Chromium turned it on in
  2022 because "in (simplistic) testing they have negligible performance overhead"; where that flag
  lives today was not found, so its current state is unverified.
- Redox, Tock, Theseus: release profiles do not set it, so off.
- seL4 proves "no arithmetic overflows" in its C; how the proof models word arithmetic was not
  on the page read and is from memory.
- Measured costs elsewhere: Dan Luu measured 28% on bzip2 with C overflow checks, mostly lost
  optimisation rather than branches. No published number for Rust-for-Linux or Hubris was found.

Hubris, Rust-for-Linux and Android's Rust ship with checks on. The kernels that ship them off do not
say why.

## Options

No decision is taken here. Each is reversible: a profile line, and nobody outside the tree has acted
on the current setting.

### A. Checks on everywhere

One `[profile.release] overflow-checks = true` in the root and in the three standalone program
workspaces. The shipped build then behaves like every tested build. Costs: the fast-path footprint
above (4 to 9% on the round trip), the release kernel 2 to 11% larger, userspace 3 to 6% larger, a
re-baseline of `fastpath-footprint` and a re-derived frame budget. In instructions (opt-level 0, an
upper bound): under 1.1% on IPC and switches, 2 to 6% on map and spawn, 5.5 to 7.2% on CoreMark. A
latent wrap in shipped code becomes a halt instead of a wrong answer.

### B. Checks in test and CI only, extended to the three release-only programs

Keep release off for the kernel; add a checks-on build of `redoxfs_server` and the `std` programs to
the test boots (or build them dev under test, as the workspace programs already are). Closes the one
real gap in coverage without touching what ships. Leaves the gap between what is tested and what is
shipped.

### C. On for the kernel, off on the fast paths by writing them explicitly

Option A, then replace the arithmetic on the IPC and syscall paths with `wrapping_*` or `checked_*`
where the bound is known, until `fastpath-footprint` is back inside its band. Every remaining
unchecked operation is a written claim at its site. More work, and the work is on the code that is
most performance-tuned.

### D. Unchanged

The three programs stay unchecked everywhere.

## Recommendation

**Option A**, with B's fix folded in, and C done as follow-up on the paths the footprint gate names.

The reason is the second fact under the first table: today the build that is tested and the build
that is shipped disagree about what `a + b` means. Every overflow this tree has found was found
because a tested build panicked; the same input in the shipped build would have wrapped, and in the
`dtb` case the wrap was the worse outcome. A kernel that halts is a visible bug a person can report.
A kernel whose frame allocator believes a wrong memory map is not. That is the same argument Hubris
prints in its build and Rust-for-Linux makes its default.

Question 7: if A to D cost the same, would I still choose A? Yes; this is not about effort. D is the
least work and loses on correctness; C is the most and is what A becomes on the hot paths over time.
The measured price is 4 to 9% more fast-path bytes (inside the 16 KiB ceiling), about 1% more IPC
instructions, and on the aarch64 release build under HVF no IPC cost above noise and 2.5% on
CoreMark. That is small enough to pay for a shipped build that means what the tested one means.

## BUGS

- The HVF cycle numbers are aarch64 on one machine (patagonia); riscv64 and x86_64, where the
  footprint cost is larger, have no release cycle measurement.
- The icount figures are opt-level 0 and overstate the release instruction cost.
- `cryptography_exerciser` and `rg` are built only by hand-run helpers, so no CI run of this
  experiment exercised them.
