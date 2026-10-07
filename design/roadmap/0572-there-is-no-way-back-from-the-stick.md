---
status: NOT-STARTED
raised: 2026-09-21
promoted_from: there-is-no-way-back-from-the-stick
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 572. There is no way back from the stick: an installed disk is never offered an install again

The number is provisional: the integrator mints it at merge. Promoted from the proposal `there-is-no-way-back-from-the-stick` on 2026-09-22, filed 2026-09-21. Raised by the rung 2a lane of milestone 198 (a package manager, and
the trivial install that makes a second customer possible), which introduced the rule this proposal
is about and found the case that makes it a defect rather than a choice.

It is a second question asked by `kernel/src/user/install_service.rs`, which holds no
disk and decides nothing a ruling has to cover.

## Correction, 2026-10-04 (UTC): the survey was not reached until PR #1652

This block's premise, that a stick booted on an installed disk surveys it and declines, **was false
on any firmware that connects the installed disk before starting the stick**, and a one-time boot
menu has to, to list it. The stick's loader is the same binary as the disk's, and its chooser found
the disk's boot slots and chain-loaded slot 0. The installed system ran, an image started from a
slot has no boot file, and the offer and its survey were skipped before either began. So the
REPLACE question below would never have been reached on a machine with slots.

Measured, not read: boot 3 of `cargo xtask install-boot` boots the stick with the installed disk as
a second boot option, and on `main` at `695134e46` it printed `uefi_loader: starting boot slot 0`
from the stick. Under OVMF with the stick as the only boot option the disk is not connected and the
premise held, which is how it went unseen.

PR #1652 (the live stick proposal's G1) makes the chooser boot only slots on the disk its own file
is on. With it, the stick runs its own system, the survey runs, finds nife and declines, and this
block's premise is true again. Everything below stands as written.

## The rule, and why it exists

An installed machine boots from a file too. `memory::boot_file_region()` is `Some` on every UEFI
boot, whether the file came off a stick or off the disk the machine is about to run from, and there
is nothing in the handoff that distinguishes them. So an offer that asked on every boot it *could*
ask on would ask an installed machine, once per boot, whether to wipe itself, and would pause thirty
seconds waiting for an answer on a machine nobody is watching.

The install offer therefore surveys first: `system_installer`'s `ROLE_SURVEY`, a process holding the disk
and no entropy endpoint, reads the partition table and answers whether a nife data partition is
already there. If it is, the offer is not made at all.

**That rule is right and this proposal does not ask to remove it.** It asks for the other half.

## The cost, which has a sharp case

The obvious cost is that a person who wants to reinstall cannot. They boot the stick, watch it
decline to offer, and have no way forward from inside the system: nothing in this tree wipes a
partition table, and `system_installer` is the only program that writes one.

**The sharp case is a failed install, and it is not hypothetical.** `install_service` runs two
programs in sequence: `system_installer` writes the table and the EFI system partition, then `mkfs` creates
the filesystem. Both are recorded as not crash-atomic. A power cut between them leaves a disk that
has a nife data partition and no filesystem in it, which is exactly the state the survey reads
as "already installed". That machine will never be offered an install again, by the stick that
half-installed it or by any other, and the person holding it has no message explaining why.

So the rule as it stands turns a recoverable failure into an unrecoverable one, using the mechanism
that was added to make an installed machine behave well. That is the argument for this proposal and
it is the whole of it.

## What it would look like

A second question rather than a second mechanism:

```text
  install     : this disk already carries nife (data partition at LBA 2048).
  install     :   Type REPLACE to destroy it and install again; anything else continues the boot.
```

The survey already reports the partition's first LBA, so the sentence can be specific. The phrase
differs from `INSTALL` on purpose: a person who typed the first one on the machine they meant to
install should not be able to type it again by habit on the machine they did not.

It should say whether there is a filesystem in there, which is what separates a working install
from the half-written one above, and `mkfs`'s own `ROLE_CHECK` already answers exactly that question
from a process holding the disk and no entropy. Wiring it is the same shape as the survey.

Cost: one more spawn, one more question, and two more lines in the gate. Small, and it is small
precisely because the confined-program-per-question shape is already built.

## BUGS

- **A second phrase is not a second safety property.** Somebody who will type `INSTALL` at a
  question naming the wrong disk will type `REPLACE` at one too. What actually protects a person is
  the offer naming the disk well, which is a different proposal.
- It does not help a disk that carries somebody else's operating system, which the survey reads
  as installable and always has. That is the proposal about surveying what is already there.
- Nothing here makes the install crash-atomic, which is the underlying fault. A reinstall path
  makes the half-written state recoverable; it does not stop it happening, and nobody has measured
  how often it would.

## Index row

An installed machine boots from a file too, and nothing in the handoff distinguishes that file from one on a stick, so the install offer surveys first and declines to ask when a nife data partition is already there. That rule is right and this does not ask to remove it. It asks for the other half: a person who wants to reinstall boots the stick, watches it decline, and has no way forward from inside the system, because nothing in this tree wipes a partition table and `system_installer` is the only program that writes one.
