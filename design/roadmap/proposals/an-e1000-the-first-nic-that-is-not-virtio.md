---
status: PROPOSED
raised: 2026-09-29
milestone_dependencies: 421
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# An e1000: the first NIC that is not virtio

`net_stack` speaks virtio-net through the virtio service, both transports (mmio and PCI). That is
every NIC the tree can name. e1000 is QEMU's default x86_64 NIC, a device whose shape virtio did
not define, and the first test of whether the network stack's device seam is a seam at all: a
second driver behind the roster shape the disks already have (milestone 421, the block roster and
an NVMe disk).

## Done means

The boot matrix runs a non-virtio NIC row on `q35`; `net_stack`'s tests pass unchanged above the
new device; the roster names the e1000 beside the virtio devices it already lists.

Name provisional.
