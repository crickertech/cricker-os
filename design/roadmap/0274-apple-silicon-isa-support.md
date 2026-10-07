---
status: NOT-STARTED
raised: 2026-09-10
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 274. Apple Silicon's own core is untested, and not for the reason first recorded

Minted 2026-09-10 by calef, from a question about `neoverse-n2`.
Corrected the same day, after calef asked what the fix would actually be for this Mac: the
maintainer's first answer rested on a run that never used HVF at all, and the real HVF run, taken to
answer the follow-up honestly, found a different and larger blocker. *(Number provisional until the
merge queue lands it.)*

Measurement, not a design fork.

## The correction, first, because it is the reason this block reads the way it does

The first version of this block claimed a direct HVF measurement of Apple Silicon's `FEAT_RNG`
support from `cargo xtask test --arch aarch64` (no `--hvf` flag). **That command does not run under
HVF.** `xtask`'s `test()` only takes the HVF leg when `--hvf` is passed on the command line
(`xtask/src/main.rs`, `let hvf = std::env::args().any(|a| a == "--hvf")`); without it, the aarch64
leg runs under TCG's default `cortex-a72`, the same model `notes/entropy.md` already documented
as predating `FEAT_RNG`. So the original "3 skipped" result, including the RNDR skip and the
PMUv3 pass, measured **QEMU's emulated `cortex-a72`, not the physical core**, and told this project
nothing it did not already know.

Caught by actually running the real thing (`cargo xtask test --arch aarch64 --hvf`) to answer
calef's direct question rather than re-asserting the wrong answer with more confidence.

## What the real HVF run found, 2026-09-10

```
qemu-system-aarch64: HVF does not support GICv2 emulation
qemu-runner-aarch64: QEMU refused virt,accel=hvf,gic-version=2,iommu=smmuv3
test --hvf: nothing ran.
```

The HVF leg cannot boot this kernel at all, on any test, for any reason, today. This is not new:
milestone 222 (a leg fails instead of skipping) and milestone 227 (*GICv3 driver*) both already exist, and
`notes/interrupts.md`'s `BUGS` section already carries the full detail: `kernel/src/drivers/gic.rs`
speaks GICv2 only, and QEMU 11.1.1's HVF accelerator requires GICv3. `script/gates` already skips
this leg out loud rather than failing on it.

So the honest state is: nothing about `FEAT_RNG`, or anything else, can currently be measured on
this Mac's physical core, and the reason has nothing to do with entropy. It is a pre-existing,
already-tracked interrupt-controller gap that blocks HVF outright.

## What this milestone actually is, now that the premise is corrected

Not a fresh finding about Apple Silicon's ISA. It is the much narrower thing that survives:

1. This test has passed exactly once in this project's history, established independently of the
   HVF confusion above, by reading the record directly rather than running anything:
   `.github/workflows/ci.yml`'s `test` job runs `script/ci-build` on `ubuntu-24.04-arm` over
   QEMU/TCG with no `NIFE_CPU` set, so it takes the default `cortex-a72`. The `cpu-matrix` job that
   does sweep CPU models is riscv64-only by its own name. Milestone 162, which built this test,
   already recorded that the default skips it, and its correctness claim rests on one manual
   `script/test --arch aarch64 --cpu neoverse-n2` run at the time it landed. Nothing since has
   repeated that leg. `321 passed, 3 skipped` (or `328 passed, 2 skipped` on riscv64) has read as
   clean in CI and on this dev machine on every run since, and nothing compares the skip count
   against a baseline to notice that one of them has never actually been exercised as designed.
2. Whether Apple Silicon implements `FEAT_RNG` is now genuinely unknown, not "confirmed absent."
   Finding out needs either milestone 227's GICv3 driver landing first (so HVF can boot at all), or
   some other way to read the core's ID registers that does not require booting nife under HVF,
   which is out of this milestone's scope to invent.

## What this needs

1. Give aarch64's TCG leg the same CPU-matrix coverage riscv64 has, so `neoverse-n2` runs on
   every CI merge instead of having run once, by hand, over two weeks ago. This is real,
   independent of anything about HVF.
2. Correct `entropy_tests.rs`'s comment, which currently reads as though `--cpu neoverse-n2`
   reliably fixes the skip. It fixes the TCG case only; there is currently no fix under HVF, and the
   reason is milestone 227, not this test.
3. **Do not attempt to answer the Apple Silicon ISA question from this machine** until milestone 227
   lands. Checking radon (the JH7110's own core) is a separate, unrelated question about a different
   chip and stays open regardless.

## BUGS

- A skip that has stood in for a pass since roughly 2026-08-25 was not caught by any gate, and
  this part of the finding did not depend on the corrected premise: `script/lint` and CI's own
  required-check machinery both consider `321 passed, 3 skipped` a clean run, and nothing compares
  the skip count against a baseline the way other counted-claims checks in this tree do. Same shape
  as milestone 268's finding 4 (the tour's checks report failure by printing a word and continuing):
  a real assertion existed, and nothing was reading its result.
- This block's own first version is the cautionary case for AGENTS.md's "correct yourself
  loudly" rule. It stated a confident, specific, wrong claim about real hardware, in a roadmap
  block calef was about to act on, because a flag that was never passed was assumed to have been.

## Follow-on

- **Milestone 227.** Blocks any real measurement of Apple Silicon's ISA from this machine.
- **Milestone 222.** Already covers HVF's silent-failure shape; this is a second instance of the
  same class, now on record.
- **Milestone 268.** The same class of defect as this block's BUGS entry: a real check whose
  verdict nothing enforces. Worth reading together rather than as coincidence.

## Index row

calef, 2026-09-10. Corrected same day: HVF cannot boot this kernel at all yet (blocked on 227), so
nothing about the core's ISA is measured; this test has still passed exactly once in the project's
history, by hand.
