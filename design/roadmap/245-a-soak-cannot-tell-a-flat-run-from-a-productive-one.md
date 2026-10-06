---
status: NOT-STARTED
raised: 2026-09-03
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 245. A soak cannot tell a flat run from a productive one, so its duration is guesswork

Minted 2026-09-03 by calef, from the duration question asked of the first
radon soaks and answered from the literature in notes/soak.md. *(Number provisional until the merge
queue lands it.)*

Milestone 240 (the soak reports what happened and not where, so an eightfold
difference cannot be explained) built the placement instrument this would histogram, and it is
merged.

In brief. `script/soak` prints eight counters every five seconds and every one of them is a
volume. `rounds`, `wakes`, `crossings`, `remote`, `steals` and `deferred` all count how much
happened. Nothing counts how many *different* things happened, so no beat can be compared with the
beat before it and a run that stopped learning anything an hour ago looks exactly like one that is
still finding new interleavings.

That is why the duration question has no answer today. notes/soak.md's own research section reached
this and named the gap rather than papering over it:

> This is the gap worth closing before the duration argument is worth having

## Why this is the milestone and not more hours

The saturation is measured, by somebody else, on this exact question. Burckhardt, Kothari, Musuvathi
and Nagarakatte, *A Randomized Scheduler with Probabilistic Guarantees of Finding Bugs*, ASPLOS 2010,
instrumented a work stealing queue with twenty events and 168 possible event pairs and compared
coverage against run count:

> We restrict the horizontal axis to the 8192 runs as stress did not explore any new event pair beyond
> those already explored in the new runs after that and PCT eventually explored all the event pairs.
> [...] Fig. 11 shows that stress does not cover more than 20% of the event pairs, few of which result
> in a bug.

Their stress harness inserted random sleeps, thread suspensions and priority changes, which is a
superset of this soak's jitter. Coverage climbed, flattened, and the flat part was free. This
project has no way to see which part it is in.

And the cost of not knowing is now measured here rather than assumed. Two radon runs on
2026-09-03, the same card:

| | crossings total | crossings/s |
|---|---|---|
| the 2 h 59 m run | 5,509 | 0.51 |
| the run that followed it | 197,952 in 17 min | 186 |

The second boot passed the first's entire three-hour crossing count in about thirty seconds. A
duration chosen in hours is therefore choosing an unknown multiple of the thing that matters, and the
multiple is set by a boot-time lottery nobody controls.

## What would close it

A coarse histogram over the placement and scheduling decisions the soak makes, reported per beat,
so a beat can be compared with the one before it. Milestone 240 already computes the raw material:
`sched::last_cpus` knows where every worker is, and the census prints it whenever it changes. What is
missing is the summary that turns a sequence of arrangements into *"this beat saw an arrangement no
earlier beat saw"* or *"this beat saw nothing new."*

The shape is deliberately not specified here, because the interesting design question is what counts
as a distinct behavior, and that is the milestone rather than a detail of it. Some candidates, none
endorsed:

- Distinct settled arrangements, which is the coarsest and probably the most honest, given that
  240 found the arrangement is stable on silicon (`drifted=0` held for a whole run) where it churns
  under QEMU.
- Distinct (waker core, woken core) pairs, which is closest to PCT's event pairs and to the
  wake path radon's defect was once thought to live on. That reading was retracted by
  [`notes/visionfive2.md`](../../notes/visionfive2.md)'s fifth bench stop (2026-08-15), so this
  counts for PCT's sake and not for a recorded defect.
- Distinct orderings within a group's exchange, which is the most sensitive and the most
  expensive.

**Whatever it is, it must be cheap enough to run for hours** and must not itself perturb what it
measures. notes/soak.md already records a 6% throughput effect from merely draining the serial port,
seen once, which is the standing warning about instruments on this workload.

## The proof that this milestone worked

**A run says, from its own output, whether it is still finding new behavior**, and two runs of
different lengths can be compared on that basis rather than on hours. Concretely: a beat line, or a
line beside it, from which a reader can see the curve flatten.

Not a new counter that rises monotonically, which is what the existing eight already do.

## BUGS

- This does not decide a duration, and should not be read as promising one. It makes the duration
  decidable from evidence; the number stays an architect's, on the axis notes/soak.md gives them.
- A flat histogram is not proof a run is worthless. PCT's own finding is that stress covers a
  fraction of the space and stays there; a soak that has gone flat on placement may still be
  accumulating hours of the *same* interleaving, which is exactly what a wearout or leak question
  would want and a concurrency question would not.
- It measures this soak's behavior, not the kernel's. A histogram over the workload's placements
  says nothing about paths no worker takes, and reading it as a coverage number for the scheduler
  would be the same overclaim `notes/mutation-testing.md` warns about for its own score.
- **Nothing here helps the slow draw.** A run at 0.51 crossings per second will look flat because it
  is barely doing anything, and distinguishing "flat because saturated" from "flat because starved"
  is a real hazard this block does not solve.

## Index row

all eight soak counters are volumes; nothing counts distinct behavior, so a saturated run and a
productive one look identical
