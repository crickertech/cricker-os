---
status: PROPOSED
raised: 2026-10-06
milestone_dependencies: 127, 494
decision_dependencies: unwritten
machine_requirements: aarch64 silicon with a Tegra210 PCIe root port and a free slot
specific_machine: argon (the Jetson TX1 this is for; nothing else in the tree is a Tegra210)
needs_person: yes
---
# Ethernet on argon

Written by `lane/argon-ethernet-proposal`, a research lane, on calef's request of 2026-10-06 (UTC).
The title, the slug and every name below are provisional. No code was written.

In brief. argon is a Jetson TX1, not a Nano, so the NIC it carries is not the RTL8111 on PCIe that
the request recalled. It is a Realtek RTL8153 on USB 3.0, hung off Tegra210's XUSB controller, which
needs an NVIDIA firmware blob before it will run. The second finding matters more for ordering:
nothing named as waiting on this does. Milestone 225 (run the soak on radon, argon and xenon) and fatal risk 9's argon row run
over serial, as radon's 8 h soak did with no NIC driver in the tree. The recommendation is to build
nothing until argon prints its banner. Then put an Intel 82574L card in the carrier's PCIe x4 slot
and drive it with milestone 494 (a driver for the network card a PC actually has)'s existing `e1000e` driver, behind a Tegra210 PCIe host adapted from
NetBSD. The onboard RTL8153 comes later, behind a firmware ruling and a Tegra SMMU driver.

This depends on `design/roadmap/proposals/argon-boots-the-aarch64-kernel.md` (pull request #1728,
unmerged at writing), whose option A calef ruled on 2026-10-05: a build-time board option relinks
the kernel for tegra210. That proposal has no number yet, so `milestone_dependencies` names 127
(argon's first light) and this paragraph names the rest.

## What NIC argon has, read rather than recalled

Milestone 127 (the seL4 machine)'s block says argon is a "factory-sealed TX1 developer kit", bought as the seL4
machine. The kit is the P2180 module on the P2597 carrier. Upstream Linux describes that carrier
in `arch/arm64/boot/dts/nvidia/tegra210-p2597.dtsi`, read on 2026-10-06:

- `aliases { ethernet = "/usb@70090000/ethernet@1"; }`. The board's Ethernet is a USB device.
- Under `usb@70090000` (`nvidia,tegra210-xusb`, the XUSB host), `ethernet@1` has
  `compatible = "usb955,9ff"`: vendor 0x0955 (NVIDIA), product 0x09ff.
- USB2 port 1 is powered by a fixed regulator named `RTL_5V`, switched by GPIO H1. Its USB3
  companion is port 0, on the `pcie-6` pad lane.

FreeBSD's `sys/dev/usb/usbdevs` names that id `NVIDIA RTL8153 0x09ff USB 3.0 Ethernet`, and FreeBSD's
`if_ure.c` claims it with `URE_FLAG_8153`. OpenBSD's `if_ure.c` claims it as `TEGRAETH`. Linux's `r8152.c` lists `USB_DEVICE(VENDOR_ID_NVIDIA,
0x09ff)`. Four sources agree: the RJ45 on argon's carrier is an RTL8153 with NVIDIA's
ids, wired to the carrier, behind XUSB.

The recalled premise was right about a different board. The Nano's tree,
`tegra210-p3450-0000.dts`, aliases Ethernet to `/pcie@1003000/pci@2,0/ethernet@0,0`, a PCIe
function. Which Realtek part sits there was not read; RTL8111 is from memory.

Confidence: high that a TX1 developer kit's port is this RTL8153, and high that argon is a TX1
kit. Not read off argon itself: the carrier revision, and whether the RTL8153's MAC address is in
its own eFuse or supplied by NVIDIA's boot chain. The bench step below settles both in five minutes.

## Nothing named as waiting on this does

- Milestone 225's soak reads a heartbeat over the UART. radon ran it for 8 h 09 m on 2026-09-25
  with no Ethernet driver in the tree (the network half of milestone 53 (the board's own peripherals) is still outstanding).
  argon's soak waits on first light, not on a NIC.
- Fatal risk 9's widened grain asks whether a second machine costs a directory. Its experiment
  rides on 225, so it is serial too.
- Milestone 198 (a package manager, and the trivial install) on argon is the one real consumer: rung 3 fetches a package over a network. But
  198's stranger has a PC, and nobody will run nife on an end-of-life TX1. It is parity, not the
  customer path.

What Ethernet on argon would add is evidence, not unblocking. It would put a confined NIC on aarch64
silicon for the first time (fatal risk 6 has only radon's NVMe and TRNG). It would also test risk 9
more sharply than the soak can, because a machine's buses are where a HAL leaks. That second point
is below.

## What the tree already does, and where Tegra210 leaves it

| piece | in the tree | on Tegra210 |
|---|---|---|
| PCIe config access | `kernel/src/pci.rs` and `crates/pci`: ECAM only, from `pci-host-ecam-generic` or ACPI MCFG | not ECAM. A 256 MiB window at `0x0200_0000` puts extended-register bits at [27:24], above bus, device and function (FreeBSD `tegra_pcie.c`, `PCI_CFG_EXT_REG`). Root ports have their own windows at `0x0100_0000` and `0x0100_1000` |
| PCIe host bring-up | none: QEMU and xenon's firmware leave the link up | AFI and PADS registers, PLLE, pad lanes in the XUSB pad controller, resets, link training, INTx and MSI through the AFI |
| clocks and resets | `kernel/src/drivers/jh7110_clock_and_reset.rs`, 146 lines, radon's subset | Tegra210 CAR and PMC power partitions, a much larger register file |
| NIC driver | `crates/e1000e` (971 lines) and `kernel/src/e1000e.rs`: 82574L (`0x10d3`) proven under QEMU, I219 for xenon | reusable whole for an 82574L card |
| USB host | `crates/extensible_host_controller_interface`, milestone 242 (USB host and HID): found through PCI, control and interrupt transfers only, no bulk | XUSB is a platform device at `0x70090000` with IPFS and FPCI shims, and runs a falcon firmware |
| DMA coherence | aarch64 does no cache maintenance anywhere; `crates/paging` has two MAIR slots, device and write-back | Tegra210's tree marks nothing `dma-coherent`, so every DMA buffer needs cleaning and invalidating |
| IOMMU | `kernel/src/arch/aarch64/iommu.rs` drives an SMMUv3 | the SMMU is part of the memory controller (`nvidia,tegra210-mc`, `#iommu-cells = <1>`), NVIDIA's own design. Swgroups `AFI` (3) and `XUSB_HOST` (12) exist in `tegra210-mc.h` |

Two consequences follow from the table.

The xHCI driver refuses to start without confinement. `kernel/src/extensible_host_controller_interface.rs`
says "No confinement, no keyboard". So the onboard RTL8153 cannot carry a frame until a Tegra SMMU
driver exists. The `e1000e` driver instead runs "as confined as its arithmetic" when no IOMMU is up
(`notes/e1000e.md`, BUGS). A PCIe card therefore reaches a first DHCP lease before the SMMU exists,
under a limitation the tree already records, and becomes confined when it does.

The non-coherence is milestone 655 (DMA on a non-coherent RISC-V machine)'s problem on a second ISA. 655 prices cache maintenance for
the TH1520 and asks whether a confined driver can do its own. On aarch64 the answer is cheaper,
from memory of the Arm ARM rather than read today: `DC CIVAC` is legal at EL0 when the kernel sets
`SCTLR_EL1.UCI`, while `DC IVAC` is EL1-only. A data-plane process could clean and invalidate its
own buffers with no system call. The seam should be one design across both ISAs, not two.

## Prior art, and the licences

Every row was read on 2026-10-06 (UTC); line counts are `wc -l`. Linux's drivers for every piece
here (`r8152.c`, `pci-tegra.c`, `xhci-tegra.c`, `tegra-smmu.c`) are GPL-2.0 and are not to be
translated. They were opened only to confirm device ids.

| piece | candidate | licence | lines | covers argon |
|---|---|---|---|---|
| PCIe host | NetBSD `sys/arch/arm/nvidia/tegra_pcie.c` | BSD-2-Clause (Jared McNeill) | 819 | yes: its only compatible is `nvidia,tegra210-pcie` |
| PCIe host | FreeBSD `sys/arm/nvidia/tegra_pcie.c` | BSD-2-Clause (Michal Meloun) | 1,623 | yes: Tegra124 and Tegra210 |
| clocks | FreeBSD `sys/arm64/nvidia/tegra210/` `car`, `clk_pll`, `clk_per`, `pmc` | BSD-2-Clause (SPDX) | 597, 1,503, 968, 624 | yes; nife wants a small subset |
| pad controller | FreeBSD `tegra210_xusbpadctl.c` | BSD-2-Clause | 1,957 | yes: USB and PCIe lanes, UPHY PLL |
| XUSB host | FreeBSD `sys/arm/nvidia/tegra_xhci.c` | BSD-2-Clause | 1,126 | yes, loads `tegra210_xusb_fw` |
| RTL8153 | FreeBSD `sys/dev/usb/net/if_ure.c` | BSD-2-Clause (Kevin Lo) | 2,315 | yes, lists NVIDIA `0x09ff` by name |
| RTL8153 | OpenBSD `if_ure.c` | BSD-2-Clause | 2,643 | yes, as `NVIDIA TEGRAETH 0x09ff` ("Tegra Ethernet") |
| RTL8153 | NetBSD `if_ure.c` | BSD-2-Clause | 1,138 | 8152 and 8153, Realtek ids only |
| GPIO | FreeBSD `sys/arm/nvidia/tegra_gpio.c` | BSD-2-Clause | 890 | one pin needed (H1) |
| Tegra SMMU | none permissive found. FreeBSD `tegra_mc.c` only reports SMMU faults | n/a | n/a | written from NVIDIA's Tegra X1 TRM |
| 82574L | this tree's `crates/e1000e` | MIT OR Apache-2.0 | 971 | yes |
| RTL8111 card | Redox `drivers/net/rtl8168d` (MIT, Rust), FreeBSD and OpenBSD `re(4)` (BSD) | permissive | not counted | only if an RTL8111 card is bought instead |

Rust operating systems: Redox's `drivers/net` holds `alxd`, `e1000d`, `ixgbed`, `rtl8139d`,
`rtl8168d` and `virtio-netd`, and its `usb` holds `xhcid`, `usbhubd` and `usbctl`. There is no
USB Ethernet driver and no Tegra support. Nothing else was searched.

### The firmware blobs

- XUSB needs `nvidia/tegra210/xusb.bin` from linux-firmware: 126,464 bytes, version v50.24. Its
  licence, `LICENSES/LICENCE.nvidia`, allows redistribution "solely for use on operating systems
  distributed under the terms of an OSI-approved open source license", unmodified and with the
  licence attached. nife's MIT OR Apache-2.0 qualifies. It forbids reverse engineering and
  "translation" into another architecture or language. It also limits use to NVIDIA-designed
  processors, which the falcon inside XUSB is.
- The RTL8153 runs without firmware in FreeBSD. Linux optionally loads `rtl_nic/rtl8153a-2.fw` and
  siblings under `LICENCE.rtlwifi_firmware.txt` (binary only, no reverse engineering). Not needed.
- An 82574L card needs none.

So the onboard port brings a 124 KiB bus master running code nobody here may read, as the Wi-Fi
proposal's 8265 does. That is a firmware ruling in the sense of that proposal's fork F1, and it
raises the same confinement stakes.

## The ways onto a network, and what each costs

Estimates are by analogy. Milestone 494 wrote 1,934 lines for one NIC; 242's xHCI is 1,817 lines
of crate plus 867 of driver; radon's clock driver is 146. A BSD driver kept to one chip is guessed
to land near half its length in Rust, and that guess is recorded as one. Every step below runs on
argon only, because QEMU has no Tegra machine.

| | B. 82574L card in the x4 slot | A. the onboard RTL8153 | C. a USB dongle | D. SLIP on a spare UART |
|---|---|---|---|---|
| hardware | a card, bought | none | a dongle, bought | a second USB-serial cable |
| shared base (below) | yes | yes | yes | no |
| host controller | Tegra PCIe from NetBSD, 600 to 900, plus a config-access seam in `pci.rs`, 150 to 300 | XUSB from FreeBSD, 600 to 900, with xHCI found from the device tree | as A, or a new EHCI driver for the micro-B port | none |
| transfers | none new | bulk endpoints in 242's xHCI, 400 to 600 (shared with the Wi-Fi proposal's T1) | as A | none |
| NIC driver | none: `e1000e` | `ure` subset from FreeBSD, 800 to 1,200 | per dongle | a SLIP framer, about 150 |
| firmware | none | 124 KiB, NVIDIA licence, a ruling | none for CDC-ECM | none |
| first lease without the SMMU | yes, as `e1000e` runs today | no: xHCI refuses | no | yes, no DMA |
| lines to a first lease | about 1,750 to 3,250 | about 3,600 to 6,000 | more than A | about 300 |
| tests | the HAL's PCIe seam, and a confined NIC on aarch64 silicon | a USB device behind platform glue and a blob | the same as A | nothing a fatal risk asks |

The shared base, needed by A, B and C:

1. Cache maintenance for DMA on aarch64, one seam with milestone 655: 150 to 300 lines.
2. The Tegra210 clock, reset and power-partition subset for one controller, adapted from FreeBSD:
   400 to 700 lines. `jh7110_clock_and_reset.rs` is the shape.
3. The pad controller lanes and UPHY PLL for that controller, from FreeBSD's `xusbpadctl`: 400 to
   800 lines. PCIe lanes are in this block too, so B needs it as well as A.
4. A Tegra SMMU driver, for confinement: 600 to 1,000 lines with a new table format in
   `crates/paging`. NVIDIA's format is two-level with 32-bit IOVAs (from memory of the TRM). It is
   on A's critical path and a follow-on for B.

Option C loses to A. The carrier's full-size USB port is also XUSB, so a dongle needs all of A's
host work and buys a NIC driver argon already has. The micro-B port runs on a legacy ChipIdea EHCI
(U-Boot uses it), but nife has no EHCI and no other machine wants one.

Option D loses on purpose rather than cost. At 115,200 baud it would carry 198's package fetch,
but it exercises no DMA, no bus and no confinement, so it answers no fatal risk. It would also take
J21's other UART pins, unread.

Option B needs a card. An Intel Gigabit CT (82574L) costs about $15 to $35, from memory of
listings rather than read today. Any 82574L works; a later I210 would need a new driver.

### Why B tests risk 9 and the soak does not

`pci.rs` knows one way to reach config space. Every machine nife has run on offers ECAM: QEMU
`virt` on three ISAs and xenon through MCFG. Tegra210 is the first that does not. The cost of that
seam is a direct measure of the widened grain: whether a machine's bus costs a directory or a
restructure. Milestone 163 (the JH7110's PCIe root complex), NOT-STARTED, would have asked the same on radon,
but it is closer to ECAM.

## A firmware shortcut, measured before it is relied on

Upstream U-Boot's `p2371-2180_defconfig` enables `CONFIG_PCI_TEGRA`, `CONFIG_CMD_PCI` and
`CONFIG_RTL8169`, and only EHCI for USB. If argon runs that U-Boot, `pci enum` brings the link up
itself. A boot script could run it before `booti`, and nife could start by reading a live link. That
defers step 2 and most of step 3 of the shared base for B. It is a firmware contract, like radon's,
and it fails silently if U-Boot tears the controller down at handoff. It is worth one bench command
to learn, not a design.

Upstream U-Boot also cannot reach the onboard RTL8153, since it has no XUSB driver. That matches
the survey's note that stock U-Boot on a TX1 has no TFTP. L4T's own U-Boot was not read.

## The bench step for calef's 20-minute session

Milestone 225's block on pull request #1728 lists a 20-minute argon session (its section "What
calef can do at argon today"). Add one step after its step 4, at the same U-Boot prompt:

> 4b. Type `version`, `usb tree`, `pci enum`, `pci`, `dm tree` and `printenv ethaddr eth1addr`.
> An "Unknown command" reply is itself the answer for that command, so note it and move on.

What each line settles:

- `version` says upstream U-Boot or NVIDIA's L4T fork, which decides the shortcut above.
- `usb tree`, after the stick step's `usb start`: a `0955:09ff` device means this U-Boot drives
  XUSB, which would contradict the upstream reading.
- `pci enum` then `pci`: whether this U-Boot can bring the Tegra PCIe link up, and what root ports
  bus 0 shows. With an 82574L card in, the card should list as `8086:10d3`.
- `dm tree`: which Ethernet, PCI and USB drivers are bound.
- `ethaddr`: whether the boot chain hands down a MAC address.

Optional, five more minutes, settling the NIC on argon itself. Let autoboot continue into L4T's
Linux, if the eMMC holds one, and log in (the default user is recalled, not read). Then run
`cat /etc/nv_tegra_release`, `lsusb`, `lsusb -t`, `lspci -nn` and
`readlink /sys/class/net/eth0/device/driver`. Expected: `0955:09ff` at 5000M on driver `r8152`.

The log goes where the session's log goes, `bench/argon-<date>/uboot.log`.

## The sequence, if promoted

| step | ships | exit |
|---|---|---|
| 0 | argon prints its banner (the board-option proposal, then 127) | already 127's exit |
| B1 | the DMA cache-maintenance seam, shared with 655 | a host test of the line arithmetic; QEMU unchanged on all three ISAs |
| B2 | Tegra PCIe: clocks, pads, AFI, link, the non-ECAM config accessor | argon's boot prints the 82574L's `8086:10d3` behind the root port |
| B3 | `e1000e` on argon, unconfined under its recorded limitation | `net_stack` gets a DHCP lease on argon |
| B4 | the Tegra SMMU for swgroup `AFI` | the same lease, with a deliberate stray DMA faulting rather than landing |
| later | the onboard RTL8153: firmware ruling, XUSB, bulk, `ure` | a lease through the carrier's own port |

B2 and B3 each need an argon boot. B4 is the step that makes the result say "confined".

## The seven questions

1. Alternatives. A loses on its blob and on needing the SMMU before a first frame. C is A plus a
   purchase. D answers no fatal risk. Doing nothing yet is the recommendation's first half.
2. The tree's analogue. 494 for the NIC and its confinement split. 242 for xHCI. 655 for
   non-coherent DMA. The radon clock driver for a board's own clock tree.
3. Prior art. The tables above, read today. The SMMU format, the EL0 cache rules and the card's
   price are recalled and marked.
4. The premise. "argon's NIC is an RTL8111 on PCIe" is false: that is the Nano. "Ethernet unblocks
   the soak and risk 9" is false: both are serial. "198 on argon" is true but is parity.
5. Cost. B to a first lease, about 1,750 to 3,250 lines plus a card. A, about 3,600 to 6,000 plus
   a ruling. Estimates by analogy, not measurements.
6. Reversibility. All code is reversible, and nobody has acted on any of it. The irreversible item
   is A's blob in a distributed image. The `pci.rs` seam is internal and leaves the syscall surface
   alone.
7. Equal cost. B's lead is partly effort, said plainly: it reuses a driver. At equal cost B still
   wins, by a smaller margin. It carries no unreadable firmware and has one fewer bus between the
   NIC and `net_stack`. It also tests the PCIe seam, which is risk 9's question. A's real advantage,
   no purchase and the port the board ships with, is worth little on a bench board.

## Recommendation

Do not start before argon prints its banner, and add step 4b to the 20-minute session now. Then
promote this with B1 to B4 as its scope: an 82574L card, `e1000e` unchanged, the Tegra PCIe host
adapted from NetBSD's `tegra_pcie.c` with FreeBSD's for the clocks and pads, and the SMMU last. Raise
the onboard RTL8153 as its own proposal once the firmware ruling exists, and decide 655's seam once
for both ISAs.

## Reuse

Taken or adapted: NetBSD `tegra_pcie.c` (BSD-2-Clause) for the host. FreeBSD's Tegra210 `car`,
`clk_pll`, `clk_per`, `pmc` and `xusbpadctl` (BSD-2-Clause) for the subsets. This tree's `e1000e`
whole. Notices kept in each adapted file.

Written, with reasons: the config-access seam in `pci.rs`, because it is this tree's own
abstraction. The Tegra SMMU driver, because no permissive implementation was found.

Refused: every Linux driver named above, for licence.

## BUGS

- Linux and U-Boot disagree on the x4 slot's lanes. Linux's `tegra210-p2371-2180.dts` gives root
  port `pci@1,0` lanes `pcie-0` to `pcie-3`; U-Boot's names `pcie-1` to `pcie-4` as x4 and `pcie-0`
  as x1. `pci` at the bench says which root port the slot is.
- Whether Tegra's AFI can reach DRAM above 4 GiB (argon's 4 GiB runs to `0x1_7fff_ffff`) was
  not read. Allocating DMA regions below 4 GiB avoids the question.
- The RTL8153's MAC source, the carrier revision and the L4T release are unread on argon.
- All line counts are analogies to 494, 242 and radon's clock driver.
