---
status: PARTIAL
raised: 2026-10-05
promoted_from: each-cores-percpu-on-its-own-cache-line
milestone_dependencies: none
decision_dependencies: none
machine_requirements: riscv64 silicon with four or more harts
specific_machine: radon (fatal risk 4's null-syscall numbers were taken there)
needs_person: yes
---
# 766. Each core's PerCpu on its own cache line

Raised 2026-10-05 (UTC) by `lane/null-syscall-hvf`, measuring fatal risk 4's null syscall under
load, and promoted from `design/roadmap/proposals/` the same day by `lane/percpu-own-line`. The
number 766 is provisional until the queue lands it. *(Title, slug and every name below are drafts.)*
`needs_person` is yes only because the acceptance measurement is taken at radon's bench.

## Index row

No two cores' per-core blocks share a cache line on any architecture, which takes the last
measured per-trap growth out of the null syscall under load on Apple cores. A compile-time
assertion keeps it that way.

## The defect

`kernel/src/cpu.rs` declared `PerCpu` with `repr(align(256))` on x86_64 and nothing on aarch64 or
riscv64, where it aligned to 8. The block is 128 bytes, so `PERCPU[i]` shared lines with its
neighbours. In the aarch64 job-mix build `PERCPU` sat at 24 mod 64; on riscv64 the parent note found
16 mod 64. A field another core writes (the inbox depth, the steal slot) then shared a line with
`held_rank`, which the owning core writes twice per lock it takes.

## The evidence

From [the HVF appendix](../../notes/job-mix/null-syscall-hvf-full-mix.md): four Apple M3 cores
under HVF, the full job mix, with milestone 761 (capability lookup off the global lock) in. The
null syscall's per-trap excess from one task to four, in 24 MHz ticks:

| Build | boots | excess at 4 tasks, 95% interval |
|---|---|---|
| `main` with #1663 | 19 | 0.042 [0.018, 0.066] |
| the same, `PerCpu` at `repr(align(128))` on aarch64 | 18 | -0.010 [-0.019, -0.006] |

The two were interleaved boot by boot. The excess grows one step per added core (0.009, 0.021,
0.041 over 30 boots), needs the spawn job (0.002 with it stubbed), and is not a lock (0.012% of
lookups contended). It is about 1.7 ns on a 33 ns trap there. Radon's size is unknown.

## What was built

- `PerCpu` is `repr(align(128))` on aarch64 and riscv64. x86_64 keeps its `repr(align(256))`
  (milestone 758 (the IPC fast paths shrink back inside their band), where the struct is 160 bytes). The size stays 128 on the first two, so the
  power-of-two assertion below `PERCPU` still holds and no core pays padding.
- A private constant `FALSE_SHARING_SPAN` (128) and a compile-time assertion that
  `align_of::<PerCpu>() >= FALSE_SHARING_SPAN`. Size is a multiple of alignment, so that one check
  is what puts every element of `PERCPU` on a span of its own. Losing either attribute, or adding an
  architecture without one, fails the build. 128 rather than 64 on every ISA: Apple's line is 128
  bytes, and adjacent-line prefetch pairs 64-byte lines on other parts.
- `PERCPU` moved from `0x...508` to `0x...580` on aarch64 and from `0x...d50` to `0x...d80` on
  riscv64 (release kernels): both now 0 mod 128. x86_64's address is unchanged.

**Reuse:** x86_64's own `repr(align(256))` on the same struct is the precedent this extends; no
crate is involved, since the change is one attribute and one assertion in the kernel.

## Gates

- `script/fastpath-footprint`: byte-identical to the parent commit on all three ISAs.
- Release kernel sections (`.text`, `.rodata`, `.data`, `.bss`) unchanged on aarch64 and riscv64;
  the padding falls inside page rounding.
- No Kani or loom harness models `PerCpu`. The steal slot's loom model is on
  `work_steal_slot::Slot`, whose layout this does not touch.
- CI runs the rest, including `script/stack-frame-check`, the boot-file-size check and the icount
  tripwire.

## What HVF said

MEASUREMENT PENDING

## Done means

- E's result from committed code: the HVF sweep above, at least 18 boots interleaved against the
  parent commit, four-task excess within its interval of zero.
- A radon run by [the parent note's](../../notes/job-mix/null-syscall-under-load.md) procedure
  and its one-task guard, recording the excess before and after. That is the acceptance
  measurement, and its result goes to risk 4's line whatever it is.
- `script/fastpath-footprint` and `script/bench` unchanged within their floors on all three ISAs.

## Follow-on

- **Outstanding.** The radon run, by the parent note's procedure and its one-task guard, against the
  parent commit. No bench session was available on 2026-10-05.
