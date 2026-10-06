---
status: NOT-STARTED
raised: 2026-08-27
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 187. Read the x86_64 arch tree through the lens the first arch audit used

Minted 2026-08-27 by calef, from finding 8 of the architecture-list parity
sweep. That sweep asked how many siblings `script/stack-frame-check`'s hardcoded
`arches="aarch64 riscv64"` had and found eleven. Ten are strings in shell scripts that a one-line
edit fixes, and they are the worklist of the sweep's own milestone, minted alongside it on pull
request #568 and not yet on `main` when this block was written; cite that one by number once it
lands. This one is not a string: `notes/arch-audit.md` contains zero occurrences of "x86", and the
directory it does not mention is the largest of the three.

The lens exists, the template exists, and the disposition vocabulary exists
(DECISIONS §35, how a scanner's findings get dispositioned). Nothing here is a design fork. Findings
may raise forks, which is what an audit is for; raising them is not the same as this milestone
needing one to start.

## What the existing audit actually did, because "on its own terms" is the whole scope

`notes/arch-audit.md` is not a review. Reading it as "somebody read the arch tree and looked for
bugs" is what would make this milestone unpickable, so the shape is worth stating precisely.

It took exactly one bug class, stated generally before any code was read, under that note's
heading *The bug class, stated generally* (reproduced without an attribution line because it is a
block quote there too, and `script/citations` cannot match a quote of a quote past the source's own
`>` prefixes):

> Kernel code that stages state in single-copy hardware registers, or in any per-CPU location that
> the exception path itself clobbers, across more than one instruction, while an exception,
> interrupt, or preemption can land in the middle.

It asked four fixed questions of every candidate, and the fourth is the one that does the work:
(a) what is the window, (b) what can land in it, (c) what state is corrupted if something does, and
(d) **is it reachable**, or is it a window that exists in the instruction stream that nothing can
ever land in. Two of the three findings turned entirely on (d), and one of them (the RISC-V
`sstatus` mask) was reported as a *record* defect rather than a code defect: the code was correct,
and the comment claiming it was correct "by construction" was false on that ISA.

It recorded the cleared candidates too, at length, and that section is longer than the findings
section. The stated reason is that "we looked and it is fine" is the other half of an audit, and
that each cleared candidate is a place a future change could break something. A pass that reports
three findings and nothing else has thrown away most of what it produced.

It established structural facts that bound the search, and these are the cheapest thing in the
whole method. Two, on the ISAs it read: there is exactly one `eret` and exactly one `sret` in the
kernel, both inside the restore sequence, so there is no second way out of the kernel to audit; and
there are exactly two places that fabricate a trap frame, one per ISA. Establishing those took
minutes and made the rest of the read finite.

It named its own general lesson, which is the finding that outlived the three bugs: when the
same fix is applied to two architectures, check whether it is load-bearing for the same reason on
both. Finding 1 was a complete fix on aarch64 and a partial one on RISC-V, and the symmetry of the
two patches is what concealed it. Finding 3 was the same shape from the other side: one ISA quietly
getting a guarantee from its hardware (the GIC's write-1-to-set enables) that the other does not.

**What it deliberately did not cover.** It read `kernel/src/arch/` plus the code on the other side of
the seam that the class reaches into (`drivers/gic.rs`, `drivers/plic.rs`, `sched.rs`'s switch path,
`cpu.rs`, `smp.rs`'s `secondary_main`, and `user.rs`'s trap-frame placement, read only). It did not
read for memory ordering, for integer overflow, for time-of-check-to-time-of-use, or for anything
else; those became milestone 43's lenses precisely because this one did not take them. It closes by
saying so: an audit by reading finds what the reader thinks to look for, and the bug that
prompted it was found by a failing test rather than by any of the earlier readings of that same
file.

## The scale, measured

How this was counted, since "lines" is ambiguous and the number should reproduce:

```sh
$ find kernel/src/arch/x86_64 -type f -exec cat {} + | wc -l
```

Every file in the directory, `.rs` and `.s` alike, counting physical lines including blanks and
comments. That is not SLOC, and for an audit by reading it is the right measure: a reader reads the
comments, and on this tree the comments are where the hardware constraints are written down.

The check that this is also the method the original note used: the same command over
`kernel/src/arch/aarch64` and `kernel/src/arch/riscv64` at commit `8334d0ba`, the commit that added
`notes/arch-audit.md`, gives **6,202 lines**, against that note's own "the whole arch tree is about
6,200 lines". The figure reproduces, so the comparison below is like for like.

| | Files | Lines | State |
|---|---|---|---|
| `kernel/src/arch/x86_64` | 18 | **6,797** | never read under any lens |
| `kernel/src/arch/aarch64` + `riscv64`, at the audit (2026-07-29) | 25 | 6,202 | read in full |
| `kernel/src/arch/aarch64` + `riscv64`, today | 29 | 9,835 | read in full as of 2026-07-29 |

Two things follow, and the second is a correction to how the sweep put it.

The unread directory is larger than the entire two-ISA tree that the one affordable pass read in
full. That is finding 8's claim and it holds exactly.

But x86_64 is **41% of today's arch tree, not a majority of what is unread**. The two ISAs the audit
did read have grown by 3,633 lines since it read them, 59%, and riscv64 has more than doubled
(2,374 to 4,932). "Read in full, both ISAs" describes a state of the tree that no longer exists.
This milestone does not take that on, and says so under *What this does not decide*, but a reader
who takes 6,797 as "the unaudited fraction" is reading it too favorably.

## Why this is the highest-value unaudited surface

Three reasons, in the order they matter, and one caveat that cuts against.

It is in the trusted computing base, and no prover reaches it. `script/verify`'s Kani harnesses
cover the pure-logic crates: `paging`, `frames`, `slab`, `elf`, `capability`, the allocators, the
ring validators. There is no tool in this project that can prove `trap.s`, which is why
`notes/arch-audit.md` calls the hand-written architecture assembly the least-verified code in the
TCB and why milestone 20 (a portable HAL, proven on a second architecture) says the same. An audit
by reading is the compensating control, and on the largest of the three trees it has not been paid
at all.

It is the newest port and has had the least review time. aarch64 has been read since 2026-07-12,
riscv64 since the parity work, and both were read deliberately on 2026-07-29. The x86_64 arch layer
landed 2026-08-23 (commit `797a20ac`, milestone 161 (the x86_64 kernel port), which is still
PARTIAL). Four days of elapsed time, no dedicated read.

Less of it is exercised than on the other two ISAs. Every x86_64 boot in this tree is QEMU q35,
and that runner's own header says what is not attached: "no virtio disks, no NIC, no GPU, no RNG",
with NVMe wired as the exception. x86_64 also has no interactive boot at all until milestone 182
(x86_64's own interactive-boot entry point) lands. So the suite does reach the trap path, the MMU,
the timer and the interrupt controller on every run, and a reader should not imagine this code is
unexecuted; what it does not reach is the device and userspace surface the other two runners cover.
The narrower claim is the true one: on aarch64 a mistake has more ways to become a failing test than
it does here.

**The caveat, stated so this is not manufactured urgency.** By exposure, this is the *least* urgent
of the three architecture trees, for the same reason it is the least reviewed: nobody is running it.
The argument above is not "x86_64 is on fire"; it is that the compensating control this tree relies
on for its least-verified code has been applied to two of the three architectures and to neither of
them recently, the gap is invisible to every gate we run (see below), and the cost of closing it is
known because the same read has been done once already at a comparable size.

## What makes it tractable

The original pass read 6,202 lines under one lens and produced three findings, a longer cleared
list, and two bounding facts. It was one lane. This is 6,797 lines, so the honest price is **three
lanes** if the passes are cut by risk rather than by file, and each pass is then *smaller* than the
single pass that already proved affordable.

The cut below is a partition: the three sums are 1,741, 1,722 and 3,334, and they total 6,797.

Pass 1: the entry and exit path (1,741 lines). `trap.s` (303), `exceptions.rs` (934),
`context.s` (89), `context.rs` (94), `segments.rs` (321). This is where the bug class lives, it is
smaller than either ISA the original read, and it should go first because everything else is
measured against what it establishes.

The bounding facts to re-establish here, which is the first hour's work and not the audit proper:

- One way out, confirmed. `grep -E '^\s*(iretq|sysretq)'` over the tree finds exactly one
  instruction, `trap.s:182`. The original note's "no second way out of the kernel to audit" transfers.
- One frame-fabrication site, `TrapFrame::for_user_entry` at `exceptions.rs:134`, matching both
  other ISAs. Transfers.
- **Two ways *in*, and this one does not transfer.** `exceptions.rs` configures `IA32_LSTAR`,
  `IA32_STAR`, `IA32_FMASK` and `IA32_EFER.SCE`, so `syscall` is a second entry whose masking
  discipline is set by an MSR rather than by a gate descriptor. The original audit's archetype is
  "a path that enters the save/restore sequence with interrupts live", and what `IA32_FMASK` clears
  is exactly the question that archetype asks. Establish it before reading anything else.
- `swapgs` is the x86_64 archetype of the class, and neither other ISA has one. Three
  instructions, `trap.s:105`, `:178`, `:206`. The window is single-copy by construction: between the
  exit `swapgs` and the `iretq` the CPU is in ring 0 holding the *user's* GS base, and `mmu.rs:529`
  already documents that window in its own comment. aarch64's per-CPU pointer is `TPIDR_EL1`, a
  system register no frame carries; riscv64's is `tp`, whose migration hazard the original audit
  already cleared. So the class transfers to x86_64 with an instance that has no precedent in the
  note, which is the best possible sign that the lens is still worth aiming.

Pass 2: masking and the interrupt-controller adapter (1,722 lines). `irq.rs` (789),
`mod.rs` (411), `timer.rs` (351), `port.rs` (109), `interrupts.rs` (62). Second because finding 3 is
the shape most likely to repeat: it was not a missing barrier, it was one ISA's driver written as if
its hardware gave a guarantee it does not. The asymmetry that points here is a line count.
`irq.rs` is 789 lines on x86_64 against 90 on aarch64 and 208 on riscv64, because it holds the LAPIC
and IOAPIC where the others hold a thin adapter over a driver in `drivers/`. Eight times aarch64's
adapter is eight times the surface for finding 3's shape, and none of it has a `drivers/gic.rs` to
have taken a lock already.

Pass 3: address space, bring-up, and the rest (3,334 lines). `mmu.rs` (1,295), `machine.rs`
(618), `boot.s` (457), `iommu.rs` (368), `rtc.rs` (197), `ap_boot.rs` (183), `isa.rs` (156),
`semihosting.rs` (60). Last because the original pass cleared the analogous code on both ISAs by a
structural argument that is cheap to re-check: `boot.s` and the MMU-enable sequences run on a core
with no trap vector installed and interrupts masked, so they are uninterruptible by construction
rather than by care. If that argument holds on x86_64 the pass is short; `ap_boot.rs` and
`machine.rs` have no analogue in the note and are where it is most likely not to.

One finding's disposition must be re-argued rather than inherited. Finding 2 (riscv64's
`trap_entry` parking a user-controlled value in `sscratch` across the faultable frame stores) was
left documented, on a cost argument: hardening it costs a store and a load on the hottest path in
the kernel to guard a case behind a kernel-stack overflow that is already fatal. Option 2 in that
note is "a separate per-hart trap stack, entered when `SPP = 0`", and x86_64 has that in
hardware: `segments.rs` builds a TSS with seven IST slots, and an IST vector switches stacks
unconditionally. So the same question reaches x86_64 with the expensive half already paid, and the
answer may well differ. Inheriting the riscv64 disposition would be the exact mistake finding 1
recorded as its general lesson.

## The cadence question, and why it is a second milestone rather than half of this one

The sweep's second claim is that this gap will never surface on its own, because "the audit cadence
counts elapsed time and shipped components and has no notion of an architecture". That sentence is
in pull request #568's body and in none of the files it landed, which is the record shape
`AGENTS.md` warns about most directly: a pull request body is read while the diff is open and never
again. This block is the tracked home it did not have.

The claim was checked here against `script/audits`, `design/audit-reports/README.md` and
`.github/workflows/audit-cadence.yml`. It holds, and it is worse than stated in one respect and
better in another.

Worse: the counted triggers are measurably blind to an ISA. `script/audits` has four,
DECISIONS §74 (audits run on change, not on the calendar): milestones BUILT, components
(`crates/*/` plus `[[bin]]` targets), ABI constants, and external packages. At commit `797a20ac`,
which added fourteen files under `kernel/src/arch/x86_64/` in a 2,241-line commit, every one of the
four was unchanged: built 97 to 97, components 122 to 122, ABI constants 51 to 51, external packages
108 to 108. An
architecture adds no crate, no program, no syscall constant, and no dependency, so the only counter
it can ever move is `milestones built`, by one, when milestone 161's row flips from PARTIAL. Against
a security cadence that fires at 15, an entire instruction set is worth one fifteenth of a
trigger. The script's own BUGS predicts this in general terms ("a change that lands as neither a
milestone nor a component moves no number here") without noticing that the largest single addition
to the TCB in the project's history is that shape.

**Worse still: the cadence's unit is the kind, not the lens.** `notes/arch-audit.md` is on record as
a `security` audit dated 2026-07-29. Three later `security` audits have run, so `script/audits`
prints `security last 2026-08-17` and the arch-and-assembly lens's age is invisible. The `lens` cell
is free text; `--check` validates only that it is non-empty, under a message that reads "the value
of an audit is the lens the last one lacked, so a row without one cannot be read for what to do
next". That is the field holding the information, and nothing ever reads it. In `AGENTS.md`'s
ladder this is a rung-2 gate firing reliably on the wrong question, which is the shape that reports
green over a real hole.

Better: one trigger does reach it, by accident, under a question about something else. The
`security` kind carries an uncountable judgment question, printed on every run: "has this booted on
a new machine class (a board, a cloud) since the last audit?" A reader could answer yes for x86_64
under q35. But a machine class and an instruction set are different axes (a Raspberry Pi is a new
machine class on an already-audited ISA), the arch layer landed after the most recent security
audit so the question has not yet been put to anyone, and the script's own BUGS calls this rung four
and "the weakest part of the mechanism". A mechanism that catches the right thing under the wrong
question, once, if somebody reads the output, is not a mechanism.

### The decision: split, and this milestone is the reading half

Four reasons, strongest first.

**1. The gate must not be written by the lane that satisfies it.** `AGENTS.md`'s first principle
says a gate can be written to pass. A single lane that both teaches the cadence about architectures
and performs the audit clearing it chooses the trigger's shape knowing what will read green when it
is done. Two lanes and two pull requests is the cheapest guard available, and it costs nothing.

2. They are different sizes by an order of magnitude, and bundling makes the small one wait.
The audit is three passes. The cadence change is a cadence row, a judgment entry, and possibly one
counted trigger: well under a lane. Bundled, the mechanism lands when the last reading pass lands.
Split, it can land tomorrow, and the tree stops being blind while the audit is still being read.

**3. The cadence change has value even if this audit never happens**, which is the test for whether
something is its own piece of work. The 59% growth in the audited ISAs' own trees since 2026-07-29
makes the point with no reference to x86_64 at all: a lens's coverage decays on the architectures it
*did* cover, and nothing measures that either. A tree that cannot see either kind of decay is broken
whether or not anyone reads x86_64.

**4. Part of the fix is not a lane's call.** The obvious shape, giving the arch-and-assembly lens its
own audit kind, means retroactively reclassifying the 2026-07-29 row in
`design/audit-reports/README.md` and changing what `script/lint` runs on every pull request. That is
a global record and a global gate, which `AGENTS.md` assigns to the integrator rather than to a
lane, and handing it to a lane mid-read is the worst moment to ask.

The one-milestone case, and why the roadmap row answers it. The case is that auditing x86_64
without fixing the blindness leaves the fourth architecture in the same position. That is an
argument that both must happen, not that they must be one milestone, and this tree's own mechanism
for "this must not be forgotten" is a tracked row rather than a bundle. Bundling is rung four in a
milestone's clothes: it relies on the reading lane remembering the second half.

Ordering: neither blocks the other. If the cadence milestone lands first the story is tidier,
because its red light is then what this milestone answers. Sequencing them strictly would delay this
one for no gain, since we already know x86_64 is unread and do not need a mechanism to tell us.

### The second milestone, proposed provisionally

calef mints the number and the title; this is the content, recorded so it has a home rather than
living in a report.

**Provisional title: teach the audit cadence what an architecture is.** Three questions it decides,
none of them decided here:

1. Whether the arch-and-assembly lens gets its own audit **kind** with its own cadence row, or
   whether the existing kinds get an architecture-shaped trigger. The first is a table row; the
   second is code in `script/audits`.
2. Whether a counted trigger can be made sensitive to the arch tree at all. Directories under
   `kernel/src/arch/` is one candidate and fires once per ISA, which is the right frequency for the
   gap found here. Lines under `kernel/src/arch/` is another and would also catch the 59% decay on
   the ISAs already read, at the cost of a threshold somebody has to choose.
3. What to do with the `lens` cell, which is the field that holds the information and the field
   nothing reads.

## What this audit cannot find

The `BUGS` posture, stated before the work rather than after it, because every item here is a limit
a reader should know when they read the resulting note.

The headline limit is the original note's own closing paragraph, and nothing this milestone does
changes it:

> That is a reassuring result, and it should be read with its limit attached: an audit by reading
> finds what the reader thinks to look for. The original bug was found by a failure, not by
> inspection, and it had survived every previous reading of that file.
>
> -- notes/arch-audit.md

The rest, specific to this pass:

- **One lens is one lens, not coverage.** Reading 6,797 lines for state staged across single-copy
  registers will not find x86_64's memory-ordering bugs, its integer overflows, its IOMMU
  misconfigurations, or its ACPI parsing. Those are other audits, and milestone 43 (a second
  security audit, with a different lens) is the precedent for taking them separately rather than
  pretending one pass covers a surface.
- **Most findings will not be confirmable by running anything**, and that is a property of the bug
  class rather than of x86_64. Every window this lens looks for is a few instructions wide and
  closed by an invariant somewhere else, so the usual confirmation (a test that fails without the
  fix) is mostly unavailable. The original audit said so about finding 3 in its own words: a loop of
  two harts hammering the PLIC's enable bits passes with the lock and passes without it. x86_64
  makes it somewhat worse, because the device surface the suite does not attach is a set of callers
  that cannot be made to race at all today.
- It proves nothing. After this milestone the honest position is what it was after the last one:
  `vectors.s`, `trap.s` and their x86_64 twin are trusted rather than verified, and notes like these
  are how the project pays for that.
- **It does not re-audit the two ISAs that grew 59% underneath their own audit.** That is named
  above, measured, and left to the cadence milestone to make visible rather than folded in here.
- A cleared candidate is cleared as of one commit. The cleared list is the more durable half of
  the output and it is also the half that rots silently, because nothing recomputes it.

## What a lane taking this should do

1. Re-establish the bounding facts in pass 1 before reading for findings. They are cheap and they
   decide how much of the rest is finite.
2. Read pass by pass, in the order above, one pull request per pass. A pass that finds nothing still
   ships its cleared list; that section is the deliverable, not the leftovers.
3. Fix what is a live defect and record what needs a decision. The original pass fixed two of
   three findings on review, as execution inside decided architecture, and left the third documented
   with its options priced. A finding that needs a design fork gets written up and raised, not
   settled mid-read.
4. Land the result as a note under `notes/`, named by calef, and add the rows to **both** tables of
   `design/audit-reports/README.md` with counts taken from `script/audits --baseline` at the landing
   commit. Which `kind` the row carries depends on the cadence milestone: if it has landed, use the
   kind it creates; if not, record it as `security` the way 2026-07-29 was, and let that milestone
   re-date it.

## What this does not decide

Whether the two ISAs that were read stay read. They have grown 3,633 lines since 2026-07-29,
59%, and re-reading the delta is a second body of work with its own size. It is named here because
the numbers in *The scale, measured* would mislead without it, and it is left to the cadence
milestone to make visible rather than folded in: a mechanism that can see an unaudited architecture
should be able to see an audited one that moved, and building it once is cheaper than reading twice.

How the cadence learns about architectures. The three questions under *The second milestone,
proposed provisionally* are recorded, not answered. Answering them means touching `script/audits`,
the audit index, and what `script/lint` runs on every pull request, and this milestone deliberately
touches none of the three.

Anything a finding turns out to require. A finding that needs a memory-ordering decision
(DECISIONS rule 4), a change to the trap frame two programs agree on, or a hot-path cost is a fork
for calef, raised with its options priced the way finding 2 was. An audit that quietly redesigns the
thing it is auditing has stopped being an audit.

## Prior art

The in-tree precedent is the method itself, and it has run four times: `notes/security.md`
(2026-07-15), `notes/arch-audit.md` (2026-07-29), `notes/shared-page-audit.md` (2026-08-04), and
`notes/untrusted-input-audit.md` (2026-08-15). The second is the template and the others are
evidence that a single-lens pass is the shape that works here.

The three build-versus-reuse questions the roadmap asks of every milestone do not apply: nothing is
being built, so there is no code to use and no dependency to weigh. The ecosystem question that
would apply, how other microkernels compensate for unprovable assembly, is not researched here and
is not claimed. seL4's answer is known in outline (its proof stops at the same boundary and its
assembly is axiomatised) and is deliberately not cited, because `AGENTS.md` asks for prior art read
rather than recalled and this lane did not read it.

## What this unblocks

Nothing depends on it, which is the honest answer and part of why it went unnoticed. What it changes
is the confidence attached to every x86_64 claim the project makes: milestone 182, milestone 184
(extend the `std` port to x86_64) and the rest of milestone 161's remaining work all build on this
layer, and each of them currently inherits an unread TCB. DECISIONS §19 (architectural parity is a
tenet) makes x86_64 a first-class target; this is the compensating control that parity implies and
that the other two targets have already had.

## BUGS

Not started; nothing built yet to carry its own BUGS section. The limits of the work itself are
under *What this audit cannot find*, above, and are stated in advance on purpose: they are the
reasons a reader should not treat the resulting note as a clean bill of health.

## Index row

`notes/arch-audit.md` contains zero occurrences of "x86". Measured: `kernel/src/arch/x86_64` is 18
files and 6,797 lines, larger than the 6,202-line two-ISA tree that audit read in full, in the
code that note calls the least-verified in the TCB and that no prover reaches. Priced as three
passes cut by risk, each smaller than the one pass already proven affordable. The cadence's
blindness to this (an entire ISA moved none of `script/audits`' four counters) is argued out as a
separate milestone rather than folded in, so the gate is not written by the lane that satisfies
it.
