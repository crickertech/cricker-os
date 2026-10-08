---
status: NOT-STARTED
raised: 2026-10-08
milestone_dependencies: 835, 172
decision_dependencies: 265
machine_requirements: none
specific_machine: none
needs_person: no
---
# 838. A C library: `posix_spawn`, and no fork

*(Minted 2026-10-08 (UTC) by lane/c-library from calef's ruling the same day; number provisional
until the merge queue lands it.)*

The fork ruling on PR #1856 declines fork permanently and supports spawn. §265 (a C library started
from relibc) follows it. relibc already routes `posix_spawn` through a platform call rather than
through fork and exec. This stage writes that call on milestone 172 (a capability-native subprocess
primitive). It also rewrites the three generic functions that call fork (`popen`, `system` and
`forkpty`) on `posix_spawn`, or refuses each in `BUGS` with its reason.

Reuse: relibc's `spawn` header and its `posix_spawn` file-action types (MIT), over milestone 172's
primitive.

## What this has to decide

`posix_spawn` names a program by path and passes file actions. On nife a child holds only what its
parent grants, so the file actions become grants: an `addopen` is a directory or file capability
the child receives, and an `adddup2` moves one. Whatever the parent does not pass, the child does
not have. That mapping is a design fork for an architect when this milestone is taken, not before.

## First consumers

netperf's server in a spawn mode, if 829 wants one; any C program that runs a helper. No standard
benchmark needs it to run, which is why it is last.

## Done when

A C program spawns a child with `posix_spawn`, waits for it with `waitpid`, and collects its exit
status, on all three architectures; `popen` and `system` either work on it or are refused with a
reason.

## Index row

Process creation for nife's C library: `posix_spawn` on milestone 172, and `popen` and `system`
rewritten on it, because fork is never coming.
