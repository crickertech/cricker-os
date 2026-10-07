---
status: NOT-STARTED
raised: 2026-09-29
promoted_from: an-e1000-the-first-nic-that-is-not-virtio
milestone_dependencies: 421
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 619. An e1000: the first NIC that is not virtio

`net_stack` speaks virtio-net through the virtio service, both transports (mmio and PCI). That is
every NIC the tree can name. e1000 is QEMU's default x86_64 NIC, a device whose shape virtio did
not define. It is also the first test of whether the network stack's device seam is a seam at
all: a second driver behind the roster shape the disks already have (milestone 421 (the block
roster and an NVMe disk)).

## Done means

The boot matrix runs a non-virtio NIC row on `q35`; `net_stack`'s tests pass unchanged above the
new device; the roster names the e1000 beside the virtio devices it already lists.

Name provisional.

## Index row

Every NIC the tree can name is virtio, so the network stack's device seam has never met a device
virtio did not define. This milestone puts an e1000 behind that seam on `q35`: the stack's tests
pass unchanged above a second driver, and the roster names the new device beside the virtio ones.
QEMU's default x86_64 NIC stops being unreachable.
