---
status: NOT-STARTED
raised: 2026-09-20
promoted_from: a-dying-programs-last-words-reach-nobody
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 496. A dying program's last words reach nobody

*(Number provisional until the merge queue lands it.)* Promoted from the
proposal `a-dying-programs-last-words-reach-nobody`, filed 2026-09-20, on calef's instruction of
2026-09-20 to give every proposal on `main` a number. The text below is the proposal's own, unedited
except for this paragraph: the argument is its author's and promotion is not the moment to improve
it. Written by the lane for milestone 442 (a crypto provider `rustls` can use on all three bare-
metal targets), which lost most of a day to this and then found that the thing it was chasing did
not exist.

Everything needed to reproduce it is in the tree.

## The correction this file exists to record

This proposal said something else on 2026-09-19. It claimed that a `std` program calling
`poly1305::Poly1305` directly died before executing the first statement of `main`, and it laid
out the evidence: an abort at `__rust_start_panic`, a clean three-segment binary, the failure
surviving every compiler flag. That claim was false, and the way it was false is the finding.

The program did not die early. It ran, printed six correct lines, reached a failing vector, panicked
with a perfectly clear message naming both values, and nobody could read any of it.

## What actually happens

`kernel/src/user/std_tests.rs`'s `drain_sink` reads the sink endpoint in a loop and stops on the
end-of-stream marker. That marker is sent by the std runtime's `cleanup`, which runs after
`main` returns. Under `panic = "abort"`, which every nife program uses, a panic runs the hook and
then executes a trap, so `cleanup` never runs and the marker is never sent.

So the reader blocks forever on an endpoint whose writer is dead, the watchdog eventually fires,
and the test prints nothing at all. Every byte the program wrote was delivered to that
rendezvous and thrown away.

**The symptom is indistinguishable from a program that never started**, which is exactly how it
was read, for a day, through a bisection that produced a coherent and entirely wrong story.

## Why it is worth a lane

It defeats the one debugging tool a program has. A nife program's only way to say anything is
`println!`, and this makes the failure case the case where that stops working. Every future lane
debugging a crashing `std` program will meet it, and the evidence it presents actively argues for
the wrong conclusion.

The workaround belongs at the wrong layer. `cryptography_exerciser` now installs a panic hook
that prints and calls `process::exit`, which makes `cleanup` run and the marker arrive. That works
and it is eight lines, but it is opt-in, per program, and remembered rather than enforced, which
AGENTS.md's ladder calls rung four.

## The shapes a fix could take

- Let the reader stop when the writer is gone. `drain_sink` already has the thread id at every
  call site. What it lacks is a non-blocking receive, or a pending count by rendezvous id: the hang
  diagnostic in `sched.rs` already reads exactly that count through `Rendezvous::debug_counts`, so
  the information exists and is simply not reachable from a test.
- Send the marker on the way out of a fault. The kernel reaps a faulted thread; whether it can
  or should flush that thread's sink is a capability question rather than an obvious yes.
- Make the PAL's abort send it. `sys::pal::nife`'s abort path could send end-of-stream before
  trapping. Smallest change, and it puts the fix where every program gets it without asking.

## BUGS

- Nothing here is measured. The three shapes above are sketched from reading, and none has been
  tried. The middle one in particular may be wrong for reasons the capability model will supply.
- **It is not only `drain_sink`.** Any reader of a nife program's sink has the same problem; this
  file names the one in the test harness because that is where it was met.

## Index row

This proposal said something else on 2026-09-19.
