# The UEFI boot path's limitations, in full

An appendix to [notes/x86-uefi-boot.md](../x86-uefi-boot.md), whose BUGS section keeps the short
list. This is every entry as it was written, sentence-split for the prose limits.

- The bench procedure's firmware steps are now checked against the machine; the rest of it is still
  one run old. The old entry here said every firmware-menu path, key and setting name came from the
  7050's documented behavior rather than from this machine. It said the first person to follow it
  should expect one of them to be worded differently on the screen. That prediction was right, once:
  step 3's "Serial port: enabled" is a single five-way radio with no enable, and picking `COM1` is the
  whole of it. Steps 1, 2 and 4 were correct as written. `notes/xenon-firmware.md` is the full
  transcription, and is what a bench session should read instead of guessing. What is not retired:
  the F2 and F12 keys, the `\EFI\BOOT\BOOTX64.EFI` fallback and the triage table are still from one
  successful run and Dell's documentation, not from repeated use.
- The kernel is placed at one address chosen at link time, and 32 MiB is not a guarantee. It clears
  every low reservation OVMF makes and nothing more; a firmware that wants that range refuses the
  boot. The image is not physically relocatable, and making it so is a milestone rather than a
  constant. `.boot` is linked at its physical address, because a 32-bit instruction stream cannot
  name a 64-bit one. Its absolute self-references are what would have to become
  position-independent.
- The bench procedure still boots one core, unlike the runner. Nothing stops two, and two work under
  OVMF; the procedure is written for a first bring-up where every variable costs.
- The suite under firmware runs at one core. So nothing exercises AP bring-up under UEFI *and* the
  scheduler's cross-core tests together. That is `ap_boot`'s open two-core defect rather than anything
  about firmware, and it is the PVH runner's situation too.
- Nothing verifies what the loader hands over. The kernel and the archive are bytes the loader was
  compiled with, so the trust boundary is the build. `measured_boot`'s manifest is not consulted, and
  the image is not signed. That is also why Secure Boot has to be off.
- A stale `.efi` on a stick is silent when the stick was made by hand. The loader embeds the kernel,
  so a stick that was written last week boots last week's kernel with nothing to say so.
  `cargo xtask uefi-image` rebuilds both every time, which moves the hazard to the copy step rather
  than removing it. A stick written by `stick_maker` carries `NIFE.TXT` with the build and each file's
  digest (notes/boot-stick.md), which makes it checkable, not fresh.
- `uefi-test` can go red after its own suite has passed, and the message around it points at the
  wrong cause. The sixth stranger run of milestone 117 (the stranger test) (2026-09-19, on the dev Mac, two other lanes gating
  beside it) saw the kernel suite under OVMF print `test result: ok. 215 passed, 71 skipped`. Then it
  printed `uefi-test: qemu exited Some(1), not 3`, so `script/test` failed at its last step. The lines
  QEMU printed just before it, `vtd_iova_to_sspte: detected sspte permission error` and
  `vtd_iommu_translate: detected translation failure`, read as the cause and are not. They are the
  deliberate DMA-escape tests' faults. A clean re-run of `cargo xtask uefi-test` an hour later, on the
  same machine and the same Homebrew QEMU 11.1.1, printed the same two lines and passed. The stranger
  concluded the QEMU version was to blame, which that re-run does not support. That the two lines are
  expected is written in the block of milestone 215 (a PCI function's interrupt reaches nothing on
  x86_64), and nowhere a reader meets the failure. What made QEMU
  exit 1 rather than 3 is unmeasured: one red in two runs is a rate nobody has taken yet.
