---
status: PROPOSED
raised: 2026-10-05
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: radon (fatal risk 4's null-syscall numbers were taken there)
needs_person: no
---
# Each core's PerCpu on its own cache line

Raised 2026-10-05 (UTC) by `lane/null-syscall-hvf`, measuring fatal risk 4's null syscall under
load. Title, slug and every name below are drafts.

## The defect

`kernel/src/cpu.rs` declares `PerCpu` with `repr(align(256))` on x86_64 and nothing on aarch64 or
riscv64, where it aligns to 8. The block is 128 bytes, so `PERCPU[i]` shares lines with its
neighbours. In the aarch64 job-mix build `PERCPU` sits at 24 mod 64; on riscv64 the parent note found
16 mod 64. A field another core writes (the inbox depth, the steal slot) then shares a line with
`held_rank`, which the owning core writes twice per lock it takes.

## The evidence

From [the HVF appendix](../../../notes/job-mix/null-syscall-hvf-full-mix.md): four Apple M3 cores
under HVF, the full job mix, with milestone 761 (capability lookup off the global lock) in. The
null syscall's per-trap excess from one task to four, in 24 MHz ticks:

| Build | boots | excess at 4 tasks, 95% interval |
|---|---|---|
| `main` with #1663 | 19 | 0.042 [0.018, 0.066] |
| the same, `PerCpu` at `repr(align(128))` on aarch64 | 18 | -0.010 [-0.019, -0.006] |

The two were interleaved boot by boot. The excess grows one step per added core (0.009, 0.021,
0.041 over 30 boots), needs the spawn job (0.002 with it stubbed), and is not a lock (0.012% of
lookups contended). It is about 1.7 ns on a 33 ns trap here. Radon's size is unknown.

## The change

Align `PerCpu` to 128 on aarch64 and riscv64, which keeps its size at 128 and its size a power of
two (the assertion below `PERCPU`). 128, not 64: Apple's line is 128 bytes, and the parent note's
`align(64)` would leave the sharing in place there. x86_64 keeps 256.

**Reuse:** x86_64's own `repr(align(256))` on the same struct is the precedent this extends; no
crate is involved, since the change is one attribute in the kernel.

## Done means

- E's result from committed code: the HVF sweep above, at least 18 boots interleaved against the
  parent commit, four-task excess within its interval of zero.
- A radon run by [the parent note's](../../../notes/job-mix/null-syscall-under-load.md) procedure
  and its one-task guard, recording the excess before and after. That is the acceptance
  measurement, and its result goes to risk 4's line whatever it is.
- `script/fastpath-footprint` and `script/bench` unchanged within their floors on all three ISAs.
