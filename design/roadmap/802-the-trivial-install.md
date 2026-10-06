---
status: NOT-STARTED
raised: 2026-08-30
milestone_dependencies: 242, 400, 515, 801
decision_dependencies: unwritten
machine_requirements: two x86_64 UEFI PCs, each with a USB keyboard and a monitor
specific_machine: none
needs_person: yes
---
# 802. The trivial install that makes a second customer possible: a web page a stranger follows

Split out of milestone 198 (a package manager) on 2026-10-06 (UTC), where it was rungs 1d and 4 and
the "trivial install" half of 198's title. calef, the same day: *"Don't fix the tooling to name a
rung. We should make our milestones finer grained if we're going to express dependencies on a
fraction of them."* The number is provisional until the merge queue lands it, and the title and
slug are drafts. `raised` is 198's, because this is the half of calef's 2026-08-30 sentence that is
still open: *"I don't think we expose nife to third parties (aka other customers) until we have a
package manager and a trivial install process."*

## What "trivial install" has to mean

A package manager without an install story is a mechanism nobody reaches. The constraint: a person
with hardware and no prior knowledge of this project reaches a running system. That is principle
3's test applied to running rather than to building. Today the answer is a `cargo xtask` invocation
on a development machine, which is not an install.

calef defined it on 2026-09-19 in
[§157 (a trivial install is a web page, a USB drive, and packages over the internet)](../decisions/157-a-trivial-install-is-a-web-page-a-usb-drive-and-packages.md).
It is a web page from which one downloads a minimal system, writes it to a USB drive and installs
it. The
system then grows by installing packages over the internet.

## The rungs that are this milestone

[Milestone 198's rung table](198-package-manager.md#the-rungs) maps every rung to its milestone.
Two are here:

| Rung | Ships | Exit criterion a stranger could check |
|---|---|---|
| 1d. A PC that is not xenon | Nothing new if 1a to 1c hold | The same stick on one fleet machine reaches `$` at its own keyboard and monitor |
| 4. The web page | A published release and a page | A stranger with a PC, a USB stick and no prior knowledge follows the page to rung 3c's result; the stranger harness (`notes/stranger-test.md`) runs against the download, not the build |

The others it waits on:

- rung 1b, milestone 400 (the shell on the firmware's screen)
- rung 1c, milestone 242 (USB host and HID)
- rung 2, milestone 515 (the installer)
- rung 3b, milestone 494 (a driver for the network card a PC actually has)
- rung 3c, milestone 801 (packages over the internet)
 Milestone 243 (a machine with no serial port has no way to say anything)'s fleet supplies
the second PC.

## What must be true before the page goes up

§157 makes publishing calef's act. What a lane can make true first, so the act is only a decision:

- Rungs 1 to 3 pass on a machine that is not xenon, with a USB keyboard, so milestone 242 is
  built.
- A release exists. `gh release list` returns nothing today (§157's measurement); the download
  carries a checksum, and a signature if the trust or Secure Boot ruling creates a key.
- The Secure Boot ruling is answered on the page, before its first instruction.
- Something checks DECISIONS §135's requirement 1 ("no conveyed artifact carries copyleft") for
  the image the page conveys; §135's own `BUGS` says nothing enforces it.
- The stranger harness passes against the download, and records the time from page to prompt.
- calef holds `nifeos.org`. Whether the page lives there is calef's (§157).

## Which rulings it needs

| Ruling | The question | Where the options are |
|---|---|---|
| Secure Boot | Does a stranger turn Secure Boot off, or do we sign, and if we sign, is it the same key as the package key? | milestone 500 (a stick that boots with Secure Boot on, or a page that says how to turn it off) |
| Publication | Is it time to put the page up? | §157, step 1: calef's act |

The Secure Boot question is the `unwritten` decision dependency. Publication is `needs_person`.

**Reuse:** `stick_maker`, the installer and the package client are the tree's own and are built;
the release and the page are not surveyed yet, and the lane that builds them owes that survey under
§46 (thin primitives or whole subsystems).

## BUGS

- The rungs are sequenced on one Dell. Rung 1d is the second-machine criterion, and until it
  passes, every claim here is about xenon. A stranger's laptop may have no Ethernet (milestone 788
  (Wi-Fi on a PC that has no Ethernet)), a SATA disk (no AHCI driver; the installer's `BUGS`), or
  its NVMe behind Intel RST or VMD.
- Rung 1c is milestone 242, which milestone 192 (a keyboard on real silicon) prices at *"months rather than weeks"* and 242
  itself declines to price. This milestone is on its critical path.

## Index row

The door principle 1 names: calef, 2026-08-30, no third party sees nife until there is a package
manager and a trivial install. Split out of milestone 198 on 2026-10-06, which keeps the package
manager. A stranger follows a web page from a download to a stick to an installed system that grows
by packages over the internet (§157). It holds rungs 1d (a second PC) and 4 (the page), and waits
on the screen (400), the USB keyboard (242), the installer (515), packages over the internet (801),
the Secure Boot ruling and calef's act of publication.
