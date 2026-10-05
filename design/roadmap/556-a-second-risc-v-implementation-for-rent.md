---
status: SUPERSEDED
raised: 2026-09-20
promoted_from: a-second-risc-v-implementation-for-rent
superseded_by: 89
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 556. A second RISC-V implementation, for €16 a month

Superseded 2026-09-25, by milestone 89 (Scaleway EM-RV1: a second RISC-V implementation, rented), which made the same proposal on 2026-08-03 and now carries the port plan, after calef ruled to rent the machine (§215 (the second RISC-V machine is a rented Scaleway Elastic Metal RV1)). Nothing below was lost: the offer is re-read in §215, the port's distance is measured in milestone 89, and this block's two exits (a vendor blob, a 0% SLA) are milestone 89's BUGS and §215's open questions. *(This file was renumbered on 2026-09-22 from 527 to 556 because a concurrently merged lane had taken 527.)* Promoted from the proposal `a-second-risc-v-implementation-for-rent`, filed 2026-09-20, on calef's instruction of 2026-09-20 to give every proposal on `main` a number. The text below is the proposal's own, unedited except for this paragraph.

As filed: everything the port needs is in this tree, and the machine can be rented by the hour. Milestone 89's gate is HARDWARE.

## What is rentable

Scaleway's Elastic Metal RV1, read from the product page on 2026-09-20 rather than recalled:

| | |
|---|---|
| SoC | T-Head TH1520, C910, RV64GC |
| Cores | 4 at 1.85 GHz |
| Memory | 16 GB LPDDR4 |
| Storage | 128 GB eMMC |
| Network | 100 Mbit/s |
| Price | €0.042 an hour, €15.99 a month |
| Console | serial over SSH, activatable per account |
| Own OS | supported: *"access to the server's serial console is available for installing the most exotic operating systems"* |
| Caveat | a Labs product, 0% SLA |

The two things that make it usable for nife are the two the page confirms: a serial console, and
permission to install an operating system that is not one of their three Linuxes. Provisioning is
command-line driven, which is what turns a bench session into a job.

## Why this is worth more than cheaper radon time

**It is a different implementation, not a second copy.** radon is a StarFive JH7110 with SiFive U74
cores; this is a T-Head TH1520 with C910. A port to it exercises everything this tree assumed about
one vendor's silicon: the UART's address and behaviour, what firmware hands over, the interrupt
controller's layout, the device tree's shape, and whatever the C910 does differently about memory
ordering and cache maintenance.

DECISIONS §19 (architectural parity is a tenet) makes parity a gate rather than an aspiration, and
the strongest reading of that rule is
that two implementations of an architecture are what turn "it works on riscv64" from a claim about a
board into a claim about an ISA. **This tree has never had a second implementation of any
architecture.**

And it fixes an asymmetry that would otherwise be quiet. DECISIONS §203 (capacity is rented rather
than bought) moves aarch64 and x86_64 hardware legs to rented runners. Without this, riscv64's legs
stay bench-bound while the other two become gate-able, and a reader six weeks later sees "hardware
legs in CI" and assumes three architectures.

## What the port would take, honestly unpriced

The work is unknown in size and that is the first thing to establish. What is known:

- **The boot path is not radon's.** notes/visionfive2.md records what the JH7110 needed, including a
  UART at an address with different silicon behaviour behind it, and none of that transfers.
- What the firmware hands over decides the shape. Whether the TH1520 boots through OpenSBI, what
  the device tree contains, and how the console is reached are all facts to be read off the machine
  rather than predicted.
- The eMMC and network are not needed for a first light. A serial console and a kernel that
  prints a byte is the same first milestone every board here has had, and it is how this would be
  scoped: first light first, everything else after.

## What would make this not worth doing

Stated so a lane does not have to discover it: if the port turns out to need a vendor kernel, a
signed bootloader, or a firmware blob this tree cannot inspect, it stops being a nife target and
becomes a Linux box that happens to be RISC-V. The serial console and the custom-OS statement suggest
otherwise, but they are marketing until a byte comes out of that port.

The other honest exit: 0% SLA on a Labs product. A gate that depends on a machine the provider may
withdraw is a gate that will one day be red for a reason nobody can fix. Anything built on this should
degrade to a skip that says so loudly, the way milestone 81 (an HVF leg: the test suite on the physical core) already does.

## Index row

Scaleway's Elastic Metal RV1, read from the product page on 2026-09-20 rather than recalled:
