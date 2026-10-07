#!/bin/sh
# Assemble the images notes/filesystem-crates-2026-10-07.md's host round trips ran against.
# Name provisional. macOS only (hdiutil, newfs_msdos, diskutil); needs the network and zstd.
#
#   notes/filesystem-crates-2026-10-07/make-images.sh <dir>
#
# None of these images was made by a crate under test. The btrfs and ext4 ones were made by Linux
# tools and are taken from other projects' test fixtures at pinned commits: lambutter's two were
# made by mkfs.btrfs (its docs/TESTING-AND-FUZZING-PLAN.md section 5.1), btrfsutils' fixture holds
# subvolumes and a snapshot, which only a kernel mount can create, and ext4-view's was made by
# mkfs.ext4 through its own xtask. The two ZFS pools are lamzfs's fixture (its MANIFEST.json
# records each file's sha256) and zfs-forensic's `zfs_dir` test pool, both made by OpenZFS's `zpool`. The FAT ones are made here by macOS, which is the point: a
# stick formatted and written by another operating system.
set -eu
D=${1:?usage: make-images.sh <dir>}
mkdir -p "$D"
cd "$D"
raw=https://raw.githubusercontent.com
curl -sfL -o f1.zst "$raw/lamco-admin/lambutter/6952ad57635c44e5cea7bca3710796a63d413bfc/tests/fixtures/data/f1_single_uncompressed.img.zst"
curl -sfL -o f2.zst "$raw/lamco-admin/lambutter/6952ad57635c44e5cea7bca3710796a63d413bfc/tests/fixtures/data/f2_single_zstd.img.zst"
curl -sfL -o kfix.gz "$raw/rustutils/btrfsutils/ea6ff6ae470e0987d8bbb8138a8d405adb4715e6/cli/tests/commands/fixture.img.gz"
curl -sfL -o ext4.zst "$raw/nicholasbishop/ext4-view-rs/2e072fbcb64529dc4042852af74e97ef89893e59/test_data/test_disk_4k_block_journal.bin.zst"
curl -sfL -o zl.zst "$raw/lamco-admin/lamzfs/6462e168dbd8a0a59dbc0c7bc5f117c1acc906da/tests/fixtures/single_lz4.0.img.zst"
curl -sfL -o zd.gz "$raw/SecurityRonin/zfs-forensic/55a9fced1c7262e033fdf820eecbc2762917c830/tests/data/zfs_dir.img.gz"
zstd -q -d -f f1.zst -o btrfs-f1.img
zstd -q -d -f f2.zst -o btrfs-f2.img
gunzip -c kfix.gz >btrfs-kfix.img
zstd -q -d -f ext4.zst -o ext4.img
zstd -q -d -f zl.zst -o zfs-lam.img
gunzip -c zd.gz >zfs-dir.img

populate() { # mount point
    printf 'hello from macos\n' >"$1/hello.txt"
    mkdir "$1/sub"
    python3 -c "import sys; open(sys.argv[1],'wb').write(b''.join(b'%015d\n' % i for i in range(4096)))" "$1/sub/data.bin"
    printf 'long name\n' >"$1/Long File Name.txt"
}

# An unpartitioned ("superfloppy") FAT32, written by macOS's msdos driver.
rm -f fat32.img && mkfile -n 64m fat32.img
dev=$(hdiutil attach -imagekey diskimage-class=CRawDiskImage -nomount fat32.img | awk 'NR == 1 { print $1 }')
newfs_msdos -F 32 -v NIFEPROBE "$dev" >/dev/null
mkdir -p mnt && mount -t msdos "$dev" mnt && populate mnt && umount mnt
hdiutil detach "$dev" >/dev/null

# The same, inside an MBR partition table, which is how a USB stick usually arrives.
rm -f fat32-mbr.img && mkfile -n 64m fat32-mbr.img
dev=$(hdiutil attach -imagekey diskimage-class=CRawDiskImage -nomount fat32-mbr.img | awk 'NR == 1 { print $1 }')
diskutil eraseDisk FAT32 NIFEMBR MBRFormat "$dev" >/dev/null
populate /Volumes/NIFEMBR
diskutil eject "$dev" >/dev/null
fsck_msdos -n fat32.img >/dev/null
