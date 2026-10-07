# The multi-tasking workload benchmark, and how to take its number

*(Milestone 168 (benchmark). Names here are provisional, per
the naming tenet; an architect names things.)*

Written on 2026-09-04 with the board off, rewritten on 2026-09-19 after five radon boots, and
followed by two bench evenings (Results, below). Everything else is about code in this tree, built,
host-tested and rehearsed under QEMU on all three architectures, or is a question for the bench,
marked as one.

## What this instrument is for, in one paragraph

`design/decisions/0096-process-kernel-or-event-kernel.md` asks whether this kernel should keep a kernel
stack per thread (a process kernel, what it is) or move to one stack per core with explicit
continuations (an event kernel, what seL4, OKL4 and NOVA all became). Three of its four inputs
are settled by measurement. The fourth is performance, and the retrospective it rests on says
precisely where the difference lives:

> Warton [2005] performed a thorough performance evaluation of the Pistachio process kernel vs an
> event-based (single-stack) kernel with continuations on an ARMv5 processor. He demonstrated
> comparable performance, generally within 1% on micro-benchmarks but a 20% performance advantage of
> the event kernel on a multitasking workload (AIM7).

Elphinstone and Heiser, *L4 Microkernels: The Lessons from 20 Years of Research and Deployment*, ACM
Transactions on Computer Systems 34(1), article 1, April 2016, section 4.1. Read on 2026-09-04
from <https://trustworthy.systems/publications/nicta_full_text/8988.pdf>.

Every instrument this project owns is on the left of that sentence. `ipc_rtt`, `ipc_rtt_el0`, the
icount tripwire, milestone 132 (fast)'s footprint gate, and milestone 134 (register)'s E1 through E4 are all
single-operation or single-shape measurements. This is the one that is not.

### Two things about the citation that are worth knowing before quoting it

The 20% is the retrospective's summary of Warton, not Warton's own claim. His thesis abstract
(read the same day from
<https://trustworthy.systems/publications/theses_public/05/Warton:be.abstract>) says only that
"significant memory savings can be achieved without degrading the kernels performance" and that
"preliminary results show improvement in the kernels performance due to the single stack
architecure, however more experiments are required to verify this result." Both are real citations; they are not the same strength of
claim, and this tree has carried a fabricated block quote for twelve days, so the provenance
is written down rather than remembered.

Since 2026-09-13 the thesis body has been read too, and it is harder on the number than the
abstract is: Warton expected the opposite result, called his own 20% something to treat "with
skepticism until it can be satisfactorily explained", and never ran the cache simulation that would
have explained it. The next section has the quotations.

The result is from 2005, on ARMv5, on Pistachio. Nothing about it transfers to this kernel by
assumption. That is the whole reason this milestone exists rather than a rewrite.

## What Warton ran AIM7 on, and why the 20% is not a target

Checked on 2026-09-13, because AGENTS.md's fourth question is *is the premise true?* and nobody
had asked what the 20% was measured on.

### The retrospective does not say, and that is worth knowing before quoting it

Read from <https://trustworthy.systems/publications/nicta_full_text/8988.pdf> (629,888 bytes, 30
pages). "AIM7" occurs exactly once in the paper, in the sentence this page quotes above. The
whole setup it gives is section 4.1, page 1:16: *"the Pistachio process kernel vs an event-based
(single-stack) kernel with continuations on an ARMv5 processor."* Two arms and an ISA. The
software above the kernel is not named, and "Wombat" appears in the paper only as a bibliography
entry cited from section 5.1 (Virtualisation), unconnected to Warton's measurement.

### Warton's own thesis says, and the answer is Wombat

<https://trustworthy.systems/publications/theses_public/05/Warton%3Abe.pdf>, read the same day
(361,145 bytes, 43 pages). Matthew Warton, *Single Kernel Stack L4*, BE thesis, UNSW, submitted
2 November 2005, supervisor Gernot Heiser. Section 5.4:

> The AIM7 benchmark is a measures system performance by simulating workload on a multiuser system.
> The AIM7 benchmark was modified slightly so that it could run on Wombat. The modifications included
> disabling the network operation simulations because Wombat does not support the GetHost function,
> and disabling the file system operation simulations, because Wombat runs from a ram disk, and the
> ram disk is not large enough to support the benchmarks.

(`pypdf` drops this PDF's `fi` ligature, so "modified", "file", "benefits" and "conflict" arrive
from the extractor with the `fi` missing. They are restored in every quotation on this page, since
the ligature is an artifact of reading the document rather than of writing it. Nothing else in any
quotation here is altered.)

Wombat is the paravirtualised ARM Linux, the same one the retrospective cites at Leslie, van
Schaik and Heiser 2005. So the 20% is a delta between two microkernels measured through a hosted
Linux's syscall path. Four things follow, and each one costs this instrument something different.

1. The premise fails. `crates/job_mix` runs native tasks on nife's own primitives. Warton
measured Linux processes whose every system call became an exception IPC to a user-level Linux
server. Those are different experiments, and no fidelity work on the mix turns one into the other.

2. The 20% is a ratio, and this tree has one kernel. Section 6's table, five runs:

| | single stack | multi stack | variable stack |
|---|---|---|---|
| average time | 157.69 | 197.22 | 157.78 |
| standard deviation | 0.015 | 0.344 | 0.019 |

(197.22 − 157.69) / 197.22 is 20.0%, which is where the retrospective's figure comes from. It is a
paired measurement: neither column means anything alone. nife has no event kernel to be the
other column, so the mix produces one arm and no ratio, whatever jobs it contains.

3. Warton's AIM7 run had already disabled two of the three categories `crates/job_mix`'s `BUGS`
apologizes for. Section 5.4 turned off the filesystem jobs (the ramdisk was too small) and the
network jobs (Wombat had no `GetHost`). The crate records a missing disk-file category as a fidelity
gap against AIM7; the AIM7 run being cited did not have one either.

4. It was two tasks, and there was no sweep. Section 5.4: *"The precise benchmark used was 2
clients with the normal workload file, with the disk and network tests removed"*, and section 6:
*"This workload was run in two user tasks."* This page lists a task-count sweep as one of AIM7's
four methodological properties and keeps it out to 32 tasks. The run the 20% comes from swept
nothing. The sweep is a good idea on its own merits and it is not a reproduction of Warton.

### What Warton thought of his own number

This is the part the retrospective compresses away entirely, and it is the reason the figure should
never be quoted flat. He expected the opposite result (section 5.2: *"the single stack kernel is
expected to perform similarly to or worse than the multi stack kernel"*), and the micro-benchmarks
delivered it. Section 7.3, on the AIM7 result:

> As with any experimental result, this needs to be treated with scepticism until it can be
> satisfactorily explained. Because the benchmark is not very stable and crashes on some runs, I
> initially doubted the results.

He ruled out the timer by re-running against a wall clock. Then:

> If the result is accurate, it must be due to reductions in the cache and TLB footprint of the
> single stack kernel. I did not expect that this reduction would make such a massive difference in
> performance [...] To determine the validity of this result a simulation of the cache impact of
> the kernels must be performed. There was not enough time to complete this simulation in the
> course of this thesis, due to external events.

The validating simulation was never run, and section 7.4 asks for another macro-benchmark for
the same reason. So the chain this project has been reasoning from is: a BE thesis reports an
unexplained 20% its author flagged as needing skepticism, on a modified AIM7, on two tasks, on a
hosted Linux; a retrospective eleven years later summarizes it as a flat *"20% performance
advantage of the event kernel on a multitasking workload (AIM7)"*; and this tree built an instrument
to chase it. Every step is a real citation. The compression happened at the second one.

### The hardware, for completeness

Section 5.2: a littlechips LN2410SBC single-board computer with *"a Samsung S3C2410 arm processor
clocked at 200 MHz, a 32 kilobyte, 64 way associative cache, and 64 megabytes of ram."* The thesis
never says "ARMv5"; that is the retrospective's own gloss on the SoC. Note the cache: Warton
expected *"not many caching benefits [...] due to the single kernel stacks reduction of conflict
misses"* precisely because 64-way associativity makes conflict misses rare, which is what made the
result surprising to him.

### What survives, and it is the reason to keep the instrument

The mechanism, not the number. The only explanation Warton offered for his 20% is kernel cache
and TLB footprint, which is exactly the quantity `kernel/src/bench.rs`'s `app_displacement` and
milestone 134's E1 measure. Whether per-thread kernel stacks displace enough cache to cost
throughput on this kernel is a real, open, local question, and a knee in jobs-per-minute against
task count answers it. That is a genuine input to
`design/decisions/0096-process-kernel-or-event-kernel.md`. A reproduction of 20% was never available
and is not what this instrument was ever going to deliver.

### One gloss in §96 that the paper contradicts in the same sentence

§96 twice discounts the memory argument with the clause *"Warton's result came from resource-starved
embedded systems"*. The paper's own sentence, section 4.1, page 1:17, is:

> While this decision was driven initially by the realities of resource-starved embedded systems and
> later the needs of verification, the approach's benefits are not restricted to those contexts, and
> we believe it is generally the best approach on modern hardware.

The first half is where §96's clause comes from and it is fair: the move was driven by
resource-starved embedded realities. The second half rejects the inference §96 draws from it, in the
same sentence, and §96 does not record that it exists. The authors are not neutral on this and the
tree should quote them rather than paraphrase them into agreement.

## What the workload is, and what it is not

`crates/job_mix`'s header is canonical and this page does not repeat it. The short version:

AIM7 (read on 2026-09-04 from the benchmark's own README at
<https://github.com/davidlohr/areaim/blob/master/osdl-aim-7/_NOTICES/README.aim7>) forks many
tasks, each running in random order a set of subtests called jobs, over a sequence of
subruns with the task count incremented between them, reporting jobs per minute against task
count. Four methodological properties do the work, and this instrument keeps all four: heterogeneity,
per-task ordering, a task-count sweep, and a throughput metric.

It keeps none of AIM7's 53 jobs, which name Unix services this system does not have and should
not grow in order to be measured. Seven jobs stand in for AIM7's categories: a compute grind, a
32 KiB working-set walk, a null syscall, a yield burst, a `CALL`/`REPLY` round trip against a
shared server, and (since 2026-09-19) a page-mapping job and a process-creation job that build
their objects from a per-task untyped budget. `crates/job_mix`'s `BUGS` records the one category
still missing (disk-file operations) and why, and
`design/roadmap/0493-a-disk-file-job-mix-needs-a-disk-radon-can-drive.md` carries the options.

So this is not AIM7 and no number from it is comparable with an AIM7 number. It is an
AIM7-*shaped* instrument for a capability microkernel.

## Rehearsing it, which needs no board

```sh
script/job-mix                          # aarch64
script/job-mix --arch riscv64 --smp 4   # radon's architecture and core count
script/job-mix --arch x86_64            # xenon's
script/job-mix --release                # the optimization level script/board-image builds
```

Each prints the placement census, 21 `job-mix-repeat:` lines per subrun, one result line per sweep
point and seven `job-mix-kind:` lines under each. The magnitudes are fiction (TCG models no
cache) and the rehearsal exists to prove the mechanism, not to produce a result. A job the kernel
refuses (a map or spawn budget too small, say) ends the run with `job-mix: FAILED` and exit 1, so a
green rehearsal also proves the budgets fit on that architecture.

Rehearsed on 2026-09-04 (five-job mix, best of three) and again on 2026-09-19 (seven-job mix,
median of 21) on patagonia, all three architectures, sweep complete on each: aarch64 in 115
seconds, riscv64 with four harts in 188, x86_64 in 173. The riscv64 run took 20 seconds with the
old instrument the same day; seven times the repeats accounts for most of that growth.

## What changed on 2026-09-19, and how to tell an old transcript from a new one

Two things changed at once, and each on its own would make old and new numbers incomparable.

1. The statistic. Each sweep point used to be the best of three repeats. It is now the
   median of 21, with the fastest and slowest beside it. The reason is the five radon boots
   below: at `tasks=4` the best of three moved 29% between boots of one image because the spread
   *within* a boot was 37%, and a minimum drawn from a wide distribution is the statistic that
   moves most. Resampling those boots' own repeats, the median of 21 puts `tasks=4`'s boot-to-boot
   spread at about 4%, inside the band the stable points already had. `job_mix::REPEATS` carries
   the table and milestone 168's block the method. Board time was the argument for varying the
   count by point, and it does not survive arithmetic: at 21 repeats the timed windows total
   seconds on radon.
2. The mix. Seven job kinds instead of five: `MAP` (user page mapping) and `SPAWN` (process
   creation) took one slot each from the four non-IPC kinds. See `crates/job_mix`'s header.

How the lines differ, so nobody has to diff a transcript by eye:

| | before 2026-09-19 | from 2026-09-19 |
|---|---|---|
| started line | `one job is one of 5 kinds` | `one job is one of 7 kinds` |
| sampling line | (none) | `job-mix: each point is the median of 21 repeats, ...` |
| point line | `job-mix: tasks=4 jobs=512 ticks=<best> jpm=<from best>` | `job-mix: tasks=4 jobs=512 repeats=21 ticks_min= ticks_median= ticks_max= jpm_median=` |
| repeat lines | three per point | 21 per point, same format |
| per-kind lines | (none) | `job-mix-kind: tasks=4 kind=spawn jobs= ticks= per_job= region_ticks=` |

The field names changed on purpose. `jpm=` and `ticks=` do not occur on a new point line, so a
script written against an old transcript finds nothing rather than silently reading a median as a
best. `jpm_median` is not comparable with an old `jpm` even at the same task count: different
statistic, different mix.

"Finds nothing" is the safer failure and it is still a silent one, which the tree learned the
same day (2026-09-19). Another session was building `crates/board_console`'s sweep recognizer
against a capture from the old kernel. When this change landed, its parser went on matching the
line's head, read none of the four numbers, and reported zeros; its tests stayed green, because the
fixture it asserted against had been made from the same old kernel and the two agreed with each
other. The fix re-captured the fixture from a current kernel and the limitation is recorded in
`crates/board_console/src/progress.rs`'s `BUGS`: the markers are shared through `crates/job_mix`,
the field names inside the line are not.

**What a `job-mix-kind:` line says.** For one sweep point, summed over every released task and all
21 repeats: how many jobs of that kind ran, the ticks they took (self-timed by each task, preemption
included), the average per job, and for `map` and `spawn` the ticks spent inside `SPLIT` and
`DESTROY`, which take the kernel's one memory-region lock. That last column exists because the jobs
were first refused for fear they would measure the allocator rather than the scheduler; it lets a
reader see how much of them does. The kinds whose `per_job` grows fastest with `tasks` are the kernel
paths that get more expensive under load, which is the attribution a risk 4 verdict will want.

## The next bench evening on radon, start to finish

**What it is for.** A sweep whose every point is a number, with the two jobs that block deepest
in the kernel in the mix. **Its result is what `design/fatal-risks/README.md`'s risk 4 gets a
verdict from.** First run 2026-10-04.

Everything about the card, the console and U-Boot is `notes/bench-runbook.md`'s,
`notes/visionfive2.md`'s and milestone 257's (network boot); this page does not copy it.

### 0. Before power

- `lsof /dev/cu.*`: one capture per serial port. Two split the byte stream silently, which cost a
  boot on 2026-09-16 (`notes/bench-runbook.md`'s `BUGS`).
- `pgrep -l qemu` on patagonia is irrelevant to radon's numbers, but a busy TFTP server is not:
  leave the host alone while a boot fetches.

### 1. Build the image once, from a commit that has this page's 2026-09-19 section

```sh
git log -1 --format=%h                  # write this down; it goes in the Results row
script/board-image --job-mix --tftp     # network boot, the 2026-09-16 workflow (milestone 257)
# or, with the card in patagonia:
script/board-image --job-mix --card /Volumes/NIFE
```

**The archive is not optional** and a mismatched pair halts at `MEASURED BOOT REFUSED`, which cost a
boot on 2026-09-01; `--card` copies all three files as a set for that reason (milestone 217).
`--job-mix` and `--soak` are refused together: both replace the end of the boot tour.

**A job-mix card is sealed like any other.** Since milestone 563 (a seal check that reads bytes
cannot see a check that was dropped), the kernel measures `job_mix_task` against the archive's
table before the sweep. So `NOT SEALED` from `script/board-image` or `script/card-check` means what
it says: the two files are from different builds. A card built before 2026-09-25 still reads `NOT
SEALED` falsely.

### 2. Attach the console, then power on

```sh
script/board-console --for 30m --until none --log bench/radon-$(date -u +%F)/jobmix-boot1.log
```

115200 8N1, a WCH CH343 at `/dev/cu.usbmodem*` on patagonia. `--until none` because this run ends by
halting rather than by reaching a stage the console recognises (milestone 324): read the log for
`job-mix: done`. The timed windows of the seven-job sweep sum to about 12 seconds on radon
(2026-10-04), so most of a boot is U-Boot and the fetch.

### 3. Check the boot is the new instrument before reading any number

The started line must say **`one of 7 kinds`** and the next line **`median of 21 repeats`**. If
either is missing the image is older than this page and the boot is not comparable with anything
this evening is for. Then read the census (`job-mix-census:` lines), which says where the 34
threads landed.

### 4. Boots: at least five, power-cycling between

Five is the floor because it is what 2026-09-16 used and the two evenings should be comparable in
design. Nothing can power-cycle radon remotely (milestone 224); this is a person at the bench.

A jobs-per-minute figure from a single boot is a draw rather than a result: on radon placement alone moves throughput by up to fifteenfold (`notes/soak.md`).

### 5. Clean each log before committing it

```sh
LC_ALL=C tr -cd '\11\12\15\40-\176' < raw.log > bench/radon-<date>/jobmix-bootN.log
```

The console writes bytes that are not UTF-8 under sustained output and `grep` then silently reports
nothing (`notes/bench-runbook.md`'s `BUGS`). Read with `LC_ALL=C grep -a` until they are cleaned.

### 6. What to read, in this order

```sh
cd bench/radon-<date>
grep -ah '^job-mix: tasks=' jobmix-boot*.log            # six points per boot
grep -ah '^job-mix-kind: tasks=32 ' jobmix-boot*.log    # which kinds slowed at the top
grep -ah '^job-mix-kind: tasks=1 ' jobmix-boot1.log     # and what they cost alone
```

1. Is every point a number now? For each `tasks`, compare `jpm_median` across the boots. The
   bar this change was built to clear is **under 10% boot-to-boot at every point**, `tasks=4`
   included (the resampling predicts about 4%). If `tasks=4` is still wide, the median did not fix
   it and the reason is new: record the spread and do not quote the point.
2. Is `ticks_max` far from `ticks_median` on one boot only? One outlier boot is the placement
   lottery (`notes/soak.md`, milestone 240); compare its census with the others'.
3. The shape. Throughput against task count, from the medians. Record where it stops rising,
   and whether anything past that point **declines**. On 2026-09-16, with the old mix, it rose to
   about 4 tasks and then plateaued without declining through 32.
4. **Which kinds pay.** For each kind, `per_job` at `tasks=32` over `per_job` at `tasks=1`. Kinds
   that never enter the kernel (`compute`, `touch`) grow only by preemption; a kernel-entering kind
   growing much faster than they do is kernel cost under load. For `map` and `spawn`, subtract
   `region_ticks` to separate the region lock from the rest.

### 7. What the answer means for risk 4

| What the medians show | Reading for `design/fatal-risks/README.md` risk 4 |
|---|---|
| rises to the core count, then flat or rising through 32, every point within 10% across boots | **no architectural per-crossing cost visible at this scale on this silicon**; the risk's decisive experiment ran and the defense held, within the caveats in the Warton section above |
| a knee followed by a **decline**, repeatable across boots | a cost exists and grows with load; the `job-mix-kind:` lines say which path. Milestone 188 (the IPC fastpath) is the follow-on if it is `round_trip` |
| points still wider than 10% across boots | not a verdict; record the spread and say which point failed |

**Who writes the verdict.** The risk's entry in `design/fatal-risks/README.md` is edited from these numbers
by whoever holds that file, not by a lane and not from QEMU.

### 8. Record it

A row in this page's Results table, the row in `notes/register-of-measures.md`, and milestone 168's
block: date, boots, commit, the six medians from one representative boot plus the boot-to-boot
spread per point, and the census summary.

## What each outcome means

| What the log shows | What it means | Where it routes |
|---|---|---|
| `job-mix: FAILED: no 'job_mix_task' program in the initrd archive` | an archive built before this milestone (a mismatched pair halts earlier, at `MEASURED BOOT REFUSED`) | rebuild with `script/board-image --job-mix ...`, which packs the archive before the kernel for exactly this reason |
| `job-mix: FAILED: could not spawn task N of 32` | the board ran out of memory or thread slots partway through building the pool | a real finding: `job_mix::MAX_TASKS` is 32 against `sched::MAX_THREADS`'s 256, so this is memory. Record N and reduce `MAX_TASKS` |
| `job-mix: FAILED: could not create task N's 25-page budget` | the kernel could not carve a task's untyped region | memory again, before any subrun; the same routing as the line above |
| `job-mix: FAILED: task N could not finish a map job` (or `spawn`) | the kernel refused a verb inside the job; the error is printed | a budget constant is too small for this machine (`job_mix::MAP_REGION_PAGES`, `CHILD_PAGES`), since QEMU passes on all three architectures. Record the error and raise the constant |
| the census, then nothing, ever | a task, a server or a child wedged before the first subrun finished | the hang case, and since milestone 324 the tool says so rather than leaving it to the operator: `script/job-mix` and `script/board-console --until sweep-done` both exit **2**. `ROUND_TRIP` blocks on a server and `SPAWN` on a child; either wedged looks exactly like this |
| `jpm_median` roughly flat across the whole sweep | this machine's scheduling is not the bottleneck at 32 tasks | **the honest negative**, and it is a result: see step 7 |
| `jpm_median` rising and then falling, with a knee | throughput collapsing under task count | the positive result. Record where the knee is, which `job-mix-kind:` lines grew, and compare against milestone 134's E1 knee (8 to 11% by 64 to 96 threads on the dev Mac) |
| `jpm_median` varying more between boots than across the sweep | the placement lottery dominates | not a result about §96 at all. More boots, and read `notes/soak.md`'s milestone 240 section |
| `job-mix: done` and six clean points | the sweep ran | exit **0**, and step 8 |

## Results

| Date | Machine | Boots | Instrument | Census summary | jpm at 1 / 32 | Notes |
|---|---|---|---|---|---|---|
| 2026-09-16 | radon (4 harts, 4 MHz `rdtime`) | 5 | five-job mix, **best of 3** | 32 tasks and 2 servers over 4 cores, 6 to 10 threads a core | 319,013 to 319,072 / 1,032,586 to 1,060,264 | shape solid (1.96x at 2 tasks, plateau past 8, no decline at 32); `tasks=4` spread 29.4% across boots, so **not a number**. Transcripts `bench/radon-2026-09-16/jobmix-boot*.log`; the per-point table is milestone 168's block |
| 2026-10-04 | radon, `04a8f9e6c` | 5 | seven-job mix, median of 21 | 34 threads over 4 cores; boot 5 placed differently, read the same | 323,951 to 324,033 / 897,375 to 901,122 | every point a number (widest spread 1.67%, `tasks=4` 1.12%); 2.62x at 4 tasks, rising to 2.78x at 32, no decline. Step 7's first row. Transcripts `bench/radon-2026-10-04/jobmix-boot*.log` |

The 2026-10-04 evening, read in step 6's order with its step 7 judgment, is
[its own page](job-mix/radon-2026-10-04.md). It checked this page's prediction that the median would
read about 1.73x at two tasks where the best of three had read 1.96x: it read 1.76x.

## A cross-check on the Apple cores, attempted 2026-09-19, and why it did not run

**This was never going to be a result, and in the event it produced no numbers at all.** The idea
(calef's, 2026-09-19) was to run the seven-job sweep under QEMU with Apple's Hypervisor.framework on
patagonia, where four vCPUs run on four real cores at once, as a cross-check on the curve's *shape*
and on whether the median holds `tasks=4` still. HVF timing cannot decide anything for §96 or risk
4, because macOS schedules the vCPUs underneath the guest; it could only say whether the shape
appears on real cores other than radon's.

`script/job-mix` gained `--hvf` and `--release` for it (the spellings `cargo xtask run` and
`cargo xtask bench` already use). Five attempts, each started only when no other QEMU was running
on the host, were all refused before the kernel ran:

```text
qemu-system-aarch64: HVF does not support GICv2 emulation
```

QEMU 11.1.1 on this host refuses HVF with a GICv2, and `kernel/src/drivers/gic.rs` speaks only
GICv2 (`notes/hvf-leg.md`, `notes/interrupts.md`'s `BUGS`). Milestone 227 (GICv3) fixed that;
the spawn job then fails (2026-10-05, [BUGS](job-mix/null-syscall-under-load.md#bugs)). The transcript is
`bench/patagonia-hvf-2026-09-19/jobmix-hvf-refused.log`. The flags stay, because the command that
will take the cross-check once 227 lands is then already written:

```sh
script/job-mix --hvf --release --smp 4    # alone on the host: check `pgrep -l qemu` first
```

## What this instrument cannot settle, said plainly

**It does not compare a process kernel against an event kernel, and it cannot**, because there is no
event kernel here to compare against. It measures what *this* kernel does under multi-tasking load.
That is the input §96 says it is missing, and §96 stays open either way: a flat curve says the cost
is not visible at this scale on this silicon, which is a reason not to spend a pervasive rewrite; a
knee says there is something to price, and pricing it would then want the rewrite prototyped on one
path rather than the whole kernel.

## BUGS

- The sweep has no wall-clock heartbeat, so a watcher's wedge timer is a guess with headroom.
  This entry replaces *"`crates/board_console` has no recognizer for this run"*, which was true when
  this page was written and stopped being true on **2026-09-19** (milestone 324 part 2): the
  recognizer has `Stage::Sweep` and `Stage::SweepDone`, reading `crates/job_mix`'s own marker
  constants, and `script/job-mix` judges with it and returns `script/board-console`'s five exit
  statuses. What that milestone could not fix is the thing that makes a sweep harder to watch than a
  soak. `kernel/src/soak.rs` prints every five seconds whatever the workload is doing, so a missed
  beat is a missed deadline; `kernel/src/job_mix.rs` prints only when a subrun ends, so the longest
  legitimate silence is the slowest subrun and a watcher has to allow for it. The default is sixty
  seconds against a 4.0-second subrun measured under TCG, fifteen to one, and a board outside that
  margin reads as wedged when it is merely slow. `--quiet-after 0` is the escape and it gives up the
  detection. A heartbeat in the supervisor is the real fix and is a kernel change; milestone 324's
  block records it as follow-on.
- The trap still slows with busy cores, a little. On radon `null_syscall` grew 94 ticks a job from
  one task to four, then 48 after the reaper fix, then 10 after milestones 761 and 766. Its one-task
  level moves up to 8 ticks with code placement alone.
  [`job-mix/null-syscall-under-load.md`](job-mix/null-syscall-under-load.md) has every evening.
- There is no committed baseline and no `--check`. `script/bench` gates because its icount counts
  are deterministic; a sweep whose entire subject is scheduling under contention is not, on any
  accelerator this tree has. A gate here would be asserting a tolerance nobody has measured.
- The sweep tops out at 32 tasks, where milestone 134's E1 reaches 96 threads. E1's were kernel
  threads; these are processes, each with an address space and a loaded image, and the memory is
  what binds. **If the knee is above 32 this instrument will not see it**, and the honest response is
  to record that rather than to raise the number blind: a `FAILED: could not spawn` line is the
  measurement that says how far it can go on a given board.
- The pool is built once and released in slices, where AIM7 forks a fresh set per subrun, so the
  parked tasks' kernel stacks exist during every subrun even though nothing touches them.
  `kernel/src/job_mix.rs`'s own header argues why that is the right choice for §96's question and why
  it is still a departure from AIM7.
- The mix proportions are chosen, not derived. AIM7 ships workfiles for four machine roles and
  nobody here has one for a capability microkernel. A different mix gives a different number, and no
  result from this instrument should be quoted without saying which mix produced it.
- The repeat count was sized from the five-job mix's distribution, because no seven-job run
  on silicon exists yet. The map and spawn jobs may widen or narrow `tasks=4`'s spread. Step 6.1
  of the procedure is the check, and if 21 is not enough the count is one constant
  (`job_mix::REPEATS`).
- Since milestone 227 (a GICv3 driver) `--hvf` boots, and since 2026-10-05 the full mix completes
  there ([`job-mix/spawn-destroy-gone.md`](job-mix/spawn-destroy-gone.md)).
- The supervisor is in the kernel, and that is what makes the build special (milestone 523 (moving the job-mix supervisor into userspace, and the five permissions it turns out to need)). A userspace supervisor would delete the whole
  class of problem a diverted boot has, and would measure the more honest thing: its
  own releases and drains would be real `svc` round trips inside the timed window, and it would be
  one more process the scheduler has to place, which is precisely what DECISIONS §96 (process
  kernel or event kernel) asks the cost of. It is blocked on five decisions that are an architect's,
  and milestone 523's block prices each one. **If it is ever built, the Results table below needs a
  column saying which supervisor produced a row**: same mix, same statistic, same field names,
  different timed window, so nothing in a transcript would look different.
- **This page has not been followed end to end by its author**, the same caveat
  `notes/bench-runbook.md` carries about every procedure it points at.
