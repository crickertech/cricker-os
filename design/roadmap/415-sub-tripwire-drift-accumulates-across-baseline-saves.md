---
status: PARTIAL
raised: 2026-09-15
promoted_from: sub-tripwire-drift-accumulates-across-baseline-saves
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 415. Sub-tripwire drift accumulates across baseline saves, and one architecture has no gate at all

Items 1 and 2 are built and item 4 is refused. Item 3 is ruled and being
built by the lane that recorded the ruling. Item 1 landed 2026-09-15 as commit `ba99c83`, item 2 on
2026-09-23 in PR #1126 under milestone 302. Promoted from the proposal
`sub-tripwire-drift-accumulates-across-baseline-saves`, filed 2026-09-15 by the baseline-audit lane,
which calef asked for after PR #886 found a regression that had hidden under the 10% threshold.
*(Number provisional until the merge queue lands it.)*

§190 is decided. calef ruled item 3 on 2026-09-26 as "3b", a weekly report of
cumulative drift since a fixed anchor, not a gate; the ruling is quoted at the top of §190. Item 2
had been ruled under milestone 302 on 2026-09-16, before this block's gate was written, so nothing
here waits on anyone.

The gate's history, kept because it explains the old token. It was `DECISION §190`, written up
2026-09-19 by milestone 435's slice-c lane because it named no section. An earlier clause read
*"the first item is a one-line CI change that is owed already and needs nobody's permission"*, and
item 1 landed on 2026-09-15 as `ba99c83`, which left the token more purely `DECISION`, not less.
The live argument that it was still too strong for item 2 turned out to be moot: milestone 302's
ruling already covered item 2, and the lane that raised §190 had not found it.

Premise re-checked 2026-09-26. Item 1: `script/ci-build`'s bench entry still reads
`script/bench --check && script/bench --riscv --check && script/bench --x86 --check`. Item 2:
`cargo xtask bench --save` refuses to run without `--why` and writes one `# why:` line per reason
(`xtask/src/bench.rs`, `save_reasons` and `baseline_header`); the three floors carry them since
2026-09-23. Item 3: `bench/` still holds one baseline per architecture and nothing reports drift
against an anchor. Item 4 stays refused for the reason milestone 25 already established.

## In brief

`cargo xtask bench --check` fails at >10% drift against the last saved baseline, and `--save`
rewrites that baseline. So N successive sub-threshold steps accumulate and the gate never fires.
The audit in [notes/benchmarks.md](../../notes/benchmarks.md) walked every `--save` event in the
history of all three baseline files, from git alone with no emulation, and the accumulation is real
and measurable:

| | cumulative drift | largest single step | times the gate fired |
|---|---:|---:|---:|
| riscv64 `ctx_switch` | +10.78% | +6.14% | 0 |
| aarch64 `yield_switch` | +9.16% | +6.49% | 0 |
| riscv64 `ipc_rtt` | +9.25% | +4.34% | 0 |
| aarch64 `ipc_rtt` | +8.73% | +4.95% | 0 |
| x86_64 `yield_switch` | +9.94% | +9.94% | 0 |

riscv64's `ctx_switch` is past the threshold the gate enforces, reached in steps none of which came
close to it. `coremark`, pure compute with no context switches, is flat to four decimal places
across every save on all three architectures, which is what says the rest is the kernel's switch and
IPC paths rather than measurement noise.

Most of that accumulation is honest, disclosed in the commit that caused it, and a good part of
it is whole-crate codegen churn the instrument cannot separate from real cost. The problem is not
that the drift exists. It is that nothing in the tree can tell the honest part from the rest, and
two saves on 2026-09-15 prove it.

## The live instance, which is not hypothetical

`44890a8a` re-saved the x86_64 baseline and attributed its +5 to +8% to the toolchain:
*"It also folds in the same nightly-2026-09-15 drift the other two carry."* Milestone 300 (decompose
the icount baseline drift) then measured that toolchain term across exactly those two nightlies and
found it ~0, byte-identical counts on the same code. PR #886 names the real cause, a const-`false`
element still threaded through the shared context-switch tuple that the debug build does not fold,
and recovers ~5.9% on x86_64 by deleting it.

So the x86_64 floor carries a removable regression, blessed into the baseline on a stated cause that
measures zero, and `--check` passed the whole time because 5.9% is under 10%. `85edb1ed` did the same
thing on aarch64 and riscv64 the same day, classifying the +6.5% as intended feature cost where #886
shows 91 to 93% of it is removable.

This is the move milestone 237 (the cycle-counter grant costs 136 bytes of IPC fastpath) exists to
refuse, one instrument over: re-saving because the number moved, which launders growth into the new
normal and retires the gate that was supposed to catch it.

## What to do, cheapest first

### Item 1: Pull the x86_64 leg into CI. One line, owed already, needs no decision.

`script/ci-build`'s bench entry is `script/bench --check && script/bench --riscv --check`. The third
tripwire is built, its baseline is committed, and nothing pulls it; `ci.yml` already carries a
`BUGS` note saying so. The audit shows what that costs: x86_64 has had exactly two saves ever, and
the window between them is 1,526 commits. On the two gated architectures a gross regression
eventually forces a save, so the record has granularity. On the third nothing forces one, which is
why its single step arrived at +9.94% wearing a false attribution.

Cost: one line. It may fail the first time it runs, which is why `ci.yml`'s note says it wants its
own commit rather than riding along with something else.

Done, on 2026-09-15, in its own commit. `ba99c83` ("ci: gate the x86_64 icount baseline, which
nothing ever ran") made the bench entry
`script/bench --check && script/bench --riscv --check && script/bench --x86 --check`, on the
argument this section makes and with the same 1,526-commit window as its evidence. It was safe to
gate at that point because PR #886 had already recovered the ~5.9% x86_64 regression and re-saved
that baseline, so the floor it checks against is the recovered one. That is what makes this block
`PARTIAL` rather than `NOT-STARTED`.

### Item 2: Make a save record its own attribution, beside the number. Recommended.

Require `--save` to carry a reason and write it into the file next to the rows that moved, refusing
to write without one. Today the only record of why a number moved is the commit message, which is
rung four of AGENTS.md's ladder: read once, by one person, on the day it is written. Both 2026-09-15
mis-classifications are in commit messages that nobody re-read until a lane went looking six weeks
later.

Writing the attribution into the baseline file moves it to rung three, beside the thing a reader
meets, which is milestone 115's shape. It also makes the wrong state visible rather than
unrepresentable, and that is the honest limit: a lane can still write a false reason, as
`44890a8a` did. What changes is that the false reason is then in the file the next reader opens,
where the audit above would have found it in a grep instead of a bisect.

Cost: roughly the `--save` writer plus a flag, and the rows it already knows have changed. Small.

### Item 3: A cumulative check against a fixed historical anchor. Real, and more expensive than it looks.

Keep a second per-architecture file holding a historical anchor and fail when today's number drifts
more than some bound from it, in addition to the last-floor check. This is the mechanism that
directly answers the structural hole, and the audit is the evidence that it would fire.

The cost is not the code, which is a second file and a second comparison. It is that the anchor
goes stale for correct reasons. Milestone 139 (drive the unsafe count down) is a decided feature
with a real cost at the switch. `spawn_el0` legitimately fell 32.7% on both ISAs when
`b918337b` bounded a walk by occupancy. An anchor with no ledger of intended deltas would fire on
both, and a gate that fires on correct work is the shape §61 and milestone 78 already dropped
checks for. So this option is really "an anchor plus a per-benchmark ledger of what was intended",
and the ledger is the expensive half. Item 2 is most of that ledger, which is why it comes first.

### Item 4: Tighten the threshold. Refused, and the tree already knows why.

Milestone 25 (cross-OS performance comparison) demoted `--check` from a 2% gate to the coarse 10%
tripwire deliberately, because adding unrelated live code moves untouched benchmarks several percent
non-uniformly through whole-crate inlining decisions. The audit re-confirms it: `9890eb02` moved
every kernel-side IPC row by 4 to 8.5% by adding one benchmark. Tightening the threshold rebuilds
the false-positive problem that demotion was written to escape, and it would not have caught either
2026-09-15 step, both of which were real costs honestly measured and wrongly explained.

A per-benchmark drift budget consumed across saves is item 3 with extra bookkeeping: it needs the
same anchor and the same intended-delta ledger, and adds a consumption rule on top. Not recommended
separately.

## Recommendation

Do 1 now, as its own commit. Do 2 next, because it is small and because it is the ledger that
item 3 would otherwise have to invent. Hold 3 until 2 has been in the tree long enough to say
whether the attributions it collects are good enough to gate on. Do not do 4.

The reversibility test says the same thing: 1 and 2 are undoable in an afternoon and nobody outside
this tree has acted on them, while 3 writes a second committed floor that every future measurement is
compared against, which is a fact that other work starts depending on.

## How this bears on the toolchain-bump decision, which has since been answered

Corrected 2026-09-19. This section was written against calef's then-open decision 2 from PR #883
(`icount-baselines-drift-after-a-toolchain-bump.md`, since promoted as milestone 577 (the icount
baselines predate the pinned nightly) and superseded in the same act): should a toolchain bump
re-baseline in the
same pull request, or fail loudly? He ruled on 2026-09-16 and it is milestone 302, a baseline
that records what it was saved against and fails loudly when it is stale, which is the "fail
loudly" half. The audit below remains the evidence for that ruling rather than an argument toward
it.

That proposal's premise is that a new nightly's codegen invalidates the baselines. Milestone 300
measured it: across `nightly-2026-08-27` and `nightly-2026-09-15` the toolchain term is ~0, and
the QEMU upgrade term is also ~0. On that evidence, a bump-triggered automatic re-baseline would
have written a new floor for a cause that measured zero, and in doing so would have absorbed
milestone 139's regression under a toolchain label, which is exactly what `44890a8a` did by hand.
That is an argument for the "fail loudly" half of the proposal's own option and against the
"re-baseline in the same pull request" half, and it is a measurement rather than a preference. It is
the half calef took.

## BUGS

- The cumulative numbers are anchored, and the anchor is a judgment. aarch64 is anchored at
  `74431429` (2026-07-30) rather than at its first save, because `60e75545` pinned the bench to one
  hart after finding the `-smp 4` counter was fiction, which re-means every earlier number. A
  different anchor gives different cumulative figures. The per-save tables are anchor-free and are
  the checkable record.
- This audit ran no benchmarks. Every figure is arithmetic over committed text, which is what
  made it cheap and is also its limit: it says what each save *recorded*, not what the tree measured
  between saves. A regression that appeared and was fixed inside one window is invisible here.
- Item 2 cannot make an attribution true. It moves a claim from a commit message to the file, so
  a reader meets it. `44890a8a` would still have written its false toolchain attribution; it would
  just have been findable.

## Follow-on

- **Done.** Item 1, the x86_64 leg in CI, landed 2026-09-15 as commit `ba99c83` ("ci: gate the
  x86_64 icount baseline, which nothing ever ran"), in its own commit as the section asked, against
  the floor PR #886 had already recovered.
- **Outstanding.** Item 2, a save that records its own attribution beside the number. Checked
  2026-09-19: `cargo xtask bench --save` still writes rows with no reason and refuses nothing, so
  the ledger item 3 would otherwise have to invent does not exist yet.
- **Outstanding.** Item 3, a cumulative check against a fixed historical anchor. Checked 2026-09-19:
  there is one baseline file per architecture and no second anchor file, and the recommendation is
  still to hold this until item 2 has collected attributions worth gating on.
- **Refused.** Item 4, tightening the 10% threshold. Milestone 25 demoted `--check` from a 2% gate
  deliberately, the audit re-confirmed why, and it would not have caught either 2026-09-15 step.
- **Milestone 302.** The toolchain-bump question this block's last section was written against.
  calef ruled on 2026-09-16 and the work is milestone 302, a baseline that records what it was saved
  against and fails loudly when it is stale; the section is corrected to say so.

## Index row

`cargo xtask bench --check` fails at more than 10% drift against the last saved baseline and
`--save` rewrites that baseline, so successive sub-threshold steps accumulate and the gate never
fires: riscv64's `ctx_switch` is +10.78% cumulative in steps that never reached +6.2%, and the
gate has fired zero times on any of the five rows audited, while `coremark` is flat to four decimal
places across every save on all three architectures. Two saves on 2026-09-15 blessed a removable
regression into the floor on a toolchain attribution that milestone 300 later measured at ~0. Item
1, pulling the ungated x86_64 leg into CI, landed the same day as commit `ba99c83`. What remains is
making a save record its own attribution beside the number, which moves a claim from a commit
message to the file a reader opens, and then deciding whether a second fixed anchor is worth the
per-benchmark ledger of intended deltas it needs.
