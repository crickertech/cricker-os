---
status: BUILT
raised: 2026-08-03
built: 2026-08-03
---
# 83. A mechanical rule-1 lint

Raised 2026-08-03, same survey as 79, which found the violation.

CLAUDE.md's first rule, all architecture-specific code under `kernel/src/arch/`, is what makes the
x86_64 port a new directory instead of a diff across every file, and it is enforced by discipline
alone. The discipline has already slipped once: `kernel/src/user/tests.rs` reads `SPSel` with a raw
`asm!` outside `arch/`. One violation in a tree this size is a good record, and also exactly how the
second one arrives.

The work is small: `script/lint` gains a check that fails on `asm!`, `global_asm!`, or `core::arch::`
in `kernel/src/` outside `kernel/src/arch/`, with no allowlist; and the existing violation moves
behind an arch helper (name provisional, and an architect's to make, per the naming rule).

## Scope note

Kernel only, deliberately. Crates like `user_rt` legitimately hold `asm!` in per-ISA modules, so the
rule there would be "asm lives in the ISA-suffixed module", which is a different check with its own
false-positive surface. Whether it is worth writing is a question for after this one has run for a
while.

## Follow-on

- **Refused.** Extending the check past the kernel was left out on purpose. Crates like `user_rt`
  legitimately hold `asm!` in per-ISA modules, so the rule there would be "asm lives in the
  ISA-suffixed module", which is a different check with its own false-positive surface, and
  `script/lint` has already had checks deleted for exactly that. Whether it is worth writing is a
  question for after this one has run for a while.
  This refusal is milestone 489 (design/roadmap/489-the-rule-1-lint-past-the-kernel.md), which
  carries it with the condition that would change it.

## Index row

CLAUDE.md's first rule (architecture-specific code lives under `arch/`) is enforced by nothing,
and one violation exists today: `user/tests.rs` reads `SPSel` by raw `asm!`. `script/lint` learns
the grep; the violation moves
