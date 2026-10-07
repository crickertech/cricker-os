---
status: DECIDED
raised: 2026-09-26
decided: 2026-09-26
ratified_by: calef
---

# 224. No `pwdx`: only the shell has a working directory, so there is nothing to report

calef, 2026-09-26 (UTC), on milestone 126 (who else is running, and who is allowed to ask)'s `pwdx`
fork: *"Yes, decline pwdx."* *(Section number provisional until the merge queue lands it.)*

## The question

`procps` ships `pwdx`, which prints another process's current working directory. On Linux it reads
`/proc/PID/cwd`. Milestone 126's block had filed it beside `w` as "print a name for a tid", which
was the wrong reading; pull request #1349 corrected that and wrote the fork up in section 1 of
[`notes/process-view/what-is-left.md`](../../notes/process-view/what-is-left.md). The options there
were to decline it, to have the shell answer a query about its own position, or to have the kernel
keep a path per thread.

## The decision

No `pwdx`. `procps` ships without it, and `pwd` is the answer for the one process that has a
directory to report.

## Why

A working directory here is not a kernel fact. It is a value, `grant_plan::nav::Cwd`, held in the
shell's own `Holdings` (`crates/grant_plan`). A grep for `Cwd` across `components/` and `crates/`
finds it in `grant_plan`, `grant_plan::nav` and the shell (`swish`), and nowhere else, checked
2026-09-26. Every other program receives grants, not a position to resolve against. So `pwdx` has
nothing to report about any process but the shell, and the shell already prints it with `pwd`.

The other two options lose on their own terms. A query the shell serves is a new protocol two
programs agree on, for a fact only its owner has any use for. A path kept by the kernel is wrong on
its face: the kernel resolves no paths, and a `Cwd` names components inside one capability, which
means nothing to a holder of a different one.

This is the shape §115 (no `sysctl`) took, and the `pkill` refusal before it. A Unix program whose
job is reaching into state the caller does not hold is declined. The gap in the package is recorded
rather than papered over.

## How reversible it is

Nothing is written against the absence of `pwdx`, so it can be revisited. The condition that would
reopen it is a second kind of process that holds a working directory of its own.

## What it unblocks

Milestone 126 loses one waiting fork. The package's coverage claim carries the gap plainly:
`procps` ships without `pwdx`, and a reader who expects to ask where another process is will not
find a program for it.
