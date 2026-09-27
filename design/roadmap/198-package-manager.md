---
status: PARTIAL
raised: 2026-08-30
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 198. A package manager, and the trivial install that makes a second customer possible

Minted 2026-08-30 by calef. *(Number provisional until the merge queue
lands it.)* Rung 3a's producer half was built 2026-09-23 on
`milestone/198-the-next-rung`: the one archive file §197 ruled a package is
(`crates/package_archive`, written and read by one definition), `cargo xtask package` turning a
reviewed recipe into a package, its digest and a catalogue line, and the fuzz target and Kani
harnesses §197 accepted as the container's price. Rung 3a's consumer half is built through "verified by digest" (2026-09-24): the image
carries a package catalogue, a host serves the package over plain HTTP, and `net_stack`'s client
accepts it only by the image's digest, on aarch64 and riscv64. The table §208 (installing is
granting) versions is in `crates/activation_set`. Run by digest and the installer followed on
2026-09-26, once §219 (how the shell names an installed program to the spawner) was ruled:
`package install`, `remove` and `rollback` at the prompt, across a reboot, on all three
architectures. `package install <name>` fetches over the booted system's network (aarch64,
riscv64), and `greeting`, which no image carries, runs on all three. So does §219's gate D2: a
session holding the run-unvouched capability runs bytes nobody vouched for. §221 (the boot prompt is
the owner's console) followed: `vouch`, promoted from the proposal vouch-for-a-local-build, and
the owner's list of who may run new code. notes/packages.md has the account.

[§219](../decisions/219-naming-an-installed-program-to-the-spawner.md) (how the
shell names an installed program to the spawner) was decided 2026-09-26 (UTC): option D with gate
D2, all built. The first cut took §219's two open
recommendations provisionally (the table's shape, and where a manifest travels, §197). Every earlier fork is ruled:
[§195](../decisions/195-a-recipe-vouches-and-the-owner-may-overrule.md),
[§197](../decisions/197-a-package-is-one-archive-file.md),
[§208](../decisions/208-installing-is-granting.md) and §196 (nife carries TLS). The rulings of §157
(a trivial install is a web page, a USB drive, and packages over the internet) gate later rungs,
each on a milestone of its own: the install layout on milestone 515 (rung 2a), Secure Boot on
milestone 500 (a stick that boots with Secure Boot on) for rungs 1d and 4, and publication is
an architect's act at rung 4.

**The history of that line.** **Ruled by calef on 2026-09-19 (16:32 UTC):
`MILESTONE 23` is dropped from this line.** It read `DECISION, MILESTONE 23`, inherited through
milestone 39, until then. That made a loop: this block waited on milestone 23, whose one residual
(state handoff, §116) is declined until a customer exists, and no second customer can be accepted
until this block exists. The scoping lane checked what a first slice would need from milestone 39's
repository split and found only an SDK gap (an outside author needs this repository cloned to get
the toolchain), which is a candidate milestone of its own rather than a reason to wait on the split.
The argument is in
[DECISIONS §156](../decisions/156-the-package-manager-waits-on-a-decision-not-milestone-23.md). Milestone 39
keeps its own gate; this ruling does not touch it.

A second half of that fork was already ruled and this gate did not say so, found by milestone
435's first slice on the same day.
[§151](../decisions/151-repository-goal-is-independent-release.md) (the goal of the repository
split is independent release and third-party programs) took the goal on 2026-09-15, and names this
milestone's own sentence in doing it. What §151 deliberately leaves open is the order, when the
split happens and against what preconditions, and it lists a package format existing so `basalt` has
something to assemble as one of them, which is this milestone. The format, the activation shape and
the repository split are still not this block's and are deliberately not raised here.

In brief, from calef on 2026-08-30: *"I don't think we expose nife to third parties (aka other
customers) until we have a package manager and a trivial install process."* And, in the same breath,
that he wants it early, to make our own lives easier.

Both halves matter and they point the same way.

## This is a precondition on principle 1, not an item under it

AGENTS.md ranks work by the shortest path to a system a customer runs. As of 2026-08-30 that path is
vacant, and the reason is now two reasons: the first customer's deadline passed and they went to
borg over SSH, and this system could not accept a second one if it appeared.

That makes packaging structurally different from the milestones it sits beside. It is not on the
customer path; it is the door. A roadmap that ranks by the shortest path to a customer while
being unable to take one is ranking against a door it has not built.

## The second half is the one that earns it early

The third-party argument alone would justify deferring this indefinitely, since there is no third
party and no date for one. **The reason to do it early is that the builders are the ones paying for
its absence today**, hand-wiring per program what a package would install once:

- Milestone 40 (documentation as a system service) already ships *"installed by the package that
  owns it"* and its per-package index shard, against no package that exists.
- Milestone 47's conclusion is that **installing a program is granting it into a namespace**, which
  is a packaging statement with no packager.
- `crates/system_initializer` spends a capability through a syscall per program, per architecture,
  and every new program edits it.
- Milestone 150 (adding a program should not need eight hand-maintained lists) is the same complaint
  from the other end, and the count is the argument: eight lists, hand-maintained, per program.

## What already exists to build on, so this does not start cold

- **Milestone 39** has the structural recommendation: monorepo now with the distribution as a
  separate manifest repo, executed as multiple workspaces.
- `design/haiku-bfs-and-packages.md` is the prior art the roadmap already tells you to read
  first. Haiku's `packagefs` activates packages rather than installing them, composing a
  filesystem view from read-only package files rather than letting installers mutate shared
  directories. It arrived near milestone 47's conclusion from an entirely different motive, atomic
  and rollback-able installs, which is the useful kind of convergence.
- `design/what-a-distribution-packages.md` is the speculation about the units, and is explicitly
  labelled as speculation.
- DECISIONS §135 (running GPL software is aggregation) makes packages the channel for copyleft,
  so this milestone is also what unblocks `git` and `nano` arriving the honest way rather than being
  built into an image.

## What "trivial install" has to mean, and it is the harder half

A package manager without an install story is a mechanism nobody reaches. This block deliberately
does not specify one, because nothing here has met a stranger yet, but it names the constraint:
a person with hardware and no prior knowledge of this project reaches a running system. That is
principle 3's test applied to running rather than to building, and today the answer is a
`cargo xtask` invocation on a development machine, which is not an install.

Defined by calef on 2026-09-19
([DECISIONS §157](../decisions/157-a-trivial-install-is-a-web-page-a-usb-drive-and-packages.md)): a
web page from which one downloads a minimal system, writes it to a USB drive and installs it, and
which then grows by installing packages over the internet. That puts the format, activation and
trust forks on the path, and it names work no milestone owns yet (an installer, a real network card
driver, TLS).

## Rescoped 2026-09-19 under §157

A second lane (`milestone/198-rungs-to-a-trivial-install`) built nothing and mapped §157's
definition onto rungs, each shipping something on its own, against the tree as it stood that day.
This section replaces the scoping lane's proposed first slice (a QEMU run bundle, recorded below
as history), which §157 superseded as a definition.

Every rung is proved on xenon first and on a second PC after. xenon is the only PC on the bench;
milestone 243's fleet (the family's other x86 machines) is what turns a claim about one Dell into a
claim about PCs, and each rung's last exit criterion is that second machine.

### The rungs

| Rung | Ships | Exit criterion a stranger could check | Milestones and proposals on it |
|---|---|---|---|
| **1a. A prompt from a stick, on xenon, over serial** | Nothing new: the pieces are built | xenon boots `\EFI\BOOT\BOOTX64.EFI` from a FAT32 stick, `$` appears on the serial console, and `echo hello` answers; transcript filed under `bench/` | 87 (BUILT), 299 (BUILT: the x86 prompt over serial, proven under QEMU), 182 (PARTIAL: its third `swish-check` leg), 243 |
| **1b. The prompt on the screen** | A pre-set-framebuffer driver behind the framebuffer contract | Under OVMF, `board_console::screen` reads `$` back off the framebuffer; on xenon, the prompt is on the monitor and a serial keystroke echoes there | [400](400-the-shell-on-the-firmware-screen.md) (PARTIAL: built and gated under OVMF, xenon outstanding; found by this lane); shares its driver with 157 |
| **1c. A USB keyboard** | xHCI, enumeration, HID boot protocol | xenon with a monitor and a USB keyboard, no serial cable: `echo hello` | 242 (NOT-STARTED), which closes 192 (PARTIAL) |
| **1d. A PC that is not xenon** | Nothing new if 1a to 1c hold | The same stick on one fleet machine reaches `$` at its own keyboard and monitor | 243's fleet; [a-stick-that-boots-with-secure-boot-on.md](500-a-stick-that-boots-with-secure-boot-on.md) (new) for machines whose owner will not turn Secure Boot off |
| **2a. Installed onto a disk, under QEMU** | An installer; the boot mounting the nife partition off NVMe | OVMF boots the stick image with an empty NVMe attached; the installer names the disk, asks, partitions, formats and copies; the machine reboots **with the stick detached**, reaches `$`, and reads back a file written before the reboot. One `cargo xtask` gate | [the-installer-a-stick-runs-to-put-itself-on-the-disk.md](515-the-installer-a-stick-runs-to-put-itself-on-the-disk.md) (new); [milestone 421 (the block roster cannot name an NVMe)](421-a-block-roster-that-can-name-an-nvme-disk.md) (existing); 57's partitioner and `mkfs` (BUILT) |
| **2b. Installed onto xenon's disk** | The bench half | The 2a sequence on xenon's Micron 2450, photographed, stick removed before the second boot | 261 (PARTIAL: the disk wipe, calef's, then one bench boot) |
| **3a. A package over the LAN, under QEMU** | The package client this milestone is; a host-side recipe that produces a package; a small HTTP client | A package absent from the image is fetched from a host on the same network over plain HTTP, verified by digest, installed onto the running system, run, still present after a reboot, and removed | this block; all three rulings it needed are in (§195, §197, §208). The scoping lane's recipe idea (item 1 of the superseded slice) survives here as the producer half. **Producer half BUILT 2026-09-23** (`crates/package_archive`, `cargo xtask package`, `packages/uptime.recipe.toml`, notes/packages.md). Fetch and verify built 2026-09-24 (`helpers/package-http-peer`, `crates/http_response`, a QEMU test on aarch64 and riscv64); **run by digest and the installer built 2026-09-26** on all three architectures. No TLS, since §195's digest decides whether bytes may run |
| **3b. The network card xenon has** | An Intel I219 (`e1000e` family) driver in 261's shape | Under QEMU `-device e1000e` behind `intel-iommu`, milestone 30 (the network stack as a confined component)'s DHCP and TCP gates pass through the new driver; on xenon, a lease from the house router and a measured transfer | [a-driver-for-the-network-card-a-pc-actually-has.md](494-a-driver-for-the-network-card-a-pc-actually-has.md) (new) |
| **3c. Over the internet** | Name resolution; the transport the ruling picks; a public repository | From xenon's installed system, a package fetched from the public repository by host name, verified and installed | [milestone 384](384-a-name-resolver-and-who-holds-it.md) (existing, which now has a consumer); [DECISIONS §196](../decisions/196-nife-carries-tls-and-builds-the-provider.md) (new); `a-tls-stack-and-which-one.md` (existing) if the ruling is HTTPS |
| **4. The web page** | A published release and a page | A stranger with a PC, a USB stick and no prior knowledge follows the page to rung 3c's result; the stranger harness (`notes/stranger-test.md`) runs against the **download**, not the build | calef's act; the preconditions below |

### Order, and which rungs are too big

Recommended order (reversible): 1a, then 2a and 3a in parallel lanes (they touch disjoint
subsystems: the installer and boot mount, and the package client), then 1b, 2b, 3b, 3c, with 1c
running beside all of them, then 1d, then 4.

- Rung 1 is too large for "a milestone or two", and one piece is why: 1c is milestone 242, which
  milestone 192 prices at *"months rather than weeks"* and 242 itself declines to price. It is split
  on purpose so that 2 and 3 do not wait on it: on xenon, the installer and the package client
  can be driven over the serial console. 1c is on rung 4's critical path and on nothing else's.
- 1a is one attended boot, and it should capture two more facts while it is there: the PCI
  survey now prints every function (`kernel/src/pci.rs`, since 2026-09-18), which names xenon's
  network card for 3b, and the `vt-d` line's DMAR scope, which milestone 261 names as its
  load-bearing unknown.
- Rung 2 is one milestone plus 261's bench step. The partitioner and `mkfs` exist; what is new
  is a layout sized to the disk, the loader handing its own file over, the ESP, and the boot mount.
- Rung 3 is three pieces and 3a is the only one that is this milestone. 3b and 3c are the NIC
  proposal and the resolver plus the transport ruling. 3a can start as soon as the three fork
  rulings are in, over virtio-net under QEMU, needing no new driver and no TLS.
- Trust T1 cannot serve rung 3: under T1 installing is rebuilding the image on a host, which a
  stranger cannot do. Rung 3 needs T2 or T3.

### Which rulings each rung needs

One line each, in the form calef would answer, with the rung that waits on it.

| Ruling | The question | Rung it blocks | Where the options are |
|---|---|---|---|
| ~~**Format**~~ | **Decided 2026-09-20. DECISIONS §197 (a package is one archive file) rules one archive file per package**, identified by name and version, with the reviewed recipe of §195 (a reviewed recipe vouches for a package) carrying its digest. The manifest travels in an ELF note (ruled 2026-09-26); whether the digest is a Merkle root is still calef's. | 3a | [DECISIONS §197](../decisions/197-a-package-is-one-archive-file.md) |
| ~~**Activation**~~ | **Decided 2026-09-23 (DECISIONS §208): A3 with rollback.** Installing records that a package exists (digest and manifest spawnable, data a read-only directory a session binds by name), nothing is written into shared space, and the table of entries is versioned so a set rolls back whole. | 3a's **consumer** half | [DECISIONS §208](../decisions/208-installing-is-granting.md), from [milestone 507](507-installing-a-package-mutates-or-composes.md) |
| ~~**Trust**~~ | **Decided 2026-09-19 (DECISIONS §195): a reviewed recipe vouches, trust is scoped per source the owner opted into, and the owner may overrule.** No long-lived signing key is held for now; a per-source signature can be added later without changing that. | 3a | [DECISIONS §195](../decisions/195-a-recipe-vouches-and-the-owner-may-overrule.md) |
| **Install layout** (new) | Is an installed disk an EFI system partition plus a data partition, the same with two boot slots, or a small loader plus a raw system partition? | 2a's merge (a lane can build under a provisional layout; nothing leaves the machine until 4) | the installer proposal |
| ~~**Transport** (new)~~ | **Decided 2026-09-19 (DECISIONS §196): HTTPS, `rustls` for the protocol, and the crypto provider is milestone 442's work.** Under §195 a recipe's digest decides what may run, so rung 3a does not wait for TLS. | 3c (not 3a) | [DECISIONS §196](../decisions/196-nife-carries-tls-and-builds-the-provider.md) |
| ~~**Hosting** (new)~~ | **Settled with the transport (§196):** GitHub redirects plain HTTP, and carrying TLS is what makes a GitHub-hosted source reachable. Whether `crickertech` operates a source at all, and the GPL obligation that comes with it, is still open. | 3c and 4 | [DECISIONS §196](../decisions/196-nife-carries-tls-and-builds-the-provider.md) |
| **Secure Boot** (new) | Does a stranger turn Secure Boot off, or do we sign, and if we sign, is it the same key as the package key? | 4 (and 1d on a machine whose owner will not turn it off) | the Secure Boot proposal |
| **Publication** | Is it time to put the page up? | 4 | §157, step 1: calef's act |

Not a ruling but calef's hands: **the wipe of xenon's NVMe** (milestone 261), which blocks 2b.

### What must be true before the page goes up

§157 makes publishing calef's act. What a lane can make true first, so the act is only a decision:

- Rungs 1 to 3 pass on a machine that is not xenon, with a USB keyboard, so milestone 242 is
  built.
- A release exists. `gh release list` returns nothing today (§157's measurement); the download
  carries a checksum, and a signature if the trust or Secure Boot ruling creates a key.
- The Secure Boot ruling is answered on the page, before its first instruction.
- Something checks DECISIONS §135's requirement 1 ("no conveyed artifact carries copyleft") for
  the image the page conveys; §135's own `BUGS` says nothing enforces it.
- The stranger harness passes against the download, and records the time from page to prompt.

### The superseded slice, and what survived it

The scoping lane's first slice (below, under "Scoped 2026-09-19") had four items. Item 1, packages
as host-side recipes, survives as rung 3a's producer. Item 2, image composition from a declared
set, is off the path: milestone 150, in flight as PR #968, generates the program table from one
declaration, and
nothing on a rung needs a second mechanism. Item 3, the QEMU run bundle, is off the path; §157
left it as a lane's reversible call, and the call is that it is a developer convenience rather than
a rung, worth building only if the stranger harness wants a vehicle before rung 1d exists. **Item
4**, removal, is part of 3a's exit criterion and of the activation ruling.

## Scoped 2026-09-19

*History since 2026-09-19: the trivial-install row and the first slice below are superseded by
§157; the three fork proposals stand and are placed on rungs above.*

A scoping lane (`milestone/198-package-manager-scoping`) built nothing and wrote the forks as
proposals, one decision each, for calef to rule on separately. The status does not move: nothing is
built, and the gate line above is left as minted because changing it is the first proposal's ask.

| Fork | Proposal | Shape |
|---|---|---|
| The gate | [DECISIONS §156](../decisions/156-the-package-manager-waits-on-a-decision-not-milestone-23.md) | **Decided 2026-09-19 by calef, as recommended:** `Gate: DECISION` alone. The loop is real (198 waits on 23, whose residual waits on a customer, who waits on 198), and nothing in a first slice needs the split's timing. One real dependency was found, on a downloadable toolchain rather than on the split |
| Package format | [DECISIONS §197](../decisions/197-a-package-is-one-archive-file.md) | **Decided 2026-09-20 by calef**, after comparing apt, Homebrew, Alpine, Haiku and Nix: one archive file per package, the mainstream container, vouched for by §195's reviewed recipe |
| Activation | [installing-a-package-mutates-or-composes.md](507-installing-a-package-mutates-or-composes.md) | **Decided 2026-09-23 by calef (§208): A3, with rollback.** As proposed, options with no winner: mutate, compose a union view, or only widen what may be spawned. The program namespace is sealed at boot, and the spawner gives the file service away, so nothing that builds processes can read an installed program today |
| Trust (found, not briefed) | [DECISIONS §195](../decisions/195-a-recipe-vouches-and-the-owner-may-overrule.md) | **Decided 2026-09-19 by calef**, after reading how apt, pkg, pacman, Nix, Fuchsia and Homebrew do it: Homebrew's shape (digests in reviewed recipes, per source) with the owner-vouches escape hatch every one of them keeps. The image's measured table becomes the first source |
| Trivial install | [DECISIONS §157](../decisions/157-a-trivial-install-is-a-web-page-a-usb-drive-and-packages.md) | **Decided 2026-09-19 by calef, not as recommended:** a web page, a download written to a USB drive and installed, then packages over the internet. The lane had recommended a QEMU run bundle as the first rung; its first slice is superseded as a definition and needs rescoping |

**Superseded 2026-09-19 by §157 and by "Rescoped 2026-09-19" above**, kept as the record of what
was proposed. ~~The proposed first slice needs none of the three irreversible rulings: packages
as host-side recipes, image composition from a declared set, and a run bundle tested by the stranger
harness and not published until calef says so. Its details and what it unblocks are in the
trivial-install proposal.~~

## Follow-on

Added by the lanes that built rung 3a (2026-09-23, 2026-09-24 and 2026-09-26), covering rung 3a only: this
block's other rungs carry their own exit criteria in the table above, and the rungs that are
calef's acts are named there rather than here.

- **Done.** §221 closed the `activation/` write hole (notes/packages/vouching.md).
- **Done.** §219 was ruled 2026-09-26 and its request, gate D2 and the unvouched-child
  confinement claim built (notes/packages/running-unvouched.md).
- **Proposed.** The virtio device table never reuses a slot
  (`design/roadmap/proposals/a-virtio-slot-comes-back-when-its-driver-dies.md`). The booted system
  had no network either; milestone 590 (the booted system starts its network stack) built it.
- **Done.** Every archive build runs `cargo xtask package` end to end since 2026-09-24.
- **Decision.** Whether the digest is a Merkle root is still calef's:
  `design/decisions/197-a-package-is-one-archive-file.md`, which also records the manifest ruling
  (an ELF note, 2026-09-26) and links the proposal for its build.
- **Recorded.** No compression, a `u32` ceiling on a member and on a package, a catalogue that is
  one file in `target/` rather than a repository index, a package source compiled into the
  progenitor, and a recipe that cannot say where its source came from (`crates/package_archive`'s and `xtask/src/package.rs`'s BUGS sections, and
  `notes/packages.md`).
- **Recorded.** `packages/uptime.recipe.toml` records no digest on purpose, because the program it names
  is rebuilt by this checkout whenever anything it links changes (the recipe's own comment).

## BUGS

- This block prices nothing. A package manager is a large piece of work and the estimate is not
  attempted; the sequencing claim is that it gates a customer, not that it is cheap.
- The package encoding is provisional, and so is `package_archive`'s name. §197 ruled the
  container, not the byte offsets. Its manifest question was ruled 2026-09-26 (an ELF note, not a
  member), and whether the digest is a Merkle root is left open (the built format takes the plain
  SHA-256 §197 records as the default). Reversible only until somebody outside this repository
  fetches a package, which §197 says fixes the format.
- The two Kani harnesses prove less than their names suggest. They cover a 272-byte file, which
  is the largest symbolic buffer that stays cheap, so the table they exercise holds at most two
  members and `MAX_MEMBERS` is never approached. Both discharge in 4 seconds together, and
  `script/verify`'s table carries the row.
- It does not decide the format, the activation shape, or the repository split. The scoping
  lane found the split's timing is not needed at all (see the gate proposal). ~~The format,
  activation and trust forks are proposals awaiting calef, not decisions.~~ calef decided all three
  (§197, §208, §195); the repository split is still open under §151 (the goal of the
  repository split is independent release).
- ~~"Trivial install" is undefined on purpose and that is a real gap, not a subtlety. Nobody has
  written what a stranger's first ten minutes look like.~~ Written 2026-09-19 in the
  trivial-install proposal, from the tree and from commands run that day. ~~What remains undefined
  is calef's ruling on it.~~ Ruled 2026-09-19 (§157), and mapped onto rungs above.
- ~~**No real board gives a stranger a prompt today.** x86-64 has no interactive boot (milestone 182)
  and no USB keyboard (milestone 242); radon's prompt input is unconfirmed on silicon. So the only
  interactive install this milestone can offer soon is QEMU, and that limit is outside this
  milestone's reach.~~ Reframed 2026-09-19 by §157: the install is a PC, and the limit is now
  rung 1's. x86_64 has a prompt over serial since milestone 299, proven under QEMU and not yet on
  xenon; the shell's output does not reach the screen (a new proposal), and the keyboard is still
  milestone 242.
- Rung 1 cannot be finished in "a milestone or two", and the reason is milestone 242 (USB host
  and HID). The rungs are split so rungs 2 and 3 do not wait on it; rung 4 does.
- The rungs are sequenced on one Dell. Each has a second-machine exit criterion, and until one
  passes, every claim here is about xenon. A stranger's laptop may have no Ethernet (Wi-Fi is out of
  scope in the NIC proposal), a SATA disk (no AHCI driver; the installer proposal's `BUGS`), or its
  NVMe behind Intel RST or VMD.
- A third party cannot author a package without cloning this repository, because the `nife-dev`
  toolchain, the target specifications and the linker script exist only as build steps inside it
  (`helpers/build-ripgrep.sh` is the one out-of-tree build and it needs them). §151 (the goal of the repository split is independent release)'s
  "third-party programs" needs a downloadable toolchain, which nothing tracks yet.
- The cold build time a stranger pays was not measured, because two other lanes were gating on
  the machine when this block was scoped. ~~The first slice's stranger-harness run should measure it
  alongside the bundle.~~ Under §157 a stranger does not build at all; it stays worth measuring for
  contributors, and rung 4's stranger-harness run measures the time from the page to a prompt
  instead.
- Packages do not by themselves run `git` or `nano`. Milestone 205 (no argument vector) and the
  raw-input primitive of milestones 169 and 170 still stand in front of both.
- nife cannot build software, so a package is a thing produced by a host toolchain and consumed
  by the target. Every packaging idea borrowed from a self-hosting system needs that translation
  checked rather than assumed.

## Index row

calef, 2026-08-30: no third party sees nife until there is a package manager and a trivial
install, and he wants both early, to make our own lives easier. That makes packaging a precondition on principle 1's ranking function rather than an item under it: the customer path
is vacant partly because a second customer could not be accepted if one appeared. The early half
is what earns it, since the builders pay for its absence today, hand-wiring per program what a
package would install once (milestone 40 already ships "installed by the package that owns it",
against no package). Gate: NONE since §219 was ruled on 2026-09-26; its request and the installer are built (2026-09-26); the
format, activation and trust forks are ruled too (§197, §208, §195); `MILESTONE 23` was dropped by calef on 2026-09-19 because it closed a
loop, which §156 (what the package manager waits on) records. Rescoped 2026-09-19 under §157
into four rungs, each shipping on its own, and rung 3a's producer half is built (2026-09-23:
the one archive file §197 ruled a package is, and `cargo xtask package`, which turns a reviewed
recipe into one; fetch and verify followed on 2026-09-24): a stick reaching a prompt on a PC (1a on xenon over serial
needs nothing new; the screen and the USB keyboard, milestone 242, are the rest), that system
installed onto the disk (a new installer proposal plus milestone 261's bench step), growing by
packages over the network (this milestone on a LAN first, then a real network card and the
internet), and the web page, which is an architect's act.
