#!/bin/bash
# Linux-side check for the 2x read gap: is PCIe ASPM or NVMe APST slowing reads?
# Reads only from the disk. Results go back to patagonia.
set -u
LOG=/tmp/aspm-$(date +%H%M%S).log
exec > >(tee "$LOG") 2>&1
step() { echo; echo "=== $*"; }
NV=$(basename "$(readlink -f /sys/class/nvme/nvme0/device)")
RP=$(basename "$(readlink -f /sys/class/nvme/nvme0/device/..)")
step "nvme at $NV, root port $RP"
SIZE=$(lsblk -b -d -n -o SIZE /dev/nvme0n1)
lsblk -b -d -o NAME,SIZE,MODEL /dev/nvme0n1
[ "$SIZE" = 256060514304 ] || { echo "NOT the Micron (size $SIZE); stopping"; exit 1; }
command -v lspci >/dev/null || dnf install -y pciutils
command -v fio >/dev/null || dnf install -y fio
step "before"
for d in "$NV" "$RP"; do echo "-- $d"; lspci -vv -s "$d" | grep -E 'LnkCap:|LnkCtl:|LnkSta:'; done
echo "aspm policy: $(cat /sys/module/pcie_aspm/parameters/policy)"
echo "apst max latency us: $(cat /sys/module/nvme_core/parameters/default_ps_max_latency_us)"
step "read A: as found (polled)"
modprobe -r nvme && modprobe nvme poll_queues=1
for i in $(seq 50); do [ -b /dev/nvme0n1 ] && break; sleep 0.2; done
fio --name=r --filename=/dev/nvme0n1 --direct=1 --rw=read --bs=4k --iodepth=1 \
    --offset=1M --size=64M --ioengine=io_uring --hipri --output-format=normal
step "clear ASPM on both ends of the link"
setpci -s "$NV" CAP_EXP+10.w=0:3
setpci -s "$RP" CAP_EXP+10.w=0:3
step "reload nvme with APST off and poll queues"
modprobe -r nvme; modprobe -r nvme_core
modprobe nvme_core default_ps_max_latency_us=0 && modprobe nvme poll_queues=1
for i in $(seq 50); do [ -b /dev/nvme0n1 ] && break; sleep 0.2; done
step "after"
for d in "$NV" "$RP"; do echo "-- $d"; lspci -vv -s "$d" | grep -E 'LnkCtl:|LnkSta:'; done
echo "apst max latency us: $(cat /sys/module/nvme_core/parameters/default_ps_max_latency_us)"
echo "io_poll: $(cat /sys/block/nvme0n1/queue/io_poll)"
step "read B: ASPM and APST off (polled)"
fio --name=r --filename=/dev/nvme0n1 --direct=1 --rw=read --bs=4k --iodepth=1 \
    --offset=1M --size=64M --ioengine=io_uring --hipri
step "sending results to patagonia"
curl -sS -T "$LOG" "http://192.168.8.138:8765/$(basename "$LOG")" && echo SENT || echo "SEND FAILED: photograph the two READ lines"
