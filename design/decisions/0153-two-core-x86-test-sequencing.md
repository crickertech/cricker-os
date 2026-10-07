---
status: DECIDED
raised: 2026-09-17
decided: 2026-09-18
ratified_by: calef
---

# 153. How a two-core x86_64 test earns its place, when two-core x86_64 is not yet trustworthy

calef, 2026-09-18: close milestone 315 (a port revoke that reaches every core) first, then default `NIFE_SMP` to 2.
Not one of the three options as written, because milestone 316 falsified the premise all three rested
on between this file being raised and being answered. The reasoning is in "What changed, and what the
question became" below.

What that means concretely, so no lane has to infer it:

1. Milestone 315 builds the broadcast now. It is not ordered behind `ap_boot`'s SMP bugs, and it
   does not get an opt-in leg of its own. The test it needs already exists and already passes and
   fails for the right reasons; 315 owes the `PortRange::REVOKE` IPI broadcast and nothing else.
2. `NIFE_SMP` stays 1 until 315 lands. Not as a judgment about SMP, which now works at two
   cores, but because the two-core suite is red for a true reason and a red default suite is the
   thing this tree has repeatedly found gets tolerated and then ignored.
3. When 315 lands, flip the default to 2 in the same breath, and the suite going green at two
   cores *is* the verification that the broadcast worked. The flip stops being a decision and
   becomes evidence.

Raised 2026-09-17 by the maintainer while minting milestone 315, after asking
calef the same question three times in one session without it reaching a file. That is the failure
AGENTS.md names outright: *open decisions live in a file, not in a conversation*, and five
accumulated in chat scrollback on 2026-08-04 while the milestone abolishing that medium was being
built. This file is the correction.

## What is being decided

Milestone 315's test needs two cores on `x86_64`, and this project's `x86_64` runs one by
default, on purpose. The decision is how to sequence that.

## Why one core is the default

`helpers/qemu-runner-x86_64.sh` sets `SMP="${NIFE_SMP:-1}"`, and its comment is specific about why.
The original crash that held it at 1 is fixed (a missing cross-core TLB shootdown). Two reasons
remain open, both recorded in `arch::x86_64::ap_boot`'s own `BUGS`:

- #1: AP-bring-up flakiness at three or more cores.
- #3: a boot-core-identity bug that makes `smp::tests::every_secondary_runs_scheduled_work`
  fail about half the time at two.

Separately, `design/roadmap/412-the-uefi-boot-gate-asserts-two-cores-that-do-not-always-start.md`
records `script/test`'s UEFI leg failing its two-core assertion one run in three.

So the substrate a two-core port test would run on is one where an existing SMP test is already
unreliable.

## What changed, and what the question became

Between this file being raised and being answered, milestone 316 fixed half of what it was about.
`ap_boot`'s BUG #3, the boot-core-identity defect that failed
`smp::tests::every_secondary_runs_scheduled_work` about half the time at two cores, is fixed:
`boot_cpu_id` recomputed CPUID leaf 1's initial APIC id on every call, answering *"which core am
I"* where every caller wanted *"which core booted"*. It now reads a boot-time record, the shape
riscv64's `BOOT_HARTID` already had. Measured after: 19 of 19, and 26 of 26 boots reported
`smp: 2 core(s) online`. BUG #1 never fired at two cores; it is a three-or-more bug.

So the substrate is trustworthy at two cores, and two of the three options below lost their stated
costs. Option 1's cost was waiting on unscheduled SMP work, and that work is done. Option 2's cost
was a red ambiguous between "the claim broke" and "the substrate is flaky", and it is not ambiguous:
of 12 two-core runs, 5 were fully green and all seven failures are one assertion,
`a_revoked_holder_faults_on_its_next_port_write`, `left: 2, right: 1`. The revoked holder's `out`
succeeded. That is the revocation window milestone 313's audit accepted and 315 closes, observed by
a test that already existed.

And the premise that 315 would have to write that test was wrong too. This file said 315 would
produce *"the first port test able to observe the window at all"*. It already exists in the suite
and became that observer the moment the substrate under it worked.

**So the real question was never about SMP reliability.** It was: if `NIFE_SMP` defaults to 2, the
suite goes red, correctly, until 315 lands. Do we tolerate that? Three answers were available:
default to 2 and accept an honest red that puts visible pressure on 315; keep 1 with an opt-in leg,
which is lane 316's own recommendation and leaves the observation to whoever remembers to ask; or
close 315 first and let the flip verify it.

calef took the third, and the reason it is better than a scheduling preference is that it converts
a judgment into evidence. "Should we tolerate a red" is a question somebody has to keep answering.
"Ship the fix, then flip the default, and the green is the proof" is answered once by a gate.

## The options as raised, kept because two of them were argued against a premise that moved

**1. Order milestone 315 behind `ap_boot`'s BUGS #1 and #3.** The test lands on a configuration
somebody trusts. A confinement test is only worth having if a red is believed, and a red on a flaky
substrate is not. The cost: 315 waits on SMP work nobody has scheduled, and the one-tick revocation
window stays open in the meantime.

2. Give milestone 315 an opt-in `NIFE_SMP=2` leg, separate from the default suite. The test
exists now, and it can see something nothing else can: the three port tests that exist all run on
one core and cannot observe this window at all. The cost: a test that may be red for reasons
unrelated to what it tests, which is the exact condition under which a test gets muted.

3. Both, ordered. Build the broadcast and the test now behind option 2's leg, and treat option
1's SMP work as what promotes the leg into the default suite. Costs an explicitly two-tier suite,
which this tree does not have today and would have to be willing to keep honest.

## What makes this genuinely hard rather than a scheduling preference

**This tree has spent two days finding confinement tests that could not fail.** Milestone 305 found
`the_page_tables_say_u_mode_cannot_read_the_kernels_memory` passing with the `U`-bit check removed
outright. Milestone 307 found six more whose quotable assertion cannot run. Milestone 313 found
milestone 299's two port tests hanging rather than going red.

Option 2 adds a test to that family's neighborhood: one whose failures are ambiguous between "the
claim broke" and "the substrate is flaky". Option 1 avoids that and pays by leaving the claim
untested for longer, which is the same trade that let the `U`-bit test sit vacuous for eight months.

**Neither option is obviously right, which is why this is calef's and not a lane's.**

## The recommendation as written, superseded

Option 3, weakly held, and the weakness is worth stating rather than hidden. The reason to
prefer it is that the test's value is diagnostic rather than regression-catching on day one: it is
the first thing in this tree that can observe the revocation window at all, and measuring the
window's width would replace milestone 313's reasoned "at most one tick" with a number. The reason
to distrust the recommendation is that a two-tier suite is a thing somebody must keep honest, and
this tree's own evidence is that the lower tier is where tests go quiet.

## What was blocked, and what is unblocked

Milestone 315 was `NOT-STARTED` and gated on this. It is now unblocked and its gate should move
off `DECISION`: a lane can build the broadcast today, and the test that judges it is already in
the suite failing for the right reason.

`NIFE_SMP`'s default is the one thing that stays where it is, and 315's own block should carry the
flip as its closing step rather than leaving it to be remembered.
