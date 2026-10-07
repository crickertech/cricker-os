---
status: BUILT
raised: 2026-08-03
built: 2026-08-03
---
# 84. Stack high-water: measure kernel stack depth

Raised 2026-08-03, same survey as 79.

A kernel stack overflow does not fault helpfully; it scribbles. This tree has had one (the FS-server
stack bug, notes/nifefs.md), it was found the expensive way, and nothing since measures depth on
any kernel stack. The claim "the stacks are big enough" is currently an argument, and the project's
standard is to measure instead.

The instrument is the classic one because the classic one is right: paint every kernel-owned stack
with a pattern at boot, and at the end of the test suite scan each for the deepest overwritten byte
and report it through the test channel. The first run is measurement only. Once the numbers are
seen, the threshold becomes an assertion with the margin the numbers justify, the same
measure-then-gate sequence the icount tripwire used. Runs identically on every ISA, and covers
whatever the suite actually exercises, interrupt nesting included.

## Scope note

A watermark sees only exercised paths; an unexercised deep path stays invisible, which is the same
limit coverage has. The static complement (`-Zemit-stack-sizes`, worst-case frame accounting) breaks
on indirect calls, so it lands as an advisory report if it lands at all. And the check must not
become load-bearing the way milestone 78's assertions did: depth is a property of the code and the
suite, not of the host, so this one should be immune to runner noise by construction.

## Follow-on

- **Milestone 124.** The static complement this block priced as "an advisory report if it lands at
  all" landed as a real instrument: `script/stack-depth-check`, a call-graph walker that hangs
  `-Z emit-stack-sizes` frames on the graph and takes the longest path from a thread stack's entry
  points. It earned its keep by refuting the depth hypothesis behind the guard-page faults rather
  than by measuring one.
- **Recorded.** In `notes/stack-high-water.md`: a watermark sees only the paths the suite exercises,
  so an unexercised deep path stays invisible, and the walker's answer is a lower bound because
  indirect calls and hand-written assembly carry no `.stack_sizes` entries. The same section carries
  the rule that the two thresholds must be raised together or they stop describing the same stack.

## Index row

The FS-server stack overflow already happened once, and nothing since bounds depth on any kernel
stack. Paint at boot, read the mark at suite end, assert headroom. Works identically on every ISA
and covers every path the suite takes
