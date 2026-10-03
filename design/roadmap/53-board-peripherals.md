---
status: PARTIAL
raised: 2026-07-31
milestone_dependencies: none
decision_dependencies: none
machine_requirements: riscv64 silicon with an NVMe controller and a NIC
specific_machine: none
needs_person: yes
---
# 53. The board's own peripherals: network and storage on real silicon

The storage half is built for QEMU as of 2026-08-15 (pull request #193): the
`non_volatile_memory_express` crate (queue mechanics, host-tested, 5 Kani harnesses), a rule-2 kernel driver confined
through the IOMMU before enable, class-code enumeration over §18, and an end-to-end boot test on
both ISAs. What remains of the milestone: the network half (the JH7110's GMAC), the board-side
PLDA XpressRICH root complex that carries the NVMe driver to the real M.2 slot (now tracked as its
own milestone, 163, NOT-STARTED), and the EL0
question, which is §86 (PROPOSED). Scope and honest limits: notes/non-volatile-memory-express.md, BUGS included.

In the second sense: the board is here and this needs hands on it. Bringing up
an NVMe controller and a real NIC means flashing, a serial console, and power-cycling a board that
will wedge, none of which a background lane can do. Both halves of the *old* gate are gone. The hardware cleared 2026-08-14: the board
is on the desk and 16a boots it through the full tour, so the two drivers finally have silicon to
exist on. The storage fork was decided by calef on 2026-08-15: **NVMe first**, because the backup
workload (milestone 55) measures sustained sequential write and endurance, which SD media fails,
and because a real PCIe root-complex driver compounds into milestone 87's x86 machine where an
MSHC driver serves one slot on one board. SD/eMMC stays in scope as the later path, undecided only
in its ordering against the network driver.

**In brief.** Milestone 16a boots a VisionFive 2 (firmware contract, NS16550, PLIC, Sv39). It does not
give the board a network or a disk. Everything above needs both, and **this is where virtio stops
carrying us**: every driver we have talks to QEMU's paravirtual devices, and real silicon has none.

**What it needs.**

- **Ethernet.** The JH7110 uses a Synopsys DesignWare GMAC (`dwmac`). Our net_stack (smoltcp) is
  device-agnostic above the driver, so this is a driver, not a stack rewrite. Rule 2 applies: it takes
  a base address and knows nothing else.
- **Storage**, and there is a real choice here. The SD/eMMC controller is the simplest path; **NVMe
  over PCIe** is the better one, because §18's PCIe transport already exists and NVMe would give the
  backup target actual throughput. Deciding which comes first is a fork, and it should be decided on
  measurement of what the backup workload needs rather than on what is easiest.
- **Persistence proven the hard way.** RedoxFS on the real device, with crash consistency tested by
  **actually cutting power**, which is a test QEMU cannot run.

**The parity note this milestone must carry.** These drivers are board-specific and aarch64 has no
equivalent board yet, so rule 5's "a scope note records the gap and the plan" applies rather than its
"ships on every architecture". Say so explicitly; do not let it look like an oversight.

**Effort: not estimated.** Two device drivers against real hardware with no emulator to iterate
against is a different activity from everything done so far, and estimates calibrated on QEMU work do
not transfer.
## Follow-on

- **Outstanding.** The network half: a driver for the JH7110's Synopsys DesignWare GMAC. Nothing in
  the tree touches it. The part appears only in design prose (`design/fatal-risks/README.md`,
  `notes/visionfive2.md`) and in an unrelated `reg-shift` comment in
  `kernel/src/drivers/ns16550.rs`; `kernel/src/drivers/` holds no ethernet driver of any kind.
  Checked 2026-09-03.
- **Milestone 163.** The board-side PLDA XpressRICH root complex that would carry the NVMe driver
  to the real M.2 slot. Minted 2026-08-25, still NOT-STARTED on a HARDWARE gate.
- **Decision.** Whether the NVMe driver can leave the kernel is answered rather than pending:
  `design/decisions/86-el0-nvme-driver.md` is DECIDED as of 2026-09-03, option 2a now with option 4
  addable without reshaping the EL0 driver, and the choice between them settled by measurement. The
  block's own prose still reads as though the fork were open.
- **Outstanding.** RedoxFS crash consistency proven by actually cutting power on radon. Nothing can
  power-cycle the board: milestone 224 is NOT-STARTED on a DECISION gate, and its own measurements
  record zero replies from the plug on the subnet. Checked 2026-09-03.
- **Outstanding.** SD and eMMC, kept in scope as the later path and undecided only in its ordering
  against the network driver. No MSHC or SD driver exists under `kernel/src/drivers/`, and no block
  or decision file has taken the ordering question since 2026-08-15. Checked 2026-09-03.
- **Done.** The rule-5 parity note this block says it must carry is carried, in `notes/non-volatile-memory-express.md`
  under a heading naming this milestone, stating what ships on all three architectures and what is
  board-specific.
- **Recorded.** Half the reason NVMe was picked over SD is gone: milestone 55 is REMOVED as of
  2026-08-30, so the backup workload whose sustained sequential write decided the fork no longer
  exists. The root-complex-compounds-into-87 half of that argument survives; the measurement half
  has no customer, and `notes/non-volatile-memory-express.md` still cites 55's storage bench as the thing that will want
  real queue depth.

## Index row

16a boots the board; this is what makes it able to *do* anything, and it is where virtio stops
carrying us
