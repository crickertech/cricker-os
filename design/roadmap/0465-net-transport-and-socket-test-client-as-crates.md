---
status: REFUSED
raised: 2026-09-20
refused_by: 175, 448
---
# 465. `net_transport` and `socket_test_client` as crates rather than `#[path]` modules

Refused by
milestone 175 (design/roadmap/175-user-components-fixtures-split.md), and recorded there on 2026-09-13. Backfilled
here on 2026-09-20 by milestone 448 (design/roadmap/448-a-refusal-gets-a-number.md), which gave a
refusal that names work a number, a status and a condition that would change it. *(Number
provisional until the merge queue lands it.)*

**The date is when the refusal was written down, not necessarily when it was made.** Most of this
tree's `- **Refused.**` bullets were written in one sweep on 2026-09-03, so the decision is usually
older than the bullet and its reasoning sits in the block's own prose above it.

## The refusal, in its own words

From '175. Split `user/`: `components/` for services, `fixtures/` for test and benchmark programs',
under `## Follow-on`:

> Lifting `net_transport` and `socket_test_client` out of `components/src/` into crates in this
> change. This block's own "What this does not decide" leaves that open, and rule 7 permits a
> single-consumer `#[path]` module; folding it in would have put a judgment call inside the one
> mechanical commit milestone 39 asked for. `script/lint`'s consumer counter still guards the case
> that matters, and it now reads both directories.
>
> -- design/roadmap/175-user-components-fixtures-split.md

## Why it is here rather than only there

Rule 7 says anything two binaries must agree on is a crate and never a `#[path]` module, and
`script/lint`'s consumer counter enforces it by failing at two consumers. These two sit at one
consumer each, which is the case the rule permits, and lifting them would have been a judgment call
inside a commit that was deliberately mechanical. Milestone 39 (repository structure for a loosely-coupled OS) is the three-audience split the same block also put out of scope.

## Revisit

- **Condition.** A second consumer, which is the exact threshold `script/lint`'s consumer counter
  already fails at. That makes this the unusual case where the bell is an existing gate rather than a
  person: the day either module gains a second reader, the build says so.

## Index row

Two single-consumer `#[path]` modules that rule 7 permits and that would become crates the moment
either gained a second reader. The refusal was about keeping a mechanical commit mechanical, and the
condition is already wired into a check that fails on its own.
