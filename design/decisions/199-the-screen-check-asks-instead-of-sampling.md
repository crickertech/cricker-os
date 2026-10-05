---
status: DECIDED
raised: 2026-09-20
decided: 2026-09-20
ratified_by: calef
---

# 199. The screen check asks instead of sampling

calef, 2026-09-20, choosing option A from three put to him after the check
failed under load. *(Section number provisional until the merge queue lands it.)*

The ruling. The boot's screen assertion stops sampling a transient state and starts asking for
it. A boot-time knob, off by default, makes the kernel hold the screen and say so on the serial line
before `yield_screen` clears it; the host takes its dump, answers, and the handover proceeds. The
wait is bounded, so a knob set by mistake cannot wedge a machine.

## What was wrong, and it was the mechanism rather than the timing

`cargo xtask uefi-boot` proves the claim of milestone 243 (a machine with no serial port has no way
to say anything) by polling QMP `screendump` every 50ms, needing to catch the kernel's banner in the
window between the tour painting it and the userspace terminal taking the screen, which
`xtask/src/uefi.rs` calls "a couple of seconds under TCG".

On 2026-09-20 a full `script/test` run read zero rows and reported *"the tour was never readable
on the screen"*; the same leg alone a minute later read 56. Under load the dump-write-read-decode
round trip stretches and the guest's window does not stretch with it. Two things make that worse
than a flake. `tour: None` is also what a genuinely broken framebuffer produces, so the message sent
its reader after `LocateProtocol`, the pixel order, the stride and the mapping surviving
`mmu::init`, none of which was wrong. And a longer deadline does not help, because the window closes
in guest time rather than wall-clock time.

That is rung four of `AGENTS.md`'s ladder, sampling and hoping, dressed as a test.

## The alternatives, and why each lost

- **B, an xtask-only mode where the handover does not clear.** Refused: it makes the screen-check
  boot differ from a real boot at exactly the moment under test, which is the one thing this leg
  exists to avoid.
- C, keep sampling and only fix the verdict's honesty. Cheaper and defensible, and it changes no
  boot code, but it leaves a test that fails on a busy machine and calls it a framebuffer bug. Its
  message split is worth having anyway and is folded into A rather than dropped: even with the
  handshake, "missed the window" and "the framebuffer path is broken" are different sentences.

## What this costs, stated because it is the reason C existed

A test-only affordance now exists in the handover path, and this tree is strict about that. Three
things bound it: the knob is off by default, so a real boot performs no serial read and pays no
latency; the wait is bounded, so the failure mode of a mis-set knob is a delay rather than a
dead machine; and the code says at the knob that it is a debugging affordance, since a reader who
meets it otherwise reads it as a design.

The knob's own name, and the parity question of whether `script/boot-check` shares the race on
aarch64 and riscv64, are the building lane's to answer and calef's to ratify.
