---
status: REFUSED
raised: 2026-09-20
refused_by: 222, 448
---
# 475. Extending the accelerator probe to KVM and WHPX

Refused by milestone 222 (design/roadmap/222-hvf-leg-fails-silently.md), and
recorded there on 2026-09-03. Backfilled here on 2026-09-20 by
milestone 448 (design/roadmap/448-a-refusal-gets-a-number.md), which gave a refusal that names work a number, a
status and a condition that would change it. *(Number provisional until the merge queue lands it.)*

**The date is when the refusal was written down, not necessarily when it was made.** Most of this
tree's `- **Refused.**` bullets were written in one sweep on 2026-09-03, so the decision is usually
older than the bullet and its reasoning sits in the block's own prose above it.

## The refusal, in its own words

From '222. The one command a person runs before pushing has a leg that fails instead of skipping',
under `## Follow-on`:

> Extending the probe to the other accelerated paths, KVM on cordoba and WHPX. Neither has a leg
> in `script/gates` today, so neither can fail this way yet, and a probe guarding a leg that does
> not exist is the false-skip shape `script/lint` has deleted three checks for.
>
> -- design/roadmap/222-hvf-leg-fails-silently.md

## Why it is here rather than only there

The probe exists because an accelerated leg that fails is indistinguishable from one that skips, and
a transcript that reads as coverage when there was none is worse than a red run. KVM on cordoba and
WHPX have the same failure available to them and neither has a leg in `script/gates` today.

## Revisit

- **Condition.** Either path gaining a leg in `script/gates`. The refusal names the failure mode of
  acting early: a probe guarding a leg that does not exist is the false-skip shape `script/lint` has
  deleted three checks for, so the bell is the leg landing rather than the machine appearing.

## Index row

A guard refused for a failure that cannot happen yet, because a check with nothing to check is how
this tree has lost lints before. The condition is a specific line in `script/gates`, which makes it
one of the few conditions here that a person could grep for.
