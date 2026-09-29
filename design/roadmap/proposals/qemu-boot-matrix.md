---
status: PROPOSED
raised: 2026-09-29
milestone_dependencies: none
decision_dependencies: 19
machine_requirements: none
specific_machine: none
needs_person: no
---
# A QEMU boot matrix: machine models, memory sizes, and absent devices

From a proposal calef received on 2026-09-29 (a QEMU virtual hardware compatibility lab), amended
against what the tree already has. QEMU is already the pinned instrument (`.qemu-version`,
`script/qemu-check`) and the CI suite's boot path, and §19 (architectural parity is a tenet)
already gates the three architectures. What is missing is variation along three axes, and refusal
along the rest.

The axes:

- Machine models. The runners speak `q35` (x86_64) and `virt` (aarch64, riscv64). This adds
  `microvm` (x86_64): virtio-mmio with no PCI, which makes PCI absence a tested property rather
  than an assumption.
- Memory. `-m 32M` and `-m 256M` exist as points today. A small, normal and large row per
  architecture exercises memory-map discovery, page-allocator init at the margins, and
  non-contiguous regions.
- Absence. Boot cells with no disk, no network, and no PCI. A kernel whose thesis is confinement
  must refuse loudly rather than hang when optional hardware is absent; today absence is only what
  a given runner happens to omit.

## Done means

Every cell passes or carries a scope note naming the gap and the plan, per §19's gate. The runners
stay the single front door (`script/test` over `cargo xtask`); each cell reports through
semihosting as the suite does today, and the CI summary names the cell that failed.

## Refused, with reasons

- A `nife qemu` / `nife test` CLI. The front door is `script/*` over `cargo xtask`; a second
  command surface duplicates it and mints a name the tree must then keep.
- `sifive_u`. Quirky emulation whose discovery path is close to `virt`; it buys little until a
  SoC-shaped board is the compatibility target.
- A QEMU-version axis. The tree pins one QEMU on purpose (`.qemu-version` is part of what the
  icount baselines mean); version drift is `script/qemu-check`'s job, not a matrix axis.

## Deferred until their triggers

`sbsa-ref` waits for milestone 578 (an ACPI discovery path for aarch64) to land, since SBSA tables
are that work's natural follow-on. Snapshot-based fault injection wants its own proposal when
fault injection is wanted; it fights the semihosting exit and the icount determinism conventions
today.

Name provisional. The device rows this matrix wants and the tree lacks drivers for are proposed
beside it: USB keyboards, an e1000 NIC, and a parked virtio-scsi.
