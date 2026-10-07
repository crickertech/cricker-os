---
status: REFUSED
raised: 2026-09-20
refused_by: 164, 448
---
# 462. The cost of bitsliced AES against AES-NI, unmeasured on purpose

Refused by milestone 164 (design/roadmap/164-x86-64-fs-server-aes.md), and
recorded there on 2026-09-03. Backfilled here on 2026-09-20 by
milestone 448 (design/roadmap/448-a-refusal-gets-a-number.md), which gave a refusal that names work a number, a
status and a condition that would change it. *(Number provisional until the merge queue lands it.)*

**The date is when the refusal was written down, not necessarily when it was made.** Most of this
tree's `- **Refused.**` bullets were written in one sweep on 2026-09-03, so the decision is usually
older than the bullet and its reasoning sits in the block's own prose above it.

## The refusal, in its own words

From "164. x86_64 userspace can't build `aes` (and therefore `fs_server`): no SSE, no scalar
fallback", under `## Follow-on`:

> The soft-AES cost stays unmeasured on purpose. Upstream puts AES-NI roughly an order of
> magnitude ahead of the bitsliced backend, but nothing on x86_64 mounts an encrypted RedoxFS
> volume yet, so there is no workload and a synthetic number would be a fact leaving the machine
> with nothing behind it. The number is owed when an x86_64 workload touches the crypto path, and
> Route 2 is what it would be weighed against.
>
> -- design/roadmap/164-x86-64-fs-server-aes.md

## Why it is here rather than only there

Upstream puts AES-NI roughly an order of magnitude ahead of the portable bitsliced backend. This
tree declined to produce its own number, which is the right call under its own rule about facts that
leave the machine: a benchmark with no workload behind it is a number a stranger can quote and
nobody can defend.

## Revisit

- **Condition.** Stated in the refusal and unchanged: "The number is owed when an x86_64 workload
  touches the crypto path." The refusal also names what the number would be weighed against, which is
  the SSE-enabled userspace target recorded as
  milestone 461 (design/roadmap/461-an-sse-enabled-x86-64-userspace-target.md), so the two move together.

## Index row

A measurement deliberately not taken, because there was no workload to take it on and a synthetic
number would be a fact leaving the machine with nothing behind it. The condition is stated in the
original and this records it where the bell can ring on it.
