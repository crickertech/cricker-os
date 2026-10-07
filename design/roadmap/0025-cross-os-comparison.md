---
status: PARTIAL
raised: 2026-07-30
milestone_dependencies: 74
decision_dependencies: none
machine_requirements: aarch64 silicon; PMU cycle counter
specific_machine: argon (the Jetson TX1 under seL4's published aarch64 numbers, so the comparison is like for like)
needs_person: yes
---
# 25. Cross-OS performance comparison (extends 21)

Everything but `sel4bench` is done, and `sel4bench` times single
operations through a PMU cycle counter that neither QEMU-TCG nor HVF provides. The hardware is no
longer 16a's board: milestone 127 bought the machine under the only published aarch64 seL4 numbers
(a sealed TX1 kit, 2026-08-15, arriving ~08-19 to -26), so this gate now counts down in shipping
days rather than waiting on a decision. Milestone 74 is the driver that reads that counter, and 74's block states the
dependency this one does not.

In brief. EL0-measured primitive benchmarks (syscall, context switch, IPC, map, spawn) the lmbench way, so the numbers include the trap the kernel-side benchmarks skip; then line them up against lmbench (Linux, macOS guests) and `sel4bench` (seL4), at a matched virtualization tier, with release builds. Fold in the icount codegen-sensitivity fix.

Why it matters. turns perf claims into cross-OS numbers: where does a Rust capability microkernel stand next to Linux, macOS's XNU, and seL4 on the primitives that define an OS. Largely done: four EL0 primitives (null syscall, context switch, IPC, page map) on both instruments, a release build path, and the three-way comparison (nife vs Linux-under-HVF vs native macOS) with nife winning null/IPC ~5x. `spawn` landed too (its real prerequisite was never retype, which had already shipped, but object revocation, reclaiming a child's TCB/aspace/endpoint so a spawn loop can repeat; that shipped as its own milestone, notes/object-revocation.md, and the EL0 `lat_proc` bench, `spawn_el0`, is in the suite and the committed baseline). Remaining: only `sel4bench` (built and booting for qemu-arm-virt, but it times single ops via the PMU cycle counter, which neither QEMU-TCG nor Apple HVF provides, so it is **deferred to real hardware**, the milestone-16 machine, which has a real PMU; this validates our CNTVCT + long-loop design). notes/benchmarks.md

**This milestone's own suite is entirely single-operation primitives, the same shape DECISIONS §96
means by "micro-benchmark."** [Milestone 168](0168-multitasking-benchmark.md) was minted from §96's
own text noting this milestone "has the same hole": a real multi-tasking workload, which is what
would actually reveal a process-kernel-vs-event-kernel difference. Not this milestone's own scope,
recorded here so the gap has one home rather than two.
## Follow-on

- **Outstanding.** `sel4bench` itself, the only piece of this block not done. It builds and boots
  for `qemu-arm-virt` but times single operations through the PMU cycle counter, which neither
  QEMU-TCG nor HVF provides, so it has never produced a number, and nothing in `bench/` or `xtask`
  runs it. Checked 2026-09-03.
- **Done.** The hardware half of this block's gate stopped being a delivery date. The Jetson is in
  hand as of 2026-09-01 and named argon (`design/roadmap/0127-the-sel4-machine.md`,
  `notes/target-hardware.md`). What remains is bench time, not shipping days.
  Corrected 2026-10-06: not done. The seller shipped a TK1 against the TX1 order; it is going back
  and the TX1 has no date (`notes/bench-runbook.md`).
- **Done.** Folding in the icount codegen-sensitivity fix. `--check` is a coarse 10% tripwire
  rather than a 2% gate, stated in `notes/benchmarks.md` and carried in the baseline headers under
  `bench/`.
- **Outstanding.** Milestone 74, the driver that reads `PMCCNTR_EL0` and the SBI PMU, is still
  NOT-STARTED, and it is what a real `sel4bench` comparison needs on our side of the table.
  Checked 2026-09-03 against its block.
- **Done.** The authority question underneath 74's aarch64 half is answered and built.
  `design/decisions/0139-cycle-counter-authority.md` is DECIDED (calef, 2026-09-02) and milestone
  229 shipped the per-thread grant the same day: `kernel/src/arch/aarch64/timer.rs` opens and
  closes `PMUSERENR_EL0.CR` on the context switch.
- **Milestone 168.** The multi-tasking workload this block's own suite cannot supply, minted from
  DECISIONS §96's text so the gap has one home.

## Index row

turns perf claims into cross-OS numbers
