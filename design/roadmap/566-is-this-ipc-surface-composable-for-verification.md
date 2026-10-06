---
status: NOT-STARTED
raised: 2026-09-21
promoted_from: is-this-ipc-surface-composable-for-verification
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 566. Is this IPC surface composable for verification, or only verifiable piecewise

The number is **provisional**: the integrator mints it at merge. Promoted from the proposal `is-this-ipc-surface-composable-for-verification` on 2026-09-22, filed 2026-09-21. Raised by the `incremental-path` lane, from a sentence in the paper
it was reading that nobody here has answered.

Everything it needs is in this tree and in two notes already written.

## The sentence

Li, Miller, Zhuo, Chen, Howell and Anderson, arguing for verifying a kernel module by module
(HotOS '21), say of their own final step: *"it is almost impossible to know the right interfaces for
composed verification before anything is built."*

This tree is building interfaces now and proving things about them one crate at a time. 178 Kani
harnesses across 26 packages, each proving something about its own package. Nothing anywhere asks
whether the properties compose: whether "the capability layer is sound" and "the IPC path is sound"
together say anything at all about a program that uses both.

## Why it is a question and not a worry

Piecewise verification is the honest default and this project chose it deliberately. The
alternative, designing every interface for composed proof up front, is exactly what the paper says
is almost impossible before anything is built, and it is what seL4's 20:1 proof-to-code ratio bought.

But the two positions have different consequences and this tree has never said which one it is in:

- If the surface is composable, the harnesses are a foundation and the work is to connect them.
- If it is **not**, then the proofs are a set of local facts, and `design/fatal-risks/README.md`'s risk 2
  (that the proofs prove trivia) is stronger than its current entry admits, because "no harness has
  caught a defect after the day it was written" would be joined by "and they do not add up either."

Either answer is worth having. The second is uncomfortable, which is a reason to ask rather than
a reason not to.

## What would answer it

Pick one path a program actually takes, a `SEND_CAP` that crosses the capability layer, the scheduler
and the IPC path, and ask what the existing harnesses on those three collectively guarantee about it.
Not by proving it, which is the expensive thing, but by writing down the gap: what each harness
assumes about its neighbors, and whether any neighbor establishes it.

`notes/verus.md` already did the adjacent comparison and `notes/proof-retrospective.md` already asked
the adjacent question (did the proofs catch the bugs). This is the third of that family, and its
output is a note rather than a proof.

## BUGS

- The answer will be qualitative and the temptation will be to make it a number. There is no
  metric for composability, and inventing one here would be the kind of gate this tree refuses.

## Index row

178 harnesses each prove something about one package, and nothing asks whether the properties compose
into a statement about a path a program takes; the paper this tree read says nobody can know that in
advance, which makes writing down the gap the cheap half.
