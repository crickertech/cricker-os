---
experiment_status: RUN
experiment_run: 2026-09-25
---
# 5. It cannot be made reliable on multicore, and the bugs appear only on silicon

*Risk 5 of [the nine](README.md). The status vocabulary, the rule an entry meets and the running order are there.*

The claim: the concurrency is wrong in ways that QEMU cannot show and that arrive one at a time,
forever.

Run on radon only. radon soaked 8 h 09 m clean: 4.1 million
cross-core handoffs with no refused wake, wrong reply or stall
([`notes/visionfive2.md`](../../notes/visionfive2.md)). argon and xenon have not run it. The VisionFive
2 wakeup this entry once opened with was retracted on 2026-08-15, so the gate has never fired on a
field failure ([`notes/scheduler.md`](../../notes/scheduler.md)). Milestone 201 (is multicore
reliability converging)'s curve now holds radon's four soak boots: about 12 hours and 4.1 million
crossings, zero defects ([`notes/multicore-defect-curve.md`](../../notes/multicore-defect-curve.md)).
One draw of the placement lottery is a confidence, not a verdict.

Every multicore defect this project has found whose instrument is recorded was found without
silicon: one by loom, one by an audit, and the rest under QEMU, most at two cores. The only one seen
on physical cores and not under TCG is an HVF test hang that is still unclassified. A defect
emulation can find is not evidence about the class it cannot, but it retires the reading that
silicon is the only productive instrument.

**The decisive experiment:** milestone 225 (run the soak on radon, argon and xenon), run on radon;
argon and xenon remain.

Two caveats, argued in the [appendix](multicore-reliability.md): every load-sensitive
red so far has been a test bug, which fits a healthy kernel and a blind instrument equally well, and
no result here can be green, since a flattening curve is only a confidence.

2026-10-08 (UTC): threads are to be built (§263 (threads are built), milestone 812
(`std::thread::spawn` runs real threads in one address space)), which `schbench` and `hackbench`'s
thread mode need to run here at all.

## Benchmarks that inform this risk

As of 2026-10-08 (UTC), under §262 (nife is measured with the field's standard benchmarks).
Each is to run unmodified on nife and on Linux, none has produced a number yet, and every nife
port waits on a C library, which §262 leaves to calef.

- [Milestone 827 (hackbench on nife and Linux)](../roadmap/0827-hackbench-on-nife-and-linux.md), run long as a second soak.
- [Milestone 830 (schbench on nife and Linux)](../roadmap/0830-schbench-on-nife-and-linux.md), wakeup-latency tails.
