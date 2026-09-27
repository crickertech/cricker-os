---
status: PROPOSED
raised: 2026-09-27
milestone_dependencies: 609
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# On x86_64, a registered device's first DMA arrives before its IOMMU context

Found by milestone 609 (the system tests leave the kernel crate), once the kernel's unit tests boot
without the system suite. `virtio::tests::the_iommu_faults_a_dma_that_escapes_the_domain` fails on
x86_64 alone (`script/test --arch x86_64 --test the_iommu_faults`), so this is not about test order.
Until milestone 609 the system suite always attached this disk earlier in the same boot, which hid
it. The test now skips on x86_64 with that reason; the observed sequence is in the BUGS section of
`kernel/src/arch/x86_64/iommu.rs`.

In short: after registration, the disk (rid `0x20`) makes one DMA to a frame registration mapped,
and VT-d faults it with reason `0x2`, context entry not present. The escape the test provokes then
faults correctly, as a second-level permission error, so by then the context exists. The unit has
one fault record, so the second fault is dropped and the test reads the first.

The plan, in order:

1. Log, on the kernel side, the context-entry write, the context-cache and IOTLB invalidations, and
   the device's status writes in `provoke_iommu_escape`, each with the instruction count, and read
   them against QEMU's `-d` VT-d trace for the fault. That says whether the device DMAs before the
   entry is written, or after it is written but before QEMU sees it.
2. Name which access `0xffdb000` is (descriptor table on the shadow page, or the used ring) from
   `register`'s own addresses in the same run.
3. Fix whichever it is. Then un-skip the test, and the aarch64 and riscv64 legs keep proving it.

It matters beyond the test. A device that can DMA before its domain is in place is, for that one
access, unconfined, which is the property the x86_64 PCIe confinement claim rests on.
