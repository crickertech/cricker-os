---
status: BUILT
raised: 2026-09-02
built: 2026-09-03
---
# 226. `qemu-bounded.sh` leaves an emulator behind, and the next run blames the wrong thing

Minted 2026-09-02 by the maintainer, from milestone 127's EL2 entry
lane, which hit it twice in one session. *(Number provisional until the merge queue lands it.)*

Both shapes, because they answer different failures. The leak is prevented everywhere anything
can prevent it, and the one case macOS cannot prevent now arrives as a sentence instead of a
mystery.

The leak was reproduced before it was fixed, which is the part worth keeping. Start a bounded
run, signal the wrapper and its detached killer the way `pkill -f` does, and the emulator is
inherited by pid 1 and runs until somebody notices. `helpers/qemu-bounded-selftest.sh` is that
reproduction as an artifact. Run against the pre-226 script it fails three of its seven cases (both
orphan cases and the lock diagnostic); against the current one it passes all seven.

What the killer does now. It had one reason to fire, the bound, which bounds a run that is
allowed to finish and does nothing about a run whose wrapper is killed. It has three:

1. the bound expired, unchanged in effect;
2. the wrapper is gone, noticed by a `kill -0` poll once a second, which is what covers a
   SIGKILLed wrapper, a dead session and a closed terminal, none of which run a trap anywhere;
3. the killer itself was signalled (TERM or HUP), whose trap kills the child on the way out, so
   the one process that knows the child's pid never takes that knowledge with it.

**SIGINT is deliberately not in that trap list.** A shell puts an asynchronous subshell's SIGINT to
ignore before the subshell can trap it, so listing it would be a lie that reads as coverage. Ctrl-C
is covered better anyway: the wrapper dies of it, the killer survives it, and reason 2 fires within
a second.

The early-reader property was verified rather than assumed, since the block named it as a real
requirement. A real QEMU writing continuously into a pipe whose reader takes five lines and exits is
still killed at the bound. So is the milestone 38 property beside it: a fast child in a pipeline
returns at once rather than at the end of the bound, and it now does so because the killer's output
goes to `/dev/null` instead of into the pipe, which closes that bug at the root rather than at the
cleanup.

**And the macOS constraint was tested, not reasoned about.** `perl -e 'alarm 3; exec @ARGV'` against
the installed `qemu-system-aarch64` still leaves it running eight seconds later.

**The lock diagnostic.** On a failing exit, and only then, the wrapper runs `lsof` over the image
paths in its own command line and prints the pid, ppid, start time and command of anyone holding one
**for writing**. The write filter is not a refinement, it is the difference between a diagnostic and
noise: the first version reported readers, and `script/test`'s three legs share one read-only OVMF
firmware file, so every green run that overlapped another lane's x86_64 gate printed a holder and
was wrong to. Found by running the suite rather than by reading the code, and the false positive was
itself the case the message warns about, since the other QEMU was milestone 235's lane mid-gate with
a live harness up its parent chain. It reports rather than kills, and says to walk the parent chain up first, because a QEMU whose
parent is a live harness is somebody's gate rather than a leak (AGENTS.md, 2026-08-15).

In brief. `helpers/qemu-bounded.sh` exists because `timeout(1)` does not exist on macOS and
`perl -e 'alarm N; exec @ARGV'` does not work on QEMU, which installs its own `SIGALRM` handler and
swallows the alarm. The script uses `SIGTERM`, which QEMU honours, and detaches its killer so it
survives a pipeline whose reader exits early.

**It does not survive its target detaching.** When the EL2 lane's run was killed, the emulator
survived with `ppid 1`. It then held the write lock on `nifefs-blank.img`, and the next boot failed
with `Failed to get "write" lock`, which names nothing about the real cause. The lane lost time to
it twice before recognising the shape.

## Why this is worth a block rather than a habit

AGENTS.md already narrates this failure from the other direction, at length: killing a loop script
does not kill its descendants, so a check honestly reporting "no qemu" is followed by a command that
finds one. Its advice is to ask who holds the file (`lsof target/nifefs.img`) rather than whether a
process matches a name, and to kill the tree at its root.

That advice is rung four, and this is the instance where rung four failed in the ordinary way: a
lane that had read the file still lost time, because the symptom arrives on a *later* run as a lock
error about a file, with nothing pointing at a leaked process.

Every lane uses this script. A leak here is not one lane's problem, it is the next lane's mysterious
failure, and this project runs many lanes.

## What it needs

An emulator that cannot outlive the thing that started it, or a failure that names the cause when
one does. Two shapes, and the block picks neither:

- Make the leak impossible. A process group killed as a group, or a supervisor that dies with its
  parent. Stronger, and it must not break the reason the killer is detached in the first place, which
  is a real requirement rather than an accident: a pipeline whose reader exits early must still get
  its emulator killed.
- Make the symptom honest. Whatever reports `Failed to get "write" lock` could say which process
  holds it, since `lsof` answers that in one call. Weaker, and it does not stop the leak, but it
  turns a mystery into a sentence.

**These are not exclusive and the second is worth having regardless**, because a lock can be held by
something this project did not start.

## BUGS

- A SIGKILL to the killer defeats all of it, and nothing on macOS can fix that. SIGKILL is not
  trappable and macOS has no `prctl(PR_SET_PDEATHSIG)`, so a supervisor shot in the head cannot hand
  off. `kill -9` at a whole process group is the realistic way to hit it. This is the case the lock
  diagnostic exists for, and the self-test deliberately does not assert on it: asserting on a known
  defect only pins it in place.
- The parent-alive poll can be fooled by pid reuse. If the wrapper dies abnormally and its pid is
  reused inside the bound, the killer waits out the full bound, which is exactly the old behaviour
  and never worse. It cannot fail the other way: a live wrapper's pid is not reused.
- The lock diagnostic only inspects arguments that look like disk images (`*.img`, `*.qcow2`,
  `*.raw`, and any `file=` field of a comma-separated option), and only reports holders with the
  file open for writing. A lock held on something named differently is not reported, and neither is
  a reader, which is deliberate and is the reason the message is quiet on a green run.
- The self-test is in no gate, so nothing runs it unless a person does. It starts real emulators
  and costs about a minute, which is a poor trade against every lane for a script that changes twice
  a year. That is rung two declined on purpose, and it is a foot gun: a later simplification of
  `qemu-bounded.sh` will not be caught by CI.
- **This block does not say how often it happens.** Two occurrences in one lane on one day is what
  prompted it, and nobody swept for others.
- The macOS constraint that produced this script is unchanged, and was rechecked rather than
  assumed: `perl`'s alarm is still swallowed by the installed QEMU.
- It says nothing about the emulators a killed harness leaves, which AGENTS.md records as a
  separate and larger failure: eleven QEMU processes over one day, the oldest with eight hours of CPU
  time on it.

## Follow-on

- **Refused.** The self-test is in no gate, on purpose. It starts real emulators and costs about a
  minute against every lane, for a script that changes twice a year, which is a poor trade. That is
  rung two declined deliberately, and the block says plainly that it is a foot gun: a later
  simplification of `qemu-bounded.sh` will not be caught by CI.
  This refusal is milestone 476 (design/roadmap/476-the-qemu-bounded-self-test-in-a-gate.md), which
  carries it with the condition that would change it.
- **Recorded.** In `design/roadmap/226-qemu-bounded-orphans.md`: a SIGKILL to the killer defeats all
  of it and nothing on macOS can fix that, since SIGKILL is not trappable and there is no
  `prctl(PR_SET_PDEATHSIG)`. That is the case the lock diagnostic exists for, and the self-test
  deliberately does not assert on it, because asserting on a known defect only pins it in place.
- **Recorded.** In `design/roadmap/226-qemu-bounded-orphans.md`: the parent-alive poll can be fooled
  by pid reuse, in which case the killer waits out the full bound, which is the old behaviour and
  never worse.
- **Recorded.** In `design/roadmap/226-qemu-bounded-orphans.md`: the lock diagnostic only inspects
  arguments that look like disk images and only reports holders with the file open for writing, so a
  lock on something named differently is not reported and neither is a reader. That is what keeps it
  quiet on a green run.
- **Recorded.** In `design/roadmap/226-qemu-bounded-orphans.md`: nobody swept for other occurrences,
  so the block cannot say how often this happens. Two in one lane on one day is what prompted it,
  and now that the leak is prevented everywhere anything can prevent it the count is history rather
  than a live number.
- **Recorded.** In `AGENTS.md`: the emulators a killed harness leaves behind are a separate and
  larger failure, eleven QEMU processes over one day with the oldest holding eight hours of CPU
  time. It gives the habits that catch them, which are to ask who holds the file, kill the tree at
  its root, and walk the parent chain up before killing anything.

## Index row

Both shapes. The killer had one reason to fire, the bound, which bounds a run allowed to finish
and does nothing about a run whose wrapper is killed: the emulator was then inherited by pid 1,
held a disk image's write lock, and billed a later run with QEMU's `Failed to get "write"
lock`, which names a file and no process. It now also polls its parent once a second (covering
a SIGKILLed wrapper and a dead session, neither of which runs a trap) and traps TERM and HUP,
so the one process knowing the child's pid does not take it along. SIGINT is deliberately absent:
a shell puts an async subshell's SIGINT to ignore before it can be trapped, so listing it would
read as coverage and be a lie. The leak was reproduced before and after and `helpers/qemu-bounded-selftest.sh` is that reproduction as an artifact: three of seven cases fail
on the old script, none on the new. The early-reader property and milestone 38's fast-child
property were both verified against a real QEMU, and `perl`'s alarm is still swallowed, so the
premise holds. What macOS cannot prevent (SIGKILL to the killer; no PDEATHSIG) gets the other half
instead: a failing run runs `lsof` over its own image paths and names the holder's pid, ppid and
start time, and says to walk the parent chain up first.
