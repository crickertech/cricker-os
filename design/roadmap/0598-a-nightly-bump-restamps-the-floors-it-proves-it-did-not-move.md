---
status: BUILT
raised: 2026-09-26
built: 2026-09-26
promoted_from: a-nightly-bump-restamps-the-floors-it-proves-it-did-not-move
---
# 598. A nightly bump restamps the icount floors when it proves it did not move them

Built 2026-09-26. *(Number provisional: minted by the lane, to be confirmed at merge.)*
Promoted on 2026-09-26 from the proposal `a-nightly-bump-restamps-the-floors-it-proves-it-did-not-move`,
which the lane `lane/bump-baselines` filed the same day on #1336 after being briefed to stop the
daily toolchain bump needing a person. calef ruled on it the same day, and this lane built it.

## The ruling

calef, 2026-09-26 UTC, on #1336: "Yes" to option 4 below. The bump workflow benchmarks the old and
the new nightly on one runner. If every row moved less than 0.5% and the cumulative move since the
last person-made save is less than 2%, it rewrites only the `# toolchain:` stamp and adds a `# why:`
line with the measured moves. The floors never change. Otherwise the branch stays red with the table
in the pull request body. **The stamp now means "last proven valid for".** The daily cadence stays.

This overturns part of the 2026-09-21 refusal: the part that required a person on every bump. The
part it keeps is that no machine writes a floor number. The records of the refusal now cite this
ruling: the `script/lint` comment and failure text, the `toolchain-bump.yml` header,
`script/toolchain-bump`, `notes/benchmarks.md` and `notes/benchmarks/icount-drift-and-provenance.md`.
Milestone 542 (gates that stopped meaning what they measure) lists the refusal as it stood when 542
was built, and is left as that history.

## The problem, measured

Since the stamp check landed (commit `99f13dad`, 2026-09-21), every bump that built has failed
`script/lint` until someone ran `script/bench --save` on three ISAs on the branch: #1112, #1250,
#1308, #1335. The branch is force-pushed daily, so a day the person misses loses the work. The
save itself admits what it cannot say: each `# why:` line on those saves reads "this mixes the
compiler with drift on main since the last save, not separated".

What #1335 (nightly-2026-09-26) showed, measured on patagonia on the bump branch:

- The compiler term is small and separable. The same tree built with nightly-2026-09-25 and
  nightly-2026-09-26: x86_64 identical on all 11 rows; aarch64 differs on 8 of 14, largest
  `spawn_el0` 0.09%; riscv64 differs on 9 of 15, largest `ipc_rtt` 0.21%. Repeated runs on one
  nightly are identical to the tick, so these are the compiler's.
- CI's host is a different instrument. CI's bench job on the same tree and nightly differs from
  patagonia on 5 aarch64 rows and 7 riscv64 rows, largest riscv64 `ipc_rtt` 170436 against 171468
  (0.6%). x86_64 matched exactly. The host term is larger than the compiler term.
- Milestone 300 (decompose the icount baseline drift, and re-baseline only what is proven) found the toolchain term across nightly-2026-08-27 to nightly-2026-09-15 was ~0.

## Options, as proposed

1. Status quo. A person re-records daily. Keeps the ruling. Loses: the daily person, and the
   save blesses whatever drift sat on `main` into the floor, which is milestone 415 (sub-tripwire drift accumulates across baseline saves)
   happening on a schedule.
2. The bump workflow re-saves the floors in CI (the brief's suggestion). Refused on two counts.
   It is exactly what the 2026-09-21 ruling refuses: the floors would track the compiler, so a slower
   nightly could never be seen. And a CI save would record the floors on a host that reads up to
   0.6% differently from the one every other save used, so each bump would swap instruments.
3. Restamp only when the compiler term is zero. The workflow builds the tree twice (old pin, new
   pin) on one runner, and when every row is identical it rewrites `# toolchain:` and nothing else.
   Within the ruling, since no number moves and none could have. Loses: it would have handled only
   x86_64 today, so the person is still needed daily.
4. **Recommended: restamp when the compiler term is proven small, never re-save.** Same A/B as
   option 3. If every row's compiler term is within ε and the cumulative term since the last
   person-made save stays under a cap, the workflow rewrites `# toolchain:` and appends a `# why:`
   line with the A/B's largest moves. The numbers never change. Otherwise it leaves the branch red
   as today, with the A/B table in the pull request body so the person starts from the decomposition.
   Proposed ε 0.5% per row, cap 2% (a fifth of the 10% tripwire). Today's bump passes both.
5. Bump weekly. Composes with any of the above; cuts the person's cost sevenfold and makes each
   compiler step larger. The drift workflow still reports upstream breakage daily.

Option 4 keeps what the refusal protects. A slower nightly still uses up tripwire headroom against
numbers a person recorded, and the cap stops a run of small moves from being absorbed. Each restamp
records the term it proved, so the file tells a reader what the compiler has cost since the last
save. It also stops the bump from blessing `main`'s drift, which options 1 and 2 both do.

## The seven questions

1. Considered: the five options above.
2. In the tree: `metrics/weekly` already commits generated numbers from a bot (#1333), with a person
   merging. Milestone 300 did the same A/B by hand.
3. Prior art: not researched. From memory only, rust-lang's rustc-perf compares one benchmark
   across two compiler builds, which is option 3's A/B.
4. Premise: true. Four consecutive bumps needed the person.
5. Cost: CI's bench step runs all three ISAs in about a minute (#1335, 12:47 to 12:48 UTC). Option 4
   adds a second toolchain build and a second bench run to the bump job, plus one cold QEMU build on
   its arm runner (CI's cache is keyed per arch, 10 to 20 minutes once). Not built, so not measured.
6. Reversible: a workflow, an `xtask bench` restamp mode and a lint comment. No floor number is
   written by the machine, so reverting leaves nothing to undo. Nobody has acted on it.
7. Equal cost: still option 4 over option 1, because it separates the compiler from `main`'s drift
   and today's saves say they cannot.

## What was built

- `cargo xtask bench [--riscv|--x86] --restamp [--report <file>]` (name provisional), in
  `xtask/src/restamp.rs`. It reads the nightly the floor's `# toolchain:` line names, runs the leg
  under that nightly and then under the pin (`RUSTUP_TOOLCHAIN` for every cargo it spawns), and
  compares per row. The two bounds are constants there, beside the ruling. The cumulative term is
  carried as the last clause of each restamp's `# why:` line and compounded, so a `--save`, which
  rewrites the header, resets it by construction. Seven host tests cover the arithmetic, the strict
  bounds, the ledger round trip, and that a restamp leaves every row byte-identical.
- A `restamp` job in `.github/workflows/toolchain-bump.yml`, ahead of `propose`. It installs both
  nightlies, reuses CI's cached QEMU, raises the pin, restamps each leg, uploads the floors and the
  report, and fails when any floor still names the old nightly. `propose` then commits the restamped
  floors as their own commit on the bump branch and puts the table in the pull request body.
- A `target` input on `workflow_dispatch`. A dispatch on any ref other than `main` runs only the
  `restamp` job, which is how this was proven without touching the real bump branch.

## Proof

Dispatched on branches, never on `main`, with `target` set to a nightly other than the pin
(`nightly-2026-09-26`), so each run A/Bs a real pair of compilers on the same tree:

- **Run 36253402203**, `lane/bump-baselines`, target `nightly-2026-09-25`. aarch64 restamped
  (largest `ipc_rtt_el0` +0.275%), x86_64 restamped (every row identical). riscv64 refused:
  `ipc_rtt` moved +0.607% (170436 to 171470), over the 0.5% bound, so the job went red and the
  artifact's riscv64 floor was byte-identical to `main`'s. The two restamped files changed only
  `# toolchain:` and one added `# why:` line. `propose` was skipped, as it must be off `main`.
- **Run 36253497070**, `proof/restamp-falsify`, the falsification. A throwaway commit, never
  merged, inflated `ipc_rtt` by 0.6% in the new nightly's run. All three legs refused on that row
  (aarch64 +0.600%, riscv64 +1.210%, x86_64 +0.600%), no floor changed, and the job failed with
  all three files named as still red.
- **Run 36254193708**, `lane/bump-baselines`, target `nightly-2026-09-24`. aarch64 and riscv64
  restamped; x86_64 refused on `spawn_reap` +1.780%, the same row #1308's save moved -1.74% by hand
  on the step from 09-24 to 09-25. A real compiler term, caught.

No run restamped all three legs at once, because both available pairs of nightlies really moved one
row past the bound. So every leg has restamped and refused on a real runner, but the job's all-green
path (verdict `restamped`, then `propose` committing the floors) first runs whole on `main`.

The first run's refusal was not staged. riscv64 `ipc_rtt` sits in one of two modes about 0.6% apart
(170436 and 171469 on the same commit, depending on the checkout path and the host), and the
floor's recorded value has wandered by that much across saves since 2026-09-21. A nightly that flips
it keeps the bump red for a person, which is the bound doing what calef set it to do. It is recorded
below rather than tuned away.

## BUGS

- The A/B covers what the bench leg builds with the pinned toolchain. Userspace built through the
  `nife-dev` farm is packed into both runs identically, so a compiler term there is invisible. No
  bench row is known to depend on it.
- The term is measured on CI's `x86_64` runner and applied to floors recorded on patagonia. On
  #1335 the two hosts read the same tree up to 0.6% apart. That the ratio between two nightlies
  carries across hosts is assumed, not measured.
- `propose`'s half (download the floors, commit them, write the body) cannot run off `main`,
  because the `automation` environment only deploys there. It is proven by the first scheduled run
  after this merges, not before.
- riscv64 `ipc_rtt` is bimodal about 0.6% apart, so any nightly that flips its mode is refused by
  the 0.5% bound and needs a person. Attributing the two modes is the fix; widening the bound is a
  ruling, not a lane's call.
- icount on patagonia differs between two worktrees of the same commit: riscv64 `ipc_rtt` read
  170436 in one and 171469 in another. Paths baked into the binary are the likely cause (not
  bisected). An A/B is unaffected because both runs share a checkout, but a floor saved in one
  worktree and checked in another is not the same comparison.

## Follow-on

- **Recorded.** The host assumption, the `nife-dev` blind spot and the unproven `propose` half are in
  this block's `BUGS` and in `xtask/src/restamp.rs`'s, beside the flag a reader meets.
- **Recorded.** The worktree-path icount difference is in this block's `BUGS`. It does not affect a
  restamp; it affects any floor saved in one checkout and checked in another.
- **Recorded.** riscv64 `ipc_rtt`'s two modes, in this block's `BUGS`. Until they are attributed, a
  nightly that flips the row keeps that one floor for a person.
- **Milestone 415.** The cumulative bound here is per restamp run since the last save, not against a
  fixed historical anchor; 415 (sub-tripwire drift accumulates across baseline saves) item 3 still
  owns that.

## Index row

The daily nightly bump carries the icount floors across by itself when an A/B of the two compilers
on one runner proves no row moved 0.5% (2% cumulative). It rewrites only the stamp and never a
number, so a slower nightly still spends tripwire headroom against numbers a person read. Before
this, a person re-recorded three floors on every bump.
