# Evidence for the 2026-09-24 audit

The code paths behind [the report](../2026-09-24-new-trust-boundaries.md), at the tree it read
(`origin/main` at `69be99617`, against the last audit's `8e4587056`). Line numbers are from that
commit; the fixes in the audit's own branch move some of them by a few lines.

## The automation

| Question | Answer | Where |
|---|---|---|
| What arms auto-merge | every open, non-draft PR against `main`, no author test | `helpers/merge-drain.sh` `queue()`; `helpers/queue-hold.sh` `holdable()` |
| Reviews required on `main` | `required_approving_review_count: 0` | `gh api repos/nifeos/nife/rules/branches/main` |
| Forking, auto-merge | `allow_forking: true`, `allow_auto_merge: true`, public | `gh api repos/nifeos/nife` |
| Fork workflow approval | `first_time_contributors` | `gh api repos/nifeos/nife/actions/permissions/fork-pr-contributor-approval` |
| Secrets in reach of a merge-group run | `TOOLCHAIN_BUMP_PAT` (repo), `AUTOMATION_APP_ID`, `AUTOMATION_APP_KEY` (organisation) | `gh api .../actions/secrets`, `.../actions/organization-secrets` |
| Fork PRs ever | none; 991 by calef, 9 by dependabot | `gh pr list --state all --limit 1000 --json isCrossRepository,author` |
| `pull_request_target` anywhere | no | `grep -n pull_request_target .github/workflows/*.yml` |
| Untrusted text into `run:` | none; only `github.event.*.number`, `.sha`, `.before`, `.after`, `merge_group.head_ref` | grep of every `${{ }}` in `.github/workflows/` |
| PR text in scripts | `title`, `body`, `headRefName` via `jq -r`; `body` only to `sed -n` for `Blocked-by: #N`; `head` only as a quoted `--branch` argument | `merge-drain.sh:330-334`, `:241-253` |
| `curl \| sh` | rustup's installer, `--proto '=https' --tlsv1.2 -sSf` | `script/bootstrap:27` |
| Committed bytecode | `scripts/__pycache__/name_provenance.cpython-314.pyc`, added 2026-09-18 in `bc4ba1d47` | `git ls-files -ci --exclude-standard` |

The fork approval policy makes a first pull request wait for a click; the drain then merges it
if green; the second pull request from that account runs its checks without a click. GitHub's
documentation says a fork's `pull_request` run gets a read-only token and no secrets, and says
nothing about restricting a `merge_group` run, whose ref is a branch in this repository. The
merge-group step was read, not exercised.

## The parsers

| Crate | Caller and trust boundary | Total over hostile bytes? | Tests |
|---|---|---|---|
| `package_archive` | none on target yet; a fuzz target and host tests | yes: `parse` bounds count (`MAX_MEMBERS`), table, and every `offset + len` in `usize` with `checked_add` (`lib.rs:242-270`) | doc tests, `fuzz/fuzz_targets/package_archive_roundtrip.rs` |
| `device_tree_blob` | kernel boot on aarch64/riscv64 (`main.rs:120`, `memory.rs:42`), `uefi_loader` riscv64, `machine_discovery`; firmware supplies the blob | over a slice, yes: `be32`/`be64` are `checked_add` + `get`, depth-bounded walkers, `RegionOverflow`; **`from_ptr` believed `totalsize` (`lib.rs:142-155`)**, fixed | 30 host tests in `tests/hostile.rs`, 4 Kani harnesses with falsifications, `fuzz_targets/device_tree_blob_walk.rs` |
| `firmware_configuration` | `drivers/ramfb.rs:45`, kernel, before the MMU; QEMU's `fw_cfg` DMA | yes: 64-byte `try_into`, prefix guard, `size` unused; walk bounded by `MAX_ENTRIES`, poll by `MAX_POLLS` | 8 host tests |
| `portable_executable` | `xtask/src/stick.rs:179`, host, over the ELF xtask just linked | no: `phoff + 4`, `vaddr + memsz`, `PT_DYNAMIC` unchecked, `vec![0; image_end]`, `DT_RELAENT = 0` loop (`lib.rs:109, 187, 193, 197, 227-229`) | 4 host tests; none for truncation or overflow |
| `file_allocation_table` | `components/src/installer.rs:580`; inputs are constants, the disk size and the kernel's own boot file length | it is a writer; `Volume::new` refuses `> u32::MAX`, `checked_sub` on reserved and FAT sectors, `sector()` bounds index and length | 11 host tests, one ignored integration test |

## The kernel state

| Claim | Evidence |
|---|---|
| `SURVEY` needs `ENUMERATE`, refuses unknown records before the walk, takes any cursor, passes no pointer, zeroes outputs on `DONE` | `syscall.rs:296-306`; `sched.rs:4103-4141`, `:4112-4114`, `:4119`, `:4140`; `generational_table/src/lib.rs:279-288`; `abi/src/lib.rs:441-443` |
| Domain membership is exact and proved | `capability/src/lib.rs:227-229` and its falsification patch |
| `CPU_TICKS` is zeroed on slot fill | `sched.rs:424`, `:446`, `:272-276` |
| Nothing pins a thread from userspace | writers of `Thread::placement` are `spawn_on` (`sched.rs:1551-1553`) and `start_thread_control_block` (`:4433-4438`), both from `pick_spawn_target()` |
| `PLACEMENT` leaks a run-queue comparison | `sched.rs:1596-1601`; the denial at `syscall.rs:293-295` |
| Port revoke is synchronous on `x86_64` | `sched.rs:3397-3450` under `IPC_TABLES`; `segments.rs:478-481`; `mmu.rs:606-650` NMI broadcast with ack spin; handler `:663-687` |
| The take-back reset the invoker's own core | `sched.rs:3405-3407` spares the keeper's table; `segments.rs:479` reset locally regardless; `:449-453` resets when the installed grant matches |
| Page revoke flushes synchronously on all three | aarch64 `mmu.rs:1018-1032` (`tlbi vaae1is; dsb ish`); riscv64 `mmu.rs:919-933` + `mod.rs:281-300` (SBI remote fence, honest `BUGS` at `:325-333`); x86_64 `mmu.rs:492-499` |
| The in-flight `MAP` race §13 (capability revocation and untyped reclamation) deferred is open and recorded | `syscall.rs:852-868` (PTE before log); `revoke.rs:523-527` |
| FP is eager, scrubbed, never enable-driven | `fp.rs:129-148` (four cases), `:24-29`; `thread.rs:619-624`; per-core disable at `sched.rs:1292`, `:1366`; `INITIAL` on both inserts `:408`, `:441`; first use `fp.rs:191` |
| The trap arm cannot be preempted between enable and mark-live | x86 interrupt gates `exceptions.rs:206`, `:412`; riscv64 patches the frame's `FS` `exceptions.rs:480-484` |
| The kernel emits no FP | targets `aarch64-unknown-none-softfloat`, `riscv64imac-unknown-none-elf`, `x86_64-unknown-none` with `-sse,+soft-float` (`xtask/src/main.rs:70-82`) |
| SVE/SME and V were not disabled | `arch/aarch64/fp.rs:26-29` (header), `:51-53`; `arch/riscv64/fp.rs:67-71`; fixed in `init()` on both |
| `x86_64` cannot hold `ymm`/`zmm` state | `CR4.OSXSAVE` never set (no `xsetbv` in the tree), so VEX/EVEX `#UD` |
| The two window-added revocation tests are single-core page-table walks | `revocation_in_flight_tests.rs:81-142`; `spawn_mapping_revocation_tests.rs:95-159` |
