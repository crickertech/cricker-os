---
status: NOT-STARTED
raised: 2026-09-19
promoted_from: uefi-test-exits-one-after-a-passing-suite
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 516. `uefi-test` can exit 1 after its own suite has passed, and nobody has taken a rate

*(Number provisional until the merge queue lands it.)* Promoted from the
proposal `uefi-test-exits-one-after-a-passing-suite`, filed 2026-09-19, on calef's instruction of
2026-09-20 to give every proposal on `main` a number. The text below is the proposal's own, unedited
except for this paragraph: the argument is its author's and promotion is not the moment to improve
it. Written by milestone 117 (the stranger test)'s run 6 lane, from the stranger's red `script/test` and the operator's
green re-run.

A lane can start today; it needs the dev Mac and nothing else.

**In brief.** On 2026-09-19 a stranger's `cargo xtask test` passed every leg on all three ISAs and
then failed at the last one: under OVMF the kernel suite printed `test result: ok. 215 passed, 71
skipped` and `uefi-test` reported `qemu exited Some(1), not 3`. A re-run of `cargo xtask uefi-test`
an hour later, same tree, same machine, same Homebrew QEMU 11.1.1, passed. Both runs printed the
same VT-d fault lines, which milestone 215 established are the escape test's own expected faults,
so they are not the cause, and they are the only thing on screen that looks like one.

## What it would take

Take a rate before anything else, the way notes/load-sensitive-assertions.md did for the timer
assertions: twenty runs of `cargo xtask uefi-test`, with the host load average beside each, on an
idle machine and on a loaded one. Then find out what path exits 1 after the harness has printed its
summary (a late fault, a triple fault on the way to `isa-debug-exit`, the firmware's own exit), and
make the failure message say which, since the one a reader gets today invites a QEMU-version theory
the evidence does not support. Say at the failure that the VT-d lines are expected.

Reversible, and on the customer path only by way of trust: a gate that goes red for a reason nobody
can name costs every contributor who meets it an hour, which is what it cost run 6.

## Index row

On 2026-09-19 a stranger's `cargo xtask test` passed every leg on all three ISAs and then failed at
the last one: under OVMF the kernel suite printed `test result: ok.
