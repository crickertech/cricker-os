---
status: NOT-STARTED
raised: 2026-09-03
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 336. Should the fastpath footprint gate compare against `main` instead of a stored baseline

Filed 2026-09-03 as an unnumbered proposal by the milestone 247 sweep,
from milestone 237 (the cycle-counter grant costs 136 bytes of IPC)'s block; numbered 2026-09-19 by milestone 433 (drain the proposal pile to zero, and keep). Premise re-checked 2026-09-19 and
the build is still owed. DECISIONS §144 (the fastpath footprint gate gets a delta and) answered the question on 2026-09-04 (both shapes, with the
ceiling at 16 KiB on `max(ipc_send_receive, ipc_call_reply) + syscall_entry`) and nothing has been built
against it: `script/fastpath-footprint` still compares against `bench/fastpath-aarch64.txt`,
`bench/fastpath-riscv64.txt` and `bench/fastpath-x86_64.txt` at a 5% tolerance, with no delta against
`main` and no absolute ceiling anywhere in the script. The reasoning below is kept as the record of
what §144 was deciding between.

Answered by DECISIONS §144 on 2026-09-04: both, with the ceiling at 16 KiB. What
remains is building it, and the two designs no longer share no code, because the delta half replaces
the baseline files and the ceiling half is new. The question below is kept for its reasoning.

Was: Gate: DECISION. It was calef's call, and it was the shape of the gate rather than a bug in it.
Both options have real costs and the tree has evidence for each. Nothing should be built until the
question is answered, because the two designs share almost no code.

**In brief.** `script/fastpath-footprint` measures the IPC fastpath's code size against a stored
baseline file, which somebody has to remember to re-record when growth is understood and accepted.
The alternative is to measure against `main` at the time the pull request runs, so the comparison
has no stored state and cannot go stale. Milestone 237 fixed one instance of the stored baseline
going stale and explicitly did not touch the mechanism. The work is to decide which shape the gate
takes, and then to build it.

## Why this matters

The stored baseline has already failed in the way it fails. Milestone 237 records it: two lanes
each measured "within bound" against the same stale baseline and neither re-saved it, so aarch64
headroom fell from 3.9 points to 1.5 with nothing firing. The gate was green throughout. Nothing
in the mechanism can distinguish "this change is small" from "this change is small and the last four
were not."

Leaving it undecided is the worse outcome of the two, because it means the failure recurs and gets
patched per instance. Milestone 237 was that patch. There are two more baselines waiting to do the
same thing.

## The costs on each side, which is the material calef needs

**A stored baseline** is an absolute budget. It answers "how big is the fastpath allowed to be",
which is the question that matters for a claim about this kernel, and it catches slow accumulation
across many small changes because the reference does not move. Its failure mode is the one already
observed: the reference goes stale, growth is absorbed, and re-recording is a manual act nobody
owns.

**Comparing against `main`** is a delta budget. It cannot go stale and needs nobody to remember
anything, which is a full rung up AGENTS.md's ladder. Its failure mode is the inverse and is worse
in the long run: every individual change is within bound and the absolute size grows without limit,
because the reference moves with the tree. It also costs a second build per run, since `main` has to
be built to be measured.

The third shape, which neither the block nor this proposal has priced, is both: a delta check
against `main` for the pull request plus an absolute ceiling that only calef raises. That is more
machinery, and whether the extra check earns its keep is part of the same decision.

## One data point from 2026-09-04, which cuts against `main` alone

PR #716 is the daily toolchain bump: one line of `rust-toolchain.toml`, no kernel source. It failed
the `syscall_entry` check at 10.4% on riscv64 because the newer compiler inlined a device-interrupt
helper into the trap handler (the full account is in the
`a-flat-entry-set-counts-bytes-no-syscall-fetches` proposal, which milestone 433 numbers 368).

**A delta-against-`main` gate would have failed this too, and for a worse reason.** `main` would
have been built under the *old* pin, so the comparison would have attributed the entire 194 bytes to
a pull request whose diff cannot contain them, with no stored file for anyone to inspect. The stored
baseline at least made the reference explicit and stable enough to diff against by hand. This does
not decide the question, and it is one case rather than a pattern; it is offered because it is the
kind of case the delta option is supposed to be immune to and is not.

The same lane's measurements on the other side are in the `unattributed-fastpath-residuals`
proposal, which milestone 433 numbers 361: all
three `ipc_fastpath` baselines are now stale, aarch64 having gone stale again within a week of
milestone 237 re-recording it, which is this proposal's own failure mode recurring on schedule.

## Where it came from

Milestone 237's `## Follow-on`: *"Whether the fastpath footprint gate should compare against `main`
rather than a stored baseline file. It is an architect's call and has costs on both sides. Two lanes
each measured 'within bound' against the same stale baseline and neither re-saved it, so aarch64
headroom fell from 3.9 points to 1.5 with nothing firing. This milestone fixed the instance, not the
mechanism."*

## Index row

`script/fastpath-footprint` measures the IPC fastpath's code size against a stored baseline file,
which somebody has to remember to re-record when growth is understood and accepted, and it has
already failed the way that fails: two lanes each measured "within bound" against the same stale
baseline and neither re-saved it, so aarch64 headroom fell from 3.9 points to 1.5 with nothing
firing. Milestone 237 fixed that instance and explicitly not the mechanism. DECISIONS §144 ruled on
2026-09-04 that the gate gets both shapes, a delta check against `main` that cannot go stale and
replaces the baseline files, plus an absolute ceiling of 16 KiB per architecture that only calef
raises, because they catch different failures: a delta alone lets absolute size grow without limit,
and the gate is named for an absolute property. This is the building, which nobody has done; all
three baselines were stale again within a week of 237 re-recording one.
