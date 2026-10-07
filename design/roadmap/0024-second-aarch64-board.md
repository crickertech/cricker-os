---
status: OPTIONAL
raised: 2026-07-30
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 24. A second aarch64 *board*: Virtualization.framework (optional)

Modernized 2026-08-03 to the current block standard, with the two design
forks named that the original one-liner left implicit; the substance is unchanged.

Fork two, the host-side runner, is decided: vfkit (calef, 2026-08-26: "Ratify
vfkit"). See below. Sequencing is a separate question and not a blocker, since the default EFI path
follows milestone 88's stage 1 while the Linux-boot-protocol path needs neither.

In brief. Boot under Apple's Virtualization.framework, not QEMU's `virt`: a different machine
of the *same* ISA, with no PL011, its own device discovery, and its own boot handoff.

Why it matters. Proves the `arch/` board boundary on a second machine without buying one
(cheaper than 16's silicon, distinct from 20's second ISA): if rule 1 has been kept, this is a new
directory, not a diff across the tree. It also lets nife run under the same VMM that runs
macOS and Linux guests on the dev machine. Optional; a portability exercise, **not** a
benchmarking prerequisite (guest-internal microbenchmarks are VMM-independent, and the HVF leg
already runs the physical core under QEMU's board).

Fork one, the boot loader, and it defines the milestone's size. VZ offers two:

- `VZLinuxBootLoader` boots any image carrying the ARM64 Linux boot header (a 64-byte magic
  prefix) and hands the guest a device tree with its address in `x0`, per the Linux boot
  protocol. Small delta: add the header, reuse the existing DTB front door (milestone 60), write
  the virtio-console driver, done. No UEFI, no ACPI. (The old note that QEMU does not pass a DTB
  pointer in `x0` stays true for QEMU's ELF path; under this protocol the pointer is real.)
- `VZEFIBootLoader` is generic UEFI, sharing milestone 88's boot stub. Bigger alone, but if
  88 proceeds the stub exists anyway and this milestone collapses to "same stub, different VMM."

Sequencing follows from the fork: default is the EFI path, after 88's stage 1, one boot stub
serving two VMMs and every cloud provider. The Linux-protocol path is the recorded fast-track,
worth taking early only if the virtio-console driver is wanted sooner, which 88 may decide (see
below).

The genuinely new artifact either way: a virtio-console driver. VZ has no PL011; the console
is virtio. This is the tree's first console that is not a memory-mapped UART, and it may
double-serve: whether OCI's serial console presents a 16550 or virtio-console is one of milestone
88's recorded unknowns, so this driver is potentially on 88's path, not just this one's. If 88's
stage 2 finds virtio-console, build the driver there and this milestone inherits it.

Fork two, the host-side runner: decided as vfkit, 2026-08-26. VZ requires a host binary with
the `com.apple.security.virtualization` entitlement, and the choice was framed as symmetric (a new
Swift runner in this tree, or a dependency on an existing CLI such as vfkit) without being one.
Checked rather than assumed: vfkit ([crc-org/vfkit](https://github.com/crc-org/vfkit), Apache-2.0)
is written in Go internally, but that is irrelevant here, since nothing in this tree would ever
touch its source. As an **external host binary invoked by a runner script**, it is the identical
shape this tree already depends on for `qemu-system-aarch64`/`-riscv64`/`-x86_64`: installed via
Homebrew, shelled out to from `helpers/qemu-runner-*.sh`, never vendored, zero lines in the Cargo
build graph. A Swift runner would instead be the tree's first non-Rust *source file*, with its own
`swiftc`/`Package.swift` step nothing else here needs. The "new language in the repository" framing
above applies to the Swift option only; it does not transfer to vfkit, which is why the two were
not actually symmetric.

vfkit's own CLI already covers both fork-one boot paths directly: `--bootloader
linux,kernel=...,initrd=...,cmdline=...` and `--bootloader efi,variable-store=...,create`, plus
`--device virtio-serial` (stdio or pty) for the console this milestone needs, matching the virtio
device set this tree already drives elsewhere. It is real production software (Apache-2.0, v0.6.1
as of June 2026), not a toy: adopted by podman 5.0+, minikube 1.35+, and Red Hat's crc. §46's own
rule agrees independently: this tree's dependency posture is thin architectural primitives or whole
subsystems it would never write, nothing in between, and a full VM CLI with disk/network/USB/vsock
support this milestone mostly will not use is squarely the latter, the same reasoning that vendors
RedoxFS and smoltcp rather than hand-rolling them.

The one real cost, named rather than hidden. vfkit sits outside `script/lint`'s supply-chain
check, which covers Cargo dependencies; an external host tool a shell script invokes is not touched
by it at all. This is not a new blind spot: `qemu-system-*` already has the identical one, unaudited
by the same gate. Whoever wires the runner script should document the required vfkit version the
same way the QEMU runners document their pinned QEMU version.

## Scope note

Nothing here regresses the QEMU board: `virt` stays the primary development machine, and the
board boundary this milestone exists to prove means the VZ directory must not leak into it (rule
1, applied at board rather than ISA granularity). Interrupt controller and memory layout under VZ
are facts to read from what VZ actually presents (it declares a GICv3 and its layout in the DTB
it hands over), not assumed from QEMU's `virt`.

## Index row

proves the `arch/` **board** boundary on a second machine of the same ISA; optional
