---
status: BUILT
raised: 2026-08-03
built: 2026-08-04
---
# 81. An HVF leg: the test suite on the physical core

Built 2026-08-04 (PR #95). Raised 2026-08-03, same survey as 79.

The infrastructure exists and is used one-sidedly: `NIFE_ACCEL=hvf` runs the kernel on the
physical Apple Silicon core, `script/bench --real` and `script/server --hvf` use it, and `script/test`
has never run there as a habit. That leaves the suite's only execution environments emulated ones,
while a machine with real caches, real reordering, and a real GIC sits under the emulator the whole
time. Until the board arrives this is the only real silicon this project can test on at all.

Per-PR in hosted CI is not an option: GitHub's macOS arm64 runners are themselves virtual machines
and do not expose nested virtualization, so HVF is unavailable there. A self-hosted runner on the
dev machine would close that gap and is deliberately not part of this milestone; it couples CI to a
laptop that sleeps.

**The delivery vehicle is `script/gates`** (calef, 2026-08-03), not a new script and not a habit a
human has to remember. `script/gates` already exists for exactly this reason: it is the one command
a person or an agent runs before pushing, and it was created because "three commands is two too many
to remember at the moment you are about to push". An HVF leg that lived anywhere else would be a
fourth command, which is the failure gates was built to end. So gates grows a final leg: when the
host has HVF (macOS on Apple Silicon with a capable QEMU), run the aarch64 suite again under it;
when it does not, **skip loudly**, one line saying what was skipped and why, so a CI transcript can
never be misread as having had silicon coverage. Every lane on this machine then carries the leg for
free, and the integrator's merge run does too, through the same wrapper.

The work: wire `--hvf` through `script/test`/`cargo xtask test` (today it is an env var the runner
script reads), run the full aarch64 suite under it, and fix or honestly record what differs. Timer
behavior will differ, because under HVF guest time is host time and no icount instrument exists;
part of the milestone is learning which tests that perturbs, which feeds milestone 78's per-assertion
work rather than competing with it. Then the gates leg, and notes/scripts.md.

## Scope note

HVF covers aarch64 only, with `-cpu host` mandatory, so it is one machine and no model variation;
the cpu matrix keeps its job. riscv64 has no equivalent until the board lands, so the leg's honest
name is "the aarch64 suite on real silicon", not "the suite on real silicon".

Gates' charter is checks that run locally in minutes, so the leg must be timed before it is added:
native execution should beat TCG comfortably, but that is a number to measure, not assume. If it
proves slow, the fallback is a `--full` flag rather than silent omission, and the skip-loudly line
says which mode ran.

## Follow-on

- **Milestone 78.** The load-sensitive assertions. Every failure the physical core produced belonged
  to that family, found from the opposite direction (a fast machine burning yields in nanoseconds
  rather than a slow one burning them under load), and none of them was the timer perturbation this
  block expected.
- **Recorded.** The leg was timed before adoption, which was this block's own condition, and the
  `--full` fallback turned out not to be needed: 12 to 16 seconds added to the pre-push run against
  roughly 3x on the suite. `notes/hvf-leg.md` carries the measurement.
- **Recorded.** One machine, one model, `-cpu host` mandatory, and no riscv64 equivalent, so the
  leg's honest name stays "the aarch64 suite on real silicon". `notes/hvf-leg.md`.
- **Recorded.** Semihosting is not answered under HVF, so a failing run ends in an unbounded panic
  loop on four cores and the host has to stop reading and kill the child. Anyone driving the runner
  by hand gets a QEMU that never stops. `notes/hvf-leg.md`.
- **Recorded.** The leg is not a CI gate and cannot be one: nothing enforces that it ran, and
  running `script/ci-build` with no arguments is a convention. `notes/hvf-leg.md`.
- **Refused.** A self-hosted runner on the dev machine, which is the only way to put HVF in hosted
  CI, since GitHub's macOS arm64 runners are themselves virtual machines with no nested
  virtualization. It couples CI to a laptop that sleeps, and the loud skip was taken instead so a
  transcript can never be misread as having had silicon coverage.
  This refusal is milestone 488 (design/roadmap/488-a-self-hosted-ci-runner.md), which carries it
  with the condition that would change it.

## Index row

`NIFE_ACCEL=hvf` exists and bench uses it; nothing runs `script/test` there as a habit. GitHub's
hosted runners cannot (no nested virtualization), so the leg rides `script/gates`: it runs
wherever HVF exists and skips loudly where it does not. aarch64 only
