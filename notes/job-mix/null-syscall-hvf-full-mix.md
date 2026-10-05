# The null syscall under the full mix, on four Apple cores

*An appendix of [the null-syscall note](null-syscall-under-load.md), measured 2026-10-05 (UTC) on
patagonia by `lane/null-syscall-hvf`. Patagonia is an Apple M3: four performance cores, four
efficiency cores. The file name is provisional (see the README).*

[The off-radon appendix](null-syscall-off-radon.md) found `null_syscall` flat on four real cores.
That was with milestone 761 (capability lookup off the global lock) in, and with the spawn job
stubbed, because the full mix failed under HVF. #1663 (`lane/hvf-spawn-destroy-gone`) traced that
failure to a miscompiled `yield_now` in userspace. With its fix the full mix completes. So nobody
has yet measured, on real cores, the arrangement radon ran: every job kind, spawning included.

## The questions, written before any run

The excess is the earlier notes' quantity. Take `null_syscall`'s per-job ticks at N tasks. Subtract
its one-task figure scaled by `compute`'s growth between the same points. Divide by 64 traps.
`compute` makes no syscalls, so its growth is what preemption and the host charge a job by its
length. The excess is what is charged per trap.

"Rises" means two things hold. The median excess at four tasks is above 0.05 ticks a trap, under
half the 0.11 the lock ablation showed here. And the four-task median is above every one-task boot.
"Flat" means neither holds.

| Question | Condition | Reading |
|---|---|---|
| Q1. Does `null_syscall` rise from one task to four, spawning included? | A: the full mix | Flat: the rise radon saw is gone on real cores, and radon owes only its size. Rises: the stub hid something spawning brings |
| Q2. If it rises, is the cost in the kernel or outside it? | B: A with `--lock-wait`. C: task 0 runs the full mix; tasks 1 to 3 run only `compute`, long enough to outlast it | C flat, A rising: the other cores' kernel work causes it. B then says how much is a lock (excess equal to wait). C rising like A: load with no syscalls causes it, so it is the host or the hardware, not the kernel |
| Q3. Does it scale with loaded cores? | A at 1, 2, 3 and 4 tasks | Growing with each core: queueing on something shared. A step at two: line traffic or interrupts. Only at four: the host is out of performance cores |

Patagonia has four performance cores, and QEMU's own threads want one. That is why Q3's last reading
is a live possibility here and would not transfer to radon.

Every condition runs at least ten boots of `script/job-mix --hvf --release --smp 4`. The tree is
`main` (`b3b9f92f0`) with #1663 merged (`5bd8c1e68`, not yet on `main`). One scratch change applies
throughout: the sweep is 1, 2, 3, 4 tasks instead of 1, 2, 4, 8, 16, 32. Condition C adds a second
scratch patch to the task. Neither is committed.

## What was added after the first runs

Three conditions joined once data existed, and this section says so rather than let them look
planned.

- P, the lock put back, a positive control. The earlier lane's ablation no longer boots: holding
  `IPC_TABLES` across the lookup now breaks the lock order on the map path. So P waits for
  `IPC_TABLES` on the invoke path and drops it before the lookup.
- D, spawn stubbed, the earlier lane's arrangement, so the two notes meet.
- E, `PerCpu` on its own line. Added after A showed a small residual that D did not.
  `#[repr(align(128))]` on aarch64; 128 bytes is Apple's line, and the block stays 128 bytes.

Every scratch diff is in [the patch file](null-syscall-hvf-full-mix.scratch.patch). Conditions were
interleaved boot by boot. Raw rows are in the `null-syscall-hvf-full-mix.*.csv` files beside this
note, one per condition. Twenty-three boots overlapped another lane's x86_64 QEMU. Their rows carry
`foreign_qemu=1`, and the tables below leave them out.

## The results

Ticks of the 24 MHz counter per 64-trap job. Medians, range in brackets. Excess is ticks per trap,
with a bootstrap 95% interval on the median.

| Condition | boots | 1 task | 2 | 3 | 4 | excess at 2 | at 3 | at 4 |
|---|---|---|---|---|---|---|---|---|
| A, full mix | 30 | 50.8 [50.0-61.8] | 52.4 [50.8-67.7] | 55.7 [54.2-69.2] | 59.5 [56.8-70.9] | 0.009 [-0.007, 0.017] | 0.021 [0.009, 0.030] | 0.041 [0.023, 0.058] |
| D, spawn stubbed | 30 | 49.8 [49.0-63.2] | 49.6 [46.7-60.3] | 49.8 [47.5-54.8] | 50.3 [46.8-64.7] | -0.000 | -0.000 | 0.002 [-0.006, 0.008] |
| C, quiet load | 20 | 51.9 [50.1-70.2] | 51.5 [50.3-55.2] | 51.0 [49.9-71.2] | 51.7 [50.0-53.7] | -0.012 | -0.024 | -0.004 [-0.012, 0.003] |
| B, lock-wait build | 20 | 55.8 [53.4-103.8] | 55.6 [53.7-83.0] | 58.0 [56.7-102.9] | 59.9 [57.8-104.7] | -0.019 | -0.025 | -0.010 [-0.054, 0.029] |
| P, lock back | 20 | 57.1 [55.1-58.8] | 76.4 [70.8-206.8] | 93.4 [87.7-171.5] | 117.1 [109.3-223.8] | 0.271 | 0.480 | 0.849 [0.816, 1.447] |
| A, second batch | 19 | 50.7 [50.0-142.5] | 52.6 [50.9-82.3] | 55.8 [54.3-66.4] | 60.7 [56.5-73.9] | 0.009 | 0.018 | 0.042 [0.018, 0.066] |
| E, `PerCpu` aligned | 18 | 50.6 [49.9-66.4] | 51.5 [50.5-65.5] | 53.9 [52.2-57.6] | 56.3 [55.2-57.8] | -0.002 | -0.011 | -0.010 [-0.019, -0.006] |

`compute` per job, A: 662, 672, 706, 740. D: 660, 654, 654, 655. E: 663, 673, 704, 748. In C, task
0's own `compute` read 653, 666, 670, 652. The last two rows were a second interleaved batch, A
against E.

B's lock instrument found `current_cap` contended on 0.012% of 476,000 lookups at four tasks, a
wait of 0.0001 ticks a call. `IPC_TABLES` is still the most contended lock in the mix: 63,000
contended acquisitions over 21 subruns, by IPC and scheduling. The null syscall no longer takes it.

## What it answers

**Q1: almost flat, and the rest is a defect.** At four tasks the median excess is 0.041 ticks a
trap, about 1.7 ns on a 33 ns trap. That is under the 0.05 written above, and the four-task median
(59.5) is under the slowest one-task boot (61.8). So by the rule written first, it does not rise.
But the interval excludes zero, and D, without spawning, reads 0.002. Radon's original excess was
1.4 ticks of a 10 MHz clock, 140 ns a trap. Putting the lock back (P) costs 35 ns here.

**Q2: inside the kernel, and it is not a lock.** C is flat: three cores busy in syscall-free code do
not slow task 0's traps. B finds no wait on the lookup. E removes the residual: with each core's
block on its own 128-byte line the excess is -0.010 [-0.019, -0.006], against 0.042 in the A boots
interleaved with it. That is the false sharing [the parent note's BUGS](null-syscall-under-load.md#bugs)
predicted and left unmeasured. Spawning is what brings it. Which remotely written field does it
(the parent note names the inbox and the steal slot) is not measured. B's build puts `PERCPU` at a different offset (0 mod 64 rather than 24),
which fits its showing no residual.

**Q3: it grows with each core, but only with spawning in.** A's excess goes 0.009, 0.021, 0.041.
That is one more remote writer per added core. P shows what queueing on a lock looks like on this
machine: 0.27, 0.48, 0.85. The host running out of performance cores, Q3's third reading, does not
appear. C and D are flat at four tasks.

**The raw rise is a different thing.** A's `null_syscall` rises 17% from one task to four, and
`compute` rises 12% with it. That part is charged by job length, so it is not a per-crossing cost.
It also comes with spawning (D's `compute` is flat), and it survives E.

## What transfers to radon

HVF runs the guest on real cores, with real caches and coherence, so a shared line costs what it
costs on an M3. What does not transfer: the host schedules the vCPUs, a cross-core interrupt leaves
the guest, and the M3's line is 128 bytes where radon's U74 has 64. Radon's `PERCPU` sits at 16 mod
64 (the parent note), so it straddles there too. The size on radon is unknown. The direction is the
same, and the fix the parent note proposed (`align(64)`) would not be enough on Apple silicon.

The earlier lane's finding stands and now has spawning in it. Without the global lock the null
syscall's growth under load is a cache line, about 5% of a trap at four tasks on this machine.

## What risk 4's line should say

For the maintainer. The colour is calef's. This lane would leave it AMBER until radon sizes 761,
and propose this sentence after the one ending "half explained and half open":

> 2026-10-05 (UTC): on four Apple cores under HVF, with 761 in and the full mix running (spawning
> included, after #1663), the null syscall's per-trap growth from one busy core to four is 1.7 ns
> on a 33 ns trap, against 35 ns with the global lock put back. The remainder is false sharing
> between per-core blocks, not a lock: aligning each core's block to its own 128-byte line removes
> it, and three cores busy in syscall-free code do not cause it. Nothing measured here is a cost
> the capability model forces; radon owes the size
> (notes/job-mix/null-syscall-hvf-full-mix.md).

## BUGS

- One machine, one host. Patagonia ran its usual load (load average 2 to 7) and the batches
  differ. A's four-task excess read 0.024 in its first twenty boots and 0.066 in the next twenty,
  where throughput also fell from 10.1 to 6.8 million jobs a minute. Ten of those forty boots
  overlapped a foreign QEMU and are left out. D did not move between batches.
- C is not the same load minus syscalls. In C task 0's spawned children wait for busy cores, so
  at two and three tasks its spawn job takes 108,000 to 167,000 ticks against A's 2,700. And the
  other cores do no kernel work at all. C tests syscall-free load, not A's load minus its syscalls.
- The spawn job is about twenty times slower at one task than on radon. 59,000 ticks a job at
  24 MHz (2.5 ms) against radon's 1,172 at 10 MHz (117 us). It falls to 2,700 at four tasks. That
  makes one-task throughput here meaningless. This lane first guessed that a child placed on a busy
  core waits for its 10 ms tick. The code says otherwise: the reschedule SGI drains the inbox and
  preempts. So the wait is most likely SGI delivery under HVF
  ([the proposal](../../design/roadmap/proposals/cross-core-wake-latency-under-hvf.md)).
- The excess is near the instrument's floor. One boot's excess at four tasks scatters by about
  0.1 ticks. The medians of twenty to thirty boots carry the result, and the intervals say how far.

## Proposed work

- [Each core's `PerCpu` on its own cache line](../../design/roadmap/proposals/each-cores-percpu-on-its-own-cache-line.md)
  (provisional). `align(128)` on aarch64 and riscv64, which supersedes the parent note's
  `align(64)`. Its acceptance measurement is radon's run.
- [Cross-core wake latency under HVF](../../design/roadmap/proposals/cross-core-wake-latency-under-hvf.md)
  (provisional): where the spawn job's milliseconds go when another core is idle or busy.
