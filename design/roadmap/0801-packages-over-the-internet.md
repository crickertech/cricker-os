---
status: NOT-STARTED
raised: 2026-09-19
milestone_dependencies: 384, 494, 501
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

- Proved on xenon alone until milestone 802's second-machine criterion runs.
- A laptop with no Ethernet port cannot reach this rung. That is milestone 788 (Wi-Fi on a PC that
  has no Ethernet).

## Index row

Rung 3c of the trivial install, split out of milestone 198 so that milestone 576 (how many systems
are out there) can depend on it alone. A package is fetched from xenon's installed system through
`basalt.nifeos.org`'s index by host name, over HTTPS, verified by digest and installed. It waits on
the resolver (384), the TLS client (501), xenon's network card (494) and the index's path, which is
calef's.
