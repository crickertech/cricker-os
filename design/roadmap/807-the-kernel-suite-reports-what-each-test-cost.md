---
status: NOT-STARTED
raised: 2026-10-06
promoted_from: the-kernel-suite-reports-what-each-test-cost
milestone_dependencies: none
decision_dependencies: 254
machine_requirements: none
specific_machine: none
needs_person: no
---
# 807. The kernel suite reports what each test cost, and the riscv64 suite's time is explained

*(Promoted from the proposal pile on 2026-10-06 (UTC); number provisional until the merge queue lands it.)*

*(Ruled 2026-10-06 (UTC), calef on PR #1778: "Yes on Fork 1, the printed block", and "Step summary
plus artifact, with no PR comment". §254 (a gate prints what each item cost, and a job near its
budget warns rather than fails) records all four rulings and the refused options. Milestone 808
(every gate accounts for its time) builds on this block.)*

Raised 2026-10-06 (UTC) by lane `gate-time-proposal`, a writing-only lane, from calef's ask the same
day: "Write up a proposal milestone that measures where the riscv64 suite's time goes." This is the
first slice of milestone 808 (every gate accounts for its time). Title and slug are drafts.

## Why this is its own milestone

The general milestone is open-ended. This slice is a few days of work, and two things already wait
on it alone: whether `cpu-matrix` needs a third shard in about three weeks, and the deadline
milestone 663 (bound the host pass) must take from a measurement. When a piece of work depends on
part of a milestone, the tree splits the milestone rather than naming a rung inside it. So this
slice has a number of its own, and milestone 808 depends on it.

## What happened

PR #1775 split `cpu-matrix` into two shards, because the job had reached 20.1 to 21.0 minutes
against milestone 721 (each merge-group CI job has a 20-minute budget). Its measurement covered 31 green merge-group runs on main
from 2026-09-20 to 2026-10-06. No step regressed. Each riscv64 model's boot went from 1.6 to 3.1
minutes because the suite grew: the riscv64 section of `test` went from 358 to 457 tests, and from
about 47 to 110 seconds of test time. Growth runs about 0.1 minutes per model per day. The longer
shard reaches 20 minutes again in about three weeks.

Every per-test number in that pull request was scraped from the timestamps GitHub stamps on log
lines. That is the gap this slice closes.

## The suite already measures each test, and throws the number away

`Testable::run` in `kernel/src/testing.rs` stamps `arch::timer::now()` before each test, because the
per-test ceiling needs a start time. After the test it computes the elapsed seconds and prints
`[N s]` only when the figure reaches `SLOW_REPORT_SECS`, which is 5. Below that the number is
discarded. So the instrument exists on all three architectures; it is the record that is missing.

The console is the only channel all three architectures share. aarch64 and riscv64 exit through
semihosting, but x86_64 exits through QEMU's `isa-debug-exit` device
(`kernel/src/arch/x86_64/semihosting.rs`), which has no file I/O. So the record has to travel in
the transcript, and `xtask` has to parse it, as it already parses `test result:` lines in
`xtask/src/suite.rs`.

## Where the riscv64 suite's time goes, scraped once

One run, to check the premise before building on it: the green merge-group run 37505448674 at
34cfc9c93, `test` job, 2026-10-06 17:41 to 17:56 UTC (14.9 minutes). Per-test times are deltas
between consecutive `test ...` lines, so each includes the output printed between them.

The riscv64 leg took 160.5 s:

| phase | seconds |
|---|---|
| build and boot to the kernel's `running` line | 19.1 |
| kernel unit tests (137) | 6.4 |
| build and boot `system_tests` | 6.1 |
| `system_tests` (320) | 129.0 |

The same system tests took 98.3 s on aarch64 and 59.6 s on x86_64, where 46 of 299 skip. Five
modules hold 84 of riscv64's 129 seconds:

| module | riscv64 s | aarch64 s |
|---|---|---|
| `compositor_tests` | 24.5 | 14.1 |
| `login_tests` | 18.0 | 11.4 |
| `riscv_virtio_tests` | 14.8 | (riscv64 only) |
| `display_tests` | 14.6 | 9.3 |
| `timetable_tests` | 12.2 | 11.6 |

Two tests hold 20 of the 129 seconds:

- `timetable_tests::a_calendar_entry_keeps_time_by_its_granted_clock_and_a_step_moves_it_by_s3`
  takes 10.5 s on riscv64, 10.4 on aarch64 and 10.7 on x86_64. The same figure on three emulators
  means it waits on wall time rather than computing. It does: steps 2 and 4 publish the clock at
  01:59:55 and wait for the 02:00 line to fire, five real seconds each.
- `display_tests::a_bitmap_font_and_a_vt_engine_put_readable_text_on_the_scanout` takes 9.9 s on
  riscv64 and 7.1 on aarch64. That ratio says it is compute under TCG. Why it costs that much is
  not yet measured.

This scrape reads 129 s where PR #1775 read about 110 s for the same section, on a different run
and with a different cut. Two scrapes of one suite disagreeing by 17% is the argument for having
the framework report its own numbers.

## What to build

- The guest records every test's elapsed time in milliseconds, using the start stamp it already
  takes, on all three architectures. The same code serves all three, since `testing.rs` is shared.
- At the end of the suite the guest prints the record as a block, after the existing reports and
  before `test result:`: one `time <milliseconds> <full test path>` line per test (§254, Fork 1).
  The grammar lives in a crate the kernel and `xtask` both depend on, name provisional.
- `xtask` times the host side of every leg: the build, the boot to the `running` line, each image's
  suite, and the exit. It parses the guest's record, and writes both to one machine-readable file
  per job (name provisional). The `test` and `cpu-matrix` jobs upload it as an artifact, the way
  `cpu-matrix` already uploads its per-model logs, and write the slowest tests and modules to the
  step summary (§254, Fork 3). No pull request comment.
- A cross-check that proves the guest clock is honest. For each image, the sum of the guest's
  per-test times must fall within a stated tolerance of the host's measured span for that suite.
  This matters most on x86_64, where the guest's clock is a TSC calibrated at boot. Milestone 571
  (the x86 boot calibrates the TSC once, and can be wrong by 4x) found that calibration fragile.
  Nothing else in the tree would catch a guest clock that lies.
- A notes page answering calef's question from ten or more merge-group runs: the median per test
  and per module on each architecture, the riscv64-to-aarch64 ratio, and what the growth since
  2026-09-20 consists of.
- The cheap fixes the answer names, each landed or refused with a reason. The two known today:
  - The calendar test's two five-second waits, now in flight as PR #1779. Publishing at 01:59:59
    instead of 01:59:55 should save about 8 s on every boot that runs the suite. That is ten boots per merge-group run: one on
    aarch64, one in `test` and five in `cpu-matrix` on riscv64, and three on x86_64 (q35, AMD-Vi,
    OVMF). The lane checks first whether the margin is load-bearing, since the test says no
    assertion may depend on when the timetable reads the page.
  - The display test's 9.9 s, measured before anything is changed.

## Exit criteria a stranger can check

1. `cargo xtask test` on a clean checkout prints a per-test timing record for every image on
   aarch64, riscv64 and x86_64, in §254's format, and writes the per-job file. A host test in the
   shared crate proves the parser round-trips a line and rejects a malformed one.
2. The latest green merge-group run of `test` and of `cpu-matrix` has the file as an artifact, and
   it names every test the transcript names, with no gaps. Its step summary lists the slowest tests.
3. On each architecture the cross-check holds for every image, and a host test proves that it
   fails when the two clocks disagree.
4. The notes page exists, cites the runs it read by id, and gives the riscv64 answer per module.
5. The calendar test's waits are fixed or refused in writing, and the display test has a measured
   explanation.

## Cost

- Runner time: the record adds one console line per test, about 460 lines per riscv64 boot. QEMU's
  UART has no real baud rate, so this is expected to cost under a second per boot. That figure is
  unmeasured, and the first runs are the measurement. Uploading a small artifact costs seconds per
  job.
- Claude tokens: none per pull request. The artifact is read only by whoever opens it.
- Savings: the calendar fix alone is about 80 s of arm64 runner time per merge-group run, and about
  8 s off each `cpu-matrix` model.

## Reuse

The guest side is kernel code, written here by rule. It reuses the start stamp the ceiling already
takes. The host side reuses `xtask/src/suite.rs`'s transcript reading. libtest's JSON event format
was considered for the record and not taken: it is unstable on nightly, and it would make a kernel
console line depend on a format rustc may change.

## What was considered and lost

- Lowering `SLOW_REPORT_SECS` to zero, so every `ok` line carries `[N s]`. It changes a line that
  `xtask`, `script/falsifications` and the HVF leg already parse. Whole seconds also round most of
  this suite to zero.
- Writing the record to a host file through semihosting. It works on two architectures and not on
  x86_64, which §19 (architectural parity is a tenet) refuses.
- Keeping the log scrape. It is what PR #1775 did, and it disagreed with this one by 17%.

## The fork, ruled by §254

Fork 1, the record's format, was an architect's call because two programs agree on it, the kernel
and `xtask`. calef took the recommendation: a block after the suite, one line per test, shaped
`time <milliseconds> <full test path>`, its grammar in a shared crate and never a `#[path]` module.
It leaves every existing line unchanged, it is greppable, and it costs one parser. The crate's name
is provisional.

## BUGS

- Guest time is emulated time. Under TCG it follows the host clock, so a slow runner reads as a
  slow test. The cross-check bounds the guest clock against the host, not against a fair machine.
- The per-test deltas in the scrape above include console output between test lines, and the
  record will not. The two series will not match exactly.

## Index row

The kernel suite already times each test for its ceiling and discards the number below 5 s. This milestone prints every test's time in a shared format on all three architectures, upload it from `test` and `cpu-matrix`, and answer where riscv64's 129 s of system tests go.
