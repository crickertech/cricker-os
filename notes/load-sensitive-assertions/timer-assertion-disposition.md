# The timer assertion disposition, 2026-08-18

*(An appendix of [notes/load-sensitive-assertions.md](../load-sensitive-assertions.md), which holds
the register and the rules. Moved here on 2026-09-24 and tightened; pull request #1211's first
commit has the text verbatim.)*

## The disposition, 2026-08-18: one deleted, one told to say when it measured nothing

The acceptance run for milestone 62 (tests that assert on time) named the question and left it open.
The question was *"whether the wall-clock pair keeps a claim of its own on a machine where
`script/icount` is not run, not whether eight attempts should have been sixteen."* (See
[the first loaded acceptance run](first-loaded-acceptance-run.md).) This is the answer. It is two
answers, because the two assertions turned out not to be the same kind of thing.

It also found that a premise both the roadmap and the old note had written down was false.
Everything else rested on it, so it comes first; see "The instrument was blind to the drift bug"
below.

### The diagnostic this round adds: name the band, not the direction

The first round sorted this family by the direction of a failure. That diagnostic is still right and
still the one to reach for first. This round needed a second one, because both assertions here fail
in the honest direction and are still not fixable.

Ask what band of the measured quantity makes the assertion fire, then ask what else lands in that
band. An assertion is worth keeping only if its firing band contains the defect and nothing else.
Where the defect and the host produce the same band, no threshold inside it separates them, and the
choice is between a flake and a false pass. That is not a bound to tune. It is an assertion
measuring a quantity it cannot attribute.

The two here differ exactly on this, which is why they got different verdicts:

| | the band that fires | what else is in it | verdict |
|---|---|---|---|
| `ticks_arrive_at_the_configured_rate`, the re-arm law | any deviation from the grid | nothing: a miss re-anchors the grid and is excluded by construction | **kept, untouched** |
| the same test's retry budget | eight windows in a row containing a miss | a contended host, and it dominates | **stops failing; reports** |
| `the_handler_keeps_up_when_no_lock_is_held`, the taxonomy cut | a re-arm 1 to 2 tick periods late | a host deschedule of 1 to 2 tick periods | **deleted** |

### The instrument was blind to the drift bug, and the record said the opposite

Everything below depends on `script/icount` being stronger than what it replaces. So that was
checked before anything was decided, by injecting the defect rather than by reading the code.

The injection is the one the aarch64 module header has warned about since milestone 6 (threads, the
context switch, and preemption), applied to `rearm`. Re-anchor the grid from `now` instead of
advancing it from the deadline that fired: one line, on each ISA. It is the defect that made 100 Hz
configured into about 70 Hz delivered. Milestone 62's own BUGS section names it as the reason not to
delete these tests.

| | verdict |
|---|---|
| `script/test --arch aarch64` | **red**: `timer drift: 20 ticks moved CNTV_CVAL_EL0 off the grid`, and separately `a_long_critical_section_costs_a_tick` |
| `script/icount --arch aarch64`, as it stood | **green**, and *every number byte-identical to a clean run*: arrival min/mean/max 1008, handler mean/max 1056, `missed_ticks 0`, `early_arrivals 0` |

Byte-identical is the finding, not merely green. Claim 1 compares each arrival against the deadline
that fired. A kernel that re-anchors the whole grid arms the timer with the very word it records, so
it satisfies the comparison on every tick forever. Claim 2's span starts at the same place. Claim 3
counts misses, and re-anchoring produces none. No term in claims 1 to 3 moves at all when the clock
is drifting by 30%.

The roadmap block and the old note's acceptance section both stated that `script/icount` "asserts
zero missed ticks... which is strictly stronger than either of the two wall-clock assertions". That
was true of the handler-latency assertion and false of the drift one. The disposition below was
about to be made on the strength of it. The correction is in the block and in
notes/instruction-clock.md as well as here.

So the instrument got the claim it was missing, as claim 4. Over the sample window, the armed
deadline advanced by exactly one interval per delivered tick. It needs no retry loop, which is claim
3 paying for itself. A miss is the only thing that legitimately re-anchors the grid, and
`missed == 0` is asserted a few lines above. The suite's twin has to hunt for a miss-free window
because a loaded host manufactures misses; virtual time cannot.

Measured on both ISAs, with the injection reverted afterwards:

| | clean | with the drift injected |
|---|---|---|
| aarch64 `deadline_delta` over 64 ticks | 40,000,000 of 40,000,000 | **40,004,032**, red |
| riscv64 `deadline_delta` over 64 ticks | 6,400,000 of 6,400,000 | **6,400,231**, red |

The excess is the arrival latency, once per tick. On aarch64, 4,032 counter ticks over 64 is 63 per
tick, which is 1,008 instructions: the arrival figure printed in the same run. On riscv64, 231 over
64 is 3.6 counter ticks, or ~360 instructions, against a printed arrival of 300 to 400. The
instrument does not merely notice the defect; it prices it.

### `ticks_arrive_at_the_configured_rate`: the law stays, the budget stops failing

What it meant to prove, and still does: re-arming relative to `now` compounds lateness, so the
deadlines must sit on a fixed grid. What it measures: over a window in which `MISSED_TICKS` did not
move, the deadline advanced by exactly one interval per delivered tick. That is exact, and a
contended host cannot falsify it. A deschedule long enough to slip the grid increments the miss
count, and the window is thrown away and retried. None of that changed.

The load-sensitive part was never the law. It was the retry budget, and the assertion said so in its
own words: *"either the host is too contended to observe the grid, or the handler is slower than a
whole tick period"*. The deciding word is that "or", and the guest is the one party that cannot
read it. It failed four times in forty-five loaded runs, twice per ISA.

Its implicit second claim is "the handler is not slower than a whole tick period". `script/icount`
asserts that at 2,500 instructions, against a measured 1,056 and 900: roughly 4,000 times tighter.
It also asserts `missed_ticks == 0` with no taxonomy at all.

So exhausting the budget is no longer a failure. It prints, loudly, naming what went unmeasured and
carrying the numbers a human would otherwise re-run to get:

```
test kernel::arch::aarch64::timer::tests::ticks_arrive_at_the_configured_rate ...
    (UNMEASURED: no miss-free window in 8 tries, so the re-arm law was not tested this run. 47
     misses recorded on this core, the last re-armed 1102312 counter ticks late against an interval
     of 625000. A miss re-anchors the grid, so a window containing one proves nothing either way,
     and whether these misses are a contended host or a slow handler is the one question a wall
     clock cannot answer from inside the guest. `script/icount` answers it and asserts this same law
     with no host term in it. Milestone 62; notes/load-sensitive-assertions.md.)
```

That transcript is from a real run under an injected slow handler, not a mock-up. A test that
quietly measures nothing is worse than one that flakes. The reader has to be able to tell "the law
held" from "the law was not looked at", and this is that line.

### `the_handler_keeps_up_when_no_lock_is_held`: deleted on both ISAs

What it meant to prove: with interrupts live and no critical section in the way, a missed deadline
would mean the handler itself is too slow. At milestone 6 that means threads losing time slices.
What it measured: the missed-tick delta over five tick periods, classified by how late the re-arm
was. It failed under one interval and passed at one interval or more.

Two injections settled it, on aarch64: a handler made slow on every other tick by a known number of
tick periods.

| handler slow by | the assertion | `script/icount` |
|---|---|---|
| under 1 period | silent: no miss to classify | red on the bound (2,500 instructions) |
| **1.5 periods** | **red**, correctly: "last miss re-armed 409875 counter ticks late against an interval of 625000... the handler itself is slow, which is this kernel's bug" | red |
| **2.5 periods** | **green**, printing *"(missed 1 tick(s), re-armed 1095438 ticks late, >= interval 625000: the emulator was descheduled; not this kernel's bug, not failed)"* | red: handler 25,001,200 instructions, `missed_ticks 32` |

The third row is less a false negative than a false exoneration, printed. The assertion whose entire
purpose is "the handler keeps up" met a handler taking two and a half tick periods. That is the
worst timer defect this kernel could have, and the assertion told the reader in as many words that
it was the host's fault.

The middle row is the band in which the acceptance run caught a real host deschedule wearing the
slow-handler message, twice per ISA, measured at 0.56 and 0.83 of an interval. So the band in which
this assertion fires is exactly the band in which it cannot say why. Below the band it is silent;
above it, it exonerates. There is no cut inside that band, which is the argument for deleting rather
than re-cutting. It is the first round's own sentence arriving with numbers attached: from inside
the guest a 30 ms handler and a 30 ms deschedule are the same observation.

`miss_detail` survives on both ISAs. The UNMEASURED report above now consumes it, which is what it
was added for: numbers that let a human triage without re-running.

### What this costs, measured rather than estimated

With both dispositions applied, the full aarch64 suite goes green under a handler slow by 2.5 tick
periods on every other tick. That was run, not reasoned. `script/test --arch aarch64` exited 0,
printing the UNMEASURED line above, where before this change it went red at the retry budget. The
suite has stopped making any claim about handler latency at all.

That cost is defensible only because the instrument that catches it is actually run. So the command
a person runs before pushing now runs `script/icount` (`script/gates` then, `script/ci-build` since
milestone 286 (one enumeration of the checks that gate a pull request)). It takes about seven seconds for both ISAs, placed above `script/test` by that
script's own cheapest-first rule. CI already ran it on every change that is not documentation only.
The claim did not weaken. It moved to a boot where the host is not a term, and both gates a change
passes exercise it.

The residual is a cost, not an implication. `script/icount` is `-smp 1`, so it says nothing about a
handler slowed by cross-core contention. The tick path is lock-free on both ISAs today. `tick` is an
atomic add, a watchdog feed and `rearm`; `sched::on_tick` is a relaxed store and a try-lock canary.
So there is nothing there for another core to contend on, and that is why the deletion is safe
rather than an oversight. If the tick path ever takes a contended lock, this paragraph stops being
true, and neither instrument would notice.

### The disposition's own acceptance run, 2026-08-18: 18 runs, and a new site

The same instrument ran on the tree with the disposition applied, because this milestone's rule is
that a flake is not shown fixed by a green run. `script/repeat-under-load -n 18 -s 8`, tree
`01474c8e`, 2026-08-18 18:56Z to 20:01Z, the same eight-core Mac.

Eighteen is not forty-five, and this is a first installment, not the acceptance evidence. It is
recorded because its numbers are already decisive about the two assertions and surprising about a
third.

| | 2026-08-17, before | 2026-08-18, after |
|---|---|---|
| runs / ISA legs | 45 / 90 | 18 / 36 |
| 1-min load average, min to peak | 26.1 to 63.0 | **16.2 to 116.6** |
| red at `ticks_arrive_at_the_configured_rate` | 4 legs | **0** |
| red at `the_handler_keeps_up_when_no_lock_is_held` | 4 legs | **0** (deleted) |
| green runs | 36 of 45 | 16 of 18 |

The load was substantially harsher than in the run that produced the finding. That is the direction
that makes a green result mean something. The peak was 116.6 against 63.0, and three runs took 311,
333 and 586 seconds against a median of 165. Neither dispositioned assertion failed once. That was
expected, since neither can any more. It is stated as a measurement rather than a deduction because
the previous run's expectations were wrong twice.

The new number is higher than anyone should assume. The re-arm law went `UNMEASURED` on 9 of 36 ISA
legs, 25%. The misses were concentrated entirely in the five slowest runs (4, 5, 8, 9, 10) and
absent from the other thirteen. Both legs reported it together in four of those five. That is the
ISA symmetry this register keeps finding, and the same evidence it always is: a property of the host,
not of either architecture.

Twenty-five percent is a real cost and should not be filed as a success. At these loads the law is
checked on three legs in four rather than four in four. Before, it was checked on roughly nineteen in
twenty and turned the other one red. The obvious candidate fix was deliberately not taken here. The
measurement window is a quarter second, or about 25 tick periods. The law is exact rather than
statistical, so the first drifting tick falsifies it, and a window of three periods would do. A
shorter window is not a wider bound: the `assert_eq!` is untouched, and the defect fails it on tick
one. So §61 (a lint is adopted on evidence) does not forbid it. It was refused in this lane for a
different reason. It is a change whose whole justification is a rate, and this lane's rate comes from
18 runs on one host. It wants its own measurement, and this table is the baseline it would be
measured against. (The [confirmation run](confirmation-run.md) of 2026-08-22 later found 0 of 90.)

#### A new member of the family, found by this run

Both reds are the same assertion, and it is neither of this milestone's two:

```
[PANIC] panicked at kernel/src/smp.rs:1010:17:
migration workers never drained (59/60 done)
```

`a_migrated_kernel_thread_keeps_its_hart_pointer` (riscv64, runs 8 and 9) spawns 60 migration
workers and gives them two seconds of counter time to drain; 59 of 60 arrived. That is this family's
shape exactly. Its own comment cites §28 (SMP placement). It shows the author walking up to the problem and
stopping one step short: *"Time-based: an idle hart's yields return at once under §28, so a fixed spin count would
elapse in no time."* Both halves are right. A yield count is not a duration, and a wall-clock
duration is not one either when the host owns the clock.

The failure direction is positive ("not yet"). By the first diagnostic it is honest load sensitivity,
not a wait written against something wider than the property. The fix is the one milestone 62
prescribes by name: a budget in delivered guest ticks rather than counter time.
`sched::within_ticks` already exists for this. The direction is what makes it correct rather than
merely different. A descheduled emulator delivers fewer ticks per second of wall clock, so a tick
budget stretches under exactly the conditions that need it, where two seconds of counter time does
not. Not fixed here; it wants a lane, and it is in a file this lane did not touch. (Taken the same
day: see [the migration drain](migration-drain-tick-budget.md).)
