# Why the null syscall slowed when more cores were busy

*Fatal risk 4's open finding from [the 2026-10-04 radon evening](radon-2026-10-04.md), measured on
2026-10-04 (UTC) by the `lane/null-syscall-contention` lane. Every number here is from QEMU's TCG,
which models no cache and no real inter-processor interrupt. Radon has not run the fix yet, and the
procedure for that is below.*

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

## The radon run that decides it

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

## What risk 4's line should say

For the maintainer, who owns `design/fatal-risks/README.md`. Until radon runs the fix, the open
finding's paragraph should read:

> The null syscall's near-doubling from one busy core to four has a named cause under QEMU. Every
> syscall's capability lookup takes the global `IPC_TABLES` lock, and the reaper held that lock while
> freeing a dead thread's kernel stack, six TLB shootdowns that interrupt every core. That is a
> defect, not the architecture, and it is fixed (notes/job-mix/null-syscall-under-load.md). Whether
> the fix removes radon's excess is one bench evening away.

If boots 1 to 3 land in the first row, the finding closes and the risk 4 verdict rests on the
2026-10-04 sweep's caveats alone, with this defect named as found and fixed. The residual global-lock
contention is the question milestone 17 (the multikernel-leaning scheduler) asks about scaling past
four harts, not this risk's.

## BUGS

- Measured under TCG only. The cause, the fix and the instrument are proven there; the size of
  the effect on radon is not. The procedure above is what fixes that.
- **`PERCPU` straddles cache lines.** It is aligned to 8, so each 128-byte block spans three 64-byte
  lines and shares two with its neighbours. Their remotely written fields (the inbox, the steal slot)
  can pull away a line that `held_rank`, written twice per lock, lives on. Unmeasured, and not fixed here,
  so as not to change two things in one radon comparison. `#[repr(align(64))]` keeps the size at 128.
- **aarch64's job mix wedges under TCG on `main`.** On 2026-10-04 `script/job-mix --release --smp 4`
  went quiet after 2,744,000-tick subruns, at `tasks=2` on `main` and at `tasks=1` on this branch,
  one run each, on a loaded host. The 2026-09-19 capture completed. It is a multicore hang somebody
  should bisect, and it is proposed below.
- **`script/fastpath-footprint` leaves `exception_body` out of aarch64's `syscall_entry`**, though
  every aarch64 syscall runs it (riscv64's list has `riscv_trap_body`). Moving the counter from that
  symbol into `syscall::dispatch` once read as 44 bytes of growth when it was 20.

## Proposed work

- *(proposed)* Capability lookup off `IPC_TABLES`. Each thread's capability table behind its own
  lock, so `current_cap` and the other per-thread operations stop sharing the global one. The
  inventory's "free win", now with a measured customer: after this fix, `current_cap` at four tasks
  still waits about 3 ticks a call under TCG. Revocation sweeps are the cost to price.
- *(proposed)* Bisect the aarch64 job-mix wedge under TCG.
