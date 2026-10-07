---
status: BUILT
raised: 2026-09-15
built: 2026-09-15
---
# 300. Decompose the icount baseline drift, and re-baseline only what is proven

Built 2026-09-15. *(Number provisional until the merge queue lands it.)* Re-saving a
baseline commits a new performance floor, which is an architect's call like any baseline save; this
milestone was briefed with that latitude and executes Decision 1 of the finding below. Decision 2
stays open for calef as the follow-on.

Amended 2026-09-15: PR #885's first pass classified the drift as an intended feature cost and
re-baselined to absorb it. That was wrong: milestone 237 ships the cycle-counter grant OFF, so the
cost was a removable regression, an un-`#[cfg]`'d switch tuple, the same class milestone 299 fixed
for the port grant. A follow-on lane fixed it, confirmed the recovery on both ISAs by measurement,
and re-saved the baselines against the RECOVERED numbers, superseding #885's re-baseline. The
decomposition (QEMU ~0, toolchain ~0, code is the whole move, bisected to `57399c34`) stands
unchanged; only the classification of the residual and the saved floor changed. The correction is in
"What the decomposition found" below.

This milestone promotes and carries the proposal *"the icount baselines predate the pinned nightly"*
(surfaced by milestone 299's lane, PR #883). The finding, verbatim, then what the decomposition
found, which corrected the finding's own hypothesis.

## The finding, as surfaced

`bench/baseline-{aarch64,riscv64,x86_64}.txt` are the icount regression tripwire's committed floors.
The aarch64 and riscv64 baselines were last saved 2026-08-27 (`a79fdb95`). Over the following ~19
nightly bumps and ~150 commits, all individually under the 10% tripwire, `main` drifted to ~+6.5%
over those two baselines on the switch-heavy benchmarks. The tripwire fires at +10%, so most of the
headroom was spent, and the next unrelated change adding a few percent would trip the gate for a
reason that is not its own. The proposal read this as the pinned nightly's codegen (PR #880 bumped
to `nightly-2026-09-15` changing only `rust-toolchain.toml`, without re-saving the baselines a new
nightly's codegen was assumed to invalidate) and asked for two things:

1. Re-measure and re-save the baselines on the new nightly (the decision, a new floor committed).
2. Make a toolchain bump re-baseline in the same PR, or fail loudly, so this stops being found by
   accident.

## What the decomposition found

The premise in (1) that the drift was the nightly's codegen turned out to be false, and proving
it was the point. Holding QEMU at 11.1.1 and measuring a 2x2x2 grid of {baseline code, HEAD code} x
{nightly-08-27, nightly-09-15}, plus the QEMU term (the dev Mac moved 11.0.2 -> 11.1.1 on 2026-08-28,
after the baseline was saved), splits the drift into three:

- QEMU (11.0.2 -> 11.1.1): ~0. The emulator upgrade does not move icount. Measured, not assumed.
- Toolchain (nightly-08-27 -> nightly-09-15): ~0. The two endpoint nightlies emit byte-identical
  instruction counts on the same code. The nightly did nothing across this window.
- Code (a79fdb95 -> HEAD): the entire move, and the same move on both ISAs (near-identical
  percentages, which codegen noise would not produce).

`git bisect` pinned the code component to **one commit**: `57399c34` (2026-09-02), *"sched: write the
cycle-counter grant at the context switch"*, **milestone 139 / DECISIONS 139 option 4**. It adds a
per-context-switch grant write: `yield_switch` and `ctx_switch` move by the identical +43.3
ticks/switch, and every IPC benchmark moves in proportion to its switch count, while pure compute
(`coremark`) and the no-switch map primitives do not move. Milestone 299's `44890a8a` later trimmed
the aarch64/riscv peak to the net ~+35.7 ticks/switch at HEAD.

The bisect pinned the cost to 139, and PR #885 read that as intended feature work and re-saved the
baselines to absorb it. **That classification was wrong, and this block records the correction.**
Milestone 237 (the cycle-counter grant) had already made the cycle-counter grant a
*measurement-only* feature that ships OFF: `set_cycle_counter_grant` is `#[cfg(any(test, feature =
"cycle_counter_grant"))]` and does not appear in a feature-off binary at all. So the cost 139
introduced was not paying for a shipping feature; it was a residual 237's gating left behind, the
const-`false` `next_cycle_counter` element still threaded through the shared switch tuple (a
`#[cfg]` is not allowed on a tuple element, which is why 237 reached for a fold instead). That fold
works in the release build but NOT in the debug build the icount gate measures, so the read, the
tuple element, and a gated-off `install` call all stayed in the shipping switch. This is the exact
class milestone 299 (the x86 port-range capability) fixed for the port grant, and the fix is 299's:
carry the grant in a `#[cfg]`-gated local read and installed at the switch site, keep the tuple at
its pre-139 width. Recovered, measured on both ISAs (#885 floor -> fix, near pre-139):
`yield_switch` 1167649->1101149 aarch64 / 195910->184875 riscv64, `ctx_switch` 3089320->2922971
aarch64 / 522565->495050 riscv64, ~91-93% of the drift, the ~0.5% residual within the codegen noise
floor `coremark` sits in (it stayed flat, 20915884->20915599 aarch64). The baselines are re-saved on
`nightly-2026-09-15` against these RECOVERED numbers. The full per-benchmark grid, the bisect, the
recovery table, and the standing "save from the shipping feature set" rule live in
[notes/benchmarks/drift-decomposition.md](../../notes/benchmarks/drift-decomposition.md) under the
2026-09-15 heading.

**x86_64 was NOT unaffected**, which corrects both #885 and this lane's own brief. The residual was
the *shared* switch tuple, not an aarch64/riscv-only path, so the x86 debug icount build carried the
const-`false` element too and paid for it: the fix recovers `yield_switch` 20080160->18903108 (~5.9%)
and `tss_iomap_switch` 24818161->23661444 (~4.7%), with `coremark` flat (306262395->306261408).
`bench --x86 --check` still passed against the #885 floor only because the residual sits under the
10% tripwire, but leaving that floor ~5.9% high would bake in exactly the removable regression this
milestone removes, so x86_64's baseline is re-saved against its recovered numbers as well.

## What was built

- The removable regression fixed in `kernel/src/sched.rs`: the cycle-counter grant no longer widens
  the shared context-switch tuple, it is read and installed behind
  `#[cfg(any(test, feature = "cycle_counter_grant"))]` at the switch site (299's structure), so the
  shipping tuple is back to its pre-139 width. `kernel/Cargo.toml`'s `cycle_counter_grant` block
  updated to match: the measurement build's on-cost stands, but a feature-off boot now pays nothing.
- All three baselines (`bench/baseline-{aarch64,riscv64,x86_64}.txt`) re-saved on
  `nightly-2026-09-15`, QEMU 11.1.1, against the RECOVERED numbers (superseding #885's floor), so the
  committed floor tracks the current toolchain with the regression removed. The switch tuple is
  shared across ISAs, so x86_64 recovered too (~5.9% on `yield_switch`) and is re-saved rather than
  left at the inflated floor. `bench --check` passes on all three architectures against the re-saved
  floors.
- The per-benchmark decomposition (baseline / QEMU / code / toolchain components), the bisect, and the
  recovery table recorded in `notes/benchmarks.md`, with the corrected classification.
- The `rust-toolchain.toml` pin is unchanged at `nightly-2026-09-15` (the old-nightly pin used for
  measurement was never committed).

## Follow-on

- **Milestone 302.** Decision 2 of the finding, and calef ruled it on 2026-09-16: **fail loudly**,
  rather than have a bump re-baseline itself. `script/toolchain-bump` raises the pinned nightly
  without touching the baselines, and this milestone proves the drift they leave is real (even where,
  as here, the culprit turned out to be code rather than the nightly). Written up as
  `design/roadmap/proposals/a-toolchain-bump-that-leaves-the-baselines-stale.md` and promoted to
  milestone 302, which folds in calef's second ruling of the same day: that a `--save` records its
  reason in the baseline file. The two are one mechanism, because a stale-baseline check can only
  exist if the file records which nightly produced the numbers, and today nothing does.
- **Done.** DECISIONS 139 estimated its switch cost as the compare `switch_user_root` already
  pays (~2-3 ticks when nothing is granted). #885 measured ~+35.7 ticks per context switch and
  guessed a non-inlined arch function on the hot path; that guess was wrong. The arch write
  (`set_cycle_counter_grant`) ships OFF and is absent from a feature-off binary, so it was never the
  cost. The cost was the un-`#[cfg]`'d tuple residual, now removed, and the recovered numbers land
  within ~0.5% of 139's pre-feature baseline: the estimate was right and the delivery now matches it.
  The measurement and the recovery table are beside the feature in `notes/benchmarks.md` (the
  2026-09-15 section).

## BUGS

- A re-baseline commits a floor measured on one machine, one nightly, one QEMU. These numbers are
  TCG icount on the dev Mac. A different runner moves them again, which is the whole reason the
  follow-on argues a bump should re-baseline mechanically rather than have a human eyeball a percent.
- **The QEMU term was measured out, not eliminated.** Only 11.1.1 is installable locally, so the
  11.0.2 baseline could not be re-run on its original emulator; the QEMU component was isolated by
  holding 11.1.1 across the code and nightly axes and reading the same-code/same-nightly pair (B - A),
  which came back ~0. If a future QEMU does move icount, this method still separates it.

## Index row

Decomposed the ~+6.5% icount drift over the 2026-08-27 baselines into QEMU (~0), toolchain (~0), and
code components, refuting the proposal's nightly-codegen hypothesis: the whole move bisected to one
commit, milestone 139's cycle-counter grant at the context switch (`57399c34`). PR #885's first pass
read that as an intended feature cost and re-baselined to absorb it; this block corrects that. The
grant ships OFF since milestone 237, so the cost was a removable residual, the const-`false`
switch-tuple element 237's fold did not eliminate in the debug icount build, the same class milestone
299 fixed for the port grant. Applied 299's `#[cfg]`-gated fix, recovered ~91-93% of the drift on
aarch64/riscv64 (`yield_switch`/`ctx_switch` back within ~0.5% of pre-139) and ~5.9% on x86_64 (the
tuple is shared across ISAs, so x86 was not unaffected as the brief assumed), and re-saved all three
baselines against the recovered numbers. Decision 2 (a toolchain bump must re-baseline or fail
loudly) left open as the follow-on.
