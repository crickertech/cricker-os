---
status: DECIDED
raised: 2026-10-06
decided: 2026-10-06
ratified_by: calef
---

# 251. Restarting the machine is a kernel object the progenitor hands out

calef, 2026-10-06 (UTC), on PR #1766: *"Approved, go with the new reboot object."* That approves
the recommendation of the proposal milestone 805 (`reboot` at the prompt) was promoted from, on all
three of its questions. *(Section number provisional until the merge queue lands it.)* Recorded by
the maintainer session the same day. Nothing is built yet; milestone 805 builds it.

Every name here is provisional: the object, its method, the manifest field and the program
`reboot`. Naming them is a separate ratification under design/naming.md. This section calls them
the reboot object, `REBOOT` and the `reboot` field for want of anything better.

## The ruling

1. A new kernel object kind carries the authority to restart the machine. It has no payload: the
   kernel stores nothing in it, and it names no device. It gets one `abi::objtype` number and one
   method. This is a new method within the capability model of §10 (process model:
   capability-based, microkernel) and §16 (object revocation). It adds no syscall number.
2. The kernel creates exactly one at boot and grants it to the progenitor. The progenitor endows it
   only to a program whose manifest declares the `reboot` field, the way `entropy` is endowed
   (`crates/manifest_note`). A program that does not declare it holds none.
3. The `reboot` program flushes the writable filesystem with `filesystem_protocol::fs::SYNC` and
   waits for the reply, then invokes `REBOOT`. An ordered shutdown of services is deferred to a
   later milestone, which will add its step before the `SYNC`.
4. Power-off is excluded. It is a separate program and a separate object kind, so holding the
   reboot object confers no power to switch the machine off. Neither is in milestone 805.

## The method

`REBOOT` takes no arguments. Holding the capability is the whole authority; no rights bit beyond
what invoking any object needs.

- On success it does not return. The kernel calls `arch::reboot`: PSCI `SYSTEM_RESET` on aarch64,
  SBI SRST reset type 1 on riscv64, and the FADT reset register, then `0xCF9`, then the 8042 on
  x86_64. Those routes exist today behind `--features reboot_soak_test`; milestone 805 gives them
  this second caller and leaves the soak its own feature.
- If the firmware refuses and `arch::reboot` returns, the method returns an error to the caller
  carrying the refusal. The program prints it and exits non-zero.
- A firmware that accepts the call and hangs cannot be told apart from a slow reset from inside
  the machine. radon did exactly this on 2026-09-04, and milestone 592 (radon's cold reboot dies
  in OpenSBI's PMIC write) holds the fix.
- The kernel does not sync anything. It knows no filesystem, and a microkernel that did would be
  the bug. The `SYNC` in clause 3 is the program's step. A holder that invokes `REBOOT` without it
  loses writes the device had not flushed. That is a foot gun, recorded here and in the program's
  `BUGS`; the mitigation is that only one program is endowed.
- Devices are not quiesced first. A DMA transfer in flight is cut off. After `SYNC` the block
  servers are idle, and nothing else writes to persistent storage.

## Why SYNC and no more

RedoxFS is the writable filesystem, and notes/fs-server.md measures it prefix-consistent at every
power-cut point. Milestone 37 (prove RedoxFS's crash consistency) showed a cut loses only a suffix
of the workload. `SYNC` reaches a real device flush in both the virtio and NVMe block servers, which
makes "nothing acknowledged is lost" true on a device with a volatile write cache. No service yet
holds state a stop message would save: the system log of milestone 613 (a system log service: the
in-memory half) lives in memory only.

## What was refused

| Option | Why it lost |
|---|---|
| B. A method on an existing object | No existing object names the machine, and a "may reset" bit on the root `MemoryRegion` would make every memory grant a question about power. |
| C. A user-mode driver holding device authority | PSCI and SBI calls cannot be made from user mode, so it works on x86_64 alone and breaks §19 (architectural parity). |
| D. A new syscall number | Ambient authority with no identity for the kernel to check, and a new number is its own fork. |
| E. An op on milestone 391 (kernel introspection over an endpoint)'s endpoint | §149 (may the kernel answer on an endpoint) left that open, and it would put a harmless read and a destructive act behind one capability. |
| Power-off in the same milestone, or a `--off` flag | One program, one thing (calef, 2026-09-26), and on radon a powered-off board needs a person at smart plug 2. |

At equal cost the answer is the same. The reboot object is the only option that is both a
capability and possible on all three architectures. Fuchsia's `zx_system_powerctl`, which takes a
resource handle, is the same shape; Linux's `reboot(2)` behind `CAP_SYS_BOOT` is D with an identity
check. Both are recalled, not re-read.

## Open

- Who at the prompt may reboot is not decided. Any session the progenitor endows can. Restricting
  it to some logins belongs to whichever milestone decides what a login grants.
- The new dispatch arm is cold, and `script/fastpath-footprint` will price it when it exists. No
  number is claimed before then.
