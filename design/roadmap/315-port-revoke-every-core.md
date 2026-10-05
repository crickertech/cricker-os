---
status: BUILT
raised: 2026-09-17
built: 2026-09-23
promoted_from: a-port-revoke-that-reaches-every-core
---
# 315. A port revoke that reaches every core

Built by a lane on `milestone/315-port-revocation-two-core`. Promoted
from `design/roadmap/proposals/a-port-revoke-that-reaches-every-core.md` by calef on 2026-09-17, the
day milestone 313 (the security audit that was due since August) raised it as finding 4.
*(Number provisional until the merge queue lands it.)*

It was gated `DECISION` until 2026-09-18, when calef answered
§153 (how a two-core x86_64 test earns its place),
[the decision file](../decisions/153-two-core-x86-test-sequencing.md): close this milestone
first, then default
`NIFE_SMP` to 2. Both halves are done, and the second is the verification of the first.

## What it owed, and what landed

1. The broadcast. `arch::x86_64::segments::revoke_port_grant_everywhere` resets this core's TSS
   I/O permission bitmap and then tells every other online core to do the same, over the TLB
   shootdown's NMI (`mmu::revoke_port_grant_others`). `sched::delete_port_range_caps_impl` calls it
   in place of the core-local reset.
2. The default flip. `helpers/qemu-runner-x86_64.sh` defaults `NIFE_SMP` to 2.

## The defect was reproduced before it was fixed

The scope had already shrunk while the decision was open: this block once said the test was the
deliverable and would be the first port test able to observe the revocation window, and milestone
316 found that the test already existed. `user::x86_port_tests::a_revoked_holder_faults_on_its_next_port_write`
became that observer the moment 316 gave it a working two-core substrate.

7 of 12 full two-core runs is 316's campaign figure. This lane established its own, on the same
host (patagonia, no induced load), against the filtered leg `cargo xtask test --arch x86_64 --test
x86_port` at `NIFE_SMP=2`:

| campaign | runs | failures |
|---|---|---|
| filtered, uninstrumented | 12 | **3** |
| filtered, instrumented | 30 | **2** |

Every failure was the same assertion at the same line, `x86_port_tests.rs`'s `left: 2, right: 1`:
the child's `out` was permitted and it exited cleanly (`EVENT_EXIT`) where the test demands a
fault. The filtered leg runs three tests rather than 326, which is why its rate is lower than the
full suite's; it is the same failure.

## What actually raced, with the evidence

`PortRange::REVOKE` deletes the capability from every thread's table and clears the cached grant
(`thread::Thread::port_range_grant`) under `sched::IPC_TABLES`, then resets the TSS bitmap. The reset
was **core-local**, which was the whole machine when DECISIONS §152 (the port-range capability)
recorded it and stopped being so when `smp::seat_cpus_from_acpi` made this port multi-core.

A core's bitmap permits a range only while the thread holding it is the thread running there, so the
window needs the holder to be *running on another core* at the instant of the revoke. The test's own
shape produces exactly that: it starts the child, revokes, and sends the wake, and when the send
lands before the child reaches its `RECV` the child's `RECV` returns without ever switching away. So
the grant core 1 installed at switch-in is still installed when the `out` executes, and core 0's
reset never touched it.

Measured rather than argued. A temporary snapshot of `INSTALLED_PORT_GRANT`, taken at the revoke
and read by the test after the outcome arrived:

| what the snapshot said at the revoke | runs | outcome |
|---|---|---|
| revoker on cpu 0, **cpu 1 holds a grant** | 2 | **both failed** (`EVENT_EXIT`) |
| revoker on cpu 1, cpu 0 holds a grant | 5 | all passed |
| no core holds a grant | 27 | all passed |
| revoker on cpu 0, cpu 1 holds a grant | 1 | passed (the holder switched away in time) |

A remote core holding the grant is necessary and not sufficient, which is exactly what a window of
"until that core's next context switch" predicts.

## The fix, and the one thing that makes it sound

The remote half rides `mmu::shoot_down_others`' protocol rather than a new one: `SHOOTDOWN_KIND`
says which step the round is for and `SHOOTDOWN_VA` carries a packed `(base, count)` instead of an
address. The acknowledgement mask, the single-round lock and the NMI delivery are the parts that are
hard to get right, and there is now no second copy of them to keep in step. The NMI is forced rather
than preferred for the same reason it was for a page (notes/x86-tlb-shootdown.md): the target core
is routinely spinning for `IPC_TABLES` with interrupts masked, because the revoker is holding it.

**Broadcasting alone would not have been enough**, and this is the part that is not in the proposal.
The far end writes a TSS from an NMI handler, at an arbitrary instruction boundary. Two windows stay
open if nothing else changes: a core can read a grant the sweep has already cleared and install it
*after* the broadcast, and the NMI can land in the middle of that install. Both shut on one rule,
now stated at `segments::set_port_range_grant_on`: a core's port bitmap is written only by a thread
holding `sched::IPC_TABLES`, or by an NMI that such a thread sent.

Making it true took moving `schedule`'s `install_port_grant` inside the locked region (it read the
grant under the lock and installed it after) and `delete_current_cap`'s core-local reset with it.
The cost on a machine where nothing holds a port is one compare inside the critical section instead
of one outside it, because `set_port_range_grant` returns after a single comparison when the grant
has not changed.

`delete_current_cap` owes **no** broadcast: the thread dropping its own capability is the thread
whose grant is installed, and it is running on this core, so this core is the only core to tell.

## What proves it

`user::x86_port_tests::a_revoked_holder_faults_on_its_next_port_write` at two cores, which failed
before the change and passes after it, on the counts in the tables above. It is a gate rather than a
measurement now: two cores is the default, so it runs on every `script/test --arch x86_64`. So does
`user::tests::an_asid_flush_reaches_the_other_cores`, the TLB-shootdown test from
milestone 161 (the x86_64 kernel port), which had
been verified by hand and gated by nothing.

## BUGS

- Three cores and above are still opt-in and still unmeasured on silicon. The flip is to 2, not
  to 4 like the other two runners. `ap_boot`'s `BUGS` #1 was closed on QEMU TCG in 2026-09-19 and
  xenon has never been asked to bring all four cores online; the line to read there is
  `smp: N core(s) online`.
- **Every number here is QEMU TCG on one host**, and on an aarch64 host QEMU runs an x86_64 guest
  round-robin unless `NIFE_TCG_THREAD=multi` is set, so two cores take turns rather than executing
  in the same instant. The window this closes was wide enough to fire anyway; a narrower one might
  not be visible here at all.
- **The broadcast is unbatched and unmeasured**, like the TLB shootdown it rides. A revoke of one
  range costs one full round trip to every online core. Nothing in `script/bench` prices it, and
  nothing should until a workload revokes a port range more than once per boot.
- **The NMI handler's work is proportional to the range**, where the TLB step is constant. Taking
  back COM1's eight ports is eight bit writes and nothing to think about, but a grant over the whole
  16-bit port space would spend 8 KiB of bitmap writes inside an NMI on every online core. The
  switch path has always paid the same cost for the same range and this inherits it rather than
  introducing it, and nothing in the tree grants a range wider than a device's.
- **`SHOOTDOWN_VA` now carries something that is not a virtual address**, and keeps its name. The
  spelling is what every existing reader calls it and renaming a static that still carries the older
  meaning in most of its uses is a naming decision a lane does not get to make.

## Follow-on

- **Recorded.** `exceptions::NMIS_UNCLAIMED` should stay at zero and no test asserts it, which was
  already `notes/x86-tlb-shootdown.md`'s `BUGS` entry and is now load-bearing for a second protocol
  rather than one. It stays a recorded limitation rather than becoming a milestone here.
- **Recorded.** `loom` does not fit this protocol, and the reason is worth keeping. It was the
  obvious instrument to reach for (notes/interleaving.md has five protocols modelled this way) and
  it cannot see either half of this defect. The bug was a message that was never sent rather than an
  interleaving of the messages that were, and the enforcement it failed at is a CPU reading a TSS
  bitmap on an `out` instruction, which is outside any Rust memory model. The shootdown protocol's
  atomics *are* a hand-rolled candidate and now serve two kinds rather than one, but they live under
  `arch/`, so lifting them to a host crate is the bigger question notes/interleaving.md already
  parks for `arch/*/irq.rs`. Milestone 355 (four crates were lifted so loom could search them) is
  where that would go if it goes anywhere.
- **Recorded.** The reconciliation from milestone 316 (which core booted) still stands: it saw
  26 two-core boots and zero
  `smp: cpu 1 did not start`, against the one-in-three recorded by milestone 412 (`script/test`'s
  UEFI leg asserts two cores online), and this lane's runs add
  more of the same evidence without being the dedicated measurement 412 asks for.

## Index row

`PortRange::REVOKE` cleared the capability table and the cached grants but reset the TSS I/O bitmap
on the revoker's core only, which was the whole machine when DECISIONS §152 recorded it and stopped
being so when `smp::seat_cpus_from_acpi` made x86 multi-core; a revoked holder running on another
core kept COM1 until that core's next context switch. Milestone 313's audit accepted the window on
reasoning, and at two cores it was red in 7 of 12 full runs of the test written to see it. This lane
reproduced it (3 of 12, then 2 of 30 on a filtered leg), named the cause with a snapshot of
`INSTALLED_PORT_GRANT` taken at the revoke that read "revoker on cpu 0, cpu 1 holds a grant" on both
captured failures, and closed it by broadcasting the reset over the TLB shootdown's NMI with a kind
tag on the existing protocol. The half the proposal did not have is that broadcasting alone leaves
the window reopenable: `schedule` read the grant under `IPC_TABLES` and installed it after, so a
core could install a grant the sweep had already cleared, and the NMI could land inside that
install. Moving the install under the lock gives one rule instead, stated at the writer, that every
TSS bitmap write is made by a holder of `IPC_TABLES` or by an NMI one sent. `NIFE_SMP` defaults to 2
as the closing step per DECISIONS §153, which makes both this test and milestone 161's
TLB-shootdown test gates rather than things somebody verified by hand.
