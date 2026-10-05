---
status: PROPOSED
raised: 2026-10-05
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: yes
---
# Report QEMU's PLIC enable-write defect upstream

Raised by the lane for the provisional milestone "a USB keystroke never strands `line_editor`"
(`lane/usb-keyboard-lost-wakeup`, #1657) on 2026-10-05 (UTC), after finding that the riscv64
swish-check keystroke stall was an emulator defect that nife's PLIC driver happened to expose. Title
and slug are drafts.

**Reuse:** none exists to take; this is a report to an upstream, not code. Searched QEMU's
`hw/intc/sifive_plic.c` at v11.1.1 for any re-evaluation on the enable path (there is none) and
nife's own tree for an existing upstream report or workaround record (none before #1657).

## The defect

QEMU 11.1.1's `hw/intc/sifive_plic.c` re-evaluates which contexts should see an external interrupt
(`sifive_plic_update`) after a priority write, a threshold write, a completion and a rising source
line. **It does not after a write to an enable word.** The enable branch of `sifive_plic_write`
stores the word and returns. So a source that went pending while it was disabled stays pending,
enabled, above threshold and undelivered until some unrelated event calls `sifive_plic_update`.

The specification has the PLIC notify a target whenever a source is pending, enabled and above the
threshold. A real part evaluates that continuously, so the defect is the emulator's.

nife hit it because `plic::enable`, which is also a userspace driver's `Irq::ACK`, wrote the
priority before the enable bit, so the ACK's only re-evaluation ran while the bit was clear. #1657
reordered the two writes. That works around the defect in nife. Any other guest that masks a PLIC
source in its handler and unmasks it with nothing but an enable write is still exposed. Whether
Linux's `irq-sifive-plic.c` is such a guest was not checked.

## The minimal reproducer

Any level source the guest can raise by hand works. The 16550's transmit-empty interrupt is the
one nife's test uses:

1. Enable source 10 (the UART on `virt`) for a context with priority 1 and threshold 0.
2. Raise the line (set `IER.ETBEI`), take the interrupt, claim it, clear its enable bit, complete.
3. Lower the line (`IER = 0`), then raise it again. It is now pending while disabled.
4. Set the enable bit. Nothing else.
5. `sip.SEIP` stays clear and no interrupt is taken, although source 10 is pending, enabled and
   above threshold. Any priority or threshold write now delivers it at once.

In nife that sequence is the kernel test, red with the old write order and deterministic:

```sh
git apply kernel/falsifications/sched.tests.an_interrupt_raised_while_its_line_is_masked_is_delivered_at_the_ack.patch
cargo xtask test --arch riscv64 --test an_interrupt_raised_while_its_line_is_masked
git apply -R kernel/falsifications/sched.tests.an_interrupt_raised_while_its_line_is_masked_is_delivered_at_the_ack.patch
```

The symptom that found it, measured on patagonia with the old order: 10 stalls in 300 boots of
swish-check's USB keystrokes alone (riscv64, TCG, four harts). The harness that counted them is
short enough to keep here. It boots the `--features shell` riscv64 kernel with `qemu-xhci` and
`usb-kbd`, types `echo hello` through the monitor, and calls a boot a stall when `hello` does not
come back within 15 s:

```python
#!/usr/bin/env python3
# usage: usbloop.py <worktree> <boots> <parallel> <key_delay_ms>
import os, socket, subprocess, sys, threading, time, concurrent.futures as cf

WT, N, PAR, DELAY = sys.argv[1], int(sys.argv[2]), int(sys.argv[3]), int(sys.argv[4]) / 1000
KEYS = ["e", "c", "h", "o", "spc", "h", "e", "l", "l", "o", "ret"]
DROP = ("NIFE_KEYBOARD", "NIFE_GPU", "NIFE_GPU_MON", "NIFE_NET", "NIFE_DISK", "NIFE_NVME")

def boot(i):
    sock = f"/tmp/usbloop-{os.getpid()}-{i}.sock"
    env = {k: v for k, v in os.environ.items() if k not in DROP}
    env.update(NIFE_USB_KEYBOARD="1", NIFE_USB_KEYBOARD_OPTS=",usb_version=1", NIFE_SCREEN_MON=sock,
               NIFE_INITRD=os.path.join(WT, "target/initrd-riscv.img"))
    p = subprocess.Popen(["helpers/qemu-bounded.sh", "120", "helpers/qemu-runner-riscv64.sh",
                          "target/riscv64imac-unknown-none-elf/debug/kernel"], cwd=WT, env=env,
                         stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    buf, lock = bytearray(), threading.Lock()
    def rd():
        while (b := p.stdout.read1(4096)):
            with lock: buf.extend(b)
    threading.Thread(target=rd, daemon=True).start()
    def text():
        with lock: return buf.decode("utf-8", "replace").replace("\r", "")
    def wait(pred, secs):
        end = time.time() + secs
        while time.time() < end:
            if pred(text()): return True
            time.sleep(0.1)
        return False
    verdict = "noprompt"
    if wait(lambda s: s.endswith("$ ") and "usb       : a keyboard on port" in s, 60):
        mark = len(text())
        for k in KEYS:
            with socket.socket(socket.AF_UNIX) as s:
                s.connect(sock); s.sendall(f"sendkey {k}\n".encode())
            time.sleep(DELAY)
        verdict = "ok" if wait(lambda s: "echo hello\nhello\n" in s[mark:], 15) else "STALL"
    p.terminate()
    subprocess.run(["pkill", "-f", sock])
    return verdict

with cf.ThreadPoolExecutor(PAR) as ex:
    res = {}
    for v in ex.map(boot, range(N)):
        res[v] = res.get(v, 0) + 1
print(res)
```

Run it from a worktree after `script/swish-check --arch riscv64` has built the image, as
`usbloop.py <worktree> 300 6 200`. It kills each QEMU it started by its socket name, and
`pgrep -l qemu` afterwards should be empty.

## The one-line patch

```diff
--- a/hw/intc/sifive_plic.c
+++ b/hw/intc/sifive_plic.c
@@ -219,6 +219,7 @@ static void sifive_plic_write(void *opaque, hwaddr addr, uint64_t value,
 
         if (wordid < plic->bitfield_words) {
             plic->enable[addrid * plic->bitfield_words + wordid] = value;
+            sifive_plic_update(plic);
         } else {
             qemu_log_mask(LOG_GUEST_ERROR,
                           "%s: Invalid enable write 0x%" HWADDR_PRIx "\n",
```

This is written against v11.1.1 and has not been built or sent. Upstream's development branch has
not been checked for a fix that landed after 11.1.1. That check comes first, before anything is
sent.

## Why this waits on calef

Sending it upstream is a fact that leaves the machine: a public mailing-list post or issue under
somebody's name, which nobody can take back. That is on the irreversible list in `AGENTS.md`, so it
is calef's call. The work itself is small: check upstream, build QEMU with the patch, confirm the
falsification patch goes green on it, and write the report.

## BUGS

- The reproducer above is nife's, not a standalone guest. Upstream will want either a
  `tests/qtest` case or a few lines of bare-metal assembly, and neither is written.
- The patch adds a re-evaluation for every enable write, including ones that change nothing. The
  cost is believed negligible (`sifive_plic_update` already runs on every source line change) but
  was not measured.
