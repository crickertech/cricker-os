---
status: DECIDED
raised: 2026-10-03
decided: 2026-10-03
ratified_by: calef
---

# 244. The installed disk has four partitions, and a boot slot is 64 MiB

*Section number and slug provisional until the merge queue lands them. Minted 2026-10-03 (UTC) by
the maintainer. calef ruled on the appendix
[the layout ruling](../roadmap/515-the-installer-a-stick-runs-to-put-itself-on-the-disk/the-layout-ruling.md)
of milestone 515 (the installer: a stick that puts itself on the machine's disk and is then not
needed). It holds the seven questions and the options.*

calef, 2026-10-03 (UTC): *"Ratify the shape, keep 64 MiB with both fixes."*

## The ruling

**1. The shape is ratified.** An installed disk is four GPT partitions, in this order:

| # | partition | size | type |
|---|---|---|---|
| 1 | nife data, RedoxFS, placed first so `FileSystem::open` finds it unaided | the rest of the disk | §45 (a nife partition is `EC5CC08B-D749-4434-AC38-A274C50385BA`, and that never changes) `NIFE_DATA` |
| 2, 3 | boot slots 0 and 1, each a raw image behind a 4096-byte header; slot state in GPT attribute bits 48-56 | 64 MiB each | `NIFE_BOOT` |
| 4 | EFI system partition, FAT32, holding `\EFI\BOOT\BOOTX64.EFI` as the chooser | 512 MiB | ESP |

`NIFE_BOOT` is `11631EE3-E18F-4AFC-9D7F-171572635629`
(`globally_unique_identifier_partition_table::guid::types::NIFE_BOOT`). It was ratified on
2026-09-21 as a value; this section makes it permanent the way §45 made `NIFE_DATA` permanent: a
disk written by one release has only this number to agree on with the next, so it never changes.
The attribute bit positions and the slot header's bytes are ratified with it.

**2. Slots stay at 64 MiB.** The appendix projected a full slot in about 120 days from a
15,555,072-byte `BOOTX64.EFI`. A measurement on 2026-10-03 overturned the premise: that file was a
debug build, a 7.5 MB unstripped kernel (about 5.8 MB of it debug info) plus a 7.9 MB archive of
opt-level-0 programs. A release, stripped image is about 4.5 MB (an estimate). A boot-only
image is about 1.7 MB (an estimate). A 64 MiB slot holds about 14 release images. The 120-day figure
was extrapolated from debug builds and is withdrawn. The estimates stay estimates until the gate
below records a measured release size.

3. Two conditions come with the ruling. Both are being built by another lane; this section
names them as conditions, not as done.

- (a) The installer installs a release image, never a debug one.
- (b) CI gates the release `BOOTX64.EFI` at a 16 MiB size budget, a quarter of a slot. Raising the
  budget takes a committed ratchet with a stated reason.

Until both land, the 64 MiB ruling rests on an estimate.

## What this closes

- Milestone 198 (a package manager, and the trivial install that makes a second customer possible)'s "Install layout" ruling, which blocked rung 4 (the first install nobody here can
  redo).
- Milestone 525 (a bad upgrade cannot brick the machine: two boot slots, tries and priority)'s `BUGS` entry calling the on-disk format provisional.
- Milestone 515's `decision_dependencies`, which pointed at this fork as `unwritten`.

## Reversibility

Nobody outside this tree has installed. Changing the shape before rung 4 costs code (the
installer, the chooser, `crates/boot_slot` and three gates); after a stranger's disk exists it
costs that disk. An installed disk cannot grow its slots, because the data partition sits in
front of them, so the size budget is what keeps 64 MiB honest.

## BUGS

- The release and boot-only sizes above are estimates, and the 16 MiB budget does not exist until
  condition (b) merges.
