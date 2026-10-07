#!/bin/zsh
# Run the host-built probes (probe.sh host) against make-images.sh's images. Name provisional.
#
#   WORK=<probe.sh's WORK> notes/filesystem-crates-2026-10-07/run-host.sh <image-dir>
#
# Every extracted file is checked against expected.sha256, whose hashes come from the images'
# provenance (lambutter's expected.json; the bytes make-images.sh wrote; for the kernel fixture,
# the first reader to agree with two others, which is weaker and the note says so), never from
# the probe being judged. Each write runs on a fresh copy and is judged by something other than
# the crate that wrote it: macOS's msdos driver and fsck_msdos for FAT; three other btrfs
# readers and rust-fs-btrfs's checker for btrfs (no `btrfs check` ran; see the note); ext4-view
# for ext4_rs. zsh, so the arrays below split the way they read.
set -u
IMG=${1:?usage: run-host.sh <image-dir>}
HERE=${0:A:h}
WORK=${WORK:-${TMPDIR:-/tmp}/nife-fs-probes}
B=$WORK/bin
OUT=$WORK/out
rm -rf $OUT && mkdir -p $OUT/mnt

typeset -A PATHS READERS
PATHS=(btrfs-f1 'hello.txt dir-a/nested.txt' btrfs-f2 'big.bin small.txt'
    btrfs-kfix 'toplevel.txt subvol1/hello.txt subvol2/zeros.bin'
    fat32 'hello.txt sub/data.bin' fat32-mbr 'HELLO.TXT' ext4 '')
READERS=(btrfs-f1 'lambutter rust-fs-btrfs btrfs-core btrfs-fs ferrosys'
    btrfs-f2 'lambutter rust-fs-btrfs btrfs-core btrfs-fs ferrosys'
    btrfs-kfix 'lambutter rust-fs-btrfs btrfs-core btrfs-fs ferrosys'
    fat32 'fatfs fatfs-nochrono lamfat hadris-fat ferrosys' fat32-mbr 'embedded-sdmmc'
    ext4 'ext4-view ext4_rs ferrosys')

verdict() { # outdir image file
    local name=${3//\//_} got want
    got=$(shasum -a 256 $1/$name 2>/dev/null | cut -c1-64)
    want=$(awk -v k="$2:$3" '$2 == k { print $1 }' $HERE/expected.sha256)
    if [[ -z $got ]]; then print MISSING; elif [[ $got == $want ]]; then print ok; else print WRONG; fi
}

print '## reads'
for img in btrfs-f1 btrfs-f2 btrfs-kfix fat32 fat32-mbr ext4; do
    for p in ${=READERS[$img]}; do
        o=$OUT/r-$img-$p && mkdir -p $o
        $B/$p $IMG/$img.img $o ${=PATHS[$img]} >$o/log 2>&1
        rc=$?
        line="$img $p exit=$rc entries=$(grep -c '^D' $o/log)"
        for f in ${=PATHS[$img]}; do line+=" $f=$(verdict $o $img $f)"; done
        print $line
    done
done

print '## writes'
macfat() { # image partition-suffix file: what macOS's own msdos driver reads back
    local dev
    dev=$(hdiutil attach -readonly -imagekey diskimage-class=CRawDiskImage -nomount $1 | awk 'NR == 1 { print $1 }')
    mount -t msdos -o rdonly $dev$2 $OUT/mnt && { cat "$OUT/mnt/$3"; umount $OUT/mnt; }
    hdiutil detach $dev >/dev/null
}
for p in fatfs fatfs-nochrono lamfat hadris-fat; do
    w=$OUT/w-$p.img && cp $IMG/fat32.img $w && mkdir -p $OUT/w-$p
    WRITE=1 $B/$p $w $OUT/w-$p >/dev/null 2>&1
    rc=$?
    fsck_msdos -n $w >$OUT/w-$p/fsck 2>&1
    print "$p exit=$rc fsck_msdos=$? macos=$(macfat $w '' 'nife dir/Written By Nife.txt' | tr -d '\n') $(grep -m1 'Warning: Item\|Warning: .\.' $OUT/w-$p/fsck)"
done
w=$OUT/w-sd.img && cp $IMG/fat32-mbr.img $w && mkdir -p $OUT/w-sd
WRITE=1 $B/embedded-sdmmc $w $OUT/w-sd >/dev/null 2>&1
rc=$?
dd if=$w of=$OUT/w-sd-part.img bs=512 skip=63 2>/dev/null
fsck_msdos -n $OUT/w-sd-part.img >/dev/null 2>&1
print "embedded-sdmmc exit=$rc fsck_msdos=$? macos=$(macfat $w s1 NIFE.TXT | tr -d '\n')"

for img in btrfs-f1 btrfs-f2 btrfs-kfix; do
    for f in ${=PATHS[$img]}; do
        w=$OUT/w-rfb-$img.img && cp $IMG/$img.img $w && mkdir -p $OUT/w-rfb
        print "rust-fs-btrfs overwrite $img $f: $(WRITE=1 $B/rust-fs-btrfs $w $OUT/w-rfb $f 2>&1 | grep -m1 -E '^W|^WE|mount_rw:' | cut -c1-140)"
        break
    done
    w=$OUT/w-bt-$img.img && cp $IMG/$img.img $w
    $B/btrfs-transaction $w $OUT >/dev/null 2>&1
    line="btrfs-transaction create $img exit=$?"
    for p in lambutter rust-fs-btrfs btrfs-fs ferrosys; do
        o=$OUT/wr-$img-$p && mkdir -p $o
        $B/$p $w $o nife.txt >/dev/null 2>&1
        line+=" $p=$(verdict $o new nife.txt)"
    done
    line+=" checker-findings(before/after)=$(CHECK=1 $B/rust-fs-btrfs $IMG/$img.img $OUT 2>&1 | grep -c '^C [NS]')/$(CHECK=1 $B/rust-fs-btrfs $w $OUT 2>&1 | grep -c '^C [NS]')"
    print $line
done

w=$OUT/w-ext4.img && cp $IMG/ext4.img $w && mkdir -p $OUT/w-ext4
WRITE=1 $B/ext4_rs $w $OUT/w-ext4 >/dev/null 2>&1
rc=$?
$B/ext4-view $w $OUT/w-ext4 nife.txt >/dev/null 2>&1
print "ext4_rs create exit=$rc ext4-view=$(verdict $OUT/w-ext4 new nife.txt)"
