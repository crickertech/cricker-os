---
status: NOT-STARTED
raised: 2026-09-21
promoted_from: a-coremark-score-on-three-architectures
milestone_dependencies: none
decision_dependencies: none
machine_requirements: aarch64, riscv64 and x86_64 silicon
specific_machine: none
needs_person: yes
---
# 558. A CoreMark score on three architectures, now that the rate is the machine's

The number is **provisional**: the integrator mints it at merge. Promoted from the proposal `a-coremark-score-on-three-architectures` on 2026-09-22, filed 2026-09-21. Raised by the `cntfrq` lane as work its own change unblocked:
*"a real CoreMark score is newly trustworthy on all three architectures ... that is a number nobody
is currently producing."*

A CoreMark number under QEMU is fiction, so this wants argon, radon and xenon.

## Why it was not trustworthy before

`coremark` sends its own `cntfrq` rather than taking the kernel's, and on riscv64 that was a
hardcoded 10 MHz against radon's 4 MHz. No score was ever published from it, which is the only
reason this is an opportunity rather than a correction. After the riscv64 timebase work the program's
self-reported rate is the machine's on every architecture.

## Why it is worth taking

CoreMark is the one benchmark in this tree that an outsider already has numbers for. Every other
comparison this project publishes has to explain its own methodology first; this one lands in a
column somebody else has already filled in for hundreds of parts. That makes it unusually cheap
evidence and unusually easy to get wrong in public, which is the same property.

## What it must carry, because a published score is a fact that leaves the machine

- Which build. `notes/benchmarks.md`'s standing caveat is that the bench card's kernel differs
  from the shipping one; a score that does not say which it came from is not reproducible.
- Iterations, compiler, and flags, because CoreMark's published scores are meaningless without
  them and a reader who knows the benchmark will ask first.
- The rate and where it came from, which is the whole reason this is newly possible.
- An honest comparison class. A score beside a vendor's number for the same part is a statement
  about our toolchain and our scheduler, not about the microkernel's design, and the note has to say
  so before anybody quotes it as the latter.

## BUGS

- A single-threaded integer benchmark says almost nothing about a capability microkernel. It
  exercises none of what this project is a demonstration of. Its value is as a sanity check that the
  userspace runtime is not pathologically slow, and as a number strangers can locate. Claiming more
  would be the overclaim this tree's benchmark posture exists to prevent.

## Index row

The one benchmark here that strangers already have numbers for became trustworthy when userspace
stopped hardcoding its clock; it needs three boards, a stated build, and a note that says what a
single-threaded integer score does not measure.
