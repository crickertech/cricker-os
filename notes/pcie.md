# PCIe, and driving a disk over it

The PCIe transport (DECISIONS §18): how the kernel finds a device on a PCI bus, brings it up, and
runs the same userspace virtio driver over it that runs over virtio-mmio. Scoped in
notes/pcie-transport-scope.md; this note is the "what actually happened and what the words mean"
companion after the build.

## The shape of PCI, in one screen

A PCI function is addressed by **BDF**: bus (8 bits), device (5), function (3). Every function
owns 4 KB of configuration space, and ECAM (Enhanced Configuration Access Mechanism) is
the modern way to reach it: one flat memory window, the function's page at
`base + (bus << 20 | dev << 15 | fn << 12)`. No magic I/O ports, no indirection registers; config
space is just memory-mapped bytes, which is why an empty slot "reads all-ones" (nobody drives the
bus, the read floats high) and why enumeration is a loop, not a protocol.

The first 64 bytes of config space are standardized: vendor/device id (how you recognize what it
is), the command/status registers, and six BARs (Base Address Registers). A BAR answers "where
do this function's actual registers live in memory?" and it is writable: firmware assigns each
function an address by writing one. Sizing is the famous dance: write all-ones, read back which
bits stuck (the low bits that stay zero encode the size and alignment), restore. Past the header,
optional features hang off the capability list, a linked list in config space; virtio-modern
puts everything it needs there as vendor capabilities: which BAR (and offset) holds the
common-config block, the notify doorbell, the ISR byte, the device config.

Two command-register bits matter here. Memory-Space Enable makes the BARs decode at all.
**Bus-Master Enable is DMA permission at the bus level**: a device without it cannot issue a
single memory transaction. The kernel grants it last, after the confined transport is registered,
because it is the bus-level twin of the authority the confinement layer polices.

## What the kernel is, on this bus

With `-bios default`, OpenSBI does no PCI setup: every BAR arrives zero. So the kernel is the
firmware here: it sizes each BAR and places it in the board's 32-bit PCI memory window,
bump-allocated and size-aligned. On a UEFI machine the firmware would have done this and the
kernel would only read; both paths go through the same code, because `read_bars` reports
assigned bases and the kernel places only the zeros.

The window itself, and the ECAM config window, come from the device tree, not from constants:
`memory::init` reads the `pci-host-ecam-generic` node's `reg` and `ranges` (the 32-bit
non-prefetchable entry, parsed by the pci crate's `mem32_window`) and `memory::pci_regions`
answers everyone else. They were QEMU constants (`PCI_ECAM_BASE`, `PCI_BAR_BASE`) until the
first VisionFive 2 boot, where 0x4000_0000 is DRAM base and mapping it collided with the direct
map (notes/visionfive2.md). A machine whose tree has no such node gets no window mapped and
every PCI probe answers "nobody home", the same degradation as an absent virtio-mmio device.

Division of labor, same as the mmio side: the pci crate is pure decode logic (ECAM math,
enumeration, BAR sizing, capability parsing, the INTx swizzle), host-tested against a fake config
space; kernel/src/pci.rs supplies the volatile accessors and the policy (which device, where
BARs go, which bits to set); the driver stays in userspace, unchanged.

## The transport seam

virtio is one device model over multiple buses. The queue machinery (descriptor table, available
and used rings, DMA against physical addresses) is bus-independent; only "where are the
registers and what do they look like" differs. `virtio::Transport` is that difference, contained:
the mmio variant passes the vocabulary through; the pci variant translates each register name to
the virtio-pci common-config layout, the ISR byte (whose *read* is the ack, deasserting INTx),
and the notify doorbell (`notify_base + queue_notify_off * multiplier`, resolvable only with the
queue selected). Registers pci has no equivalent for (magic, version, device id) are synthesized,
so the driver's sanity checks mean the same thing on both buses.

Everything above the seam is one copy: the shadow ring, the descriptor validator, the queue
layout contract, the userspace driver binary. The DMA confinement was written once and now
polices two buses, which is the demonstrator's argument in miniature.

## INTx

The legacy PCI interrupt is four shared wires (INTA..INTD) routed up through the bridge with a
standard rotation ("swizzle"): device `d` pin `p` lands on line `(d + p - 1) % 4`, then the board
maps the four lines onto interrupt controller inputs (32..35 on riscv `virt`). It is
level-triggered: the line stays asserted until the ISR byte is read. That plugs directly into the
kernel's existing model: the PLIC delivery masks the source, the driver's WAIT wakes, its
INTERRUPT_STATUS read (the ISR, via the transport) deasserts the line, and its Irq-capability ACK
re-enables the source. MSI-X (the device writes a message to raise an interrupt, many vectors,
no sharing) is the modern mechanism and a deliberate later step; nothing we drive needs it.

The swizzle is a hardcoded formula with a **witness**: host tests parse the riscv fixture's
device tree and hold `intx_irq` against all sixteen of the machine's own `interrupt-map` entries
(crates/pci/tests/qemu_virt_dtb.rs), the UART pattern. The ECAM base was a constant held the same
way; it is now parsed from the tree (above), and the same test pins the parse to the old value.

## What is proven, and where the edges are

The riscv suite's `a_userspace_driver_reads_a_file_over_the_pcie_transport` runs the whole line:
ECAM enumeration finds 00:01.0, the kernel places its BARs, sets up queue 0 through
common-config, the driver (byte-identical to the mmio one) submits a request past the shadow-ring
validator, the doorbell rings, and the completion arrives as INTx through the PLIC.

Both boards run it: riscv (INTx via the PLIC) and aarch64 (INTx via the GIC, SPIs 3..6, and the
**highmem** ECAM at 0x40_1000_0000; the machine names the node `pcie@10000000` after the low MMIO
base, and trusting the name instead of the `reg` is a mistake the witness test now guards). The
per-arch cost of the second board was constants plus two map entries, the portability claim in
concrete form.

Edges, honestly: INTx only, no MSI-X; the modern function only (`disable-legacy=on` in the runner,
because QEMU's default virtio-blk-pci is transitional and we do not drive the legacy layout);
both boards keep their working mmio paths alongside. The DMA
confinement is unchanged in spirit and in code; what PCIe adds to the trust story is Bus-Master
Enable, the bus-level DMA switch the kernel now controls explicitly.

## More than one bus (milestone 320)

The line above used to read "bus 0 only is mapped and enumerated (QEMU `virt` is flat; widening is
one constant)". It was true and it cost a day, which is worth keeping rather than quietly deleting:
a limitation recorded honestly still hides, when the machine that violates it is the first real one
you meet. On xenon, a Dell OptiPlex 7050, the M.2 NVMe sits behind a PCIe root port. The kernel
enumerated bus 0, `find_nvme_device` returned `None`, and the confinement test skipped with
QEMU's explanation for an absence (`NIFE_NVME not set on this leg?`) that had a different cause
entirely. Nothing failed. `bench/xenon-2026-09-17/nvme-attempt-2-no-controller-found.log`.

**Where the topology is written down is the bridge**, not ACPI and not the device tree. A PCIe root
port is a PCI function with a type-1 header, and bytes 0x18..0x1a of that header are the primary,
secondary and subordinate bus numbers whatever enumerated the machine wrote there. `pci::walk` reads
them and descends breadth-first over a 256-bit visited set, so a cycle in firmware's bus numbering
terminates rather than recursing, and each bus is scanned once.

**Why not simply map and scan all 128 buses the MCFG describes**, which is less work and needs no
new code at all? Measured on `q35` under QEMU with 256 MiB of RAM, it is also not very expensive:

| ECAM mapped | page tables |
|---|---|
| 1 bus (1 MiB), the old behavior | 560 KiB |
| 2 buses, the root-port topology | 560 KiB |
| 128 buses (128 MiB), what xenon's MCFG describes | 812 KiB |

252 KiB against a boot that spends 32,840 KiB on page tables is 0.8%, so the cost argument against
the flat scan is weak and should not be the one anyone repeats. The reason to walk the bridges is
the other one: a flat scan over a range ACPI happens to describe is the same species of assumption
that produced this bug, it issues configuration reads to bus numbers no bridge on the machine
decodes, and it stops being right on the first machine that numbers its buses differently.

How the kernel knows how much to map before it has read anything. `pci::survey` runs from
`kernel_main` before `arch::mmu::init`, which is the only window in the boot where the answer is
free: the x86 boot tables still cover the low 4 GiB indiscriminately, so every bus the MCFG
describes is already readable. The survey walks, prints every function it found, and records the
bus count; `map_everything` a hundred lines later maps exactly that. No lazy mapping, no second
pass, no arch-specific special case in the walk itself.

### BUGS

- The mapped range is contiguous, not the set of buses found. `pci::survey` records
  `highest_bus + 1` and the direct map covers bus 0 through that, empty buses included. A machine
  whose firmware numbers a root port's subtree 0x60 pays sixty-one buses of page tables to reach two
  buses of devices. The cost is bounded by what mapping the whole MCFG window would have cost, which
  the table above prices, so this is a waste rather than a hazard. Nothing measured does it yet.

- A bridge's memory window is neither read nor written. A PCI-to-PCI bridge forwards a memory
  transaction only inside the window its own base/limit registers describe, and this kernel does not
  touch them. Where firmware programmed them (every real machine so far) adoption keeps every BAR
  inside a window that already works. Where it did not, a device behind the bridge enumerates and
  does not work: under `NIFE_PCIE_ROOT_PORT=1` on QEMU, which boots PVH with no firmware, the
  controller on bus 1 answers configuration reads and its BAR does not decode. That is why
  `kernel::pci::tests::a_controller_behind_a_bridge_is_found_on_the_bus_behind_it` asserts
  enumeration and not a working disk. See
  `design/roadmap/proposals/a-bridge-window-the-kernel-programs-itself.md`.

- **Only `x86_64` surveys.** Both `virt` boards keep `PCI_ECAM_BUSES = 1`, which is the truth on a
  machine whose device tree describes a flat root complex and where QEMU puts nothing behind a
  bridge. The walk itself is the same code on all three and would follow a bridge the moment one
  appeared, except that it would refuse to descend past the one mapped bus and would say so. The
  survey has no home on those two architectures because there is no equivalent of the x86 boot map:
  their MMU is off until `mmu::init`, so there is no moment before the fine map where an arbitrary
  bus is readable through `phys_to_virt`. If a device-tree board ever grows a root port, that is the
  problem to solve, and lazy per-bus mapping (`pci::adopt` already maps a page at enumeration time,
  so the machinery exists) is the likely shape.

- The topology is read once, at boot. Hot plug would change it and nothing re-surveys.
