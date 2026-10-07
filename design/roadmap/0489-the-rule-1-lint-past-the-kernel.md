---
status: REFUSED
raised: 2026-09-20
refused_by: 83, 448
---
# 489. The rule-1 lint, past the kernel

Refused by milestone 83 (design/roadmap/83-rule-1-lint.md), and recorded there
on 2026-09-03. Backfilled here on 2026-09-20 by
milestone 448 (design/roadmap/448-a-refusal-gets-a-number.md), which gave a refusal that names work a number, a
status and a condition that would change it. *(Number provisional until the merge queue lands it.)*

**The date is when the refusal was written down, not necessarily when it was made.** Most of this
tree's `- **Refused.**` bullets were written in one sweep on 2026-09-03, so the decision is usually
older than the bullet and its reasoning sits in the block's own prose above it.

## The refusal, in its own words

From '83. A mechanical rule-1 lint', under `## Follow-on`:

> Extending the check past the kernel was left out on purpose. Crates like `user_rt` legitimately
> hold `asm!` in per-ISA modules, so the rule there would be "asm lives in the ISA-suffixed
> module", which is a different check with its own false-positive surface, and `script/lint` has
> already had checks deleted for exactly that. Whether it is worth writing is a question for after
> this one has run for a while.
>
> -- design/roadmap/83-rule-1-lint.md

## Why it is here rather than only there

Rule 1 says architecture-specific code lives under `kernel/src/arch/`, and the lint checks it there.
Outside the kernel the rule is different: crates like `user_rt` legitimately hold `asm!` in per-ISA
modules, so the check would have to be "assembly lives in the ISA-suffixed module", which is a
different rule with its own false-positive surface.

## Revisit

- **Condition.** Stated in the refusal: "Whether it is worth writing is a question for after this
  one has run for a while." The evidence would be the kernel check's own false-positive record, and
  `script/lint` has deleted three checks for exactly the surface the wider rule would have.

## Index row

A lint deliberately scoped to where the rule it enforces is the rule, with the wider version left
until the narrow one has a track record. The condition is time plus evidence, which is the weakest
kind of trigger and is at least written down.
