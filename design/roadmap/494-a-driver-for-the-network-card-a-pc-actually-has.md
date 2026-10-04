---
status: PARTIAL
raised: 2026-09-19
promoted_from: a-driver-for-the-network-card-a-pc-actually-has
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 494. A driver for the network card a PC actually has

*(Number provisional until the merge queue lands it.)* Promoted from the
proposal `a-driver-for-the-network-card-a-pc-actually-has`, filed 2026-09-19, on calef's instruction
of 2026-09-20 to give every proposal on `main` a number. The text below is the proposal's own,
unedited except for this paragraph: the argument is its author's and promotion is not the moment to
improve it. Written by milestone 198 (a package manager, and the trivial install)'s rungs lane (`milestone/198-rungs-to-a-trivial-install`) for
rung 3 of the trivial install DECISIONS §157 (a trivial install is a web page,) defines: packages over the internet need a network
card, and the only one nife can drive is virtio-net, which no physical machine has.

The driver is written and tested under QEMU, which emulates the family xenon has.
The bench half needs xenon powered, which is the same attended boot milestone 261 needs.

## What was built, 2026-10-04

Exit criterion 1 is met; criterion 2 is calef's bench step, staged and written down. Procedure,
evidence and the confinement finding: [notes/e1000e.md](../../notes/e1000e.md).

- The datasheet question, answered from the register map. Ring base and ring tail share a
  4 KiB page (`0x2800`/`0x2818`, `0x3800`/`0x3818`, read from Linux's `regs.h`; Intel's I219
  datasheet was not read). So the process that moves a tail can repoint the ring, and VT-d is the
  whole of the DMA confinement on this device, as the section below predicted. Watched rather
  than assumed: with the kernel's `iommu::confine` skipped, QEMU's VT-d refused the NIC's first
  ring fetch and no lease came.
- Milestone 261 (the NVMe driver leaves the kernel)'s shape, which DECISIONS §86 (whether an NVMe
  driver can leave the kernel) authorised for NVMe, and no new syscall surface. The kernel
  (`kernel/src/e1000e.rs`) resets the NIC, reads its MAC address and programs the ring bases; the
  process is handed BAR0's two queue pages and an 18-page confined DMA region, and is denied page 0
  and page 5 (reset, the receive filter, the MAC address, the PHY). It polls; no interrupt is
  granted. The spawn is `kernel/src/user/e1000e_service.rs`.
- Inside `net_stack`, as recommended below, through a `Nic` enum that now carries smoltcp's
  `phy::Device` for both NICs, with full 1514-byte frames. The recommendation was about effort,
  and it is recorded that way in `components/src/e1000e_transport.rs`.
- `crates/e1000e` holds every computed thing: register map, descriptor formats, the DMA layout,
  the spawn handoff (a crate, by rule 7), and the whole receive and transmit ring logic behind two
  traits. 14 host tests against a simulated device and 3 Kani harnesses, each with a replayable
  falsification patch.
- The gates, on all three architectures, as DECISIONS §19 (architectural parity is a tenet)
  requires. `system_tests/src/user/e1000e_tests.rs` passes milestone 30 (the network stack as a
  confined component)'s DHCP and TCP gates over QEMU's `e1000e` behind each machine's IOMMU. It
  adds a UDP round trip and rung 3a's package fetch. All four are green locally on aarch64, riscv64
  and x86_64 (PVH and OVMF). No scope note is owed: QEMU attaches `e1000e` on every bus the
  runners have.
- x86_64 has a NIC under QEMU for the first time. The OVMF runner turned out to have carried
  QEMU's default `e1000e` all along, unchosen; it is now explicit.
- The bench boot: `cargo xtask network-bench` stages a `network_bench` kernel that prints the
  NIC, the DMAR preflight, the lease, a timed transfer from a LAN peer, and one verdict, and
  rehearses it under OVMF.

## Which card xenon has, and the tree disagreed with itself

xenon's network card is an Intel I219-LM. Dell's own specification sheet for the OptiPlex 7050
says, for the Micro: *"Integrated Intel® i219-LM Ethernet LAN 10/100/1000"* (read on 2026-09-19 from
`i.dell.com/.../OptiPlex-7050-Towers-Technical-Specifications.pdf`, which covers the Tower, Small
Form Factor and Micro). The machine's own System Information page records only `LOM MAC Address
D8-9E-F3-74-B2-A2` and `Wi-Fi Device: Intel Wireless` (`notes/xenon-firmware.md`), so the firmware
names no model.

Two records in the tree say otherwise, and neither cites anything. Milestone 260 (boot xenon over the network)'s `BUGS` and
`script/netboot-rehearsal`'s header both say xenon has *"a Broadcom LOM rather than QEMU's e1000"*.
Both arrived in one lane's commits on 2026-09-05 (`55702345`, `962d0ba9`) with no photograph, log or
specification behind them, and milestone 87, which chose the machine partly for its NIC, says I219.
The Broadcom sentence is false by Dell's specification, and this lane cannot edit either file;
it is listed for the maintainer.

What one boot would capture, and it costs nothing new. The PCI survey has printed every
function's `vendor:device` and class since commit `672d3b97` (2026-09-18 06:48 UTC,
`kernel/src/pci.rs`, `survey`). xenon's last recorded boot predates it and printed only `15
function(s) on the bus` (`bench/xenon-2026-09-17/first-light-095500.log`). The next xenon boot of
any current build prints the network card's device id on a line with class `020000`, and the
Intel 8265 wireless card on class `028000`. Photograph the survey; that settles the id. The I219's
PCI device ids vary with the chipset generation and are not recorded here, because they would be
recalled rather than read.

## What a stranger's PC most plausibly has

A judgement, with the parts recalled rather than read marked. Desktop boards mostly carry an Intel
or a Realtek gigabit or 2.5-gigabit controller (Intel I219, I225 or I226; Realtek RTL8111 or
RTL8125, recalled). Many laptops have no Ethernet port at all, and Wi-Fi is a different order of
work (firmware blobs, 802.11 management, WPA), so it is out of scope and recorded in `BUGS`. A USB
Ethernet adapter is the plausible bridge for such a laptop, and it rides on milestone 242's host
controller, so it is a follow-on to 242 rather than to this.

## Options for the first card (reversible, recommended)

| | Card | QEMU model | Who has it | Kept or lost |
|---|---|---|---|---|
| N1. Intel I219, `e1000e` family | xenon's | Yes: QEMU emulates `e1000e` (milestone 87 (the x86_64 bare-metal machine), checked against the pinned QEMU binary) | xenon; a large share of business desktops (recalled) | Recommended. Developed under QEMU, proved on the bench machine, one driver spanning both, which is the property milestone 87 bought xenon for |
| N2. Intel I225/I226, `igc` | Protectli VP2430, newer boards | No (milestone 87: *"It does not emulate `igc`"*) | Newer desktops | Lost as the first: no emulator, and nobody here owns one. Milestone 87 says `igc` is `igb`'s descendant, so N1's shape carries |
| N3. Realtek | Common on consumer boards | QEMU has an old Realtek model, not the RTL8111 family (recalled) | Consumer desktops | Lost as the first: no bench machine and no emulator for the part |

## How it is confined, and which precedent it follows

Two drivers in this tree are the analogues, and they differ in the way that matters:

- virtio-net (milestone 30) runs inside `net_stack`: `components/src/net_transport.rs` is a
  `#[path]` module of that binary presenting smoltcp's `phy::Device`, and the kernel owns the
  device's registers and mediates it through a `Virtio` capability with the shadow-ring validator.
  None of that mediation exists for a non-virtio device.
- The EL0 NVMe server (milestone 261, DECISIONS §86 option 2a) is the precedent for a real
  device: the kernel keeps the admin plane, the process holds a page of BAR0 and a confined DMA
  window, and on xenon VT-d is the confinement. It added no syscall surface.

So the I219 driver takes 261's shape, and one difference is worth checking before the design is
drawn. NVMe's specification put a page boundary exactly where the authority boundary belongs
(controller registers below `0x1000`, doorbells above). For the `e1000e` family the ring base
registers and the ring tail registers sit in the same 4 KiB page (recalled from the older 8254x
datasheets, not read from the I219's), so the page split that let 261's server ring doorbells
without being able to repoint its rings may not exist here. If it does not, the driver can aim its
rings anywhere inside its IOMMU domain, and VT-d is the whole of the confinement, which is
milestone 261's own load-bearing unknown (whether xenon's DMAR scope covers the function) applied to
a second device. Read the datasheet first.

Where the driver runs is a second, smaller question: a second `phy::Device` inside `net_stack`
beside `net_transport`, as virtio-net does today, or its own process with frames crossing an
endpoint. Recommendation: inside `net_stack` first, matching the tree; a separate process is a
frame protocol two programs agree on, which is the expensive category and wants its own reason.
The §92 test: at equal cost the separate process would be preferred for confinement (a NIC parser
and a TCP stack in separate address spaces), so this recommendation is about effort and is
recorded as such.

## Costs, from the tree rather than adjectives

- Milestone 87's estimate: *"A minimal driver is 1,500-3,000 lines against Intel's public
  datasheet; the plumbing around it (PCI decode, DMA confinement, the userspace net server) already
  exists."* An estimate written at purchase time, not a measurement.
- The measured neighbours: `net_transport.rs` is 390 lines (virtio, with the kernel doing the
  register work); `components/src/non_volatile_memory_express.rs` is 417 lines and
  `crates/non_volatile_memory_express` 1,086, host-tested with Kani harnesses, which is the split
  this driver should copy.
- Interrupts: x86 does not yet route a device line to a userspace waiter (milestone 299 (the x86 port-range capability)'s scope
  note), and 261's server polls. The first NIC driver polls too.
- The MTU: `net_transport.rs` fixes `MTU = 576` because its whole DMA region is one page. A 10 MB
  package at 576 bytes a frame is slow for no good reason; this driver gets a multi-page region the
  way 261's server got sixteen pages of transfer buffer, and full 1,500-byte frames.

## Exit criteria

1. Under QEMU, `-device e1000e` behind `intel-iommu`: `net_stack` completes a DHCP round trip
   and a TCP transfer through the new driver, the same gates milestone 30's virtio-net passes, on
   x86_64. aarch64 and riscv64 carry the driver too, since QEMU's `e1000e` is a PCIe device on
   every bus the runners attach, or a scope note says why not (DECISIONS §19).
2. On xenon, a DHCP lease from the house router and a measured transfer from a host on the LAN,
   photographed, with the survey line naming the card.

## BUGS

- The page layout is read now, not recalled (2026-10-04, from Linux's `regs.h`):
  base and tail share a page, so the IOMMU alone confines. Intel's I219 datasheet is still unread.
- Wi-Fi is out of scope, and on a laptop it is the only network there is. A stranger with a
  laptop and no USB Ethernet adapter cannot reach rung 3.
- One family. `igc` and Realtek are follow-ons with no emulator for either.
- xenon's DMAR scope for the NIC is unread, the same unknown milestone 261 carries for the NVMe.
  The bench boot's preflight line now answers it in print.
- Nothing has touched an I219. The reset sequence is the 82574L's, and the PCH bring-up Linux
  does for the I219 is absent; the bench step is the test. `crates/e1000e`'s `BUGS`.

## Follow-on

- **Outstanding.** The bench step on xenon (exit criterion 2), which is calef's: one boot of
  `cargo xtask network-bench --stage-only --peer <patagonia>:9494`, photographed, read by
  notes/e1000e.md's table. Checked 2026-10-04: no xenon boot of this driver exists.
- **Recorded.** The booted system does not use this NIC: the progenitor builds `net_stack` from
  virtio-mmio only, so x86_64's shell has no network and swish-check's x86_64 leg still omits the
  fetch lines. Proposed to the maintainer as a milestone (the progenitor endows a stack over the
  `e1000e`), which is what rung 3 needs on xenon. `notes/e1000e.md`'s BUGS.
- **Recorded.** Polled with a 1 ms sleep, and one server at a time. `kernel/src/user/e1000e_service.rs`'s
  and `components/src/e1000e_transport.rs`'s BUGS.
- **Recorded.** `find_e1000e_device` is a second copy of `find_nvme_device` with a different
  predicate; a third (milestone 242 (USB host and a keyboard that is not a UART)'s xHCI) is where
  a shared helper is lifted. `kernel/src/pci.rs`, at the function.
- **Recorded.** `disk_throughput`, the other bench feature, is linted by nothing. `script/lint`,
  beside the `network_bench` line.

## Index row

xenon's network card is an Intel I219-LM.
