---
status: DECIDED
raised: 2026-09-19
decided: 2026-09-19
ratified_by: calef
---

# 159. Lab machines upgrade like user machines, and only a new kernel needs a reboot

calef, 2026-09-19, in conversation with the maintainer, shortly before
17:41 UTC:

> I would like to get to the point where the lab machines are like user machines. New package
> versions can be upgraded via the package manager rather than needing a full reimage, inclusive of
> the kernel. One of the benefits of our microkernel should be with so much running in userspace
> that not much should require a reboot.

And at 17:41 UTC, on whether a new kernel may need a reboot:

> I think Live-patching a kernel is a separate milestone that we may not get to for a very long
> time. Rebooting for a new kernel is fine for now.

*(Section number provisional until the merge queue lands it.)*

## What was decided

1. The lab machines (argon, radon, xenon) are upgraded the way a user's machine is: a new
   version of a package, the kernel included, arrives through the package manager (milestone 198),
   not by rebuilding and rewriting the whole image. They are the first customer of package upgrades,
   which is the reason milestone 198 gave for doing packaging early: the builders pay for its
   absence.
2. **Upgrading a userspace component does not reboot the machine.** That is the microkernel's claim
   made into a requirement: drivers, filesystems and the network stack are processes, so replacing
   one is replacing a process.
3. A new kernel takes effect at the next reboot, and that is acceptable. Live-patching a running
   kernel is a separate milestone, deferred for a long time (proposal:
   `design/roadmap/509-live-patching-the-kernel.md`).

## What it settles elsewhere

- Trust (milestone 198's fork, DECISIONS §195): T1 alone
  is ruled out. Today the kernel compiles in the digest of the table that vouches for every
  program, so a kernel and its archive are one sealed set; under T1 every package upgrade is a new
  image. The remaining choice is T2 (a publisher's signature checked by a measured userspace
  program), T3 (the machine's owner vouches), or both, with the image's own table kept as the floor.
  That choice is still calef's.
- §116 has its customer. §116 declined live state handoff on 2026-08-23 "for want of a
  customer". A lab machine upgrading a stateful server in place is one. This section does not decide
  the handoff; it records that the trigger §116 waited for has arrived, and it orders the work in
  two tiers:

  | Component | Upgrade without a reboot | What it needs |
  |---|---|---|
  | Stateless (the console) | Built: live replacement (§41) | Nothing |
  | Stateful, a restart is acceptable (the filesystem server, the network stack) | Stop the old instance, start the new one, clients reconnect | Supervision restarting a component under a new binary, and clients that survive a reconnect |
  | Stateful, no interruption allowed | Open files and connections handed to the new instance | §116's handoff, a wire format and so calef's |

  The restart tier comes first; the handoff tier waits for a component that cannot tolerate one.
- Milestone 198's rungs gain an exit criterion: a lab machine takes a new package version with
  no reimage and no reboot, and a new kernel with one reboot.

## A measure worth publishing, recorded so it is not lost

What forces a reboot is a comparison this demonstrator can make honestly: on nife, the kernel;
on Linux, the kernel and every driver and filesystem built into it (live-patching aside, which both
would then have). It belongs in the benchmarks beside the other cross-OS numbers once upgrades work,
and it is a claim about architecture that a measurement, not an adjective, should carry.

## What this does not decide

- The package format, the activation shape, T2 versus T3, and the handoff wire format.
- Whether the kernel stays one file with the loader and the base archive. With T2 or T3 the image
  can shrink to the kernel and a minimal base, with everything else a package; that is milestone
  198's to design.
