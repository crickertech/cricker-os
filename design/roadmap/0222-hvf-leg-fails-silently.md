---
status: BUILT
raised: 2026-09-02
built: 2026-09-02
---
# 222. The one command a person runs before pushing has a leg that fails instead of skipping

Minted 2026-09-02 by the maintainer, from milestone 208's (the x86_64
kernel image ships an RWX segment) lane, which hit it while proving an unrelated boot path and
reproduced it outside this project. *(Number provisional until the merge queue lands it.)*

It was minted with no gate, on the grounds that the diagnosis was done and nothing here needed
hardware this project does not have. That held: the whole of it was measured and built on
patagonia, and the one thing that would have needed hardware, a GICv3 driver, was refused.

**In brief.** `script/test --hvf` fails:

```
qemu-system-aarch64: HVF does not support GICv2 emulation
```

Reproduced on 2026-09-02 with a bare `qemu-system-aarch64 -machine virt,gic-version=2 -accel hvf`
and **no nife kernel involved**, on QEMU 11.1.1. The runner asks for GICv2; HVF requires GICv3.

Why it is worth a block rather than a shrug. `script/gates` calls this *the one command a person
runs before pushing*, and the `--hvf` leg is where it reaches a real core rather than an emulated
one. Today that leg is skipped by failing, which is worse than either alternative: a leg that
runs tells you something, a leg that skips loudly tells you it did not, and a leg that fails tells
you something is broken without saying whether it is yours. A contributor meeting this has no way to
know the failure is the accelerator's constraint rather than their change.

That is AGENTS.md's third principle, the newcomer who must succeed without asking anyone, failing at
the exact command the tree tells them to run.

## What it needed, and what was decided

The measurement first, because the block said finding out was the first thing to do and the
answer changed the decision. `-machine virt,gic-version=3` had never been booted here; the note that
recorded that (`notes/interrupts.md`) expected "a recorded failure with an error message attached".
There is no error message. Under GICv3 the kernel boots the whole tour, brings four cores online,
prints `timer: 100 Hz tick, interrupts ON`, and then takes zero interrupts and zero preemptions,
with nothing faulting and nothing said. `memory::gic_regions()` matches the device tree node by name
(`intc@`) and takes its first two `reg` blocks, so a GICv3 node hands it the distributor and the
**redistributor**, and `gic::init_this_cpu` writes `GICC_PMR` and `GICC_CTLR` into registers that
are not there. The real CPU interface is a set of system registers nothing touches.

So the honest limitation is not "GICv3 is unsupported". It is "GICv3 boots and silently loses
every interrupt", which is the more dangerous of the two, and it makes the runner's
`gic-version=2` pin load-bearing rather than tidy.

The GICv3 route therefore lost, and it lost on size rather than on taste. Priced from that
failure in `notes/interrupts.md`: a version decision at init that nothing does today, a
redistributor frame per core, a system-register CPU interface (`ICC_*`, which is `msr`/`mrs` and so
belongs under `arch/aarch64/` by rule 1 rather than beside the MMIO driver), affinity routing
replacing the eight-bit `GICD_ITARGETSR` mask, and both drivers coexisting because GICv2 is what the
runner, the CI matrix, every recorded measurement and argon's own silicon use. Roughly ten call
sites reach `drivers::gic::` directly. That is a driver with its own tests and its own dispatch, not
a runner flag, and building it here is the failure mode this milestone was warned about: a small
number quietly growing into a large one. Proposed as its own milestone instead (see BUGS).

The loud skip won, and it reuses a mechanism this file already had rather than inventing one.
`script/gates` has carried four `hvf_missing` conditions since milestone 81 (an HVF leg: the test suite on
the physical core), each naming what did not run and why, with a closing line that says the run was
TCG only. This is a fifth, and the four before it ask whether HVF *exists* where this one asks
whether QEMU will start the machine the runner actually configures.

How it asks. `helpers/qemu-runner-aarch64.sh` grows a probe: `NIFE_PROBE=1` starts that script's
own `$MACHINE` paused (`-S`, so nothing executes), quits it from the monitor, and reports QEMU's own
refusal. It lives beside the machine string on purpose. A probe in a script of its own would have to
restate `virt,accel=hvf,gic-version=2,iommu=smmuv3`, and the day the runner's string changed the
probe would answer about a machine nobody runs, which is a false skip: the exact shape
`script/lint` has deleted three checks for. Nothing here tests a QEMU version number, so the answer
follows QEMU and this kernel without anyone remembering to update it. It costs about fifty
milliseconds, and it exits on its own, which matters because a nife kernel that reaches `halt()`
never does.

What a contributor sees now. `script/gates` prints `gates: SKIPPED the HVF leg (QEMU will not
start the machine this leg needs. It said: qemu-system-aarch64: HVF does not support GICv2
emulation. ...)` and closes with `TCG only; the HVF leg was skipped`. `script/test --hvf` still
fails, because that is a thing a person typed by name and a skip would be answering a different
question than the one asked, but it fails with the refusal, the sentence `THIS IS NOT YOUR CHANGE`,
and a pointer to `notes/interrupts.md`.

And `xtask` asks the probe before it stands anything up, which turned out to matter more than it
sounds. The HVF leg constructs a scanout referee and two network probers before the child, because
each sets an environment variable the runner reads. Against a QEMU that never started, all three
then reported their own failures: *"QEMU's scanout never held the composed screen"*, *"could not
connect to 127.0.0.1:52846"*, *"is the runner's NIFE_MCAST_PORT block attaching the injection
hub?"*. Four confident messages about monitors and forwarded ports, not one of them the reason, on
top of the correct one. That is the same defect this milestone is about, one level up, and it was
only visible by running the thing.

## BUGS

- This machine now has no accelerated coverage at all. Every gate a contributor can run is TCG,
  and until a GICv3 driver exists that is the state. A loud skip is a record of the gap, not a
  substitute for it, and the closing line of `script/gates` says so on every run.
- PROPOSED MILESTONE: a GICv3 driver for aarch64 (number to be minted by the integrator). What
  it is and what it costs are priced in `notes/interrupts.md`'s "What a GICv3 driver would actually
  be". It is worth its own block for two reasons that do not overlap: it is what puts the HVF leg
  back, and `notes/aarch64-board-survey.md` already records GICv3 as the largest single item barring
  most modern aarch64 boards, which is a customer-path constraint rather than a test-harness one.
  Milestone 127 (the sel4 machine) said the same thing from its own side and asked for it to be
  minted deliberately.
- **This is not a fatal risk and does not pretend to be.** It is a gate telling a contributor
  something false, which this tree treats as a defect in the tree rather than in the contributor.
- **The probe answers about the machine, not about the suite.** A QEMU that starts
  `virt,accel=hvf,gic-version=2,iommu=smmuv3` will be believed, and if the leg then fails for its
  own reasons that failure is real and is reported as one. That is the intended split, but it means
  the probe can never turn a genuine red into a skip, and it should not be extended until it can.
- Nothing here checks the other accelerated paths. Whether the same shape exists for KVM on
  cordoba, or for WHPX anywhere, is unexamined. Neither has a leg in `script/gates` today, so
  neither can fail this way yet.
- **The GICv3 boot was measured on a plain `cargo xtask build` kernel, not the test binary**, with
  no disk, NIC or GPU attached. What was proven is that interrupts stop arriving; the suite under
  GICv3 was not run, because a suite whose timer never ticks has nothing further to say.

## Follow-on

- **Milestone 227.** A GICv3 driver for aarch64, proposed in this block's `BUGS` and minted since.
  It is what puts the HVF leg back, and `notes/aarch64-board-survey.md` already records GICv3 as the
  largest single item barring most modern aarch64 boards, which makes it a customer-path constraint
  rather than a test-harness one.
- **Recorded.** `script/ci-build` says on the closing line of every run that the leg was skipped and
  the run was TCG only. Until that driver exists this machine has no accelerated coverage at all,
  and the loud skip is a record of the gap rather than a substitute for it.
- **Recorded.** `design/roadmap/0222-hvf-leg-fails-silently.md` states the split the probe holds: it
  answers about the machine, not about the suite, so a QEMU that starts
  `virt,accel=hvf,gic-version=2,iommu=smmuv3` is believed and a genuine red can never become a skip.
  It should not be extended until it can hold that line.
- **Recorded.** `notes/interrupts.md` carries the GICv3 measurement and the pricing built on it. The
  boot was measured on a plain `cargo xtask build` kernel with no disk, NIC or GPU attached, so what
  is proven is that interrupts stop arriving and not what the suite does under GICv3.
- **Refused.** Extending the probe to the other accelerated paths, KVM on cordoba and WHPX. Neither
  has a leg in `script/gates` today, so neither can fail this way yet, and a probe guarding a leg
  that does not exist is the false-skip shape `script/lint` has deleted three checks for.
  This refusal is milestone 475 (design/roadmap/0475-the-accelerator-probe-on-kvm-and-whpx.md), which
  carries it with the condition that would change it.

## Index row

`script/gates` now skips the HVF leg out loud, on a probe that asks QEMU rather than a version
test; GICv3 measured and refused as its own milestone
