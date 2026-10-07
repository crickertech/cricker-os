---
status: BUILT
raised: 2026-09-17
built: 2026-09-17
---
# 314. The x86_64 ECAM second witness stops reading as a defect on real hardware

Built by a lane on `milestone/314-ecam-second-witness`.
*(Number provisional until the merge queue lands it.)*

On 2026-09-17 xenon, a Dell OptiPlex 7050, booted nife for the first time and printed:

```
pcie ecam 0xf0000000, buses 0..=127 (mmu::PCI_ECAM_PHYS says 0xb0000000)
```

A maintainer read that as a defect and carried it to calef as an open thread. It is not one.
Nothing was wrong on xenon, nothing was mapped at the wrong address, and the two numbers were never
supposed to agree on a machine QEMU did not build. The defect is in the sentence.

## What the line actually says

`PCI_ECAM_PHYS` is a second witness, never a source of truth, and its own doc has said so since
the window became ACPI-sourced. `pci.rs` reads `ECAM_BASE` from `memory::pci_regions()`, which
`main.rs` fills from the MCFG; the constant is `#[allow(dead_code)]` outside tests and
`map_everything` has not mapped it for some time. Printing it beside the discovered base is a
cross-check, and on xenon the cross-check ran and reported exactly what it should have.

## Why it was misreadable, and it is one word

The same sentence means two different things depending on the architecture, and the wording only
matched one of them.

- aarch64 and riscv64 boot QEMU `virt` and nothing else, so there the second witness is an
  equality invariant: a difference would be a finding.
- x86_64 runs on machines QEMU did not build. Every one of them sizes and places its own ECAM
  window, so off q35 the two values differ permanently and correctly.

"`mmu::PCI_ECAM_PHYS` says `0xb0000000`" names *our* constant, which invites the reading that our
constant is the expectation and the machine disagreed with it. Naming the machine the number belongs
to says what is being compared instead. The new line:

```
                pcie ecam 0xf0000000, buses 0..=127 (q35's default is 0xb0000000)
```

and under QEMU with SeaBIOS, where the comparison does bind:

```
                pcie ecam 0xb0000000, buses 0..=255 (q35's default is 0xb0000000)
```

A reader on xenon now sees a fact about q35 next to a fact about their machine, rather than a
disagreement. The comment beside the `println!` says the rest, and the constant's doc gained a short
paragraph on the asymmetry, because the boot log is not where a reader can discover that two of
three architectures hold this as an invariant and the third cannot.

### The line was already lying, in our own gate, and nobody had noticed

This is the part that turns a wording fix from tidy into worth doing, and it was found by running
`script/test --arch x86_64` to check the quoted output rather than by reasoning about it.
`script/test --arch x86_64` boots three times: once on SeaBIOS and twice under OVMF (milestone 205's
real-firmware runs). **OVMF relocates the ECAM window**, and both of its boots print:

```
                pcie ecam 0xe0000000, buses 0..=255 (q35's default is 0xb0000000)
```

So two of every three x86_64 boots in the merge gate have been printing an apparent discrepancy
since the real-firmware runs were added, on the same QEMU q35 machine, with nothing wrong. xenon was
not the first machine to trip this reading; it was the first one anybody read the log of closely
enough to care. That also disposes of the tempting narrower fix, which would have been to keep the
constant's name in the line and call the difference a hardware-only case. It is not hardware-only.

## The equality test the doc claimed does exist

`PCI_ECAM_PHYS`'s doc asserts that aarch64 and riscv64 hold their old hardcodes "equal to the
discovered value by `pci.rs`'s own test", and this lane checked rather than trusted it, because a doc
citing a test that is not there is the shape this tree has been finding all week.

It is there.
`pci.rs`'s `the_discovered_pci_windows_are_the_machines_own_and_match_the_old_constants` parses the
live DTB, asserts the recorded window is the tree's, and then asserts the discovered pair against the
retired hardcodes per architecture: `(0x3000_0000, 0x1000_0000)` on riscv64 and
`(0x40_1000_0000, 0x1000_0000)` on aarch64, each tagged `"was PCI_ECAM_BASE"`. It is
`#[cfg(not(target_arch = "x86_64"))]`, and it skips rather than panics where `pci_regions()` is
`None` (the JH7110, which has no generic-ECAM node at all).

One wrinkle worth recording, since the doc's wording does not quite survive contact. It says the
other two architectures "also kept" a second witness. They kept the **value**, as literals inside
that test; they did not keep a live `const`. `PCI_ECAM_BASE` is gone from both
`arch/aarch64/mmu.rs` and `arch/riscv64/mmu.rs`, surviving only in prose explaining why it went
(DECISIONS §43, the first VisionFive 2 boot). x86_64 is the only architecture with a second witness a
reader can `grep` for as a constant, which is not wrong, and is part of why its printout carried more
authority than it was entitled to.

## BUGS

- Nothing checks the two arms stay consistent. A future architecture that hardcodes an ECAM base
  and prints it beside a discovered one can reintroduce the identical ambiguity, and no gate can tell
  a misleading diagnostic string from a clear one.
- **The QEMU lines above were read off a local `script/test --arch x86_64` on this lane's worktree**,
  not off a merge. The `buses 0..=255` half is a live value, not a promise, and OVMF's `0xe0000000`
  is that firmware's placement rather than anything nife chose.
- `PCI_ECAM_PHYS` is still `#[allow(dead_code)]` outside tests. That is the correct shape for a
  witness nothing consumes, and it also means a typo in it would be caught by nothing but the
  comparison this milestone just made harder to misread as an alarm.

## Follow-on

- **Done.** The reported thread is closed: no defect, no change to the constant, the mapping, or what
  is discovered. The one artifact that changes is the sentence a person reads on a real machine.
- **Recorded.** The constant-versus-literal asymmetry between x86_64 and the other two architectures
  is a `BUGS`-adjacent fact and now lives in `PCI_ECAM_PHYS`'s doc, where a reader meets it. Nobody
  is asked to reconcile it; making aarch64 and riscv64 re-grow constants to match would reverse
  DECISIONS §43 for symmetry's sake, which is worse than the asymmetry.

## Index row

xenon's first boot printed `pcie ecam 0xf0000000, buses 0..=127 (mmu::PCI_ECAM_PHYS says
0xb0000000)` and a maintainer carried it to calef as a defect. It was not one: `PCI_ECAM_PHYS` is a
deliberate second witness, the real window comes from the MCFG through `memory::pci_regions()`, and
the cross-check was working. The defect was in the sentence, which named our own constant as though
it were the expectation. On aarch64 and riscv64, which only ever boot QEMU `virt`, the second witness
is an equality invariant and a difference would be a finding; on x86_64 it can only bind under
SeaBIOS on q35. The wording fix turned out to be worth more than a tidy: **two of the three x86_64
boots in the merge gate already print a different base**, because OVMF relocates the window to
`0xe0000000`, so the misleading line has been firing on every real-firmware run since milestone 205
added them and nobody read the log closely enough to notice. The line now reads
`(q35's default is 0xb0000000)`, naming the machine the number belongs to rather than implying a
disagreement, and the asymmetry is recorded in the constant's own doc. The equality test the doc
claims for the other two architectures was checked and does exist; what those architectures kept is
the value as a test literal, not a live `const`, which is part of why the x86_64 printout read with
more authority than it had.
