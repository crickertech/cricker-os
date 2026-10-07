---
status: BUILT
raised: 2026-10-03
built: 2026-10-03
promoted_from: the-boot-thread-cannot-halt-while-runnable
milestone_dependencies: 628
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 720. The boot thread cannot halt while it is runnable

Number provisional (the 720 lane, 2026-10-03 UTC): minted by the integrator at merge. Promoted from
the proposal `the-boot-thread-cannot-halt-while-runnable`, raised 2026-10-03 by the maintainer
session that wrote the merge-rate correction (`notes/coes/2026-10-03-the-merge-rate.md`); calef
approved closing it the same day.

A thread that halts the core while it is still on the run queue costs a timer tick every time round
robin reaches it. Milestone 628 (the x86_64 swish-check leg costs what the others do) found x86_64's
boot thread doing that after the hand-over, at 7.7 s a swish-check line, and ended it in
`sched::exit()`. aarch64 and riscv64 ended theirs the same way, in `arch::halt()`.

What ships:

1. **`arch::halt` takes an `arch::HaltReason`**, a zero-sized token with a private field. calef
   ratified the name on 2026-10-03, replacing the lane's provisional `Terminal`, which clashed with
   the display terminal and tty sense the tree already uses. The constructors' names are
   provisional. Its constructors are the whole list of who may stop a core for good.
   `panicked(&PanicInfo)` needs what only the panic handler holds, and `test_build()` exists only
   in a test image. `measurement_boot()` exists only in a `bench`, `icount`, `soak_test`,
   `job_mix`, `disk_throughput` or `tsc_probe` build. `before_scheduler()` panics if the scheduler
   is running. An ordinary or `shell` build compiles only the first and the last, so a boot path there that reaches
   for `halt` does not compile, and one that reaches for `before_scheduler` too late panics on its
   first run. This is rung 1 of the ladder for every case but the last, which is rung 2.
2. The aarch64 boot's end and both riscv64 hand-overs (`shell` and the default boot) call
   `sched::exit()`, as x86_64's has since 628. All three architectures now leave the scheduler the
   same way, per DECISIONS §19 (architectural parity is a tenet).
3. The measured-boot refusals in `kernel/src/trust.rs` and the progenitor's archive failures in
   `kernel/src/user.rs` run on the boot thread after `sched::init`, so they leave the same way. What
   a reader sees is unchanged: the refusal prints, and nothing after it runs.
4. aarch64's semihosting `exit` ends in its own `wfi` loop, as riscv64's and x86_64's already did.

## How it was proved

The compiler is the test that fails before the fix. On the base tree, the aarch64 and riscv64 boot
endings are `arch::halt()` calls in builds that cannot make a `HaltReason`. They do not compile
against the new signature. CI's suite boots all three architectures, and `swish-check` runs each
one's hand-over to a prompt.

Measured on CI, the swish-check legs before and after, per line (the proposal's step 1). The
proposal inferred the cost would be small, because the aarch64 and riscv64 input drivers wait on an
interrupt and so nothing keeps the rotation turning while the shell waits.

| leg | main, 5 merge-group runs (first boot, 145 lines) | this branch, run 37144785954 |
|---|---|---|
| aarch64 | 26.4 to 27.5 s, median 27.2 s | 27.2 s |
| riscv64 | 30.7 to 31.1 s, median 31.1 s | 30.7 s |

The second boot's eight lines took 1.3 to 1.4 s on main and 1.4 s here, on both legs. So the
inference held: on these two architectures the parked boot thread cost nothing measurable, because
nothing kept the rotation turning. This is an honest tie. What the milestone buys is the parity and
the gate, not speed. Main's runs: 37141733965, 37141058488, 37139101598, 37137625820, 37136859644.

## BUGS

- `before_scheduler()` is checked at run time, not by the compiler. A boot that fails before the
  scheduler exists has nothing a type could carry. It is used once, on x86_64's no-boot-info path.
- The measurement boots still halt the boot thread while it is runnable, by design: their run is the
  halt and the harness kills QEMU at the marker. A measurement that is ever paced by ticks on that
  core should exit instead.
- The `exit` paths of the three semihosting modules are `wfi`/`hlt` loops written out in `arch/`
  without a `HaltReason`. Each is unreachable unless the host ignores the exit device.

## Follow-on

- **Recorded.** The runtime-checked constructor and the measurement boots' runnable halt are in this
  block's BUGS and in `kernel/src/arch/mod.rs`, at `HaltReason`'s constructors.
- **Milestone 505.** Milestone 505 (an x86_64 input driver that never lets the core idle) is the
  other half of this defect class: x86_64's polling input driver still keeps that core from idling
  at a prompt.

## Index row

`arch::halt` takes a `HaltReason` that only a panic, a test image, a measurement boot or a pre-scheduler failure can make; every boot thread ends in `sched::exit()`.
