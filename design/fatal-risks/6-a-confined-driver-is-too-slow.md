---
experiment_status: RUN
experiment_run: 2026-10-04
---
# 6. A capability-confined userspace driver cannot drive real hardware at real speed

*Risk 6 of [the nine](README.md). The status vocabulary, the rule an entry meets and the running order are there.*

The claim: the thing that makes the thesis interesting, drivers outside the kernel behind an IOMMU,
does not survive contact with a real device.

AMBER (calef, 2026-10-04). The first silicon evidence was the
TRNG, 2026-09-16: its three parts were measured on silicon, and they were never one claim. On radon, milestone 159 (a real hardware entropy source: the JH7110's TRNG)'s
driver is an EL0 process reaching the TRNG through a capability that names no device. Confined,
2026-09-03. Driving real hardware, 2026-09-04, reproducibly. At real speed, MEASURED 2026-09-16 at
about 8.4 us per round trip. The committed boots read 973,384 to 992,248 bytes/s (four boots; the
original 955,223 bytes/s has no committed transcript, corrected 2026-10-03 per §216 from #1495).

**The decisive experiment: RUN, 2026-10-04, on xenon.** An EL0 process holding one page of the Micron 2450's BAR0 and a DMA window confined by VT-d drove the real NVMe in three boots, all `CONFINED-AT-RATE`: both preflights PASS (the catch-all unit `0xfed91000` owns 01:00.0; 512-byte LBAs), and 16384 of 16384 blocks verified each time. Medians: write 458142471 B/s (range 457744197 to 474990381), read 271854622 B/s (237098519 to 281608311, a 16% spread), and an IPC floor of 1197 ns per round trip, which is 13% of a write block and 8% of a read block. Getting there found and fixed a kernel defect: xenon's VT-d units do not snoop the CPU caches (`ECAP.C` = 0), and the kernel never wrote its tables back, which QEMU cannot show (`bench/xenon-2026-10-04/`, milestone 261 (the NVMe driver leaves the kernel)).

**Verdict: AMBER, leaning to GREEN (calef, 2026-10-04).** Real hardware (a real NVMe with DMA), driven correctly (16384 of 16384 verified, three of three boots), from EL0 behind a translating VT-d unit, at 458 MB/s write and 272 MB/s read (medians, queue depth 1). Two things stand between this and GREEN, each written down before it is run:

1. "Real speed" has no reference. A Linux `fio` run at queue depth 1 on the same disk and window must come out with nife within about 0.8x of Linux for both read and write.
2. "Confined" is asserted, not attacked. A replayable test where an EL0 server aims a PRP outside its DMA region must go red with the IOMMU skipped and be refused with it on. Milestone 261 lists this as Outstanding.

Not GREEN, because a risk about speed would be scored without a reference speed. Not RED, because nothing on the night came close to falsifying the claim.

Caveats. Every figure is one command in flight, polled completion, and one pass per boot with no warm-up, so it is a lower bound on the device and not comparable to `fio` at queue depth 32. No Linux `fio` run on the same disk at queue depth 1 has been made, so "at real speed" is still unclaimed: the figures say "confined, on silicon, at this rate," and the Linux ratio is what would turn that into "real speed." Read varied 16% across boots. [Appendix](the-confined-driver.md).
