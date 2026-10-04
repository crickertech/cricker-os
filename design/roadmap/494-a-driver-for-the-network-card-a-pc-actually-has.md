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

## Reuse

Reviewed 2026-10-04 (UTC), after calef's §46 (thin primitives or whole subsystems) amendment of the
same day: outside the kernel and the Kani-proved crates, take or adapt first, and record a reason
to write. Every row was read from its source, not recalled. Code lines exclude comments, blank
lines and tests.

| candidate | licence | parts it claims | how it touches hardware | code lines | what carries into 261's shape |
|---|---|---|---|---|---|
| Redox `e1000d` (`redox-os/drivers`, `net/e1000d`) | MIT | 8254x (`100e`, `100f`, `1004`), 82573L (`109a`), 82579V (`1503`). No 82574L, so not QEMU's `e1000e`; no I219 | a userspace daemon: `pcid` maps BAR0, `common::dma::Dma` allocates its own rings, an IRQ file, a `NetworkScheme` | 364 | register constants and the init order, which `crates/e1000e` already has. Nothing that allocates, maps or waits |
| `eth-intel` 0.2.4 (crates.io; rcore-os `tgoskits`) | MIT | `100e`, `100f` only | `mmio-api`, `dma-api`, `rdif-eth` traits from the ArceOS stack; owns DMA and IRQ | 412 | the same constants; its four dependencies would be §46 additions for none of the I219 |
| `e1000-driver` 0.1.0 (crates.io) | GPL-2.0 | 82540EP/EM | a kernel-module shape with its own allocator hooks | not counted | refused: it would be linked into `net_stack`, and §135 (running GPL software is aggregation) draws the GPL's line at the process boundary, where every linked GPL candidate before it was refused |
| FreeBSD `sys/dev/e1000` (`if_em.c`, `e1000_ich8lan.c`, `e1000_mac.c`, `e1000_phy.c`) | BSD-3-Clause (Intel) | every I219 generation, with field-found workarounds | C over FreeBSD's `iflib` and OS shims | about 16,000 across the four files | the I219's bring-up knowledge, which no Rust candidate has. Ported in part, below |

No Rust crate covers the 82574L or any I219, and no crate on crates.io names `e1000e` at all
(searched for `e1000`, `e1000e`, `i219`, `igb`, `intel-ethernet`).

Would adapting Redox's driver have been better than writing `crates/e1000e`? No, and not now
either. Its 364 lines map almost one for one onto this crate's 297 lines of register map and ring
logic, which is the easy part. It would not have run under QEMU without adding `10d3`, and every
line that allocates DMA, maps a BAR or waits on an interrupt is a Redox interface that 261's
shape replaces with a kernel control plane and spawn-time mappings. Its data path also trusts the
device: it slices a receive buffer by the descriptor's length unchecked (a length above 16 KiB
panics the driver), it enables promiscuous receive, and its reset, link and full-ring waits spin
without a bound. Adapting it would have saved perhaps a day on the register table and bought those
four defects to remove. What nife writes regardless is the split (`kernel/src/e1000e.rs`,
`e1000e_service`), the descriptor validation and its Kani proof, the host simulation, and the
`net_stack` integration; none of the candidates has any of it.

The I219 is where reuse pays, because that knowledge comes from field exposure. Against this
block's own BUGS list:

| I219 step | FreeBSD source | decision |
|---|---|---|
| leave ultra-low-power mode through the Management Engine | `e1000_disable_ulp_lpt_lp`, ME branch | ported, `crates/e1000e/src/pch.rs` `ulp` and `kernel/src/e1000e.rs` |
| empty the descriptor rings before a reset (SPT unit hang) | `em_flush_desc_rings`, `em_flush_tx_ring`, `em_flush_rx_ring` | ported, one recorded divergence (`pch::flush::tail_after`) |
| stop bus mastering before reset; STRAP writes around it; `KABGTXD.BGSQLBIAS` after | `e1000_disable_pcie_master_generic`, `e1000_reset_hw_ich8lan` | ported |
| leave ULP without an ME; unforce SMBus; LANPHYPC toggle and PHY-access check | `e1000_disable_ulp_lpt_lp` software branch, `e1000_init_phy_workarounds_pchlan`, `e1000_phy_is_accessible_pchlan` | proposed: needs MDIO |
| PHY reset with the MAC, then post-reset PHY workarounds | `e1000_reset_hw_ich8lan`'s `PHY_RST`, `e1000_post_phy_reset_ich8lan`, `e1000_hv_phy_workarounds_ich8lan` | proposed: needs MDIO. `CTRL.PHY_RST` is deliberately not set until then |
| the software/firmware semaphore and MDIO access | `e1000_acquire_swflag_ich8lan`, `e1000_read_phy_reg_mdic`, `__e1000_read_phy_reg_hv` | proposed: the layer the two rows above stand on |

The ported half is 54 code lines in the crate and about 110 in the kernel, under Intel's licence
kept at the top of `pch.rs`. It touches MAC registers and configuration space only, and it is
skipped for QEMU's 82574L, so nothing in the gates exercises it. The unported half is a PHY access
layer of roughly 600 lines of FreeBSD for the SPT path (measured by line count of the functions
above, not by porting them).

Recommendation. Keep `crates/e1000e`, recorded reason: no candidate covers QEMU's part or the I219,
and the code nife wrote is the confinement split and the input validation, which no candidate has.
Adapt FreeBSD for the I219, with attribution, in two steps: the MAC-register half now (done here),
and the PHY layer as its own piece only if xenon's bench boot shows it is needed. On a machine
whose firmware and Management Engine leave the PHY configured, the MAC half may be enough, and that
is a measurement to take rather than an argument to have. Take no dependency; vendor nothing.

## BUGS

- The page layout is read now, not recalled (2026-10-04, from Linux's `regs.h`):
  base and tail share a page, so the IOMMU alone confines. Intel's I219 datasheet is still unread.
- Wi-Fi is out of scope, and on a laptop it is the only network there is. A stranger with a
  laptop and no USB Ethernet adapter cannot reach rung 3.
- One family. `igc` and Realtek are follow-ons with no emulator for either.
- xenon's DMAR scope for the NIC is unread, the same unknown milestone 261 carries for the NVMe.
  The bench boot's preflight line now answers it in print.
- Nothing has touched an I219. The reset is the 82574L's plus FreeBSD's MAC-register steps for the
  I219; FreeBSD's PHY-register steps are not ported (Reuse, above). The bench step is the test.
- The bench step as written needs Ethernet at xenon, and the room xenon is in has no Ethernet
  port (calef, 2026-10-04). Its step 4 cannot happen there. Options, each calef's: move xenon to a
  port for one boot; a long cable; or a direct cable to patagonia with macOS Internet Sharing
  serving DHCP, which proves the driver but gives a lease from patagonia rather than from the
  house router, so it meets criterion 2 only in spirit. notes/e1000e.md says the same at the step.
- Intel's BSD licence asks a binary redistribution to reproduce its notice in the
  documentation. The source carries it (`crates/e1000e/src/pch.rs`); no image or package this tree
  ships carries a third-party notices file, and none exists.

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
- **Recorded.** The I219's PHY-register bring-up (MDIO under the firmware semaphore, ULP exit
  without an ME, the post-reset PHY workarounds), about 600 lines of FreeBSD, proposed to the
  maintainer as a milestone to build only if the bench boot needs it. `crates/e1000e/src/pch.rs`'s
  header.
- **Recorded.** No third-party notices file ships with an image, which Intel's BSD licence asks
  of binary redistribution. `crates/e1000e/src/pch.rs`'s BUGS.
- **Recorded.** `disk_throughput`, the other bench feature, is linted by nothing. `script/lint`,
  beside the `network_bench` line.

## Index row

xenon's network card is an Intel I219-LM.
