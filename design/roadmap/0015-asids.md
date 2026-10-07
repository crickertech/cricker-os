---
status: BUILT
raised: 2026-07-23
built: 2026-07-23
---
# 15. Tagged address spaces (ASIDs)

In brief. 16-bit ASIDs, generation/rollover; stop flushing the whole EL1 TLB per switch

Why it matters. perf the real-workload path needs on real silicon. **Built** (8-bit fixed bitmap, no rollover: milestone 14's bounds made generations unnecessary; notes/address-space-identifiers.md)

Deliverable. Give each address space an ASID so a context switch stops doing `tlbi vmalle1is`
(discard every EL1 translation, machine-wide) and instead flushes nothing.

Why. `mmu::set_ttbr0` does the sledgehammer flush today and says so: "no ASIDs yet ... every
address space uses ASID 0 ... ASIDs are the fix." A self-contained exercise in ASID allocation and,
more interestingly, ASID *reuse* (there are only so many; a real system recycles them and must flush
exactly the reclaimed one). It has no measurable payoff on QEMU, which does not model TLB cost, so it
is here for the mechanism, and as the honest prerequisite for reasoning about the
Spectre/address-space-switch cost the discussion raised. You cannot measure that cost while every
switch already flushes the world.

Detail. Standard aarch64 (ASID in TTBRx, `TCR_EL1.A1`); kernel/src/arch/aarch64/mmu.rs carries
the deferral.

## Follow-on

- **Milestone 58.** The RISC-V half. `sfence.vma` is local, so discharging an ASID machine-wide
  needs an IPI to every hart through SBI RFENCE, and until that existed every RISC-V context switch
  flushed the whole TLB and made the tag pointless.
- **Refused.** The generation and rollover scheme this block sketched from Linux. Milestone 14
  bounds concurrent address spaces at 160, below the smallest hardware ASID space of 256, so the
  exhaustion path the generations guard is unreachable here, and machinery whose hard path can never
  run is machinery that rots. If `MAX_SPACES` ever passes 255 the first answer is 16-bit ASIDs, not
  a new algorithm.
  This refusal is milestone 458 (design/roadmap/458-asid-generations-and-rollover.md), which carries
  it with the condition that would change it.
- **Recorded.** `notes/address-space-identifiers.md`: RISC-V permits `satp.ASID` to be zero bits wide, so "255 numbers
  for at most 160 spaces" is an aarch64 fact. A machine that cannot tell the tags apart keeps
  flushing on every switch, and the width is probed at boot.
- **Milestone 363.** `design/roadmap/363-what-asids-bought.md`, put a number on what ASIDs bought.
  The switch stopped flushing the TLB in July and no machine has measured what that changed, nor the
  address-space-switch cost this block calls ASIDs the prerequisite for reasoning about. QEMU cannot
  show it; argon, radon or xenon can. The mechanism ships with its payoff asserted rather than
  measured.

## Index row

a context switch stops flushing every translation
