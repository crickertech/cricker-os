---
status: IN-PROGRESS
raised: 2026-09-30
branch: milestone/paint-path
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 623. The x86_64 paint path stops repainting the world

Number provisional (the paint-path lane, 2026-09-30): minted by the integrator at merge like every
global name. Raised from the icount attribution's finding
(`notes/benchmarks/icount-tick-scales.md`, PR #1470): the swish-check x86_64 leg spends 321.1 s
against 6.9 s (aarch64) and 7.3 s (riscv64), 15 s per transcript line, and the cost is not the
kernel (its paths agree within 7 percent once ticks are scaled); it is the paint path, where every
scroll repainted the whole 924x344 surface, every flush crossed an uncacheable aperture one word at
a time, and the console blocked on a full paint per write.

Three fixes, one per cost: the engine reports a scroll as movement and the painter moves pixels it
already holds (`video_terminal::Damage`, `display_terminal`'s scroll fast path); `Aperture::copy_wide`
stages pixel pairs into qword stores, 158,928 against 317,856 stores per full-surface flush, with a
u64 the ceiling on a `-mmx,-sse` target; and the console batches screen writes into one paint per
20 ms window over a bound notification (milestone 151 (notification objects: async multiplexing
without wait-any)) and a timer (milestone 106 (a wait that ends on either the interrupt or the
deadline)), so an ack promises the wire and the screen within one window. The console's ack meaning
changed; the change is recorded in `components/src/console.rs`'s module doc rather than slipped in.

The proof owed, and not yet paid: the x86_64 leg's line-time distribution before and after
(`NIFE_SHOW_LINE_TIMES` exists for exactly this), `SWISH_CHECK_X86_LINE_SECS` shrunk or deleted with
those numbers, and the parity cost on aarch64 and riscv64 stated (the paint path is shared; their
boots also carry the batching). The lane cannot run QEMU; the maintainer gates the legs.

## Index row

IN-PROGRESS on `milestone/paint-path` (PR #1471). Three fixes take the x86_64 swish leg's paint cost
apart at its three sources (whole-surface repaint per scroll, word-wide aperture stores, one
blocking paint per write); the wall-clock proof is owed by the maintainer-gated legs.
