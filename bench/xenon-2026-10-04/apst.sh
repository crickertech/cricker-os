#!/bin/bash
# Second check: is NVMe APST (drive power saving) or the IOMMU behind Linux's read tail? Reads only.
set -u
LOG=/tmp/apst-$(date +%H%M%S).log
exec > >(tee "$LOG") 2>&1
step() { echo; echo "=== $*"; }
SIZE=$(lsblk -b -d -n -o SIZE /dev/nvme0n1)
[ "$SIZE" = 256060514304 ] || { echo "NOT the Micron (size $SIZE); stopping"; exit 1; }
command -v nvme >/dev/null || dnf install -y nvme-cli
step "iommu"
cat /proc/cmdline; ls /sys/class/iommu; cat /sys/kernel/iommu_groups/*/type 2>/dev/null | sort | uniq -c
step "apst as found"
nvme get-feature /dev/nvme0 -f 0x0c -H | head -5
nvme get-feature /dev/nvme0 -f 0x02 -H | head -3
RD() { fio --name=r --filename=/dev/nvme0n1 --direct=1 --rw=read --bs=4k --iodepth=1 \
    --offset=1M --size=64M --ioengine=io_uring --hipri | grep -E 'READ:|clat \(|50.00th|90.00th|99.00th|ctx='; }
step "read C: as found"; RD
step "turn APST off (feature 0x0c = 0), until reboot"
nvme set-feature /dev/nvme0 -f 0x0c -v 0
nvme get-feature /dev/nvme0 -f 0x0c -H | head -3
step "read D: APST off"; RD
step "read E: APST off, again"; RD
step "sending results to patagonia"
curl -sS -T "$LOG" "http://192.168.8.138:8765/$(basename "$LOG")" && echo SENT || echo "SEND FAILED: photograph the READ lines"
