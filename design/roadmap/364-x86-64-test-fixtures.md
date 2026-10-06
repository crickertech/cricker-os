---
status: SUPERSEDED
raised: 2026-09-03
superseded_by: 303, 420
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 364. Most x86_64 tests take a "no RedoxFS disk attached" arm, because the runner attaches little

Superseded 2026-09-19, in two parts. The first piece this file names, making the FS
server's disk lookup transport-blind and attaching the RedoxFS image, was built by milestone 303 on
2026-09-16. The rest was restated by the milestone 303 lane the same day as the proposal
`the-rest-of-the-x86-64-fixture-set`, which milestone 433 numbers 420, with the device list
itemised and the structural obstacle recorded as gone. Filed here on 2026-09-03 by the milestone 247
sweep, from milestone 215's block. Checked on 2026-09-19: `helpers/qemu-runner-x86_64.sh` now
attaches two `virtio-blk-pci` functions and an NVMe controller and still no NIC, GPU, keyboard or
RNG, and the tree carries 50 `skip!("no RedoxFS disk attached")` sites rather than the 36 this file
counted, which is the measure growing rather than the gap closing. The work is real and it lives in
the newer block.

The blocker is gone. Milestone 215 made a PCI function's interrupt reach a userspace
driver on x86_64, and it did the whole thing on patagonia under QEMU's `q35`, so nothing here waits
on xenon.

**In brief.** `helpers/qemu-runner-x86_64.sh` starts a much barer machine than the aarch64 and
riscv64 runners do. The RedoxFS image, the GPT and blank disks, the NIC, the GPU, the keyboard and
the RNG each need a line in that script plus the wiring behind it. The first piece is making the FS
server's disk lookup transport-blind, since it currently assumes the transport the other two
architectures use. The measure of done is the count of tests taking a "no RedoxFS disk attached"
arm, which is 36 today.

## Why this matters

Architectural parity is a gate rather than an aspiration (DECISIONS §19): a kernel capability ships
on every supported architecture, proven by the same suite, or a scope note records the gap.
Thirty-six tests that pass by taking an early-exit arm are the worst version of that, because they
are green. The suite reports a passing x86_64 leg while a third of the interesting surface is
untested there, and nothing in the output distinguishes "this works on x86_64" from "this was
skipped on x86_64".

The specific risk is a defect that only x86_64 has and that only the fixtures would find. Milestone
215 is the existence proof: PCI interrupt routing reached nothing on x86_64, no userspace driver
could run there, and the suite was green the entire time because nothing on that leg ever asked a
device for an interrupt. Every fixture still unattached is another arm of that same blind spot.

## Where it came from

Milestone 215 (a PCI function's interrupt reaches nothing on x86_64) named it as the natural next
step from what it built: *"Attach the rest of the x86_64 test fixtures now that a function's
interrupt works: the RedoxFS image, the GPT and blank disks, the NIC, the GPU, the keyboard and the
RNG, each a line in `helpers/qemu-runner-x86_64.sh` plus its wiring, starting with making the FS
server's disk lookup transport-blind. The measure is the 36 tests taking a 'no RedoxFS disk
attached' arm."*

## Index row

`helpers/qemu-runner-x86_64.sh` started a much barer machine than the other two runners, so most
x86_64 tests passed by taking an early-exit arm and the suite reported a green leg over an untested
surface. Milestone 303 took the first piece named here on 2026-09-16 (the transport-blind disk
lookup and the RedoxFS image) and the milestone 303 lane restated the remainder the same day with
the device list itemized, so this block is the older half of a pair. Promoted and disposed of in one
act by milestone 433. The measure it proposed is worth keeping: the count of tests taking a "no
RedoxFS disk attached" arm, which was 36 when this was written and is 50 now.
