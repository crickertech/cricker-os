---
status: NOT-STARTED
raised: 2026-09-16
promoted_from: the-rest-of-the-x86-64-fixture-set
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 420. x86_64 has one fixture of five and no NIC, GPU, keyboard or RNG, so its skip count is a device list

Promoted from the proposal `the-rest-of-the-x86-64-fixture-set`, filed
2026-09-16 by the milestone 303 lane (x86_64's FS disk), which attached the RedoxFS image and
deliberately stopped there so that one milestone proved one thing. Milestone 215's block listed the
same set without numbering it. *(Number provisional until the merge queue lands it.)*

Every device named here is one QEMU emulates on `q35`, and milestone 303 removed the
structural obstacle: `virtio::find_block_device_n` spans virtio-mmio and virtio-pci, so a wiring on
a machine with no mmio bus can find its disk.

**Premise re-checked 2026-09-19 and still true, device by device.**
`helpers/qemu-runner-x86_64.sh` attaches two `virtio-blk-pci` functions and one `nvme` controller
and nothing else: no `virtio-net-pci`, no `virtio-gpu-pci`, no `virtio-keyboard-pci`, no
`virtio-rng-pci`, and no crash, GPT or blank disk.

What the work is. `helpers/qemu-runner-x86_64.sh` attaches two `virtio-blk-pci` functions (the
nifefs image and the RedoxFS image) and an NVMe controller, and nothing else. Missing, each with a
test in the tree that skips for want of it: milestone 37's crash disk, milestone 57's GPT and blank
disks, a `virtio-net-pci` NIC, a `virtio-gpu-pci`, a `virtio-keyboard-pci` and a `virtio-rng-pci`.
Each is one `-device` line, the matching `mk*disk` call in `cargo xtask test`'s x86_64 leg, and, for
the three disks, nothing else at all, because the lookup already counts across both buses.

What it would settle. How much of this port's skip count is a machine fact and how much is a
runner that was never asked. It is also the honest answer to DECISIONS §19 for a family of
capabilities currently green on two architectures because the third has no device, not because the
third cannot drive one.

**Why it was not done in 303.** Scope: that milestone's subject was one disk and one transcript, and
bundling six more devices into it would have made a green suite ambiguous about which change bought
which test. The display half in particular is not merely a `-device` line: milestone 161's own note
records that `console`, `input` and `keyboard_driver` want work beyond attaching hardware.

Recorded in the meantime where a reader meets the gap:
`design/roadmap/0303-x86-64-fs-disk.md`'s `BUGS`, and
`design/roadmap/0215-x86-64-pci-interrupt-routing.md`, which listed the set first.

## Index row

`helpers/qemu-runner-x86_64.sh` attaches the nifefs image, the RedoxFS image and an NVMe controller,
and nothing else, so milestone 37's crash disk, milestone 57's GPT and blank disks, a NIC, a GPU, a
keyboard and an RNG are each a test in the tree that skips for want of a device QEMU emulates on
`q35`. Each is one `-device` line plus the matching `mk*disk` call in the x86_64 leg and, for the
three disks, nothing else, because milestone 303 removed the structural obstacle:
`virtio::find_block_device_n` spans virtio-mmio and virtio-pci, so a wiring on a machine with no
mmio bus finds its disk. What it settles is how much of this port's skip count is a machine fact and
how much is a runner nobody asked, which is the honest answer to §19 for a family of capabilities
green on two architectures because the third has no device. The display half is not merely a
`-device` line, and milestone 161's note says what else it wants.
