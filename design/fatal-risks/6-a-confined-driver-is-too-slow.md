---
experiment_status: RUN
experiment_run: 2026-10-04
---
# 6. A capability-confined userspace driver cannot drive real hardware at real speed

*Risk 6 of [the nine](README.md). The status vocabulary, the rule an entry meets and the running order are there.*

The claim: the thing that makes the thesis interesting, drivers outside the kernel behind an IOMMU,
does not survive contact with a real device.

GREEN (calef, 2026-10-07); amber 2026-10-04. The first silicon evidence was the
TRNG, 2026-09-16: its three parts were measured on silicon, and they were never one claim. On radon, milestone 159 (a real hardware entropy source: the JH7110's TRNG)'s
driver is an EL0 process reaching the TRNG through a capability that names no device. Confined,
2026-09-03. Driving real hardware, 2026-09-04, reproducibly. At real speed, MEASURED 2026-09-16 at
about 8.4 us per round trip. The committed boots read 973,384 to 992,248 bytes/s (four boots; the
original 955,223 bytes/s has no committed transcript, corrected 2026-10-03 per §216 from #1495).

**The decisive experiment: RUN, 2026-10-04, on xenon.** An EL0 process holding one page of the Micron 2450's BAR0 and a DMA window confined by VT-d drove the real NVMe in three boots, all `CONFINED-AT-RATE`: both preflights PASS (the catch-all unit `0xfed91000` owns 01:00.0; 512-byte LBAs), and 16384 of 16384 blocks verified each time. Medians: write 458142471 B/s (range 457744197 to 474990381), read 271854622 B/s (237098519 to 281608311, a 16% spread), and an IPC floor of 1197 ns per round trip, which is 13% of a write block and 8% of a read block. Getting there found and fixed a kernel defect: xenon's VT-d units do not snoop the CPU caches (`ECAP.C` = 0), and the kernel never wrote its tables back, which QEMU cannot show (`bench/xenon-2026-10-04/`, milestone 261 (the NVMe driver leaves the kernel)).

**Verdict: GREEN (calef, 2026-10-07); amber 2026-10-04.** Real hardware (a real NVMe with DMA), driven correctly (16384 of 16384 verified, three of three boots), from EL0 behind a translating VT-d unit. The two conditions written on 2026-10-04 are met:

1. **Real speed has a reference.** Linux `fio` on the same disk and window, 4 KiB, queue depth 1 (`bench/xenon-2026-10-04/linux-fio.log`, hand-transcribed from photographs). Writes: nife 458 MB/s against Linux's best, 425 MB/s (`io_uring`, polled), 1.08x. Reads: nife 272 MB/s against Linux's 126 MB/s, 2.16x. The condition was within about 0.8x of Linux for both.
2. **Confined is attacked.** The out-of-region DMA test, `a_confined_el0_server_cannot_dma_outside_its_region`, was done 2026-10-04 (milestone 261 (the NVMe driver leaves the kernel)) and its falsification replayed red 2026-10-05.

**The caveat, plainly.** A read 2.2x faster than Linux on the same disk is suspicious and unexplained. Polling does not explain it, since polled Linux reads were no faster. But nife's read would have to be overstated by more than 2.7x to fall below 0.8x. The rest: one pass per boot, figures hand-transcribed from photographs, read varied 16% across boots, and "real speed" is claimed at queue depth 1 only. The follow-up is the proposed milestone `design/roadmap/proposals/the-same-storage-benchmark-on-nife-and-linux.md`: one job file on both systems, queue depths 1, 4 and 32, with a per-I/O latency histogram on each.
Caveats. Every figure is one command in flight, polled completion, and one pass per boot with no warm-up, so it is a lower bound on the device and not comparable to `fio` at queue depth 32. A Linux `fio` run on the same disk at queue depth 1 has been made (corrected 2026-10-07 per §216 (fatal-risk facts are correctable, and verdicts are the architect's), from `bench/xenon-2026-10-04/linux-fio.log`, which was hand-transcribed from photographs): Fedora 44 live USB, fio-3.40, the Micron 2450 NVMe 256GB (`nvme0n1`), 4 KiB blocks, queue depth 1, `--direct=1`, offset 1 MiB, 64 MiB, one pass, with the same window nife used. The main run (psync) read 30.7k IOPS at 126 MB/s and wrote 77.6k IOPS at 318 MB/s; two further runs with polling requested (pvsync2 did not engage polling; io_uring did) read 126 MB/s and wrote 272 MB/s and 425 MB/s. The comparison of these against the 0.8x condition above is owed to calef; this correction states no ratio and changes no verdict. "Real speed" stays unclaimed until that comparison is made. Read varied 16% across boots. [Appendix](the-confined-driver.md).
