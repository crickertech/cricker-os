# How this is built, and why the method is half the claim

**Draft for calef's review, 2026-09-21. Every number here leaves the machine if this is published,
so it is his to approve or cut, not a lane's and not the maintainer's.**

`AGENTS.md`'s second principle says the method is a result and that nothing in this tree states it.
This page is the attempt. It is written for a stranger who has found this repository and wants to
know what they are looking at.

## The two claims

**nife is a demonstrator, and it makes two claims rather than one.**

1. **A capability microkernel can run real workloads.** A small machine-checked trusted core, every
   driver and server an ordinary process, hosting software that was not written for it.
2. **A system of this size can be built this way at all**, where "this way" means many machine
   agents working in parallel lanes with one person reviewing architecture and outcomes.

The second claim is the one this page is about. It is probably the more interesting of the two to
someone who is not building an operating system, and it is the one nobody had written down.

## The numbers, measured on 2026-09-21

From a first commit on 2026-07-12: **71 days**, 5,039 commits, **228 milestones built**, 203
architecture decisions, 72 crates, 50 user programs, **178 Kani proof harnesses**,
39,892 lines of kernel code and 88,334 elsewhere, on **three architectures**, with a booting kernel
on real RISC-V silicon, a shell, a filesystem, a network stack, a compositor, and an unmodified
third-party program (`ripgrep`, zero patches) running on all three.

One person is the architect and reviewer. He does not write the lines.

**The human cost, because it is the number that makes the rest mean anything.** calef works on this
full time, so **one calendar week is one person-week**, and the whole project to date is **about ten
person-weeks** of human effort. That is his own statement rather than a measurement, and it is the
denominator every figure above should be read against.

**What that is worth comparing to, carefully.** Atmosphere, a hardware-isolated Verus-verified
microkernel, reports **1.5 person-years on verification alone**; seL4 reports about eleven
person-years, plus nine more. **Those are not the same work**, and the comparison is dishonest if
stated without that: seL4 and Atmosphere carry machine-checked proofs of functional correctness for
their kernels, where this tree has 178 Kani harnesses covering parts of it and a risk register that
says so. The honest sentence is narrower: **ten person-weeks bought a three-architecture capability
microkernel that boots on silicon and runs unmodified third-party software**, and what it did not
buy is a verified kernel.

## Five caveats, because the numbers are worthless without them

**There is deliberately no denominator here.** An earlier draft said "228 built of 515 recorded",
and that ratio means nothing: the roadmap holds refusals, proposals promoted to numbers, and
decisions recorded as milestones, so it grew by 68 entries in a single day without anyone building
anything. A reader would have computed a completion percentage and been wrong, and that would have
been this page's fault rather than theirs.

**This is size and rate, not quality.** A count of milestones marked BUILT is a count of blocks
marked BUILT. This tree found nine of them misrecorded in a single sweep, and a whole risk register
exists because the proofs might be proving trivia.

**The line count includes comments, deliberately**, and `kernel/src` measures about 40% comment. Any
comparison against another project belongs in code lines.

**The rate is not the method; the gates are.** The same speed without them produces a great deal of
code nobody can trust, faster. What actually holds this together is a set of machine checks that
fail loudly: every architecture builds and boots or the merge fails; a proof harness must have a
recorded way to make it go red; a citation must say what it cites; a refusal must say what would
change it; a limitation is documented beside the feature rather than in a tracker. **Every failure
this project has had is evidence for that**, and they are written down in the same files as the
successes.

**One architect is a bottleneck that moves.** On one night eleven lanes shipped and the queue went
idle twice, because the constraint had stopped being how fast lanes produce and become how fast one
merge queue lands. Adding lanes made it worse.

**And the honest one: this has no customer.** The project's own first principle ranks work by the
shortest path to a system somebody runs, and in August 2026 the first customer went elsewhere,
because this system could not meet a real deadline. That is recorded as the principle working rather
than failing, and the customer path is still vacant.

## What a stranger can check

Nothing here asks to be taken on trust, which is the point of publishing it:

- **`design/fatal-risks/README.md`** lists nine claims that, if false, mean this project should stop, each
  with the experiment that would settle it and the honest verdict so far. Two are amber. One already
  fired.
- **`design/decisions/`** holds every architectural decision with its reasoning, **including the ones
  that were refused**, so a reader can disagree with an argument rather than with an authority.
- **`design/roadmap/`** holds every milestone, including 42 marked REFUSED, each stating what would
  change its mind.
- **The gates are scripts in `script/`** and they run on every pull request. They are readable, and
  each one says in its own header what it cannot check.

## What this page does not claim

That the method is better than a team of people. Nobody here has run the control experiment, and the
comparison this project can honestly make is against its own record rather than against anyone
else's. What can be said is narrower and still unusual: **this is what it looks like when the work is
done this way, with the failures left in.**
