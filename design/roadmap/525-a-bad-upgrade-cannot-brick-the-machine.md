---
status: BUILT
raised: 2026-09-21
built: 2026-09-21
---
# 525. A bad upgrade cannot brick the machine: two boot slots, tries and priority

*(Number provisional until the merge queue lands it; 524 onward is contested
between branches in flight.)* Built 2026-09-21 by the lane `abboot/tries-and-priority`, on calef's
ruling of the same day: *"Yes, write the tries and priority attributes in 2b."*

This is the design half of rung 2b of milestone 198 (a package manager, and the trivial install that
makes a second customer possible). The bench half is not this: 2b's own row asks for the rung 2a
sequence on xenon's disk, photographed, and that is gated on calef wiping the disk. What is here is
built and proven under OVMF and waits on hardware for nothing but the photograph.

## The problem, stated so a stranger can check it

Milestone 198 rung 2a put nife on a disk. A machine with one copy of its boot image has a property
nobody would accept from a system they depend on: **the next upgrade is one bad write away from a
machine that does not start**, and recovering means somebody with a USB stick standing in front of
it. An appliance nobody can reach, which is what a home server is by the second week, has to be able
to undo its own upgrade.

So: two boot images, and if a newly written one fails to come up, the machine goes back to the
previous one by itself, with nobody at the console.

## What was built

| | |
|---|---|
| `crates/boot_slot` | the state and the policy: priority, tries, confirmed, and which slot to start. Pure computation, host-tested |
| `globally_unique_identifier_partition_table`'s `NIFE_BOOT` | the partition type that makes the attribute bits ours |
| `components/src/installer.rs` | lays out two slot partitions, fills the first, and writes its state |
| `uefi_loader/src/chooser.rs` | the selector: reads the state, spends a try, writes it, chain-loads the slot |
| `cargo xtask rollback-boot` | three boots that watch a doomed upgrade be abandoned |

## The four questions, answered where the code is and repeated here

Where the state lives. In the GPT attribute bits of the slot partitions. UEFI 2.11 section 5.3.3 gives
bits 48 to 63 to the owner of the partition's type GUID and tells everybody else to leave them
alone, so on a partition of a type that is ours they are ours, and no other operating system's
partition tool will touch them. ChromeOS's own positions, verified against the ChromiumOS
disk-format reference rather than recalled: priority 48-51, tries 52-55, successful 56.

And therefore, the correction this rung turns on. It was said on this project that *firmware*
would do the selecting. It will not. ChromeOS gets firmware-level selection because depthcharge is a
coreboot payload that replaces UEFI; generic UEFI, OVMF included, reads those bits for exactly no
purpose, because the specification just told it not to. So the bits are the right place for the
state and the selector has to be ours, which is why `\EFI\BOOT\BOOTX64.EFI` is now a chooser.

The other candidate, UEFI `Boot####` variables with `BootOrder`, was evaluated and refused: rung 2a
deliberately proved its boot with the firmware variable store deleted, so a design depending on
those variables surviving contradicts the property 2a exists to demonstrate.

Two files or two partitions. Two partitions, and the specification decides it rather than taste:
the attribute bits are per-entry and belong to a type's owner, so a slot has to be a partition of a
type that is ours. The distribution shape (one ESP, one `.conf` per kernel under `/loader/entries/`,
which is the Boot Loader Specification) puts the state in a file, and a file is something anything
that can mount the volume may rewrite. The state that decides whether a machine boots should not be.

Who decrements tries, and when. The chooser, before it hands off, and this is the crux
rather than an ordering detail. A chooser that only read, leaving the booted system to mark itself
good, would protect against an image that crashes visibly and not against one that hangs: a
machine wedged before userspace would retry the same bad image forever and show nothing to the
console nobody is standing at. Writing first costs a disk write on every trial boot. ChromeOS's
firmware decrements before the launch for the same reason.

What marks a boot successful, and how far in. Today: only the install, on the slot it just
wrote, and that claim is a record of an observation rather than an assumption, because the bytes
going into slot 0 are the bytes the firmware started that machine with seconds earlier. Nothing
marks a *trial* boot successful yet, and the consequence is exact: an upgrade that came up perfectly
still rolls back once its tries are spent. That fails safe and it means upgrades do not stick. The
criterion it should use, and the program that applies it, are milestone 554 (a good
upgrade sticks: what marks a trial boot successful), which built them.

What happens when both slots are bad. The machine boots the image in the chooser's own file,
which is a complete nife image the install wrote at the same moment as slot 0. Never-booting is a
worse outcome than booting something old, and this design gets the fallback for free because the
chooser and the image are the same binary. ChromeOS instead drops an exhausted slot's priority to
zero so its firmware's plain highest-priority scan moves on; nife's selection predicate already
excludes an exhausted slot, so the write is unnecessary and not doing it keeps the record of what
the priorities were.

## The proof, which is the deliverable

`cargo xtask rollback-boot`, three boots, no human in any of them after the install:

```console
--- boot 2 of 3: the machine tries the upgrade, and the upgrade never comes up ---
uefi_loader: boot slots on one disk, 2 of them
uefi_loader: starting boot slot 1
rollback-boot: killed at the handoff, which is what a hang looks like to the disk
--- boot 3 of 3: nobody touched anything ---
uefi_loader: starting boot slot 0
$ wc made-on-target
  1 10 57
rollback-boot: slot 1 is priority 3, 0 tries, not successful
rollback-boot: PASS
```

Killing the machine at the handoff is the test rather than a shortcut. A kernel that wedges, a
driver that spins and a power cut are the same event from the disk's point of view, which is that
the try was spent and nothing came back, and it is the one failure class the chooser cannot observe
for itself. The upgrade staged into slot 1 is a byte-for-byte copy of the working image on purpose:
garbage would be caught by the slot header's checksum inside one boot and would test the cheap half.

`cargo xtask install-boot` also grew two assertions, so an ordinary installed boot is now held to
going through the chooser.

## The one thing this found that was already broken

Every `Spawn::maps` page has cost a share of a revocation log page since `map_physical` started
recording on 2026-09-21. Four call sites were updated to pay for it and the installer's
eleven-megabyte boot-file window could not be, because it goes through `user::run` and `run` had
nowhere to put the number; it fitted inside `AS_OVERHEAD`'s slack by luck. Growing the boot image by
a megabyte took the luck away, and the failure was an `OutOfPageFrames` panic three frames from
anything that mentions memory. The accounting now happens in `user::load`, from `spawn.maps` itself,
so no caller has to remember.

The first version of it overcharged and the frame ledger said so, deterministically and on the
first run: charging the whole window took a frame per long-lived process, permanently, into a region
that never used it, and the aarch64 suite went from 22249 kept frames to 22317. A caller owes the
cost above what `AS_OVERHEAD`'s margin already carried, which is what `WINDOW_IN_OVERHEAD`
names. That gate is the reason this landed right rather than landing quietly wrong.

## Scope note (DECISIONS §19 (architectural parity is a tenet; the targets are aarch64, riscv64 and x86_64))

x86_64 only, and this is a narrowing of an existing one rather than a new gap. Rung 2a is an
x86_64 claim because a device-tree handoff has one initrd slot in `/chosen` and no second one for
the loader's own file; without that file there is nothing to install and nothing to put in a slot.
Nothing about the slot format, the bit layout or the policy is x86-specific, and `crates/boot_slot`
compiles and is tested on the host. What does not exist on aarch64 and riscv64 is the chooser, and
what blocks it is the same milestone that blocks the install, 568 (the boot file has nowhere to go
on a device-tree machine):
`design/roadmap/568-the-boot-file-has-nowhere-to-go-on-a-device-tree-machine.md`.

## BUGS

Each of these is also recorded where a reader meets the feature, which is where it belongs; they are
collected once here because a roadmap block is what somebody reads before deciding to trust this.

- The on-disk format is ratified (the attribute bit positions, the slot header's bytes and the `NIFE_BOOT` type GUID; the GUID and bits on 2026-09-21, the whole layout by DECISIONS §244 (the installed disk has four partitions, and a boot slot is 64 MiB) on 2026-10-03). It is a format two programs agree on, so changing it now costs an installed disk.
- **Nothing marks a trial boot successful.** Answered by milestone 554 (a good upgrade sticks:
  what marks a trial boot successful); it was the largest missing piece.
- **Nothing writes the second slot on a running machine.** There is no upgrader, so the rollback
  path is exercised by a gate and by nothing a person does.
- **Two disks carrying boot slots is a refusal, not a choice.** The chooser falls back to its own
  image rather than guessing which disk is the one it was started from; the fix is a device-path
  parser that `place_boot_file` already wants for its own reason.
- **A power cut during the chooser's table write is not survivable.** Four block ranges, no journal;
  the machine then falls back to the chooser's own image, which is safe rather than good, and
  nothing in this tree cuts power to a QEMU between two block writes.
- **Two slots cost 128 MiB** on every installed machine, upgraded or not, and the floor for an
  installable disk rose from about 600 MiB to about 700.
- **It has run on OVMF and on no real firmware**, which is 2b's bench half and calef's hands.

## Follow-on

- **Milestone 554.** (a good upgrade sticks: what marks a trial boot successful) what marks a
  trial boot good, the program that does it, and the wire the slot number crosses on.
- **Recorded.** An upgrader: the program that writes the spare slot and puts it on trial. It is
  milestone 198 (a package manager, and the trivial install that makes a second customer possible)'s
  own territory and should not be built before the row above is answered, or it would ship a machine
  that reverts every upgrade a few boots after it succeeded.
- **Milestone 198.** The bench boot, which is rung 2b's other half: the sequence on xenon's own disk,
  photographed, gated on milestone 261 (the NVMe driver leaves the kernel, on the machine that can finally confine it)'s
  disk wipe, which is calef's hands rather than a lane's.

## Index row

An installed machine keeps two copies of its boot image and the state that chooses between them in
GPT attribute bits the UEFI specification reserves to the owner of the partition type, which is why
a boot slot is a partition of nife's own type rather than a file. The chooser is
`\EFI\BOOT\BOOTX64.EFI` itself, because generic firmware does not read those bits and never will,
and it spends one of the chosen slot's tries and writes that to the disk before handing off, which
is the only thing that bounds an image that hangs rather than one that fails where somebody can see
it. `cargo xtask rollback-boot` watches a doomed upgrade be tried, killed at the handoff, and
abandoned, with the machine coming back on the previous image by itself.
