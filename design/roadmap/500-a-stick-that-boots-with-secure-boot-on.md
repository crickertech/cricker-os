---
status: NOT-STARTED
raised: 2026-09-19
promoted_from: a-stick-that-boots-with-secure-boot-on
milestone_dependencies: none
decision_dependencies: unwritten
machine_requirements: none
specific_machine: none
needs_person: no
---
# 500. A stick that boots with Secure Boot on, or a page that says how to turn it off

*(Number provisional until the merge queue lands it.)* Promoted from the
proposal `a-stick-that-boots-with-secure-boot-on`, filed 2026-09-19, on calef's instruction of
2026-09-20 to give every proposal on `main` a number. The text below is the proposal's own, unedited
except for this paragraph: the argument is its author's and promotion is not the moment to improve
it. Found by milestone 198 (a package manager, and the trivial install)'s rungs lane (`milestone/198-rungs-to-a-trivial-install`) while mapping
rung 1 of the trivial install DECISIONS §157 (a trivial install is a web page,) defines. §157's own measurement of the x86 row says
"Secure Boot off"; nothing owns what a stranger does about it.

Signing is a fact that leaves the machine (a key trusted by other people's
firmware), which is AGENTS.md's irreversible category. **Options, no winner.**

## The gap

`uefi_loader`'s first `BUGS` entry: *"Secure Boot must be off. This image is unsigned and nothing
here signs it. On the `OptiPlex` that is a firmware setting ... Signing is milestone 22's
neighbourhood (`measured_boot`), not this one."* Milestone 22 is about measuring what init loads,
not about firmware trust. `git grep -i 'secure boot'` over `design/` and `notes/` finds the
instruction to turn it off in five places (`notes/x86-uefi-boot.md`, `notes/serial-less-output.md`,
`notes/xenon-firmware.md` among them, and xenon's setting photographed as Disabled, IMG_4058) and
no block or proposal that owns signing.

**Why it is on a stranger's path and not on ours.** A judgment, recalled rather than read: PCs sold
with Windows 11 ship with Secure Boot on, and turning it off is a firmware menu that differs by
vendor. On a machine using BitLocker, changing Secure Boot state can make Windows ask for the
recovery key at its next boot (recalled). So "turn Secure Boot off" is a step a stranger can fail at,
and one that touches the OS they already have.

## Options

| | What the stranger does | What nife does | Cost and consequence |
|---|---|---|---|
| **K1. Turn it off** | Follows a page of per-vendor firmware instructions | Nothing | No key to hold. Every stranger pays a step, and some will not manage it. The web page must say so first, not in a footnote |
| **K2. A shim signed by Microsoft's third-party UEFI CA** | Nothing, on most PCs | Ships a signed first-stage loader (`shim`) carrying nife's own key, and signs `BOOTX64.EFI` with that key; the shim goes through the community review Linux distributions use (recalled, process not read) | A private key whose compromise lets anyone boot code on every machine that trusts it; a review process outside this project; a GPL-licensed shim binary conveyed on the page, which DECISIONS §135 requirement 1 ("no conveyed artifact carries copyleft") must then answer (shim's licence is recalled, not read) |
| **K3. The owner enrolls nife's key** | Enrolls a key through the firmware's setup or a machine-owner-key tool on first boot | Signs with its own key, publishes the certificate | Fewer strangers succeed than with K1 (more steps), and the key custody of K2 without its reach. Best fit for owners who want Secure Boot kept on |

K1 and K3 compose: the page can say "turn it off" and offer K3 to those who will not.

## What the tree does in the analogous case

The measured-boot chain (milestone 104, DECISIONS §26) chose a hash over a signature for exactly
K2's reason: *"today they are built by one command in one tree in one sequence, so the hash is
strictly better"*, and key custody was the objection §26 left standing. The trust fork's T2
(DECISIONS §195) asks the same custody question for
packages. If both a package key and a boot key exist, whether they are one key is a ruling in
itself; the safer default is two.

## Reversibility

K1 is reversible: it is a page. K2 is not, once a signed shim is public: revocation runs through
firmware revocation lists nife does not control. K3 is reversible per owner.

## The §92 test

K1 is by far the cheapest. At equal cost K2 would be preferred for the stranger's experience, and
K1 for having no key to lose, so this is a judgment about who carries the risk, not about effort.

## What it blocks

Rung 4 (publishing the page) cannot go up without an answer, because the page's first instruction
depends on it. Rungs 1 to 3 on xenon do not: xenon's Secure Boot is already off.

## BUGS

- Every fact above about Microsoft's process, shim's license and BitLocker's behavior is
  recalled, and a lane taking K2 or K3 reads them first.
- **This says nothing about aarch64 or riscv64**, which have no UEFI path in this tree.

## Index row

`uefi_loader`'s first `BUGS` entry: *"Secure Boot must be off.
