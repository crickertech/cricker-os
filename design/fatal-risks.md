# The nine things that would kill nife

<!-- prose-budget: exception. 4,235 words against a 3,000-word cap. Ratified by calef on 2026-09-24
     (UTC). Reason: nine entries each keeping a claim, a status, an experiment with an owner and a
     cost, and their caveats do not compress below this without dropping one of the five; the running
     order and BUGS spend about 960 words after the last entry. Marker syntax is PROVISIONAL until
     the prose-budget gate exists. See this file's BUGS section. -->

calef, 2026-08-30: *"something that would kill nife for me as a project is a fatal characteristic
that would demonstrate the approach isn't viable... We should then try to prove or disprove those
things."*

This file is the falsification list. Not a risk register, which tracks things that might go badly. It
is a list of claims that, if false, mean the project should stop. A risk you can mitigate belongs in
a milestone. A risk you can only answer belongs here.

It was written the same week the project's first customer left, when the family's backups moved to
borg over SSH on cordoba because nife was not ready. With no customer, the ranking function has
nothing to rank by. The substitute is "find out whether this can work at all."

It is a six-pager, and the depth is in appendices (calef, 2026-09-23). At 17,742 words, reading it
once cost a maintainer session most of a context window. A reader can decide what to work on next here,
without opening a single appendix. Each entry links one, under
[`design/fatal-risks/`](fatal-risks/), holding that risk's evidence, numbers, corrections and refusals
for anyone who wants to verify or challenge a verdict. Studies with a home of their own stay in
`notes/`. Superseded numbers are in `git log -p design/fatal-risks.md`.

## The rule an entry has to meet

Three properties, and an entry that lacks one is a worry rather than a risk:

1. It can come back red. An experiment that can only confirm is not a test. Where an experiment
   is structurally confirmation-biased, the entry says so and names the second pass that fixes it
   (risk 2 is the worked example).
2. The experiment is cheap relative to the project. A test that costs a year answers a question
   the year would have answered anyway.
3. It does not wait on more of the project being built. Otherwise it is a schedule, not a test.

The ranking is chance-of-fatal times cheapness-of-test, which is why the running order at the bottom
is not the numbering. The numbers are identity, like a milestone's.

## Who may change an entry

Verdicts are the architect's: the Experiment status word, the colour and the running order. The
maintainer corrects a factual error (a wrong date or instrument, a claim the machine disproves)
without asking, dated and citing its source. Facts arguing for a new verdict go to the architect.
[§216 (fatal-risk facts are correctable, and verdicts are the architect's)](decisions/216-fatal-risk-facts-are-correctable-verdicts-are-the-architects.md),
2026-09-25.

## What an entry's Experiment status says, and the three words it may say it in

Every entry carries one Experiment status line. It answers one question: has the experiment happened.
calef ratified the field and its three values on 2026-09-23, in
§211 (what a fatal-risk verdict says, and what the chart can plot as a result).
`script/fatal-risks` fails on a fourth value, because the set was open until then and three lanes
minted three words in one day. The script's own header carries the ratification and the refusals.

| value | what it asserts |
|---|---|
| `RUN` | the experiment has been performed, whatever it found |
| `NOT-RUN` | it has not been performed, and could be |
| `CANNOT-RUN` | it cannot be performed at all, and the entry says what would change that |

What it does not say is what the experiment found. That is prose, and it is where `GREEN`, `AMBER`,
`MEASURED` and `AUDITED` live. None of the four is a value of this field. A reader who wants to know
whether nife is in trouble reads the paragraph.

## 1. Only software written for nife runs on nife

The claim, stated so it can fail: the platform runs hand-written Rust and nothing else, so every
piece of software anyone wants has to be rewritten. It is the most dangerous entry because it is
structural. Optimization cannot fix "nothing runs here". A system in this state is a research
demonstrator forever, which is not what DECISIONS §14 (a verified-Rust capability microkernel that
runs real workloads) claims.

**The experiment:** milestone 121 (`ripgrep`: enumeration as a capability), for its real dependency
tree, its filesystem walk and its threads.

**Experiment status: RUN, 2026-08-31.** GREEN on all three architectures since 2026-09-16, and the
blocker is not what anyone predicted. Unmodified `ripgrep` 14.1.1, forty transitive crates, zero
patches, and three byte-identical transcripts from three separately built binaries
([`notes/ripgrep-on-nife.md`](../notes/ripgrep-on-nife.md)). What stopped it was the missing
argument vector, milestone 205 (how a foreign program is told what to do). Correction, 2026-09-27:
205 is BUILT, and what keeps `ripgrep` from the prompt now is the 256 KiB image ceiling (#1399).

DECISIONS §105 (`std::thread::spawn` stays declined, until a customer needs it) was never reached,
and that reverses the premise. `ripgrep` asks `available_parallelism()` rather than assuming it, and
nife answers `Ok(1)` honestly. A platform answering `Unsupported` there would have failed this
program.

The caveat. The structural fear is retired. The one published argument that speaks to this says it
goes badly: clean-slate kernels have *"significantly fewer features than Linux ... impeding
adoption"*, risk 8's paper ([`notes/incremental-path.md`](../notes/incremental-path.md)).
[Appendix](fatal-risks/somebody-elses-software.md).

## 2. The proofs prove trivia, and the real bugs live where Kani cannot reach

The claim: the verification half of DECISIONS §14 is real but narrow, and narrow in the direction
that does not matter.

**The experiment:** milestone 191 (did the proofs catch the bugs?), against this project's own defect
history, plus a reverse pass asking which harnesses prove a property that could plausibly be false.

**Experiment status: RUN, 2026-08-30.** AMBER. The red half is that no standing proof has caught a
regression: every defect a proof caught was caught while its harness was being written (rule 1's
survivorship asymmetry). The second reason is reach. Eight harnesses prove kernel
code on all three architectures; none passes `asm!`, fixed-address MMIO or an `arch/` subtree its
host skips. Files with `asm!` hold 15,966 of `kernel/src`'s 86,528 lines, about 18%
([`notes/kernel-proofs.md`](../notes/kernel-proofs.md)). Reworded 2026-09-25 on the architect's
ruling; it said "the red half is structural", naming a crate boundary milestone 193 (put
`kernel/src` within reach of the prover) removed on 2026-08-30
([`notes/proof-retrospective.md`](../notes/proof-retrospective.md); PR #589).

The first x86_64 proof went red on a latent defect, the first of the class this risk asks about. The
claim: proofs over the pure crates and slices of a mostly unverified kernel. [Appendix](fatal-risks/proofs-and-their-reach.md).

## 3. The tests do not test anything, and the quality is illusory

The claim: AGENTS.md's principle 2 says the method works because of the gates, the proofs and the
review discipline. If the suite would not notice the code being wrong, that sentence is decoration.

**The experiment:** milestone 85 (mutation testing over the host crates), read as a census and
re-read against the baseline.

**Experiment status: RUN, 2026-09-19, re-read 2026-09-24.** MEASURED
rather than merely observed, and AMBER. calef ruled amber on the 2026-09-14 numbers; the fall behind
it did not happen. On 2026-09-21 the 38 baseline crates read 96.1% against
92.4% in August. The corpus reads 92.4%, or 93.6% without 132 mutants no host build
compiles ([`notes/mutation-testing.md`](../notes/mutation-testing.md)).

It stays amber on the standard this entry holds: milestone 85's rule that every survivor becomes a
test, an exclusion carrying its reason, or a recorded gap. The 2026-09-21 census counted 771 missed
survivors; after #1277's 164 kills and triage, 414 are projected, measured at the next census
([triage](../notes/mutation-testing/census-2026-09-21-triage.md)), and
milestone 326 (nobody has been assigned to turn a mutation score upward) owns the repair. Green is a ruled
condition rather than a number (calef, 2026-09-20): inflow, meaning the survivors a merged pull
request adds on its own lines are triaged.

Two caveats. This verdict speaks for the host-testable corpus and not for the kernel, where a census
is roughly 500 runner-hours against 52 minutes today. And one convention is load-bearing and
unchecked: whether a timeout counts as a kill moves this entry two points. That rule rests on a
hand-check of 96 timeouts in August; 206 stood on 2026-09-21.
[Appendix](fatal-risks/the-mutation-verdict.md).

## 4. The architecture imposes a per-crossing cost that cannot be engineered away

The claim, and calef named this one first: a capability microkernel pays on every boundary crossing,
and on workloads that cross constantly the cost is architectural rather than a matter of tuning.

**Experiment status: RUN, 2026-09-23.** No verdict yet; one bench evening stands between here and
one. Everything measured is a single crossing, and the claim is about a cost that cannot be amortised.
Amortisation is a property of a workload. The single-crossing numbers are four wins and a tie against
Linux on the same core, every caveat beside its number
([`notes/benchmarks.md`](../notes/benchmarks.md)), over committed floors
([`bench/baseline-aarch64.txt`](../bench/baseline-aarch64.txt)).

**The decisive experiment that has not been run:** milestone 168 (a multi-tasking workload
benchmark), one radon evening, at least five boots, by [`notes/job-mix.md`](../notes/job-mix.md)'s
procedure. Its step 7 wrote down what each outcome means before the numbers exist, so the reading
cannot become a defence afterwards. The one silicon sweep so far is not quotable: 29.4% spread
between boots at four tasks, and no page mapping or process creation in the mix.

Two caveats. The counter-thesis is published: the crossing can be removed rather than made cheap. If
RedLeaf and the 2017 Rust-kernel paper are right, a capability crossing is a cost this project chose
rather than inherited, and their open problem is risk 5. And `sel4bench` has never produced a number,
so the peer is Linux rather than the state of the art in minimal kernels. It ranks sixth although a
skeptic expects the project to die here, because this is where the most evidence says it will not.
[Appendix](fatal-risks/the-crossing-cost.md).

## 5. It cannot be made reliable on multicore, and the bugs appear only on silicon

The claim: the concurrency is wrong in ways that QEMU cannot show and that arrive one at a time,
forever.

**Experiment status: RUN, 2026-09-25, on radon only.** radon soaked 8 h 09 m clean: 4.1 million
cross-core handoffs with no refused wake, wrong reply or stall
([`notes/visionfive2.md`](../notes/visionfive2.md)). argon and xenon have not run it. The VisionFive
2 wakeup this entry once opened with was retracted on 2026-08-15, so the gate has never fired on a
field failure ([`notes/scheduler.md`](../notes/scheduler.md)). Milestone 201 (is multicore
reliability converging)'s curve now holds radon's four soak boots: about 12 hours and 4.1 million
crossings, zero defects ([`notes/multicore-defect-curve.md`](../notes/multicore-defect-curve.md)).
One draw of the placement lottery is a confidence, not a verdict.

Every multicore defect this project has found whose instrument is recorded was found without
silicon: one by loom, one by an audit, and the rest under QEMU, most at two cores. The only one seen
on physical cores and not under TCG is an HVF test hang that is still unclassified. A defect
emulation can find is not evidence about the class it cannot, but it retires the reading that
silicon is the only productive instrument.

**The decisive experiment:** milestone 225 (run the soak on radon, argon and xenon), run on radon;
argon and xenon remain.

Two caveats, argued in the [appendix](fatal-risks/multicore-reliability.md): every load-sensitive
red so far has been a test bug, which fits a healthy kernel and a blind instrument equally well, and
no result here can be green, since a flattening curve is only a confidence.

## 6. A capability-confined userspace driver cannot drive real hardware at real speed

The claim: the thing that makes the thesis interesting, drivers outside the kernel behind an IOMMU,
does not survive contact with a real device.

**Experiment status: RUN, 2026-09-16.** All three of its parts are now measured on silicon, and they
were never one claim. On radon, milestone 159 (a real hardware entropy source: the JH7110's TRNG)'s
driver is an EL0 process reaching the TRNG through a capability that names no device. Confined,
2026-09-03. Driving real hardware, 2026-09-04, reproducibly. At real speed, MEASURED 2026-09-16 at
955,223 bytes/s, about 8.4 us per round trip.

**The decisive experiment:** one real, non-virtio device on real silicon, confined, at throughput.
Every piece now exists and the remaining distance is a bench evening. Milestone 261 (the NVMe driver
leaves the kernel, on the machine that can finally confine it) is §86 (whether an NVMe driver can
leave the kernel, and what capability would let it)'s option 2a. xenon has a plain PCIe NVMe function
and VT-d, booted nife on 2026-09-17, and calef wiped its disk that day.

Two caveats. This does not retire the risk, and the reason is the device. A TRNG has no DMA and one
register window, so it is the smallest real device on the board. The rate is not comparable to a
Linux `hwrng` figure either, which is a read from an in-kernel driver with no IPC in it. And on the
night two things must hold, neither of them code: the DMAR's device scope must cover the NVMe
function, and the LBA size must give `blocks_per` in `1..=8`, or the line reads `skipped`. A skip is
not a pass. [Appendix](fatal-risks/the-confined-driver.md).

## 7. The confinement claim is false

The claim: a confined component escapes, and the property the whole system is built to provide does
not hold. What was missing: every test of it was written by the same people who wrote the thing being
tested.

**The experiment:** milestone 202 (every confinement test is a ritual until somebody breaks the
confinement and watches it fail).

**Experiment status: RUN, 2026-08-31.** 26 claims
enumerated, three of them stated nowhere, and 25 harnesses now carry a replayable falsification, up
from 6 ([`notes/confinement-claims.md`](../notes/confinement-claims.md); PR #614). The finding is
worse than a missing test. A page-table assertion was patched to remove the check it exists for and
still passed, because it answered "U-mode cannot read the kernel" by refusing to look. It had done so
since milestone 41 (dead code: triage the suppressions, and un-blindfold the gate), with every gate
green throughout. A test that cannot come back red is indistinguishable from a test that passes, and
three independent sweeps have each found confinement tests that could not fail.

The adversarial pass is AUDITED, 2026-09-17: a qualified yes with one exception, found and fixed.
Milestone 313 (the security audit that was due since August) found DECISIONS §12 (call/reply IPC: a
one-shot reply capability)'s claim that a consumed capability cannot be used again false on x86_64,
on a path every boot takes.

The caveat that keeps the gate closed: it was us attacking our own system. A hole we closed ourselves
is the same category of evidence as the audit that found it. The outsider trying to escape is gated
behind milestone 198 (a package manager, and the trivial install that makes a second customer
possible), by calef's no-third-parties position. Nothing here says the confinement holds. What it
supports is that these named claims are tested, and each shown to fail when broken.
[Appendix](fatal-risks/the-confinement-claims.md).

## 8. Nobody needs it

The claim: everything works and no one has a reason to run it.

**Experiment status: CANNOT-RUN, 2026-09-23.** Untestable by this project's own policy, and no
verdict. The other eight can come back red; this one cannot come back at all, which is the most
dangerous state a fatal risk can be in.

It has already fired once. In August 2026 the customer had a
real deadline, nife could not meet it, and he solved the problem with Linux. That is principle 1
working as designed. What it changed: a first customer should be something nife can plausibly be
adequate at within a milestone or two.

Why no verdict can be rendered. Nobody can be asked to run nife until it installs, which waits on
milestone 198. calef ruled its three forks by 2026-09-23 and the last, §219 (the shell names an installed program),
on 2026-09-26. Fetch and verify are built, and installing waits only on code. Milestone 530 (name a customer, or
admit the ranking function has nothing to rank) ruled on 2026-09-21 that the path stays vacant
(blocked, not empty), so 198 holds the ranking function's top slot. What would falsify it: somebody
who is not calef installs nife on purpose and is still running it two months later. The install is
the weak half, and retention is the claim.

Two caveats. What the green results buy is narrower than it
reads: risks 1 and 9 answer *could somebody run this*, and this entry asks *does somebody want to*.
Treating capability as demand is the error this entry exists to prevent. And the rest is a hope,
recorded as one: one user, who left, zero others, and no evidence here that users arrive once it
installs.
[Appendix](fatal-risks/nobody-needs-it.md).

## 9. The HAL is a fiction, and an architecture costs a restructure rather than a port, and so does the next machine

The claim, calef's, 2026-08-30: *"another proof/disproof of the nife thesis is actual functioning on
the three silicons. If we can't get it to run on one, that would also likely kill the effort."*

Sharpened, because the ISA count is not the fatal part. What would be fatal is what a failure would
reveal: that adding an architecture requires changing the kernel rather than adding a directory under
`arch/`. That is what DECISIONS §4 (kernel shape, with two cheap rules)'s rule 1 and 
§19 (architectural parity is a tenet) claim it does not.

Widened 2026-09-23 from architectures to machines, and the ruling is calef's: *"A nife that runs on
one cloud platform but not another is also its own form of risk."* So it reads at two grains now, an
architecture and an implementation, a particular machine of one. Both words are provisional. The
implementation grain is the earlier warning, and the only one that can be bought.

**Experiment status: RUN, 2026-09-17.** GREEN.
Milestone 87 (the x86_64 bare-metal machine) reached `nife self-test: 5 of 5 passed` on xenon's own
firmware, so nife runs on all three declared architectures on real hardware. Everything it needed
lives under `kernel/src/arch/x86_64/`, and its one defect was fixed inside `arch/x86_64/mmu.rs`. The
cost was measured rather than merely passed: 42 compiler errors, every one "this `arch::` name does
not exist yet", with `crates/paging` unchanged.
[`notes/x86-port.md`](../notes/x86-port.md): *"That is the whole diff above `arch/`. A new ISA was a
new directory."*

**The experiment for the widened grain, which has not been run:** a second machine of an architecture
nife already boots, riding on milestone 225 (run the soak on radon, argon and xenon). It is a boot
rather than a purchase, and finding no difference is a result too.

Three caveats. The verdict is one machine per architecture, and for aarch64 not even that, since
argon has never booted nife. So those 42 errors price a third *architecture* and say nothing about a
second *machine*. The failure this grain fears is silence. The appendix has a worked instance, closed
on 2026-09-23 by milestone 186 (derive the architecture list, and close what it does not reach): five
functions that compiled, shipped and did nothing on the architecture nobody had run them on. And
parity multiplies every other risk here. If the project ever needs to buy time, dropping to two
architectures is the largest single lever available, and it should be a decision rather than a drift.
[Appendix](fatal-risks/the-hal-and-the-next-machine.md).

## The running order

Ranked by chance-of-fatal times cheapness-of-test, not by number. Each cell's verdict is the entry's.

| order | risk | experiment | owner | cost |
|---|---|---|---|---|
| ~~1~~ | 2, the proofs | **RUN, 2026-08-30: amber**, because no standing proof has caught a regression, and `asm!` bounds the reach | milestone 191 | done |
| 2 | 9, the HAL, on the board that already boots | the on-board test-suite exit, so silicon becomes gate-able | milestone 16 (real hardware and IOMMU-backed driver isolation) | bench time, board proven since 2026-08-14 |
| ~~3~~ | 9, the HAL, on the architecture that carries the risk | **RUN, 2026-09-17: GREEN**, five of five on xenon, everything it needed inside `arch/x86_64/` | milestone 87 (the x86_64 bare-metal machine) | done |
| 4 | 9, the HAL, at the implementation grain, widened 2026-09-23 | a second machine of an architecture nife already boots | milestone 225 (run the soak on radon, argon and xenon) | riscv64: about 30 rented hours, €1.51, milestone 89 (Scaleway EM-RV1); still unrented |
| ~~4~~ | 1, the ecosystem | **RUN, 2026-08-31: GREEN on all three since 2026-09-16.** The blocker is a missing argv, not threads | milestone 121 | done |
| ~~5~~ | 3, the tests | **RUN, 2026-09-19: amber.** 96.1% like-for-like against 92.4% on 2026-09-21, and 771 missed survivors (414 projected after #1277) hold the amber | milestone 326 | done; the triage remains |
| 6 | 4, performance | the multi-tasking workload number, from the 2026-09-19 instrument | milestone 168 | one radon bench evening |
| 7 | 9 and 6 together | journey 3, end to end on three boards | journey 3 | months, and it is the capstone |
| -- | 5, multicore | **RUN on radon, 2026-09-25:** 8 hours clean, 4.1 million crossings. A linear defect-discovery curve is the red result | milestone 201 (is multicore reliability converging) | weeks, hardware |
| ~~7~~ | 7, confinement | **RUN, 2026-08-31, extended 2026-09-16, AUDITED 2026-09-17.** A confinement test could not fail, and DECISIONS §12 was false on x86_64. Fixed | milestones 202, 305, 313 | done; the adversarial half remains |
| -- | 8, nobody needs it | **CANNOT-RUN, 2026-09-23.** No experiment, and none available: milestone 576 (how many systems are out there, and what do they run) is behind milestone 198 (a package manager, and the trivial install that makes a second customer possible) | milestone 576 | blocked, not costed |

## BUGS

- ~~Nothing gates this file.~~ Closed 2026-09-11 for the mechanical half by milestone 275 (a gate
  that diffs `design/fatal-risks.md` against the roadmap it cites). `script/fatal-risks --check` runs
  in `script/lint` and compares what this file claims about a milestone or a decision against what
  the record holds. It found four live disagreements on its first run.

  What is not closed is the larger half. A gate can see a status word contradicting the record. It
  cannot see a premise being overtaken, which is what happened to risk 9's cost line: it priced
  milestone 87 as bench time when `notes/x86-port.md` already recorded that no real firmware speaks
  PVH. A green `script/fatal-risks` means no status word here contradicts the record it names. It is
  not a warrant that the arguments still hold.
- Nothing gates an appendix. `script/fatal-risks` parses this file alone, so a verdict restated under
  `design/fatal-risks/` can drift and no check will say so. The appendices therefore do not carry the
  Experiment status field at all, and each says at its head that this document is the claim of
  record. That is rung three of AGENTS.md's ladder, and honest about being rung three.
- The bold this document carries is the gate's, not the prose's. `script/fatal-risks` reads the
  Experiment status lines, the experiment lead-ins and the running order's verdict cells as markup,
  so those spans are machinery rather than emphasis. Every other bold span is gone, and what is left
  spends the whole writing-convention budget of four per thousand words. Editing this file
  means spending the gate's budget, not your own. calef ruled on 2026-09-24 (UTC) that markup a gate
  parses is counted like any other bold, so no exclusion exists and this file has none left. He also
  said the likely answer is to move these fields out of bold entirely, and that the decision waits on
  the decision-frontmatter pilot, pull request #1195, reporting. Until then, do not add a bolded span
  here without removing one.
- ~~Two entries have no owner.~~ Closed 2026-08-31: risks 5 and 7 are milestones 201 and 202, both
  scoped by calef and both reframed in the process, risk 7's by §134 (a harness carries a
  machine-replayable falsification record, or it is not evidence). Neither can return a clean green,
  and both blocks say so where a reader meets them.
- The ranking is a judgement, not a calculation. "Chance of fatal" is nobody's measurement, and two
  readers could order this differently on the same evidence.
- A green result is not proof of anything. Every experiment here can only fail to kill the project.
- The word budget is a constraint on this document, not on the truth. calef asked for 3,000 words
  and accepted 4,235 on 2026-09-24 (UTC) as a marked exception; the marker at the top is what a
  future gate reads. The file grew past that the same day, and calef ruled it back to 4,235. Nine
  entries that each keep a claim, a status, an experiment with an owner and a cost, and their
  caveats did not compress below that without dropping one. The running order and this section are a
  fifth of the budget alone. The count is in the weekly prose-budget series regardless, so the
  exception's cost stays visible. Put any growth in the appendix beside the entry.
