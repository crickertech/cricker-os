---
status: DECIDED
raised: 2026-07-13
decided: 2026-07-13
ratified_by: calef
---

# 7. Testing: QEMU harness + host-testable crates, from commit one

A custom test harness boots the kernel in QEMU, runs tests, and exits with a status code
`cargo test` understands. Separately, pure logic (allocator algorithms, page-table math,
scheduling policy, filesystem parsing) lives in crates that compile for the *host*, so
most tests run in milliseconds with no emulator.

Front-loads about a day. Prevents a year of debugging by `println!`.
