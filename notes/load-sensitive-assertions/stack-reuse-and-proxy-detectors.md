# Stack reuse and proxy detectors: the fifth round, 2026-08-17

*(An appendix of [notes/load-sensitive-assertions.md](../load-sensitive-assertions.md), which holds
the register and the rules. Moved here on 2026-09-24 and tightened; pull request #1211's first
commit has the text verbatim.)*

## The fifth round, 2026-08-17: the assertion that could not catch the bug in its own name

This round was briefed on the block's three negative-discrepancy assertions.
[The fourth round](miss-taxonomy-and-clockless-loops.md) had already found them finished and said
so. The brief was written from the roadmap block for milestone 78 (the load-sensitive assertions).
Its gate line still asked for "three small changes with three arguments", and its evidence table
still read as a worklist. So the fourth round's own complaint (a reader coming to the block first is
sent to three finished sites) had a second victim. This time the fix went into the table rather than
into the note. The block's evidence rows now carry a disposition column, and its gate line names the
icount instrument, which is what is actually left. A note saying the table is stale does not stop
the table being read.

Checking the three a second time produced one record correction and one defect.

### The reaper test could not catch milestone 6's leak, and an injection proves it

`a_finished_thread_is_reaped_and_its_memory_returned` exists for one defect of milestone 6 (threads,
the context switch, and preemption): stack address ranges that are not reused. Then an L2 and an L3
page table accumulate per 2 MiB of address space consumed, forever. The test asserted that a second
batch of eight threads costs zero extra frames. The first round re-aimed that assertion from `==` to
a waited `<=`, arguing that "sensitivity to the milestone-6 bug is unchanged".

The sensitivity was unchanged, and it was never there. Deleting the `FREE_STACK_VAS` push from
`KernelStack::drop`, which is that defect exactly, passed the entire aarch64 leg, this test included.
The arithmetic says why, and it is not luck:

- a slot is `STACK_SLOT_SPAN`, 7 pages, 28 KiB;
- eight of them consume 224 KiB of fresh address space;
- a leaked page table costs a frame only when the bump crosses a 2 MiB L3 boundary;
- 224 KiB is 11% of one table's span.

So the frame count sees the defect only when the batch happens to straddle a boundary. And it is
worse than 11% random. Where `NEXT_STACK_VA` stands when this test runs depends on how many threads
the tests before it spawned, which is fixed for a given tree. So for any given tree the assertion
either always catches the defect or always misses it, and unrelated code upstream decides which.
That is a detector whose sensitivity is set by its neighbors. It is the family's theme from a new
direction: not a neighbor causing a false failure, but a neighbor deciding whether a real failure
is visible at all.

The frame count was always a proxy for reuse. The fix is to assert the mechanism. Every thread in
the second batch must land below the watermark that stood before the batch began, which is what
reusing a dead thread's range means. Each thread reports its own `sp`. A test cannot read the stack
out of a thread it is simultaneously waiting to see reaped, and a local's address is a stack address
with no race in it.

The failure direction is one-way, so this is not a new global exposure. `NEXT_STACK_VA` moves only
when the free list is empty. A neighbor spawning inside the window can only raise the watermark,
and raising it makes the claim easier to satisfy. Contention cannot fail it. That is the test the
first round applied to `used() <= before`, applied to the half that was still a proxy.

The frame assertion stays, scoped to the leak it can actually see: a per-thread frame the reaper did
not return. The comment claiming its milestone-6 sensitivity was corrected in place rather than
deleted. What it said was true about the arithmetic it defended and false about the bug.

#### The first version of that assertion joined the family

It asserted the watermark claim for all eight threads of a batch, and failed on a clean kernel on
the first full gate run: thread 1, two slots above the watermark, with nothing injected. The two
runs before it (the defect injected, then reverted) had both agreed with it. That is how long this
family usually takes to look settled.

The diagnosis is the register's own, turned on its author. The watermark is the high-water mark of
`FREE_STACK_VAS` running dry. That is a fact about how many threads were alive at once, not about
whether dead ones are reused. Eight sequential spawns need as many slots as the reaper falls behind
by, and that number is a scheduling outcome. A batch whose threads are reaped later relative to
spawning legitimately needs more slots than the previous batch did, and bumps the watermark. So the
assertion was written against something wider than the property, and load decides whether the extra
width shows. That is the first-round diagnostic verbatim, committed while fixing the site it
diagnoses.

The fix is to make one thread the unit, because one thread cannot exceed a high-water mark that
eight have just set. After a batch of eight has been spawned and reaped, `FREE_STACK_VAS` is provably
non-empty. Over the batch, eight pushes against eight pop-or-bump decisions leave the list at its
starting size plus the number of bumps, and a bump only happens when it was empty. So the list holds
at least one slot. Every slot in it was handed out below the watermark, and a single pop cannot drain
what eight deaths just stocked. The probe therefore lands below the watermark on any kernel that
reuses at all, and at the watermark on one that does not.

Two lessons, and the second is about method rather than stacks. A one-way failure direction is
necessary and not sufficient. This assertion had one (a neighbor can only raise the watermark) and
was still wrong, because it bounded the wrong quantity. And an injection that fires proves only that
the assertion can fail, never that it fails for the right reason. The injected run went red and the
clean run went green, while the assertion was still measuring concurrency. Only a second clean run
under different scheduling said so. That is the argument for
[the second round's load recipe](measurement-windows-and-the-load-recipe.md), from a third direction.

### Three panics that could still print the impossible quantity

The three converted sites all wait on a one-directional bound, then re-sample the allocator to
format the panic. The first round re-aimed the measurement and not the message. The message is what
a person reads at 2 a.m. deciding "known or real".

- `user/tests.rs` prints `used() as i64 - before as i64`. A genuine timeout whose frames land in the
  gap between the wait giving up and the panic being formatted still prints "-19 frames did not come
  back". That is the exact string this milestone is named for, now emitted by a form that can no
  longer fail for that reason. A reader who trusts the sign re-runs the whole 2026-08-03
  investigation.
- The two `sched.rs` sites use `saturating_sub`, which is worse. It clamps the impossible quantity to
  "leaked 0 frames" and removes the sign that gave the original bug away.

All three now report the observation the predicate actually decided on. `wait_for` re-evaluates
once past its deadline, so a `false` return leaves the captured sample strictly on the failing side
of the bound. The printed count is positive by construction, with no cast and no clamp.

### The injections, and what each one settled

Every injection was reverted. All are recorded, including the two that did not reach their target,
because the failed injections produced the useful facts.

| injection | what it is | result |
|---|---|---|
| delete the `FREE_STACK_VAS` push in `KernelStack::drop` | milestone 6's leak, exactly | **whole aarch64 leg green.** The defect is invisible to the suite |
| the same, against the first (eight-thread) reuse assertion | | **red on thread 0**, naming both addresses |
| restore the push, keep that assertion | | green, and misleading: two runs agreed with an assertion that was measuring concurrency |
| nothing injected, full gate | the run that caught it | **red on thread 1, on a clean kernel.** The eight-thread form was wrong; see above |
| the VA push deleted again, against the single-thread probe | | **red**, sp `…6ff80` against watermark `…69000` |
| skip `untyped::destroy` in `AddressSpace::drop` | a dead space returns nothing | never reached the target test: exhausted memory ~370 tests in, at "no stack region for the net client" |
| leak one frame per `AddressSpace::drop` | a dead space returns all but one frame | caught by `reclaim_frees_an_unbound_address_spaces_region` first, `left: 61501, right: 61502` |
| leak one frame per dead user thread, in `finish_switch`'s reap arm | narrowed to the aspace test's own subject | caught by `destroy_force_kills_a_runaway_and_reclaims_its_region` first, `left: 52403, right: 52404` |
| leak four frames inside the aspace test's measured window | the defect arranged where only this assertion can see it | **red**, "four user address spaces came and went and **4** frames did not come back" |

Three things from the misses are worth more than the hit.

The whole-region injection is too coarse to test anything downstream of it. A leak injection aimed
at a late test has to be one frame wide.

The exact-equality frame assertions the fourth round deliberately left alone are genuinely sensitive.
Two of them (`reclaim_frees_an_unbound_address_spaces_region` and
`destroy_force_kills_a_runaway_and_reclaims_its_region`) each caught a one-frame leak, ahead of the
target test both times. That measurement backs the fourth round's argument that a synchronous
bracket keeps its `==`, which until now was only an argument.

And it says something about the aspace test that three rounds of prose did not. An earlier, exact,
synchronous assertion catches a leak in the kernel path it guards before this test runs, in both
narrowings tried. Its `used() <= before` is not the tree's first line of defense against that defect
and probably never was. What it uniquely covers is a leak that appears only after a user thread has
faulted and been reaped four times over, which no `reclaim_frees_*` bracket stages. Know that before
spending another round on it. The final row proves that assertion directly: the defect arranged
inside its own window, the wait timing out, and the new message naming the injected count exactly
rather than a figure re-sampled after the fact.

### What this round did not do

The icount instrument was still recommended and still not built. That was unchanged from the fourth
round, and it remained the whole of the block's "What is left". (Built the same day: see
[the icount claims](the-icount-claims.md).)

The other sites in the scope note were not audited, again. This round adds a third grep to the
reading order, beside the fourth round's two: a frame count standing in for a mechanism. The two
greps for a global baseline and for a clockless loop find assertions that fail when they should not.
This one finds assertions that pass when they should not, and nothing in the note had looked for
those.
