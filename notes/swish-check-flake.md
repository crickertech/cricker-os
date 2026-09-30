# The swish-check CI flake, measured (name provisional)

Two `build + test (host + QEMU)` failures on the evening of 2026-09-29 (UTC), both at the
swish-check step, on pull requests touching no runtime code: #1444 (hooks only, riscv64, "could not
spawn (the progenitor is out of memory)", run 36619551608) and #1442 (ledger only, aarch64, "the
prompt never echoed `packages/greeting/0.1.0/greeting`", run 36622549563). Both were green on rerun.
This note measures how often that happens, explains each signature mechanically, and prices
remediations without enacting any. Written by an investigation lane on
`maintainer/swish-flake-measurement`; every date is UTC.

## The question

Is swish-check flaky in CI, how often, and by what mechanism; or is the evening of 2026-09-29
unlucky?

## Method, and its limits

- The CI workflow (`ci.yml`) over 2026-08-30..2026-09-30, metadata only, from the Actions API. The
  filtered listing caps at 1000 rows per query, so the window was fetched per day and deduplicated
  by run id; 5380 unique runs, newest 2026-09-29T23:19Z. A run's conclusion reflects its
  **latest attempt**, so a failure rerun green reads as success; `run_attempt` in the listing finds
  the reruns, and each prior attempt's failed jobs were read separately. The API throttled to 502
  zero times, so the sleep-and-retry path never fired.
- Job and step conclusions for every run whose latest attempt failed (444 runs), then **15 job
  logs, tail only**: the two phenomenon attempts, the five 2026-09-29 evening failures, a spread of
  the 2026-09-26/27 block, both 2026-09-28 failures, one 2026-09-25, one 2026-09-21. Check-run
  annotations carry no failure text (exit code only), so no cheaper classification exists.
- Local reproduction from the lane worktree: `helpers/qemu-bounded.sh` around every
  `script/swish-check --arch <arch>`, one QEMU at a time, 10 runs per architecture plus warm-ups,
  `pgrep -l qemu` clean after every batch.

Two lane hazards found while setting up, recorded because the next lane will hit them:

- **`CARGO_TARGET_DIR` silently drops the file server from the archive.** `redoxfs_server_build`
  honors the variable but `redoxfs_server_elf` reads `redoxfs_server/target/<triple>/release/...`
  (xtask/src/disk.rs:104), so the packer cannot find the server and the boot runs with no
  filesystem: the archive shrinks 101 programs to 99, the progenitor prints neither
  directory-grant sentence, and the gate fails on the missing sentence rather than naming the
  cause. Symlink `redoxfs_server/target` at the same directory `CARGO_TARGET_DIR` names.
- **A target dir shared across differently-based worktrees cross-compiles.** Stale rlib metadata
  from a newer `main` made this worktree's `fixtures` fail E0425 and `components` fail E0004
  against crates this tree defines correctly. A worktree-local target dir fixed both. The
  measurement below ran entirely from a clean local target.

## Counts, with denominators

- 5380 CI runs in the window: 4278 green, 447 failed at latest attempt, 653 cancelled. Events:
  3399 pull_request, 1170 merge_group, 639 push, 172 workflow_dispatch.
- 158 runs failed `build + test (host + QEMU)`. Of those, the gate's step
  (`script/ci-build test swish-check swish-check-graphical   boot-check`, .github/workflows/ci.yml:403)
  failed in 84, and its pre-rename `shell-check` spelling in 54 more (renamed 2026-09-23). Nineteen
  failed an older generic step name and are unclassified: no log budget was spent on them. With 16
  rerun-hidden failures found in prior attempts, 154 runs are known to have failed the gate's step
  in some attempt, and the 15 logs below cover 15 of those 154.
- 27 runs were rerun at least once. Eight of those had the gate's step fail in the prior attempt
  and stand green now, including both phenomenon runs. Rerun-green is where flakes hide: run-level
  counts undercount them by construction.
- The 84 swish-named failures concentrate: 49 on 2026-09-26, 25 on 2026-09-27 (an incident block;
  sampled logs show kernel test panics, `sched.rs:2572`, `sched.rs:2636`, `virtio.rs:740`, not
  flakes), 5 on 2026-09-29, 3 on 2026-09-25, 2 on 2026-09-28.
- Signatures in the 15 sampled logs: 7 real breakage (kernel test panics or in-flux code, including
  one `fs_service.rs:2313` panic), 4 one-cluster package-source truncation ("the package source did
  not send a whole package", 2026-09-29 22:00, correlating with the `614-every-version-live` work),
  2 echo-wait false reds, 1 progenitor out-of-memory, 1 std-program illegible-output failure.

So the two phenomenon signatures are real, repeat across days, and are a minority of red runs at
this step: most reds are real breakage arriving through the same step.

## Each signature, mechanically

**The echo wait (signature of #1442).** Each typed line is waited for as an exact string,
`wait_after(at, "{echoed}\n", 30s)` (xtask/src/swish_check.rs:2192; the 30s constant at :1217, boot
120s at :1216, x86_64 90s at :1246). The failure is usually not slowness. Run 36622549563's
transcript shows the kernel's progenitor-stack gauge spliced **inside** the echo itself:

```
$ packages/greeting/0  progenito.1.0/r stack: 27400 of 49152 bytes at peak, 21752 spare
greeting
```

The command ran (its output follows); the exact string can never match because two processes write
one UART with no arbiter (script/swish-check's first BUGS, lines 66-89). `degauge`
(swish_check.rs:1508) was built for this after #1371 and #1420, but it strips a gauge it can find
whole; when the splice lands inside both strings at once, neither is recoverable. The same
signature hit the riscv64 reboot leg on 2026-09-28 (run 36362665042, "never echoed
`packages/greeting/0.1.0/greeting`" beside an uptime line). The gate's own constant doc already
records this class (:1206-1215) and names the remedy: rerun the leg before reading the transcript.

**The progenitor OOM (signature of #1444).** The shell prints the sentence when the progenitor
answers with the `SPAWN_FAILED` sentinel (components/src/swish.rs:3053; the sentinel is
`u64::MAX`, crates/grant_plan/src/spawnproto.rs:633). A `std` program needs one contiguous
384-page region (`STD_REGION_PAGES`, crates/grant_plan/src/lib.rs:1713) carved from a job pool of
exactly six native regions plus one std region (crates/system_initializer/src/lib.rs:702).
`split_job` waits for an in-flight reap by **yielding 1024 times**, and its own doc calls that
"kept on effort, and a foot gun", recording the same CI failure on 2026-09-27
(system_initializer/src/lib.rs:4002-4032). Reclamation is asynchronous (`components/src/job_undertaker.rs`),
and its BUGS note a mode where a parked fault-report SEND wedges the collecting loop entirely
(job_undertaker.rs:38-51). QEMU memory is not implicated: both runners give 256 MiB
(helpers/qemu-runner-aarch64.sh:537, helpers/qemu-runner-riscv64.sh:273), and the constraint is the
progenitor's own bounded pool. Whether the 2026-09-29 instance was the yield budget or
fragmentation of the pool is not determined; the in-tree precedent says yield budget.

## Local reproduction: zero

Ten `--arch aarch64` and ten `--arch riscv64` runs (plus warm-ups), each a full leg with two boots,
from a clean worktree-local target: 20/20 green, no failure of either signature. Zero is a result:
on this fast, unloaded host the races do not close. CI's runner is measured 1.5x to 1.8x slower
than this machine on these legs (swish_check.rs:1232-1234), which widens both windows: the
gauge-versus-echo interleave and the reap-versus-yield race.

## What the data supports, and refuses

Supports: both phenomenon failures are timing races the tree already knows about, not regressions
in the PRs that carried them; each has an in-tree record predating this evening (#1371/#1420 for
the interleave, 2026-09-27 for the OOM). The step is red for real reasons far more often than for
these two signatures; the flake is a minority mode that reruns cure.

Refuses: a per-run flake probability. Signatures need logs, the log budget was 15 of 154
gate-step-failing runs, and the sample was targeted at the phenomenon and the clusters, not drawn
at random. It also refuses "slow echo" as the echo failure's cause: the waited-for string never
arrives because it was destroyed, 30 seconds is not too few, and #1442's command had already
answered.

## Remediation candidates, priced; none enacted

- **Raise the echo wait (30s to 60s).** Fixes a mode absent from the sample (genuine slowness);
  the observed echo reds never see the string at all. Cost: a real hang reports twice as late.
  Not recommended on this data.
- **Retry the failing leg once inside the gate.** Mechanizes what the constant doc already tells a
  person to do (swish_check.rs:1211-1212). Cures both signatures as observed (both rerun green).
  Cost: one extra leg of CI time only on red, and a deterministic interleave defect would surface
  as intermittent rather than red, trading loudness for throughput.
- **Raise QEMU `-m`.** Buys nothing: both signatures are bounded by the progenitor's internal pool
  and by a two-writer race, not by machine RAM. Not priced further.
- **The root fixes, both already tracked.** The interleave class dies when the kernel stops writing
  the UART once userspace owns it (§175, ruled 2026-09-27, unbuilt; the degauge doc points there).
  The OOM race dies when the undertaker tells the progenitor a job's memory is back
  (`design/roadmap/proposals/a-job-is-finished-when-its-memory-is-back.md`). Both are architect
  territory, not this lane's.
- **Runner variance as the finding.** Both confirmed signatures are races whose windows scale with
  runner speed; the local host cannot reproduce them in 20 legs and CI's slower runners can in one
  evening. If the flake needs one sentence: it is the gate meeting CI's machines, not a defect in
  what the gate measures.
