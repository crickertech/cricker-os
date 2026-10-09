# The lab machines, read rather than assumed

An appendix of [lab machines update themselves](../0858-lab-machines-update-themselves.md). Read from the tree on
2026-10-07 (UTC). No lab machine was powered, booted or reached to write it.

| | radon | xenon | argon |
|---|---|---|---|
| machine | VisionFive 2, JH7110, riscv64 | Dell OptiPlex, Q270, x86_64 | not in hand |
| how it boots nife today | QSPI U-Boot 2021.10 runs a script from the microSD card's FAT; the script TFTPs kernel and archive from patagonia and falls back to the card (milestone 257 (boot radon over the network), BUILT) | a stick, or since 2026-10-04 its own NVMe disk through `uefi_loader`'s chooser (milestone 198 (a package manager), rung 2b); network boot is milestone 260 (boot xenon over the network), PARTIAL, one photographed power cycle short | |
| can it choose a slot | no: the chooser is `#[cfg(target_arch = "x86_64")]` in `uefi_loader/src/main.rs` | yes, `uefi_loader/src/chooser.rs`, proven under OVMF by `cargo xtask rollback-boot` and `confirm-boot`; never on xenon's firmware | |
| can nife write its disk | not yet: `crates/designware_mobile_storage` (milestone 53 (the board's own peripherals), PARTIAL) is unproven on silicon and writes only before the first partition by design | yes, NVMe | |
| can it restart itself | `reboot` is built (milestone 805 (`reboot` at the prompt)), but radon's reset hangs in OpenSBI's PMIC write until milestone 592 (radon's cold reboot dies in OpenSBI's PMIC write)'s bench run passes | `reboot` built; xenon's reset untested on silicon | |
| does anything reset it when it hangs | no: milestone 593 (a wedged kernel resets itself) has no JH7110 step | the Intel TCO, built and proven under QEMU only (593 step 1) | |
| power without a person | plug 2, manual by calef's choice (milestone 224 (nothing can power-cycle radon)) | none (milestone 653 (xenon may carry Intel AMT), NOT-STARTED); also halts at POST with no keyboard (260) | |

argon is corrected here. The board delivered as argon is a Jetson TK1 (tegra124, 32-bit ARMv7), which
nife cannot run, and it is going back; argon will be the TX1 (tegra210, aarch64) once it arrives
(`notes/bench-runbook.md`, "argon is not in hand"). Even then, the aarch64 kernel fits only QEMU
`virt` until milestone 803 (argon boots the aarch64 kernel) is built. argon is a scope note in this
proposal, not a member of the first rung.

