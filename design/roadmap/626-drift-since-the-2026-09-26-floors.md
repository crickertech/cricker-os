---
status: RECORDED
raised: 2026-10-02
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 626. Drift since the 2026-09-26 floors: one counter, one layering, and three ISAs

*(Number provisional until the merge queue lands it.)* The question came from PR #1474's riscv64
re-save, which recorded `map_new` +7.89%, `spawn_reap` +3.30% and `yield_switch` +2.98% that the
compiler did not cause, and asked why riscv64 alone. The measurement, the tables and the caveats are
in [notes/benchmarks/drift-since-the-2026-09-26-floors.md](../../notes/benchmarks/drift-since-the-2026-09-26-floors.md).
This block holds what was found and the decision it leaves open. It is `RECORDED` because the
attribution is done and the remedy is deliberately not taken here.

## What was found

- **The premise was false.** aarch64 and x86_64 drifted with the same shape. Their floors were only
  restamped, and a restamp never compares `main` to the floors. x86_64 `map_new` is at +8.96%, the
  row nearest the 10% tripwire on any ISA.
- **Milestone 126 (the `procps` package)'s per-switch counter is the switch and IPC drift on all
  three ISAs**, commit `0d90e750c` in #1360. Stubbing `machine_statistics::context_switch()` returns
  `yield_switch` and `ctx_switch` to within 0.14% of their floors everywhere. It is about 134
  instructions a switch at the -O0 the gate measures and 10 in release, where it still costs
  riscv64 `yield_switch` 4.5% and `ipc_rtt` 2.4%. The 24-to-32 capability-table raise in the same
  commit is not the cause of those rows; it is part of `spawn_reap`'s.
- Milestone 23 (a capability-routed component OS with live replacement)'s run `RETYPE` is most of `map_new`, commit
  `b4f657d8a` in #1373: about 206 instructions per page at -O0, +0.7% in release.
- Milestone 206 (the user address-space map) adds a fixed 43 ticks to `map_new`, commit
  `7a90fcc59` in #1352, once per run rather than per page. Its mechanism is not found.

## What was built

`script/bench --riscv` no longer inherits `NIFE_DISK`. It failed on a fresh checkout and attached
five disks it never reads; without them rows move by at most 0.05%.

## The open question: what the per-switch counter should cost

This is a cost a wanted feature pays, so it is calef's call rather than a lane's. Three options:

1. Accept it and re-save the aarch64 and x86_64 floors with a `# why:` naming #1360, #1373 and
   #1352, as #1474 did for riscv64. Cheapest, and reversible. Costs 10 instructions a switch in
   release for as long as the counter exists.
2. Make the increment cheaper, then re-save. The release sequence spends 7 of its 10
   instructions finding this core's word (`tp`, `PAGE`, subtract, shift, index). A pointer cached
   per core would cut it to about 3. The obvious home, `cpu::PerCpu`, is the wrong one: it must stay
   128 bytes, and growing it to 136 cost riscv64's IPC fastpath 5.4% once (`kernel/src/sched.rs`,
   `CPU_TICKS`'s doc). Not prototyped, so the saving is arithmetic, not a measurement.
3. Count switches somewhere already paid for. `schedule()` already writes per-core state, so
   a count could live beside it and be copied to the page on the tick, as `machine_statistics::tick`
   already does for four other words. That moves the cost from the switch to the tick, and the page
   would be up to one tick stale. Not prototyped.

What the tree does in the analogous case: milestone 300 (decompose the icount baseline drift)
recovered +35.7 ticks a switch from a feature-gated grant by removing it from the shipping switch.
The counter is shipping, so that fix does not apply as it stands; options 2 and 3 are the shipping
analogue.

**Recommendation: option 1 now, with option 3 as a measured follow-up.** The counter answers
`vmstat`'s `cs` column, which is what it was built for, and 10 instructions is the release price of
answering it from the switch. Would we still choose this if both options cost the same? No. Option
3 would then win, because it takes the cost off the hottest path in the kernel. So this
recommendation is about effort, said in those words, and option 3 is the one to measure before
anyone calls the price settled.

What is blocked until it is answered: re-saving the aarch64 and x86_64 floors, which carry this
drift unrecorded, with x86_64 `map_new` 1.04% from tripping.

## Identified work, and its home

- Re-save the aarch64 and x86_64 floors, with this block as the `# why:`. Home: this block's
  open question; it waits on the answer.
- The +43 fixed term from `7a90fcc59`. Home: the note's BUGS section.
- Drift on `main` between saves is invisible. The weekly drift report reads committed floors, and
  the restamp measures only the compiler, so nothing compared `main` with the aarch64 and x86_64
  floors until this lane did. Home: a proposal to milestone 415 (sub-tripwire drift accumulates
  across baseline saves)'s owner, recorded here. A weekly `--check` of `main` on all three ISAs,
  reported in the same file, would have shown it on the day #1360 merged.

## Index row

The riscv64 drift #1474 recorded is on all three ISAs. Milestone 126's per-switch counter costs
`yield_switch` about 3% at -O0 and 4.5% in release; milestone 23's run `RETYPE` costs `map_new`
about 5.6% at -O0 and 0.7% in release; milestone 206 adds a fixed 43 ticks. The remedy is calef's
call: accept and re-save, or move the counter off the switch.
