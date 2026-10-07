---
status: NOT-STARTED
raised: 2026-09-19
promoted_from: live-patching-the-kernel
milestone_dependencies: none
decision_dependencies: unwritten
machine_requirements: none
specific_machine: none
needs_person: no
---
# 509. Live-patching the kernel, so that even a new kernel does not need a reboot

*(Number provisional until the merge queue lands it.)* Promoted from the
proposal `live-patching-the-kernel`, filed 2026-09-19, on calef's instruction of 2026-09-20 to give
every proposal on `main` a number. The text below is the proposal's own, unedited except for this
paragraph: the argument is its author's and promotion is not the moment to improve it. Recorded by
the maintainer from calef's ruling in DECISIONS §159 (lab machines upgrade like user machines, and): *"Live-patching a kernel is a separate
milestone that we may not get to for a very long time. Rebooting for a new kernel is fine for now."*

Deferred by calef, deliberately and for a long time. This file exists so the
idea has a home and is not re-proposed as new.

## In brief

§159 makes every userspace upgrade reboot-free and accepts one reboot for a new kernel. Removing
that last reboot means replacing or patching kernel code while the machine runs, which Linux does
with livepatch (function-level redirection) and kexec (a fast reboot into a new kernel, which is
still a reboot). Neither has been read for this file; both are named from memory.

## Why it waits

- A microkernel's argument is that the kernel is small and changes rarely, so the reboot it forces
  is rare. Live-patching spends a great deal to remove the one reboot the architecture already made
  uncommon.
- It is on the verification path: a kernel that rewrites its own code while running is the hardest
  thing in the tree to prove anything about, and the Kani harnesses assume the code they check is
  the code that runs.

## What would promote it

A customer whose uptime requirement a kernel reboot breaks, measured, after upgrades without a
reimage (§159) work. Until then, `script/roadmap --proposed` shows its age, which is the intent.

## Index row

§159 makes every userspace upgrade reboot-free and accepts one reboot for a new kernel.
