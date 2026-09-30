---
status: PARTIAL
raised: 2026-08-25
milestone_dependencies: none
decision_dependencies: none
machine_requirements: real silicon; PMU cycle counter
specific_machine: none
needs_person: yes
---
# 168. A multi-tasking workload benchmark: the number that would decide the event-kernel question

The instrument is built, gated and rehearsed on all three architectures.
**Five boots of radon on 2026-09-16 produced numbers, and this block still does not turn `BUILT`**,
because one of the six sweep points did not produce a number so much as a distribution. The measured
curve and the reason are in "What five boots measured" below; the honest summary is that the
**shape** is solid and `tasks=4` was not a figure anyone should quote. **On 2026-09-19 both holes
in the instrument were closed without a board** (every point is now the median of 21 repeats, and
the mix gained page mapping and process creation; see "What changed on 2026-09-19"), so what is
outstanding is one radon bench evening with the new instrument, by `notes/job-mix.md`'s procedure. Started and left partial
2026-09-04 by a lane with no board, which is what the gate below predicts rather than a shortfall
against it. Minted 2026-08-25, from [DECISIONS §96](../decisions/96-process-kernel-or-event-kernel.md)'s own recommendation: *"Build the instrument that could decide it. The blocker is that a multi-tasking workload is the only place the difference appears, and we have none."*

Real silicon, and that is all. **Corrected 2026-09-04 by calef**; it read
`HARDWARE, MILESTONE 127` from 2026-08-25 until then, and the paragraph below explains why that was
right when written and is not now.

**The old gate was doing two jobs and only one of them still needs argon.** Its stated reasons are
that a multi-tasking difference *"needs real hardware scheduling behavior, not TCG or HVF timing"*
and that it wants *"a place to run it that has real PMU access"*. **radon satisfies both.** It is
real silicon, it boots, and milestone 74's riscv64 half landed on 2026-09-04, so
`kernel/src/arch/riscv64/pmu.rs` reads cycles through the SBI PMU extension. When this block was
written radon had no cycle counter at all, which is why milestone 127 was named.

**What milestone 127 is actually for is a different question**, and its own title says so: *the seL4
machine: a Jetson TX1, so identical silicon referees the comparison.* That matters for **milestone
25**'s cross-OS numbers, which are compared against seL4's published runs and therefore want the
machine those runs were made on. It does not matter for DECISIONS §96's question, which asks how much
of *this kernel's* time goes into process-kernel overhead under multi-tasking load. That is a number
nife produces about itself, on any real silicon.

**So the two uses split, and this block serves both without needing the same machine for each:**

- **For §96** (process kernel or event kernel), radon is sufficient and available now.
- **For milestone 25**'s comparison against seL4, argon is still required, and nothing here loosens
  that.

**And building the instrument is gated by neither.** The workload is architecture-neutral and can be
developed and tested under QEMU; what needs silicon is the *number*, not the code. A lane may build
this today and leave the measurement to a bench evening, which is how this milestone was launched.

## What this number will be compared against, written before it exists

**Recorded 2026-09-21, deliberately in advance.** When this milestone produces a multi-tasking
number it will be read against the language-isolation line of work, by anyone who knows the
literature: *The Case for Writing a Kernel in Rust* (APSys '17, DOI 10.1145/3124680.3124717) and
RedLeaf (OSDI '20). Writing down now what makes that comparison honest is what stops it being a
defence afterwards.

**Three things the comparison has to say, or it is not one.**

- **The guarantee differs.** A language-isolated crossing trusts the compiler and the absence of
  `unsafe` in the isolated code; a capability crossing trusts the hardware and a kernel small enough
  to prove things about. A faster number under a weaker guarantee is not a better result, and a
  slower one under a stronger guarantee is not an excuse. Say which was bought.
- **The hardware and the baseline are part of the number.** This tree's own rule already: state what
  each number means and where it is not apples-to-apples, the way milestone 25 (cross-OS performance
  comparison) records the map "tie" as zeroing-bound and the spawn caveat as a lighter object than a
  Unix process.
- **What each system can isolate is not the same set.** nife runs an unmodified `ripgrep` with zero
  patches and confines C behind DECISIONS §31 (the foreign-language seam)'s narrow interface; a
  language-isolated system isolates code it compiled. That is a difference in what the crossing is
  *for*, and it belongs beside the timing rather than after it.

**This section does not say who wins.** That is a claim that leaves the machine, and
`AGENTS.md` puts it in calef's hands along with every other published fact.

## What this is for

DECISIONS §96 asks whether nife should stay a process kernel (what it is today: every thread gets its own kernel stack) or move to an event kernel (one stack per core, explicit continuations), the model seL4, OKL4 and NOVA all eventually adopted. §96 found three of the four inputs to that question already settled by measurement (memory savings are negligible at this project's scale; stack-shrinking is closed off, no slack remains; the verification argument doesn't transfer, since Kani never reaches `kernel/src` here). The fourth input, performance, is the one live, unmeasurable argument: the paper §96 cites (Warton, on Pistachio) found event kernels roughly tied with process kernels on micro-benchmarks but **20% better on a real multi-tasking workload (AIM7)**. Every instrument this project currently owns (`ipc_rtt`, `ipc_rtt_el0`, the icount tripwire, milestone 132's footprint gate) is a micro-benchmark, and would show approximately nothing for this question.

**This milestone is that missing instrument, and nothing else.** It does not decide §96; it produces the number §96 needs to decide itself.

## What was built, 2026-09-04

Four pieces, and the shape is `kernel/src/soak.rs`'s rather than `kernel/src/bench.rs`'s, for a
reason the gate makes concrete: **this number has to be taken on a board, and the bench boot has
never run on one.** What a board takes is `script/board-image` plus `script/board-console`, which is
a kernel feature printing to the serial console.

- **`crates/job_mix`**, the workload's definition, host-tested. What AIM7 actually is, read on
  2026-09-04 from the benchmark's own README and from the ACM TOCS 2016 retrospective rather than
  guessed at; which four of its methodological properties are kept (heterogeneity, per-task random
  order, a task-count sweep, a throughput metric) and which of its 53 Unix-shaped jobs are
  deliberately not. Five jobs stand in for its categories.
- **`fixtures/src/job_mix_task.rs`**, one EL0 task. Processes rather than kernel threads, which is the one
  place this differs from milestone 134's E1 on purpose: §96 asks what a process kernel costs a
  *workload*, and a workload is processes.
- **`kernel/src/jobmix.rs`** (`--features jobmix`), the supervisor. It builds the pool once, releases
  a subrun's worth of tasks, owns the wall clock, and **prints the placement census before the first
  number**, because milestone 240 found placement decides throughput on radon by up to fifteenfold
  and a figure without its arrangement is a draw rather than a result.
- **`script/job-mix`** (QEMU rehearsal) and **`script/board-image --jobmix`** (the card).
  `notes/job-mix.md` is the bench procedure, with a table mapping every observable outcome to what it
  means and where it routes.

Rehearsed on patagonia on 2026-09-04: aarch64, riscv64 and x86_64, sweep complete on each, six
points per run. **Those magnitudes are not a result and are not recorded as one**; TCG models no
cache.

**On the citation, because this tree has carried a fabricated block quote before.** The 20% figure is
the *retrospective's* summary of Warton: Elphinstone and Heiser, ACM TOCS 34(1), April 2016, §4.1,
*"generally within 1% on micro-benchmarks but a 20% performance advantage of the event kernel on a
multitasking workload (AIM7)"*. Warton's own abstract claims something weaker (memory savings
"without degrading the kernels performance", with a performance improvement called preliminary and
needing more experiments). Both are real; they are not the same strength of claim.
`notes/job-mix.md` carries the provenance and the URLs.

## What it needs

- **A real multi-tasking workload**, not a single-operation timing. The cited paper's own instrument is AIM7 (see `notes/l4-lessons.md`'s citation for the exact numbers this project has already quoted from it: *"20% performance advantage on a multi-tasking workload (AIM7)"*); whoever builds this should read the paper's own methodology rather than guess at AIM7's shape from the name, and decide whether to port something AIM7-equivalent or design a workload that exercises the same property (many threads, contended scheduling, real context-switch pressure) in a way that fits this project's own capability model.
- **Real hardware**, per the gate above: this specific difference does not show up under QEMU TCG or Apple's HVF, the same reason `sel4bench` (milestone 25) is deferred to real silicon.
- **A place to run it that has real PMU access**, matching what milestone 25's own `sel4bench` piece already needs from milestone 127's machine, so the two pieces of hardware-gated work should likely be sequenced together rather than treated as unrelated.

## Why it matters, beyond §96

**Milestone 25's cross-OS comparison has the same hole**, and this milestone closes it too rather than duplicating it. Checked directly: milestone 25 is explicitly a set of EL0-measured *primitive* benchmarks (single syscall, single context switch, single IPC round trip, single map, single spawn) compared against lmbench and `sel4bench`; every one of them a micro-benchmark in the same sense §96 means the word. Milestone 25's own remaining piece (`sel4bench`) is also single-operation PMU timing, not a multi-tasking workload. So neither milestone currently has an instrument that could show what a real multi-tasking difference looks like, and building one here serves both.

## What this does not decide

Whether nife should actually switch kernel models. That is DECISIONS §96's own question, and it stays open until this milestone's number exists (or until a real customer-path workload starts creating threads in the hundreds, the other condition §96 names for reopening early).

## What five boots measured, radon, 2026-09-16

Transcripts `bench/radon-2026-09-16/jobmix-boot1.log` through `boot5.log`, all five complete
(six sweep rows and `job-mix: done` each), booted over TFTP from an identical image.

Each `job-mix:` line is the **best of three repeats**, which was `job_mix::REPEATS` and its rule
until 2026-09-19 (the minimum as the least-contended sample). **These are the old instrument's
numbers**, from the five-job mix, and are not comparable with anything the current one prints.

| tasks | boot 1 | boot 2 | boot 3 | boot 4 | boot 5 | spread |
|---|---|---|---|---|---|---|
| 1 | 319,052 | 319,013 | 319,039 | 319,013 | 319,072 | **0.0%** |
| 2 | 626,242 | 622,410 | 626,452 | 628,362 | 629,044 | 1.1% |
| 4 | 849,610 | 793,137 | 929,866 | 766,361 | 991,671 | **29.4%** |
| 8 | 935,893 | 901,011 | 923,732 | 911,129 | 997,548 | 10.7% |
| 16 | 987,572 | 1,003,653 | 940,648 | 946,568 | 987,017 | 6.7% |
| 32 | 1,032,586 | 1,060,264 | 1,054,227 | 1,057,537 | 1,050,944 | 2.7% |

**The shape is the result, and it held on every boot.** 1 to 2 tasks is 1.96x on a machine with four
usable harts, so nearly free; the knee is between 2 and 4; past 8 it is flat; and at 32 tasks, which
is 8x oversubscription, throughput still creeps up rather than falling. A kernel losing significant
time to process-kernel overhead under load would bend earlier **and keep declining**. This plateaus.

**`tasks=4` is not a number, and five boots are what established that rather than five boots being
what it needed.** The variance is *within* a boot, not between boots. Pooling all fifteen repeats at
each sweep point:

| tasks | min ticks | max ticks | worst within-boot spread |
|---|---|---|---|
| 1 | 96,279 | 96,402 | 0.1% |
| 2 | 97,672 | 112,124 | 14.2% |
| 4 | **123,912** | **181,408** | **37.3%** |
| 8 | 246,364 | 299,644 | 18.5% |
| 16 | 489,731 | 544,301 | 6.1% |
| 32 | 927,165 | 1,016,672 | 6.8% |

Boot 3's three repeats at `tasks=4` span 132,148 to 181,408 on their own, which **contains the
entire boot-to-boot range of the best-of-three values**. So a best-of-three drawn from a
distribution this wide is itself a coin flip, which is why boot 4 read 766,361 and boot 5 read
991,671 from the same image. **More power cycles sample the wrong axis.**

**Why `tasks=4` and not elsewhere, from the code rather than from the shape of the output.**
`job_mix::ECHO_SERVERS` is 2, deliberately fewer than the task count so the endpoint is genuinely
contended. At 1 and 2 tasks there is no contention (1:1 or better); at 4 there is 2:1 contention for
the first time, with too few samples to average it; by 8 and above the contention is deeper but the
averaging is better. **The knee and the instability are the same phenomenon**, which is worth
stating because reading them as two facts would suggest the knee is a scheduling cost when it is a
contention point the instrument was built to create.

**A correction, recorded because it was nearly written into this block as a finding.** The
`job-mix-census:` lines were read across three boots as evidence that thread placement is
deterministic (6/9/9/10 every time) and therefore that `notes/soak.md`'s fifteenfold placement
hazard had not materialised. That inference was wrong: the census prints **once, before the sweep**,
and describes the 32-thread pool. It says nothing about where four tasks land during the `tasks=4`
subrun, which is exactly the configuration whose variance was being explained.

**What is solid.** `tasks=1` at 0.0% across five cold boots is a determinism check on the instrument
itself, and it passes. The curve shape is stable. The `tasks=32` figure, about 1.05M jobs per minute,
is the most repeatable point on the sweep at 2.7%.

## What changed on 2026-09-19: the two holes, closed without a board

A lane on patagonia, with no hardware, closed the two things that stopped the five-boot result above
carrying a verdict for `design/fatal-risks/README.md`'s risk 4. Neither changes what a bench evening does
beyond reading different lines; `notes/job-mix.md` has the rewritten procedure.

### The first hole: every point is the median of 21, not the best of 3

**The evidence, from the five boots' own repeats.** Pooling the fifteen repeats each sweep point
produced and resampling them (five simulated boots of N repeats, 2,000 trials, the reported
statistic's boot-to-boot spread averaged), with `tasks=4` as the worst case:

| statistic | N = 3 | N = 5 | N = 9 | N = 15 | N = 21 | N = 31 |
|---|---|---|---|---|---|---|
| best of N, `tasks=4` | 21.9% | 19.1% | 13.7% | 8.7% | 6.0% | 3.3% |
| median of N, `tasks=4` | 16.5% | 11.9% | 7.6% | 5.1% | 4.2% | 3.3% |
| median of N, `tasks=8` | 7.9% | 6.5% | 5.0% | 4.0% | 3.4% | 2.9% |
| median of N, `tasks=32` | 5.4% | 4.8% | 3.9% | 3.1% | 2.6% | 2.0% |

The simulated best-of-3 (21.9%) is in the neighbourhood of the observed 29.4%, which is what makes
the rest of the table worth reading. **The best-of rows flatter themselves at large N**: a resample
can never go below the fifteen samples' own minimum, where real repeats would keep finding luckier
ones. That artifact is exactly why the minimum is the wrong statistic for a wide distribution, and
the median has no such floor to hide behind.

**Why the median and not the best, beyond the numbers.** The best-of rule is `kernel/src/bench.rs`'s
and it is right for a micro-benchmark on a shared host, where everything above the minimum is
somebody else's load. Here the spread is the tasks' own contention, the thing the instrument exists
to measure. **A reading it changes**: radon's `tasks=2` repeats are bimodal (five near 98,000 ticks,
ten near 111,000), best-of-3 reported the fast mode on every boot, and the median sits in the slow
one, about 1.73x over one task where the table above reads 1.96x.

**Why 21 and not a table per point.** The proposal this absorbs (below) wanted repeats to vary by
point because board time was thought to be the cost. It is not: at 21 repeats the timed windows are
about eleven seconds at the old mix's rates, against a boot that takes minutes. One constant is one
fewer thing for a reader to hold, and 21 is odd so the median is a sample. The TCG rehearsal is what
grew (riscv64 from 20 seconds to 188), and a gate pays that, not a person.

**The printed line changed, on purpose so nothing reads it silently.** `ticks=` and `jpm=` no longer
occur on a point line; it now carries `repeats=`, `ticks_min=`, `ticks_median=`, `ticks_max=` and
`jpm_median=`, and a new line after the census says which statistic is in use and that transcripts
before 2026-09-19 are not comparable. `notes/job-mix.md` has the old-and-new table. Only
`cargo xtask job-mix` reads these lines in-tree, and it counts points by the unchanged
`job-mix: tasks=` prefix; `crates/board_console` has no recogniser for this run (milestone 324).

### The second hole: page mapping and process creation are in the mix

`job_mix::MAP` splits a region from the task's own budget, builds an address space and a frame in
it, maps the frame at 32 addresses and `DESTROY`s the lot. `job_mix::SPAWN` builds two children from
EL0 (address space, stack, thread, the shared child code), `RECV`s each one's exit and reclaims its
region. **No syscall, method or object type was added.** Each task is granted an untyped budget
(`memory_region_cap`) and a child-done endpoint through the existing spawn grants, which is the shape
`kernel/src/bench.rs`'s `map_el0` and `spawn_el0` already had; the proposal's worry that a map job
"needs the spawn path to hand a task a capability on its own address space" dissolved once the task
could create a space of its own. The mix is still sixteen jobs a round: two of each non-IPC kind and
four round trips, where it was three of each of four kinds and four round trips.

**The allocator worry, measured rather than assumed.** The proposal refused a spawn job because at 32
tasks it "would measure the memory-region allocator rather than the scheduler". Each task now times
its `SPLIT` and `DESTROY` calls, and the supervisor prints them per kind on a new `job-mix-kind:` line
beside each point, with the kind's total ticks and job count, collected after the clock stops. Under
TCG on aarch64 (a proof the accounting works, not a finding): region calls were 35% of a map job at
one task and 20% at 32, 27% of a spawn job at one task and 9% at 32. So under emulation the share
**falls** with load, because waiting for a child to be scheduled grows faster than the lock. Whether
that holds on radon is one of the things the next evening reads.

**The disk-file mix was not built**, and not for effort. Nothing nife runs on radon can read a disk
(every file-service path starts at a virtio block device), so it could only ever produce QEMU
numbers; and the file service maps one channel into every client, which 32 concurrent tasks would
race on. Both, with options and no recommendation (the second is a wire format), are in
`design/roadmap/493-a-disk-file-job-mix-needs-a-disk-radon-can-drive.md`.

### Smaller things found on the way

- **`script/job-mix --arch riscv64` failed on any fresh checkout** (exit 3, before the kernel printed
  a line): the riscv64 runner refuses a missing `NIFE_DISK` and only the aarch64 arm built one. It
  passed on 2026-09-04 because an aarch64 run had come first. Fixed in `xtask`.
- **The hand-assembled child both spawn loops run moved to `user_mode_runtime::child_stub`**, from a
  private constant in `os_primitives_benchmarker`, rather than being copied. Bytes unchanged.
- **The supervisor now fails loudly on a refused job**: a task reports `REPORT_FAILED` with the error
  and the kind, and the run ends at `job-mix: FAILED` rather than printing ticks for work not done.

### The HVF cross-check calef asked for could not run

On 2026-09-19 calef asked for the sweep under HVF on patagonia's M3 cores as a cross-check on the
shape (never a result). `script/job-mix` gained `--hvf` and `--release`, spelled as `run` and
`bench` already spell them. Five attempts on an otherwise idle host were each refused by QEMU 11.1.1
before the kernel ran, `HVF does not support GICv2 emulation`, because `kernel/src/drivers/gic.rs` is
GICv2 only. **Milestone 227** (a GICv3 driver) is what gives this cross-check a machine. Transcript
`bench/patagonia-hvf-2026-09-19/jobmix-hvf-refused.log`; `notes/job-mix.md` has the section.

### Two proposals absorbed, and closed as the milestones they had become

**They were deleted by the lane that absorbed them and restored as numbered blocks at merge**, both
on 2026-09-19, because another session had promoted them hours earlier under milestone 433's drain
and the two sessions could not see each other. calef's ruling there decides it: a proposal is
promoted and then closed, since **a numbered block marked BUILT is a record and a deleted file is
nothing**. So each is `BUILT` below rather than absent, with what it bought and what it did not.

- [**Milestone 382**](382-three-aim7-job-categories-the-job-mix-does-not-have.md) (filed 2026-09-04):
  three AIM7 categories missing, why each was refused, and the order to add them (map, spawn, then
  disk as a second mix). Its content is "The second hole" above, and the refusals it recorded are answered there:
  the map job did not need a new capability, the spawn job's allocator share is now printed, and the
  disk mix has its own proposal.
- [**Milestone 419**](419-more-repeats-where-the-job-mix-contends.md) (filed 2026-09-16): `tasks=4`
  needs more repeats, not more power cycles, with four options and a recommendation of reporting
  the spread. What was built is its options 1 and 3 together (a uniform 21, and the median with the
  ends beside it); its option 2, a per-point table, was refused above on measured board time. It
  marked itself `Gate: DECISION` because the line format changes; the maintainer's brief for this
  lane assigned the change, both readers are in-tree, and it is recorded here so the decision is
  visible rather than implied. **That gate had a written-up section by the end of the same day**,
  [§191](../decisions/191-job-mix-repeats-and-what-the-line-reports.md), from milestone 435's sweep
  of forty-five blocks whose `DECISION` gate named no decision. calef ratified what shipped on
  2026-09-26 (*"Yes, ratify §191 as shipped."*), so §191 (whether the job mix reports the spread
  rather than the best) is `DECIDED`.

## BUGS

- **No seven-job, median-of-21 sweep has run on silicon.** Every claim about the new instrument's
  stability is a resampling of the old instrument's data. The map and spawn jobs may change
  `tasks=4`'s distribution; step 6 of `notes/job-mix.md`'s procedure is the check.
- **The disk-file category is still absent**, with its own proposal. A flat curve from this mix is
  evidence about compute, memory, trap, scheduling, IPC, mapping and process creation, and silent on
  the filesystem.
- **The per-kind ticks include preemption**, by design, so they are not a pure path cost. Read their
  growth with task count, not their magnitude.
- **The breakdown exchange between repeats warms the report endpoint and the supervisor's stack**,
  which the old sweep did not do. It is identical for every repeat.
- **The HVF cross-check has no machine until milestone 227.**

## Follow-on

- **Outstanding.** **One radon bench evening with the 2026-09-19 instrument**, at least five boots,
  by `notes/job-mix.md`'s procedure (its step 7 says how the medians read for risk 4). This bullet
  said until 2026-09-19 that "the number itself, on radon" was outstanding and that the Results
  table was empty; **both were stale by then**, since five boots on 2026-09-16 produced numbers and
  the table now has that row. What those boots could not produce was a number at every point, and
  what closed that was an instrument change rather than more boots.
- **Outstanding.** **What the sweep cannot see above 32 tasks.** `job_mix::MAX_TASKS` is 32 because
  every task is a process with an address space, where milestone 134's E1 reached 96 kernel threads.
  Checked by reading `crates/job_mix` and `kernel/src/jobmix.rs`: nothing has measured where a board
  actually runs out, and the supervisor's `FAILED: could not spawn task N` line is the measurement
  that would say. It is part of the first bench evening rather than separate work.
- **Milestone 324.** `crates/board_console` has no recogniser for this run, so an operator tells a
  finished sweep from a wedged one by reading the log, with the reason it
  was deliberately not written blind: the outcome table in `notes/job-mix.md` becomes evidence after
  the first bench evening and a guess before it.
- **Milestone 493.** The disk-file mix, blocked on a disk radon can drive and on a file-service
  channel per client (a wire format, so calef's).
  milestone 493 (a disk-file job mix needs a disk), `design/roadmap/493-a-disk-file-job-mix-needs-a-disk-radon-can-drive.md`.
- **Milestone 227.** The HVF cross-check on patagonia runs once the kernel has a GICv3 driver:
  `script/job-mix --hvf --release --smp 4`, alone on the host.
- **Recorded.** The mix proportions are chosen rather than derived from an AIM7 workfile, so a
  different mix gives a different number and no result is quotable without saying which mix produced
  it. It stays a limitation: nobody has an AIM7 workfile for a capability microkernel, and inventing
  one would be a stronger claim than the evidence supports. Recorded beside the feature in
  `crates/job_mix`'s own `BUGS` and again in `notes/job-mix.md`.
- **Refused.** A committed baseline and a `--check` for this sweep, the way `script/bench` gates
  against `bench/baseline-*.txt`. Refused because the icount instrument's determinism is what makes
  that gate meaningful, and a workload whose entire subject is scheduling under contention is not
  deterministic on any accelerator this tree has. A gate here would be asserting a tolerance nobody
  has measured, which is how `script/lint` has already lost three checks.
  This refusal is
  milestone 464 (design/roadmap/464-a-committed-baseline-for-the-multitasking-sweep.md), which
  carries it with the condition that would change it.

## Index row

Minted from DECISIONS §96's own recommendation: every benchmark this project owns is a
micro-benchmark, and the one number that could decide process-kernel-vs-event-kernel only shows up
on a real multi-tasking workload, on real hardware. The instrument is built, gated and rehearsed
on all three architectures (`crates/job_mix`, `fixtures/src/job_mix_task.rs`, `--features job_mix`, `script/job-mix`). Five radon boots (2026-09-16) measured the shape; the 2026-09-19 instrument (median of 21, map and spawn jobs) **needs one more radon evening** for a number at every point. Closes the same hole in
milestone 25's cross-OS comparison too.
