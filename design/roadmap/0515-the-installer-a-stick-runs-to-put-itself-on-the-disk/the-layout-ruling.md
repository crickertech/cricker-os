# The install layout, as built, for calef to ratify

Status: ruled 2026-10-03 (UTC), DECISIONS §244 (the installed disk has four partitions, and a boot slot is 64 MiB). The shape is ratified and slots stay at 64 MiB; the 120-day projection in question 5 was extrapolated from a debug build and is withdrawn there. What follows is the question as asked.

An appendix of [milestone 515 (the installer: a stick that puts itself on the machine's disk and is
then not needed)](../515-the-installer-a-stick-runs-to-put-itself-on-the-disk.md). Written
2026-10-03 (UTC) by the lane briefed to build rung 2a of milestone 198 (a package manager, and the
trivial install that makes a second customer possible). A lane does not write `design/decisions/`,
so this waits here for the maintainer to mint a section from it.

Milestone 198's rulings table still lists install layout as open, and milestone 525 (a bad upgrade cannot brick the machine: two boot slots, tries and priority)'s `BUGS` calls
the on-disk format provisional pending calef. One layout is now built, so the ruling is "ratify this,
or say which part moves". As written by `installer` on a
1 GiB disk (`sgdisk -p`, 2026-10-03):

| # | partition | size | type |
|---|---|---|---|
| 1 | nife data, RedoxFS, first so `FileSystem::open` finds it unaided | rest of the disk (382 MiB here) | §45 `NIFE_DATA` |
| 2, 3 | boot slots 0 and 1, raw image behind a 4096-byte header; state in attribute bits 48-56 | 64 MiB each | `NIFE_BOOT` |
| 4 | EFI system partition, FAT32, `\EFI\BOOT\BOOTX64.EFI` is the chooser | 512 MiB | ESP |

Options only, because this is the irreversible kind. The seven questions:

1. Considered. L1 (ESP plus data) lost when calef ruled tries and priority on 2026-09-21, which
   needs a second image. L2 as two files on the ESP lost on the specification: on an ESP the
   attribute bits are Microsoft's. `Boot####` variables lost because
   rung 2a proves its boot with the variable store deleted. What is built is L3 without L3's cost:
   the "small loader" is the full image, so no second loader exists.
2. This tree. §45 (a nife partition is `EC5CC08B-D749-4434-AC38-A274C50385BA`, and that never changes) already fixed one partition type GUID forever; `NIFE_BOOT` is the same class
   of decision, made provisionally.
3. Prior art. ChromeOS's bit positions, read against the ChromiumOS disk-format reference by
   milestone 525's lane. Android A/B and systemd-boot's counted entries are recalled, not read.
4. Premise. The table says the ruling blocks 2a's merge. It did not: #1056 merged without it.
   What it blocks now is the first install this project cannot redo: rung 2b on xenon can be wiped
   and reinstalled, a stranger's disk after rung 4 cannot.
5. Cost, measured. 640 MiB is fixed (ESP plus slots), 0.25% of xenon's 256 GB. The floor is
   about 700 MiB. The slot size is the number with the least room. `BOOTX64.EFI` from this gate
   is 15,555,072 bytes; the code recorded "about 10 MiB" on 2026-09-21. At those twelve days' rate
   (a projection, not a measurement) the image reaches 64 MiB in about 120 days, and an installed
   disk cannot grow its slots, because the data partition sits in front of them.
6. Reversibility. Nobody outside this tree has installed. Acted on it: the installer, the
   chooser, `crates/boot_slot` and three gates. Changing it before rung 4 costs code, not anybody's disk.
7. Equal cost. The built shape was chosen on the specification's bit ownership and on 2a's
   no-variables property, not on effort, so the answer is yes for the shape. The 64 MiB slot is
   inherited from `uefi_loader`'s `BOOT_FILE_MAX` and was never argued on its own.

The ask, answerable without the diff: ratify the four-partition shape, and pick a slot size. Keep
64 MiB, or raise slots and `BOOT_FILE_MAX` together (256 MiB slots cost 384 MiB more per disk). A no breaks nothing on `main`; a change costs more only after rung 4.
