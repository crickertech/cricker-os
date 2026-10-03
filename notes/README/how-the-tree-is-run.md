# Notes index: How the tree is run

Gates, records and the merge queue: the machinery that keeps many lanes honest.

Part of [the notes index](../README.md), which says how to add a line.

- [The `script/` entry points](../scripts.md): the normalized front-door commands and what each does.
- [Every check in this repository](../check-inventory.md): audit of what runs, blocks, and asserts. Name provisional.
- [Selectors that can select nothing](../empty-selectors.md): gates that pass when their pattern matches nothing. Name provisional.
- [What to do when `main` goes red](../main-is-red.md). Names provisional.
- [The merge queue, and the three things that watch it](../merge-queue.md): the scripts that land, watch, and flag queue work. Names provisional.
- [A merge-queue ejection, caught before the queue and recovered after it](../queue-ejection.md): the ready check, the whole lint in the pre-push hook, and the drain's hold on an ejected head. Names provisional.
- [A paused draft names its blocker, and the drain reads it back](../blocked-by-drafts.md): `Blocked-by:` on a draft pull request, and the `unblocked` label. Name provisional.
- [Working from a cloud session](../working-from-a-cloud-session.md): what past cloud sessions hit, how to set up, claim and gate in CI, and what needs patagonia. Name provisional.
- [The automation's own identity](../automation-identity.md): the `smelter` GitHub App that replaces a personal token. Name provisional.
- [Hardening the repository itself](../repo-hardening.md): the GitHub settings that cannot be committed.
- [The roadmap](../roadmap.md): how to add a milestone, and its vocabularies.
- [Follow-on work, and what happened to it](../follow-on-work.md): the Follow-on section every finished block must answer. Name provisional.
- [The untracked-work sweep, and what each finding became](../untracked-work-sweep.md).
- [The dependency census](../dependency-census.md): real prerequisite edges between milestones, measured against declared ones.
- [Citations that name what they cite](../citations.md).
- [Counted claims](../counted-claims.md): numbers in prose that a gate re-derives. Name provisional.
- [The register of measures](../register-of-measures.md): the numbers this kernel holds itself to. Name provisional.
- [Project metrics: what moved, week by week](../project-metrics.md): weekly charts of the project's measures, from git history. Script and data names provisional.
- [How stable the interface is](../interface-stability.md): what the interface is, the weekly counts of what broke and what grew, the proposed threshold, and what the counts cannot see. Names provisional.
- [The violation ledger](../rule-violations.md): counting how often each written rule is broken. Name provisional.
- [Load-sensitive assertions](../load-sensitive-assertions.md): the register of assertions that fail under host load, how to fix one, and each site's status. Appendix names provisional.
- [The CI log baseline](../ci-log-baseline.md): which check failed each CI job, from expiring logs. Names provisional.
- [Every place that enumerates architectures, and whether the list is complete](../architecture-list-sweep.md).
- [Rustdoc coverage](../doc-coverage.md): the doc-example floor and the `missing_docs` ratchet.
- [The documentation sweep](../documentation-audit.md): how to run a documentation sweep, and what counts.
- [Prior art and reuse](../prior-art.md): where to look before building, and the build-versus-reuse rule.
- [Handing a session over](../session-handoff.md): superseded 2026-07-29 restart point, kept as history.
- [Corrections of error](../coes/README.md): the register of recorded errors and the mechanisms
  that replaced them, one dated file each.
- [Cobble, the mascot](../mascot.md): the project's mascot, drawn by Clay.
- [The system tests and the kernel crate](../system-tests-and-the-kernel-crate.md): what the kernel linked for its tests, the gate that keeps them out, and where the tests go. Name provisional.
- [The swish-check CI flake, measured](../swish-check-flake.md): two timing-race reds at one gate, counted over 30 days, explained mechanically. Name provisional.
