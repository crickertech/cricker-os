---
status: BUILT
raised: 2026-09-24
built: 2026-09-24
promoted_from: ci-arch-split
---
# 587. Most CI jobs do not need an arm64 host, and the arm64 queue is where the wait is

Built, 2026-09-24 (pull request #1249). *(Number minted at promotion.)* Promoted from the proposal
`ci-arch-split`, filed 2026-09-24 by the `maintainer/ci-arch-split` lane, which the maintainer
briefed after three merge-group builds waited over an hour at 19:05 UTC with 13 jobs running, all
of them `ubuntu-24.04-arm`. **calef ruled option B on 2026-09-24** (*"Yes, proceed with B"*), below
under "The ruling". The text after this paragraph is the proposal's own except for that section,
its gate line (dropped once BUILT), `## What was built`,
`## Follow-on` and the `## Index row` section.

## What is being decided

Whether to move the jobs that do not need an arm64 host onto `ubuntu-24.04` (x86_64), keeping
arm64 only where the host architecture changes what a job proves.

## The ruling

**Option B, calef, 2026-09-24:** move the 15 jobs listed under "The recommendation" to
`ubuntu-24.04`; keep `build + test`, `cpu matrix` and `re-falsify` on arm64; add `prove the kernel
on aarch64` inside the `verify (Kani proofs)` aggregate; and correct `ci.yml`'s header, which says
testing on aarch64 is "the one place CI catches" ordering bugs, a claim the host tests do not bear
out (see "What is lost").

**Done means**: the implementation's own run shows every moved job green on x86_64 and the aarch64
kernel proof green; this block becomes BUILT in that pull request; and the first merge-group builds
after it lands are measured against the 16:00 to 18:00 rows of the table below.

**The re-measure trigger.** One week after the implementation merges, re-run the created-to-started
queue measurement over that week's `ci.yml` and `verify.yml` jobs, by label. If the arm64 jobs that
remain still wait a median over ten minutes in busy hours, the next lever is option D (the x86_64
guest legs out of `build + test`); if the x86_64 jobs have started waiting like arm64 did, the
premise that x86_64 supply is looser was a single afternoon and this is revisited.

## What was built

- **`ci.yml`**: every job but `test` (`build + test (host + QEMU)`) and `cpu-matrix` runs on
  `ubuntu-24.04`, the `draft gate` included, so a run's first job no longer waits in the arm64
  queue. The header's weak-ordering paragraph is rewritten with the evidence below: the host tests
  race no threads, so the claim holds only for the SMP guest legs, which are the two jobs kept.
- **`verify.yml`**: `gate`, `scope`, both `prove` shards and the `verify (Kani proofs)` aggregate
  run on `ubuntu-24.04`; `falsify` stays on arm64. **`prove-kernel-x86_64` became
  `prove-kernel-aarch64`** rather than gaining a sibling: with the shards on x86_64 their own
  `kernel` row proves `arch/x86_64/`, so the old job would repeat it and `arch/aarch64/` would be
  proved nowhere. The aggregate judges the new job with the same vocabulary, so it is inside the
  required check exactly as the shards are. The runner count per run is unchanged; the arm64 share
  of it went from 18 jobs to 4.
- **Caches cannot cross hosts**, and nothing was needed to make that so: `Swatinem/rust-cache`
  keys on the host (the measurement's cache keys read `Linux-x64` and `Linux-arm64`) and the QEMU
  cache key already carries `runner.arch`. The one shared key is the vendored tarball cache, which
  holds `.crate` files that are the same bytes on either host.
- **Collision**: rebased over #1220 (A′, a push to `main` cites the merge group that tested it),
  which edits the same two `gate` jobs. Its `permissions:` blocks and push branch are kept whole;
  only the `runs-on:` line beside them changed.

**The implementation's own run**, dispatched on the branch before merge: `ci.yml` 36060856009 and
`verify.yml` 36060858268, both `success`. Every moved job green on `ubuntu-24.04`; `prove the kernel
on aarch64` green on `ubuntu-24.04-arm` in 0.8 minutes, proving `arch::aarch64::iommu::proofs`
alongside the syscall harnesses; the aggregate printed "the kernel proved on aarch64 too". Both
shards proved on x86_64 in 15.6 and 16.3 minutes of wall time.

## The evidence that arm64 supply, not our quota, is the constraint

- **The organization's 60-job cap was not close.** Counting every job in every nife run from
  15:24 UTC onward (nife is the only repository in the organization that ran Actions that
  afternoon), 19 to 25 jobs were running between 17:50 and 18:50 while **128 to 173 jobs sat queued
  for more than a minute**. The running count reached 50 only at 19:31, after the queue had drained.
- **x86_64 jobs in the same runs waited far less.** `prove the kernel on x86_64` is the one job
  already on `ubuntu-24.04`, so every `verify.yml` run is a paired sample. Across the day's `ci.yml`
  and `verify.yml` jobs, by the hour each job was created:

  | Hour (UTC) | arm64 jobs | arm64 median / max queue | x64 jobs | x64 median / max queue |
  |---|---|---|---|---|
  | 16 | 249 | 14.8 / 42.9 min | 10 | 0.2 / 7.1 min |
  | 17 | 315 | 23.9 / 70.8 min | 7 | 8.1 / 15.1 min |
  | 18 | 290 | 14.5 / 57.3 min | 10 | 2.6 / 13.6 min |
  | 19 | 405 | 0.1 / 29.2 min | 4 | 0.0 / 5.9 min |

  In every same-run pair where the arm64 jobs waited more than a few minutes, the x86_64 job
  waited less, often by an order of magnitude (41.0 against 13.6 min, 28.4 against 2.7). When
  neither pool was backed up, both waited under a minute and the order between them is noise.
- **x86_64 is not immune**, and the table says so: one x86_64 job waited 15 minutes at 17:00. The
  claim is that the arm64 pool is the tighter of the two, not that the other one never waits.
- **The measurement run itself queued for 0.0 to 0.1 minutes on every job**, but it started at
  19:12, after the arm64 backlog had cleared, so it is not evidence on its own. The hourly table is.

## The measurement

One run of each workflow on `ubuntu-24.04`, from a temporary branch whose workflows were copies of
`ci.yml` and `verify.yml` with the runner label changed, the draft gate dropped, and (for verify)
the scope job dropped so the proofs actually ran. Compared against the latest `main` run of the same
job on `ubuntu-24.04-arm`.

| | x86_64 run | arm64 reference |
|---|---|---|
| `ci.yml` | 36046550177 (tree at `47c3a3c9d`) | 36030250577 (`main` at `47c3a3c9d`, same tree) |
| `verify.yml` | 36046550184 (tree at `47c3a3c9d`) | 36029132635 (`main` at `0b72f6736`, the latest `main` run whose shards proved) |

**The check step is the comparison, not the job's wall time.** The x86_64 jobs ran with every cache
cold (the QEMU cache key includes `runner.arch`, and `Swatinem/rust-cache` keys on the host triple),
so their wall time includes a from-source QEMU build of 6.1 to 7.1 minutes and a first `cargo
install` of the lint tools. That is paid once and then cached; the named `script/ci-build` step is
what recurs.

## Every job, classified

**No job in either workflow uses KVM or any accelerator.** Every QEMU leg in CI is TCG
(`helpers/qemu-runner-aarch64.sh` takes HVF only when `NIFE_ACCEL=hvf`, which only the dev Mac's
`script/test --hvf` sets, and `script/ci-build`'s `hvf` row is never named by a CI job). So "needs
host-native speed" applies to nothing here. What does apply is whether **the host's architecture
changes what gets proved or tested**, and that is three things: Kani compiles for the host,
falsification records for `kernel/src/arch/` compile only on their own architecture, and guest
cores under multi-threaded TCG run on the host's memory model.

Times are minutes: the named check step, then the job's wall time in brackets. Queue is the
reference run's created-to-started wait for that job.

| Job | Needs arm64? | Why, from its steps | x64 | arm64 | Queue x64 / arm64 |
|---|---|---|---|---|---|
| `ci` draft gate | No | One `gh pr view` call | (not run) | 0.1 | n/a / 11.1 |
| rustfmt | No | `script/fmt --check`: text | 0.4 (0.7) | 0.2 (0.4) | 0.0 / 51.2 |
| clippy | No | `script/lint`: clippy on host and bare-metal targets, then text gates. On x86_64 the host build lints the `x86_64` arm of the few `cfg(target_arch)` items in host crates; the aarch64 arm is still linted through `aarch64-unknown-none-softfloat` | 1.7 (4.3), **red, measurement artifact** | 1.8 (2.6) | 0.0 / 47.8 |
| fastpath footprint | No | Cross-compiles release kernels for all three targets and walks `objdump` | 0.2 (0.7) | 0.3 (0.6) | 0.0 / 49.5 |
| reproducible build | No | Two builds on one host compared to each other | 0.2 (0.6) | 0.2 (0.6) | 0.0 / 51.1 |
| image permissions | No | Cross-compiles three kernels, reads ELF segment flags | 0.2 (0.8) | 0.1 (0.7) | 0.1 / 60.1 |
| stack frames | No | Cross-compiled `.stack_sizes`; host only supplies the tools | 0.5 (1.2) | 0.5 (1.1) | 0.1 / 53.1 |
| bench | No | TCG with `-icount`, single hart: counts guest instructions | 0.9 (10.5) | 0.7 (1.2) | 0.1 / 57.3 |
| supply chain | No | `cargo-deny` and tarball hashes | 0.1 (0.6) | 0.1 (0.5) | 0.1 / 54.9 |
| fuzz | No | libFuzzer on host builds of pure-logic parsers, 60 s a target | 5.7 (6.2) | 5.3 (5.7) | 0.1 / 47.4 |
| coverage | No | Host tests under `cargo llvm-cov`, 80% per-file floor | 0.6 (1.2) | 0.9 (1.4) | 0.1 / 62.5 |
| build + test | **Keep, weakly** | Host tests, then the kernel suites under TCG at `-smp 4` (aarch64, riscv64) and `-smp 2` (x86_64), then `swish-check` and `boot-check`. The SMP guest legs are the one place CI runs guest code on a weakly ordered host. See "What is lost" | 15.7 (24.1) | 20.2 (20.8) | 0.0 / 52.4 |
| cpu matrix | **Keep, weakly** | The riscv64 suite under five QEMU CPU models, `-smp 4`; the same weak-host argument | 8.7 (17.8) | 11.2 (11.9) | 0.0 / 59.8 |
| `verify` draft gate | No | One `gh pr view` call | (not run) | 0.1 | n/a / 11.8 |
| verify scope | No | `script/verify --affected-since`: git and `cargo metadata` | (not run) | 0.0 (0.4) | n/a / 4.3 |
| prove (shard 1/2) | No, **except the `kernel` row** | Kani compiles for the host, so the `kernel` row proves `arch/aarch64/` on arm64 and `arch/x86_64/` on x86_64 (milestone 304 (`cargo kani -p kernel` only ever compiled one architecture, and it was the runner's)). Every other row is portable logic | 15.1 (15.9) | 12.9 (13.4) | 0.1 / 27.1 |
| prove (shard 2/2) | No, as above | As above | 13.6 (14.3) | 16.9 (17.5) | 0.1 / 9.1 |
| re-falsify | **Yes, for kernel arch records** | `script/falsifications` replays `kernel/falsifications/arch.aarch64.iommu.*` only on an aarch64 host; its own header records that the sweep "replays a harness only on a host whose architecture compiles it" | (not run) | 45.2, cancelled at its timeout | n/a / 21.0 |
| `verify (Kani proofs)` | No | Reads two job results as strings | (not run) | 0.0 (0.1) | n/a / 4.0 |

**Results matched on every job but one, and that one is an artifact of the copy.** clippy went red
on x86_64 because the temporary workflow file copied `ci.yml`'s comments, and `script/citations
--ratchet` read their `milestone N` references as new unglossed citations. clippy itself had
finished and passed by then; the failure is the ratchet's, on a file that was never going to merge.
Beyond pass/fail:

- **Host tests**: 3,067 `ok` lines on both hosts, and the same distribution of per-binary results.
- **bench**: every instruction count matched to within one tick (`coremark` 20,913,779 against
  20,913,778; `null_syscall` 410,003 against 410,004). `-icount` is host-independent in practice,
  as `ci.yml` says it is designed to be.
- **fastpath footprint and stack frames**: the check output was identical line for line; the only
  differences were cache keys and temporary paths in the post-job cleanup.
- **Kani**: both shards green on both hosts. The shard times move in opposite directions
  (shard 1 slower on x86_64, shard 2 faster), and the arm64 reference is a different commit, so the
  honest reading is "no difference this sample can see", with a critical path of 15.1 minutes on
  x86_64 against 16.9 on arm64.
- **The x86_64 guest legs were much faster on an x86_64 host**: `swish-check (x86_64)` under OVMF
  took 7.9 minutes against 12.4, and the OVMF boot 16 seconds against 56. The aarch64 and riscv64
  kernel legs were a few seconds *slower* (78 against 70 s, 90 against 85 s). Net, the `build +
  test` check step was 15.7 against 20.2 minutes.

## The recommendation

**Move 15 of the 18 arm64 jobs to `ubuntu-24.04`, keep three on arm64, and add one small arm64
job.** This is a reversible change: each job is one `runs-on:` line, and reverting it is the same
line.

- **To x86_64**: both draft gates, verify scope, the `verify (Kani proofs)` aggregate, rustfmt,
  clippy, fastpath footprint, reproducible build, image permissions, stack frames, bench, supply
  chain, fuzz, coverage, and both `prove` shards.
- **Stay on arm64**: `build + test`, `cpu matrix`, and `re-falsify`.
- **New on arm64**: `prove the kernel on aarch64`, the mirror image of the existing
  `prove the kernel on x86_64` job (`script/verify --only kernel`, measured at 0.1 minutes of
  proving on the reference run), folded into the `verify (Kani proofs)` aggregate the same way.
  Without it, moving the shards to x86_64 would leave `arch/aarch64/` proved nowhere, which is the
  exact gap milestone 304 closed in the other direction. The shards would then prove the `kernel`
  row on x86_64 as a duplicate of the existing x86_64 job; five seconds, and removable later.

**What it buys.** On the reference run, every arm64 job waited behind a draft gate that itself
queued 11 minutes on arm64, and then 47 to 62 minutes more. Moving both gates removes the serial
arm64 wait at the front of every run. And a full run's arm64 demand drops from 18 jobs to 4, three
of them the long QEMU jobs that genuinely want the host, so the queue that remains is for work that
earns it.

**Why the two QEMU jobs stay, and why only weakly.** Recalled from QEMU's documentation, not
measured here: multi-threaded TCG runs each guest core on its own host thread and translates guest
loads and stores to plain host ones, so an aarch64 or riscv64 guest on an aarch64 host can observe
reorderings the host permits, while on an x86_64 host it sees only what TSO allows. The tree already
records the other half of that rule: `helpers/qemu-runner-x86_64.sh` notes that QEMU refuses
parallel cores for an x86_64 guest on an aarch64 host (it falls back to round-robin), which is what
a host weaker than its guest looks like. So the SMP kernel legs on arm64 are the only place CI puts
guest code in front of weak-memory reorderings, and moving them would remove that silently. "Weakly"
because nothing in this tree records a failure those legs caught that an x86_64 host would have
missed; the value is a plausible class, not a demonstrated one.

**Would we still choose this if both options cost the same?** Yes. The recommendation keeps every
job whose result depends on the host where it is, and moves the ones measured to produce identical
results. The case is capacity, not effort, and the effort is small either way.

## What is lost, said plainly

- **Host tests on an aarch64 host, for the jobs that move.** Coverage and fuzz would run on x86_64
  builds. This costs almost nothing today, and that is a measurement rather than a hope: in the host
  crates, **one** non-loom test spawns a real thread (`memory_corruption_canary_gate`'s `bounded`,
  which asserts liveness through a channel), and the concurrent protocols (`work_steal_slot`,
  `thread_wake_handshake`, `memory_regions`, `clock_protocol`, `memory_corruption_canary_gate`) are
  checked for ordering by loom, which models C11 on any host and is not in CI at all
  (milestone 80 (loom: the hand-rolled atomic protocols, model-checked)). So `ci.yml`'s header
  sentence that testing on aarch64 "is the one place CI catches that class" is **not true of the
  host tests**, which exercise no cross-thread ordering. It is true, in the recalled-QEMU sense
  above, of the SMP guest legs, which this proposal keeps on arm64. That header should be corrected
  whichever way this is decided. And `build + test`'s own host-test pass stays on arm64 under this
  proposal, so nothing about host ordering changes unless option C is taken.
- **"Assume weak memory ordering" (AGENTS.md's fourth rule) is not weakened by the recommended
  split**, for the reason above. It **would** be weakened by option C: every guest SMP leg would run
  under TSO, and a missing acquire in kernel code that shows up only as a reordering would pass CI.
  The dev Mac (aarch64, and HVF runs real cores) would still see it, but lanes now gate in CI rather
  than locally (`briefs/gate-in-ci.md`), so the dev Mac is no longer where most lanes' code runs.
- **`elf`'s host tests would exercise the `x86_64` accept arm, not the `aarch64` one**, on any job
  that moves and runs them. Milestone 288 (host tests that assume the host is aarch64) made those
  tests host-neutral, so they pass either way; what changes is which arm is exercised. Under the
  recommendation only coverage is affected.
- **The CI host stops matching the dev Mac's architecture for most jobs.** `ci.yml`'s header gives
  "passes locally means passes in CI" as a second reason for arm64. It matters less than it did
  because lanes no longer gate locally.
- **Cold caches, once.** The first x86_64 runs pay a QEMU source build (6 to 7 minutes) in each
  QEMU job and a `cargo install` of the lint tools; the recommended split moves only `bench` among
  the QEMU jobs, so that cost lands on one job.

## The options

- **A. Status quo.** Everything on arm64 except the one x86_64 Kani job. Costs the waits above
  whenever the arm64 pool is short, which on 2026-09-24 was most of the afternoon.
- **B. The recommendation above.** 15 jobs move, three stay, one small arm64 job is added.
- **C. Everything to x86_64**, with `prove the kernel on aarch64` and `re-falsify` kept on arm64
  because they cannot move without losing proofs. Removes almost all arm64 demand. Loses the SMP
  guest legs' weak-host exposure described above; gains a faster x86_64 guest leg (12.4 to 7.9
  minutes of OVMF shell check).
- **D. B, plus splitting the x86_64 guest legs out of `build + test` into an x86_64 job.** The
  in-tree comment on that job already names the x86_64 leg as "the one to move to a job of its own"
  if it grows. It would shorten the arm64 job by about ten minutes and run that leg about a third faster. It
  is a larger edit than B (the `test`, `swish-check` and `boot-check` rows would need an `--arch`
  split in `script/ci-build`), so it is listed rather than recommended.

## What is blocked until this is answered

Nothing is blocked. The queue drains when arm64 supply returns, as it did by 19:30. What this
decides is how often a lane waits an hour for a runner, and DECISIONS §203 (capacity is rented
rather than bought) already names "merge throughput" as one of the three things runners buy.

## BUGS

- **One x86_64 sample.** Queue waits come from a full afternoon of real runs; the per-job timings
  come from one run per host, with a different commit behind the arm64 Kani reference.
- **The weak-host argument for the QEMU legs is recalled, not measured.** Measuring it would need a
  deliberately broken ordering in kernel code that fails on an arm64 host and passes on x86_64,
  which is its own piece of work.
- **The x86_64 guest under multi-threaded TCG on an x86_64 host was not confirmed.** It passed at
  `-smp 2`; whether QEMU chose parallel cores or round-robin there was not checked.

## Follow-on

- **Milestone 648.** Milestone 648 (re-measure CI queue waits by runner label, a week after the arm64 split). The one-week re-measure of queue waits by runner label, and option D as the next
  lever if arm64 still waits: `design/roadmap/648-ci-queue-remeasure.md`, dated so its age
  shows in `script/roadmap --proposed` rather than living only in this block's trigger paragraph.
- **Recorded.** That multi-threaded TCG shows an aarch64 or riscv64 guest the host's reorderings is
  recalled from QEMU's documentation, not measured; the header of `.github/workflows/ci.yml` says
  so where it keeps `test` and `cpu-matrix` on arm64 for that reason.
- **Decision.** The coverage job now runs the host suite on x86_64 on every pull request, which is
  part of what `design/decisions/184-an-x86-64-host-in-the-host-pass.md` asks about (and milestone
  403 (an x86_64 host in the host pass) builds). It is coverage's pass, not `script/test`'s, so the
  section's question stays open; the maintainer should tell whoever holds it.

## Index row

On 2026-09-24 arm64 hosted runners were the bottleneck, not the 60-job cap: ~20 jobs ran while 150+ waited. One measured x86_64 run of every job matched arm64's results, so 15 of 18 jobs move to `ubuntu-24.04`, the two SMP QEMU jobs and the arch falsification replay stay on arm64, and a one-minute aarch64 kernel proof keeps `arch/aarch64/` proved.
