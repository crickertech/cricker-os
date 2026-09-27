---
status: NOT-STARTED
raised: 2026-09-03
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 330. Nothing stops the lost prompt returning, because the fault path has no regression gate

Filed 2026-09-03 as an unnumbered proposal by the milestone 247 sweep,
from milestone 235's block; numbered 2026-09-19 by milestone 433 (drain). Premise re-checked 2026-09-19 and
it holds, with one correction to where the sibling assertion lives. No program in
`components/src/` or `fixtures/src/` faults on purpose, and `script/swish-check` spawns none. The
no-thread-killed assertion milestone 235 (a command that faults hangs the prompt) named is real but is not in `script/swish-check`: it is
in `xtask/src/main.rs`, reading the kernel's fault-report text out of the transcript through the same
`KERNEL_FAULT_TOKENS` milestone 230 (`script/shell-check` is red on `main`, on both architectures) introduced (this block read `KERNEL_WRITER_ANCHORS` until 2026-09-19; no such name exists anywhere in the tree, and the constant's own doc comment is the sharpest statement of the property this milestone protects: the fault report is "the only thing it writes after the userspace console has started"). `script/swish-check`'s own `BUGS` still says "a
killed user thread is not itself a failure here" and calls that assertion "the obvious next
ratchet", which stopped being true when milestone 233 landed it; that stale sentence is one of the
class milestone 333 collects.

A lane could start this today. The reasoning that produced `DECISION` is sound as
far as it goes (a new program is a new name, and names are an architect's) and it does not reach a gate,
because a name has never been a blocker in this tree. `design/naming.md`, which DECISIONS §155
made the rule, says it in one sentence: a new crate, program or module *"ships a provisional
name, says so in its report, and expects it to change"*. AGENTS.md says the same thing from the
other side, that `script/names --unratified` is a worklist rather than a wall *"precisely so that an
unratified name never blocks anyone's build"*, and milestone 115 (the names that were ratified, and
the ones that were refused)'s gate takes `unrecorded` as a truthful answer for the same reason. So the lane ships `faulter` or whatever it proposes, marks it
provisional, and calef ratifies or replaces it on the worklist afterwards.

**The token was `DECISION` from 2026-09-03 to 2026-09-19**, carried across unchanged when milestone
433 promoted this from the proposal pile, and it is corrected here rather than quietly because of
which way it was wrong. `DECISION` kept a startable milestone off `script/roadmap --ready` for
sixteen days, which is the cheap half of the failure: nobody stalls on a gate that is too strict,
they just never see the work. The expensive half is the precedent, since a tree where "this needs a
new name" reads as a gate has a naming rule that blocks rather than one that unblocks, which is the
opposite of what that rule was written to do. Nothing else in this block waits on anybody.

**In brief.** Milestone 235 fixed a shell that hangs forever when a spawned command traps. The
evidence it was fixed was a **scaffold**: milestone 233's lane patched `components/src/least_authority_demo.rs` to trap,
watched the prompt hang, and the patch was removed afterwards. So the defect is fixed and the proof
is gone. The work is a program whose job is to fault, wired into `script/swish-check` so a hung
prompt fails a gate, plus teaching milestone 233's no-thread-killed assertion to except that one
program.

## Why this matters

The fix is real and nothing holds it in place. A faulting command reaching the prompt is a property
of three couplings that milestone 235's block describes, any one of which a future change can break
quietly, and the symptom is a hang rather than an error. A hang under a shell test looks like a slow
test, which is the failure mode most likely to be waited out and rerun.

This is also the whole class rather than one command. Every program in the tree that can fault took
this path, so the gate is not protecting one binary; it is protecting the shell's answer to any of
them.

## What it would take

A small program that traps deliberately (a name calef gives), an entry in the shell test that spawns
it and asserts the prompt comes back with a fault reported, and an exception in
`script/swish-check`'s no-thread-killed assertion, which milestone 233 (`login` dies on every boot) added and which this program
exists to violate on purpose. The exception has to be narrow: excepting the assertion generally
would retire the check that found this defect in the first place.

## Where it came from

Milestone 235's `## Follow-on`: *"A regression gate for the fault path. It needs a program that
faults on purpose, which is a new name and therefore calef's, and milestone 233's no-thread-killed
assertion in `script/swish-check` has to learn to except it. The scaffold that proved this milestone
was a patch to `components/src/least_authority_demo.rs` and was removed afterwards, so nothing stops the lost prompt
returning."*

## Index row

Milestone 235 fixed a shell that hangs forever when a spawned command traps, and the evidence it
was fixed was a scaffold: milestone 233's lane patched `components/src/least_authority_demo.rs` to
trap, watched the prompt hang, and the patch was removed afterwards. So the defect is fixed and the
proof is gone. A faulting command reaching the prompt is a property of three couplings any one of
which a future change can break quietly, and the symptom is a hang rather than an error, which under
a shell test looks like a slow test and is the failure most likely to be waited out and rerun. The
work is a program whose job is to fault (a new name, so calef's), an entry in the shell test that
spawns it and asserts the prompt comes back with a fault reported, and a narrow exception in the
killed-thread assertion this program exists to violate on purpose. It protects the shell's answer to
every program in the tree that can fault, not one binary.
