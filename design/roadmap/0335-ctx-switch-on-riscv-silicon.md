---
status: NOT-STARTED
raised: 2026-09-03
milestone_dependencies: none
decision_dependencies: none
machine_requirements: riscv64 silicon with an ASID-tagged TLB
specific_machine: none
needs_person: yes
---
# 335. The ctx_switch number on real RISC-V silicon

Filed 2026-09-03 as an unnumbered proposal by the milestone 247 sweep,
from milestone 58's block; numbered 2026-09-19 by milestone 433. Premise re-checked 2026-09-19 and
it holds. `bench/baseline-riscv64.txt` still carries `ctx_switch 495050 5000` from QEMU and no
radon run exists: none of `bench/radon-2026-09-04`, `bench/radon-2026-09-05` or
`bench/radon-2026-09-16` has a `ctx_switch` row. `notes/riscv-tlb-shootdown.md` line 189 also still
says the VisionFive 2 "has not arrived", which stopped being true on 2026-08-14.

Only a core with a genuinely ASID-tagged TLB can charge the right price, which
means radon, the VisionFive 2, with a person at the bench rig. That has been the whole of it since
2026-09-16 at the latest, and the second token came off on 2026-09-19.

**The `DECISION` half was `DECISION` from 2026-09-03 to 2026-09-19 and there was no decision behind
it**, which is recorded rather than quietly dropped because it is the shape milestone 435 was minted
to find. It stood for *"nobody has said which board leg owns a RISC-V number"*, and the tree had
answered that by practice before this block was filed: board numbers live in `bench/<board>-<date>/`
(`bench/radon-2026-09-04`, `bench/radon-2026-09-05`, `bench/radon-2026-09-16`,
`bench/xenon-2026-09-17`), and `notes/bench-runbook.md` carries a *"radon, in order"* procedure that
a `ctx_switch` arm is one more step in. Choosing where a log file goes when the convention already
exists is a lane's call and a reversible one, which is the category AGENTS.md says to decide quickly
rather than route to calef.

**What is left of that half is a miscitation rather than a fork.** Milestone 58's block points this
measurement at milestone 24, and milestone 24 is *A second aarch64 board: Virtualization.framework*,
status `OPTIONAL`: an aarch64 target that cannot produce a RISC-V number under any ruling. Fixing
the pointer is an edit to milestone 58's block, not a question for anybody, and it is named in the
handoff rather than done here because this lane does not hold that file.

In brief. Milestone 58 removed the unconditional `sfence.vma` from `write_satp` behind a probe,
which is the whole reason ASIDs exist on RISC-V. The number that would show the win has never been
taken. QEMU's softmmu TLB is not ASID-tagged and flushes wholesale whenever `satp.ASID` changes, so
it charges for the added probe gate and credits nothing for the removed flush; icount came back
**+1.2% on `ctx_switch`**, which is the measurement reading backwards. Re-run `ctx_switch` on radon,
with and without the flush, and record what an ASID-tagged TLB actually saves.

## Why this matters

A milestone exists whose entire justification is a performance claim that has never been measured,
and the one measurement in the tree points the wrong way. `bench/baseline-riscv64.txt` carries
`ctx_switch 492350 5000` from QEMU, which is not evidence for or against the change. This project's
own standard is *measure, do not argue*, and an honest tie or loss recorded plainly is worth more
than a win nobody checked. Right now there is neither.

The second cost is the record. Milestone 58's block sends a future reader to milestone 24 for this
number, and milestone 24 is an aarch64 board that can never produce it. So the one pointer that
exists is wrong, and a reader who follows it wastes the trip. That miscitation only gets fixed by
somebody deciding where the number really lives.

`notes/riscv-tlb-shootdown.md` also says the board "has not arrived", which stopped being true on
2026-08-14. The blocker is no longer the hardware; it is that nobody owns the run.

## What it would take

The mechanism is built and gated: `asid_tagging_is_trusted()` in
`kernel/src/arch/riscv64/mmu.rs` is what decides whether the flush is issued, so both arms of the
comparison are already reachable from one build. What is missing is a run on radon over the bench
rig (UART into cordoba, smart plug 2), a `ctx_switch` number from each arm, and a home for the
result beside the existing baselines.

## Where it came from

Milestone 58's block: *"Take the `ctx_switch` number on real RISC-V silicon, where the TLB is
genuinely ASID-tagged. QEMU charges the added gate and credits nothing for the removed flush, so the
win this milestone exists for has never been measured, and the block's citation of milestone 24 is
wrong (that is an aarch64 board). Somebody still has to say which board leg owns the measurement."*

`notes/riscv-tlb-shootdown.md`'s BUGS section states the constraint: *"QEMU cannot exercise the case
the probe measures for the reason a real core would."*

## Index row

Milestone 58 removed the unconditional `sfence.vma` from `write_satp` behind a probe, which is the
whole reason ASIDs exist on RISC-V, and the number that would show the win has never been taken.
QEMU's softmmu TLB is not ASID-tagged and flushes wholesale whenever `satp.ASID` changes, so it
charges for the added probe gate and credits nothing for the removed flush; icount came back +1.2% on
`ctx_switch`, which is the measurement reading backwards. So a milestone exists whose entire
justification is a performance claim that has never been measured, against this project's own
standard of measuring rather than arguing. The second cost is the record: milestone 58's block sends
a future reader to milestone 24 for this number and milestone 24 is an aarch64 board that can never
produce it, so the one pointer that exists is wrong. Both arms are reachable from one build through
`asid_tagging_is_trusted()`; what is missing is a run on radon over the bench rig, and a ruling on
which board leg owns a RISC-V number.
