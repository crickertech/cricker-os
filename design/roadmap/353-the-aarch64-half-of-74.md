---
status: PARTIAL
raised: 2026-09-03
milestone_dependencies: none
decision_dependencies: unwritten
machine_requirements: none
specific_machine: none
needs_person: no
---
# 353. The aarch64 half of milestone 74: two decisions left, now that the counter runs

Filed 2026-09-03 as an unnumbered proposal by the
`milestone/74-cycle-counters-riscv` lane, numbered 2026-09-19 by milestone 433 (drain), and the work it
described was built the same day by the `milestone/74-cycle-counters-aarch64` lane: `PMCR_EL0.E`
and `PMCNTENSET_EL0.C` are written, so `PMCCNTR_EL0` is no longer a stopped counter. What is left
is not code. It is two rulings, and they are what this block now holds.

Nothing is blocked from building. What is blocked is publishing: no aarch64
cycle figure should be quoted until decision A is made, and no program should read the counter
through a shared function until decision B is.

Two branches met in this file and both were right. One numbered the proposal; the other rewrote
it because it had built the thing the proposal proposed. The number and the filename are this
branch's, the content below is that lane's, and nothing of either was dropped.

Rewritten 2026-09-19 by the `milestone/74-cycle-counters-aarch64` lane, which built the half this
file used to describe. The first version was the riscv64 lane's
handoff: `PMCR_EL0.E` and `PMCNTENSET_EL0.C` were never written, so `PMCCNTR_EL0` was a stopped
counter. That part is built (milestone 74's block, "What the aarch64 half built"). What is left is the
two things that lane and this one were both told are an architect's, and this file is now those two
decisions, each answered against AGENTS.md's seven questions, with options and no winner, because
both are facts that leave the machine.

## Decision A: what `PMCCFILTR_EL0` counts

Ruled 2026-09-19 (21:34 UTC), calef: wait for argon's firmware value. The TX1 is awaited,
undated (2026-10-06). The choice is deferred
until argon's first boot prints `firmware left PMCCFILTR_EL0 0x...`, and then the filter is set to
match what that firmware left, which is what seL4's published 413 and 426 were counted under
(Question 3 below reads their source for this). Until then the provisional `0`
stands, the boot line keeps saying `PROVISIONAL`, and no aarch64 cycle figure is published,
milestone 25's included.

Why this is not a deferral in the bad sense. Every option below is an argument about what
seL4's number means; one line of argon's own console output replaces the argument with evidence,
and that boot is scheduled work (milestone 127 (sel4)'s bring-up). The cost of waiting is that
milestone 25's aarch64 row stays empty until argon's first evening, which it would anyway: there
is no aarch64 board number yet.

What the bench evening owes this block, beyond reading that line: if firmware left `P` set
(kernel cycles excluded), say so loudly, because then seL4's figures exclude the kernel and every
comparison in `notes/benchmarks.md` needs re-reading rather than re-running. If it left `0`, which
is what QEMU reports, A1 is what argon inherits and the provisional value was right by accident.
Either way the value goes in this block, the filter stops being provisional, and decision B stays
open.

### What is being decided

`PMCCFILTR_EL0` says in which exception levels `PMCCNTR_EL0` increments. Its reset value is
architecturally UNKNOWN on every field (Arm's `AArch64-pmccfiltr_el0` page, read 2026-09-19 at
`arm.jonpalmisc.com/latest_sysreg/AArch64-pmccfiltr_el0`). The fields that matter here:

| bit | field | meaning when set |
|---|---|---|
| 31 | `P` | do not count EL1 |
| 30 | `U` | do not count EL0 |
| 29, 28 | `NSK`, `NSU` | Non-secure EL1, EL0: counted as `P`, `U` say when equal to them, not counted when different |
| 27 | `NSH` | do count EL2 (the one field whose sense is inverted) |
| 26 | `M` | EL3: counted as `P` says when equal to it, not counted when different |

What the kernel writes today is `0`, provisionally (`arch::aarch64::pmu::PMCCFILTR_PROVISIONAL`,
whose doc comment carries the reasons and points here). That counts EL0 and EL1, not EL2, and EL3
wherever `MDCR_EL3` permits counting there at all. The boot line prints `PROVISIONAL` beside it and
the bench probe's meaning line says the same, so a number cannot leave the machine without the
qualifier.

### The options

| | value | counts | the question it answers |
|---|---|---|---|
| A1 | `0x0000_0000` | EL0, EL1 (EL3 if permitted) | "what did this operation cost the machine, kernel included, with nothing below us" |
| A2 | `0x0800_0000` (`NSH`) | EL0, EL1, EL2 (EL3 if permitted) | the same, plus any time a hypervisor below us spends on our behalf |
| A3 | `0x8000_0000` (`P`) | EL0 only | "what did userspace spend", a profile |
| A4 | not written | whatever firmware left | "the same as whatever else ran on this board's firmware" |

### Question 1: What else was considered, and why each is a live option rather than a loser

None loses on the facts; they answer different questions, which is why this is an architect's.

- A3 cannot referee an IPC comparison, and that is the one thing to know about it. An IPC round
  trip timed from EL0 spends almost all of its cycles in the kernel, and `P` = 1 stops the counter
  there. The number would be the user-mode instructions around `svc`. It is the right instrument for
  a userspace profile (milestone 147's profiler) and the wrong one for milestone 25.
- A4 is what seL4 does, and it is the one option this tree's own rule argues against. It inherits
  an UNKNOWN value, which is the exact defect milestone 228 fixed for `PMUSERENR_EL0`. It is listed
  because it is literally the seL4 configuration (below), and a reader deserves to know that the
  comparison target made this choice.
- A1 versus A2 changes nothing on this kernel today. `boot.s` drops to EL1 and installs no EL2
  vector table, so no cycle is ever spent at EL2 on argon or under QEMU TCG. They diverge only under
  a hypervisor that lets a guest program the PMU, or if this kernel ever ran at EL2 (VHE). Choosing
  between them is choosing what a future number would mean, not what today's means.

### Question 2: What this tree already does in the analogous case

Both other architectures count every privilege level, including the kernel's, and neither was
decided as a published position; each is the mechanism's default:

- riscv64 (`arch::riscv64::pmu`): `sbi_pmu_counter_config_matching` is called with
  `CLEAR_VALUE | AUTO_START` and none of the specification's inhibit flags (`SINH`, `UINH`, `MINH`,
  `VSINH`, `VUINH`), so the counter runs in every mode, M-mode firmware included. radon's
  published `cycles_per_tick 250.00` was measured that way.
- x86_64 (`arch::x86_64::pmu`, milestone 309 (unhalted)): `FIXED_CTR1_BOTH_RINGS`, ring 0 and ring 3, and
  its doc comment says why in one line: "a counter that stopped at the ring boundary would make
  `x86_64`'s number mean something different from theirs".

So A1 (or A2) is the option consistent with the two numbers in this tree, and A3 would make
aarch64 the odd one out. That is precedent, not a ruling: neither of those was put to calef either.

### Question 3: The prior art, read

- seL4 never writes `PMCCFILTR_EL0` in the configuration it publishes. The kernel's
  `arm_init_ccnt` (`src/arch/arm/benchmark/benchmark.c`, read 2026-09-19) writes
  `PMCR = E | C | P` and `PMCNTENSET` bit 31, and nothing else. libsel4bench's `sel4bench_init`
  (`libsel4bench/arch_include/arm/armv/armv8-a/sel4bench/armv/sel4bench.h`, read 2026-09-19) writes
  the filter only `#ifdef CONFIG_ARM_HYPERVISOR_SUPPORT`, and then to `BIT(27)`, which is `NSH`:
  with the kernel at EL2, it counts EL2. sel4bench's `settings.cmake` (read 2026-09-19) turns
  hypervisor support on only when `VCPU` is set, and the build line on
  `sel4.systems/performance.html` for the TX1 (read 2026-09-19: `init-build.sh` with `FASTPATH`,
  `HARDWARE`, `FAULT` and `AARCH64` set to `TRUE`, `ITERATIONS=5` and `PLATFORM=tx1`) sets no
  `VCPU`. So the 413 and
  426 were counted under whatever filter the TX1's firmware left, with seL4's kernel at EL1. The
  kernel is counted unless that firmware set `P`.
- What that firmware left is now something argon prints. Since this lane, the boot line reads
  `firmware left PMCCFILTR_EL0 0x... on this core before it was overwritten`. QEMU says `0x0`. argon
  runs the same family of TF-A and U-Boot the Foundation's board did, so its first boot is the best
  evidence available for what seL4's figures included. It is not proof: nobody here knows the
  firmware revision on the Foundation's bench.
- Linux `perf` counts everything by default and only userspace when it is not allowed more.
  `armv8pmu_set_event_filter` (`drivers/perf/arm_pmuv3.c`, read 2026-09-19) sets
  `ARMV8_PMU_EXCLUDE_EL1` (`P`) only for `exclude_kernel`, `EXCLUDE_EL0` only for `exclude_user`,
  and `ARMV8_PMU_INCLUDE_EL2` (`NSH`) unless `exclude_hv`. A plain `perf stat -e cycles` as root is
  therefore A2. Unprivileged, with the default `perf_event_paranoid` of 2, the tool retries with
  `exclude_kernel` and `exclude_hv` set and says so (`tools/perf/util/evsel.c`, read 2026-09-19:
  "kernel.perf_event_paranoid=%d, trying to fall back to excluding kernel and hypervisor samples"),
  which is A3. So the Linux answer to "what does `cycles` count" depends on who is asking.

### Question 4: Is the premise true?

Mostly, with one correction worth making before calef rules. The brief framed this as
"seL4-comparable versus userspace-only". **seL4-comparable is itself not a single value**: it is A4
on the published board, which is whatever that board's firmware left (argon's boot line is the best
guess at it, and QEMU's `0x0` would make it A1), and A2 in seL4's own hypervisor builds. So the decision is between "count the kernel" (A1 or A2) and "count userspace" (A3),
with A1 against A2 a second, smaller question about a configuration nothing runs today.

### Question 5: What each costs, measured rather than asserted

**The same, to build: one constant.** `PMCCFILTR_PROVISIONAL` is written once per core at init and
never on the switch path, so no option moves `script/fastpath-footprint` or the icount tripwire
(both were run with A1 in place; the block has the exit codes). **Not measured**: whether the filter
changes what a read costs, because QEMU does not model that and there is no silicon here. One real
cost difference exists and it is not performance: **A3 breaks `bench::cycles_per_tick`**, which
reads the counter at EL1 and would see it stopped. Choosing A3 means moving the probe to EL0, which
is milestone 237's grant build and a measurement build rather than the plain `--features bench`
boot.

### Question 6: How reversible, and who has already acted on it

The code is reversible in a line. **The number is not**, once published, and nothing has been: this
lane printed only QEMU's `16.00`, which is an instruction count and says so. radon's `250.00` and
xenon's future figure are the precedent a reader will compare an aarch64 number against, and both
count the kernel.

### Question 7: Would the choice be the same if every option cost the same?

They do cost the same. Nothing here is an effort argument, which is why the table above carries no
recommendation: it is a question about what nife wants its published numbers to mean.

### What happened when calef ruled

He took none of A1 to A3 and chose the evidence instead: **wait for argon's firmware value**
(2026-09-19). The paragraph at the top of this decision is the ruling; this section is kept because
its prediction held, which is that the provisional A1 stays and milestone 25 cannot publish an
aarch64 cycle figure until the bench evening.

## Decision B: the portable user-mode cycle read, its name and its promise

### What is being decided

`fixtures/src/cycle_counter_reader.rs` carries the only user-mode read in the tree, one `mrs` (or
`csrr cycle`, or `rdtsc`) per architecture, and its doc comment says a portable function is milestone
74's to design. **This lane did not add it**, by instruction. The questions are what
`crates/user_mode_runtime` should call it, what it should promise, and whether it should exist yet.

### The fact that decides most of it: the three user-readable counters are not the same quantity

| | what EL0/U-mode/ring 3 reads | is it core cycles? | granted by |
|---|---|---|---|
| aarch64 | `PMCCNTR_EL0` | yes, filtered by decision A | milestone 229's grant (`PMUSERENR_EL0.CR`) |
| riscv64 | the `cycle` CSR | yes, **but** the kernel's own probe may have been handed a different counter (`hpmcounter3` on `rva23s64`) | 229's grant (`scounteren.CY`), and `mcounteren.CY` in firmware |
| x86_64 | `rdtsc` | **no**: constant-rate, the same counter as `user_mode_runtime::now()` | ambient (DECISIONS §139 part 3) |

The kernel's `cycles_per_tick` probe on x86_64 deliberately does **not** read the TSC (milestone
309's whole argument: it would divide one counter by itself). A user-mode function called
`cycles()` that read `rdtsc` on x86_64 would therefore be the exact mistake 309 was written to
prevent, in the one place a stranger would call it. Reading core cycles from ring 3 needs `rdpmc`
with `CR4.PCE` set, which milestone 228 deliberately closed and which is a DECISIONS §139 question,
not a naming one.

### The second fact: EL0 cannot ask whether it may read

An ungranted read is not an error return, it is a fault that ends the thread (that is the negative
half of `a_granted_thread_reads_the_cycle_counter_and_an_ungranted_one_faults`). So a user-mode
function cannot probe and fall back. Whatever it promises, it has to know from somewhere other than
trying.

### The options

- B1. No shared function yet. Programs that need the counter read it raw, as the one fixture
  does. The name waits for a second consumer, which is this tree's usual rule against building the
  abstraction first. Cost: each consumer re-derives the per-architecture caveats above.
- B2. A raw read whose name says what it reads, for example a `cycle_counter()` that exists on
  aarch64 and riscv64 only and is documented as "fatal unless the program's manifest carries the
  cycle-counter grant". Honest and small; the cost is that it is not portable, which on a project
  with §19 as a gate needs a scope note for x86_64.
- B3. A portable read that promises core cycles on all three, which on x86_64 means `rdpmc` of
  fixed counter 1 and therefore reopening `CR4.PCE` behind the grant. That is a §139 extension
  (a new door, granted) and a syscall-surface-adjacent change, so it is two decisions wearing one
  name.
- **B4. A read paired with its meaning**, a function returning the count together with what it
  counts (the filter, the core it ran on), in the shape the kernel's probe prints a meaning line.
  Linux's answer is closest to this: user-space self-monitoring reads the counter directly, and the
  `perf_event_mmap_page` it maps says whether it may (`cap_user_rdpmc`) and how wide the counter is
  (`pmc_width`). That is also what solves the second fact above. It is the most machinery and the
  only option that lets a program ask before it reads.

### The seven questions, briefly

1. Alternatives: the four above; each answers a different "what does a caller need".
2. This tree: `user_mode_runtime::now()` and `cntfrq()` are the analogous pair, and `now()` is
   documented per architecture with its caveats in a `BUGS` section. The kernel names the per-arch
   module `pmu` and the reading `cycles()` on all three, and on x86_64 that kernel `cycles()` is not
   the TSC. A user-mode `cycles()` that was the TSC would collide with the kernel's own meaning of
   the word.
3. Prior art: seL4's `sel4bench_get_cycle_count()` is a raw read with no promise beyond
   "whatever `PMCCNTR` says" (libsel4bench, read 2026-09-19); Linux has no libc function and uses the
   mmap page above; Rust's `core::arch::x86_64::_rdtsc` names the instruction rather than the
   quantity.
4. Premise: true that it is a naming decision; **not only** a naming decision, because B3 is a
   §139 extension on x86_64.
5. Cost: B1 zero; B2 one function and a scope note; B3 a `CR4.PCE` grant on the switch path
   (milestone 237 measured what the aarch64 and riscv64 version of that cost the fastpath); B4 a
   page or a call. None was built or measured here.
6. Reversibility: a public function in `user_mode_runtime` is a name programs compile against;
   renaming it later touches every caller, which is the expensive category.
7. **Equal cost**: B1 and B2 are not chosen for being cheap; B4 would still be the most honest if
   all four cost the same, and B3 is the only one that is more than a naming question.

### What happens if calef says nothing

B1 holds: the read stays in the fixture, which is where it is today, and nothing is worse than it
was.

## Where it came from

The riscv64 lane's handoff (2026-09-03, first version of this file) and the aarch64 lane that built
the counter (2026-09-19). Everything cited above was read on the date given, not recalled.

## Follow-on

- **Outstanding.** *Decision A, what `PMCCFILTR_EL0` counts.* Blocks publishing any aarch64 cycle
  figure, because a count that excludes the kernel is not comparable to seL4's and one that includes
  it is not comparable to a userspace-only profile. The aarch64 meaning line says `PROVISIONAL` so a
  number cannot travel without it. Options and costs are above, with no winner named, because it is
  a fact that leaves the machine.
- **Outstanding.** *Decision B, what a program calls the cycle-counter read and what it promises.*
  Blocks a shared function; the read stays in the fixture until it is answered, which is where it is
  today.
- **Recorded.** *Both decisions live in this roadmap block rather than in `design/decisions/`*, which
  is the defect milestone 435 swept forty-five blocks for the same evening. They are kept here
  rather than minted because a section number is global to the tree and two sessions collided on one
  three times in ninety minutes; the integrator mints them once this branch lands. Until then the
  options and their costs are at the thing itself, which is rung three, and that is better than the
  paragraph-addressed-to-one-person the sweep was correcting.

## Index row

`PMCR_EL0.E` and `PMCNTENSET_EL0.C` were never written by this kernel, so `PMCCNTR_EL0` was a
stopped counter: a thread holding the milestone 229 grant could read it, legally, and got the same
number every time. That was built on 2026-09-19 and `bench::cycles_per_tick` now has no `cfg` at
all, so the measurement half of milestone 74 exists on all three architectures. **The three are not
yet the same quantity**, and closing that gap is a ruling rather than code: riscv64 counts every
mode including M-mode firmware, x86_64 counts ring 0 and 3, and aarch64 counts EL0 and EL1 under a
provisional `PMCCFILTR_EL0`. Two things here are an architect's and both are facts that leave the
machine: what `PMCCFILTR_EL0` counts, because a count excluding the kernel is not comparable to
seL4's and one including it is not comparable to a userspace-only profile, and what a program calls
the cycle-counter read and what it promises. §19 (parity) makes the first a
parity gap in the one subsystem whose entire purpose is cross-machine comparison, and
milestone 25 (performance)'s `sel4bench` needs it. Nothing can be settled on
Apple silicon: the PMU is not architected state a hypervisor must present, so the machine that
decides it is argon, with a person at it.
