---
status: DECIDED
raised: 2026-08-15
decided: 2026-09-04
ratified_by: calef
---

# 86. Whether an NVMe driver can leave the kernel, and what capability would let it

Answered 2026-09-03 by calef, after the research pass below: option 2a now,
designed so option 4 can be added without reshaping the EL0 driver, and the choice between them made
by measurement rather than argument. The reasoning is in "The decision" at the end of this section.
Raised by milestone 53's storage lane (2026-08-15, pull request #193), which
built the NVMe driver kernel-resident and stopped exactly here, correctly: the alternative needs
new syscall surface, and that is a boundary a lane does not cross (§10, §16).

## What is being decided

Every other DMA driver in this tree runs at EL0 behind a `Virtio` capability: the kernel owns the
queue addresses, parses each descriptor on its way to the device, and refuses any address outside
the driver's DMA region. That checkpoint exists because virtio's descriptors pass through a
doorbell path the kernel mediates.

NVMe has no such point to stand on. The controller fetches 64-byte commands directly out of
driver-written memory, and the DMA targets (the PRP fields) ride inside those commands. Nothing
kernel-side sees a command on its way past, so the `Virtio` capability's design does not transfer;
a confined EL0 NVMe driver needs a different contract. The question: does the NVMe driver stay in
the kernel, or does the syscall surface grow whatever lets it leave?

## The options

1. Stay kernel-resident, with the IOMMU as the whole confinement story. This is what is built.
   `bring_up` confines the controller's requester id to its six-page DMA region before enabling it,
   and on both `virt` machines an unconfined controller cannot fetch its first command, so
   forgetting the confinement fails loudly. The cost is architectural: the driver joins the TCB,
   against the microkernel thesis that drivers are user programs (§21, §23 spent real work getting
   virtio out).
2. A kernel-owned admin plane. The kernel keeps the admin queues (queue creation names physical
   addresses; that is the dangerous authority) and hands EL0 a capability to a pre-built I/O queue
   pair whose rings and data buffers all live inside the driver's own confined DMA region. The
   IOMMU bounds every fetch and every transfer to that region, so the kernel never parses commands;
   it only guarantees the geometry inside which any command is harmless. This is the `Virtio`
   capability's *shape* (kernel owns setup, EL0 owns the data path) with the IOMMU replacing
   per-descriptor validation.
3. Delegate the whole controller. Map BAR0 into the driver and let the IOMMU alone bound it,
   admin queues included. Simplest surface, but the driver can then re-point queues anywhere inside
   its region and the kernel cannot name which pages are rings versus data; and BAR0 contains every
   doorbell including the admin pair, so revocation semantics get murky. Recorded because someone
   will propose it, not because it is recommended.

## Recommendation

Option 2, **but not yet**. The recommendation is to hold this PROPOSED until the board-side work
(the JH7110's PLDA XpressRICH root complex, tracked as milestone 163, NOT-STARTED) exists, for the
same reason §23 built multi-queue confinement only after single-queue worked: the second data point
tells you which parts of the contract are QEMU artifacts. A second, possibly faster path to the same
data point may exist through milestone 87's x86 machine instead; see milestone 163's own text for
why that does not make 163 unnecessary. Option 1 is honest in the meantime because the limitation is recorded
in notes/non-volatile-memory-express.md's BUGS section rather than implied away.

A scoping lane has now reported on x86_64 (milestone 165, PARTIAL), and it is half an answer
rather than a whole one. Milestone 165 wired real PCI enumeration on x86_64 through ACPI's MCFG
and proved it under QEMU's `q35`, which is stronger evidence than an equivalent RISC-V QEMU proof
would be, because x86's discovery mechanism (ACPI tables naming an ECAM window) is the same one
real x86 hardware uses, unlike RISC-V's QEMU-only fake ECAM device. That closes the *discovery*
half of what a real data point needs. It does not close the *confinement* half this decision turns
on: x86 has no IOMMU driver at all (VT-d, roadmap item 6 of milestone 161, unbuilt), so a real
NVMe-behind-IOMMU controller cannot yet be brought up confined on either machine this tree targets.
This stays PROPOSED, pending either the JH7110 board bring-up or VT-d, whichever lands first. (A
separate lane was scoping a dedicated milestone for the JH7110 driver as this report was being
written; check the roadmap index for its number before citing it.)

The VT-d half has now landed and been exercised against this driver (2026-08-25), and the
*confinement* gap this section named is closed on x86_64. VT-d itself landed earlier this
session (milestone 161 item 6, `kernel/src/arch/x86_64/iommu.rs`) but had confined no real PCI
device; this is that exercise. `helpers/qemu-runner-x86_64.sh` now attaches `-device nvme` the same
way the aarch64 and riscv64 runners do (no `iommu_platform` flag, same as the other two, since a
real PCI device's DMA is not virtio's opt-in), and
`kernel/src/nvme.rs::tests::the_nvme_disk_serves_the_block_interface_end_to_end` now runs for real (an account: that module is `kernel/src/non_volatile_memory_express.rs` since 2026-09-18, and milestone 261 replaced that test with the EL0 one, so neither the path nor the symbol resolves today)
on all three architectures instead of skipping on x86_64. The result: the confinement claim
holds under VT-d exactly as it does under SMMUv3 and the RISC-V IOMMU. The existing driver,
unmodified in shape, enumerates the controller over ACPI's real MCFG, confines its requester id to
its six-page DMA region before enabling it, and serves SIZE/WRITE/READ over the blk-IPC verbs, on
QEMU's `q35` with `-device intel-iommu`.

Getting there took two fixes, neither of which is a confinement-contract difference between VT-d
and the other two IOMMUs, and both recorded in full in `notes/non-volatile-memory-express.md`'s new "What x86_64 needed
that the other two did not" section:

- `kernel/src/pci.rs::place_bars` trusted any nonzero BAR as already placed and already mapped,
  true on the two device-tree architectures (nothing runs before this kernel to place one) and
  false on x86_64's PVH boot, where QEMU resets the NVMe device's BAR0 to a live address of its own
  choosing, unrelated to the kernel's own mapped BAR window. Fixed to check the existing address
  against the window this kernel actually mapped rather than against zero.
- The frame allocator (`memory::bring_up_page_frames`) and the direct map
  (`arch::x86_64::mmu::map_firmware_regions`) both sized themselves from the e820 map's `usable`
  RAM entries alone. Attaching VT-d and NVMe together grows the ACPI tables QEMU parks above the
  top of guest memory enough that the adjacent `reserved` entry swallows the last few hundred bytes
  of the initrd (placed by the PVH loader at a fixed offset below the top of memory, sized for a
  smaller device set). Fixed to size the bitmap from whatever `forbidden` reaches past RAM's own
  end, and to map the initrd's recorded bounds explicitly regardless of how the memmap classified
  them.

Both are the kind of gap only a real, non-virtio DMA device on a real PVH boot could have found
(virtio's BARs and a bare boot's initrd never got close to either edge), and both would in
principle also bite a real UEFI x86 machine (milestone 87), which is why neither fix is
architecture-conditional: they check against what is actually true (which window is mapped, which
bytes the kernel has claimed) rather than against which architecture is running. Neither changed
the driver's shape, the syscall surface, or the confinement contract itself.

With this data point in, the two data points §86 was waiting on are the JH7110 board and the
VT-d/NVMe exercise; the second is now done. This section still does not decide option 1 versus
option 2; it reports that the confinement claim itself has now been checked, not merely built, on
all three of this tree's targeted architectures.

If the answer is option 1 permanently, nothing is blocked, and notes/non-volatile-memory-express.md's BUGS entry becomes
the standing record. If it is never decided, the driver silently becomes load-bearing kernel code,
which is how a microkernel stops being one; that is the failure this entry exists to prevent.

## What is blocked until it is answered

- An EL0 `nvme_server` program (the block-server shape `fs_proto::blk` already speaks).
- `block_roster` growing an NVMe transport kind (the surveyor cannot list the NVMe disk today);
  small, but its wire shape depends on who owns the controller.
- Milestone 55's storage stack picking its final backend: it can benchmark against the
  kernel-resident driver meanwhile, and those numbers stay honest if they are labeled with it.

---

## The research pass, 2026-09-03

Everything above this line is the record as it stood. What follows is a research lane
(`maintainer/research-86-el0-nvme`, no code) answering the six questions AGENTS.md asks of a fork
before it reaches calef, so that this section can be decided by reading it. It appends rather than
rewrites, the way the two findings above it did.

### The hold is spent, and it was measuring the wrong variable

The recommendation above is "option 2, but not yet", held "pending either the JH7110 board bring-up
or VT-d, whichever lands first". VT-d landed on 2026-08-25 and the block above records it. By its
own words the hold is over.

**Do not re-arm it on the JH7110.** The stated reason for waiting was that a second data point tells
you which parts of the contract are QEMU artifacts, and that reason does not survive being looked at:

- **The VT-d exercise confined a kernel-resident driver, so it is not a userspace-driver data point
  and could never become one.** The thing whose confinement it checked *is* the kernel. Every future
  IOMMU data point has the same shape while option 1 stands, so no number of them moves this
  decision. That is the sharp version of the QEMU worry and it is not about QEMU at all.
- Three IOMMUs under QEMU is three emulations. The x86_64 result is genuinely stronger evidence
  for *discovery*, and the block above gives the right reason (ACPI names an ECAM window on real
  hardware exactly as it does on `q35`). It is not stronger for *confinement*: QEMU's `intel-iommu`
  is a model, and its invalidation, IOTLB and fault behavior are the model's.
- **And the board the hold names has no IOMMU at all.** Milestone 143 (silicon IOMMU) exists to
  carry the RISC-V IOMMU driver to hardware, and milestone 16's block says why it is separate: it
  "waits on a board that ships the ratified RISC-V IOMMU spec and no such board exists today"
  (`design/roadmap/0016-real-hardware-iommu.md`). The JH7110 is not that board. So on radon, option 2's
  entire confinement story is absent, and a real-silicon NVMe experiment there confines nothing
  unless something in software does. Waiting for milestone 163 (the JH7110's PCIe root complex) to
  decide §86 would deliver a data point in which option 2 cannot be evaluated.

What actually bears on this decision is readable today, because it is a question about the NVMe
interface and about this tree's existing capability vocabulary. Both were read for this pass, and
both changed the answer.

### The premise is false, and that is the largest single finding

The section above says NVMe "has no such point to stand on", that "the controller fetches 64-byte
commands directly out of driver-written memory", and that "nothing kernel-side sees a command on its
way past". The last clause is wrong, and it is the clause the whole argument rests on.

The NVM Express Base Specification, revision 1.4c (fetched 2026-09-03 from
`https://nvmexpress.org/wp-content/uploads/NVM-Express-1_4c-2021.06.28-Ratified.pdf`; QEMU's `nvme`
device documents itself as implementing version 1.4), section 7.2.2:

> Host software writes the corresponding Submission Queue doorbell register (SQxTDBL)
> to submit one or more commands for processing.
>
> The write to the Submission Queue doorbell register triggers the controller to consume one or more
> new commands contained in the Submission Queue entry.

And section 7.2.1, steps 1 through 3, in order: the host places commands in the queue slots; the host
updates the Submission Queue Tail Doorbell register, which "indicates to the controller that a new
command(s) is submitted for processing"; **then** "the controller transfers the command(s) from in
the Submission Queue slot(s) into the controller for future execution".

That is a doorbell, and it is the structural twin of the one this tree already stands on.
`abi::virtio::NOTIFY`'s own documentation describes the identical mechanism from the other side:
"or `DeviceRefused` if a newly-published descriptor on that queue points outside the driver's DMA
region. On refusal the device is NOT told to go." A command written into an SQ slot is inert until
the doorbell is rung, exactly as a descriptor is inert until the queue is notified.

Three things are true that the premise garbled, and they are worth separating, because two of
them are real constraints and only one of them was stated:

1. Ring placement is already kernel-ownable, through the admin plane alone. An I/O queue's base
   address is named in a PRP field of an admin Create I/O Submission/Completion Queue command
   (`crates/nvme/src/lib.rs::Command::create_io_sq` / `create_io_cq`), and the admin queues' own
   bases live in the ASQ and ACQ registers. A driver that never issues an admin command and never
   touches those registers cannot choose where any ring lives. The section above has this right and
   it is what option 2 is built on.
2. **A mediating kernel must copy, not inspect in place.** Section 7.2.4 says a submission queue slot
   is free for reuse only once a completion reports the head advanced past it, so between the
   doorbell and the fetch the slot is the controller's and the driver's at once. A validator that
   read the driver's slot and left it there would be checking a value the driver can change before
   the controller reads it. This is the same time-of-check hazard that made
   `dma_validator::validate_and_shadow` copy into a kernel-private shadow ring rather than validate
   in place, and it has the same answer.
3. PRP2 can be a pointer to a PRP List, so a validator recurses one level for a transfer that
   spans more than two pages, and SGL-mode commands (CDW0.PSDT) would have to be refused or walked.
   Today neither occurs: `non_volatile_memory_express::prp_pair` answers `None` rather than build a list, and notes/non-volatile-memory-express.md
   records "One namespace, PRP-only, no SGLs, no PRP lists". Milestone 55's bulk path is what would
   change that, and it is the honest cost of the mediating option.

The tree half-knew this and lost it in a parenthesis. notes/non-volatile-memory-express.md's `BUGS` already says a
confined EL0 driver needs a capability "in the `Virtio` capability's mold (or command parsing at the
doorbell, which is the same decision wearing worse clothes)". The doorbell was named. Calling it the
same decision is what buried it, and it is not the same decision: parsing at the doorbell confines
without an IOMMU, and the mold it is being compared to does not.

So the option set above is incomplete. There is a fourth option, and it is the one that makes
NVMe match what §23 (multi-queue DMA confinement) did for virtio.

### The options, repriced

The three above stand, with one correction to option 2 and one addition.

Option 1, stay kernel-resident. Unchanged and still honest. The limitation is recorded in
notes/non-volatile-memory-express.md's `BUGS`.

Option 2, a kernel-owned admin plane. Still the right shape, and it is also what the field does
(see the prior art below). One thing it never priced: the split it describes is already a page
boundary in the hardware. NVMe puts the controller and admin registers at offsets 00h through
0FFFh (CC, CSTS, AQA, ASQ, ACQ among them) and the first doorbell at 1000h, with SQyTDBL at
`1000h + ((2y) * (4 << CAP.DSTRD))` and CQyHDBL at `1000h + ((2y + 1) * (4 << CAP.DSTRD))`
(revision 1.4c, sections 3.1.25 and 3.1.26). So "the kernel keeps the admin plane and EL0 gets the
data path" is expressible by mapping one page of BAR0 and not the other, with the `DeviceFrame`
capability this tree already has and **no new syscall surface at all.** That splits option 2 in two:

- 2a, map the doorbell page. Zero new surface. The catch is real and must be stated where a
  reader meets it: with `CAP.DSTRD` = 0, which section 8.6 calls "the expected doorbell stride value"
  for hardware implementations, the **admin** doorbells share that page with the I/O ones, so the EL0
  driver can ring the admin doorbell. It cannot write the admin submission queue if the kernel keeps
  the `PageFrame` capabilities for the pages the admin rings live in, which is entirely in the
  kernel's gift, since the kernel allocates and confines the region and mints every capability to
  it. What is left is that the driver can make the controller re-fetch admin slots the kernel wrote
  or never filled. That is a denial-of-service surface against the controller, not an escape, and
  the region-relative confinement is untouched by it.
- 2b, keep BAR0 and add a capability. The kernel maps no part of BAR0 to EL0 and exposes a
  doorbell-ring method, mirroring `Object::Virtio` exactly. Costs one enum variant and a few method
  constants, priced below.

Option 3, delegate the whole controller. Unchanged, and the interrupt finding below is one more
reason it is not recommended.

Option 4, new: mediate at the doorbell, the way §23 mediates virtio. 2b plus validation: on the
ring, the kernel walks the newly-published submission queue entries, bounds PRP1 and PRP2 into the
driver's DMA region, copies each validated entry into a kernel-private shadow submission queue (the
one the controller was actually told about at Create I/O SQ time), and only then writes the
hardware doorbell. This is `dma_validator::validate_and_shadow`'s design transferred, and the whole
reason to want it is that **it does not depend on an IOMMU.** On a board with none, which is every
board this project owns, it is the only option in this list that confines anything.

### What this tree already does, item by item

Question 2 turned out to decide most of the cost. Every material an EL0 `nvme_server` needs already
exists, and the closest analogue is `components/src/gpu_driver.rs`, which is a confined EL0 driver holding
a multi-page DMA region.

| What an EL0 NVMe driver needs | What supplies it today | New surface |
|---|---|---|
| BAR0's doorbell page mapped | `Object::DeviceFrame` plus `abi::page_frame::MAP` | none |
| The DMA region's frames | one `PageFrame` naming the whole run, per §102 (a Frame names a run of pages); `gpu_driver` slot 5 | none |
| The region's **physical** base, because PRP fields carry physical addresses | a spawn argument in `x1`. `components/src/entropy.rs` says it plainly: "Descriptors speak physical addresses; a process knows virtual ones, so the spawner passes this in" | none |
| The controller confined to that run | `crate::iommu::confine`, already called from `kernel/src/non_volatile_memory_express.rs::bring_up` before the controller is enabled | none |
| Completions | `Object::Irq`'s `WAIT` and `ACK` | none |
| The admin plane | stays in `Nvme::new` and `bring_up` | none |

Nothing in that table is speculative. The one place option 2 could still want new surface is the
doorbell, and only under 2b.

### What each option costs, counted

Adding a capability, measured against the precedent. `Object::Virtio` is one enum variant
(`kernel/src/cap.rs:124`), **six sites** across `kernel/src`, and **four method constants** in
`crates/abi`. It added **no new syscall number**: it dispatches through the existing `invoke`, which
is the pattern §16 (object revocation) established when it grew `Untyped`. One constraint to respect
if a variant is added: `kernel/src/cap.rs` asserts `size_of::<Object>() == 24` at compile time, so
the new variant must carry no more than `Virtio(usize)` does.

How much kernel code leaves, and how much stays. `kernel/src/non_volatile_memory_express.rs` is 474 lines, 416 of them
before `mod tests`.

| Piece | Lines | Under option 1 | Under 2 or 4 |
|---|---|---|---|
| `Nvme::new`, the admin plane (reset, admin queues, enable, identify, create the I/O pair) | 111 to 186, **76** | kernel | kernel |
| The data path (`size_bytes` through `next_cid`: transfer, transact, the polling loop) | 187 to 343, **157** | kernel | EL0 |
| `wait_rdy` and the register accessors | 344 to 378, **35** | kernel | split |
| `bring_up`, the policy function | 391 to 414, **24** | kernel | kernel |
| `crates/nvme`, pure arithmetic, already host-tested | **716** | linked by the kernel | linked by the EL0 program |

So roughly 157 lines leave the kernel and about 100 stay. The EL0 program's template is
`components/src/block_driver.rs`, which is **65 lines**: it is a thin shell around a driver's logic
serving `filesystem_proto::blk`, which is the shape an `nvme_server` takes.

Option 4's validator is smaller than the thing it is modeled on. `crates/dma_validator` is 1082
lines including its harnesses, and most of that is walking descriptor **chains** with indirect
descriptors and a per-queue high-water mark. NVMe has no chain: per newly-published entry it is two
`u64` range checks (PRP1, PRP2), an opcode check, and a 64-byte copy into the shadow slot. It grows
one level of recursion the day a PRP list appears, which today it cannot, and which milestone 55
(Time Machine) is what would change.

What option 4 costs at run time is the honest open question and it is measurable rather than
arguable. Today's driver completes one command before submitting the next (notes/non-volatile-memory-express.md's `BUGS`),
so the per-command copy is invisible against a QEMU round trip. At real queue depth it is a syscall
per batch plus 64 bytes copied per command, and `script/bench` is what would say whether that
matters. Nobody should assert it either way from this section.

### Prior art, fetched rather than recalled

Read on 2026-09-03. Where a claim came back as a search paraphrase rather than a page this lane
pulled, it is marked as such, per the tree's own fabricated-quote scar.

- seL4 hands out `seL4_X86_IOSpace`, `seL4_X86_IOPageTable` and `seL4_X86_Page_MapIO`, which
  map memory into an IO address space assigned to a PCI device
  (`https://docs.sel4.systems/projects/sel4/api-doc.html`). It mediates no command stream, and it is
  explicit that the IOMMU is the whole mechanism: on the seL4 devel list, Gerwin Klein wrote
  "Without an IOMMU, you will need to trust the drivers and the hardware of DMA-capable devices to
  either not use DMA or to use it safely only"
  (`https://lists.sel4.systems/hyperkitty/list/devel@sel4.systems/message/XN7ZN344AKBLF5BIIKP7YU5TUICKPVIV/`).
  The official manual PDF could not be parsed and is not cited here.
- Genode's platform driver keeps enumeration, config space, interrupt routing and, load-bearing
  for this decision, **DMA buffer allocation**: "a device driver must allocate DMA buffers at the
  ACPI/PCI server (while specifying the PCI device the buffer is intended for) instead of using
  core's RAM service to allocate buffers anonymously"
  (`https://genode.org/documentation/release-notes/13.02`). No command inspection.
- Redox is the outlier and the cautionary one. `nvmed` calls `pcid_handle.map_bar(0)`, taking the
  whole BAR including the doorbells, and reads `.physical()` off its own DMA wrappers to program the
  controller (`https://github.com/redox-os/drivers/blob/master/storage/nvmed/src/main.rs` and
  `.../src/nvme/mod.rs`). No IOMMU appears anywhere in it. That is option 3 with nothing underneath
  it, and Redox's own team names the missing IOMMU as a known gap (search paraphrase; the news page
  answered 403 and is not quoted).
- Linux VFIO and SPDK are the mainstream shape. The kernel keeps the IOMMU group as the unit of
  ownership, the DMA-map ioctl, and interrupt registration by eventfd
  (`https://docs.kernel.org/driver-api/vfio.html`). Userspace gets the BAR mapped: SPDK says "User
  space drivers utilize features in uio or vfio to map the PCI BAR for the device into the current
  process, which allows the driver to perform MMIO directly" (`https://spdk.io/doc/userspace.html`).
  Nothing inspects submission queue entries.
- Software mediation of NVMe queues exists, and it stops at the admin queue. "High-performance
  and Scalable Software-based NVMe Virtualization Mechanism with I/O Queues Passthrough"
  (arXiv 2304.05148, read at `https://ar5iv.labs.arxiv.org/html/2304.05148`) traps and emulates the
  admin queue, then bounds-checks PRP addresses and LBA ranges per request and lets the I/O queues
  run through with no host software in the path. MDev-NVMe (USENIX ATC 2018) is described the same
  way in secondary sources; its PDF answered 403 and this lane did not read it, so it is named and
  not relied on.

Two things follow, and they cut in opposite directions. Option 2 is the field's answer: everyone
who has solved this keeps the admin plane and hands over the doorbells. And **option 4 is genuinely
novel**, in the specific sense that this lane found nobody doing inline per-entry mediation of an
I/O submission queue, with the cited paper explicitly avoiding it on cost grounds. Novel is not the
same as wrong, and this tree's own §23 is the same trick at a smaller scale, but a decision for
option 4 should be taken knowing that nobody is going to have measured it for us.

### The axis nothing in this section has named: who may forge an interrupt

VFIO refuses to hand a device to an untrusted userspace driver on a machine without interrupt
remapping, and the escape hatch is a module parameter named `allow_unsafe_interrupts` (mechanism
confirmed by search paraphrase, not by a fetched kernel source file; the driver-api page says only
the general "Many modern systems now provide DMA and interrupt remapping facilities to help ensure
I/O devices behave within the boundaries they've been allotted"). The reason is that an MSI or MSI-X
message is a memory write to an architecturally special address, so plain DMA remapping does not
confine it: a driver that can write the MSI-X table can aim an interrupt wherever the platform will
deliver one.

**Every option here that maps any part of BAR0 to EL0 has to answer where the MSI-X table lives and
who may write it, and this tree has never asked.** Measured, not asserted:

- ~~`helpers/qemu-runner-x86_64.sh` sets `IOMMU="-device intel-iommu"` with no `intremap=on`, so
  interrupt remapping is off in every x86_64 boot this tree runs.~~ False. Amended 2026-09-18
  by milestone 317, which asked the machine instead of the script. `ECAP` read from inside the
  guest, QEMU 11.1.1, `q35`/TCG: `-device intel-iommu` gives `0xf00f4a` with `IR` **set**;
  `intremap=on` gives the identical `0xf00f4a`, a no-op; only `intremap=off` clears it, at `0xf42`.
  QEMU's `intremap` defaults to `auto`, which resolves **on** with no in-kernel irqchip.
  **So the capability was offered in every boot this tree has ever run, and the kernel never writes
  `GCMD.IRE`.** `kernel-irqchip=split` is not required either; that is a KVM constraint, and all
  three irqchip settings start `intremap=on` with no diagnostic.
- `helpers/qemu-runner-aarch64.sh` uses `gic-version=2`, which has no ITS, so there is no MSI
  translation path on that machine either. **Confirmed 2026-09-18, and it cannot simply be
  switched.** `gic-version=3` moves `reg[1]` from the GICC to the GICR and adds `its@8080000`, but
  `memory::init` matches the interrupt controller by its `intc@` name prefix and never reads
  `compatible`, so a GICv2 driver takes the redistributor and the machine gets zero timer ticks
  (3 of 5 self-tests against 5 of 5 at `gic-version=2`). That is milestone 227's territory, and its
  item 1, read `compatible` and report what was found, turns a silent wrong answer into a refusal
  and is far smaller than 227 itself.
- `riscv64` was missing from this list, and it is the one architecture where the mechanism is
  already reachable. Added 2026-09-18. The RISC-V IOMMU puts MSI translation inside its own
  device context rather than behind a separate feature or a separate device:
  `arch/riscv64/iommu.rs` reads `CAP_MSI_FLAT` and widens the device context from 32 to 64 bytes
  when the IOMMU reports it. Measured: `CAPS = 0x78c2cf4f10`, `MSI_FLAT` set, so the extended
  context is live and `attach` writes all four MSI words zero. **No flag is needed and none exists.**
  The inversion is worth stating: riscv64 is confined in the emulated case this project *can* test
  and untestable on the silicon it owns, since radon has no IOMMU at all. That is the exact reverse
  of x86_64's position now that xenon boots, and milestone 143's 2026-09-17 survey found a board
  with a ratified IOMMU buyable at about $299, which would close that gap and this one together.
- The NVMe driver never touches MSI-X. notes/non-volatile-memory-express.md's `BUGS` says so: "The controller is created
  with IEN=0 and no MSI-X table is touched." Still true after milestone 261 moved the driver to
  EL0 on 2026-09-17: it polls, holds no `Irq` capability, and the controller is still created
  `IEN=0` naming no vector.

So the gap is latent rather than live, and it becomes live the moment a driver leaves the kernel and
wants interrupts instead of polling. A driver has since left the kernel (milestone 261's EL0
NVMe server, 2026-09-17) and it polls, so the gap is still latent by choice rather than by
accident; `notes/interrupts.md` records that this tree's polling is a confinement choice wearing a
performance choice's clothes.

And this section's own reasoning needs one correction, which is the point of the amendment above.
It treated the gap as unexercisable because the platforms did not offer the mechanism. Two of three
do. What is missing is a kernel that enables it, which is a smaller and far more answerable thing
than a platform that lacks it. calef declined to mint the MSI-X ownership decision on 2026-09-17 for
want of an experiment; milestone 317 built the experiment the next day, and the question can now be
prototyped rather than argued. riscv64 makes it cheapest: the mechanism is one mode field in a
structure `attach` already writes, so it needs neither a new capability nor a new driver.

The parity shape belongs in whatever this becomes (DECISIONS §19): x86_64 and riscv64 can offer
interrupt remapping, aarch64 cannot until milestone 227, so any claim built on it starts with a
recorded gap rather than acquiring one later. Whatever this section settles has to say who owns the page
holding the MSI-X table, because that is the one part of the confinement claim an IOMMU doing DMA
remapping does not cover.

It is work rather than argument, so it has a home: notes/confinement-claims.md now carries it as a
fifth claim that is stated nowhere, beside the timing and device-values entries milestone 202 put
there for the same reason, with the two runner flags that would make it exercisable at all. A
proposal file under `design/roadmap/proposals/` would be the better home and was written first;
`script/roadmap --check` rejects that directory on `main` today, because the convention lands with
milestone 247 (follow-on work named by a finished milestone goes nowhere) and has not merged. When
it does, this belongs there.

### Reversibility, and who has acted

- Option 1 commits nothing. Fully reversible.
- Option 2a adds no syscall surface at all, so the only durable commitment is what an
  `nvme_server`'s spawn contract says, and this tree changes spawn contracts routinely.
- Options 2b and 4 add an `Object` variant and its methods. The precedent says these grow
  compatibly: §23 added a queue argument to `SETUP_QUEUE` and `NOTIFY` without a new syscall and
  without changing the disk's ABI.
- The genuinely expensive thing is the one this section already named, and it is not the
  capability. It is the wire shape `block_roster` grows for an NVMe transport kind, because that is
  something two programs agree on and its content depends on who owns the controller.
- Who has acted: nobody outside this repository. The only named future consumers are
  `block_roster` and milestone 55's backend, both unbuilt.

### What this lane would decide, and what is left to calef

Per AGENTS.md, a fork that touches the syscall surface gets options rather than a recommendation, so
this is offered as a reading and not as an answer.

**Two things this pass does settle, and they are not calef's to rule on**: the hold condition above
is satisfied and should not be re-armed, and the premise the argument rested on is false, which puts
a fourth option on the table that was never considered.

The reading. Option 2a is the cheapest honest thing and costs no surface, which makes it hard to
argue against on this tree's own tenets. Option 4 is the one that answers what §86 is ultimately
for: fatal risk 6's decisive experiment is "one real, non-virtio device on real silicon, confined, at
throughput", the silicon this project owns has no IOMMU, and option 4 is the only entry here that
confines without one. Those two are not exclusive; 4 is 2b plus a validator, and 2a is what you build
if you never intend to write the validator.

What is genuinely calef's: whether a new `Object` variant is minted at all (2b and 4) or the
doorbell page is simply mapped (2a); and, if a variant is minted, whether it ships with the validator
(4) or without it (2b), because that choice decides whether this system's confinement claim for
non-virtio devices depends on hardware nobody has yet put on a board.

If the answer is option 1 permanently, that is still a decision worth having, and it wants one
extra line the section above did not have: it means fatal risk 6 has no route left that this project
can run, because the driver whose confinement the risk is about would be the kernel by design rather
than by deferral.

## The decision, 2026-09-03

Option 2a now, designed so option 4 can be added without reshaping the EL0 driver, and the choice
between them settled by measurement. calef, after the head-to-head below: *"I agree with your
recommendation."*

### A correction to the research pass, which changes option 4's central claim

The pass argues that option 4 is the only entry that confines anything on hardware this project
owns, because no board here has an IOMMU. That is true of radon and false of xenon, and milestone
87's own requirements list says so: xenon was selected partly *for* VT-d, "because IOMMU-backed
driver isolation (milestone 16) is a parity theme (§19), and the x86 side of the DMA-confinement
story needs real hardware eventually."

So option 2 has a hardware home on a machine already on the desk. Option 4's argument survives for
radon and for any board without an IOMMU; it is not the categorical advantage the pass states.
Whether xenon's firmware actually exposes a DMAR is unverified and is already the last item of
notes/x86-uefi-boot.md's bench procedure.

### Why 2a first, and it is a sequence rather than a preference

**The irreversible part of this decision is not in 2a.** It costs zero new syscall surface: the
doorbell page is a `DeviceFrame`, the DMA run a `PageFrame` (§102), the physical base a spawn
argument the way `components/src/entropy.rs` already passes one. A capability variant belongs only to 2b
and 4. So building 2a produces what fatal risk 6 (a capability-confined userspace driver cannot
drive real hardware at real speed) needs while deferring the expensive choice, and the *move fast
on what can be undone* test, "who else has already acted on this", answers nobody.

It moves 157 lines out of the kernel, which is the microkernel thesis cashing out and the same
win §23 bought for virtio.

And option 4 is a superset of 2b rather than a rival. Which submission queue the controller is
told about is an admin-plane decision the kernel owns under both, so adding mediation later changes
`bring_up` and the ring path and almost nothing in the EL0 driver. That is the property the
implementation must preserve, and it is this decision's one design constraint.

### The two options answer different risks, which is why measurement decides

| | option 2a | option 4 |
|---|---|---|
| what confines it | the IOMMU | kernel validation at the doorbell |
| hardware needed | an IOMMU: xenon yes, radon no | none |
| new syscall surface | **zero** | one capability variant, no new syscall number |
| kernel on the hot path | no, EL0 writes the doorbell | yes, a trap per ring plus two range checks and a 64-byte copy per command |
| reachable by Kani | no, it is a hardware claim | yes, `crates/dma_validator` has harnesses |
| prior art | mainstream: seL4, Genode, VFIO, SPDK | novel |

Option 2a optimizes risk 6 by removing the kernel from the data path. Option 4 optimizes
risk 7 (the confinement claim is false) and risk 2 (the proofs prove trivia), and pays for it
on the hot path.

### What would move this to option 4, stated in advance

**Three measurements, and two need no new code.**

1. What doorbell mediation costs, priced from the mechanism already shipped. §23's virtio path
   *is* option 4's design: `crates/dma_validator` walks descriptors on `NOTIFY` and shadows them,
   and `script/bench` carries icount baselines on all three ISAs. NVMe's validator is strictly
   cheaper (two range checks and a copy against chain-walking), so virtio's number is an upper
   bound.
2. The depth at which it stops mattering. One trap amortized over a 32-command batch is noise;
   one trap per command at depth 1 is the whole cost. Milestone 101 (the L4 calibration, read from
   the IPC number that pays for the trap) is the calibration to read it against.
3. Whether xenon exposes a DMAR, which decides whether option 2 is viable on owned hardware at
   all. One boot, already the last item of notes/x86-uefi-boot.md's bench procedure.

If mediation is cheap at realistic queue depths, take option 4. A confinement claim that holds
without special hardware, and that Kani can reach, is worth more to this project than the last few
percent of throughput; that is the demonstrator's own argument. The reason not to buy it tonight is
that nobody knows the price.

### What this does not decide

The interrupt gap the pass found is orthogonal and survives both options. A confined component's
MSI-X write is a memory write to a special address, so DMA remapping does not cover it, and this
machine runs `-device intel-iommu` with no `intremap=on` and `gic-version=2` with no ITS. It is
latent while every BAR-reaching component is the kernel and live the moment one is not, which is
what this decision creates. It is recorded in notes/confinement-claims.md and is not answered here.
