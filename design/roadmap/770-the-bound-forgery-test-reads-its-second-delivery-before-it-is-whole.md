---
status: NOT-STARTED
promoted_from: the-bound-forgery-test-reads-its-second-delivery-before-it-is-whole
raised: 2026-10-03
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 770. The bound-forgery test reads its second delivery before it is whole

Raised 2026-10-03 (UTC) by the lane for milestone 718 (provisional), diagnosing a failed riscv64 `rva22s64` cpu-matrix leg on PR #1534 (run 37149879144). That PR touches only `cfg(kani)` harnesses, patch records and notes, so no kernel or test binary changed.

## The finding

`system_tests/src/user/notification_tests.rs` publishes the receiver's second message into `SECOND`, five atomics initialised to `u64::MAX`, one word at a time (the loop near line 74). The test waits only for `SECOND[0]` to leave `u64::MAX` (line 116), then loads all five (line 119). On a second hart the receiver can have stored word 0 and not yet words 1 to 4. The failing log shows exactly that: left `[2, MAX, MAX, MAX, MAX]`, right `[2, 57005, 7, 0, 0]`. Word 0 arrived and the rest had not. The assertion text ("must still arrive with w4 == 0") describes the kernel property; the failure was the reader being early, not the kernel delivering a wrong word.

## The fix, and what is unproven

Wait on `SECOND[4]` (the last word written) instead of `SECOND[0]`, or load all five with Acquire after a release store of a final flag. It is one line and reversible.

The same job also reported the riscv64 inbound check serving 0 of 4 connections (one connect failure, one reset after 51 s). The log shows the host not oversubscribed. This note does not claim the two are related: the inbound check is a separate boot after the test binary exits, and nothing here ties them. If the rerun fails the inbound check alone, that is its own finding.

## Index row

A riscv64 cpu-matrix leg failed because the bound-forgery test waits on the first word of a five-word message and reads all five. Waiting on the last word, or an Acquire load after a release flag, is a one-line fix to a test that can otherwise report a kernel failure that is not one.
