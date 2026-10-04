#!/usr/bin/env python3
"""Replay a cap-trace file for the progenitor's table. Usage: analyze.py TRACE ELF"""
import os, re, subprocess, sys
from collections import defaultdict

trace, elf = sys.argv[1], sys.argv[2]
pat = re.compile(r"^(\d+) (P|X) ([-+=]) slot=(\d+) used=(\d+) pc=(0x[0-9a-f]+) tid=(\d+) (.*) rights=(0x[0-9a-f]+) bt=\[(.*)\]")
ev = []
for line in open(trace):
    m = pat.match(line)
    if m and m.group(2) == "P":
        seq, _, op, slot, used, pc, tid, obj, rights, bt = m.groups()
        bt = [int(x, 16) for x in bt.split(", ") if x != "0"]
        ev.append((int(seq), op, int(slot), int(used), int(pc, 16), int(tid), obj, rights, bt))

addrs = sorted({a for e in ev for a in e[8]})
symcache = {}
if addrs:
    out = subprocess.run([os.environ.get("LLVM_SYMBOLIZER", "llvm-symbolizer"), "--obj=" + elf, "-f", "-C", "-i"],
                         input="\n".join(hex(a - 4) for a in addrs) + "\n", capture_output=True, text=True).stdout
    blocks = out.strip("\n").split("\n\n")
    for a, blk in zip(addrs, blocks):
        lines = blk.split("\n")
        symcache[a] = [(lines[k], lines[k + 1]) for k in range(0, len(lines) - 1, 2)]

def where(bt):
    """Innermost frame in system_initializer (inline-expanded), plus the leaf helper."""
    leaf = None
    for a in bt:
        for fn, loc in symcache.get(a, []):
            short = loc.split("/crates/")[-1]
            if leaf is None:
                leaf = fn.split("::")[-1]
            if "system_initializer/src/lib.rs" in loc:
                return f"{short.replace('system_initializer/src/lib.rs', 'si')} ({leaf})"
    return f"? ({leaf})"

occ = {}
intervals = []
peak = 0
snaps = []
for seq, op, slot, used, pc, tid, obj, rights, bt in ev:
    if slot in occ and op in "-=":
        o = occ.pop(slot)
        intervals.append(dict(slot=slot, obj=o[0], rights=o[1], minted=o[2], at=o[3], released=seq, rel_at=where(bt)))
    if op in "+=":
        occ[slot] = (obj, rights, seq, where(bt) if bt else f"kernel grant (tid {tid})")
    assert len(occ) == used, (seq, len(occ), used)
    if used > peak:
        peak, snaps = used, [(seq, dict(occ))]
    elif used == peak:
        snaps.append((seq, dict(occ)))
for slot, o in occ.items():
    intervals.append(dict(slot=slot, obj=o[0], rights=o[1], minted=o[2], at=o[3], released=None, rel_at="held at end of trace"))

print(f"# {trace}: {len(ev)} events, peak {peak}, {len(snaps)} events at peak, seq {snaps[0][0]}..{snaps[-1][0]}")
first = snaps[0][0]
def iv(slot, minted):
    return next(i for i in intervals if i["slot"] == slot and i["minted"] == minted)
print("\n## occupancy at the first peak event")
for slot in sorted(snaps[0][1]):
    obj, rights, minted, at = snaps[0][1][slot]
    i = iv(slot, minted)
    print(f"{slot:2} | {obj:28} | r={rights:4} | +{minted:<5} {at:60} | -{i['released']} {i['rel_at']}")
print("\n## every distinct peak moment: the event that reached it")
seen = set()
for seq, s in snaps:
    e = next(e for e in ev if e[0] == seq)
    key = where(e[8])
    if key in seen:
        continue
    seen.add(key)
    print(f"seq {seq}: {e[1]} slot {e[2]} {e[6]} at {key}")
print("\n## slots whose occupant changes across peak moments")
var = defaultdict(set)
for seq, s in snaps:
    for slot, v in s.items():
        var[slot].add(v[3])
for slot, ats in sorted(var.items()):
    if len(ats) > 1:
        print(f"{slot}: {sorted(ats)}")

for want in [int(x) for x in sys.argv[3:]]:
    occ2 = {}
    for seq, op, slot, used, pc, tid, obj, rights, bt in ev:
        if op in "-=":
            occ2.pop(slot, None)
        if op in "+=":
            occ2[slot] = (obj, seq, where(bt) if bt else "kernel grant")
        if seq == want:
            print(f"\n## occupancy at seq {want} ({len(occ2)})")
            for s_ in sorted(occ2):
                o = occ2[s_]
                print(f"{s_:2} | {o[0]:28} | +{o[1]:<5} {o[2]}")
            break
