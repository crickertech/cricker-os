# Why the null syscall slowed when more cores were busy

*Fatal risk 4's open finding from [the 2026-10-04 radon evening](radon-2026-10-04.md), measured on
2026-10-04 (UTC) by the `lane/null-syscall-contention` lane. The cause and the fix were found under
QEMU's TCG, which models no cache and no real inter-processor interrupt; radon then ran the fix the
same night, and [its result](#what-radon-said) is PARTIAL by the thresholds written before it.*

## The question

On radon the job mix's `null_syscall` job (64 traps that each bounce off an empty capability slot)
cost 108 ticks at one task and 202 at four, while `compute` grew 6%. Past four it grew only 17% more
by 32 tasks. Why does the cheapest syscall nearly double when more cores are busy?

`per_job` is self-timed wall clock, so a task descheduled mid-job charges the wait to that job. But
anything that arrives at a uniform rate in time (a timer tick, an IPI, a remote TLB fence) lands on a
job in proportion to its length, so it charges `null_syscall` the same percentage it charges
`compute`. The 6% is that baseline, and the excess, about 1.4 ticks (0.35 us) per trap, has to be
paid per trap.

## What a null syscall touches that another core also writes

Read on all three architectures, from trap entry to return:

| What | Shared with other cores? |
|---|---|
| `arch::exceptions::SVC_COUNT.fetch_add` | Yes, one line every syscall on every core wrote. Read by the system tests, and by a riscv64 tour line that printed 0 on every captured boot |
| `IPC_TABLES.lock()` in `sched::current_cap` | Yes, the kernel's one global lock, taken by every IPC, every `schedule()` and every capability operation |
| `cpu::current()`, `held_rank` | Per core. But `PERCPU` is aligned to 8, at offset 16 mod 64 on riscv64 and 24 on aarch64, so neighbouring cores' blocks share a line (not fixed; see BUGS) |

The trap entry and exit assembly touch nothing shared, and a syscall never reschedules on its way
out (only an interrupt asks for `preempt_if_needed`). So the candidates were one shared counter and
one global lock.

## What TCG said

riscv64, `script/job-mix --arch riscv64 --smp 4 --release`, ticks of the 10 MHz timer. One run per
row unless stated; two runs of the first row agreed within 2.5%.

| Kernel | `null_syscall` 1 / 4 tasks | `compute` 1 / 4 tasks |
|---|---|---|
| `main` (two runs) | 638 / 1,250 and 654 / 1,253 | 314 / 412 and 323 / 416 |
| `SVC_COUNT` increment removed | 657 / 1,203 | 328 / 428 |
| ...and `current_cap` reading without the lock (a racy experiment, never committed) | 564 / 857 | 327 / 439 |

Taking the lock out of the path took away most of the excess; the counter was a small part of it.
So the lock got instrumented. `--features lock_wait` (kernel/src/lock_wait.rs) counts, during the
timed windows only, every acquisition that found a lock held and how long it spun, and every
`current_cap` call:

| `current_cap` at 4 tasks | calls | found `IPC_TABLES` held | wait per call |
|---|---|---|---|
| before the fix | 478,659 | 19.5% | 9.94 ticks |
| at 1 task, for scale | 119,659 | 6 calls | 0.008 ticks |

9.94 ticks a call is 636 ticks per 64-trap job, against a measured growth of 589 from one task to
four in the same run. Waiting for `IPC_TABLES` was the whole of it, under TCG.

### Who held it

A scratch hold-time table keyed by `#[track_caller]` call site (QEMU only, never committed) put
`finish_switch` far ahead of everything else. Timing inside it found where: of 42.2 million ticks of
`finish_switch` hold at four tasks, 41.2 million were inside `threads.remove(prev)`, on 2,688
calls. That is one per child the `spawn` job reaps, at 0.3 to 1.5 ms each under TCG across two
runs.

`remove` drops the `Thread`, and the `Thread` owns its kernel stack. Freeing the stack is six page
unmaps, and each one discharges its TLB obligation on every other hart. On riscv64 that is an SBI
remote fence, which interrupts every hart and waits for all of them; on x86_64 an NMI round.
All of it ran with the global lock held and interrupts masked. A thread exiting anywhere stalled every
IPC and every capability lookup on every core. The shape fits the radon finding. At one task nothing
else exits while the `null_syscall` job runs. At four, other tasks' `spawn` jobs reap children
throughout. Past four, the number of cores that can be waiting stops growing.

`START` had the mirror image: it built the new thread's stack (six pages mapped) under the same lock,
at about 380 ticks a start under TCG, the second largest holder at 32 tasks once the reaper was
fixed.

## The fix

Each change has its reason in its commit message:

- **The shared counter is compiled into the system tests only.** They are its one real reader. The
  soak's one read discarded its result, and the riscv64 tour line it fed printed 0 on every captured
  boot because it raced the program it counted. The first commit said aarch64 and x86_64 never read
  it; the system tests do, which `script/lint` caught, and the correction is its own commit.
- **No kernel stack is built or freed under `IPC_TABLES`.** The reaper takes the stack and the
  address space out under the lock, marks the thread `being_reaped`, frees both with no lock held,
  then takes the lock again to remove the thread. `START` builds the stack before taking the lock.
- **The ordering is load-bearing, and was learned by breaking it.** The first version removed the
  thread first and freed the stack after. The `spawn` job then saw `DESTROY` succeed and built its
  next child before the dead one's stack was back, and the sweep failed with `OutOfMemory` and
  `BadPointer` within a subrun, three runs out of three. So the thread stays in the table, `Finished`,
  while its stack is freed, and region teardown refuses it passively, exactly as it refuses a thread
  still standing on its stack (`region_reap_verdict`'s `standing`).
- **The address space goes before the thread too.** The first version still dropped the space just
  after removing the thread, as the old code always had. Under TCG on x86_64 the `lock_wait` build
  then failed 2 sweeps in 10 (`OutOfMemory` in `spawn`, `BadPointer` in `map`) where `main` failed 0
  in 10. The window was old: `reclaim_region` assumes a bound space "died with its thread", and a
  `DESTROY` on another core could reclaim the region between the removal and the space's teardown.
  Dropping the space in step 2 closed it: 8 x86_64 and 2 riscv64 `lock_wait` sweeps, all clean.

After the fix, same instrument, same machine:

| At 4 tasks | before | after |
|---|---|---|
| `current_cap` wait per call | 9.94 ticks | 3.02 ticks (2.65 in a run with another lane's QEMU on the host) |
| all `IPC_TABLES` waits | 18.9 M ticks | 4.9 M ticks |
| reaper's hold, per reap | 2,800 to 15,000 ticks (two runs) | 12 ticks; the stack's 2,200 is now outside any lock |

What is left holding the lock is the IPC path itself: `schedule`, `CALL`, `REPLY`, `current_cap`,
each a few ticks a hold, roughly evenly. That is the contention
[`ipc-tables-lock-inventory.md`](../ipc-tables-lock-inventory.md) predicted, and its "free win"
(capability tables off the global lock) is the next step, not this one.

### What TCG cannot say, plainly

- **The `null_syscall` job itself shrank less than its lock wait did under TCG.** After the fix it
  read 671 / 1,125 ticks at one and four tasks: the growth fell from about 600 to about 450, where
  the wait alone fell by about 440 ticks a job. Under TCG a remote fence stops every vCPU whether or
  not anyone holds a lock, and TCG's own costs rise with busy vCPUs, so the rest is not evidence about
  radon either way. The host was also loaded by other lanes for part of the session, and a later run
  read `compute` at one task as 555 rather than 330; it is not used.
- x86_64's TCG throughput roughly doubled (`jpm_median` 293,839 to 561,875 at four tasks, 316,469
  to 751,072 at 32, one run each), because TCG's NMI shootdowns are very expensive. That is a
  property of the emulator, not a claim about xenon.
- aarch64's job mix wedges under TCG on `main` too, before and after this change (see BUGS), so
  it gave no comparison.

## The radon run that decided it

Two payloads, built from this branch's tip, so no build happens at the bench:

```sh
cd ~/projects/nife-worktrees/null-syscall-contention
script/board-netboot --root target/board-fix          # boots 1 to 3: the kernel that ships
script/board-netboot --root target/board-lock-wait    # boots 4 and 5: the same plus lock_wait
```

Power-cycle smart plug 2 for each boot, and capture each boot with step 2 of
[`notes/job-mix.md`](../job-mix.md)'s procedure. Never smart plug 3, never the USB hub. Steps 3 and 5
apply unchanged. Then read, in this order:

1. `null_syscall`'s `per_job` at 1 and 4 tasks on boots 1 to 3, against 108 and 202 from 2026-10-04
   (every boot then read 195 to 207 at four).
2. On boots 4 and 5, `job-mix-lock: ... site=current_cap`: `wait_ticks / calls` at 4 tasks, in ticks
   per trap, against the job's per-trap growth `(per_job(4) - per_job(1)) / 64`.
3. `site=reap`: `stack_free_ticks / reaps` is what used to be held under the lock per reap, on
   silicon, for the first time.

What each outcome means, written before the numbers exist:

| Boots 1 to 3 at 4 tasks | Reading |
|---|---|
| 130 or below | the excess was this defect; the remainder is in line with `compute`'s preemption share |
| 130 to 180 | the defect was part of it; boots 4 and 5's residual wait says how much of the rest is the IPC path's own contention |
| 180 or above | this was not radon's cause. Boots 4 and 5's `current_cap` wait says whether it is still the lock. If that wait is small, suspect the line traffic the counters cannot see (BUGS) |

## What radon said

Five boots on 2026-10-04 (UTC), power-cycled by calef: three of `target/board-fix` and two of
`target/board-lock-wait`, built from `b500b3d48`. Transcripts are `bench/radon-2026-10-04-fix/`.
Every boot printed `one of 7 kinds`, `median of 21 repeats` and `job-mix: done`.

`per_job` on the three boots of the kernel that ships, in 4 MHz ticks, against the 2026-10-04 sweep
before the fix:

| | 1 task | 4 tasks | 32 tasks | growth 1 to 4 |
|---|---|---|---|---|
| `null_syscall`, before | 108 | 202 (195 to 207) | 236 | 94 |
| `null_syscall`, after | 99, 99, 99 | 147, 146, 147 | 171, 170, 170 | 48 |
| `compute`, after | 2,409 | 2,547 to 2,552 | 2,898 to 2,917 | 5.8% |

**The fix removed about half the excess, so the reading is the middle row: PARTIAL.** At four
tasks the job fell from 202 to 147 ticks. Nine of those 55 ticks are the one-task cost falling from
108 to 99, which is the shared counter leaving the path, so the growth from one task to four
fell from 94 ticks to 48. Taking out `compute`'s preemption share (about 6 ticks either way), the
per-trap excess went from about 1.37 ticks to about 0.66: 0.16 us of 0.34 us remains.

Throughput rose with it. `jpm_median` at four tasks was 932,371, 926,438 and 928,910 against 844,124
to 853,611 before (+9.4% on the medians); at 32 it was 997,945, 998,717 and 998,880 against 897,375
to 901,122 (+11.0%). One task moved 0.6%, so the gain is the multi-core part. The curve now reads
2.85x at four tasks and 3.07x at 32, against 2.62x and 2.78x.

### Where the rest is, from boots 4 and 5

**The instrumented kernel is slower** (`null_syscall` 118 at one task and 342 to 345 at four;
`jpm_median` 662,475 to 667,670 at four), so its ticks are not comparable with the boots above.
That is the cost the module warned of: the clock is read only on the contended path, and at four
tasks that path is taken 41% of the time. Its counts are what to read.

| At 4 tasks, boots 4 / 5 | |
|---|---|
| `current_cap` calls that found `IPC_TABLES` held | 41.4% / 41.8% (0.015% / 0.012% at 1 task; 51% at 32) |
| `IPC_TABLES` share of all lock wait | 95% (2,593,130 / 2,646,216 ticks) |
| `current_cap`'s share of the `IPC_TABLES` wait | 28% (734,022 / 744,371 ticks) |
| mean wait per contended acquisition | 3.70 ticks at `current_cap`, 3.80 everywhere else |
| reaper: stack free per reap, no lock held | 167 ticks, 42 us (149 at one task) |
| reaper: removal under `IPC_TABLES` per reap | 2.8 ticks |

So the reaper's share is gone on silicon too: the 42 us a reap used to hold the global lock for is
now spent holding nothing, and its last critical section is 2.8 ticks. What remains is ordinary
contention for `IPC_TABLES` itself. Four cores run IPC, `schedule()` and capability lookups through
one lock, and nearly half of all acquisitions find it held. The other 72% of the wait belongs to the
lock's other acquirers, the hot rows of
[`ipc-tables-lock-inventory.md`](../ipc-tables-lock-inventory.md). They are `ipc_call`,
`ipc_receive_cap` and `ipc_reply` (the `round_trip` job and its two echo servers). They are
`schedule()` twice per switch and `finish_switch` (the `yield` job and every block). And they are
`take_ipc_aborted` after each IPC. TCG's
hold-time table after the fix put the same ones on top, with `start_thread_control_block`.
`KERNEL_MMU` (rank 45) now shows 2,800 contended acquisitions a point: the stack frees and builds
moved out of `IPC_TABLES` meet each other there, at 1.6% of the wait.

## The next step: each thread's table off the lock

Milestone 761 (capability lookup off the global lock), its number provisional, gives every thread's
capability table its own lock, so `current_cap` stops taking `IPC_TABLES`. Under TCG the lookups
that waited fell from about 20% to almost none; the block has the numbers.

### The radon run that decides it

Two payloads from `74bbb19a1`, built before the bench:

```sh
cd ~/projects/nife-worktrees/capability-lock-per-thread
script/board-netboot --root target/board-caplock             # boots 1 to 3: the kernel that would ship
script/board-netboot --root target/board-caplock-lock-wait   # boots 4 and 5: the same plus lock_wait
```

Bench rules as [above](#the-radon-run-that-decided-it); transcripts to `bench/radon-<date>-caplock/`.
Read, in this order:

1. `null_syscall`'s `per_job` at 1 and 4 tasks on boots 1 to 3, against 99 and 147 from
   `bench/radon-2026-10-04-fix/`.
2. On boots 4 and 5, `site=current_cap`: `contended / calls` at 4 tasks, against 41% on that evening.
3. On boots 4 and 5, `rank=60 name=ipc_tables`: contended count and wait at 4 tasks, against that
   evening's boots 4 and 5.

What each outcome means, written 2026-10-04 (UTC) before any boot:

| Boots 1 to 3, `null_syscall` at 4 tasks | Reading |
|---|---|
| 115 or below | it worked: the remainder is about `compute`'s preemption share (about 6 ticks) over 99. Risk 4's per-crossing cost under load is explained |
| 116 to 135 | partial: the lock was part of the rest. Boots 4 and 5 say whether `current_cap` still waits (it should not) and what the IPC rows still cost the cores |
| 136 or above | the lock was not radon's remaining cause. If boots 4 and 5 show `current_cap` contention near zero and the job did not move, suspect the line traffic no counter sees (BUGS, `PERCPU`) |

One guard on the single-task cost, since a lock path dearer when uncontended would hide inside the
four-task number. **`null_syscall` at 1 task above 101** (99 today, plus 2%) is a regression to
explain before any row above is claimed.

## What risk 4's line should say

For the maintainer, who owns `design/fatal-risks/README.md`. The colour stays AMBER (calef's
ruling). The open finding's paragraph should read:

> The null syscall's near-doubling from one busy core to four was half a defect and half
> contention. The defect: the reaper held the global `IPC_TABLES` lock while freeing a dead
> thread's kernel stack, six TLB shootdowns that interrupt every core. Fixing it on radon
> (2026-10-04) cut the null syscall's growth from one task to four from 94 ticks to 48, and raised
> throughput 9% at four tasks and 11% at 32. The rest is the one global lock itself: at four tasks
> 41% of syscalls find it held. That is a lock this kernel chose and can split, not a cost of the
> capability model, and splitting it is proposed
> (`design/roadmap/761-capability-lookup-off-the-global-lock.md`). Until that is measured,
> the per-crossing cost under load is half explained and half open
> (notes/job-mix/null-syscall-under-load.md).

## BUGS

- The fix is sized on one machine, radon, with four harts, over five boots of one evening. The
  three plain boots agreed to within one tick at every `null_syscall` point and 0.64% on
  `jpm_median` at four tasks, so the size is a number, but it is radon's.
- **`PERCPU` straddles cache lines.** It is aligned to 8, so each 128-byte block spans three 64-byte
  lines and shares two with its neighbours. Their remotely written fields (the inbox, the steal slot)
  can pull away a line that `held_rank`, written twice per lock, lives on. Unmeasured, and not fixed here,
  so as not to change two things in one radon comparison. `#[repr(align(64))]` keeps the size at 128.
- **aarch64's job mix wedges under TCG on `main`.** On 2026-10-04 `script/job-mix --release --smp 4`
  went quiet after 2,744,000-tick subruns, at `tasks=2` on `main` and at `tasks=1` on this branch,
  one run each, on a loaded host. The 2026-09-19 capture completed. It is a multicore hang that
  wants a bisect, and this entry is where that work lives until someone takes it.
- **`script/fastpath-footprint` leaves `exception_body` out of aarch64's `syscall_entry`**, though
  every aarch64 syscall runs it (riscv64's list has `riscv_trap_body`). Moving the counter from that
  symbol into `syscall::dispatch` once read as 44 bytes of growth when it was 20.

## Proposed work

- Capability lookup off `IPC_TABLES`: built as milestone 761 (capability lookup off the global
  lock), number provisional, [its block](../../design/roadmap/761-capability-lookup-off-the-global-lock.md),
  and waiting on the radon evening [above](#the-next-step-each-threads-table-off-the-lock).
