---
status: BUILT
raised: 2026-09-18
built: 2026-09-25
promoted_from: the-riscv-iommu-driver-has-no-proof
---
# 432. The RISC-V IOMMU driver has no counterpart to the SMMU's proofs

Two harnesses in `kernel/src/arch/riscv64/iommu.rs`, both
falsified, proved from an aarch64 host; "What was built" below. Promoted from the proposal
`the-riscv-iommu-driver-has-no-proof`, filed 2026-09-18 from the confirmation pass of milestone 322
(one machine matrix for three architectures), which went looking for an unexercised IOMMU
configuration, found that all three architectures already boot behind one, and turned up this
instead. *(Number provisional until the merge queue lands it.)*

Premise re-checked 2026-09-19 and still true, by count.
`kernel/src/arch/aarch64/iommu.rs` carries two `kani::proof` harnesses and
`kernel/src/arch/riscv64/iommu.rs` carries zero. `notes/iommu.md`'s "What is proved, as against
tested" still says the RISC-V IOMMU has no counterpart, and still records that the register offsets
and bit constants are not proved and cannot be. This is the newest block in milestone 433's promotion
and the only one of the twenty-five whose premise needed no qualification at all.

In brief. `notes/iommu.md`, "What is proved, as against tested", names it plainly: **the RISC-V
IOMMU has no counterpart** to the aarch64 proofs. `arch/aarch64/iommu.rs` carries two Kani harnesses
over its entry-building arithmetic, both falsified before they were believed
(`the_smmu_is_handed_exactly_the_tables_the_kernel_built` and
`no_stream_can_reach_another_streams_tables`, the latter replayable in
`kernel/falsifications/`). The RISC-V side writes its device context in 64-bit stores with no split,
so that first property does not apply to it, **and nothing was written in its place**.

## Why the asymmetry is not benign

The boot-time confinement test is what stands in for a proof on both sides, and the same note says
what it cannot do: it *"proves the hardware stops an escaping device. It cannot prove the kernel
wrote the right entry, because a wrong entry that still confines this device on this board is
invisible to it, and this board is the only one either driver has ever run on."*

So on riscv64 that test is the whole of the assurance, with no proof beside it, on exactly one
board. §19 makes architectural parity a gate rather than an aspiration, and this is a parity gap in
the isolation boundary that milestone 35 calls *"the one isolation boundary we test instead of
prove"*.

## What it is not

Not "port the aarch64 harnesses". The property they prove is about a 64-bit physical address
surviving a split across two 32-bit words without colliding with the control bits sharing the low
word. The RISC-V device context has no such split, so a transliteration would prove a hazard that
does not exist there and miss whatever the real one is. Finding the right property is most of this
work, and writing it down is the deliverable even if the harness that follows is short.

Not a claim that the riscv64 driver is wrong. Nothing here reports a defect. It reports that one
of two rhyming drivers is checked in a way the other is not, which is the asymmetry §19 exists to
catch and the kind this tree prefers to find by looking rather than by failing.

## One limit to carry over rather than discover again

`notes/iommu.md` records that the register offsets and bit constants **are not proved and cannot
be**: nothing in this tree can check a constant against Arm IHI 0070, or against the RISC-V IOMMU
specification, so a misreading of the document makes the code and the proof wrong together. Whatever
is written here inherits that, and should say so where a reader meets it rather than implying a
completeness it cannot have.

## What was built (2026-09-25, UTC)

The block's original gate, NONE, rested on a premise that was half wrong, and the fix is how
this got built. `script/verify` reaches `kernel/src` since milestone 193 (put `kernel/src` within
reach of the prover), but only the host's `arch/` subtree, and no host here is riscv64, so these harnesses could not run where the gate said. The `lane/price-kani-kernel-reach`
lane (pull request #1276) measured that `iommu.rs` compiles unchanged on an aarch64 host anyway, and
its proposal's option 1 is what this milestone built:
`design/roadmap/650-riscv64-code-the-prover-can-already-compile.md`.

- A proof-only module in `kernel/src/arch/mod.rs`, `cfg(all(kani, target_arch = "aarch64"))`,
  holding `mod riscv64 { mod iommu; }`. No `#[path]`, and the harness paths are the native ones.
- `attach`'s arithmetic lifted into four pure functions in the same file (`device_context`,
  `is_in_directory`, `context_offset`, `iodir_inval_ddt`). The words written, their order and the
  barriers are unchanged; `cargo clippy -p kernel --target riscv64imac-unknown-none-elf` is clean.
- The properties, which this block said were most of the work: the device context names exactly
  the domain built and its MODE is Sv39 for every root (MODE 0 is Bare, translation off); and the
  directory bound agrees with the stride in both context formats, with the invalidation naming the
  same device. notes/kernel-proofs.md has them in full.
- Falsified, both replayable on an aarch64 host from `kernel/falsifications/arch.riscv64.*`.
- The caveat, as a gate. In that module `crate::arch` is aarch64's. notes/kernel-proofs.md
  stub-list item 8 says so, and `script/lint` check 5b fails when the file names a `crate::arch`
  path it has not recorded.

`cargo kani -p kernel -Z unstable-options --ignore-global-asm` on patagonia: 6 of 6 verified,
20.4 s wall including the compile, 444 MB peak resident.

## Follow-on

- **Milestone 650.** Milestone 650 (some of `arch/riscv64/` compiles under Kani on an aarch64 host today, and the tree says none can). `design/roadmap/650-riscv64-code-the-prover-can-already-compile.md`.
  Extending the proof-only module past `iommu.rs` (`context.rs` compiles today, `irq.rs` has eight
  resolve errors) and the two larger options, `asm!` wrappers and a native riscv64 Kani, stay
  there with their costs. About 91% of `arch/riscv64/` is still reached by nothing (564 of 6,083
  lines; 94% was from a doubled total, corrected 2026-09-25).
- **Recorded.** `notes/kernel-proofs/riscv64-from-an-aarch64-host.md`, linked from
  notes/kernel-proofs.md's stub-list item 8. In the proof-only module `crate::arch` is aarch64's,
  so only code that never calls through it is proved, and `script/lint` check 5b is its gate. Its
  BUGS section records what stays unreached: `init`, `cmd_push`, `take_fault`, the write order in
  `attach`, and the constants against the specification.
- **Milestone 536.** `design/fatal-risks/README.md` risk 2's sentence about riscv64 reach is now narrower
  than the record says, and that file is calef's. Milestone 536 (two records still say the prover
  cannot see `kernel/src`) already holds the correction of that record.

## Index row

`arch/aarch64/iommu.rs` carries two Kani harnesses over its entry-building arithmetic, both
falsified before they were believed, one of them replayable in `kernel/falsifications/`. The RISC-V
driver writes its device context in 64-bit stores with no split, so the property those prove does
not apply to it, and **nothing was written in its place**: the boot-time confinement test is the
whole of the assurance on that side, on exactly one board, and the same note says what that test
cannot do, which is prove the kernel wrote the right entry, since a wrong entry that still confines
this device on this board is invisible to it. §19 makes architectural parity a gate rather than an
aspiration and this is a parity gap in the boundary milestone 35 calls the one isolation boundary we
test instead of prove. The work is not porting the aarch64 harnesses, which would prove a hazard
RISC-V does not have; finding the right property is most of it, and writing it down is the
deliverable even if the harness that follows is short.
