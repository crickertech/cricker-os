# The fork: UEFI or GRUB, decided by two commands

An appendix to [notes/x86-uefi-boot.md](../x86-uefi-boot.md), which keeps the decision. This file
holds what each option cost when it was priced.

Both options were real, both were priced, and this is what the pricing found. The rule the tenets
give here is *recommend on reversible forks*. The two paths can coexist: a GRUB Multiboot 2 header is
thirty lines and does not disturb what is below. So this is a recommendation acted on, rather than a
question sent up.

## What UEFI costs

```console
$ ls /opt/homebrew/share/qemu/ | grep edk2-x86_64
edk2-x86_64-code.fd
edk2-x86_64-secure-code.fd
```

OVMF, the open-source UEFI implementation, ships with the QEMU this project already pins. Nothing to
install. And the FAT filesystem the firmware reads is QEMU's own `vvfat` block driver, which
synthesizes one out of a host *directory*. So there is no image-building step and no `mtools`:

```console
$ command -v mformat xorriso
$                                   # neither is installed
```

On the machine end, the OptiPlex 7050 is UEFI-native. So the operational cost is copying one file to
a FAT32 stick as `/EFI/BOOT/BOOTX64.EFI`. That path is the removable-media fallback every UEFI
implementation looks for with no configuration at all.

The engineering cost is what §46 (thin primitives or whole subsystems) would have to weigh if a
dependency were involved, and there is none. This loader speaks six firmware functions and two GUIDs,
hand-written in `uefi_loader/src/efi.rs`. The `uefi` crate was not taken. The reason is that crate's
size against this need, rather than anything about its quality.

## What GRUB costs

```console
$ brew info grub
Error: No available formula with the name "grub".
```

GRUB cannot be installed on this development machine at all. Homebrew has no `grub` formula on macOS,
so `grub-mkrescue` cannot be run here, and neither can `xorriso`, which it needs. The GRUB path could
still be *written* on patagonia, but it could not be proved on patagonia. Gating it would mean a Linux
container in the loop for every run, or building GRUB for the `x86_64-efi` target from source.

On the machine end GRUB also costs calef more, not less: a bootloader installed on the box, or a
rescue ISO written to a stick with a tool he does not have.

## The decision

UEFI. It is testable today with what is installed, it is the shorter path to the machine, and it is
what the target hardware natively is. GRUB Multiboot 2 stays available and cheap to add if a
BIOS-only machine ever turns up. The 32-bit trampoline in `kernel/src/arch/x86_64/boot.s` is already
the entry state GRUB delivers, so the delta would be a header and a second handoff decoder.

And the Multiboot hazard was checked rather than assumed. `notes/x86-port/boot.md` records QEMU
refusing an image over a Multiboot 1 header, fatally. Nothing in this milestone adds a Multiboot
header of any version, so that hazard is untouched. The PVH note is still the only boot header in the
image, and `script/test --arch x86_64` still boots through it.
