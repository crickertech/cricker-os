---
status: PROPOSED
raised: 2026-10-07
milestone_dependencies: 198, 809, 805, 23, 666, 592, 593, 53
decision_dependencies: unwritten
machine_requirements: x86_64 and riscv64 silicon with a writable boot disk and a hardware watchdog
specific_machine: radon (calef's preferred lab machine, and the one whose chooser, disk writes and watchdog are unbuilt)
needs_person: yes
---
# Lab machines update themselves through packages, and only a new kernel reboots them

calef asked on 2026-10-07 (UTC): *"We should have a milestone that lets our lab machines updated
themselves for runs using our package management solution. Download their updates, apply them, and
restart (if it is the kernel). Sound right? We want to be testing updates early."* Written by lane
`lab-self-update-proposal`, which built nothing and touched no lab hardware. Every name here is
provisional. Two appendices: [the lab machines](lab-machines-update-themselves/the-lab.md) and
[prior art](lab-machines-update-themselves/prior-art.md).

## A correction, first

The first version of this proposal updated the whole base set through a boot slot, with a reboot.
calef stopped it the same day: *"The whole system needs to be updatable through packages. The kernel
will need a reboot for now and that is fine. However should base updates require reboot? Why?"*
Then, to the model below: *"Yes. We have discussed this before so it definitely needs updating in
the tree."*

He had. §159 (only a new kernel needs a reboot) ruled on 2026-09-19 that *"Upgrading a userspace
component does not reboot the machine"*, and §208 (installing is granting) built on it four days
later. The tree drifted from both, and this proposal followed the drift. §235 (the OS is built and
updated from packages) put a full copy of the base in each slot. §229 (how a bare name reaches an
installed program) refused any package that names a base program, because a slot would update it.
§241 (a threadbare base) kept twelve programs whose only update was a reboot. Milestone 198 (a package
manager) never carried the exit criterion §159 gave it. None of them cited §159. This lane's commit
records the ruling as §159's amendment of 2026-10-07 and points each drifted record at it.

## The model, ruled

1. A boot slot holds the kernel and the root programs that nothing can restart: the progenitor, and
   the root supervisor once a real boot starts one. They update with a reboot, and a bad slot rolls
   back by its tries (milestones 525 and 554).
2. Every other program, base included, is a package that `jig` (milestone 809 (the package client
   becomes a program)) installs. It takes effect when a supervisor restarts it from the new bytes,
   or by a live swap (milestone 23 (a capability-routed component OS with live replacement)).
3. A base package that needs a new kernel ABI rides the kernel's reboot.
4. A service whose state cannot be handed over is restarted, not rebooted.
5. A bad package rolls back with `jig rollback`, §208's generations. A bad slot rolls back as built.

Proteos, the MINIX 3 rewrite, is the closest precedent: every process updates in a transaction and
only the kernel reboots. Fuchsia is the contrast, where the whole base is an over-the-air update
and a reboot (appendix).

## What the tree has against the model

Checked rather than assumed. Each is a gap the model must close before a lab machine can use it.

1. **No package can update a base program without a new slot today.** Install checks a package's
   digest against the image's catalog (`notes/packages.md`), which the slot carries. §241 says so:
   until §220 (signed builds) is built, a new version of any package is a new catalog, which is a
   new slot. §220's build is milestone 666 (a signed build installs up to its key's ceiling),
   NOT-STARTED. This is the hard dependency.
2. **§229's refusal** is built (`activation_set::Error::ImageName`) and must narrow to slot programs.
3. **The progenitor builds every service from the archive.** `crates/system_initializer` names each
   by archive entry. Starting a base service from the activation set is what §241's B2 required and
   nothing has built.
4. **The kernel itself starts `block_driver` and `redoxfs_server`** (`kernel/src/user.rs`, §241's
   fact 5). Nothing can restart those, so in effect they are slot programs until §235's P6 moves the
   spawns to the progenitor.
5. **No real boot starts `root_supervisor`.** Only `authority_tests` does. Today the progenitor is
   the root, and the slot holds the kernel and the progenitor.
6. **The restart tier is unbuilt.** Live swap exists for `line_editor` alone (§232 (the line editor
   swap contract)). §241's table lists `net_stack`, `entropy`, `clock`, `login` and the display stack
   as "at the next boot". §209 (state handoff is an opaque blob, and optional) gives the handoff a
   shape; nothing a real boot runs uses it.
7. **The manifest note has no ABI revision field.** §235 ruled one; nothing writes it, so "needs a
   new kernel ABI" cannot yet be read off a package.

## What to build, in order

1. Narrow the slot. The progenitor starts every base service from the live generation, and the
   archive carries only the kernel's boot set. The kernel's two spawns move to the progenitor (P6).
2. Accept a base package update (Fork 2). Narrow §229's refusal; check the ABI revision.
3. Apply it (Fork 3): a supervisor restart, a live swap, or the next spawn, by how the program lives.
4. Confirm it or roll it back (Fork 4).
5. A slot writer for the kernel and progenitor, and a confirmation that the machine can take the
   next update.
6. The chooser on riscv64 (Fork 7), and a watchdog on every lab machine.
7. The lab channel (Fork 8) and the run record.

Most of 1 to 4 is milestone 198 and 809's work, which the lab is the first customer of, as §159
said. This milestone is the lab's half: items 5 to 7, and the gates that prove 1 to 4 on silicon.

**Reuse:** `crates/boot_slot`, `uefi_loader`'s chooser, `crates/activation_set`, `crates/component_plan`'s
dependency graph, the `reboot` program, `helpers/package-http-peer` and basalt's gate. MINIX 3's
service update, QNX's HAM, Mender, RAUC and SWUpdate were considered for their state machines, not
their code: each assumes an OS or a bootloader nife does not have.

## Safety, for machines nobody is standing at

calef's rules bind every step: plug 3 is never switched, radon's USB hub is never unplugged, and no
lane powers, boots or reaches radon or xenon. So each machine recovers by itself, or stops somewhere
safe to leave.

The model moves most risk off the slot, which is the point. A bad package costs a restart and a
generation rollback, not a boot. Three risks remain.

- A hang after a kernel update is recovered only if something resets the machine. The chooser
  spends a try before handoff, but nobody power-cycles. xenon's TCO watchdog (milestone 593 (a
  wedged kernel resets itself)) is proven only under QEMU. radon has no watchdog step. xenon also
  halts at power-on without a keyboard (milestone 260 (boot xenon over the network)).
- A package can break the path to its own rollback: a bad `net_stack` cannot fetch, and a bad
  `swish` cannot type `jig rollback`. §241 kept those in the slot for that reason. Fork 1 replaces
  that floor.
- radon's microSD card is its only boot medium, and nife cannot yet write it (milestone 53 (the board's own peripherals), PARTIAL).

No machine takes an unattended update until its watchdog is proven on its own silicon, and a doomed
kernel and a doomed package have each been rolled back on it once with calef watching.

## The run records the update

A failed update is a test failure. The run log names the slot build and generation the machine
started on and was offered. The banner prints no commit and the slot header holds a length and a
CRC, so both gain the build's identity. It then gives one outcome per step: `confirmed`, `rolled back
by tries`, `rolled back by the chooser`, `package rolled back` (naming the service that failed its
check), `fetch refused`, or `not offered`. patagonia already captures both machines' serial
(`script/board-console`). The new part is a parser that turns those lines into a verdict, the way
`swish-check` reads the prompt.

## Fork 1: what replaces the floor when a package breaks its own rollback

- 1a. Generations get tries, like slots. The progenitor marks a new generation on trial, and if the
  boot's required set (`console`, `input`, `line_editor`, `swish`, `job_undertaker` today) fails
  or the boot is not confirmed, the next boot starts the previous generation. systemd's boot
  counting and greenboot do this for boot entries; this applies it one layer up.
- 1b. A small recovery set stays in the slot: a recovery shell and `net_stack`, used only when the
  generation fails. §241 noted this and did not rule it. It reopens the floor the model removed.
- 1c. A person with a stick or network boot. True today, and not unattended.

Recommend 1a, with the chooser's own image as the last resort it already is. It keeps the slot to
what nothing can restart, and it is the mechanism §208 already has, with tries added.

## Fork 2: how the progenitor accepts a base package update

calef's model needs the progenitor to accept a newer base program than its slot names.

- 2a. A trusted key vouches (§220, built by milestone 666), and the progenitor checks the package's
  ABI revision against the running kernel's. A package needing a newer ABI is staged, and activates
  on the boot that confirms the new slot.
- 2b. A signed index the progenitor verifies (809's option I3). Narrower trust, but every channel's
  index needs a key the progenitor holds.
- 2c. The slot's catalog stays the only vouch. Every base update is then a slot, which is the model
  calef refused.
- 2d. The owner's `vouch`. §229 refused letting a vouch claim a bare name, so a vouched base program
  could run only by path.

Recommend 2a. The ABI check is what makes model point 3 a rule rather than a hope. §229's refusal
narrows to the slot's programs in the same change.

## Fork 3: how new bytes take effect

- 3a. Immediately, by kind. A per-use program at its next spawn. A swappable component by live
  swap. A long-lived service by a restart in `component_plan`'s dependency order, the way NixOS
  restarts changed units. A service with state it cannot hand over is restarted and its clients
  reconnect.
- 3b. At the next boot, as Fuchsia does for its base. Simple, and the reboot calef asked to remove.
- 3c. `needrestart`'s way: install, then report what still runs old bytes, and the owner or the lab
  run triggers restarts.

Recommend 3a for the lab, because testing the restart is the point, with 3c's report printed either
way. The supervisor that restarts is the progenitor for the services it starts, until a real boot
runs `root_supervisor`.

## Fork 4: what marks an update good

- For a package: the restarted service reports ready, and a probe it answers passes, the way
  milestone 23's swap reports `PROBE_SURVIVED`. Fail, and `jig rollback` restores the generation and
  restarts the old bytes.
- For a slot: the shell answers, the network is up, and the lab index is fetched and its digest
  checked. Milestone 554 (a good upgrade sticks)'s file-server-ready is too early; the whole run passing would roll back a
  healthy kernel for an unrelated red test.

Recommend both, with the run's own result recorded and not gating. Fuchsia's committer and Android's
`update_verifier` confirm what the next update needs, and so does this.

## Fork 5: one step or two for a kernel update

- 5a. Two steps. The slot is confirmed, then the generation moves.
- 5b. One step. A generation records the slot build it needs; it activates on the boot that confirms
  that slot, and a slot rollback restores the previous generation.

Recommend 5b. Model point 3 makes them depend on each other, and 2a's ABI check is what reads it.

## Fork 6: who decides when to update

- 6a. The machine polls its channel at boot and when idle, like `update_engine`, Mender and Fuchsia.
  Needs nothing inbound.
- 6b. The CI run pushes, LAVA's model. nife has no inbound shell, no workflow reaches the lab, and
  calef chose manual power for radon (milestone 224 (nothing can power-cycle radon)).
- 6c. The boot server picks, for netbooted machines. A netbooted machine installs nothing and
  spends no try, so it tests none of this.

Recommend 6a. Network boot stays the recovery and bench path. Which mode a machine is in is calef's
switch, since it changes his lab workflow.

## Fork 7: how radon and argon choose a slot

- 7a. `uefi_loader`'s chooser through U-Boot's `bootefi`. The loader builds for riscv64 and
  aarch64 already (`notes/boot-stick.md`), so all three machines share one chooser and one format.
  Whether radon's U-Boot 2021.10 has `bootefi`, and whether its block I/O writes: unverified.
- 7b. U-Boot's `bootcount`, kept in radon's QSPI environment, written on every trial boot.
- 7c. A tries file on the card's FAT, written by a U-Boot script with `fatwrite` (unverified).

Recommend 7a, with 7c as the fallback. 7b writes the flash radon cannot boot without. Under the
model, the slot changes only with the kernel, so this matters less than it did.

## Fork 8: where the lab channel lives and when it advances

basalt pins nife by commit and bumps the pin daily at 06:17 UTC. It publishes nothing a machine can
fetch: its gate's builds are 14-day workflow artifacts. No channel exists.

- 8a. A `lab` index at `basalt.nifeos.org`. Waits on the index path (§250 (an image names its
  distribution's package index)), TLS (milestone 801 (packages over the internet)), and calef publishing.
- 8b. A `lab` index on patagonia over the LAN, served by `helpers/package-http-peer`, the same
  format as 8a.
- 8c. Package bytes as GitHub Releases on basalt, the index on patagonia.

Recommend 8b now and 8a later. The channel advances on each basalt gate that passes, with the pin
bump triggered per nife merge. The runner minutes that costs are not measured.

## Scope note (rule 5)

The package path is architecture-free. The chooser is x86_64 today; Fork 7 adds riscv64. argon is
the TX1 once it arrives and waits on milestone 803 (argon boots the aarch64 kernel); the board in
hand is a TK1 that nife cannot run. radon's and argon's watchdogs are milestone 593's later steps.

## BUGS

- Nothing here has run on silicon. The x86_64 rollback is proven under OVMF only.
- A service that passes its probe can still be subtly wrong. That is the run's job.
