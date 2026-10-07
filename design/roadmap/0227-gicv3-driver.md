---
status: BUILT
raised: 2026-09-02
built: 2026-09-19
---
# 227. A GICv3 driver, because GICv2 boots and silently loses every interrupt

Minted 2026-09-02 by the maintainer, from milestone 222's (the one
command a person runs before pushing has a leg that fails instead of skipping) measurement. Built by
the lane on `milestone/227-gicv3-driver`. *(Number provisional until the merge queue lands it.)*

**In brief.** `kernel/src/drivers/gic.rs` spoke GICv2 only, and milestone 222 measured what happened
under a GICv3: the kernel booted the whole tour, brought four cores online, printed
`timer: 100 Hz tick, interrupts ON`, and then took **zero interrupts and zero preemptions, with
nothing faulting**. `memory::gic_regions()` matched the node by the name prefix `intc@` and took its
first two `reg` blocks as distributor and CPU interface; a GICv3's second block is the redistributor
array, so `gic::init_this_cpu` wrote `GICC_PMR` and `GICC_CTLR` into registers that were not there.

**Now the kernel drives both, chooses by the device tree's binding, and asks the hardware whether
the tree is right before driving anything.** Under `gic-version=3` the boot self-test is 5 of 5
where milestone 317 recorded 3 of 5 with no ticks, the whole aarch64 suite passes under TCG, and the
suite runs on the physical Apple core under HVF for the first time since QEMU 11.1.1 refused the
GICv2 machine.

## Where the driver lives, and what lost

The block asked for this to be decided first. Two files, split along DECISIONS §4 rule 1, with one
adapter that holds both:

| piece | file | why there |
|---|---|---|
| distributor and redistributors (MMIO) | `kernel/src/drivers/gicv3.rs` | memory behind a pointer, handed its addresses: a driver in rule 2's sense, like the GICv2 one beside it |
| CPU interface (`ICC_*` system registers) | `kernel/src/arch/aarch64/gic_cpu_interface.rs` | `msr`/`mrs` are architecture code, and rule 1 says that is the only place they go |
| the version choice and the dispatch | `kernel/src/arch/aarch64/irq.rs` (existing) | already the aarch64 `arch::irq` adapter; now the one place that knows which GIC this is |
| the binding read and the cross-check | `crates/machine_discovery/src/gic.rs` | pure logic, host-tested against QEMU's own trees in milliseconds (DECISIONS §7) |

What else was considered, and why each lost:

- The whole GICv3 driver under `arch/aarch64/`. It keeps one version in one file, which is
  tidy. It loses because the distributor and redistributors are ordinary MMIO, and moving them under
  `arch/` would put a device driver where the architecture boundary is, making `arch/` the place
  drivers go whenever one of their halves is a system register. The GICv2 driver would then be the
  odd one out in `drivers/` for no reason a reader could recover.
- The whole GICv3 driver under `drivers/`, `asm!` included. One file again, and it is what
  Linux does (`drivers/irqchip/irq-gic-v3.c` calls into `arch/arm64/include/asm/arch_gicv3.h` for
  the registers, which is the same split we chose, one level down). Refused outright: rule 1 says an
  `asm!` outside `arch/` is the bug, and a lint-shaped rule with a first exception is no longer a
  rule.
- A trait (`InterruptController`) with two implementations. Refused by this file's standing
  rule against speculative trait-ification: two versions of one controller on one architecture is
  not a second requirement, and the dispatch is a byte and a branch in the one adapter every caller
  already goes through. `enum`-and-`match` is what the tree does in the analogous case
  (`machine_discovery::aarch64::Conduit` for PSCI's `hvc`/`smc`).
- Keeping the ten `drivers::gic::` call sites and branching at each. Refused because it
  multiplies the version question by ten, and rule 1 already asked that portable code name
  `arch::irq` (the scheduler and a test did not). All of them now name `arch::irq`; nothing outside
  `arch/aarch64/irq.rs` names a GIC driver.

*Would we still choose this if both options cost the same?* Yes. The single-file-under-`arch/`
option was also the cheaper one to write, and it lost on where a reader expects to find a driver.

## What was built

1. Discovery by binding (`machine_discovery::gic::discover`). `arm,gic-v3` is a GICv3;
   `arm,gic-400`, `arm,cortex-a15-gic` and `arm,cortex-a7-gic` are a GICv2 (the aarch64 half of
   Linux's `irq-gic.c` table at v6.16). An `intc@` node with any other binding is a refusal that
   names it; a tree stating several redistributor regions is a refusal too. `memory::gic_regions()`
   now returns the version with the addresses, so the addresses cannot be read in the wrong roles.
2. **The cross-check** (`Gic::confirm`): `GICD_PIDR2.ArchRev` must be 3 or 4 for a GICv3 claim, and
   `GICC_IIDR.ArchitectureVersion` 2 to 4 for a GICv2 one, read from the very block the tree calls
   the CPU interface. `arch::irq::init` panics with both sides named before any configuration
   write if they disagree.
3. The GICv3 distributor: off, wait `RWP`, every SPI Group 1 / inactive / disabled / default
   priority, then `ARE_NS | EnableGrp1A | EnableGrp1`, wait `RWP` (Linux `gic_dist_init`). SPIs
   are routed with `GICD_IROUTER` when enabled, to the affinity of the core the existing
   round-robin policy picks, which is when GICv2 wrote `ITARGETSR`.
4. One redistributor per core, found by walking the region and matching `GICR_TYPER[63:32]`
   against this core's `MPIDR_EL1` (Linux `gic_iterate_rdists`), woken (`GICR_WAKER`), private
   interrupts made Group 1 and all sixteen SGIs left permanently enabled (below).
5. The system-register CPU interface: `ICC_SRE_EL1.SRE` set and read back (panics if it will
   not stick), `PMR = 0xff`, `BPR1 = 0`, `CTLR = 0` (EOI mode 0), `AP1R0 = 0`, `isb`,
   `IGRPEN1 = 1`, `isb`. Acknowledge is `ICC_IAR1_EL1` then `dsb sy`; EOI is `ICC_EOIR1_EL1` then
   `isb`; an SGI is `dsb ishst`, `ICC_SGI1R_EL1`, `isb`. Every barrier is Linux v6.16's
   (`irq-gic-v3.c`, `arch_gicv3.h`, `arm-gic-v3.h`), read rather than recalled, and cited at its
   site.
6. `boot.s` opens the interface to EL1 when entered at EL2: `ICC_SRE_EL2.SRE|Enable`, read back,
   `ICH_HCR_EL2 = 0`, guarded by `ID_AA64PFR0_EL1.GIC` (Linux `__init_el2_gicv3`), so argon's GICv2
   never touches registers it does not have.
7. The runner: HVF gets `gic-version=3`, which it requires; TCG keeps 2. `NIFE_GIC` (milestone
   317's reproduction flag) is now a supported choice of 2 or 3 and refuses anything else.
   `script/ci-build`'s skip text no longer says the kernel is GICv2-only.

SGIs stay enabled, and that was found rather than planned. QEMU's GICv2 model forces SGI
set-enables to all ones and ignores SGI clear-enables (`hw/intc/arm_gic.c`), and the GIC-400 does
the same. The Irq capability's mask-on-fire, unmask-on-ACK protocol had been relying on that for its
SGI tests, where the mask and the unmask can run on different cores. A GICv3 redistributor honors an
SGI disable, per core, so a faithful port would have left an SGI masked on one core forever. The
GICv3 driver keeps the GICv2 contract instead, and says why at `gicv3::disable`.

## Proven, per configuration

Measured 2026-09-19 on patagonia (Apple M-series, QEMU 11.1.1). Exit codes are the gate's own.

| configuration | `boot-check` self-test | kernel suite | exit | notes |
|---|---|---|---|---|
| TCG, `gic-version=2` (the default) | 5 of 5 | `script/test` all architectures: pass | 0 | the unchanged path |
| TCG, `gic-version=3` | 5 of 5 (was 3 of 5, zero ticks) | `NIFE_GIC=3 script/test --arch aarch64`: pass | 0 | the whole suite, host crates and image checks included |
| TCG, `gic-version=3`, entered at EL2 | 5 of 5 | not run | 0 | `NIFE_EL2=1`; also 5 of 5 on `gic-version=2` |
| HVF, `gic-version=3`, physical core | not applicable (boot-check is TCG) | `script/test --hvf`: 322 passed, 3 skipped, scanout and inbound checks green, on the final run | 0 | the first HVF runs since milestone 222; one test is flaky here, below |

The HVF leg's two failures, neither of them the GIC:

- The cycle counter's negative half. With `PMUSERENR_EL0` at zero, an EL0 read of `PMCCNTR_EL0`
  was answered rather than refused. The hypervisor decides that outcome, so the assertion now skips
  on an Apple core (reachable only under a hypervisor here), the positive half still runs, and
  `arch::timer::set_cycle_counter_grant`'s BUGS carries the measurement for milestone 74.
- `a_std_program_serves_a_granted_listening_port` hung in two of the three full `--hvf` runs
  that reached it, and passes alone under HVF and in every TCG run. Recorded below as a proposal;
  `script/ci-build` names it when the HVF leg goes red. So `script/ci-build`'s HVF leg now runs,
  and is sometimes red on that one test, where before it was always skipped.

The two tests this milestone adds, both in `arch::irq::tests`, pass on GICv2 and GICv3:
`every_online_core_takes_its_own_timer_ticks` (a GICv3 redistributor is per core, so one core can go
silent while the test's own keeps ticking; injecting "secondaries never enable Group 1" fails it
naming core 1), and `the_driven_gic_is_the_one_the_tree_names`. The host crate adds nine
(`crates/machine_discovery/tests/gic_versions.rs`), against QEMU's own GICv2, GICv3 and HVF trees.

What it cost the GICv2 path, measured against the base commit in a throwaway worktree:

| gate | base | this branch |
|---|---|---|
| `script/fastpath-footprint` (aarch64) | 5356 / 7028 / 1504 bytes | **identical** |
| `script/bench --check` | pass | pass; every row within one tick of base |
| `script/icount` handler instructions (aarch64) | 1056 | 1088, claim 2's bound is 2500 |

The icount delta is the version dispatch on the interrupt path: two relaxed byte loads and two
branches per interrupt, which the unoptimized icount build calls rather than inlines. It measured
+48 before the three per-interrupt calls were marked `inline(always)` and +32 after.

Does `script/bench --real` run under HVF now? Yes, to completion (finding, not a benchmark
result; HVF timings are not recorded as numbers here). **`script/job-mix --hvf`** is milestone 168's
flag and was not on `main` when this lane ran (pull request #969), so it was not run; it had nothing
to run on until now.

## Correction to this block's own BUGS

This block said 227 was **"not on any fatal risk's critical path"**. On 2026-09-19 that stopped being
true: fatal risk 4's decisive experiment is milestone 168's job mix, whose HVF cross-check on real
Apple cores could not run without this, and milestone 74's cycle-counter lane could not test under
HVF either. It is also now the only multi-core aarch64 silicon a contributor can reach without a
board, which is fatal risk 5's territory.

## Scope note (DECISIONS §19)

A GIC is aarch64 hardware; riscv64's analog is the PLIC and x86_64's the local and IO APICs, each
with its own driver and none of them touched. Nothing here is a kernel capability that one
architecture has and another lacks: the portable surface (`arch::irq::enable`, the reschedule IPI,
the Irq capability) is unchanged, and aarch64 now satisfies it on two controller versions instead of
one.

## BUGS

- **The HVF leg is flaky on one test that is not understood** (two hangs in three full runs). See the proposal below.
- No ITS. A `gic-version=3` machine offers one under TCG (`its@8080000`); nothing drives it, so
  there are no LPIs and no MSI translation. Milestone 317 owns the reason it matters.
- **Not proven on GICv3 silicon.** QEMU's TCG model and HVF's are the only GICv3s this has met. The
  EL2 step in `boot.s` in particular cannot be proven on QEMU, whose `ICC_SRE_EL2` reads as set
  whether or not the step runs (measured by skipping it: still 5 of 5); the EL1 `SRE` assertion is
  what would stop a board where it matters.
- **`ID_AA64PFR0_EL1.GIC` is not part of the boot check**, because the first HVF boot overruled it:
  it reads zero on an Apple core under HVF (`0x1101000010110011`) while QEMU emulates the registers.
  `machine_discovery::gic`'s module comment has the measurement.
- The GICv2 SGI has no barrier before `SGIR`, found while writing the GICv3 one. Recorded in
  `drivers/gic.rs`'s BUGS and not changed, so the GICv2 path's counts stay still; argon is where it
  could matter.
- One redistributor region, no extended INTID ranges, no redistributor power-down, each named
  where a reader meets it (`machine_discovery::gic`, `drivers/gicv3.rs`).

## Follow-on

- **Milestone 514.** milestone 514 (the HVF leg's std listener test hangs), `design/roadmap/0514-the-hvf-std-listener-hangs-in-some-full-runs.md`: the HVF
  leg's std listener hang. `user::tests::a_std_program_serves_a_granted_listening_port` hung in two
  of three full `script/test --hvf` runs: the std program aborts (`__rust_abort`, a panic), the kernel test waits
  for a report that never comes, and the watchdog reports a lost-wakeup hang. It passes alone under
  HVF and under TCG. The likeliest reading, not established: `probe_inbound` holds one connection
  until the run ends, and if it opened that connection between the hand-written listener's window
  and the std one, the std listener's bounded `accept` expires (`std-nife`'s `accept` returns
  `WouldBlock`, and `serve_one_inbound` panics on it) while the prober waits on a connection nobody
  will accept. Two defects may be here: the timing, and a kernel test that hangs rather than fails
  when its reporter dies. `notes/hvf-leg.md` carries the evidence, and `script/ci-build` names the
  failure until this lands.
- **Milestone 168.** Its HVF cross-check (`script/job-mix --hvf`, pull request #969), fatal risk 4's
  decisive experiment on real Apple cores, had no machine to run on and has one now.
- **Milestone 274.** Apple Silicon's own core is untested, and its index row says HVF cannot boot
  this kernel, "blocked on 227". It boots now, so that milestone's premise changed.
- **Recorded.** Milestone 74's cycle counter under HVF, in `arch::timer::set_cycle_counter_grant`'s
  BUGS: an HVF run is not evidence about `PMUSERENR_EL0`, because the hypervisor answered a read the
  register should have refused.
- **Recorded.** The GICv2 SGI's missing barrier before `SGIR`, in `drivers/gic.rs`'s BUGS, with the
  one-line fix and why it was not taken here.
- **Recorded.** The ITS, in `drivers/gicv3.rs`'s BUGS; milestone 317 owns why it matters.
- **Recorded.** `notes/aarch64-board-survey.md` says its GIC column is stale: the i.MX8MM's GIC cost
  is now a proving run rather than a driver.

## Index row

the kernel drives GICv2 and GICv3, chosen by the device tree's binding and confirmed against the
hardware at boot; `gic-version=3` goes from zero ticks to the whole suite under TCG, and the suite
runs on the physical core under HVF again
