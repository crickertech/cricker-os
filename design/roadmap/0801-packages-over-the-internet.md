---
status: PARTIAL
raised: 2026-09-19
milestone_dependencies: 384, 494, 501, 809, 855
decision_dependencies: unwritten
machine_requirements: x86_64 PC with a wired network card on the internet
specific_machine: xenon (rung 3c's exit criterion is xenon's installed system)
needs_person: yes
---
# 801. Packages over the internet: a host name, HTTPS and the distribution's index

Split out of milestone 198 (a package manager) on 2026-10-06 (UTC), where it was rung 3c. calef,
the same day: *"Don't fix the tooling to name a rung. We should make our milestones finer grained if
we're going to express dependencies on a fraction of them."* Milestone 576 (how many systems are out
there, and what do they run) depends on this rung and on nothing else of 198, and a dependency
field names whole milestones. The number is provisional until the merge queue lands it, and the
title and slug are drafts. `raised` is the day §157 put the rung on the path.

## What it is

From xenon's installed system, a package is fetched through `basalt.nifeos.org`'s index by host
name, verified by its digest and installed. That is rung 3c of §157 (a trivial install is a web
page, a USB drive, and packages over the internet), which [milestone 198's rung
table](0198-package-manager.md#the-rungs) still maps.

Rung 3a, the package client over plain HTTP on a LAN, is built and is milestone 198. Rung 3b, the
network card xenon has, is milestone 494 (a driver for the network card a PC actually has). This
milestone is what is left: the name, the transport, and §250's index.

## The work list

Today's `fetch` (`crates/system_initializer`) has the image's catalog and one compiled-in source.

1. Split the index from the package locations: fetch the index, then each package from where it
   says, verified by its digest.
2. The index's format, a crate by rule 7, with its "moved to" field. Ruled before rung 4, which is
   milestone 802 (the trivial install).
3. Resolve the index's host name: milestone 384 (in a capability system the resolver is a grant).
4. Speak HTTPS to it: milestone 501 (a TLS client that speaks to one pinned peer), under §196 (nife
   carries TLS). Under §195 (a reviewed recipe vouches for a package) a recipe's digest decides
   what may run, so TLS protects the index, not the package's bytes.

## Built 2026-10-09 (UTC): items 1, 3 and 4 under QEMU, and item 2's survey

Lane `milestone/801-packages-over-the-internet` (PR #1884). The account, the transcripts and the
seams are `notes/packages/over-the-internet.md`.

- Item 1. `crates/package_index` (provisional) reads an index, finds the one entry a name asks for,
  and admits fetched bytes through `package_archive::installable_as` with the entry as a one-line
  catalog, the installer's own check. The encoding is a stand-in until item 2 is ruled.
- Item 3. A std program's `ToSocketAddrs` asks the resolver badge at
  `std_runtime_protocol::RESOLVER_SLOT` (9, provisional), keeping each refusal's reason.
  `a_std_program_resolves_its_granted_zone_and_nothing_without_a_grant` gates it on all three
  architectures in CI, falsified once.
- Item 4. `package_fetch_exerciser` (provisional) resolves `basalt.test`, reads the index over TLS
  pinned to the test root, fetches `greeting` from another host over plain HTTP and admits it by
  the index's digest, and refuses a location serving a flipped byte. Green on all three
  architectures locally; it skips in CI until milestone 855 (the TLS graph enters the gated build).
- Item 2. Stopped at a proposal. §250's amendment and milestone 858 (lab machines update
  themselves through packages)'s Fork 9 already made the index
  a TUF repository; what TUF leaves open is in `notes/packages/the-index-format.md` and on #1884.

The status is PARTIAL rather than BUILT: item 2 waits on calef, the exit criterion is xenon's
installed system, and installing from an index is `jig`'s (milestone 809 (the package client
becomes a program)).

## What is calef's

Asked on #1884 with options and a recommendation each, under `needs-architect`:

1. Where a package's bytes are named in basalt's TUF metadata (recommended: an ordered
   `custom.locations` per target, with TUF's own `targets/` as the floor).
2. What "moved to" means (recommended: carried by a signed root; a client rewrites its source only
   after the new place serves a root chaining to the trusted one).
3. An image carrying basalt's TUF root contradicts §220 (signed builds: a vendor signs, a developer
   self-signs, and trusting a key is scoped)'s "No key ships in any image" (recommended: carry the
   root's digest, and say plainly that this amends §220 for a distribution's root).
4. The index's path on the host, already pending (recommended: a channel prefix, as cordoba's
   `/lab/`).
5. The resolver grant `jig` holds (recommended: the root zone, since bytes may live anywhere).

Not rulings but calef's hands: DNS and a host for `basalt.nifeos.org`, and the exit criterion on
xenon.

## What it waits on

- Milestones 384, 494 and 501, in the frontmatter.
- §250 (an image names its distribution's package index, and the bytes may live anywhere) is
  ruled, except the index's path and file name on that host, which calef is deciding. That is the
  `unwritten` decision dependency.
- Not a ruling but calef's hands: DNS and a host for `basalt.nifeos.org`. That is `needs_person`.

Reuse: the package client, digest check and installer of milestone 198 are this milestone's
base. Nothing outside the tree was surveyed for the index format yet, and the lane that builds it
owes that survey under §46 (thin primitives or whole subsystems).

## Architectural parity

The capability here is the resolver, the TLS client and fetching the distribution's index over
HTTPS. It ships on aarch64, riscv64 and x86_64, proven under QEMU by the same suite on each, as rung
3a's fetch already is: aarch64 and riscv64 since 2026-09-24, x86_64 since 2026-10-05 (milestone
198's rung table and its "x86_64 fetches" entry). xenon is the silicon reference for the exit
criterion, not the scope. radon follows on silicon once the network half of milestone 53 (the
board's own peripherals: network and storage on real silicon), a driver for the JH7110's GMAC,
exists. aarch64 silicon waits on an aarch64 board.

## BUGS

- Nothing installs from an index yet. The progenitor installs only what the image's catalog
  vouches for; an index copy it trusts is `jig` writing one, milestone 809's ruling I2.
- The index client cannot run at the prompt: the booted system starts no resolver and gives a std
  program no network. Proposed: `design/roadmap/proposals/a-std-program-at-the-prompt-holds-the-network-and-a-resolver.md`.
- The whole-fetch gate skips in CI, as milestone 501's does, until milestone 855.
- `notes/packages/over-the-internet.md`'s BUGS carry the stand-in's limits (no "moved to", one plain
  HTTP location, no producer in the tree).

- Proved on xenon alone until milestone 802's second-machine criterion runs.
- A laptop with no Ethernet port cannot reach this rung. That is milestone 788 (Wi-Fi on a PC that
  has no Ethernet).

## Follow-on

- **Proposed.** `design/roadmap/proposals/a-std-program-at-the-prompt-holds-the-network-and-a-resolver.md`:
  the booted system starts the resolver from the lease, and a std program at the prompt is granted
  the network and a resolver zone. `jig` needs it to reach the index by name.
- **Milestone 809.** `jig` adopts `package_index` and `package_fetch_exerciser`'s fetch, writes the
  index copy, and installs from it. That is this rung's exit criterion short of xenon.
- **Milestone 855.** Whether a gate builds the TLS graph, which is whether item 4's test runs in CI.
- **Milestone 858.** The TUF client and verifier, items 4 and 5 of its work list, replace
  `Index::parse` once item 2's questions are ruled.
- **Done.** The std farm's stamp now covers `byte_sink_protocol`, found while adding the resolver's
  wire to the PAL.
- **Done.** A fresh `net_stack` starts its ephemeral ports at a counter-derived point, not 49152.
  A resolver's first `CONNECT` failed three runs in four after another stack's test had used the
  same 4-tuple (CI run 38009701250). Milestone 783 (the network stack seeds its random generator
  from the clock, and TCP sequence numbers come from it) is where that seed should come from
  entropy.
- **Done.** Every std program the test harness starts hands back its frames when it ends: the clock
  service, the configuration page and the stack now come from one region a holding reclaims
  (calef, 2026-10-10 (UTC): "Lets fix the leaks."), proved by
  `a_std_program_gives_back_every_frame_it_was_given`. About 56 frames a spawn had been kept for
  the boot.

## Index row

Rung 3c of the trivial install, split out of milestone 198 so that milestone 576 (how many systems
are out there) can depend on it alone. A package is fetched from xenon's installed system through
`basalt.nifeos.org`'s index by host name, over HTTPS, verified by digest and installed. It waits on
the resolver (384), the TLS client (501), xenon's network card (494) and the index's path, which is
calef's. Built under QEMU 2026-10-09: the index split from where the bytes live, a std program
resolving through its grant, and the whole fetch over pinned TLS. The index's format is a proposal.
