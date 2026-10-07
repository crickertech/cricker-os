---
status: NOT-STARTED
raised: 2026-10-07
milestone_dependencies: 22, 23, 166, 600
decision_dependencies: 19, 26, 32, 86, 102, 121, 130, 159
machine_requirements: none
specific_machine: none
needs_person: no
---
# 811. The boot services leave the kernel: the kernel starts only the progenitor

*(Minted 2026-10-07 (UTC) by lane `boot-services-leave-the-kernel`. The number is provisional until
the merge queue lands it; the title and slug are drafts.)*

calef, 2026-10-07 (UTC): *"'move the boot service starters from the kernel to the progenitor' seems
like a milestone that's part of making nife ready for customers."*

The purpose: on a real boot the kernel builds one process, the progenitor, and hands it the device
capabilities. Every driver and server after that is built by userspace and has a supervisor. A
process the kernel starts has no parent that can restart it. §159 (lab machines upgrade like user
machines, and only a new kernel needs a reboot) was amended on 2026-10-07 by PR #1805: upgrading a
userspace component does not reboot the machine. A kernel-started process cannot meet that, because
nothing can restart or update it short of a reboot. Today that includes the file server.

On principle 1 this sits on the customer path. A customer's machine whose file server dies has to be
rebooted, and so does one taking a package update to its disk driver. That is the gap between "runs"
and "runs it" for any first customer, whoever it turns out to be.

## What exists, checked 2026-10-07 (UTC)

Read from `main` at PR #1804's merge; line numbers are from that commit. "Real boot" means a
build with none of `test`, `system_tests`, `bench` or the bench boots' features. A "tour" entry
runs only on the default build, which has no `shell` feature, before the hand-over on that
architecture.

### Started by the kernel on a real boot, inside `boot_progenitor`

| starter | call site | spawns | arch | what the process is handed |
|---|---|---|---|---|
| `fs_service::root_directory` | `kernel/src/user.rs:2463` | `block_driver` (`fs_service.rs:466`), `redoxfs_server` (`fs_service.rs:600`) | all | virtio: a `Virtio` cap, an `Irq`, DMA pages as spawn-time mappings; the FS server gets a `MemoryRegion` budget and the channel pages |
| the NVMe arm of the above | `fs_service.rs:436`, only when booted from NVMe | `non_volatile_memory_express` (§86's data plane) | all (exercised on `x86_64`) | BAR0's doorbell page and a DMA run, as mappings; the kernel keeps the admin plane |
| `boot_clock_page` | `user.rs:2442` → `user.rs:3188` | `clock` (`clock_service.rs:58`) | all | the clock page, and the RTC page as a mapping on aarch64 and riscv64; on `x86_64` a CMOS reading as an argument (§130) |
| `boot_instruction_entropy` | `user.rs:2545` → `user.rs:3331` | `entropy` in instruction mode | `x86_64`, aarch64 with `FEAT_RNG`; riscv64 answers `None` | two endpoints; no device |
| `boot_usb_keyboard` | `user.rs:2704` → `user.rs:2944` | `usb_keyboard_driver` | all | an `Irq`, about a dozen xHCI register pages and eleven DMA pages as mappings |
| `boot_screen_terminal` | `user.rs:2721` → `user.rs:3591` | `framebuffer_driver`, `display_terminal` | all, UEFI boots only | the firmware screen's aperture |
| `boot_e1000e_network` | `user.rs:2850` → `user.rs:3392` | `net_stack` over an `e1000e` | all (a PCIe part; exercised on `x86_64`) | BAR0's two queue pages and an 18-page DMA region as mappings |
| `boot_designware_network` | `user.rs:2852` → `user.rs:3451` | `net_stack` over radon's GMAC | riscv64 | nothing until `PROVEN_ON_SILICON` |
| `install_service::confirm` | `user.rs:2881` | `system_installer` (confirm role) | `x86_64` | one disk's NVMe endpoint |

### Started by the kernel on a real boot, outside `boot_progenitor`

| starter | call site | spawns | arch |
|---|---|---|---|
| `install_service::offer` | `kernel/src/lib.rs:1051` | `non_volatile_memory_express`, `entropy`, `system_installer`, `mkfs` | `x86_64`, a boot from a file only |
| `x86_userspace_demo` | `lib.rs:943` → `user.rs:2018` | two hand-assembled children | `x86_64`, every boot |
| tour: `console_service::start` | `lib.rs:2134` | `console` | aarch64 |
| tour: `virtio_service::start`, `start_pci` | `lib.rs:2207`, `lib.rs:2244` | the virtio and PCIe block demos | aarch64 |
| tour: `outlaw` | `lib.rs:2267`, `lib.rs:1328` | `outlaw` | aarch64, riscv64 |
| tour: `memory_region_service::start` | `lib.rs:2304` | `memory_region_depleter` | aarch64 |
| tour: `riscv_least_authority_demo`, `riscv_uart_driver_demo` | `lib.rs:1419`, `lib.rs:1520` | demo programs | riscv64 |
| tour: `entropy_service::ensure` (MMIO, JH7110) | `lib.rs:1665`, `lib.rs:1721` | `entropy`, `jh7110_entropy` | riscv64 |

### Already granted, not started

The kernel already hands the progenitor raw device authority for the UART (slots 1-2), virtio-rng
(7-9) and virtio-net (13-15, milestone 590 (the booted system starts its network stack)). It does
the same for virtio-gpu and the virtio keyboard (12, 17-22, milestone 600 (the graphical terminal
stack is built in userspace)). The progenitor builds those services itself. That is the shape every
row above moves to.

### Test and bench only

`credential_service`, `compositor_service`, `disk_service`, `identity_provisioner_service`,
`login_service`, `ntp_service`, `raw_mode_service`, `rmle_service`, `keyboard_service::start`, the
rest of `display_service` and `fs_service`, and `holding`. Their module declarations in
`kernel/src/user.rs` (3006-3822) carry `allow(dead_code)` outside `system_tests`. The bench boots
reach `e1000e_service` (`network_bench.rs:48`), `designware_ethernet_service`
(`network_bench.rs:193`), `designware_mobile_storage_service` (`storage_bench.rs:296`),
`non_volatile_memory_express_service` (`disk_throughput.rs:43`) and `fs_service` (`bench.rs:1514`
on). This milestone leaves them where they are: a harness that wires a driver by hand is measuring
the driver, not the boot.

## Can the progenitor receive what each one needs?

| device authority | capability kind | exists? | needed by |
|---|---|---|---|
| virtio transport, interrupt, DMA run | `Virtio`, `Irq`, `PageFrame` run (§102) | yes, granted today for rng, net, gpu, keyboard | `block_driver` |
| file-service channel pages, a budget | `PageFrame` run, `MemoryRegion` | yes | `redoxfs_server` |
| one MMIO page | `DeviceFrame` | yes | RTC for `clock`, NVMe doorbell, e1000e's two queue pages |
| a DMA run for a PCIe device | `PageFrame` run | yes (unverified that the IOMMU binding survives a grant rather than a kernel-built mapping) | NVMe, e1000e, xHCI |
| several contiguous MMIO pages | `DeviceFrame` names one page | no | xHCI's register window |
| a firmware aperture | `PageFrame` run (`display_service.rs:121`) | yes | `framebuffer_driver` |
| "the CPU has a seed instruction" | none; a fact, not an object | no slot for a fact (`user.rs:3331`'s doc) | `entropy` in instruction mode |
| CMOS wall clock on `x86_64` | none; ports stay kernel-resident (§121 (what a device capability is when the device has no page: x86 port I/O)) | the reading can travel in the clock page | `clock` |

Two gaps are real. A multi-page device window has no capability, and the progenitor's capability
table is full: all 32 slots are named in `components/src/progenitor.rs`'s `GRANTS`. Moving the rows
frees seven (5, 10, 11, 16, 27, 29, 30) and needs about eleven. Fork 2 is that question.

The progenitor can already map a capability into a child without the child holding it. So the
confinement each kernel starter argues for (the driver holds no name for its window, so it cannot
delegate it) survives the move. The supervisor holds the name, which is also what lets it re-map the
window into a restarted driver.

## The work, in order

1. Supervision on a real boot (fork 3). Today nothing starts `root_supervisor`, `spawner` or
   `sub_server_supervisor` on any boot; only `system_tests/src/user/authority_tests.rs` does
   (recorded by PR #1796's proposal, the init package holds only what init does). Without this,
   moving a starter changes who built it and not whether it can restart.
2. `block_driver` and `redoxfs_server` on the virtio path. Every capability exists. The file
   endpoint is created by the supervisor, not the server, so a client's capability outlives a
   restart (milestone 23 (live replacement)'s "a supervisor restores the service with no authority
   it did not already hold"). `fs_service::start_crash` and `recover_crash` (`fs_service.rs:721`,
   `:773`) already prove a respawned server recovers the same disk. Slots 5 and 6 become the device
   trio.
3. The NVMe arm, then `install_service::confirm`. Moving confirm into the progenitor retires the
   foot gun its own header names (`install_service.rs:325`): the single-user window now holds
   because one process sequences both, not because the kernel is single-threaded.
4. `clock`: the RTC page as a `DeviceFrame`; on `x86_64` the kernel writes its CMOS reading (§130
   (how the kernel-resident CMOS RTC reaches the userspace clock service)) into the clock page
   before the grant.
5. `entropy` in instruction mode: the fact travels in the inert-configuration page (§111 (inert
   configuration is a read-only page)) or the boot-information page of fork 2, and the kernel stops
   building the service.
6. `net_stack` over `e1000e`, then over the GMAC once `PROVEN_ON_SILICON` is true.
7. The screen terminal.
8. The xHCI keyboard, last, because it needs the multi-page device window.
9. `install_service::offer`, and the tour (fork 1).

Drivers and the file server go first because they are what §159 and #1805 need restartable. A
display or a keyboard that dies is visible to the person at the machine. A file server that dies
takes every program with it.

## Parity

All three architectures move together, per §19 (architectural parity), and the slot layout stays
identical (milestone 166 (one boot loader, reached two inconsistent ways)). aarch64 and riscv64 take
the virtio file-service path under QEMU. `x86_64` takes it under QEMU and the NVMe path on xenon.
riscv64's GMAC row is inert until radon proves it. aarch64 has no silicon here until argon, so its
exit is QEMU's. The tour rows differ per architecture today, and fork 1 decides whether that
difference survives.

## Dependencies

- PR #1796's finding: no real boot starts the supervision tree. Work item 1 closes it.
- Milestone 23 (live replacement), PARTIAL. Restart is this milestone; live swap of a driver is 23's
  and is not required for the exit.
- PR #1805's proposal (lab machines update themselves) lists "the kernel still starts `block_driver`
  and `redoxfs_server` itself" as a gap. This milestone is that gap.
- The kernel's capability minting: fork 2.
- §26 (the fault endpoint: thread death becomes a message) is how a supervisor learns of a death.
  §32 (a supervisor may collect a corpse without being able to build one) is how it collects one.
  §26 also says the kernel never relaunches anything, which is why the kernel cannot be the
  restarter.

## Forks for an architect

### Fork 1: does the kernel keep any starter?

- A. None on any default build. The tour's demonstrations move behind a non-default `tour` feature,
  `x86_userspace_demo` with them, and the install offer moves into the progenitor. The offer's
  argument (ask before the authority to wipe exists) holds there too, because the progenitor holds
  the disk capabilities and need not delegate them until a person answers.
- B. The tour stays on the default build; the exit is proved on the `shell` build only.
- C. The install offer stays in the kernel, because it asks on the console before the hand-over.

Prior art (recalled, not reread): seL4's kernel starts only the root task and hands it every
capability. Genode's core starts only `init`. Milestone 267 (the milestone tour is three things
wearing one name) already left open whether the tour's console server earns its place.

Recommendation: A. A boot whose kernel starts demonstrations is not the boot a customer runs, and B
proves the claim on a build nobody ships. The tour's entries need kernel privilege, which is why
they are demonstrations, and a feature keeps them.

### Fork 2: how device authority reaches the progenitor

- A. Fixed slots, as today. `DeviceFrame` gains a page count, as §102 (a frame names a run of
  pages) gave `PageFrame` one, and `CAPABILITY_TABLE_SLOTS` grows to 64 (calef raised it from 16
  to 32 on 2026-09-27).
- B. seL4's shape: the kernel grants device regions plus a read-only boot-information page naming
  each device's kind, slot and interrupt. The progenitor reads the page instead of knowing slot
  numbers.
- C. A kernel-served endpoint the progenitor asks for a device by kind. It puts a protocol server in
  the kernel, which the narrow syscall surface argues against.

Prior art (recalled): seL4's `BootInfo` frame lists untyped and device-untyped regions. Genode's
platform driver hands out MMIO and IRQ sessions. Fuchsia's board driver passes resources to the
driver framework.

Recommendation: A for this milestone. It is additive, it is what every existing grant does, and
about ten devices fit. B is the better shape once devices are counted in dozens, because a slot
number per device stops scaling; it is a new boot ABI and should be its own milestone. Would I
choose A if both cost the same? For ten devices yes, on fewer moving parts. Past that, no.

### Fork 3: which process is a driver's supervisor?

- A. The progenitor itself (`system_initializer`) holds a fault endpoint on what it builds and
  restarts it. Genode's `init` is the parent that restarts (recalled).
- B. The progenitor starts `root_supervisor`, whose `spawner` builds one program and whose
  `sub_server_supervisor` holds no memory and restarts it (milestone 22 (trusted init)'s B.2 tree).
  MINIX 3's reincarnation server is the analogue (recalled).
- C. A driver manager per class, as Fuchsia's driver framework (recalled).

Recommendation: B. It is the tree milestone 22 (trusted init) built and #1796 kept in `init` for
this job, and its restarter holds the least authority. A puts restart in the process that already
holds everything. A is less work, and that is the only argument for it.

## Exit

A test proves both, on aarch64, riscv64 and `x86_64` under QEMU, on the default build and the
`shell` build:

1. The kernel spawns only the progenitor. The kernel counts every user process it builds (the
   `run`/`run_with` path, `user.rs:1648`) and prints the count at the hand-over;
   `script/swish-check` requires it to read 1 at the prompt.
2. `redoxfs_server` dies and its supervisor restarts it. The test arms the server to fault (the
   arming `fs_service::start_crash` uses, carried by the progenitor's spawn). The transcript shows
   the death and the restart, and a `cat` at the prompt afterwards succeeds through the shell's
   original capability.

**Reuse:** the supervision tree is `root_supervisor`, `spawner` and `sub_server_supervisor`
unchanged in shape. The restart test reuses `fs_service`'s crash arming and its recovery proof.
Nothing is written that the tree already has.

## BUGS

- The NVMe admin plane stays in the kernel by §86 (whether an NVMe driver can leave the kernel), so
  a restarted NVMe server inherits whatever state a crashed one left the I/O queue pair in. Whether
  the kernel must reset the pair before a restart is unmeasured.
- A restart loses requests in flight. A client sees an error, not a retry; that is milestone 23's
  contract to settle, not this one's.

## Index row

The kernel stops starting the file server, the disk driver and a dozen other services on a real
boot, and hands the device capabilities to the progenitor instead. Everything userspace runs then
has a supervisor, so the file server can die and come back without a reboot, which is what §159's
updates without a reboot need first. It is on the customer path.