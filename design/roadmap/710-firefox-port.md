---
status: NOT-STARTED
raised: 2026-10-03
promoted_from: firefox-port
milestone_dependencies: none
decision_dependencies: 131, 145
machine_requirements: none
specific_machine: none
needs_person: yes
---
# 710. Porting Firefox to nife

Promoted from `design/roadmap/proposals/firefox-port.md` on 2026-10-03 (UTC). The number 710 was minted by the maintainer in a batch promotion of the proposal pile and is provisional until the queue lands it. *(Title and slug are drafts.)*

Raised by calef's ruling below. It builds nothing. It is the tracked home for the browser question
so that §131 and §145's "out of reach" has somewhere to point. The file name is provisional.

## calef's ruling

2026-10-03 (UTC): "We will need a web browser some day and it is logical one to go after. However
it is low on the priority stack. We're targetting service workloads first."

**This is low priority. Service workloads come first. No lane is to be started on it until calef
says so.** The only work this proposal licenses today is the measurement under "First step", and
that waits for the same word.

## What it reopens

- [§131 (hold at rung two)](../decisions/131-hold-at-rung-two.md): rungs three and four of
  [the display ladder](../display-ladder.md) reopen the parked competitor question, and calef
  held at rung two on 2026-08-26. A browser needs rung three (real applications, software
  rendering; WebRender has a software fallback, from general knowledge). Rung four, GPU through
  virtio-gpu Venus, is optional and would only make it faster.
- [§145 (compartmentalization at process cost)](../decisions/145-compartmentalization-at-process-cost.md),
  which names Firefox as the software people actually run Qubes for and puts it out of reach. That
  sentence is what this proposal revisits.

## The gap beyond graphics

Every item below is unmeasured and from general knowledge.

- A POSIX-style platform: threads, `mmap` with protection changes, signals.
- Firefox's multi-process model and its IPC (shared memory, sockets, handle passing). What a
  process costs is §145's concern, and the 66 microsecond `spawn_el0` figure there is the only
  number we have.
- JIT executable memory against W^X and capability confinement. Firefox runs with the JIT disabled,
  much slower.
- Networking and TLS, fonts, audio.
- Cross-compiling the build, including its Rust and C++ dependency tree.

## First step: a measurement, before any build work

Inventory the syscalls and libc functions a headless Firefox build uses, and diff them against what
nife's std PAL and libc layer provide.

- The question it answers: how large the platform gap is, in counted functions.
- What it would decide: whether the port is one milestone or a program of milestones.

Until it is run, nothing here is an estimate.

## Index row

calef ruled that a web browser is logical to go after some day but low on the priority stack, behind service workloads. Proposed: nothing built; the tracked home for the browser question, with one measurement that waits on calef's word.
