---
status: PARTIAL
raised: 2026-09-04
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 378. Read the DMAR, because a firmware checkbox is not an IOMMU

Filed as a proposal on 2026-09-04 by the lane that transcribed xenon's firmware
into notes/xenon-firmware.md; promoted by milestone 433 on 2026-09-19, when the tree was read
against it. **Most of what this asks for was built by two other milestones while it sat in the
pile**, and what is left is smaller and sharper than the file describes.

Re-checked 2026-10-03 (UTC): one item is left, printing the DMAR's own `INTR_REMAP` flag, and it
needs no board. The boot on xenon was taken on 2026-09-17, and carrying every DRHD is milestone
594's.

Built, and by whom. Milestone 161 (the x86_64 kernel port) gave `crates/machine_discovery`'s `parse_dmar` and
`first_drhd`, so the table is parsed at boot, its well-formedness checked, and the first remapping
unit's register base carried into `arch::x86_64::machine`. Milestone 317 (2026-09-17) answered the
question this file was really asking: `ECAP.IR`, the bit that says the hardware can remap interrupts
at all, is read from inside the guest on every boot and printed on the `iommu` line as
`interrupt remapping offered` or `absent`. It also overturned the premise §86 had been reasoning
from, since x86_64 has been offering interrupt remapping in every boot this tree has ever run and
nobody knew because no code read the bit.

What was left, checked 2026-09-19, and what is left on 2026-10-03. Three things were named, in the order they matter. Only the second remains.

1. No boot on xenon has reported it. Answered 2026-09-17, see the correction below. The one xenon transcript that exists
   (`bench/xenon-2026-09-17/first-light-095500.log`) predates milestone 317, so it carries the DRHD
   base and not the remapping line. The 2026-09-17 audit says the same, and one boot of a stick
   that already exists settles it.
2. The DMAR's own `INTR_REMAP` flags bit is parsed and thrown away.
   `kernel/src/arch/x86_64/machine.rs`'s `read_dmar` decodes the fixed part to prove the table is
   well-formed and keeps only `first_drhd`; `crates/machine_discovery/src/acpi.rs`'s own `BUGS`
   records the bit as read and never used. That is the platform's claim about itself, beside the
   unit's claim about itself, and this file asks for both.
3. Only the first DRHD is carried. A machine can have more than one. Done by milestone 594
   (every VT-d unit translates its own devices) on 2026-09-25.

*Corrected 2026-09-25 (UTC), from pull request #1275, the rehearsal for milestone 261 (the NVMe
driver leaves the kernel), on two of the three items.* The first is answered and does not mean what
it was expected to. xenon reported `interrupt remapping offered (unused)` on 2026-09-17 at 22:51 UTC
(`bench/xenon-2026-09-17/tour-display-225100.log`), but from the first unit, `0xfed90000`. The third
is the most urgent of the three, not the least. On the OptiPlex 7040, which shares xenon's register
addresses, that first unit covers only the integrated graphics and the catch-all is `0xfed91000`. So
"nothing this tree boots has a second unit", said twice below, is probably false of xenon, and the
one unit this tree brought up there was likely not the one in front of the NVMe. #1275 decodes every
DRHD and brings up the catch-all. All of this is inferred from the sibling machine, and stays
unverified until milestone 261's bench evening reads xenon's DMAR (`notes/risk-6-bench-evening.md`,
added by #1275).

The body below is left as it was filed on 2026-09-04, so the argument that moved this work from
a bench trip to a parser is readable as it was made. Read it against the three outstanding items
above rather than as a description of the tree today.

## What is being proposed

Parse the DMAR (DMA Remapping Reporting table) at boot and print what it says: the remapping units,
their register base addresses, the `INTR_REMAP` flag in the table's own flags field, and each
unit's extended capability register, whose `IR` bit is the one that says the hardware can remap
interrupts at all.

## Why

Two in-tree records have been waiting on a fact that turns out not to live where everyone assumed.

`notes/x86-uefi-boot.md` says the third of its open questions is xenon's alone: *"whether the
OptiPlex's firmware leaves interrupt remapping off. Nothing under QEMU can answer that, because the
answer is a setting in somebody else's firmware."* `notes/confinement-claims.md` carries the same
thing as a fifth claim: a confined component's MSI-X write is a memory write, so DMA remapping does
not cover it, and VFIO refuses to hand a device to an untrusted userspace driver on a machine
without interrupt remapping for exactly that reason. `design/decisions/86-el0-nvme-driver.md` names
it as the axis its options had not priced.

**The premise turned out to be half wrong, which is why this is a proposal rather than a bench
task.** Seventy photographs of the 7050's entire setup UI contain no interrupt-remapping control,
and the whole Virtualization Support menu is three pages: Virtualization, VT for Direct I/O,
Trusted Execution. So it is not a setting in somebody else's firmware in the sense that phrase
implies; there is no switch to photograph. What there is instead is a table the firmware
publishes, which any kernel can read, and which this kernel is already three lines from touching.

That relocates the work from the most expensive place in this project to the cheapest. AGENTS.md
notes that a null modem is where discovery costs most; a DMAR parser is host-testable logic in a
crate, exercised under QEMU, and the bench trip is reduced to reading one line off a boot tour.

## What it would settle

- `notes/confinement-claims.md`'s fifth claim stops being latent-with-no-way-to-check and
  becomes a measured yes or no on the one machine that has the hardware.
- DECISIONS §86's option set gets the number it was priced without: whether a userspace NVMe
  driver on xenon could be given interrupts at all, or would have to poll the way the current one
  does.
- `design/roadmap/195-uefi-boot-finish.md`'s remaining open question closes, or is shown to
  need something else.

## What it does not do

It does not enable interrupt remapping, program a remapping unit, or change any driver. It reads
and reports. Turning the unit on is a separate and much larger piece of work, and it should not be
started until somebody knows whether the hardware here supports it.

## The honest cost

The DMAR's structure is a short table with a variable-length list of remapping-structure entries,
and this tree has parsed several ACPI tables already (`notes/x86-port/acpi-and-pci.md` records the XSDT, MADT
and MCFG work). The expensive part is not the parse, it is deciding what the kernel should *do*
when it finds a DMAR it did not expect, and the answer for this proposal is nothing: print it.

## Follow-on

- **Done.** One boot of xenon under a kernel built since milestone 317 (the interrupt-remapping flags): the tour of 2026-09-17
  22:51 UTC printed `VT-d drhd at 0x00000000fed90000, root table default-deny, translating,
  interrupt remapping offered (unused)` (`bench/xenon-2026-09-17/tour-display-225100.log`, line
  119). Checked 2026-10-03. That settles `notes/confinement-claims.md`'s fifth claim as offered by
  the unit and unused by this kernel.
- **Outstanding.** Printing the DMAR's own `INTR_REMAP` flag beside the unit's `ECAP.IR`. Needs no
  board and no decision: a lane's work. Rechecked 2026-10-03, still true (`crates/machine_discovery/src/acpi.rs`'s
  BUGS still lists the bit as read and never used). First checked 2026-09-19: `read_dmar` in `kernel/src/arch/x86_64/machine.rs` decodes the fixed part and keeps
  only the DRHD base, with the comment saying nothing reads either field yet, and
  `crates/machine_discovery/src/acpi.rs`'s `BUGS` says the same from the other side. The two flags
  are different claims, one by the platform and one by the unit, and this block wants both.
- **Done.** Carrying more than one DRHD, which #1297 showed was the most urgent of the three,
  since xenon very likely has a second unit. Milestone 594 (every VT-d unit translates its own
  devices) brings up every unit and routes each device to its owner, 2026-09-25. Verifying that on
  xenon is 594's own outstanding boot.
- **Recorded.** This block reads and reports and never enables interrupt remapping, programs a
  remapping unit, or changes a driver. Turning a unit on is a much larger piece of work and should
  not start until somebody knows whether the hardware supports it, which is what the first bullet
  buys. Recorded beside the code in `kernel/src/arch/x86_64/iommu.rs`, whose header says `ECAP.IR`
  is read to report and never to act on.

## Index row

Two in-tree records were waiting on a fact nobody could check: whether xenon's firmware leaves
interrupt remapping off. The premise was half wrong, and finding that out is what this block is.
Seventy photographs of the OptiPlex 7050's setup UI contain no interrupt-remapping control at all,
so there is no switch to photograph; what there is instead is a table the firmware publishes and any
kernel can read, which relocates the work from the most expensive place in this project to the
cheapest. Most of it has since been built elsewhere: milestone 161 parses the DMAR and carries the
first unit's register base, and milestone 317 reads `ECAP.IR` from inside the guest and prints it,
overturning DECISIONS §86 (whether an NVMe driver can leave the kernel)'s reading that x86_64 boots with remapping off. What remains is the DMAR's own `INTR_REMAP` flag, which is parsed and discarded. The xenon boot
(2026-09-17) and the second remapping unit (milestone 594) are done.
