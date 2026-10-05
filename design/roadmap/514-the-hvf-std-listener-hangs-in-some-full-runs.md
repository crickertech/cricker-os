---
status: NOT-STARTED
raised: 2026-09-19
promoted_from: the-hvf-std-listener-hangs-in-some-full-runs
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 514. The HVF leg's std listener test hangs in some full runs and passes alone

*(Number provisional until the merge queue lands it.)* Promoted from the
proposal `the-hvf-std-listener-hangs-in-some-full-runs`, filed 2026-09-19, on calef's instruction of
2026-09-20 to give every proposal on `main` a number. The text below is the proposal's own, unedited
except for this paragraph: the argument is its author's and promotion is not the moment to improve
it. Found by milestone 227 (a GICv3 driver, because GICv2 boots and)'s lane (a GICv3 driver), on the first two full `script/test --hvf` runs
this machine could make since milestone 222 (the one command a person runs before).

It needs patagonia, or any Apple Silicon Mac with HVF, and nothing else.

## In brief

`kernel::user::tests::a_std_program_serves_a_granted_listening_port` hung in two of the three full
`script/test --hvf` runs that reached it (the third was green end to end), and passes when run alone under HVF
(`cargo xtask test --hvf --test a_std_program_serves_a_granted_listening_port`) and in every TCG run,
GICv2 and GICv3 alike. It is the one flaky test left on the HVF leg, and `script/ci-build` names it when the
leg fails so a contributor can tell it is not theirs. That sentence should go when this lands.

What the transcript shows, in order:

```
test kernel::user::tests::a_std_program_serves_a_granted_listening_port ...
  user thread 4294967418 killed: BRK instruction
    pc 0x000000000040e7dc   far 0x0000000000a50000   user sp 0x0000000000500b10   esr 0xf2000000
  the kernel is fine.

WATCHDOG: no progress for ~60 s. Every core idle, every thread blocked: a lost-wakeup hang.
```

and after the run, from the host's inbound prober:

```
inbound check (aarch64) FAILED: the guest served 2 of the 4 inbound connections ...
    +18822 ms: answered after 18613 ms, 9 bytes
    +18823 ms: answered after 1 ms, 9 bytes
    +109147 ms: stopped-while-waiting after 90324 ms, 0 bytes
```

`0x40e7dc` in `std_exerciser` is `__rust_abort`, so the std program panicked, and its message
went nowhere a transcript reads.

## The likeliest reading, which is not established

`serve_one_inbound` (`std_exerciser/src/main.rs`) panics with "round N: nobody connected" when
`accept` returns an error, and `std-nife`'s `accept` returns `WouldBlock` when `net_stack`'s
bounded wait expires. `probe_inbound` (`xtask/src/main.rs`) holds each connection it opens until it
is answered or the run ends. So if the prober opened a connection in the gap between the
hand-written listener's window and the std one, and that connection is never delivered to the std
listener, the prober waits on it for the rest of the run (the 90-second `stopped-while-waiting`
above) and the std listener's bounded wait expires with nobody connecting. HVF runs the suite about
three times faster than TCG, which reshapes that gap. None of this was confirmed by
instrumenting it.

## Two defects may be here, and the second is independent of HVF

1. **The timing**, as above: whatever it is, it is HVF-shaped.
2. **A kernel test hangs rather than fails when the program it waits on dies.** The test waits for
   a report on a rendezvous; the std program is dead; the watchdog fires 60 seconds later and calls
   it a lost wakeup, which it is not. A test that saw its reporter killed and failed at once, saying
   so, would have turned this into a one-line red.

## What would settle it

Capture the std program's panic message (its stderr reaches no transcript today), and log the
prober's connection opens against the guest's listen and accept times on one clock. If the reading
above holds, the fix is on the host side of the prober or in the listener's retry, not in the
kernel.

## Index row

`kernel::user::tests::a_std_program_serves_a_granted_listening_port` hung in two of the three full
`script/test --hvf` runs that reached it (the third was green end to end), and passes when run alone
under HVF (`cargo xtask test --hvf...
