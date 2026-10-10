---
experiment_status: RUN
experiment_run: 2026-10-10
---
# 5. It cannot be made reliable on multicore, and the bugs appear only on silicon

*Risk 5 of [the nine](README.md). The status vocabulary, the rule an entry meets and the running order are there.*

The claim: the concurrency is wrong in ways that QEMU cannot show and that arrive one at a time,
forever.

Run on radon only, and radon has now met §259 (a multicore soak counts toward risk 5 at ten
million crossings over three boots): 10,288,805 crossings over 3 plain boots as of 2026-10-10,
with the two longest runs at 10 h 17 m and 3 h 53 m, no refused wake, wrong reply or stall
([`notes/soak.md`](../../notes/soak.md), "radon, 2026-10-09 to 10"). The same evening sampled the
boot-placement lottery five times in fifteen minutes, because radon now resets itself (milestone
592 (radon's cold reboot dies in OpenSBI's PMIC write), proven deterministic 2026-10-10), and
every draw was clean. The exposure table's every radon row is clean and the defect table's
silicon-only column is empty: the VisionFive 2 wakeup this entry once opened with was retracted
on 2026-08-15, so the gate has never fired on a field failure
([`notes/scheduler.md`](../../notes/scheduler.md)). calef's ruling, 2026-10-10: these are facts,
not a verdict. The verdict waits until a second architecture runs §259, and xenon is the
reachable one; one SoC's implementation of one ISA is not the risk's whole claim.

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
Each is to run unmodified on nife and on Linux, and none has produced a number yet. Corrected
2026-10-10 (UTC): a port no longer waits on a C library, since milestone 835 (#1896) built one from
relibc under §265 (a C library started from relibc). It still waits on whatever that first stage
lacks, such as threads or sockets.

- [Milestone 827 (hackbench on nife and Linux)](../roadmap/0827-hackbench-on-nife-and-linux.md), run long as a second soak.
- [Milestone 830 (schbench on nife and Linux)](../roadmap/0830-schbench-on-nife-and-linux.md), wakeup-latency tails.
