# Unattended update with rollback: the prior art behind the forks

An appendix of [lab machines update themselves](../lab-machines-update-themselves.md). Read from
primary sources on 2026-10-07 (UTC) unless marked. "Snippet" means the claim came from a search
result quoting the official page, not a full read.

## The pattern every one of them shares

A bootloader holds an attempt counter and spends it. The booted system clears it only after a
health gate passes. None of them handles a netbooted machine: all keep the state on local media.

## Android A/B

<https://source.android.com/docs/core/ota/ab>, <https://source.android.com/docs/core/ota/ab/ab_implement>

- Two slots. Per slot the bootloader keeps a successful flag, an unbootable flag and a retry count
  (fastboot: `slot-successful`, `slot-unbootable`, `slot-retry-count`). `set_active` clears
  unbootable and resets the count.
- The `boot_control` HAL: `markBootSuccessful()`, `setSlotAsUnbootable()`, `setActiveBootSlot()`.
- `update_engine` writes the inactive slot. After reboot, `update_verifier` checks the new slot
  through dm-verity, then marks it successful. A slot that fails falls back to the old one.
- Virtual A/B writes a copy-on-write snapshot merged after a successful boot
  (<https://source.android.com/docs/core/ota/virtual_ab>). Default retry count: unverified.

## ChromeOS

<https://www.chromium.org/chromium-os/developer-library/reference/device/disk-format/>

GPT attribute bits: 56 successful, 52 to 55 tries remaining, 48 to 51 priority (0 means not
bootable). Firmware decrements tries before booting. The OS sets successful and zeroes tries once
it is up. nife's `crates/boot_slot` uses these positions (`notes/boot-slots.md`). Which daemon marks
success: unverified.

## Fuchsia

<https://fuchsia.dev/fuchsia-src/concepts/packages/ota>,
<https://fuchsia.googlesource.com/fuchsia/+/refs/heads/main/src/firmware/lib/abr/README.md>

- `system-updater` fetches the update package; the paver writes the alternate slot, which is then
  priority 15, 7 tries. The bootloader checks the image against vbmeta.
- After boot, `system-update-committer` runs health checks (for example a BlobFs read). Pass: the
  slot is marked healthy and the other slot unbootable. Fail: it may reboot, by configuration.
- libabr has A, B and a recovery slot R, and never fails to choose: the fallback is R. It says
  marking success "should not be done by the bootloader except in response to an operator command."

## OSTree, rpm-ostree, greenboot

<https://ostreedev.github.io/ostree/deployment/>, <https://ostreedev.github.io/ostree/atomic-rollbacks/>,
<https://github.com/fedora-iot/greenboot>

- A deployment is a checkout under `/ostree/deploy/`, with one Boot Loader Spec entry each; switching
  versions is switching entries. OSTree has no boot counting and points to greenboot.
- greenboot runs `required.d` and `wanted.d` checks before `boot-complete.target`, counts boots in
  GRUB's `boot_counter`, and rolls back with `rpm-ostree rollback`. The exact trigger value:
  unverified.

## NixOS and systemd boot assessment

<https://nixos.org/manual/nixos/stable/>, <https://systemd.io/AUTOMATIC_BOOT_ASSESSMENT/>, the
`systemd-boot.nix` module in nixpkgs

- Each generation is a boot entry; `nixos-rebuild --rollback` returns to the previous one.
- `boot.loader.systemd-boot.bootCounting.enable` (off by default, 3 tries): a `+<left>-<done>`
  suffix on the entry's file name, decremented by systemd-boot, removed by `systemd-bless-boot`
  once `boot-complete.target` is reached. An entry at zero sorts after the others.

## Mender, RAUC, SWUpdate

- Mender requires U-Boot's `CONFIG_BOOTCOUNT_LIMIT` and `CONFIG_BOOTCOUNT_ENV` for rollback, and an
  `ArtifactCommit` step that disables the bootloader's rollback
  (<https://docs.mender.io/operating-system-updates-yocto-project/board-integration/bootloader-support/u-boot/manual-u-boot-integration>;
  the commit step is a snippet).
- RAUC keeps `BOOT_ORDER` and `BOOT_<slot>_LEFT` in U-Boot's environment, decremented by a board
  script, and `rauc status mark-good` confirms, usually from a service ordered after the critical
  ones (<https://rauc.readthedocs.io/en/latest/integration.html>).
- SWUpdate sets `ustate=1` after install; the bootloader sets 3 on fallback (snippet).

## U-Boot bootcount and UEFI BootNext

- U-Boot (<https://docs.u-boot.org/en/latest/api/bootcount.html>): `bootcount` increments each
  boot and is saved only while `upgrade_available` is set; past `bootlimit`, `altbootcmd` runs.
  Stored in the environment, or a FAT or ext file with `CONFIG_BOOTCOUNT_FS`. Something in the OS
  must reset it.
- UEFI 2.10, section 3.1.2 (<https://uefi.org/specs/UEFI/2.10/03_Boot_Manager.html>): `BootNext` is used
  once and deleted before control passes. nife refused firmware variables for slot state
  (`notes/boot-slots.md`).

## Netboot and test labs

- Pixiecore's API (<https://github.com/danderson/netboot/blob/main/pixiecore/README.api.md>): the
  server answers each MAC with a kernel, initrd and command line. No success report, no fallback;
  rollback is the server's to build.
- LAVA (snippets from docs.lavasoftware.org): power through `power_on_command` and
  `hard_reset_command` via PDUDaemon; a failed health check marks the device Bad. Recovery is a
  power cycle and a fresh deploy per job, not on-device slots.
- KernelCI's labs: not checked.
