---
status: SUPERSEDED
raised: 2026-09-03
superseded_by: 420
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 362. The x86_64 runner has no RNG, so four of six NTP tests skip there

Superseded 2026-09-19, by the proposal `the-rest-of-the-x86-64-fixture-set`, which
milestone 433 numbers 420 and which names the RNG as one of its six devices. Filed as a proposal on
2026-09-03 by the milestone 247 sweep, from milestone 176's block. Checked on 2026-09-19 and the
underlying gap is real: `helpers/qemu-runner-x86_64.sh` still attaches no RNG and says so in its own
header ("no NIC, no GPU, no RNG"). This file's own instruction is what disposes of it, and it was
written knowing this would happen: *"whoever promotes either should merge the two rather than run
two lanes at the same fixture file"*. The larger lane is now numbered, so this one is the duplicate
rather than the placeholder.

It is a line in `helpers/qemu-runner-x86_64.sh` plus its wiring, and both the device
and the client exist on the other architectures already.

**In brief.** Attach a virtio-rng function to the x86_64 test runner and wire it through, so the
NTP client has a nonce source on that architecture. Without one, four of the six tests in
`ntp_tests.rs` skip on x86_64.

**This is one item inside a larger piece of work, and it should be folded into it.** Milestone 176's
own bullet says so: milestone 215 proposes attaching the rest of the x86_64 test fixtures (the
RedoxFS image, the GPT and blank disks, the NIC, the GPU, the keyboard and the RNG) as one lane, and
the RNG is one line of it. That larger lane now has a proposal of its own,
`design/roadmap/364-x86-64-test-fixtures.md`, written by the same sweep. This file exists so
the item is not lost while that larger proposal is written; whoever promotes either should merge the
two rather than run two lanes at the same fixture file.

## Why this matters

A skipped test reads as a passing suite. Parity is a gate in this project, not an aspiration, and
the specific failure it guards against is a feature that works on one instruction set architecture
and silently not another. Four skipped NTP tests on x86_64 is that state today, and the reason is
not that the NTP client is wrong on x86_64, it is that the runner never gave it entropy. Nobody can
tell those two apart from the outside, which is what makes the gap worth closing rather than
recording.

The cost is small and the payoff is proportional: one device on a command line, and four tests
change from skipped to proving something.

## Where it came from

Milestone 176's Follow-on: *"Attach a virtio-rng function to the x86_64 test runner and wire it, so
the NTP client has a nonce source there; four of `ntp_tests.rs`'s six tests skip on x86_64 without
one. Milestone 215's block proposes this as one item in a larger x86_64 fixture lane, so take it
there rather than as a second piece of work."*

Milestone 215's Follow-on names the larger lane: *"Attach the rest of the x86_64 test fixtures now
that a function's interrupt works ... each a line in `helpers/qemu-runner-x86_64.sh` plus its
wiring, starting with making the FS server's disk lookup transport-blind. The measure is the 36
tests taking a 'no RedoxFS disk attached' arm."*

## Index row

Four of the six tests in `ntp_tests.rs` skip on x86_64 because the runner gives the NTP client no
nonce source, which is one `-device virtio-rng-pci` line plus its wiring. Filed as a placeholder by
the milestone 247 sweep so the item would not be lost while the larger x86_64 fixture lane was
written up, with the instruction that whoever promoted either should merge the two. That larger lane
now exists and names the RNG among its six devices, so this is the duplicate. A skipped test reads
as a passing suite, and nobody outside can tell "the NTP client works on x86_64" from "the runner
never gave it entropy".
