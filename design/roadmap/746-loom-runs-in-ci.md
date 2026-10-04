---
status: BUILT
built: 2026-10-04
raised: 2026-10-03
milestone_dependencies: 80
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 746. Loom runs in CI

Raised 2026-10-03 (UTC) by the maintainer session. The number 746 is provisional until the queue
lands it, and the title and slug are drafts.

## Index row

`script/interleaving-check`, the loom harnesses for the five hand-rolled atomic protocols, runs as a
per-pull-request job in `ci.yml`.

## Why

The script from milestone 80 (Loom: the hand-rolled atomic protocols, model-checked) is the only thing in the tree that can falsify a weak-ordering mistake in
`work_steal_slot`, `clock_protocol`, `thread_wake_handshake`, `memory_corruption_canary_gate` and
`memory_regions`. No workflow ran it (`notes/check-inventory.md`, section 4), and `ci.yml` said loom
"is not a CI job at all". A regression was caught only if someone remembered to run the script.

## What shipped

- The `interleavings` job in `.github/workflows/ci.yml`: the shared documentation-only scope
  predicate, `rust-cache`, then `script/interleaving-check`, with the wall-time budget steps and a
  25-minute `timeout-minutes`.
- Per-pull-request rather than weekly, because the measured cost is small: 23.8 seconds wall clock
  for all 26 harnesses in a cold worktree on the dev machine (2026-10-03, other builds running),
  and 0.4 seconds for the falsification witness alone once built.
- The witness still fails as designed. With the `freed == 2` check in
  `the_pre_fix_protocol_double_frees_and_this_model_finds_it` mutated to `freed == 3`, the harness
  fails with "vacuous harness: no execution ever reached an execution in which both callers freed
  the same run". Reverted; nothing from that run is committed.
- The comments that said loom was not a CI job, in `ci.yml`, the script header, `notes/check-inventory.md`
  and `notes/interleaving.md`, now say it is.

## BUGS

- The job is not a required status check. Adding it to the ruleset is an architect's edit; the pull
  request body carries the ask. Until then a red `interleavings` run does not block the queue.
- Hosted runner: the job's first run (run 37168817628, 2026-10-04 UTC) took 46 seconds from start to
  completion, checkout and cache restore included, against the dev machine's 23.8 seconds.
- The `gate` job's `run=false` skip (a push to `main` already tested by a merge group) skips this job
  too, as for every other job.

## Follow-on

- **Done.** The hosted run measured 46 seconds and passed.
- **Recorded.** The required-status ask, in the BUGS section of this block.
