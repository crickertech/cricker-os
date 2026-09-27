# The PMU, and the two clocks in an aarch64 core

The Performance Monitoring Unit (PMU) is counting hardware built into the CPU core, separate from
the part that runs instructions. Its job is to tally low-level events as the core executes: clock
cycles, instructions retired, cache misses, branch mispredictions, TLB misses, and dozens more. It is
the core keeping score on itself.

Picture a bank of hardware counters wired into the pipeline. Each can be told "count event 0x08"
(instructions retired) or "count event 0x03" (L1 data cache misses), and it ticks every time that
event fires, at hardware speed, with no software in the loop. This is the machinery behind `perf` on
Linux and Instruments on macOS.

## The counter that matters here: the cycle counter

aarch64 gives the PMU one always-present counter, `PMCCNTR_EL0`, that counts CPU clock cycles. On
an Apple M-series core near 4 GHz that is one tick roughly every 0.25 ns. You read it with a single
`mrs` (move-from-system-register), which itself costs only a handful of cycles.

That resolution and that cheapness are why cycle-accurate microbenchmarks reach for it. To time one
operation:

```
t0 = read PMCCNTR
do_the_thing()        // one syscall, one IPC, one page map
t1 = read PMCCNTR
cost = t1 - t0        // cycles, resolvable to nearly a single cycle
```

A whole seL4 IPC is a few hundred cycles, so a single-shot measurement of it *needs* this resolution.
(This line read "~200-400 cycles" until 2026-08-16, which was folklore about the L4 lineage rather
than a figure anyone here had read. seL4's one published aarch64 platform measures 413 for the
call and 426 for the reply, one-way each; notes/benchmarks.md compares against that pair and
notes/aarch64-board-survey.md says which machine it is. The argument is unchanged either way: at
~0.25 ns per cycle, one operation of that size is unresolvable by a 41 ns tick.)
That is exactly how `sel4bench` works (notes/benchmarks.md), and exactly why it could not run on this
Mac.

## Two clocks, and why the difference is the whole story

An aarch64 core has two unrelated counters, and confusing them is a category error:

| | PMU cycle counter (`PMCCNTR_EL0`) | Generic timer (`CNTVCT_EL0` / `CNTPCT_EL0`) |
|---|---|---|
| counts | CPU clock cycles | a fixed reference tick |
| rate | the CPU clock (~4 GHz), and it varies with frequency scaling | fixed, advertised in `CNTFRQ_EL0` (24 MHz here) |
| resolution | ~0.25 ns | ~41 ns |
| what it is for | profiling, microbenchmarks | wall-clock timekeeping |

The generic timer is the OS's clock: a steady reference tick used to tell time and schedule
deadlines (`CNTPCT`, `CNTP_CVAL`; see interrupts.md). It is what our own bench reads, through
`user_mode_runtime::now` at EL0 (abi.md opened `CNTKCTL_EL1.EL0VCTEN` for exactly this). It is coarse, ~41 ns per
tick, so one IPC reads as "1 tick, maybe 2." We beat the coarseness by timing a loop of thousands
of operations and dividing; the per-op cost falls out cleanly and the tick noise averages away.

The PMU cycle counter is the opposite trade: fine enough to time a single operation, but it counts
*cycles*, not time, and the cycle rate moves with clock scaling (DVFS), so turning cycles into
nanoseconds needs the current frequency, which is not fixed.

Two ways to measure a fast operation, then: one shot at high resolution (PMU, sel4bench) or a
long loop at low resolution (generic timer, ours). Both are valid; they fail under different
conditions.

## Why virtualization keeps the PMU out of reach

The generic timer is *architected* state, part of the CPU's published contract, so a hypervisor is
expected to pass it through, and Apple's HVF does: `CNTVCT` reads work fine inside a guest. That is why
our bench runs under QEMU-HVF at all.

The PMU is different. It is microarchitectural, core-private, and awkward to expose safely: it can leak
information across VM boundaries, and it is real work to save and restore across guest switches. So it
is commonly left unvirtualized:

- QEMU-TCG (pure emulation, our deterministic `icount` mode) has no real cycles to count, it just
  translates code, so `PMCCNTR` returns quantized junk (we saw 0 and 1000). Measured again on
  2026-09-19 once this kernel started the counter (below): without `-icount` it moves in steps of
  1000, about 32 per generic-timer tick; under `-icount` it *is* the instruction count, 16 per tick at
  QEMU's 62.5 MHz `CNTFRQ`. Both move, so both pass the kernel's did-it-move check, and neither is a
  cycle.
- Apple HVF does not virtualize the guest PMU, so a guest's `PMCCNTR` reads are unstable.

Either way a single-shot cycle measurement has no usable clock. This is why `sel4bench` (single-shot,
PMU) cannot produce numbers on this Mac while our bench (long loop, generic timer) can, and it is the
same constraint design/roadmap/74-cycle-counters.md flags for any plan that wants real cycle counts: they wait on real
silicon. A Raspberry Pi has a real PMU because it *is* real silicon, not a guest, which is where the
seL4 comparison goes (milestone 24).

The lesson worth keeping: **the coarse, boring generic timer is the one that survives virtualization.**
Choosing it plus long loops, back at milestone 19e, is what makes our cross-OS numbers possible on a
laptop instead of only on hardware. See notes/benchmarks.md for how the two instruments are used, and
notes/abi.md for how EL0 got read access to the generic timer.

## How this kernel starts it, and what it prints (milestone 74's aarch64 half)

Until 2026-09-19 this kernel never started the counter. `PMCR_EL0.E` and `PMCNTENSET_EL0.C` were
never written, so `PMCCNTR_EL0` was a stopped counter that a thread holding milestone 229's grant
could read legally and get the same number from every time. `kernel/src/arch/aarch64/pmu.rs` now does
this on every core, from `timer::init`, gated on `ID_AA64DFR0_EL1.PMUVer` the way the `PMUSERENR_EL0`
write beside it is:

1. `PMCCFILTR_EL0 = 0`, **a provisional value**: EL0 and EL1 counted, EL2 not. What it should be is
   calef's (design/roadmap/proposals/the-aarch64-half-of-74.md), because it decides what every
   published cycle number means.
2. `PMCR_EL0 = E | C | LC`, assigned rather than read-modify-written, so a divide-by-64 bit firmware
   left set cannot survive.
3. `PMCNTENSET_EL0` bit 31, the cycle counter's own enable.
4. Read the counter across 100 generic-timer ticks and refuse it if it did not move.

The boot prints one line after the secondaries are up, in every build:

```
  cycles      : PMCCNTR_EL0 running on 4 of 4 cores (33000 over 1125 ticks at boot), 6 event counters visible, PMCCFILTR_EL0 0x0 PROVISIONAL (EL0+EL1 counted, EL2 not)
```

and the other answers it can give are `enabled but did not advance ...; refused` (the `Stuck`
outcome), `no PMUv3 (ID_AA64DFR0_EL1.PMUVer 0x0); ticks only`, and a second line naming any core that
disagrees with the boot core. `script/bench` prints the probe that converts every tick-denominated row:

```
  probe: cycles_per_tick 16.00 (10000226 cycles over 625003 ticks at cntfrq 62500000)
```

That line is the emulator's instruction count, not a measurement, and the exact 16 is the tell. On
argon the ratio should be near the core clock over 19.2 MHz and **not** a clean integer.

### BUGS

- Nothing here has run on silicon. argon's bench procedure is milestone 127 (sel4)'s.
- `PMCCFILTR_EL0` is provisional, and no aarch64 cycle figure is a result until an architect
  rules.
- The `Stuck` refusal has never fired. Every QEMU `-cpu` this tree boots models PMUv3 and moves
  the counter, and `-cpu cortex-a72,pmu=off` takes the no-PMU path instead. The first machine that
  can exercise it is one whose secure firmware prohibits Non-secure counting.
- **HVF could not be tried.** This QEMU refuses HVF with a GICv2 (`HVF does not support GICv2
  emulation`), and this kernel's GIC driver is GICv2-only, so what HVF does to a started counter is
  still the unmeasured claim in the section above.
