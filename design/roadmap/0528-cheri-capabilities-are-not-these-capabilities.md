---
status: NOT-STARTED
raised: 2026-09-20
promoted_from: cheri-capabilities-are-not-these-capabilities
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 528. CHERI's capabilities are not this kernel's capabilities, and the tree never says so

*(Number provisional until the merge queue lands it.)* Promoted from the proposal `cheri-capabilities-are-not-these-capabilities`, filed 2026-09-20, on calef's instruction of 2026-09-20 to give every proposal on `main` a number. The text below is the proposal's own, unedited except for this paragraph: the argument is its author's and promotion is not the moment to improve it. Filed by `maintainer/riscv-summit-research` from the RISC-V Summit
Europe 2026 reading (`notes/riscv-summit-2026.md`). *(Number and slug provisional until the merge
queue lands it.)*

The work is one note in `notes/` and a cross-reference from `notes/acronyms.md`,
both of which a lane can start today against a published specification. One sentence inside it is
calef's, and the milestone should stop at that sentence rather than write it: any claim about how
nife stands relative to a hardware capability ISA is a positioning claim, and AGENTS.md puts *facts
that leave the machine* in the irreversible category. Write the mechanics; leave the comparison for
ratification.

## What happened

CHERI is no longer being proposed as a RISC-V extension. It is a new base ISA family. Krste
Asanović's State of the Union, slide 12, *"RISC-V New Security Extensions in Progress"*
(https://riscv-europe.org/summit/2026/presentations#P-N9KRDZ):

> CHERI
> - New base ISAs (RV32Y/RV64Y) bringing capabilities to RISC-V

Checked away from the talk: the specification is `v0.9.10-draft-a3b39b6, 20260918`, marked
**"DRAFT---NOT AN OFFICIAL RELEASE"** and *"in the Stable state"*, defining `RV32Y`/`RV64Y` with
`RV64LYA` as the 64-bit base plus its capability-encoding format (https://riscv.github.io/riscv-cheri/).
**Not ratified and not shipping.**

Separately, Tariq Kurd argued the commercial forcing function
(https://riscv-europe.org/summit/2026/presentations#P-PC8KYU, abstract only): the EU Cyber
Resilience Act *"is fully enforced in for all products 'with a digital element' sold in the EU from
December 2027"*, and *"CHERI systems have memory safety bu[i]lt-in which resolves 70% of vulnerabilities
seen in weaker non-CHERI legacy systems."* The 70% is the speaker's figure and is unsourced on the
page.

## Why it is worth writing something

Because the word is about to become ambiguous, and this project spent its thesis on it.
DECISIONS §14 (the project's direction: a verified-Rust capability microkernel that runs real
workloads) uses "capability" to mean an unforgeable kernel-held reference that names an object and
carries rights, attenuable and revocable. CHERI uses it to mean a hardware-tagged fat pointer
carrying bounds and permissions over an address space. These are different objects with the same
name, and an argument that one substitutes for the other would be wrong in both directions: CHERI
does not name a thread or a page of untyped memory, and nife's capabilities do not bound a `memcpy`.

Three of this project's own principles say to fix that here rather than in a conversation:

- A newcomer must be able to succeed without asking anyone. The reader who arrives in 2028
  having heard "RISC-V has capabilities now" will be holding the wrong model of this kernel, and
  nothing in the tree corrects it.
- A name is a claim, and the reader meets it first. This is the naming discipline applied to a
  word the project did not mint and cannot control.
- The refusals are the valuable half. The honest version of this note includes what CHERI would
  and would not do for nife if it shipped, including the uncomfortable half: a CHERI machine gives
  intra-process memory safety that a Rust microkernel's type system already gives for Rust programs
  and does not give for anything else, and this tree runs foreign binaries (fatal risk 1's
  `ripgrep`).

## What the work is

One note, roughly four sections, and none of it is speculative design:

- What a CHERI capability is, read from the specification rather than recalled, with the
  encoding and the tag bit named.
- What a nife capability is, cross-referenced to the existing notes rather than restated.
- The table of what each one can and cannot express, which is where the value is.
- **A `BUGS` section** saying plainly that RV64Y is a draft base ISA with no silicon, so nothing in
  the note is a plan.

Then one line in `notes/acronyms.md`, because CHERI will be the acronym a reader meets first.

## What would make this not worth doing

If calef reads the summit note and judges that the ambiguity is not going to reach this project's
readers, close it. It costs one lane and it is documentation, so the downside of skipping it is
deferred rather than compounding, which is exactly the kind of thing that should lose to a milestone
on the customer path if one appears.

## Index row

CHERI is no longer being proposed as a RISC-V extension.
