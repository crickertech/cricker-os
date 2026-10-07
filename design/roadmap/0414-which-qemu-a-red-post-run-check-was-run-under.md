---
status: NOT-STARTED
raised: 2026-09-14
promoted_from: which-qemu-a-red-post-run-check-was-run-under
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 414. A red post-run check does not say which emulator produced it

Promoted from the proposal `which-qemu-a-red-post-run-check-was-run-under`,
filed 2026-09-14 by milestone 288 while establishing that a red kernel leg was the environment
rather than its change, on a box that happens to have two QEMUs installed. *(Number provisional
until the merge queue lands it.)*

A lane can close this. It is a line of output, and the question underneath it is
where the version is read rather than whether to print it.

**Premise re-checked 2026-09-19 and still true.** Nothing in `xtask/src/main.rs` prints an emulator
version beside a post-run check or in the per-architecture banner; the only place the tree ties a
number to a QEMU is `.qemu-version` and `script/qemu-check`, which the icount baselines cite and the
post-run checks do not. `notes/load-sensitive-assertions.md` still carries no `BUGS` entry saying
the post-run set is emulator-dependent, so the smaller version of the fix is also still open. *(Corrected
2026-09-24: #1211 added that entry to the main page's `BUGS` in
[notes/load-sensitive-assertions.md](../../notes/load-sensitive-assertions.md), naming the 8.2.2 and
11.0.2 split and this milestone as the fix. The smaller version is done; the version line is not.)*

## What was measured

One tree, one kernel binary, one `script/test --arch aarch64`, two emulators on PATH in turn:

| Emulator | `a_keystroke_from_a_virtio_keyboard_becomes_a_terminal_byte` | the three `scanout` checks |
|---|---|---|
| apt's 8.2.2 | **FAILS**: `sendkey` never reaches the device | **pass**, pixel for pixel |
| the pinned 11.0.2 | **passes** | **FAIL**: *"no screendump was ever taken (did QEMU get a monitor?)"* |

`inbound` and `multicast` fail under both. The two versions fail disjoint sets of host-side
checks, and neither set is empty, so "the host-side referees fail here because the box is headless"
is true and is not enough to identify anything.

## Why it matters more than it looks

The post-run checks are this project's only evidence for the things a guest cannot witness about
itself: that pixels reached the *device*, that a keystroke crossed the monitor socket, that a host
process could connect into the guest. `script/test`'s own output already goes to some length to
make a red one readable (the `inbound` check prints the attempt histogram and tells the reader to
read the timestamps first). The emulator version is the one input to those checks that changes the
answer and is not in the report.

The concrete cost, paid twice in two days: a lane sees a red post-run check, cannot tell an
environment failure from its own regression, and spends a control run finding out. The block for the
`script/bootstrap` milestone landing with [#847](https://github.com/nifeos/nife/pull/847)
records three `scanout` failures on this box; milestone 288 saw them pass there and a different
check fail. Both accounts are correct and they read as contradicting each other.

## What would close it

`script/test` already resolves the emulator it runs. Print its version beside each post-run check
result, or once in the per-architecture banner, so a red line carries the one fact needed to
interpret it. `script/qemu-check` knows how to read a version and `helpers/qemu-path.sh` knows which
binary was chosen, so nothing new has to learn how; the question is which of the two is the right
place, and whether the banner is enough or each failing check should carry it.

**A smaller version of the same fix, if the banner is judged enough**: say in the `BUGS` of
notes/load-sensitive-assertions.md that the post-run set is emulator-dependent, so at least a reader
who goes looking finds the table above rather than deducing it a third time.

## Index row

The post-run checks are this project's only evidence for what a guest cannot witness about itself,
that pixels reached the device, that a keystroke crossed the monitor socket, that a host process
could connect into the guest, and a red one does not say which emulator produced it. Measured on one
tree and one kernel binary with two QEMUs on PATH in turn, the versions fail disjoint sets of those
checks and neither set is empty, so "the referees fail here because the box is headless" is true and
identifies nothing. The cost was paid twice in two days: two correct accounts of the same box read
as contradicting each other, and a lane spends a control run distinguishing an environment failure
from its own regression. `script/test` already resolves the emulator it runs, so the work is where
to print the version rather than whether to.
