---
status: NOT-STARTED
raised: 2026-08-31
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 209. A session starts in a fit state, or says which part is not

Minted 2026-08-31 by calef, after a day in which the machine slept twice and
killed two lanes mid-response, the `launchd` watchers were found unloaded, and the toolchain link
pointed into a worktree about to be pruned. *(Number provisional until the merge queue lands it.)*

Every check it makes is a command that already exists; what is missing is anything
that runs them together and says which one is false.

In brief. Several things must be true for work to proceed safely, and each is currently checked
by somebody remembering. calef, 2026-08-31: *"Shall we make it a standard to caffeinate when a
session is started... Seems like maybe that should be built into a program of some sort."*

This is the operational half of what `script/catch-up` already does for information. That script is a
computed view of what changed; this is a computed view of whether the machine is fit to work in.

## What it checks, and every one of these failed today

| | what goes wrong | how it was found today |
|---|---|---|
| Sleep is held | an unattended lane dies mid-response and its uncommitted work is one prune from gone | two lanes killed in an hour; a 138-line enumeration and a real `rmle` overflow bug were untracked when it happened |
| The watchers are loaded | the queue stalls and trunk goes red with nobody assigned | `launchctl list` was empty five days after the plists were installed to make remembering unnecessary |
| `nife-dev` points at the main checkout | pruning a worktree breaks the toolchain account-wide, surfacing far from the cause | it pointed into a lane's worktree that was about to be pruned |
| No leaked QEMU | emulators accumulate and hold `target/nifefs.img`, failing unrelated tests | AGENTS.md records eleven leaked over one day, 729% CPU |
| No unclaimed lane branch | two lanes can take one milestone | three of five lanes today pushed a branch and opened no draft pull request |
| Disk and load headroom | the 2026-07-31 zero-bytes incident; today's OOM | load hit 83 on eight cores with three lanes and a mutation sweep |

## The distinction that decides the design

Machine-scoped things belong to `launchd` and this script only verifies them. `merge-drain` and
`trunk-health` already work that way and were right to; the script must not start them silently,
because loading a `launchd` job is a machine-global act and a session should not take one without
saying so.

Session-scoped things are this script's to hold. `caffeinate` is the clear case: holding sleep
forever on a laptop is wrong, and holding it while lanes are running is right. That is a property of
the session rather than of the machine, which is exactly why it does not belong in `launchd` next to
the watchers.

## What it must not do, and each of these has bitten

- Never kill a QEMU it finds. AGENTS.md's rule is to walk `ps -o pid,ppid` upward first,
  because a QEMU whose parent chain ends in a live harness is somebody's gate in flight; the
  maintainer killed a lane's mid-suite emulator this way on 2026-08-15. Report, never reap.
- Never relink `nife-dev` while a gate is running. That is the same race in the other direction,
  the one that took the link out from under a lane on 2026-08-04. Detect and refuse rather than fix.
- Never start a second `caffeinate`. Check the assertion (`pmset -g assertions`) rather than the
  process, since a timed one from elsewhere may already be holding it and will expire.
- Never load a `launchd` job silently. Say it is unloaded and how to load it.

## The rung it should reach

A script is rung two: it fires only when run. **The thing that makes it fire without anybody
remembering is a `SessionStart` hook in `.claude/settings.json`**, checked into the repository, which
is rung one for the sessions this project actually runs in.

That is worth doing and is not sufficient on its own: the script must stand alone for a person at a
prompt, for another tool, and for a machine that is not calef's. The hook is how it gets invoked, not
what it is.

## BUGS

- This is operational hygiene and is on no fatal risk's path. It buys back attention and prevents
  losses; it moves none of the nine.
- A preflight that reports six things nobody reads is worse than none. The output has to be
  silent when everything is fine, which is `merge-drain`'s own posture, or it becomes noise within a
  week.
- It cannot check the thing that actually costs most, which is whether a lane is committing as it
  goes. Twice today uncommitted work survived only because a worktree did.
- **The name is provisional.** An architect names things, and this one is awkward: it both reports
  and acts, so it sits between `script/`'s noun-shaped reporters and its verb-shaped doers.

## Index row

calef, 2026-08-31, after a day in which the machine slept twice and killed two lanes mid-response,
the `launchd` watchers were found unloaded, and `nife-dev` pointed into a worktree about to be
pruned. The operational half of what `script/catch-up` does for information. The design turns on
one distinction: machine-scoped things belong to `launchd` and are only **verified** here, while
session-scoped things like `caffeinate` are this script's to hold. It must report and never reap:
killing a QEMU or relinking the toolchain blindly are both races that have already bitten.
