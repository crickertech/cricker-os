---
status: NOT-STARTED
raised: 2026-08-28
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 189. A page-frame budget per architecture, so the ledger constrains all three

Minted 2026-08-28, calef, out of his own question about why x86_64 has no
page-frame budget. It has one; it has the same one, which is the problem.

The ledger already prints, on every leg, the figure this needs. Splitting one
constant into three is not a design fork, and the shape it splits into is the one
`bench/baseline-aarch64.txt`, `bench/baseline-riscv64.txt` and `bench/baseline-x86_64.txt` already
use for the icount tripwire.

## What is wrong

`SUITE_PAGE_FRAME_BUDGET` (`kernel/src/testing.rs`) is a single number, and
`report_page_frame_ledger` is called unconditionally from the suite's closing summary. There is no
`cfg(target_arch)` anywhere in that file. So every architecture is measured, and all three are
measured against a ceiling that was fitted to one of them.

The value has always been set from aarch64, the tighter of the two the ledger's own history
discusses. Its doc comment is one of the most carefully maintained records in this tree, a dated
entry per raise with the measurement behind it, and **not one entry mentions x86_64.**

That would be a nuisance if the three suites were the same size. They are not:

| leg | tests run | skipped |
|---|---|---|
| aarch64 | 306 | 1 |
| riscv64 | 309 | 1 |
| **x86_64** | **194** | **57** |

And the skips are not a random 57. They are `fs_service::NO_FS_SERVER` (vendored RedoxFS's
unconditional `aes` dependency does not compile for `x86_64-unknown-none`, which milestone 164
owns), `NO_STD_EXERCISER` (no `x86_64-unknown-nife` target yet, which milestone 184 owns), plus
`NO_RTC` and `NO_UART_PAGE`. Those name the heaviest and longest-lived frame consumers in the
suite: the file servers, the `std` farm, and the services that keep a session's scratch for the
rest of the boot. The tests that do not run on x86_64 are disproportionately the ones that keep
memory.

Both named skip causes have since closed, and the 57 above is the 2026-08-28 count, kept as
measured. Milestone 164 made `redoxfs_server` build for x86_64 on 2026-09-01, and milestone 184 built
`x86_64-unknown-nife` and its `std` farm on 2026-09-14, so `std_exerciser` now runs there. The file
servers still mostly skip on x86_64, for a different reason: no disk the FS service can find
(`design/roadmap/proposals/an-fs-service-with-no-disk-on-x86-64.md`). Re-count before relying on this
block's premise that x86_64 undercounts the heaviest consumers.

So x86_64 sits far under a ceiling it cannot approach, and the gate that reads green there is not
reporting a healthy leg. It is reporting that a number fitted to a different, larger suite was not
exceeded by a smaller one.

A leak that appeared only on x86_64 would have to be enormous before this noticed.

## The fact that makes it worse, and it is a small one

Nobody had written down what x86_64 keeps, and it was absent from the constant's history, from
`notes/frames.md`, and from every lane report searched when this milestone was minted.

Measured 2026-08-28, and it is worse than this milestone assumed. PR #546's lane reported the
figure while re-measuring the other two legs:

| leg | frames kept | against the shared budget |
|---|---|---|
| aarch64 | 22,217 | the number the constant is fitted to |
| riscv64 | 21,941 | 276 under aarch64 |
| **x86_64** | **7,514** | **roughly 14,700 frames of slack** |

So x86_64 keeps about a third of what aarch64 keeps, and sits nearly fifteen thousand frames
below a ceiling it is nominally gated by. The gate is not merely loose there. x86_64's retained
frames could triple and the ledger would still report green, which is not a tripwire in any useful
sense.

That single reading does not remove the need for the milestone's first step. One number is not a
baseline: it wants a second reading to establish whether the leg is stable, and it still cannot say
whether 7,514 already carries a leak, for the reason recorded at the end of this block.

## Why this is the same failure the tree already named

`notes/architecture-list-sweep.md` found eleven silent parity gaps and stated the pattern: what is
complete at three architectures is a Rust `match` the compiler pushed on, or a per-architecture file
whose absence a build notices; what is stale is a string in a shell script, a YAML step, a TOML
array, or a sentence in a note.

This is the same failure in a shape that sweep did not look for: **not a list missing an entry, but
one number doing duty for three suites.** It cannot be found by grepping for architecture names,
because the constant does not mention any. Worth recording as a widening of that note's own method
rather than only as a fix here.

## What to build

1. Measure x86_64 first, on a quiet machine, more than once, and write the number down before
   changing anything. If it turns out close to the aarch64 figure, the premise above is wrong and
   this milestone should be rewritten rather than executed.
2. Split the constant into three, one per architecture, selected by `cfg(target_arch)`. Keep the
   existing narrative doc comment as the shared history it is, and give each constant its own dated
   entry in that voice. Do not fork the prose three ways.
3. Choose each headroom on that leg's own evidence. The ledger's conventions are +15 ordinarily
   and +32 where a known flake perturbs the reading; whether x86_64 needs either is a question its
   own measurements answer.
4. Say in the failure message which architecture's budget was exceeded, since a single message
   naming a single constant will now be ambiguous.

## What this does not fix

- **It does not make x86_64's suite comparable.** 57 skips remain, and closing them is milestones
  164 and 184, not this. A per-architecture budget measures what that leg actually runs; it does not
  make that leg run more.
- **It does not find a leak, it makes one findable.** The ledger is a tripwire on retained frames,
  and a tighter ceiling on x86_64 only helps for growth that happens after the number is recorded.
- It cannot say whether today's x86_64 figure is already carrying a leak, because there is no
  earlier reading to compare against. The first measurement establishes a baseline and blesses
  whatever is already there, which is the honest cost of having gone this long without one.

## Index row

`SUITE_PAGE_FRAME_BUDGET` is one number for three suites and was fitted to aarch64. x86_64 runs
194 tests against aarch64's 306, and its 57 skips are disproportionately the heavy frame keepers
(`NO_FS_SERVER`, `NO_STD_EXERCISER`), so it sits far under a ceiling it cannot approach. Measured
2026-08-28: x86_64 keeps 7,514 frames against aarch64's 22,217, roughly 14,700 under a ceiling it
is nominally gated by, so its retained frames could triple and still read green.
