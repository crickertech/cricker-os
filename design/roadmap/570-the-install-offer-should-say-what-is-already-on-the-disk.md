---
status: NOT-STARTED
raised: 2026-09-21
promoted_from: the-install-offer-should-say-what-is-already-on-the-disk
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 570. The install offer should say what is already on the disk

The number is **provisional**: the integrator mints it at merge. Promoted from the proposal `the-install-offer-should-say-what-is-already-on-the-disk` on 2026-09-22, filed 2026-09-21. Raised by the rung 2a lane of milestone 198 (a package manager, and
the trivial install that makes a second customer possible), which built the offer and recorded this
in `install_service`'s `BUGS` on the way past.

The program already exists and already holds exactly the right authority; what is
missing is one spawn and one sentence.

## What the question does not say

`kernel/src/user/install_service.rs` asks one question before a disk is destroyed, and it asks it
with one fact in hand:

```text
  install     :   TARGET: the NVMe disk attached to this machine, 1073741824 bytes.
  install     :   EVERYTHING ON THAT DISK WILL BE DESTROYED.
```

It surveys the disk first, but only for one thing: is there a nife data partition, so that an
installed machine is not offered an install every boot. A disk carrying Windows, or a Linux root, or
somebody's photographs on a single large partition, answers that question "no" and is therefore
described to the person about to lose it as an unqualified target.

A confirmation is only worth what the person can check it against. The install's whole safety
property is that somebody read the sentence and agreed, and the sentence currently withholds the one
fact that would most reliably stop a wrong answer: that this disk is not empty and here is what is
on it.

## The program already exists

`components/src/disk_surveyor.rs` is milestone 57 (partitioning and formatting a real drive)'s read
half. It holds a block-service endpoint for one disk and nothing else, reads LBA 0 through 33, hands
the bytes to `crates/globally_unique_identifier_partition_table`, checks the backup table against the
primary, and reports what is there. `guid::types::name` already turns a type GUID into a phrase a
person reads: `EFI system`, `Microsoft basic data`, `Linux filesystem`, `Apple APFS`, `nife data`.

Its authority is the same authority the survey role already takes, and it takes it the same way:
a disk endpoint and no entropy endpoint, so nothing it holds can write a partition table
anything would read back. So this changes nothing about what the offer may do before a person
answers; it changes what the offer knows.

## What it would look like

```text
  install     :   TARGET: the NVMe disk attached to this machine, 1073741824 bytes.
  install     :   IT IS NOT EMPTY. It carries 3 partitions:
  install     :     1  EFI system            260 MiB
  install     :     2  Microsoft basic data  892 GiB
  install     :     3  Windows recovery       990 MiB
  install     :   EVERYTHING ON THAT DISK WILL BE DESTROYED.
```

and, for the case the current survey covers, the line it already implies:

```text
  install     :   It is empty: no partition table this system can read.
```

Cost: one spawn beside the one already there, a report wide enough for a handful of partitions, and
a printing loop. The survey role in `system_installer` could equally grow the answer rather than a second
program being spawned, and the second program is the better shape: `disk_surveyor` is the thing
in this tree whose whole job is saying what is on a disk, it is already tested against a table
`sgdisk` wrote, and duplicating its GPT reading inside the installer would be two readers that can
disagree.

## What it would settle

Whether the most destructive act this system offers is described to the person who authorizes it.
Today the description is accurate and incomplete, and the incompleteness is on the side that loses
data.

## BUGS

- **It names partition types, not content.** A disk whose single partition is `Linux filesystem`
  tells a person something; a disk that was wiped and repartitioned by somebody else last week tells
  them the same thing and means nothing. There is no filesystem probing here and there should not
  be.
- A disk with no partition table at all is indistinguishable from an empty one, which is right
  for a fresh disk and wrong for one holding a filesystem written straight to the device. Nothing in
  this tree can tell those apart and nothing proposed here changes that.
- **It does not help on a machine with more than one disk**, where the question is which disk rather
  than what is on it. That is the proposal about naming the disk by model.
- A longer question is a question people stop reading. Three extra lines is the budget; a
  partition list on a disk with a dozen entries needs a limit and a "and 7 more", which this
  proposal does not specify.

## Index row

The install surveys the target disk for exactly one thing, whether a nife data partition is already there, so that an installed machine is not offered an install every boot. A disk carrying Windows, a Linux root, or somebody's photographs on one large partition answers that question no and is described to the person about to lose it as an unqualified target. A confirmation is worth only what the person can check it against, and the offer withholds the one fact that would most reliably stop a wrong answer.
