---
status: BUILT
raised: 2026-10-04
built: 2026-10-04
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 744. Measure the kernel's overflow checks

Raised 2026-10-04 (UTC) by calef. The number 744 is provisional until the integrator mints it at
merge; the title, the slug and the note's name (`notes/overflow-checks.md`) are drafts.

## Why

`-C overflow-checks` decides whether `a + b` panics or wraps on overflow, and it follows the cargo
profile rather than the code. Nobody had written down which builds carry it. This is a measurement:
it changes no profile, and the choice it prepares is calef's.

## What was measured

[notes/overflow-checks.md](../../notes/overflow-checks.md) holds all four answers:

1. Which builds carry checks, every gate and every shipped path, on three ISAs, and what a panic on
   overflow does in the kernel and in a program.
2. What turning them on in release finds, run through the full CI gate on a throwaway branch.
3. What it costs: fast-path footprint, kernel and userspace code size, and icount on the bench
   headlines, checks on against checks off on one commit.
4. Prior art: Rust, Linux, Rust-for-Linux, Hubris, Android, Chromium, Redox, Tock, seL4.

It ends with four options and one recommendation for calef. Found: no overflow in any checked run;
the shipped (release) build is the only unchecked one, and three image programs were unchecked even
under test. Cost with checks on in release: fast-path footprint +4 to +9% on the round trip, kernel
code +2 to +11%, IPC instructions under 1.1%; on aarch64 release under HVF, no IPC cost above noise
and CoreMark +2.5%.

## Follow-on

- **Milestone 750.** calef chose option A on 2026-10-04; milestone 750 (overflow checks in the
  shipped build) turned the profiles on, rewrote the fast paths and re-derived the frame budget.
- **Recorded.** Release cycles on riscv64 and x86_64, and the hand-built `cryptography_exerciser`
  and `rg`, are unmeasured, in the BUGS section of `notes/overflow-checks.md`.

## Index row

Which builds carry Rust overflow checks, what checking the shipped build would find and cost, and
the prior art, measured for calef's decision; no profile changed.
