---
status: NOT-STARTED
raised: 2026-09-29
promoted_from: a-virtio-scsi-driver-parked-behind-its-trigger
milestone_dependencies: 421
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 620. A virtio-scsi driver, parked behind its trigger

The block layer names NVMe (milestone 421 (the block roster and an NVMe disk)) and virtio-blk.
virtio-scsi would be a third way to say disk, and nothing in the tree, in QEMU's defaults, or in
the physical lab's plans asks for it yet.

This milestone exists so the cut device row has a home rather than a shrug. The trigger that
builds it is a real scsi device becoming compatibility-relevant: a physical lab machine with
scsi storage, or a boot-matrix configuration virtio-blk cannot express. Until the trigger fires
it is deliberately unbuilt, and this file is the record of that decision.

Name provisional.

## Index row

A third way to say disk, parked deliberately unbuilt. The block layer names NVMe and virtio-blk,
and nothing in the tree, in QEMU's defaults, or in the lab's plans asks for scsi yet. This block is
the record of that decision, and the trigger is a real scsi device becoming compatibility-relevant.
