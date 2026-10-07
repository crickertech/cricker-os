---
status: PARTIAL
raised: 2026-08-03
milestone_dependencies: 75
decision_dependencies: none
machine_requirements: aarch64 and riscv64 silicon; PMU cycle counter
specific_machine: none
needs_person: yes
---
# 74. Cycle counters: SBI PMU on RISC-V, `PMCCNTR_EL0` on aarch64

Raised 2026-08-03, from an audit of what milestone 16a actually needs. Its
deliverable includes "the benches on real cycles via the SBI PMU extension", and until 2026-09-03
nothing in the tree implemented it: `PMU` appeared only in device-tree test fixtures and in this
file. Both ISA halves are now built (riscv64 2026-09-03, aarch64 2026-09-19), and it stays
`PARTIAL` for two reasons that are not code: argon has not run the aarch64 half, and what aarch64's
counter counts (`PMCCFILTR_EL0`) and what the user-mode read is called are an architect's
(`design/roadmap/353-the-aarch64-half-of-74.md`).

Both halves of this line are under correction, and neither says
what it used to.

Milestone 75's index row reads `NOT-STARTED` and that is false: its mechanism is built.
`design/decisions/0139-cycle-counter-authority.md` was DECIDED by calef on 2026-09-02,
`kernel/src/sched.rs`'s `install_cycle_counter_grant` (near line 1734) applies the per-thread grant
at every context switch behind the `cycle_counter_grant` feature, and
`kernel::user::tests::a_granted_thread_reads_the_cycle_counter_and_an_ungranted_one_faults` passes
on all three architectures, negative case included. Whether 75's row is flipped, and what that
unblocks, is an architect's rather than a lane's; the mechanism cited here is what a reader can
check today. **The aarch64 half of 74 was out of the riscv64 lane's scope, not blocked by an
unanswered question.** Until 2026-09-19 `PMCR_EL0.E` and `PMCNTENSET_EL0.C` were never written by
this kernel, so `PMCCNTR_EL0` was a stopped counter that a granted thread read the same value from
every time. That is built now; see "What the aarch64 half built" below.

The `HARDWARE` half is now the second sense of that gate rather than the first: the riscv64 code
is written and gated, and what remains is a person at radon following
notes/riscv-cycle-counters.md's procedure, because QEMU-TCG models an instruction counter that has
nothing to do with cycles.

The riscv64 half is built (2026-09-03): the SBI PMU extension is probed as an optional fifth
row of `SBI_TABLE`, `kernel/src/arch/riscv64/pmu.rs` asks firmware to find and start a counter for
`SBI_PMU_HW_CPU_CYCLES`, checks it is actually counting, remembers which CSR reads it and records
why when there is none, the boot prints all of it, and `cargo xtask bench --riscv` prints one
`cycles_per_tick` probe. The aarch64 half is built too (2026-09-19), in the same shape; what it
leaves open is two decisions, `design/roadmap/353-the-aarch64-half-of-74.md`.

## What we read today, and why it is not cycles

Both ISAs read a fixed-rate reference counter, not a cycle counter:

| | aarch64 | riscv64 |
|---|---|---|
| today | `CNTVCT_EL0` + `CNTFRQ_EL0` | the `time` CSR (`rdtime`) |
| counts | a fixed tick, 62.5 MHz under QEMU | a fixed tick |
| resolution | ~41 ns on real silicon | comparable |
| the cycle counter we lack | `PMCCNTR_EL0` | SBI PMU, or the `cycle` CSR when `mcounteren` permits |

notes/pmu.md already sets this out for aarch64 and calls confusing the two a category error. The
generic timer is the OS's clock; the PMU counts CPU cycles at ~0.25 ns resolution and its rate moves
with frequency scaling. Two ways to measure a fast operation: one shot at high resolution (PMU,
which is what sel4bench does) or a long loop at low resolution (the generic timer, which is what we
do). Both are valid and they fail under different conditions.

## Why it matters more than "another counter"

The thesis claim is a cross-OS comparison, and the literature it is compared against is denominated
in cycles, not nanoseconds. notes/benchmarks.md does the conversion by hand and draws the honest
conclusion:

> seL4 publishes, for the same-core different-address-space path, 413 cycles for the IPC call and
> 426 for the IPC reply, one-way each ... So the corrected figure is roughly 1.1x to 1.7x an
> L4-lineage round trip, not 4 to 7 times.
>
> -- notes/benchmarks.md

This file has now quoted that paragraph wrongly twice, which is why the quote above carries an
attribution line a gate can check. The first version quoted the retracted arithmetic (*"At ~3.2
GHz, 705 ns is ~2,200 cycles round trip... we are 4 to 7 times heavier"*) as the current record,
after milestone 101 had re-measured it and found three errors. The replacement written on 2026-08-04
was a paraphrase presented as a quotation: it read "At ~3.2 GHz, 350 ns is roughly 960 to 1,420
cycles round trip", and those words appear in no note.

It also put back the one assumption the correction had removed. The note says in as many words
that the old paragraph's 3.2 GHz "is not this machine"; the 960-to-1,420 range is 350 ns against the
M3's two clocks, 2.75 GHz on an E-core and 4.05 GHz on a P-core, and nothing pins the vCPU
thread to either. At 3.2 GHz, 350 ns is ~1,120 cycles, a single number rather than a range. So the
sentence attached a correct range to the clock that had just been rejected, and read as sober while
doing it.

The mechanism rather than the apology: a prose block quote of another document is a citation no gate
resolves, and `script/citations` (milestone 97) checks one only when it carries a `-- path`
attribution line. Neither wrong version had one, so both passed every gate in the tree, twice, in
the file notes/citations.md already uses as its worked example. The binding form costs one line and
is rung two of CLAUDE.md's ladder instead of rung four.

The figure above is still arithmetic performed on a nanosecond measurement using an assumed clock
rate, which is the whole point of this milestone. Measuring cycles directly turns the project's
most-cited number from a derived figure into a read one, and it is the number a reader from the L4
world will look for first.

## Two things block on it

- 16a cannot deliver "benches on real cycles" without it.
- Milestone 25's `sel4bench` is built and booting but was deferred to real hardware precisely
  because it times single operations through `PMCCNTR_EL0`, which neither QEMU-TCG nor Apple HVF
  provides. notes/pmu.md's last section explains why virtualization keeps the PMU out of reach: the
  generic timer is architected state a hypervisor must present, and the PMU is not.

## Parity: the capability half is two ISAs, the measurement half is three

**This heading read "Parity makes this two ISAs, not one" until 2026-09-16, and that was worse than
wrong: it asserted completeness over a gap.** It was true when written on 2026-08-03, when x86_64 was
declared and not started. x86_64 now boots, enumerates PCI, runs `std` and runs unmodified `ripgrep`,
so a reader met a parity claim covering two of three architectures with nothing saying which one was
missing or why. §19's wording is that a capability ships everywhere or a scope note records the gap
and the plan; this block had the gap and no note. It has one now, below.

The two halves of this milestone have different parity answers, and conflating them is what hid
the gap.

**The capability half is legitimately two ISAs, by a recorded exception rather than an omission.**
[DECISIONS §139 part 3](../decisions/0139-cycle-counter-authority.md) rules that x86_64 keeps its
ambient counter: `CR4.TSD` is clear at reset, this kernel never writes it, so ring 3 may execute
`rdtsc` and **an ungranted read is not an error**. There is therefore no x86_64 counterpart to
"grant the counter, fault without it", and
`kernel::user::tests::a_granted_thread_reads_the_cycle_counter_and_an_ungranted_one_faults` skips
its negative half there, saying so in the comment beside the `cfg`. That is §19 working: an
exception with a reason, where a reader meets it.

The measurement half is three ISAs and is three (it was one until 2026-09-17 and two until
2026-09-19).
`bench::cycles_per_tick`, the probe that converts every tick-denominated board row to cycles, was
`#[cfg(target_arch = "riscv64")]`; milestone 309 added the x86_64 arm. The ratio is the thing
cross-machine comparison needs, which is this milestone's whole purpose and milestone 25's and
§96's, so having it on fewer than three architectures is the parity gap that matters here rather
than a cosmetic one.

And the paragraph above used to name the wrong mechanism for x86_64, which is worth keeping
rather than silently correcting, because it is the reasoning most likely to be re-derived. It said
"x86_64 already reads `rdtsc` in `arch/x86_64/timer.rs` for its own calibration", implying the probe
could be built on that read. It cannot: `arch::x86_64::timer::now()` is `rdtsc`, so such a probe
divides one counter by itself and prints an exact `1.00` on every part. Milestone 309 built it on
`IA32_PERF_FIXED_CTR1` (unhalted core cycles) instead, and its block has the argument.

Scope note, per §19. The measurement half is built for riscv64 and aarch64 (this milestone)
and x86_64 (milestone 309, `design/roadmap/309-x86-64-core-cycles.md`), so `bench::cycles_per_tick`
now has no `cfg` at all. **The three are not yet the same quantity**, and the gap is a decision
rather than code: riscv64 counts every mode including M-mode firmware, x86_64 counts ring 0 and 3,
and aarch64 counts EL0 and EL1 under a **provisional** `PMCCFILTR_EL0` until calef rules
(`design/roadmap/353-the-aarch64-half-of-74.md`, decision A). The aarch64 meaning line says
`PROVISIONAL` so a number cannot travel without it. This scope note read "not built for aarch64"
until 2026-09-19.

### The capability half, per ISA

§19 is a gate, and it bites here in an unobvious direction. The milestone reads as RISC-V work because
16a is the RISC-V board, but `PMCCNTR_EL0` is equally unimplemented, so a RISC-V-only cycle
counter would create a parity gap in the one subsystem whose entire purpose is cross-machine
comparison. Both sides are small and they are not symmetrical in shape:

- aarch64: enable the counter (`PMCR_EL0`), open it to EL0 (`PMUSERENR_EL0`), read `PMCCNTR_EL0`.
  Register writes, no firmware call. Whether that EL0 opening happens at all was milestone 75's
  decision, and it was answered by DECISIONS §139 (who may read the cycle counter, and by what
  authority): a per-thread grant, which milestone 229 built. The counter is ~160x finer than the one §10 already excepted, so it does not
  inherit that exception, and it does not: starting the counter opens nothing to EL0 by itself.
  Built 2026-09-19, below.
- riscv64: the SBI PMU extension (EID `0x504D55`), which discovers counters, configures an event,
  and starts and stops it. The tree already makes SBI calls (`SBI_HSM_EID`, `SBI_IPI_EID`,
  `SBI_RFENCE_EID`, SBI TIME), so the plumbing exists and this is a fifth extension rather than new
  machinery. Built 2026-09-03, and the one thing it forced was not the `ecall`: `SBI_REQUIRED`
  was every row of `SBI_TABLE` by construction, because every extension the kernel had ever called
  was one it could not boot without. PMU is the first that is an instrument, and a kernel that
  refused to boot without an instrument would have confused the measurement with the thing measured.
  So `SbiRow` grew the `required` field `Row` already carried one source over, and the existing
  accumulation test failed on the first run, which is what it was for.

## What can be done before the board, and what cannot

Buildable now: both drivers, the `Isa`-style capability probe, the benchmark harness change, and
the aarch64 path end to end (Apple Silicon has a real PMU; whether macOS lets a guest reach it is the
open question notes/pmu.md raises).

**Not verifiable until silicon:** the RISC-V numbers. QEMU-TCG models an instruction counter that has
nothing to do with cycles, so a green test under emulation proves the plumbing and says nothing about
the measurement. Say so in the note rather than publishing an emulated cycle count.

And the emulator was more misleading than that prediction allowed for, which is the one finding
worth carrying out of the riscv64 half. Under `-icount` the `cycle` CSR and the `time` CSR are driven
off the same virtual clock, so the probe reads `cycles_per_tick 100.00`, exactly the ratio
between the two declared rates, with a rounding wobble and nothing else. That is not a number that
looks wrong. It looks like a clean measurement of a 1 GHz core, and a reader who did not know what
TCG does to these two registers would have every reason to write it down. The defense is that the
probe line prints its inputs (`10000029 cycles over 100000 ticks at cntfrq 10000000`) rather than
only the ratio, so the arithmetic is visible, and that notes/riscv-cycle-counters.md's outcome table
names an exact round ratio as the tell rather than as the answer.

## Scope note

**Do not turn this into a profiling framework.** One counter, read before and after, on two ISAs. The
PMU can count dozens of events and the temptation to expose them generically should wait for a second
consumer, which is CLAUDE.md's rule against speculative trait-ification. `sel4bench` comparability is
the requirement; anything beyond it is scope.

## What the riscv64 half built, and what it deliberately did not

Built 2026-09-03, lane `milestone/74-cycle-counters-riscv`.

- `SBI_PMU` / `EID_PMU` as an optional row of `machine_discovery::riscv64::SBI_TABLE`, with
  `SBI_REQUIRED` narrowed to the rows that ask for it. Probed by the existing `probe_sbi` loop with
  no new code, and printed on the existing `firmware    :` boot line for the same reason.
- `CounterInfo`, the host-tested decode of `sbi_pmu_counter_get_info`'s packed word, including the
  `+ 1` on a width field the specification writes as one less than the width, and `None` for both
  CSR and width on a firmware counter (the specification says they "should be ignored", so they are
  not returned rather than returned as numbers a caller could use by accident).
- `kernel/src/arch/riscv64/pmu.rs`: the four calls, the CSR-number dispatch, and one boot line.
  Four kernel tests, each of which asserts plumbing and says in its own doc comment that it is not
  asserting a measurement.
- One `bench-probe: cycles_per_tick` line from `cargo xtask bench --riscv`. **A probe, not a row**:
  it is a rate rather than a duration, so it never enters `bench/baseline-riscv64.txt` and `--check`
  never polices it, which is the existing convention `map_new`'s shootdown probe established. One
  measured ratio converts every existing tick-denominated row at once, which is why it is one line
  at the top rather than a second number on every line.
- notes/riscv-cycle-counters.md, the bench procedure, written in notes/x86-uefi-boot.md's shape:
  in order, with real commands, and a table mapping each observable line to what it means. Step one
  is whether radon's OpenSBI implements the extension at all, because that is a fact about somebody
  else's firmware and every later step is conditional on it.

**Deliberately not built**, and each is the scope note above rather than an oversight: no second
event, no per-hart counter record (SBI PMU counters are per-hart and this configures the boot
hart's; the one consumer is a single-hart probe), no user-facing read (the U-mode `rdcycle` path is
milestone 229's grant and 237's measurement build, which already exist), and no firmware-counter
support (reading one costs an `ecall` per read, and an `ecall` inside a cycle measurement measures
the `ecall`).

## The riscv64 number, measured on radon 2026-09-16

`bench-probe: cycles_per_tick 250.00 (25000940 cycles over 100002 ticks at cntfrq 4000000)`,
transcript `bench/radon-2026-09-16/bench-134300.log`. That is this milestone's `HARDWARE` half in
its second sense: a person at the board, following `notes/riscv-cycle-counters.md`. The counter is
the one the riscv64 half built, and the boot reported it before the sweep:
`cycles : SBI PMU counter 0, CSR 0xc00, 64 bits`.

A round number is exactly what milestone 16a warned would be an artifact, so the reason it is real
is recorded rather than assumed. 16a's block says QEMU-TCG yields "an implausibly exact 100.00"
because one virtual clock drives both the `cycle` and `time` CSRs, and an exact ratio is the tell.
This one is 250 for a physical reason: `cntfrq` is 4 MHz, the core is running at 1.0 GHz, and both
derive from the same PLL, so the ratio is an integer by construction of the silicon rather than of
the emulator. **The evidence that two independent counters are being read is that the measurement is
not exact**: 25,000,940 over 100,002 ticks is 250.0044, and a single clock feeding both would give
250.0000 every time. The 0.0044 is the drift an emulator cannot produce.

What it converts. Every tick-denominated row of a board bench becomes cycles and wall time at
250 cycles per tick. From the same boot, single-hart (`--bench` parks the secondaries, so these are
uncontended costs):

| row | cycles | time |
|---|---|---|
| `null_syscall` | 322 | 0.32 us |
| `yield_switch` | 571 | 0.57 us |
| `ipc_rtt` | 1,046 | 1.05 us |
| `call_reply` | 1,256 | 1.26 us |
| `map_el0` | 1,477 | 1.48 us |
| `ctx_switch` | 1,980 | 1.98 us |
| `relay_rtt` | 2,306 | 2.31 us |
| `broker_rtt` | 2,512 | 2.51 us |
| `ipc_rtt_el0` | 6,222 | 6.22 us |
| `spawn_reap` | 17,086 | 17.09 us |
| `spawn_el0` | 64,820 | 64.82 us |

**These are not comparable to `bench/baseline-riscv64.txt`**, and the reason is the unit rather than
the machine: that file records QEMU icount, which counts guest instructions, while these are
real cycles on silicon. The icount baseline remains what the tripwire checks; this table is what the
hardware costs. Putting them in one table would be the apples-to-apples failure
`notes/benchmarks.md` exists to prevent.

The aarch64 half was untouched by this when it was measured; it was built three days later,
below, and has not met silicon.

## What the aarch64 half built (2026-09-19)

Lane `milestone/74-cycle-counters-aarch64`. The brief was the proposal's first version, and the
shape is the riscv64 half's: start one counter, check it moves, refuse it and say why when it does
not, print it, and add the probe line. Nothing here is on the context-switch path.

- `kernel/src/arch/aarch64/pmu.rs` (module name provisional, matching its two siblings). Per
  core, from `timer::init`, gated on `ID_AA64DFR0_EL1.PMUVer` exactly as milestone 228's
  `PMUSERENR_EL0` write is: `PMCCFILTR_EL0` = a provisional `0`, `PMCR_EL0` = `E | C | LC`
  (assigned, so a firmware-set divide-by-64 cannot survive), `PMCNTENSET_EL0` bit 31, an `isb`, then
  a 100-tick read across the generic timer. The outcome is per core (`NoPmuV3`, `Stuck`, `Running`),
  which the other two halves are not; the brief asked for every core started, and checking each one
  costs microseconds. It also records what firmware left in `PMCCFILTR_EL0` before overwriting
  it, because that is what seL4's published TX1 figures counted (the proposal, decision A, question 3).
- One boot line, every build, printed after the secondaries are up:
  `cycles : PMCCNTR_EL0 running on 4 of 4 cores (...), 6 event counters visible, PMCCFILTR_EL0 0x0
  PROVISIONAL (EL0+EL1 counted, EL2 not)`, then `firmware left PMCCFILTR_EL0 0x0 ...`, then a line
  naming any core that disagrees with the boot core.
- `bench::cycles_per_tick` on aarch64, with its own meaning line carrying `PROVISIONAL`, a
  `CNTFRQ`/100 window, and a new check that the window did not migrate cores (aarch64's counters
  are per core and zeroed at each core's init, so a migrated window would difference two unrelated
  numbers). `bench/baseline-aarch64.txt` is unchanged.
- **The tests stopped carrying values they did not check.** `a_granted_thread_reads_the_cycle_
  counter_and_an_ungranted_one_faults` now asserts the EL0 reads moved forward wherever the kernel
  reports the counter running on every online core, and always on x86_64 (the TSC); riscv64 is not
  asserted, and the test says why. `arch::aarch64::pmu` has four tests of its own, one of which
  reads the three registers back so a silently ignored enable or a surviving `D` bit fails.
- A defect found on the way, in milestone 127's EL2 drop, fixed. `boot.s` wrote `MDCR_EL2 = 0`,
  and `HPMN` = 0 is a reserved value without FEAT_HPMN0, which the Cortex-A57 lacks. The new boot
  line showed it: `0 event counters visible` under `NIFE_EL2=1`, 6 without. `boot.s` now writes
  `HPMN` = `PMCR_EL0.N` read at EL2, which is Linux's `init_el2`, and prints 6 both ways. Only event
  counters were affected, and nothing uses one yet.

### Which path each configuration took (all 2026-09-19, QEMU 11.1.1)

| configuration | outcome | what it proves |
|---|---|---|
| `script/test`, TCG `cortex-a72` | `Running` on 4 of 4, about 32 per tick, in steps of 1000 | the enable took and the plumbing is right; not a cycle |
| `NIFE_EL2=1`, entered at EL2 | `Running` on 4 of 4, 6 event counters (0 before the `HPMN` fix) | the EL2 drop leaves the PMU usable at EL1 on every core |
| `--cpu cortex-a57` (argon's core) | `Running` on 4 of 4 | QEMU's A57 model takes the same path |
| `--cpu cortex-a53` | `Running` on 4 of 4 | likewise |
| `--cpu cortex-a72,pmu=off` | `NoPmuV3` on every core; the grant tests skip | the absent path: nothing is written, nothing faults |
| `--cpu max` | never reached: the kernel refuses the boot on `TGran4` | unrelated pre-existing defect, recorded in `crates/machine_discovery/src/aarch64.rs`'s BUGS |
| `script/bench`, TCG `-icount` | `cycles_per_tick 16.00 (10000226 cycles over 625003 ticks at cntfrq 62500000)` | the instruction count: exactly `script/icount`'s `instructions_per_counter_tick 16`. The round ratio is the tell |
| HVF | not runnable | this QEMU refuses HVF with a GICv2, and the kernel's GIC driver is GICv2-only |

The refusal path (`Stuck`) ran on no configuration, and that is the honest reading of the brief's
expectation that QEMU would mostly exercise it: every QEMU model this tree boots drives the counter
once it is enabled (the "0 and 1000" notes/pmu.md recorded was the counter before anyone started it,
or seL4's reading of it). `Stuck` is exercised by nothing but the code review; the first machine that
can reach it is one whose secure firmware prohibits Non-secure counting.

### BUGS

- No aarch64 cycle figure is a result. `PMCCFILTR_EL0` is provisional, and every number above is
  an emulator's.
- **`Stuck` has never fired**, above.
- The first PMU access on a board is still milestone 228's `PMUSERENR_EL0` write in `timer::init`,
  before the banner. If TF-A leaves `MDCR_EL3.TPM` set it traps there, not here; 127's procedure
  now says what that looks like.
- Per-core counters are unrelated numbers. `C` zeroes each at its own init; `arch::pmu::cycles`
  reads the current core's, and only a same-core difference means anything.
- The user-mode read is still the fixture's, deliberately (decision B).

### argon's first evening, for this half

After 127's steps 1 to 5, read these lines, in this order:

1. `cycles : PMCCNTR_EL0 running on 4 of 4 cores (N over T ticks at boot)`. `N/T` should be near
   1.9 GHz over 19.2 MHz, about 99, and **not** a round integer. A round number means something is
   feeding both counters from one clock.
2. `firmware left PMCCFILTR_EL0 0x...`. Write this down whatever it says: it is the best
   evidence of what seL4's 413 and 426 counted. `0x0` means kernel included; bit 31 set means EL1
   excluded. This line is now the ruling itself: calef decided on 2026-09-19 that the filter
   is set to match what argon's firmware left, rather than chosen in advance
   (design/roadmap/353-the-aarch64-half-of-74.md, decision A). So this reading closes milestone
   353's decision A, and until it is taken no aarch64 cycle figure is published.
3. `6 event counters visible`. The A57 implements six; a different number means `HPMN` or firmware.
4. If the line says `refused` or `disagrees`, the secure world is prohibiting counting on at least
   one core; record which.
5. Then `script/bench` on the board: `cycles_per_tick` with its inputs. Do not publish it until
   decision A is made.

## Follow-on

- **Milestone 353.** The aarch64 half, rewritten 2026-09-19 now that the counter runs: decision A
  (what `PMCCFILTR_EL0` counts) and decision B (the user-mode read's name and promise), each with
  options and no winner. It was an unnumbered proposal when that lane rewrote it and milestone 433
  numbered it the same day.
- **Recorded.** `crates/machine_discovery/src/aarch64.rs`'s BUGS: `TGran4 = 0b0001` (FEAT_LPA2) is
  read as "no 4 KiB granule", so the kernel refuses `-cpu max` and any LPA2 part. Found by this
  half's CPU sweep; a two-line fix outside this milestone.
- **Recorded.** `notes/riscv-cycle-counters.md`: three limits the riscv64 half ships with, each
  beside the feature. It configures the boot hart only, because SBI PMU counters are per-hart
  and the one consumer is a single-hart probe. It refuses a firmware counter rather than reading
  one at an `ecall` per read. And nothing has been measured on silicon, which is the whole
  reason that note is mostly a bench procedure.
- **Recorded.** `kernel/src/arch/riscv64/pmu.rs`: QEMU's `rva23s64` model hands back an
  `hpmcounter` that TCG does not drive, so a counter firmware describes as working can read zero
  forever. `init` now refuses such a counter, and the module's `BUGS` and the stop test's own doc
  comment carry the observation.

## Index row

16a's deliverable names "benches on real cycles via the SBI PMU extension". Both halves are
built. riscv64 (2026-09-03): SBI PMU is probed as the first *optional* row of `SBI_TABLE`,
`arch::riscv64::pmu` starts a cycle counter and remembers which CSR reads it, and `bench --riscv`
prints one `cycles_per_tick` probe; radon measured `250.00` on 2026-09-16. aarch64 (2026-09-19):
every core starts and checks `PMCCNTR_EL0`, the boot prints the answer and what firmware left in
`PMCCFILTR_EL0`, and `bench` prints the probe under a **provisional** filter; argon has not run it.
What aarch64's counter counts and what the user-mode read is called are an architect's
(design/roadmap/353-the-aarch64-half-of-74.md)
