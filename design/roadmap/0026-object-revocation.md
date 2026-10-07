---
status: BUILT
raised: 2026-07-26
built: 2026-07-26
---
# 26. Object revocation: tear a process back down

**In brief.** Reclaim the TCBs, address spaces, and endpoints a process built, and the regions behind them, so a workload that comes and goes can leave. Built: region-ownership + generational staleness (no CDT), `Untyped::SPLIT`/`DESTROY`, generational region slots (retires the 256-lifetime cap), endpoints (safe subset). Extends §13 from frames to objects; DECISIONS §16, notes/object-revocation.md

Why it matters. the teardown half of "run real workloads": a process can be reaped, not just built

## Follow-on

- **Refused.** No capability derivation tree. Region ownership plus generational staleness answers
  the same question for every case that exists, and what a CDT would additionally buy is the general
  non-LIFO return-of-pages-to-parent. `notes/object-revocation.md` records the refusal in the words
  "we still have no reason to build one", and the LIFO case is built.
  This refusal is milestone 456 (design/roadmap/456-the-capability-derivation-tree.md), which
  carries it with the condition that would change it.
- **Recorded.** `notes/object-revocation.md` names "the honest remaining limit": endpoint revocation
  is the safe subset, so a service blocked on an endpoint that is *not* in any region being
  reclaimed is untouched, and the wake-with-an-error path reaches only waiters inside the region
  going away. The workaround, giving such endpoints a region of their own, is beside it.
- **Recorded.** `notes/object-revocation.md`'s BUGS section is honest that the single-winner claim
  over `untyped::destroy` is checked by loom rather than by the machine: loom models C11 rather than
  aarch64 or riscv64, assumes the mutual exclusion `IrqSafeMutex` is supposed to deliver, and
  searches who wins the claim rather than whether the winner then frees the right pages.

## Index row

the teardown half of "run real workloads": a process can be reaped, not just built
