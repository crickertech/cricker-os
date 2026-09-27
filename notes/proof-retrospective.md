# Did the proofs catch the bugs?

Milestone 191. `DECISIONS §14` promises a machine-checked core, `notes/verification.md` explains how
the harnesses work, and until this note nothing in the tree had asked whether they ever found
anything. This is that question, asked against the only evidence that cannot be arranged after the
fact: the project's own written record of real defects.

*Note name provisional, per the naming tenet. An architect names things.*

## The finding

Amber, and the red half is structural rather than a criticism of any harness.

No Kani harness in this tree has ever caught a defect after the day it was written. Every defect
in the corpus below was found by something else: a flaky suite, a boot on real silicon, a fuzzer, a
mutation sweep, loom, a code read, or a CI lint. Not one red `script/verify` run appears anywhere in
the record.

Two real defects were caught while harnesses were being written, which is the proofs working
exactly as designed and is invisible in any defect list. `dtb::be32`/`be64` had an unchecked `at + 4`
that panicked on a near-`usize::MAX` offset out of a corrupt device tree, on the kernel's boot path,
before there is any way to report a failure. `pci::intx_irq` underflowed on a pin of 0 and panicked
in debug builds. Both were found by trying to state totality and discovering it was false, and both
were hardened rather than merely proved. That is the survivorship asymmetry milestone 191's block
predicted, and it is the reason the answer is amber rather than red.

The reason for the red half was one sentence, in `script/verify`'s own header: `cargo kani -p
<crate>` never compiles the kernel, the user programs, or xtask. So 64,818 lines of `kernel/src`,
holding the scheduler, IPC, trap frames, timer, arch trees and resource accounting, were out of
reach by construction. That is, without exception, where the expensive defects were. The harnesses
were not weak; they were not pointed at the code that broke. *(Corrected 2026-09-25: false since
that day, when `kernel` became a harness crate; [`kernel-proofs.md`](kernel-proofs.md) has the
reach now.)*

And the corpus is not evenly distributed over kinds of defect. Of the eighteen entries below,
five are concurrency, four are a hardware or firmware contract, three are resource accounting across
a program's control flow, three are a specification that was never written down, and three are
records rather than code. Kani, as this tree uses it, is aimed at none of those five classes. It is
aimed at the sixth, panic-freedom and algebraic correctness over hostile input, and in that class the
record shows zero escapes: nothing that a shipped harness states has ever afterwards turned out
to be false in the code it covers.

That is the honest shape of the result. The proofs are a net with a known, documented perimeter, and
every fish caught in this project swam around the outside of it.

## The numbers, every one counted here

Prose in this tree quotes "112+ harnesses" (the roadmap) and "over 100 across more than 20 crates"
(`notes/fuzzing.md`, correctly hedged as a floor). Neither is the tree's number today. Every figure
below was taken from the merged worktree at base `f1f138a8` on 2026-08-30, with the command shown.

| Quantity | Value | Command |
|---|---|---|
| Lines matching `kani::proof` anywhere | 148 | `grep -rn "kani::proof" --include="*.rs" . \| grep -v target \| wc -l` |
| Of those, doc-comment mentions in `scripts/kani-lint-shim/` | 3 | same, filtered to that path |
| Actual harnesses | 145 | `grep -rn "^\s*#\[kani::proof\]" --include="*.rs" . \| grep -v target \| wc -l` |
| Harnesses under `crates/` | 143 | same, scoped to `crates/` |

The shim directory was `scripts/kani-lint-shim/` when these were measured; it is
`helpers/kani-lint-shim/` since 2026-09-23, and the captured output below is left as it ran.
| Harnesses in `vendor/redoxfs` | 2 | same, scoped to `vendor/` |
| Packages carrying at least one harness | 25 (24 under `crates/`, plus vendored RedoxFS) | see EXAMPLES |
| Crates in the workspace | 65 | `ls -d crates/*/ \| wc -l` |
| Harnesses `script/verify` actually runs | 140 | sum the shard table in `script/verify` |
| Source lines in harness-carrying crates | 31,725 | see EXAMPLES |
| Source lines of Rust outside `vendor/` and `target/` | 206,728 | `find . -name "*.rs" ... \| xargs wc -l` |
| Source lines of `kernel/src`, which no harness then compiled | 64,818 | `find kernel/src -name "*.rs" \| xargs wc -l` |
| `kani::cover!` vacuity guards, and the four crates holding them | 19 (calendar 7, paging 5, dma_validator 5, glob 2) | `grep -rn "kani::cover" --include="*.rs" crates/` |

Two of those rows are findings rather than facts, and they are picked up below: the gap between 145
and 140, and the fact that twenty of the twenty-four harness crates carry no vacuity guard at all.

Take these numbers from the tree, not from here. This note will drift the same way the roadmap's
112 did. The EXAMPLES section is the re-derivation.

## The method

Four fixed questions per defect, from milestone 191's block:

1. What was the defect, as a property that was false? Not "the board hung". The invariant.
2. Was that property provable at all? A property about what a U74 does with a store buffer is not
   a property of our source and no prover here will ever see it.
3. Did a harness exist that covered it? A harness that existed, passed, and let the defect through
   is the most interesting outcome available, and it gets its own section.
4. What would it have cost to have one? This is what makes the study a worklist.

Then the reverse pass, which is not optional: which harnesses prove something that could plausibly
have been false, and which prove a tautology of the operators they use. Without that pass a study of
proofs can only confirm.

Every counterfactual below is marked as a judgement. "A proof would have caught this" is not a
measurement, and the ones that are close to measurements say why.

## The defect corpus

Sources, all in-tree: `notes/exceptions.md`, `notes/arch-audit.md`, `notes/intrusive-queues.md`,
`notes/interleaving.md`, `notes/visionfive2.md`, `notes/instruction-clock.md`, `notes/fs-server.md`,
`notes/fuzzing.md`, `notes/mutation-testing.md`, `notes/load-sensitive-assertions.md`,
`notes/citations.md`, `design/decisions/76-roadmap-status-versus-tree.md`, and `git log`.

Column 4 is a judgement in every row.

| # | Defect | The false property | Provable? | Harness existed? | Found by |
|---|---|---|---|---|---|
| 1 | aarch64 exception-return race | staged `SPSR_EL1`/`ELR_EL1` survive to the `eret` | no (assembly, hardware semantics) | none possible | a flake, 1 run in 4 |
| 2 | riscv64 `trap_return` sibling | same, for `sepc`/`sstatus` | no | none possible | reading, after #1 |
| 3 | riscv64 `trap_entry` `sscratch` window | no user value is a `&TrapStash` | no | none possible | the arch audit; left documented |
| 4 | PLIC enable-bit lost update | concurrent `enable`/`disable` on one word preserve each other | by loom, not by Kani | none | the arch audit |
| 5 | wake-before-switch-out race | a thread is never queued while a core still executes it | by loom, and now is | Kani harness existed, structurally blind | a flake, 10/10 to 8/10 |
| 6 | `clock_protocol` seqlock missing `fence(Release)` | a reader never sees a torn `(state, offset)` pair | by loom | none | loom, on its first run |
| 7 | timer re-arm drift (milestone 6 (threads)) | the deadline advances one interval per delivered tick | yes, and the proof now exists elsewhere | none | measurement, 100 Hz became ~70 Hz |
| 8 | FS-server stack 528 bytes short | the grant exceeds the deepest handler recursion | no (a property of codegen) | none | a wall-clock ceiling, misread for a day |
| 9 | `nifefs::write_image` accepted a NUL name | every accepted name reads back as itself | yes, cheaply | 2 harnesses, neither states it | the fuzzer, under a minute |
| 10 | `dtb::node_reg` indexed past its cell stack | the walker is total | no (the BMC wall, named in advance) | 4 harnesses, all on the leaves | reading, while writing the fuzz target |
| 11 | `dtb::Region::end` overflowed | `start + size` does not wrap on a hostile memory map | yes | none for `end()` | the fuzzer, ten minutes cold |
| 12 | `gpt::parse` accepted a backup-boundary equality | no accepted table puts a usable block in the backup array | yes | 9 harnesses, none states it | mutation testing |
| 13 | `login::mint()` cspace slot leak | `mint` returns the cspace to its entry occupancy | yes, after an extraction | none | a 9th login answered DENIED |
| 14 | `login::connect()` rendezvous leak | a served channel leaves no global registry slot behind | yes, after an extraction | none | an unrelated later test, "512 live at once" |
| 15 | x86_64 `boot_cpu_id` answered "which core am I" | `boot_cpu_id` is the core that booted, on every arch | no (a cross-arch contract; rung 1 territory) | none | a test failing half the time |
| 16 | x86_64 PVH `pci::place_bars` trusted any nonzero BAR | a BAR is usable only inside the window this kernel mapped | yes, and a proof would have proved the wrong predicate | 5 pci harnesses | attaching a real NVMe device |
| 17 | nine roadmap statuses wrong in both records | the roadmap describes the tree | not a code property | the gate compared two records; the tree is a third | a sweep, after a lane burned its budget |
| 18 | a retracted benchmark quoted as current | an attributed block quote still exists in its source | not a code property | no gate read quote targets | `script/citations`, built for it |

Six rows deserve more than a cell.

### 7. The timer re-arm drift, and the one property this study can price exactly

The defect. `kernel/src/arch/aarch64/timer.rs`'s module header says it plainly: *"We shipped this
bug and then measured it."* Re-arming with `TVAL`, a relative countdown, means every tick starts its
countdown late by however long the trap took, and the lateness is never recovered. Measured under
QEMU: 100 Hz configured, about 70 Hz observed. Thirty percent of the kernel's preemptions, gone,
with nothing saying so.

The property. `next_deadline = fired_deadline + interval`, except that a deadline in the
past re-anchors and costs exactly one fire rather than a backlog.

Provable? Yes, and the proof already exists in this tree, over a different subsystem.
`crates/timetable`'s `next_after(prev, period, now)` is the same three arguments and the same law, and
its five harnesses state it: `a_fire_is_strictly_in_the_future`, `a_fire_is_at_least_one_whole_period_on`
(`next >= prev + period`, which is the drift law), and `a_stall_costs_one_fire_and_not_a_backlog`
(`next <= now + period`, which is the re-anchor). Put `rearm`'s twelve lines of arithmetic behind that
function and the milestone-6 defect becomes unrepresentable.

The honest qualification, and it matters. The *original* defect was one register below the
arithmetic: `TVAL` versus `CVAL`, relative versus absolute, with no `next` computed at all. A pure
function proof does not see a register's semantics, so it would not have caught the first form.
It would have caught the second. Milestone 62 injected exactly that second form, `sbi_set_timer(now +
interval())` with the grid still maintained correctly beside it, and `notes/instruction-clock.md`
records that the instruction-count instrument went green with every number byte-identical to a
clean run, because that instrument compares each arrival against the deadline that fired, which a
re-anchoring kernel satisfies forever. A `timetable`-shaped proof is the one form of evidence in this
tree that would have gone red on that injection without needing an emulator.

Judgement, and it is the strongest one in this study: this is a defect a proof would have caught,
the proof is already written, and the only thing missing is that the timer does not call it. That
is the Phase-2 extraction pattern `memory_regions`, `inter_process_communication`, `dma_validator` and `paging::domain` all
took, applied to a subsystem that has not had it yet.

### 5 and 6. The two concurrency defects, and why one tool found one of them

Both are races. Neither is a Kani property as this tree uses Kani, and `notes/verification.md` says
so before either was found, in its list of what a green check does not mean: *"Concurrency is the
sharpest edge of this limit: every queue and endpoint proof here is single-threaded, and the
wake-before-switch-out race (notes/intrusive-queues.md) lived precisely in the SMP interleaving those
proofs cannot see. Green harnesses and a real race coexisted; the flaky test found it."*

That paragraph is the model for how this project should talk about its proofs, and it was written
before this retrospective existed.

The seqlock defect (#6) is the corpus's one unambiguous green for a formal method, and the method
is loom rather than Kani. The writer claimed the sequence with a compare-exchange and then wrote the
data with nothing ordering the claim ahead of them. Three of four harnesses failed on the first run.
The instructive part is the table of fixes that do not work: `Acquire`, `AcqRel` and `SeqCst` on the
compare-exchange all still fail, because an ordering on an RMW orders accesses around itself and what
a seqlock writer needs is a store-store barrier between its claim and the stores that follow. Nothing
else in the tree could see it: the ten host tests, the kernel clock tests on both ISAs,
`script/verify`, `script/fuzz` and `script/undefined-behavior-check` all passed before the fix and all
pass after it.

What this says about coverage. The class Kani cannot reach is precisely the class loom can, and
this project noticed and built the second tool. Five of the eighteen defects are concurrency and
`crates/` now holds five loom-checked protocols. That is the verification programme working as a map
even where it did not work as a net.

### 9, 11 and 12. Three defects in crates that already had harnesses

These are question 3's interesting outcome, and each one fails for a different reason.

`nifefs::write_image` and the NUL name. `nifefs` has two harnesses, both about the *reader*:
`the_validation_implies_reads_slice_is_in_bounds` and `a_short_image_is_refused_not_indexed`. The
crate's reader is proved total with no bound on the image size at all, which is strong. The defect was
in the writer: a name containing a NUL went into a NUL-padded field, so `"a\0b"` was written and read
back as `"a"`, and two names agreeing up to their first NUL collapse into one entry. Nothing panicked,
so no totality proof could have found it. `notes/fuzzing.md` states the reason correctly: *"it is a
property violation, and the property had never been written down."* The property is one line
(`for every accepted name, read(name) returns what write stored`) and it is Kani-shaped. Worklist.

`dtb::Region::end`. Four harnesses, all on `be32`/`be64`. `end()` was `self.start + self.size` on
two `u64`s straight out of the blob, and the kernel calls it on every RAM region the device tree
declares, in a dev-profile build with overflow checks on. A `/memory` node claiming
`start = size = u64::MAX` panics on the boot path; in release it wraps, which is worse. Nobody had
thought to look at `end()`. That is not a limit of the tool, it is a gap in what anyone chose to
state. Worklist.

`gpt::parse`, the best near-miss in the corpus. Nine harnesses, including
`create_never_lays_out_a_table_parse_would_reject`, which sounds exactly like the property the defect
violated. It is not. That harness asserts that whenever `Gpt::create` succeeds, its output satisfies
the four inequalities `parse` checks, and it spells those inequalities out as the code writes them
(`h.last_usable_lba + span < h.alternate_lba`). The defect was a wrong-*accept* in `parse`: `>` where
`>=` was needed at the backup-array boundary, putting one usable block inside the backup entry array
where a partition would overwrite it. `create` makes tight tables, so a create-then-parse harness
never reaches the equality, and a harness that restates the reader's own inequality cannot detect that
the inequality is wrong. `notes/mutation-testing.md` draws the same conclusion from its own side:
*"an exhaustive sweep proves rejection, not rejection for the right reason."*

This is verification.md's failure mode 1, on the record, in a shipped crate: a proof proves what
you asserted, not what you meant. The fix is to state the property absolutely rather than relative
to the writer: *no accepted table places a usable block inside either entry array*. Worklist.

### 13 and 14. Resource accounting, a class nothing here is aimed at

`login::mint()` never freed its own copy of the caretaker's construction-region capability on the
success path. `login`'s cspace is fixed at 16 slots and 8 are spent at rest, so the leak bounded the
service to exactly eight successful logins ever, and a ninth correctly-authenticated login was
answered `DENIED`, indistinguishable from a wrong password. `login::connect()` had the same shape one
level out: two rendezvous objects per connect, retyped from a shared region and released with
`cap_delete`, which drops login's reference but leaves the kernel object in a global 512-slot registry
until the region is destroyed. Roughly twenty-nine test-run connects were enough to fail an unrelated
later test with "out of rendezvous points: 512 live at once".

Both are "every path through this function returns the resource to its entry occupancy". That is a
provable property and nothing in this tree proves it for any function. It needs the slot bookkeeping
lifted out of `components/src/login.rs` into a crate, which is a real extraction rather than a harness, and
it is the class most likely to recur: sixteen slots is a small ceiling and every service program has
one. Worklist, and the largest item on it.

### 16. The defect a proof would have got wrong

`pci::place_bars` trusted any nonzero BAR as already placed and already mapped. True on the two
device-tree architectures, where nothing runs before this kernel. False on x86_64's PVH boot, where
QEMU resets `-device nvme`'s BAR0 to a live address of its own choosing, unrelated to the window
`mmu::map_everything` actually mapped, and the first register touch page-faults.

`crates/pci` has five harnesses, one of which is `ecam_offset_stays_inside_the_window`. A sixth
stating "a BAR this function accepts is inside the mapped window" would have been written, in
2026-07, against the predicate the code used, because that predicate was believed. A proof would
have encoded the bug. The thing that found it was attaching a real device to a real boot path, which
is the ranking function at the top of `AGENTS.md` doing its job.

Recorded because a study of proofs that only lists cases where a proof would have helped is
propaganda.

### 17 and 18. The non-code defects, which fail the same way one level out

Nine milestones (81, 82, 85, 94, 96, 100, 101, 109, 110, 113) had merged implementation pull requests
while both roadmap records read `NOT-STARTED`. `script/roadmap --check` was green throughout, because
it compares two records and the tree is a third record nothing compares against. A developer spent
its entire budget re-running mutation tests for milestone 85, which had merged hours earlier.

A block quote in `design/roadmap/74-cycle-counters.md`, attributed to `notes/benchmarks.md`, quoted
arithmetic that milestone 101 had re-measured and retracted. The roadmap was citing a retraction as
the current record and nothing could see it, because a prose block quote attributed to another file is
a citation no gate resolved. `script/citations` exists because of it.

The shape is identical to the code cases and is worth naming as one thing: a checker compares two
artifacts and the truth lives in a third. `parse` against `create`, a roadmap file against a README,
a quote against nothing at all. The generalisation for the proofs is that a harness written against
the code's own predicate is that shape exactly.

## What the proofs did catch

The corpus above is a list of escapes. This is the other half, and it is the reason the finding is
amber.

Two real defects, both found at harness-writing time.

- `dtb::be32` and `be64`. The original `at + 4` was a bare add on a `usize` taken straight out of
  an untrusted blob. Trying to state `be32_is_total` made it fail, and the readers were hardened to a
  checked add returning `Truncated`. This is a panic on the kernel's boot path, reachable from
  firmware, in the first parser that runs on both ISAs. Real, and found by the attempt to prove.
- `pci::intx_irq`. The pin-0 case underflowed and panicked in debug builds. Hardened with
  saturating arithmetic while writing `intx_irq_is_total_and_bounded`. The comment records the honest
  scope: every caller checks `pin == 0` first, so this is defence in depth rather than a live hole.

One proof obligation closed, marked honestly as not a bug. `paging::domain::grant_pages` gained a
wrap refusal because `an_enumerated_page_lies_inside_the_grant` cannot compute the grant's limit
without it. The comment says out loud that the first draft of itself overstated this: reaching a
wrapped address needs a `size` within a page of `2^64` and the frame allocator would run dry tens of
orders of magnitude earlier. What the check buys is totality, not a fix. That paragraph is the model
for how a proof-driven change should be described.

One specification falsified by the prover. In `crates/glob`, Kani came back in 42 seconds with a
counterexample to the *harness*, not the code: with a fully symbolic class body, `[!y]` is already a
negated class, so "`[xy]` and `[!xy]` are complements" is false when `x` is `!`. `notes/verification.md`
keeps it because *"the reflex on a red harness is to suspect the code."* A prover that corrects the
person writing the specification is doing something no test can.

And the counterfactual that cannot be measured. Nothing that a shipped harness states has
afterwards turned out to be false. The capability model's twelve harnesses, the DMA validator's seven,
the untyped-region no-double-free pair, the address arithmetic, the IOMMU domain's page set: all of
these underwrite security claims that would otherwise rest on argument, and none has ever needed
correcting. That is either a strong result or an unfalsifiable one, and the reverse pass below is the
only thing that tells them apart.

## The reverse pass

The question that keeps this study from being able only to confirm: which harnesses prove a property
that could plausibly have been false?

### Harnesses that could not have failed

Judgement, harness by harness, on the ones that look weakest.

`capability::subset_is_reflexive` is the clearest case. It proves `a.is_subset_of(a)` for every
`a`, and `is_subset_of` is `self.0 & !other.0 == 0`. Reflexivity is then `a & !a == 0`, a tautology of
the two operators. Work through the plausible ways to get `is_subset_of` wrong and none of them
break it: reversing the operands passes, writing it as `self.0 & other.0 == self.0` passes, using the
wrong mask constant passes. There is no implementation error this harness distinguishes from the
correct one. It is documented as *"the reflexive base case of the derivation order"*, which is a fair
description of what it is for and not a claim that it constrains anything.

Its siblings are not in this category. `subset_is_transitive` is the property that licenses a flat
check instead of a derivation-tree walk, and it is exactly the sort of thing a hand-rolled rights
check gets wrong. `from_bits_cannot_forge_a_right` quantifies over an attacker-controlled syscall
register. `derive_never_widens_rights` runs over the real `CapabilityTable::derive`.

**Restated-implementation harnesses.** `be32_reads_big_endian_when_in_bounds` asserts that an
in-bounds read equals `bytes[at..at+4]` most significant byte first, which is the function's body
written a second way. `align4_rounds_up_to_a_multiple_of_four` is one arithmetic identity. Neither is
worthless (a byte-order flip is a real error and the assertion is not literally the same expression),
but both are much closer to a unit test that Kani happens to execute than to a proof of something
anyone could have got wrong at scale. Their siblings `be32_is_total` and `be64_is_total` are where the
value is, and they earned it by failing.

**The parity family.** Twenty-six of the 145 harnesses live in `crates/paging`, and six properties
appear three times each, once per ISA (`aarch64.rs`, `sv39.rs`, `x86_64.rs`). That is not padding: Sv39
has three levels where the other two have four, and the leaf codecs are genuinely different formats,
so `index_is_always_in_bounds` is a different theorem in each file and rule 5 requires all three. It
does mean the raw harness count over-represents distinct properties by about twelve, and anyone
quoting 145 as "145 things proved" is quoting 133 at best.

**The pattern behind all three.** The harnesses that could not have failed are the ones stated over a
single operator or a single expression. The ones that could are stated over a *composition*: a table,
a walk, a chain, a round trip, a second implementation. `gpt::crc32_matches_its_bitwise_definition` is
the shape done right, a table-driven implementation checked against a bitwise one, and
`overlap_is_exactly_sharing_a_block` says in its own comment that it is *"stated as the **definition**
(the intersection is non-empty) rather than as the implementation, so the proof is not the code
compared with itself."* That sentence is the whole reverse pass in one line, and the tree already
knew it.

### What actually separates a load-bearing harness from a decorative one

Not the property. The **falsification**, and the tree's own rule already says so:
*"A harness that cannot be made to fail is not evidence."*

The harnesses with a recorded falsification are the ones this study can vouch for without judgement.
Milestone 35's whole set was broken on purpose before being believed, and one falsification corrected
a claim in the code (soundness rested on `grant_pages` flooring, not on the partial-page guard the
comment pointed at). The calendar's two central properties were broken and both harnesses caught it in
under half a minute. `glob`'s complement claim was falsified by the prover itself. `memory_regions`'
loom side carries a named falsification witness.

**That is a minority of the 145.** No gate requires a falsification record, no naming convention marks
one, and there is no way to enumerate them except by reading. That is rung four behaving like rung
four.

The second separator is **non-vacuity**. An assumption or a bound can silently empty a harness's input
set and a vacuous harness reports `SUCCESSFUL`; `kani::cover!` is the one check that catches it. There
are **19 `cover!` sites, in four crates** (calendar 7, paging 5, dma_validator 5, glob 2). The other
twenty harness crates have none. Every harness that constrains its inputs with `kani::assume` and
carries no `cover!` is a harness whose input set nobody has confirmed is non-empty, and there are many
of them: `jh7110_trng`'s three all assume, `timetable`'s five all assume, `network_time_protocol` and
`credential_protocol` assume throughout.

Not a claim that any of them is vacuous. A claim that **nothing in this tree would say so if one
were**, which is the same shape as every other finding here.

## A hole found while counting: three harnesses that nothing runs

`script/verify` proves a **hand-kept crate list**. On 2026-08-16 milestone 125 found `multicast_dns_protocol` had
landed with three harnesses and never been added to it, so the suite had never run them, and the way
it showed up was the suite going green *faster*. The file's own comment now says a missing row *"is
the one way this table can make the proofs wrong"*.

**It has happened again.** `crates/jh7110_trng` is a workspace member (`Cargo.toml` line 35) carrying
three harnesses, and the string `jh7110_trng` appears nowhere in `script/verify`. The shard table
holds 23 crates summing to 140 harnesses; `crates/` holds 143 across 24. Nothing runs
`a_lockup_bit_is_never_overridden`, `ready_requires_rand_rdy_and_carries_the_words_untouched`, or
`neither_bit_set_is_always_not_ready`, and they guard the entropy source on the board this project's
first-silicon milestone runs on.

The counted-claims gate cannot see it. `harness-crates` and `kani-harnesses` are **`count-at-least`
floors** against prose, so they check that the tree has at least as many harnesses as a note claims;
neither compares the tree against the shard table. The multicast_dns_protocol case was caught only because a prose
claim happened to be one crate off, which is luck rather than a mechanism.

`vendor/redoxfs`'s two harnesses are also unrun, and that one is **fine**: `script/lint`'s
`_harness_hits` docstring says so on purpose, *"counting them would make the number describe a suite
nobody executes"*. That is a decision. `jh7110_trng` is an omission, and the difference is that
nobody wrote anything down.

**Not fixed here, and the reason is not tidiness.** These three harnesses have never been run, so
nobody knows whether they pass. Adding the row from a lane that cannot run the prover risks turning
`main` red on a check that takes forty minutes. It wants a lane that runs `cargo kani -p jh7110_trng`
first, adds the row with a measured cost column, and then closes the gap properly: a gate comparing
`_harness_hits()`'s crate set against `script/verify`'s table, which is rung two and turns this from a
recurring accident into an impossible state. **Worklist.**

**Closed on 2026-08-30 by milestone 193, and the crate has since been renamed.** The three harnesses
were run (all pass), the row was added, and `script/lint` grew the check that makes it the last time.
The names above are the ones the count was taken under: the crate is `jh7110_entropy` now
(calef ratified `jh7110_entropy_source` on 2026-09-13 and replaced it with `jh7110_entropy` later
the same day, performed 2026-09-14), so `script/verify`'s table carries it under that spelling and
the measurement above
stays traceable to the run that produced it. Item 1 of the worklist below is done.

## The worklist

Each item names the defect that motivates it. The first two are the ones this study would spend its
next lane on.

| # | Harness or gate | The defect it answers | Shape | Cost |
|---|---|---|---|---|
| 1 | A gate comparing the harness-crate set against `script/verify`'s shard table | `multicast_dns_protocol` 2026-08-16, `jh7110_trng` today | a check in `script/lint`, plus the missing row once it is known to pass | small, and it is rung two |
| 2 | Extract the timer re-arm law and point both ISAs' `rearm` at it | timer drift, milestone 6: 100 Hz became 70 Hz | Phase-2 extraction; `crates/timetable`'s three harnesses already state the law | small, and the proof is written |
| 3 | `every_accepted_name_reads_back_as_itself` in `nifefs` | `write_image` accepted a NUL name, 2026-08-02 | one harness over the writer, which is the half currently unproved | small |
| 4 | `no_accepted_table_puts_a_usable_block_in_an_entry_array` in `gpt` | `Gpt::parse`'s backup-boundary wrong-accept | state the property absolutely, not relative to `create` | small |
| 5 | `region_end_is_total` in `dtb`, and a sweep for its siblings | `Region::end` overflowed on a hostile memory map | the `be32` hardening applied to every `pub` arithmetic helper on blob-derived fields | small |
| 6 | Lift `login`'s cspace bookkeeping into a crate and prove occupancy is restored | `mint()`'s 8-login ceiling; `connect()`'s rendezvous leak | a real extraction, the largest item here | a lane |
| 7 | A loom crate over the PLIC enable-bit read-modify-write | the lost update the arch audit found and fixed by inspection | the `work_steal_slot` pattern, which is precedented | modest |
| 8 | A convention that marks a harness's falsification record | the reverse pass could not enumerate them | naming or an attribute; **an architect's call**, since it is a convention | a decision, not a lane |
| 9 | `cover!` guards on the assuming harnesses that have none | twenty of twenty-four harness crates carry no vacuity guard | mechanical, one per harness that assumes | modest |

Item 8 is a proposal rather than a task, and it is the only thing here that is an architect's rather
than a lane's: it is a tree-wide convention, and this note has no authority to mint one.

## EXAMPLES

Re-derive every number in this note. Run from the repository root.

Harnesses, the way to count them that is not wrong:

```
$ grep -rn "^\s*#\[kani::proof\]" --include="*.rs" . | grep -v target | wc -l
     145
```

The looser pattern most readers reach for first, and why it over-counts by three:

```
$ grep -rn "kani::proof" --include="*.rs" . | grep -v target | wc -l
     148
$ grep -rn "kani::proof" --include="*.rs" . | grep -v target | grep kani-lint-shim
scripts/kani-lint-shim/kani.rs:28://! `kani::proof_for_contract`, ... are all real and
scripts/kani-lint-shim/kani_attributes.rs:6://! next door has no choice about its name, ...
scripts/kani-lint-shim/kani_attributes.rs:23:/// `#[kani::proof]`: keep the function, ...
```

Packages carrying harnesses, and which ones the suite actually proves:

```
$ grep -rln "^\s*#\[kani::proof\]" --include="*.rs" crates/ vendor/ \
    | sed -E 's|^(crates\|vendor)/([^/]*)/.*|\2|' | sort -u | while read c; do
        grep -q "^$c	" script/verify || echo "in no shard: $c"
      done
in no shard: jh7110_trng
in no shard: redoxfs
```

Source lines the harnesses can and cannot reach:

```
$ find kernel/src -name "*.rs" | xargs wc -l | tail -1
   64818 total
$ find . -name "*.rs" -not -path "./target/*" -not -path "./vendor/*" | xargs wc -l | tail -1
  206728 total
```

Every harness name, which is the input to the reverse pass:

```
$ for f in $(grep -rln "^\s*#\[kani::proof\]" --include="*.rs" . | grep -v target); do
      grep -A3 "^\s*#\[kani::proof\]" "$f" | grep -oE "fn [a-z0-9_]+" | sed 's/fn //'
  done | sort
```

Vacuity guards, by crate:

```
$ grep -rn "kani::cover" --include="*.rs" crates/ | sed -E 's|crates/([^/]*)/.*|\1|' \
    | sort | uniq -c | sort -rn
   7 calendar
   5 paging
   5 dma_validator
   2 glob
   1 work_steal_slot
   1 memory_regions
   1 memory_corruption_canary_gate
```

The last three are loom crates, not Kani harness crates, which is why the figure quoted above is 19
rather than 22.

## BUGS

**A retrospective cannot prove a counterfactual, and every judgement here is marked as one.** "A proof
would have caught this" is an argument. The nearest thing to a measurement in the whole note is item 7
of the corpus, where the property is already proved over already-written code in `crates/timetable`,
and even that carries a qualification: the defect's original form was a register choice below the
arithmetic, and a pure-function proof would not have seen it.

**Survivorship runs both ways and the reverse pass only partly fixes it.** Defects the proofs
prevented never entered the record, which is why the two harness-writing catches (`dtb`, `pci`) are
the tip of an unmeasurable quantity. Defects nobody has found yet are not in the corpus either. This
note can say what the proofs did not catch; it cannot say what they did.

**The corpus is what the tree wrote down, and the tree writes down what it noticed.** Eighteen entries
is not eighteen defects; it is eighteen defects somebody found interesting enough to record. The
selection bias runs toward defects that were hard, because a defect fixed in five minutes leaves no
note. That bias runs *against* the proofs' case: easy defects are the ones a harness is most likely to
have caught silently.

**Ten of the eighteen entries are single-sourced.** Where the corpus rests on one note's account of
itself (`notes/visionfive2.md`'s bench stops most of all, whose fourth stop's conviction was overturned
by its own fifth stop the next day), this study inherits that note's confidence and no more.

**The reverse pass is a reading, not a measurement.** "Could this property plausibly have been false"
has no gate behind it. A proper version is mutation testing pointed at the harnesses, which
`notes/mutation-testing.md` explicitly excludes today (`mod proofs` and `mod verification` are excluded
from the mutation run, correctly, because they are not the test suite's to defend). Mutating the
*subject* code and asking which harness goes red would answer this question mechanically, and nothing
in the tree does it.

**This note has no gate and produces no artifact the build checks**, which is milestone 191's own
`BUGS` entry and remains true. Its worklist is nine rows of prose, and prose is rung four. The one
thing that stops it going the way milestone 94's inventory went is that item 1 is a gate, item 8 is a
decision for an architect, and the rest name a crate a reader can grep for.

**The counts will drift.** They were taken on 2026-08-30 at base `f1f138a8`. The EXAMPLES section
exists so the next reader re-derives them instead of quoting this table, which is the failure
`notes/fs-server.md` records about its own injector counts and the roadmap records about "112+".

## See also

- `notes/verification.md`, which is where the harnesses are explained and which already states, before
  this study existed, most of the limits it confirms.
- `notes/interleaving.md` for the class Kani cannot reach and loom can, including the seqlock defect.
- `notes/fuzzing.md`, whose "What fuzzing finds that Kani does not" section is the same question this
  note asks, restricted to parsers.
- `notes/mutation-testing.md` for the third tool and the gpt defect.
- `notes/arch-audit.md` for the compensating control where no prover reaches at all, and for the two
  defects it cleared and the one it fixed.
- `notes/exceptions.md`, `notes/intrusive-queues.md`, `notes/instruction-clock.md`,
  `notes/fs-server.md`, `notes/visionfive2.md`, `notes/load-sensitive-assertions.md` for the defects
  themselves.
- `design/decisions/14-project-direction.md` for the thesis this is measured against, and
  `design/decisions/76-roadmap-status-versus-tree.md` for the record-level defect with the same shape.
