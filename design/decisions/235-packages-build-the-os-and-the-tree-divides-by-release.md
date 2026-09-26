---
status: PROPOSED
raised: 2026-09-26
---

# 235. The OS is built and updated from packages, and the tree divides by what releases together

*Section number provisional. §234 (the prompt shows one tree) was the highest on `main` when this
branch was rebased on 2026-09-26, so this took 235 and may move at merge. The file name is a lane's
coinage and provisional too.*

Raised 2026-09-26 (UTC) by calef, who asked for milestone 607 (provisional number): nothing ties
together how packages build and update the OS and how the monorepo splits into divisions. Written by
the design lane `milestone/607-packages-and-divisions`, which builds nothing. The measurements and
the prior art are in [notes/packages-and-divisions.md](../../notes/packages-and-divisions.md); this
section holds the forks and does not restate the numbers beyond what a fork needs.

## What is already decided, and holds under every option here

- §151 (the goal of the repository split is independent release): the split's goal is independent release and third-party programs. The order is open.
- §197 (a package is one archive file), named by name and version.
- §195 (a reviewed recipe vouches for a package): the recipe carries the digest.
- §208 (installing a package is granting it): the set of what is installed is versioned.
- §220 (signed builds): a vendor signs, a developer self-signs, a signature is checked at install and the digest is
  then pinned. No key ships in any image.
- Milestone 525 (a bad upgrade cannot brick the machine) and 554: two boot slots with priority, tries and a success mark. Each slot holds one
  complete image.
- Milestone 104 (the measurement continues past init): the kernel's compiled-in trust root pins the progenitor and the table of program
  digests. Milestone 450 (a signature over the init image) refused a signature in its place.
- Pull request #1338 (in flight, provisionally numbered 597, not yet on `main`): a program's
  manifest travels in an ELF note, version 1.
- §201 (one roadmap until a citation has to cross): one roadmap until a citation has to cross a repository.

## The premise, checked

The question assumes packages could build the OS and the tree could split along release lines.
Three facts from the tree qualify that. Each is measured in the note.

1. The kernel binary is a function of every base program. `kernel/build.rs` compiles the digests of
   the progenitor and of `PROGRAM_MEASUREMENTS` into `TRUST_ROOT`. Any base service change relinks
   the kernel. §151 calls the kernel an independently released component; the build makes it the
   last link of the image.
2. The kernel crate is also the integrator and the test harness. `kernel/src/user/` is 28,441 of the
   kernel's 89,885 lines. The kernel depends on nine service crates and three fixture crates, and
   half of the commits in that directory touch another crate.
3. Contracts still move with their consumers. Of 215 commits touching a contract crate in 60 days,
   76% touched another division too. §151 names protocols that stop churning as a precondition of
   the split. It is not met.

None of these makes the question wrong. They say where the work is, and fact 1 decides which forks
are live.

## Fork 1: what an OS update ships

What a slot receives, and whether a base service can change without a slot swap. The on-disk layout
and the update manifest are formats two programs agree on (the updater and the booting system), so
this fork gets options only.

| option | shape | what it costs |
|---|---|---|
| U1. The image stays the unit | Packages are a build input. An image recipe names base packages by digest, the build assembles them, and a slot receives the whole image. User packages stay under §208. | Every base fix is a full image, about 10 MB today. No new on-disk format. The trust root is untouched. Base services cannot be fixed without a reboot into the other slot. |
| U2. The image is a list of digests | A slot holds the loader, kernel, progenitor and a system manifest. Base packages live in one content-addressed store both slots read. An update is a new manifest plus the blobs the store lacks. This is Fuchsia's and OSTree's shape. | A new store format, read by the progenitor before any file service runs, which grows the boot path. A slot stops being self-contained, so rollback depends on the store keeping the old slot's blobs. A collection rule is new, and a mistake in it bricks both slots at once. |
| U3. Two tiers | U1 for the part that boots to the package manager (kernel, progenitor, drivers, the network, the installer). §208 activation for every other base service, updated without a reboot. | The line between tiers is itself a decision, and it moves whenever a service is added. Two rollback mechanisms: 525's tries for the lower tier, §208 generations for the upper. A bad upper-tier service is not caught by a trial boot. |

Every option keeps 525's slots. U1 and U3 need no new boot format; U2 needs one. U2 is the only one
where bandwidth falls with the size of a change, and at ten megabytes an image that saving is small.

## Fork 2: where the trust root lives

This decides whether the kernel can ever release on its own. The kernel and the progenitor agree on
it, so it is irreversible once an installed machine boots it.

| option | shape | what it costs |
|---|---|---|
| T1. Compiled into the kernel (today) | `TRUST_ROOT` pins the progenitor and the table | The kernel and the base image release as one unit. §151's independent kernel is not reachable. Nothing new in the trusted computing base. |
| T2. Handed over by the loader | The slot carries one manifest digest, the loader passes it to the kernel, and the kernel checks the progenitor and table against it | The kernel binary stops depending on userspace. Trust moves to the loader, which only Secure Boot (milestone 500 (a stick that boots with Secure Boot on)) or the slot checksum protects. Three handoff paths to change: a device tree `/chosen` on aarch64 and riscv64, PVH on x86_64. Milestone 525 records that the device-tree path has one initrd slot. |
| T3. A signature over the system manifest | The kernel verifies a vendor key's signature over the manifest | Reopens milestone 450's refusal. Keys and verification code enter the kernel. §220 put signatures at install, never at boot, and this would move one to boot. |

T1 fits U1 and U3 without change. U2 works under T1 too: the kernel pins the manifest's digest, and
the manifest pins the blobs. So Fork 2 is about the kernel's independence, not about packages.

## Fork 3: the divisions, and whether they become repositories

The names of divisions and repositories are calef's; those below are provisional. Seven candidates
come out of the dependency graph: kernel, contracts, runtime, boot, services, fixtures and host.
Proofs are not a division, because each proof lives beside its code in 28 places.

| option | shape | what it costs |
|---|---|---|
| R1. One repository, divisions released from it | Each division becomes a workspace (milestone 39 (repository structure for a loosely-coupled OS)'s B) with its own version and release tags. The SDK is a release artifact. `script/test` stays one command. | Nothing moves between repositories. A third party never needs this one, only its releases. Versioning discipline arrives without the split's integration cost. §151's goal is met without a split, which may not be what calef meant by it. |
| R2. Split out the SDK alone | Contracts and runtime move to their own repository. Everything else stays. | 76% of contract commits touch another division today, so most contract changes become two pull requests in two repositories. It gives the third-party boundary a hard wall. |
| R3. Split along all seven | Milestone 39's option C, with `basalt` running the whole-system gate. | 27% of code commits cross a division. `kernel/src/user/` must leave the kernel first. The whole-system proof on three architectures moves to a repository that must run on every change to any other, or it stops running. |

Two things stay single-tree under every option. One `script/test` proves every architecture (§19 (architectural parity is a tenet)),
and the integration test suite needs every division at a known version to run. R3 keeps them by
building that gate in `basalt`; R1 and R2 keep them by not moving them.

## Fork 4: how a third party builds against the ABI

This is milestone 198 (a package manager)'s SDK gap: an outside author must clone this repository to get the toolchain.
The closure a `std` program needs is ten crates, about 24,400 lines, plus the 4,486-line `std`
overlay, the target files and the linker script. Publishing any of it is a fact that leaves the
machine, so options only.

| option | shape | what it costs |
|---|---|---|
| S1. Publish crates and a toolchain | The ten crates go to crates.io with real versions. A toolchain archive carries the patched `rust-src`, the targets and the linker script. | A semver promise to strangers. crates.io names are global and first come. Milestone 39 warns that the first real outside caller finds the shape wrong. |
| S2. One SDK archive per release | Fuchsia's IDK and Genode's `api` archive: the crates and toolchain pieces in one file, used through `[patch]` or a path. | Nothing published to a registry. A bespoke install step for every third party. |
| S3. An upstream Rust target | The `std` overlay moves upstream as a tier 3 target. | Upstream review, and a claim that the ABI is stable enough for someone else's tree. Months, not a milestone. |

Under any of them, a program should carry the ABI revision it was built against, so the progenitor
can refuse one it no longer supports. The ELF note of #1338 has a version field for its own
layout. Whether the ABI revision is a field in that note or a second note is a wire format, and it
is calef's.

## Recommendations on the reversible parts

Sequencing is reversible, and so is the first step, because it lives on the host and leaves the
machine in no form.

First, three pieces that are right under every option:

1. Build the image from packages (proposal P1 below). `cargo xtask` turns each base program into a
   §197 package, and the image is assembled from a declared set by digest. This is milestone 198's
   superseded "item 2", now wanted because it is what "the OS is built from packages" means at
   build time. It changes no format on the device.
2. Pull the integrator and the test harness out of the kernel crate (P2). This is the one boundary
   the graph says is wrong under every fork, and it is code, so it is cheap to undo.
3. Cut the two contract-to-runtime edges (P3). Two crates, reversible.

Then, gated on rulings: the SDK (P4) on Fork 4, the update path (P5) on Fork 1, the kernel's trust
handoff (P6) on Fork 2, and division workspaces (P7) on Fork 3.

The order of the rulings is reversible too, so a recommendation: rule on Fork 1 first. It decides
what P5 builds, whether Fork 2 matters before the kernel wants its own release, and what the SDK
must version. Fork 3 can wait longest, because its precondition (contracts that stop churning) is
not met.

The proposals, none minted:

| id | proposal | waits on |
|---|---|---|
| P1 | The image is assembled from base packages by digest | nothing |
| P2 | The kernel crate stops carrying the bring-up and the system tests | nothing; collides with the test-wiring hotspot, so one lane alone |
| P3 | A contract never links the runtime | nothing |
| P4 | A third party builds `greeting` with no clone of this repository | Fork 4 |
| P5 | An OS update is a set of packages that lands through a slot | Fork 1, P1 |
| P6 | The kernel stops pinning userspace | Fork 2, P2 |
| P7 | Each division is a workspace with a version | Fork 3, P2, P3 |

## The seven questions

1. What else was considered? Each fork's table is the answer. Also refused: proofs as a division,
   because it separates a proof from its subject. Refused: a shared blob store built ahead of a ruling,
   because U2's collection rule is the one mistake that bricks both slots.
2. What does the tree already do? The image is a closure pinned by `TRUST_ROOT` (milestone 104).
   User packages are archives, activated by grant (§197, §208). One out-of-tree build exists,
   `helpers/build-ripgrep.sh`.
3. Prior art: Fuchsia, Nix, FreeBSD pkgbase, ChromeOS, OSTree and bootc, and Genode's depot, read
   2026-09-26. What to take and refuse is in the note.
4. Is the premise true? Partly, and three facts above qualify it. The kernel cannot release
   independently under T1, whatever the repository shape.
5. What does each option cost? Measured where a command could measure it: the image size, the edge
   counts, the commit coupling, the SDK closure. The costs a command cannot give (a store's
   collection rule, a semver promise) are named as such.
6. How reversible, and who has acted on it? Nobody outside this machine has fetched a package,
   installed a slot, or built against the ABI. Every fork is still cheap to rule on; each becomes
   expensive at its own first outside consumer.
7. Would we choose the same if every option cost the same? The recommendations are about order, not
   effort. P1 goes first because every Fork 1 option needs it, not because it is small.

## What cannot be undone, and what can

The update manifest and store layout (Fork 1), the trust handoff (Fork 2), repository boundaries
and their names (Fork 3), and anything published to a stranger (Fork 4) are irreversible once a
machine or a person outside acts on them. Everything in P1 to P3 is host code and can be reverted.

## What is blocked until this is answered

Nothing on the customer path today. P1 to P3 can start now. Milestone 198's rung 4 (the web page)
needs Fork 4 answered before the SDK sentence on it can be written. Milestone 39 and §151's order
wait on Fork 3.

## BUGS

- The division assignment is a judgment per crate. The note says which crates are ambiguous.
- The image size is a stale local build. P1's lane should measure a fresh one.
- The word "manifest" now means two things: a program's ask (#1338's ELF note) and the list of
  packages an image holds (U2 and T2). The second needs its own name before it is built.
