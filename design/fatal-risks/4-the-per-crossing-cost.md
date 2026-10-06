---
experiment_status: RUN
experiment_run: 2026-10-04
---
# 4. The architecture imposes a per-crossing cost that cannot be engineered away

*Risk 4 of [the nine](README.md). The status vocabulary, the rule an entry meets and the running order are there.*

The claim, and calef named this one first: a capability microkernel pays on every boundary crossing,
and on workloads that cross constantly the cost is architectural rather than a matter of tuning.

GREEN (calef, 2026-10-05); amber 2026-10-04, #1613. The throughput defence
held on step 7's first outcome. The one number not fully explained was a per-crossing cost under load, the
null syscall going from 108 to 202 ticks between one task and four, and that is exactly this risk's
claim. What turned it green was explaining that
slowdown and showing it is a fixable defect, lock contention on `IPC_TABLES` and false sharing between
cores' per-core blocks, rather than an architectural cost. Both are fixed and the growth is 10 ticks. Everything measured before it is a single crossing, and the claim is about a cost that cannot be amortised.
Amortisation is a property of a workload. The single-crossing numbers are four wins and a tie against
Linux on the same core, every caveat beside its number
([`notes/benchmarks.md`](../../notes/benchmarks.md)), over committed floors
([`bench/baseline-aarch64.txt`](../../bench/baseline-aarch64.txt)).

**The decisive experiment:** milestone 168 (a multi-tasking workload benchmark), run by
[`notes/job-mix.md`](../../notes/job-mix.md)'s procedure, whose step 7 wrote down what each outcome
means before the numbers existed.

2026-10-04 (UTC). Five sealed radon boots of the seven-job, median-of-21 instrument
([the evening's page](../../notes/job-mix/radon-2026-10-04.md)). Every point agreed within 1.67%
across boots. Throughput rose to 2.62x at four tasks, the core count, and to 2.78x at 32, with no
decline on any boot. That matches step 7's first outcome, which reads: "no architectural
per-crossing cost visible at this scale on this silicon; the risk's decisive experiment ran and the
defence held". So the defence held on this experiment. That is not the same as the risk being
retired, and seven things bound it:

- One machine, radon, with four harts.
- No disk-file job in the mix.
- Nothing above 32 tasks, so a knee beyond that would not be seen.
- One kernel model, so there is no ratio to set against Warton's 20%.
- At 32 tasks, 80% of task time is `round_trip` queued on the instrument's two echo servers. That
  shapes the plateau: a curve held flat by a server bottleneck is a weaker witness than one held
  flat by spare cores.
- About 5 ticks (about 4%) of the growth from one task to four is not yet decomposed. It is smaller
  than the 8-tick code-placement swing, so it cannot be told from noise on one build.
- Every radon number used debug userspace. That moves the level, not the growth
  ([`release-userspace-in-board-images`](../roadmap/proposals/release-userspace-in-board-images.md)).

One finding was open and is now explained. The null syscall's near-doubling from one busy core
to four was half a defect and half contention. The defect:
the reaper held the global `IPC_TABLES` lock while freeing a dead thread's kernel stack, six
TLB shootdowns that interrupt every core. Fixing it on radon (2026-10-04) cut the null syscall's
growth from one task to four from 94 ticks to 48, and raised throughput 9% at four tasks and 11% at
32. The rest was the same lock on every capability lookup, which milestone 761 removed, and false
sharing between cores' per-core blocks, which milestone 766 (each core's PerCpu on its own cache
line) removed. On radon (2026-10-05) the
null syscall's growth from one task to four fell from 48 ticks to 10, half of it the preemption
every job pays; undoing 766 alone puts back 7.5. One caveat travels with every single-crossing
number on radon: moving the kernel's text by a few bytes, with no code change, moves the null
syscall by up to 8 ticks (7%), so builds are compared on growth, not level
(notes/job-mix/null-syscall-under-load.md).

Measured 2026-10-05 (UTC), recorded by `lane/radon-2026-10-05-record`; calef read it and ruled this risk green on 2026-10-05.
Radon ran milestone 761 (capability lookup off the global lock) and milestone 766 (each core's
PerCpu on its own cache line). The null syscall's growth from one task to four fell from 48 ticks to
10, and `current_cap` found its lock held on 0.05% of calls. The one-task level moved up to 8 ticks
with code placement alone, so builds are compared on growth. The replacement for the open finding above is the sentence that page proposed
(`notes/job-mix/null-syscall-under-load.md`, "What risk 4's line should say"), applied as written.

Two caveats. The counter-thesis is published: the crossing can be removed rather than made cheap. If
RedLeaf and the 2017 Rust-kernel paper are right, a capability crossing is a cost this project chose
rather than inherited, and their open problem is risk 5. And `sel4bench` has never produced a number,
so the peer is Linux rather than the state of the art in minimal kernels. It ranks sixth although a
skeptic expects the project to die here, because this is where the most evidence says it will not.
[Appendix](the-crossing-cost.md).
