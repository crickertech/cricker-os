# The null syscall under the full mix, on four Apple cores

*An appendix of [the null-syscall note](null-syscall-under-load.md), measured 2026-10-04 (UTC) on
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
