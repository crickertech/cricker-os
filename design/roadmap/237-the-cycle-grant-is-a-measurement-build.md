---
status: BUILT
raised: 2026-09-03
built: 2026-09-03
---
# 237. The cycle-counter grant costs 136 bytes of IPC fastpath for an instrument nothing can request

Minted the same day by calef, from reading why
`script/fastpath-footprint`'s aarch64 headroom had shrunk. *(Number provisional until the merge
queue lands it.)*

It was minted with no gate and needed none: the measurement was already taken, the pattern
(`--features soak`) already existed in this tree, and nothing external blocked it.

In brief. Milestone 229 (build the cycle-counter grant DECISIONS 139 decided) put a per-thread
grant on the context switch, which is where it has to be: `PMUSERENR_EL0` is one register shared by
every thread on a core, so the only moment a per-thread grant can mean anything is when a thread
starts running. The read is the enforcement, not overhead around it.

Measured on aarch64, per symbol, against the commit the baseline was saved at:

| symbol | baseline | now | delta |
|---|---|---|---|
| `sched::schedule` | 1244 | 1380 | **+136** |
| `sched::finish_switch` | 824 | 880 | **+56** |
| `sched::ipc_recv` | 1320 | 1324 | +4 |
| `sched::ipc_send` | 952 | 956 | +4 |
| everything else | | | 0 |
| **total** | **5788** | **5988** | **+200** |

**Correction, and it is the useful kind: those 200 bytes are not all this milestone's.** Building
the kernel both ways, which is a thing that could not be done until the feature existed, splits them
exactly. With `--features cycle_counter_grant` off, `schedule` returns to 1244 and every other row
above stays where it is: `finish_switch` is still 880, `ipc_recv` still 1324, `ipc_send` still 956.
So the grant's whole cost is 136 bytes in `sched::schedule`, and the other 64 belong to milestone
231 (capability-slot high-water mark), which is also what 231's own recorded figure of 5852 says.

    5788 (baseline)  +  64 (milestone 231)  +  136 (milestone 229)  =  5988

The block was minted reading 192, which is `schedule` plus `finish_switch`, and it was the best
reading available from one binary. It is worth leaving the original table standing above rather than
editing it away: the reason the attribution was wrong is the same mechanism failure this milestone
is about. With no build that lacks the instrument, "what did 229 cost" can only be answered by
diffing against a stored number that two milestones had moved, and a stored number cannot say which
of them moved which byte. The feature is now the thing that can answer it.

The other two architectures, measured the same way:

| ISA | feature off | feature on | the grant's cost |
|---|---|---|---|
| aarch64 | 5852 | 5988 | 136 |
| riscv64 | 5132 | 5148 | 16 |
| x86_64 | 6687 | 6687 | 0 |

x86_64's zero is DECISIONS 139 part 3 showing up in the codegen rather than an omission: `rdtsc` is
ambient there by decision, its `set_cycle_counter_grant` is an empty function, and LLVM drops the
field read that feeds it along with the call.

**And its only consumer is a benchmark on a machine that has never booted nife.** DECISIONS 139's
own accounting: a user-level cycle read is what milestone 25 (cross-OS performance comparison) needs
to reproduce seL4's published 413 and 426 cycle figures on argon, and nothing else in the tree
wants it. The kernel may read `PMCCNTR_EL0` at EL1 with no grant at all, and milestone 168 (a
multi-tasking workload benchmark) is a long-loop measurement the generic timer already serves. Since
229's ABI was deliberately deferred, no program can request the grant today: the only writer is a
`#[cfg(test)]` helper.

## What to do, and why it is not deletion

calef's question was whether the code can be switched off once the benchmark is taken. It can, and
doing so would cost two things:

- The number would stop being reproducible. A cycle figure published against a competitor's,
  taken once with an instrument that was then removed, is a claim nobody can re-check, including us
  the next time IPC changes. `notes/register-of-measures.md` opens with exactly that complaint.
- It would quietly reverse DECISIONS 139, which answered who may read the counter and by what
  authority. Deleting the enforcement returns the answer to "closed for everyone", which is not the
  answer that was given.

So: a measurement build, the way `--features soak` is. Production never carries it; anyone can
rebuild and re-measure at any time.

- The production fastpath returns to 5852, keeping milestone 231's slot counter, which has a real
  consumer and prints on every boot. Measured, not assumed: 5852 exactly.
- The instrument stays reproducible rather than being spent once.
- DECISIONS 139's authority model still stands: the grant is how it works when built.

## What was built

**The feature is `cycle_counter_grant`, declared in `kernel/Cargo.toml`, and the name is
provisional** like everything a lane mints. It is the field it builds
(`Thread::cycle_counter_grant`) rather than a new word for the same thing; names are an architect's.

**The gate on every piece is `any(test, feature = "cycle_counter_grant")`, not the feature alone**,
and that is the answer to this block's first `BUGS` entry. Milestone 229's proofs are the only
things in the tree that exercise this path (the embryo refusal in `sched::tests`, the EL0 round trip
and its ungranted-faults half in `user::tests`, the register-field assertions in each architecture's
`timer::tests`), so folding `test` into the predicate makes every `script/test` run a keep-alive on
all three architectures rather than a CI job that only builds it. The release kernel
`script/fastpath-footprint` measures has no `test` cfg, so production still pays nothing. Confirmed:
all four suites green with the grant's tests reported `ok`, the negative half faulting with the real
`esr 0x6230e51b` on aarch64 and `scause 0x2` on riscv64.

`script/lint` clippies the feature on all three architectures as a second keep-alive, on the two-ISA
loop `soak` is in plus a line of its own for x86_64. That third line needs saying because it is a
second exception to a paragraph that already had one: the loop's targets are aarch64 and riscv64
because seven of the nine build-mode features select a boot path x86_64 has not written, and this
feature selects no boot path at all, which is why it can be linted there when `shell` and `soak`
cannot.

Gated: `Thread::cycle_counter_grant` and its four initializers, the read out of the locked block
beside `ttbr0` in `sched::schedule`, the `arch::timer::set_cycle_counter_grant` call beside
`switch_user_root`, `sched::grant_cycle_counter`, and all three architectures'
`set_cycle_counter_grant` / `cycle_counter_grantable`.

**Deliberately not gated:** milestone 228 (the cycle counters are closed by assumption)'s write of
the closed default in each `timer::init`. Closing what we claim is closed is right whether or not
anyone can be granted an exception, and 228 landed for reasons independent of this one, so a
production kernel still writes `PMUSERENR_EL0` to zero on every core and `scounteren` to `TM` alone
on every hart.

The switch site keeps its shape through two pairs of one-line functions rather than a fourth
`#[cfg]` on a sixty-line block, because the value has to leave the lock through a tuple and `#[cfg]`
is not allowed on a tuple element. Without the feature `sched::cycle_counter_grant_of` is the
constant `false` and `install_cycle_counter_grant` is empty, so the optimizer folds the whole thing
out. A zero-sized `()` in the tuple was the first shape tried and `clippy::let_unit_value` refused
it, which turned out to be an improvement: a `bool` that is always `false` reads at the switch site
as what it is.

## The comparability caveat, and where a reader meets it

A benchmark build has 136 more bytes in `sched::schedule` than production, so a cycle figure taken
with the instrument on is slightly pessimistic about nife. That is recorded in `notes/benchmarks.md`,
in the "why that is not a win" list beside the seL4 calibration, which is where milestone 25
(cross-OS performance comparison) already keeps its comparison and its caveats; and in `notes/abi.md`
beside the mechanism itself. It is milestone 221 (a soak that crosses cores)'s rule one instrument
over: compare a soak number only with another soak number.

**It reads as a weakness and is not.** seL4's published 413 and 426 need `sel4bench` to read
`PMCCNTR_EL0` from user level, which on Arm needs `KernelArmExportPMUUser`, and seL4's own
configuration reference describes that option as *"Grant user access to the performance monitoring
unit. While useful for benchmarking, this option opens the possibility of timing channels"*, with a
default value of `OFF` (docs.sel4.systems/projects/sel4/configurations.html, read 2026-09-03). Both
sides measure in a benchmarking build, which is like for like.

The comparability question answers itself, and in our favour. A gated build is not the production
binary, so its numbers carry a caveat, which milestone 221 (the soak never crosses cores) already
records for soak builds. But seL4's published figures come from `KernelArmExportPMUUser`, a
configuration seL4 **does not verify and does not ship on by default**. Both sides would be measuring
in a benchmarking build, which is like-for-like and more honest than comparing our production kernel
against their benchmark one.

The residual cost is worth recording rather than hiding: a benchmark build has 136 more bytes in
`schedule` than production, so the number it produces is slightly pessimistic about nife.
Understating ourselves is the right direction to err in a comparison we intend to publish.

*(This paragraph and the title both read 192 as minted, which was `schedule` plus `finish_switch`.
Building the kernel both ways attributed `finish_switch`'s share to milestone 231 instead; the
correction and its arithmetic are above. Roadmap titles are drafts, so the number in this one was
fixed rather than left standing wrong where a reader meets it first.)*

## The mechanism failure this came out of, which outlives the fix

Two lanes each measured "within bound" against the same stale baseline, and neither re-saved it.
Milestone 231 took the aarch64 figure from 5788 to 5852, milestone 229 from 5852 to 5988. Both were
honest, both were under the 5% bound, and the bound is measured against a stored number, so the
growth accumulated with nothing firing. Headroom went from 3.9 points to 1.5 without anyone deciding
to spend it.

**The baseline must not be re-saved to absorb growth nobody attributed.** `bench/fastpath-aarch64.txt`
already carries one such re-record (`ff38e4a2`, "re-record the fastpath baseline that PR #316 moved
and did not record"), and its own header asks for the opposite: *"Updating this file is a statement
that a footprint change is intended and understood; do it in the commit that causes it."*

## BUGS

- A gated path rots unless something builds it. Answered above by folding `test` into the
  predicate, so `script/test` compiles and *runs* it on all three architectures, plus `script/lint`
  compiling the feature without `test`. That is a stronger keep-alive than `soak`'s, which is built
  and not run; it is not free, because the configuration a production kernel actually ships
  (`not(test)`, feature off) is now the one nothing runs, and it is the one whose correctness is
  "this code is absent".
- **This block does not price milestone 74's arrival.** Unchanged, and cheaper than it looked: the
  fastpath churn is 136 bytes in one function, and when 74's aarch64 half lands it turns the feature
  on rather than reverting anything.
- Only the aarch64 baseline was re-recorded, and only to 5852. That is what this commit
  deliberately leaves behind and the 64 bytes it is above 5788 are attributed above to milestone 231.
  riscv64 sits at 5132 against a 5106 baseline and x86_64 at 6687 against 6639, and those residuals
  are not attributed to anything, so re-saving them would be exactly the absorb-the-growth move
  this milestone exists to refuse. Whoever attributes them should re-record them in the commit that
  does it.
- Nothing here fixes the accumulating-baseline problem, only this instance of it. Whether the
  gate should compare against `main` rather than a stored file is a separate question with its own
  costs.

## Follow-on

- **Milestone 74.** Cycle counters on both ISAs. When its aarch64 half lands it turns this feature
  on rather than reverting anything, and the churn it meets is 136 bytes in one function.
- **Recorded.** Folding `test` into the gate makes `script/test` a keep-alive on all three
  architectures, at the cost that the configuration a production kernel actually ships
  (`not(test)`, feature off) is now the one nothing runs, and it is the one whose correctness is
  that the code is absent. `design/roadmap/237-the-cycle-grant-is-a-measurement-build.md`.
- **Recorded.** A benchmark build carries 136 more bytes in `sched::schedule` than production, so a
  cycle figure taken with the instrument on is slightly pessimistic about nife. That caveat sits
  beside the seL4 calibration in `notes/benchmarks.md`, which is where milestone 25 keeps its
  comparison, and beside the mechanism in `notes/abi.md`.
- **Recorded.** The feature name `cycle_counter_grant` is provisional, like everything a lane mints;
  names are an architect's. It is the field it builds rather than a new word for the same thing.
  `kernel/Cargo.toml`.
- **Milestone 361.** `design/roadmap/361-unattributed-fastpath-residuals.md`, attribute the riscv64
  and x86_64 fastpath residuals, then re-record those baselines in the commit that does it. riscv64
  sat at 5132 against a 5106 baseline and x86_64 at 6687 against 6639, neither gap bisected to a
  milestone, so re-saving them then would have been the absorb-the-growth move this block exists to
  refuse. Only aarch64 was re-recorded here. That block is SUPERSEDED: milestone 188's phases 1 to 3
  changed what the gate measures and re-recorded all three baselines on 2026-09-04, so the numbers
  above no longer exist to bisect against.
- **Milestone 336.** `design/roadmap/336-fastpath-footprint-against-main.md`, whether the fastpath
  footprint gate should compare against `main` rather than a stored baseline file. DECISIONS §144
  answered it on 2026-09-04 (both, with a 16 KiB ceiling) and the building is what remains. Two
  lanes each measured "within bound" against the same stale
  baseline and neither re-saved it, so aarch64 headroom fell from 3.9 points to 1.5 with nothing
  firing. This milestone fixed the instance, not the mechanism.

## Index row

`--features cycle_counter_grant`, gated as `any(test, feature = ...)` so `script/test` keeps
compiling and running 229's proofs; production aarch64 `ipc_fastpath` back to 5852, measured, and
the +56 the mint attributed here turned out to be milestone 231's
