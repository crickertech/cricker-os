---
status: PARTIAL
raised: 2026-08-03
milestone_dependencies: none
decision_dependencies: none
machine_requirements: a rented Scaleway Elastic Metal RV1
specific_machine: none
needs_person: yes
---
# 89. Scaleway EM-RV1: a second RISC-V implementation, rented

Raised 2026-08-03 alongside milestone 88 (nife on rented silicon: Oracle's free tier first, Graviton metal for the
PMU), when the cloud-hardware survey
turned up rentable riscv64 silicon. Rewritten 2026-09-25 (UTC) as the port plan, after calef ruled
to rent the machine ([§215 (the second RISC-V machine is a rented Scaleway Elastic Metal
RV1)](../decisions/215-the-second-risc-v-machine-is-a-scaleway-rv1.md)). Milestone 556 (a second
RISC-V implementation, for €16 a month) was the same proposal filed a second time and is folded in
here. The host-side PLIC prep is built; the rest is below.

One rented RV1, which needs calef's Scaleway account before anything runs.
Every step that can run under QEMU or on the host is ordered first, so the meter starts at a step
that needs the machine.

## What this is for

Fatal risk 9's implementation grain: a second machine of an architecture nife already boots
([`design/fatal-risks/README.md`](../fatal-risks/README.md)). radon is a StarFive JH7110 with SiFive U74 cores.
The RV1 is a T-Head TH1520 with four C910 cores. A second JH7110 would share every assumption this
tree could have made about one vendor's silicon; this machine shares almost none.

The verdict is a count. Every change the port needs lands in one of three places: under
`kernel/src/arch/riscv64/`, in portable code that reads a device tree, or elsewhere. The first two
are what §4 (kernel shape, with two cheap rules) promises. A change of the third kind is evidence
that the seam is not where the tree says it is, and the port's report lists each one.

## How far away it is, measured 2026-09-25

Read from Linux's `th1520.dtsi` at `165768bb7` and OpenSBI 1.9, then grepped against this tree.
radon's bring-up note opened by calling the JH7110 "startlingly close to QEMU's `virt` machine".
The TH1520 is not.

| Difference | What it breaks here | Where the fix lives |
|---|---|---|
| Peripherals at `0xff_d800_0000` and up, 40 bits | `phys_to_virt(pa) = pa + KERNEL_VA_BASE` only reaches 256 GiB under Sv39; the UART address overflows it at compile time | `arch/riscv64/mmu.rs` |
| UART0 at `0xff_e701_4000`, IRQ 36 | `console.rs`'s `UART_PHYS` and `UART_NODE` are one riscv64 constant, shared by luck with the JH7110 | a per-machine early-console constant, which `console.rs` already names as the right shape |
| PLIC states `thead,c900-plic`, two-cell interrupts | the region lookup and the context map both missed it | built: one shared list, `machine_discovery::plic::COMPATIBLES` |
| DRAM from physical 0 | the Image header's `text_offset` of `0x4020_0000` lands the kernel at the wrong address under `booti` | none if the FIT's own load address is used, which the first hour confirms |
| QEMU-only mappings (virtio window at `0x1000_1000`) are mapped unconditionally | that range is DRAM on the TH1520, and radon's first boot died of exactly this collision | `arch/riscv64/mmu.rs`, from the tree |
| XTheadMae: PTE bits 63:59 carry the memory type, and firmware turns it on | nife writes zero there; the behaviour of zero is undefined in T-Head's spec | `crates/paging`'s Sv39 leaf encoding plus a probe in `arch/riscv64/` |
| XTheadVector's `sstatus.VS` sits at bits 24:23, not 10:9 | `fp.rs` clears the standard field only, which is the 2026-09-24 audit's fix missing this core | `arch/riscv64/fp.rs`, keyed on the vendor id |
| No Sstc, timebase 3 MHz | nothing: the timer is SBI TIME and the rate comes from the tree | none |
| DMA is not coherent (`dma-noncoherent`), no Zicbom | nothing for first light, which polls the UART. Every DMA driver later | follow-on |
| SBI SRST not in upstream OpenSBI for this SoC | the `board` build's test exit calls SRST | probe; power API as fallback |

## Steps, in order

### Before renting: host and QEMU

1. Built: the PLIC is found by `thead,c900-plic` and its two-cell specifier decodes, witnessed by
   `crates/machine_discovery/tests/riscv64_th1520.rs` against a fixture modeled on the dtsi.
2. A device window for high MMIO. Map the top gigabyte of a 40-bit physical space
   (`0xff_c000_0000` to `0x100_0000_0000`) at the top gigabyte of the Sv39 high half, root index
   511, and make `phys_to_virt` piecewise. One gigabyte holds the TH1520's PLIC, CLINT and every
   UART. The boot table gains the same entry, inert on QEMU. The cost is one compare in
   `phys_to_virt` and a direct map capped at 255 GiB.
3. A per-machine early console, chosen at compile time (feature `board-th1520`, name provisional),
   reached through step 2's window.
4. Map QEMU's fixed devices only when the tree names them.
5. XTheadMae. Probe `mvendorid` over SBI (`0x5b7`, with `marchid` and `mimpid` zero, as Linux
   does), then read `th.sxstatus` (CSR `0x5c0`) for MAEE, bit 21. When set, RAM leaves carry bits
   62, 61 and 60 and device leaves carry 63 and 60, which are Linux's values. QEMU's `thead-c906`
   model advertises XTheadMae (`notes/cpu-models.md`), so `script/cpu-matrix` can boot the path
   before the silicon does. Whether QEMU honours the bits or ignores them is not yet checked.
6. Clear XTheadVector's `VS` field on a T-Head hart, beside the standard one.
7. A FIT image for this machine: `boot.itb` with `kernel` loaded at `0x8020_0000`, `fdt`, `opensbi`
   (`fw_dynamic`, upstream OpenSBI 1.9 `PLATFORM=generic`) and `env`. The tool is `mkimage`, which is
   not installed on patagonia; the image can be built on the RV1 itself, as Scaleway's guide does.

Steps 2 to 6 ship on all three architectures' suites unchanged, per §19 (architectural parity is a
tenet): none of them touches aarch64 or x86_64, and QEMU `virt` must boot exactly as before.

### Rented, hour one: read the machine, then a byte out of the UART

1. Rent hourly in `fr-par-2`, install Scaleway's Debian image, and enable the serial console.
2. From Linux, save `/sys/firmware/fdt`, `dmesg`'s OpenSBI banner and `/proc/cpuinfo`. The firmware
   tree replaces the modeled fixture; every assumption above is re-checked against it.
3. Reboot on the console and record U-Boot's version and environment. Scaleway does not publish
   either.
4. Keep Linux's `boot.itb` as a copy, write nife's, reboot, and watch for the banner. If nothing
   prints, the power API's rescue boot mounts the eMMC and restores Linux's image. That recovery
   costs minutes and needs no ticket.

### Rented, after first light

5. The self-test and the tour, to the line xenon printed: `nife self-test: N of N passed`.
6. The soak from milestone 225 (run the soak on radon, argon and xenon). This machine was rented
   for that cross-check of risks 5 and 9.

## What it costs

€0.042 an hour, excluding VAT, billed per hour. radon's bring-up took about seven bench sessions
of a few hours each. Budget 30 rented hours through step 5: about €1.26, or €1.51 with French VAT.
The money is not the cost. The cost is the attention to read transcripts, which is why the host
and QEMU steps come first.

One trap, not yet verified: bare-metal hourly billing usually runs for as long as the server is
allocated, powered or not. If so, a server left rented for a month by mistake costs €30.66 hourly
against €15.99 monthly. So each session ends by deleting the server, and a reinstall opens the next
one. Hour one measures how long an install takes and reads the billing page to settle the rule.

## The seven questions

1. What else was considered. A second radon shares every assumption. A bought TH1520 board was
   refused by §203 (capacity is rented rather than bought) and needs a person at the bench. RISE's
   runners and Cloud-V boot no custom kernel with a console. §215 has each.
2. What this tree already does: radon's port, whose note, `notes/visionfive2.md`, is the template.
   Its lesson was that QEMU hid the DRAM base, the online-hart set, the PLIC context formula and a
   zero-latency UART. Steps 2 and 4 are that lesson applied before the machine instead of at it.
3. Prior art, read: Linux's `th1520.dtsi`, `errata/thead/errata.c` and `irq-sifive-plic.c`;
   OpenSBI's `platform/generic/thead`; T-Head's extension spec; Scaleway's RV1 guide.
4. Is the premise true: that the RV1 boots a custom kernel was an open question on 2026-08-03. It
   is now documented. Scaleway's guide boots a user-supplied FIT image from eMMC. What is not
   documented is the U-Boot version and how the console is activated.
5. Cost: measured above in euros and estimated in hours. The hours are an estimate from radon's
   record, not a measurement.
6. Reversibility: every step is code under `arch/` or a device-tree parser, and the rental is
   hourly. Nothing here is a wire format, a syscall or a name calef has ratified.
7. Same cost for both: the window in step 2 is chosen over Linux's per-device `ioremap` because it
   is one table entry and one compare, not a virtual allocator. With equal effort the window still
   wins for a kernel that maps a handful of devices once at boot. If a machine ever puts devices in
   two distant gigabytes, that verdict changes.

## BUGS

- The fixture is modeled on Linux's tree, not dumped from the RV1's firmware. radon's modeled
  fixture got the PLIC's compatible string wrong, and only the real tree showed it. Hour one's
  second step exists to fix this one.
- A 0% SLA. Any gate built on this machine must degrade to a loud skip, the way milestone 81 (an HVF
  leg: the test suite on the physical core) already does.
- The PLIC here has 240 sources; `drivers/plic.rs` keys a table on `source % 128`. UART0's 36 is
  safe, and a source above 127 would alias.

## Follow-on

- **Outstanding.** Steps 2 to 7 of "Before renting", then the rented steps. Checked 2026-09-25
  with `grep -n 'const UART_BASE' kernel/src/arch/riscv64/mmu.rs`, which still reads
  `0x1000_0000`. Nothing in `crates/paging/src/sv39.rs` writes bits 63:59.
- **Milestone 655.** Milestone 655 (dMA on a non-coherent RISC-V machine). `design/roadmap/655-dma-on-a-non-coherent-risc-v-machine.md`, DMA on a non-coherent RISC-V machine: T-Head's `th.dcache.cpa`, `ipa` and `cipa`,
  then `th.sync.s`, behind the same seam a Zicbom machine would use. No DMA driver runs on this
  machine until then.

## Index row

Real riscv64 silicon from a second vendor, a T-Head TH1520 with C910 cores, rented at €0.042 an
hour. It is fatal risk 9's implementation-grain experiment, and a count of changes outside
`arch/riscv64/` is its verdict. The distance was measured on 2026-09-25: MMIO past the Sv39 direct
map, T-Head's own page-table memory types, and a PLIC binding this tree did not accept.
