---
status: PROPOSED
raised: 2026-10-07
milestone_dependencies: 198, 809, 805, 592, 593, 568, 53
decision_dependencies: unwritten
machine_requirements: x86_64 and riscv64 silicon with a writable boot disk and a hardware watchdog
specific_machine: radon (calef's preferred lab machine, and the one whose chooser, disk writes and watchdog are unbuilt)
needs_person: yes
---
# Lab machines update themselves from a lab channel, and roll back without a person

calef asked on 2026-10-07 (UTC): *"We should have a milestone that lets our lab machines updated
themselves for runs using our package management solution. Download their updates, apply them, and
restart (if it is the kernel). Sound right? We want to be testing updates early."* He approved a
proposal with forks. Written by lane `lab-self-update-proposal`, which built nothing and touched no
lab hardware. Every name here is provisional. Prior art, read from primary sources on 2026-10-07, is
in the appendix, [prior art](lab-machines-update-themselves/prior-art.md).

The short answer is yes, and most of the hard design is already ruled. §235 (the OS is built and
updated from packages) ruled the shape on 2026-09-27: two boot slots, each a full copy (U4).
Milestones 525 (a bad upgrade cannot brick the machine) and 554 (a good upgrade sticks) built the
slots, the tries and the confirmation on x86_64. §208 (installing is granting) made installed
packages a versioned activation set. What is missing is the program that writes a slot on a running
machine, a source to fetch from, the same chooser on the device-tree machines, and something that
resets a hung machine. The last two decide whether "without a person" is true.

**Reuse:** `crates/boot_slot`, `uefi_loader`'s chooser, `system_installer`'s block access, the
`reboot` program, `helpers/package-http-peer` and basalt's gate, all unchanged. Mender, RAUC and
SWUpdate were considered and not taken: each is a Linux userspace client over a U-Boot or GRUB
integration, and nife has neither a Linux userspace nor a bootloader it does not write. Their state
machines are taken; their code is not.

## The lab, read rather than assumed

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

## What the dependencies give, and what they do not

- Milestone 809 (`jig`), NOT-STARTED, is the client: `install`, `update`, `outdated`,
  `rollback`. Its block says plainly that no upgrade verb is proposed, and it installs into the
  activation set, never into a slot. The progenitor refuses an install that names a base program:
  *"a new base updates it, not install"* (`notes/packages.md`). So the kernel and the base set are
  not something `jig` can update today, by design.
- §251 (restarting the machine is a kernel object) and milestone 805 give a restart: one kernel object, held by exactly one program,
  `reboot`, which syncs and then resets. `grant_plan::image_can_carry` keeps the object off every
  installed image.
- The writable root is RedoxFS, made `base` by calef's ruling on #1797 fork 3 on 2026-10-07.
  That holds the activation set and a download area. The slots are not in it: they are raw GPT
  partitions behind a 4096-byte header (`notes/boot-slots.md`).
- Milestone 198 is PARTIAL. Rung 3a fetches a package over the LAN by plain HTTP and verifies it
  by digest (§195 (a reviewed recipe vouches for a package)); TLS to an internet host is rung 3c.
- basalt pins one component, `nife`, by commit (`pins.toml`), and its gate builds that commit
  and runs nife's own system test under QEMU. Its pin bump runs daily at 06:17 UTC, not per merge.
  It publishes nothing a machine could fetch: the gate's kernels and archives are workflow
  artifacts kept 14 days. There is no channel of any kind yet.
- §250 (an image names its distribution's package index) names `basalt.nifeos.org` as the index host, but the index's path is not ruled, and
  agents publish nothing there.

So "a `lab` channel that advances on every merge" is new work on both sides, and Fork 6 is where it
lives.

## What to build

1. A slot writer. A program that fetches a slot image, verifies it against the digest the
   channel lists, writes the inactive slot, sets it to try (priority above the running slot, tries
   from the channel, not successful), and spawns `reboot`. It reuses `crates/boot_slot` and
   `system_installer`'s block access unchanged. It spawns `reboot` rather than holding the reboot
   object, so §251's "exactly one program" stays true. Whether this is a `jig` verb or its own
   program is a naming and authority question for calef; calef's "one program, one thing" ruling
   points at its own program.
2. A confirmation that means the machine can take the next update (Fork 3), moved from the
   kernel's `install_service::confirm` to that program or a sibling.
3. The chooser on riscv64 (Fork 5), then aarch64 once argon boots.
4. A watchdog on every lab machine (the Safety section).
5. The lab channel (Fork 6).
6. The run record (below).

## Safety, for machines nobody is standing at

calef's rules bind every step here: plug 3 is never switched, radon's USB hub is never unplugged,
and no lane powers, boots or reaches radon or xenon. So the machine must recover from every failure
by itself, or stop in a state that is safe to leave until calef next sits down.

What already holds, on x86_64. The chooser spends a try before it hands off, so a hung image is
abandoned on the next boot. A failure the chooser can see (bad header, bad CRC, a PE that will not
start, an image that returns) moves to the other slot within one boot. And the chooser is itself a
complete image, so two bad slots still boot.

What does not hold, and is the actual blocker:

- A hang is only recovered from if something resets the machine. The decrement is on the disk,
  but nobody is there to power-cycle. On xenon the TCO watchdog (593) is that something, proven only
  under QEMU. On radon there is no watchdog step at all. Between `ExitBootServices` and the kernel
  arming the TCO, nothing watches: UEFI's own boot-services watchdog ends at that call (UEFI 2.10,
  `SetWatchdogTimer`; recalled, not read for this proposal). Recommend the chooser arm the hardware
  watchdog before handoff where it can, and the kernel take it over.
- The chooser itself is never updated by this design, so a fix to the chooser still needs a
  person with a stick. That is the price of the chooser being the last known-good image, and it
  should stay that way: the slot writer gets no capability to the EFI system partition, the same
  confinement `ROLE_CONFIRM` has today.
- Power lost during the chooser's table write is unjournaled (`notes/boot-slots.md`). A lab
  machine on a smart plug nobody switches makes this rare, not impossible.
- radon's card is its only boot medium. A slot writer that corrupts it leaves radon at a U-Boot
  prompt. U-Boot in QSPI survives, and so does the TFTP path milestone 257 built, which is why
  radon keeps network boot as its recovery path rather than its normal one.

The order that follows: no machine takes an unattended update until its watchdog is proven on its
own silicon and a doomed slot has been rolled back on that machine with calef watching once.

## The run records the update

A failed update is a test failure, so a run's log must say:

- the slot and the build it started from, and the build it was offered. The banner prints no
  commit today and the slot header holds a length and a CRC, not a version, so the slot header gains
  the build's identity (the basalt pin, or nife's commit);
- the outcome, one of: `confirmed`, `rolled back by tries`, `rolled back by the chooser` (with the
  table row that caught it), `fetch refused` (digest), or `not offered`;
- for a rollback, the tries the failed slot spent.

patagonia already captures radon's and xenon's serial (`script/board-console`, under
`bench/<machine>-<date>/`). The new part is a parser that turns those lines into a pass or fail,
the way `swish-check` reads the prompt. A rollback is a failure of the update under test even
though the machine is healthy, which is the point of the exercise.

## Fork 1: what an update writes

Mostly ruled. §235 compared the shapes and took U4: packages are the download unit, and each slot
holds a full copy. A/B slots with ChromeOS's priority, tries and success bits are built. Whole-system
generations in the OSTree or Nix sense are the U2 shape §235 refused, because a mistake in a shared
store's collection rule bricks both slots.

What is left is order. U4 needs P1 (the image assembled from base packages by digest) and an update
manifest, and neither exists.

- 1a. Write the whole image as one slot payload now, on the lab only. This is U1, which §235 did
  not take, used as a stage and marked as an exception where a reader meets it. The slot layout,
  the chooser, the tries and the confirmation are identical under U1 and U4, and those are the
  parts that can strand a machine.
- 1b. Wait for P1 and P5 (an OS update is a set of packages that lands through a slot).
  Tests the real format the first time, later.

Recommend 1a. It is not an effort argument: the risky half is the same in both, and calef asked to
test early. When P5 lands, the lab moves to it and the exception is retired. Reversible: nothing
outside this house has fetched a slot.

## Fork 2: who decides when to update

- 2a. The machine polls its channel, at boot and on an idle timer. This is Android's and
  ChromeOS's `update_engine`, Mender's client and Fuchsia's `system-updater`. It needs nothing
  inbound.
- 2b. The CI run pushes. LAVA's model: a controller deploys each job and power-cycles through
  a PDU. nife has no inbound shell, no self-hosted runner reaches the lab (no workflow in
  `.github/workflows/` names radon or xenon), and calef chose manual power for radon. It would also
  test the controller's deploy path, not the machine's update path.
- 2c. The boot server decides, for netbooted machines. radon already boots this way and xenon
  nearly does. Pixiecore's API is the shape: the server picks the image per MAC. But a netbooted
  machine writes no slot and spends no try, so it tests none of what calef asked to test. And a
  server that rolls back needs the success report the machine would have written to its own disk.

Recommend 2a. Keep network boot as the recovery path and the bench path. A lab machine is either
following the channel, booting its own disk, or on the bench, netbooting an experiment, and the
U-Boot script and xenon's boot order say which. That mode switch is calef's to flip, not a lane's.

## Fork 3: what marks a boot good

Every system in the appendix confirms after a health gate, never at the bootloader. They differ on
how late.

- 3a. Today's criterion (milestone 554 (a good upgrade sticks)): the file server mounts the installed disk and reports
  ready, before the progenitor runs. Early, and blind to the network and the shell.
- 3b. The machine can take the next update: the shell answers, the network stack is up, and the
  lab channel's index is fetched and its digest checked. This is Fuchsia's `system-update-committer`
  and Android's `update_verifier` in spirit: confirm what the next update needs.
- 3c. The whole run passes. Couples test failures to rollbacks: a red test in an unrelated
  program would roll back a healthy kernel, and the lab would stop finding the bugs it exists to
  find.

Recommend 3b, with the run's own result recorded beside it and not gating it. 3b's failure is the
one an unattended machine cannot recover from: it boots, and it can never be updated again.

## Fork 4: whether packages and the kernel move in one step

The tree has already split them. Under T1 (today) the kernel's `TRUST_ROOT` pins every base program,
so the kernel and the base set ship together in a slot. Installed packages live in §208's activation
set, which has its own generations and its own `jig rollback`.

- 4a. Two steps, slot first. The slot is confirmed, then the activation set moves to the
  channel's generation. Each rolls back by its own mechanism.
- 4b. One step. The slot names the generation it expects, and a rollback of the slot restores
  that generation. Nix's generation and Fuchsia's update package both bind the two (appendix).

Recommend 4a for the first rung, because the lab machines carry no installed packages yet and 4b's
binding is a format that should be designed once P5 exists. Record in the activation set which slot
build each generation was made under, so 4b can be added without a migration.

## Fork 5: how radon and argon choose a slot

- 5a. `uefi_loader`'s chooser, through U-Boot's `bootefi`. The loader already builds for
  riscv64 and aarch64 (`BOOTRISCV64.EFI`, `BOOTAA64.EFI`, `notes/boot-stick.md`) and booted under
  EDK2. The same code, the same GPT bits and the same gates on all three architectures: rule 5 by
  construction. Unknown: whether radon's U-Boot 2021.10 has `bootefi` (bench item 10 in
  `notes/visionfive2.md` is unanswered) and whether its EFI block I/O writes (unverified). Needs the
  `boot_slot::cmdline` word through `/chosen`, which 554 says every `hand_over` already accepts.
- 5b. U-Boot's own `bootcount` (`CONFIG_BOOTCOUNT_LIMIT`, `bootlimit`, `altbootcmd`), as Mender
  and RAUC use it. The count lives in the U-Boot environment, which on radon is the QSPI flash it
  boots from, written on every trial boot. Whether the vendor build enables it is unverified.
- 5c. A U-Boot script that keeps tries in a file on the card's FAT, via `fatwrite`. RAUC's
  `contrib/uboot.sh` is this shape with environment variables. Whether `fatwrite` is built in is
  unverified.

Recommend 5a, with 5c as the fallback if `bootefi` fails at the bench. 5b writes the flash radon
cannot boot without. Two choosers with two state formats is the parity gap rule 5 forbids. This
also answers milestone 568 (the boot file has nowhere to go on a device-tree machine) for the device-tree machines' slots. The chooser does not need its own
file, only the slot images, so 568's boot-file handoff stays the installer's problem.

## Fork 6: where the lab channel lives and when it advances

- 6a. A `lab` index at `basalt.nifeos.org`. The real path. Waits on the index path ruling,
  §196 (nife carries TLS)'s TLS (198 rung 3c), and calef publishing.
- 6b. A `lab` index on patagonia, over the LAN. `helpers/package-http-peer` serves rung 3a
  today, digest-verified and plain HTTP, which §195 permits. Index format identical to 6a's, so
  moving is changing one URL.
- 6c. GitHub Releases on basalt for the bytes, with the index on patagonia. §250 says the bytes
  may live anywhere.

When it advances: on every basalt gate that passes, with the pin bump triggered by each nife merge
rather than daily. A lab machine then never fetches a commit that failed QEMU, and nife's merge
queue already gates `main`, so the basalt gate is the second check, not the first. The runner
minutes are not measured here; one gate run per merge is the cost to measure first.

Recommend 6b now, 6a when rung 3c and the index path exist.

## Scope note (rule 5)

The slot writer, the confirmation and the run record are architecture-free. The chooser is x86_64
today; Fork 5 brings riscv64; aarch64 follows milestone 803 and argon's arrival, through the same
`BOOTAA64.EFI`. radon's watchdog and argon's are milestone 593's later steps. Until each lands, that
machine follows the channel only with calef watching.

## BUGS

- No claim here has run on silicon. The x86_64 rollback is proven under OVMF only.
- The lab's first rung is U1, not U4 (Fork 1), and says so.
- A confirmation that reaches the index cannot tell a correct kernel from a subtly wrong one that
  still fetches. That is the run's job, which is why the run records the update.
