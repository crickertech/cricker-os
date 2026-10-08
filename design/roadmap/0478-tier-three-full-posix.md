---
status: SUPERSEDED
raised: 2026-09-20
superseded_by: 835, 836, 837, 838
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 478. Tier three: full POSIX behind the foreign-language seam

Superseded 2026-10-08 (UTC), by milestones 835 to 838 (a C library in four stages: files, clock and
memory; threads; sockets; `posix_spawn`). Its revisit condition came true with the benchmark ruling
(PR #1854), which requires the field's standard benchmarks to run unmodified. The nine programs it
mints as milestones 826 to 834 are POSIX C, and adapting them to a narrower tier is exactly what
that ruling forbids. calef chose relibc the same day, and §265 (a C library started from relibc)
records the ruling and amends the first rule of §31 (the foreign-language seam). The refusal is kept
below because it is the argument the stages answer. They took their own numbers rather than reviving
this one, because what is built is not this milestone's subject. Full POSIX includes `fork`, which
the fork ruling (PR #1856) declines permanently. So "tier three" is now three stages and a spawn,
with no fork in any of them.

Refused by milestone 36 (design/roadmap/0036-foreign-component.md), and recorded
there on 2026-09-03. Backfilled here on 2026-09-20 by
milestone 448 (design/roadmap/0448-a-refusal-gets-a-number.md), which gave a refusal that names work a number, a
status and a condition that would change it. *(Number provisional until the merge queue lands it.)*

**The date is when the refusal was written down, not necessarily when it was made.** Most of this
tree's `- **Refused.**` bullets were written in one sweep on 2026-09-03, so the decision is usually
older than the bullet and its reasoning sits in the block's own prose above it.

## The refusal, in its own words

From '36. A foreign-language component, seam first (spike; feeds 29 and 23)', under `## Follow-on`:

> Tier three, full POSIX (`open`, `fork`, `socket`, threads), stays out. It needs a real libc
> port, which DECISIONS §15 prices at "later, if ever", and a component that wants it is a
> different and much larger project than this one.
>
> -- design/roadmap/0036-foreign-component.md

## Why it is here rather than only there

The foreign-language seam has tiers, and the third is `open`, `fork`, `socket` and threads: enough
POSIX that arbitrary C could be ported rather than adapted. It needs a real libc port, which §15 (the native ABI: formalize the convention) prices at "later, if ever", and a component that wanted it would be a different and
much larger project than this one.

## The condition it was refused under, now met

- **Condition.** A component somebody needs that cannot be adapted to the narrower tiers. Nothing on
  the roadmap has asked, and the refusal's phrasing is a pricing rather than a prohibition, which is
  why this is recorded as refused rather than as impossible.
- **Met, 2026-10-08 (UTC).** The benchmark ruling's nine standard benchmarks (milestones 826 to 834,
  on PR #1854) are the component, and they cannot be adapted by rule. See the paragraph at the top.

## Index row

The largest single thing this tree had declined to build, refused for want of a component that
would need it. The standard benchmarks became that component on 2026-10-08, and a C library started
from relibc, staged by consumer and without fork, replaces it.
