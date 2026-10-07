# The Actions cache budget, 2026-10-07 (provisional name)

Measured 2026-10-07 (UTC), read-only, at base `1f5be6843`, by lane/actions-cache-budget. Pull
request #1810 (measure Kani's install instead of caching it) found the repository's Actions caches at
10.8 GB in 461 entries, over GitHub's 10 GB limit, and rust-cache logging `No cache found`. calef
approved measuring it the same day. The question: which caches earn their bytes, and what layout
fits under 10 GB with room to spare. The scripts are in
[actions-cache-budget-2026-10-07/](actions-cache-budget-2026-10-07/), with the commands in its
`README`.

Result.

- `main` holds no rust-cache entry at all. All 463 rust-cache entries sit on pull request refs (301,
  6.8 GB) or merge-group refs (163, 3.8 GB). The five `main` entries are QEMU twice, the patched
  Kani, the vendor pins and toolchain-drift.yml's rust-cache entry: 0.43 GB, which pull
  requests and merge groups do restore.
- No merge-group job restored a rust-cache entry: 0 of 111. None can. A run restores from its own
  ref and from `main`, and every merge group gets a fresh `gh-readonly-queue/...` ref. Those saves
  were 39% of the bytes written and evicted entries that do hit.
- Pull request jobs hit 96 of 243 times (40%: 58 exact, 38 on the restore key), and only on a
  second run of the same pull request. A pull request's first run always misses, because `main` has
  nothing to fall back to.
- A hit is cheap and buys little. Restores took 0.3 to 3.1 s (median entry 19 MB). Job medians on a
  hit were within 0.4 minutes of a miss for 14 of 17 jobs; only clippy (2.0), prove (2.7) and
  falsify-shard (3.7) gained more, on 4 to 11 hits each.
- The 16 to 20 s #1810 attributed to the restore is mostly not the restore. rust-cache spends a
  median 17 s computing its key before it looks for an entry (p10 13 s, p90 21 s), on a hit and on a
  miss alike.
- The eviction horizon is 12 hours. No entry had gone unaccessed for longer, so an entry not touched
  within half a day is gone whatever its worth.

This pull request stops merge groups saving rust-cache entries, with a `script/lint` check, and on
calef's rulings drops rust-cache from 14 of the 17 jobs (section 5).

## 1. What the cache holds

`gh api repos/nifeos/nife/actions/caches --paginate`, 2026-10-07 about 14:50 UTC. Usage then read
10,733,031,662 bytes in 455 entries; the paginated list returned 469 distinct entries, 10.99 GB, as
entries came and went during the read.

| Ref | Entries | GB | Never restored |
|---|---|---|---|
| pull request (`refs/pull/N/merge`) | 301 | 6.78 | 210 |
| merge group (`gh-readonly-queue/main/...`) | 163 | 3.78 | 163 |
| `main` | 5 | 0.43 | 1 |

"Never restored" means `last_accessed_at` within ten minutes of `created_at`. GitHub updates the
access time on a restore, so this is a lower bound on waste, not an exact count.

By producer, the rust-cache entries split evenly across 17 jobs in ci.yml and verify.yml, from clippy
(39 entries, 1.35 GB, 35 MB each) down to prove-kernel-riscv64 (18, 0.21 GB). No one job is the
problem; the count of jobs times the count of refs is. Each job saves its own entry
(`add-job-id-key`), on every pull request and every merge group.

**Why `main` has none.** A′ (notes/merge-queue.md, 2026-09-24) skips the heavy jobs on a push to
`main` because the merge group already ran them. That note predicted this: "A′ lets `main`'s
Actions caches go stale ... Nobody has measured it yet." It is now measured, and it is worse than
stale. The rust-cache entries `main` once had have been evicted, and nothing writes new ones.

## 2. Hit rates and what a hit buys

Method. The last 40 runs each of ci.yml and verify.yml, the 34 that concluded success or failure,
618 jobs, 354 of them with a rust-cache step. Each job's log was read for the step's start, its
`Cache Configuration` line (key computed), the restore's outcome (`No cache found`, `Cache hit for`,
`Cache hit for restore-key`), the next step's start, and the post-job save.

| Event | Jobs | Exact hit | Restore-key hit | Miss | Key computation (median) | Restore (median) | Saves |
|---|---|---|---|---|---|---|---|
| pull_request | 243 | 58 | 38 | 147 | 17.6 s | 0.3 s | 185 |
| merge_group | 111 | 0 | 0 | 111 | 17.1 s | 0.2 s | 111 |

Saves took a median 2 s, and up to 6 s for clippy. Across the 354 jobs, rust-cache spent 102 runner
minutes computing keys, 12 saving and 3 restoring, about 3.5 minutes a run.

What a hit buys, as median job wall time on a hit against a miss:

| Job | Hits | Misses | Hit | Miss | Gain |
|---|---|---|---|---|---|
| falsify-shard | 11 | 9 | 16.7 min | 20.4 min | 3.7 |
| prove | 10 | 19 | 14.9 | 17.6 | 2.7 |
| clippy | 4 | 19 | 4.0 | 6.0 | 2.0 |
| fuzz | 4 | 16 | 11.6 | 12.0 | 0.4 |
| the other 12 | 2 to 11 | 8 to 29 | | | -2.3 to 0.2 |

The rest are noise around zero (cpu-matrix-shard's -2.3 is a slower hit). The reason is in what
rust-cache keeps. It caches `target/` after removing the workspace's own crates, so a hit saves
compiling registry dependencies only. This tree's heavy jobs spend their time building its own
crates for bare-metal targets and running QEMU, which no cache entry holds.

**Where the 17 s goes.** rust-cache 2 (commit `6323deb`, `src/config.ts` and `src/workspace.ts`)
runs `rustc -vV` once per installed toolchain and `cargo metadata --all-features` per workspace
before restoring, and hashes every `Cargo.toml` it finds. In the sampled clippy job the pinned
nightly was already installed when the step ended, with no install output in the log, so some jobs
pay rustup's install of `nightly-2026-10-06` inside this step. In the 140 jobs where that install
shows later in the log, the step still took a median 15 s. How much of the 17 s cargo would pay
later anyway (the registry index fetch `cargo metadata` performs) was not separated; it needs one
job run with and without the step.

## 3. Prior art, read

- GitHub's dependency-caching reference: "Workflow runs can restore caches created in either the
  current branch or the default branch", and a pull request also its base branch; runs "cannot
  restore caches created for child branches or sibling branches". Eviction deletes "in order of last
  access date, from oldest to most recent", entries idle for 7 days go regardless, and the 10 GB
  limit can be raised at a storage cost. The page says nothing specific about `merge_group`; the 0 of
  111 above is the evidence for it.
- rust-cache's README documents `save-if`, with `save-if: ${{ github.ref == 'refs/heads/master' }}`
  as its example "when only runs from `master` should be saved", and `shared-key` for one entry
  across jobs.
- bevy (`.github/workflows/update-caches.yml`): CI jobs use `actions/cache/restore` only, and one
  workflow on push to `main`, plus a nightly schedule, builds and saves the caches.
- tokio (`ci.yml`): plain `Swatinem/rust-cache@v2` per job, saving everywhere. It has no merge
  queue.
- rust-lang/rust (`ci.yml`): `sccache` against an S3 bucket (`SCCACHE_BUCKET:
  rust-lang-ci-sccache2`) and its own cache domain, not the Actions cache for compiler output.

## 4. What changed in this pull request

Every `Swatinem/rust-cache` step in ci.yml and verify.yml (18 at first) carries
`save-if: ${{ github.event_name != 'merge_group' }}`. A merge group still restores (from `main`,
when `main` has something), so nothing that hits today stops hitting. The measurement is at the first
step in ci.yml.

`helpers/cache_save_scope.py`, run by `script/lint`, refuses a rust-cache step without that scope in
any workflow whose `on:` lists `merge_group`. Watched failing on the 18 steps at base `1f5be6843`.

Expected effect: about 39% fewer bytes written, so the 12-hour horizon should stretch toward 20
hours at the same pull request traffic. The cache will still fill to the limit, as an LRU cache
does. Being at the limit is not the harm; evicting entries before their second use is.

## 5. One job without rust-cache, then the drop

stack-frame-check ran without the step on this branch (job 112877874401, run 37646272872): 115.7 s,
against a median 112.8 s over the 20 runs with it (range 79.9 to 120.0). The step's 19.6 s did not
vanish. The first `script/ci-build` step took 44.1 s against a median 23.0, because it now paid for
what rust-cache's `cargo metadata` and `rustc -vV` had triggered. So dropping the step costs no time
and saves none on this job; it saves the bytes and one moving part. One sample, inside the range.

**Correction: 14 jobs, not 13.** The fork and the first draft of this note said 13 jobs gained under
half a minute. fuzz gained 0.4 minutes, which is also under it, so the count is 14 of 17.

Dropped, on calef's ruling: bench, boot-check, coverage, cpu-matrix-shard, fuzz, image-permissions,
interleavings, stack-frame-check, supply-chain, swish-check, swish-check-x86_64 and test in ci.yml,
prove-kernel-aarch64 and prove-kernel-riscv64 in verify.yml. watchdog also lost its step; it ran in
none of the 34 sampled runs, so it is unmeasured, and it is neither of the three kept jobs. Kept:
clippy, prove and falsify-shard, each saving only outside merge groups.

## Caveats, together

- 34 runs over about 12 hours on one day, eight pull request branches. The per-job gains rest on 2 to
  11 hits each, and job wall time varies with the change under test, so read them as signs and
  orders of magnitude.
- "Never restored" from access times is a lower bound.
- rust-cache's key includes every installed toolchain's version. Several ci.yml jobs install the
  floating `nightly` beside the pinned one, so their restore key changes when nightly moves. Within a
  12-hour horizon that cost nothing measurable.
- The `actions/cache` steps (QEMU, the patched Kani, vendor pins) hit `main`'s entries today. When
  one of their keys changes, the new entry is saved on a pull request or merge-group ref and `main`
  gets one only from a push that runs in full. Not measured; noted in the helper's `BUGS`.

## Forks, and calef's rulings

calef ruled all three on 2026-10-07 (UTC), on #1814.

### (a) Where should a pull request's first run get a warm cache?

`main` has none, so every first run misses. The options were:

1. Nothing more.
2. bevy's layout: a workflow on push to `main` saves, and every job restores only. It costs a build
   per merge.
3. Drop rust-cache from the jobs where a hit gained under half a minute. Keep it on clippy, prove
   and falsify-shard, with `shared-key` where jobs build the same graph.

**Ruled: option 3, measured on one job first, and without `shared-key`.** The three kept jobs build
different graphs (clippy lints the host and kernel crates on x64, prove and falsify-shard run Kani,
falsify-shard on arm64), so one shared entry would be overwritten by each in turn and serve none.
Section 5 records the measurement and the drop.

### (b) sccache or another backend?

**Ruled: no remote compile cache.** rust-lang/rust's model needs a bucket, credentials and a new
dependency, and what it would cache is dependency compiles, which are not where these jobs spend
their time. Revisit if job timings show dependency compiles taking real time.

### (c) Something that shows this before a lane trips over it

**Ruled: yes.** The proposal is now milestone 822 (the Actions cache horizon is charted),
[design/roadmap/0822-the-actions-cache-horizon-is-charted.md](../design/roadmap/0822-the-actions-cache-horizon-is-charted.md).
Its number is provisional until the queue lands it. It samples bytes against 10 GB and the eviction horizon daily and charts them in
notes/project-metrics.md. The horizon is the number that matters; a full cache with a week's
horizon is healthy.
