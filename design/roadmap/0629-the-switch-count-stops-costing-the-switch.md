---
status: BUILT
raised: 2026-10-03
built: 2026-10-03
---
# 629. The context-switch statistic stops costing the switch path

*(Number provisional until the merge queue lands it.)* The drift study in #1486 (drift since the
2026-09-26 floors, still open) found that milestone 126 (the `procps` package)'s
`machine_statistics::context_switch()` cost about 3% of a yield on every ISA. It was one `fetch_add`
per `schedule()` on the machine statistics page, reached through a pointer chase from the core id.
calef ruled on 2026-10-03 at 01:41 UTC, on #1486, to move the count onto the timer tick so the
switch pays nothing. This lane measured that no form both keeps the published meaning and costs the
switch nothing, and wrote the options up on #1489. calef then ruled option B there on 2026-10-03:
keep the count exact and make it as cheap as it can be.

## What was built

- `cpu::PerCpu::switches`, a `u64` the switch increments beside the `switched_from` store it already
  makes, through the same `cpu::current()`. `machine_statistics::context_switch()` is gone.
- `sched::count_tick` passes the count to `machine_statistics::tick`, which stores it into this
  core's `CONTEXT_SWITCHES` word. The page is now exact as of the core's last tick, and the
  protocol crate, the kernel module and `notes/process-view/the-machine-and-your-share.md` say so.
- `PerCpu` stays 128 bytes on aarch64 and riscv64, the size the assertion in `cpu.rs` enforces.
  `adopted` gave up its slot and moved to `cpu::ADOPTED`, an array beside `PERCPU`. It is written
  only by the locked inbox drain and read only by one SMP test.
- A system test,
  `the_pages_switch_count_is_each_cores_exact_count_as_of_its_last_tick`, reads every online core's
  page line against that core's own `PerCpu` count. The page must never run ahead of the count, and
  once a core has ticked it must have caught up to what the count was before that tick. A
  tick-sampled or lossy copy fails the second check.
- A race fixed in the older test beside it,
  `the_machine_statistics_page_is_published_and_its_counters_move`. It broke out of its wait when
  a tick word moved and then asserted the interrupt word, which the same tick adds to a moment
  later. It failed on x86_64 in #1486's merge group (run 37089625155, before this change). It now
  waits for both, bounded by 50 ticks. The new test waits on its own claim the same way, since the
  tick's stores are relaxed.
- The bench floors on all three ISAs, re-saved with `# why:` lines citing this milestone and #1486.

## What the statistic promises, and who reads it

The `CONTEXT_SWITCHES` word is "context switches this core has made": exact, monotonic, per core.
`vmstat` prints it as `cs`, per second since boot, summed over cores. That is the column Linux and
FreeBSD print, and both count exactly on the switch: Linux with `rq->nr_switches++` in `__schedule`
(`kernel/sched/core.c`), FreeBSD with `VM_CNT_INC(v_swtch)` in `mi_switch`
(`sys/kern/kern_synch.c`). Both were read from upstream sources on 2026-10-03. `top` and `free` do
not read it. The meaning is unchanged. A reader can now see the count up to one tick late, which
the protocol already allowed for every other word.

## Measured

Debug icount (what the gate measures) on all three ISAs, and riscv64 release icount built by hand
the way #1486 did it. The before column is `main` at `5af5c712c` with the old toolchain pin. The
free column is the counter stubbed out, which equals option A below. B was re-measured on `d6a902a9b` with
`nightly-2026-10-02` and matched to the tick on every row shown.

| row | ISA | before | free | built (B) |
|---|---|---:|---:|---:|
| `yield_switch` | riscv64 | 192,465 (+2.87%) | 187,103 | 188,345 (+0.66%) |
| `ctx_switch` | riscv64 | 526,420 (+2.65%) | 512,838 | 516,252 (+0.67%) |
| `ipc_rtt` | riscv64 | 174,034 (+1.56%) | 171,359 | 172,963 (+0.94%) |
| `spawn_reap` | riscv64 | 35,576 (+1.75%) | 34,965 | 34,990 (+0.07%) |
| `yield_switch` | aarch64 | 1,163,476 (+3.32%) | 1,126,053 | 1,136,488 (+0.93%) |
| `ctx_switch` | aarch64 | 3,153,501 (+3.15%) | 3,057,234 | 3,084,758 (+0.90%) |
| `ipc_rtt` | aarch64 | 1,056,406 (+0.89%) | 1,047,088 | 1,049,353 (+0.22%) |
| `spawn_reap` | aarch64 | 220,189 (+0.36%) | 219,391 | 219,622 (+0.11%) |
| `yield_switch` | x86_64 | 19,860,248 (+2.89%) | 19,302,080 | 19,418,061 (+0.60%) |
| `ipc_rtt` | x86_64 | 17,603,719 (+1.58%) | 17,329,713 | 17,387,730 (+0.33%) |
| `spawn_reap` | x86_64 | 2,886,003 (+0.35%) | 2,876,069 | 2,926,145 (+1.74%) |
| `yield_switch`, release | riscv64 | 12,920 (+4.53%) | 12,360 | 12,480 (+0.97%) |
| `ctx_switch`, release | riscv64 | 59,616 (+2.08%) | 58,404 | 58,715 (+0.53%) |
| `ipc_rtt`, release | riscv64 | 11,980 (+2.39%) | 11,700 | 11,760 (+0.51%) |

x86_64 has no `ctx_switch` row. On riscv64 a tick is 100 instructions, so a switch costs, in
instructions at -O0 and in release: 134 and 14 before, 31 and 3 now. That is about 77% of the
cost #1486 found, recovered.

The tick pays for the copy. Measured with a temporary bench row calling `sched::count_tick` 10,000
times with interrupts masked, it went from 888 to 1,068 instructions at -O0 on riscv64, and from 52
to 60 in release. aarch64 went from 928 to 1,111 and x86_64 from 884 to 1,063. That is under 20,000 instructions a
second per core at 100 Hz. `script/fastpath-footprint` is unchanged or smaller on all three ISAs:
riscv64 `ipc_fastpath` went from 6,052 B to 6,032. `script/icount`'s handler span did not move,
because the statistics half runs after the timer is re-armed.

## Follow-on

- **Refused.** Option A, counting on the tick the ticks on which a core's running thread had changed.
  It is what the first ruling literally asked for, and it cost the switch nothing, but `cs` could
  rise by at most `TICK_HZ` per core per second. Two threads ping-ponging over IPC a million times
  a second would read as about one switch a tick. calef ruled B on #1489.
- **Refused.** A `u16` in `PerCpu`'s existing padding, folded into the page on the tick. It fits
  without moving `adopted` (a `u32` does not), but measured the same or worse than B on every row.
  riscv64 `yield_switch` was +0.93% at -O0 with `fetch_add`, which is a masked LR/SC loop on
  riscv64, and +1.62% with a load and store. It also wraps after 65,536 switches between two ticks, 6.5
  million a second per core. A release yield measures about 320 instructions a switch, so a 3 GHz
  core can get close, and a core that misses ticks gets there sooner.
- **Refused.** Deriving the count from the run queue. Every switch away from idle is a pop and a
  switch into idle could be counted by the idle thread for free, but `Fifo` keeps a depth, not a
  pop count. Making it keep both grows `PerCpu` by 16 bytes, which the assertion forbids. It would
  also go wrong silently the day someone adds a pop that is not a switch.
- **Refused.** Feature-gating the count, as milestone 300 (decompose the icount baseline drift) did
  with the cycle-counter grant. The shipping build would lose the `cs` column altogether.
- **Recorded.** The page's switch count is up to one tick old. That is written in the BUGS of
  `crates/machine_statistics_protocol` and `kernel/src/machine_statistics.rs`.

## BUGS

- x86_64 `spawn_reap` is +1.74% under B against the stubbed counter, and +0.16% under the `u16`
  variant. B changes nothing on the spawn path except where `adopted` lives, so code layout is the
  likely cause. It was not chased, and the floor records it.
- `ipc_rtt` moves by up to about 1% between builds that differ only in a bench row added after it
  (riscv64 174,034 against 172,318), so the `ipc_rtt` column is layout-sensitive at that size.
- No test proves that every switch is counted, only that the page publishes the count faithfully.
  A ping-pong test on one core can't hold that steady under SMP, because a steal or migration
  turns a switch into a yield into an empty queue. The count's correctness rests on the increment
  sitting beside `switched_from`, which every switch must write for `finish_switch` to work.

## Index row

`vmstat`'s `cs` stays an exact per-core count, but the switch now adds one to its own `PerCpu` block
and the tick publishes the total. That cuts the statistic's cost from about 3% of a yield to under
1% on every ISA, and from 14 instructions a switch to 3 in release.
