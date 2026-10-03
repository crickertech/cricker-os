---
status: PROPOSED
raised: 2026-10-03
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# A stopped merge watcher is reported within three of its own intervals

Raised 2026-10-03 (UTC) by the maintainer session writing
[the merge-rate correction](../../../notes/coes/2026-10-03-the-merge-rate.md). Name provisional,
this file's alone.

## Why

The merge-drain workflow sat `disabled_manually` from about 21:40 UTC on 2026-09-30 to the morning
of 2026-10-03, and in the week before that its five-minute cron ran 37 of 2,016 scheduled times.
Nothing reported either. `script/cadence-check` calls a workflow dead only when its last success is
over 15 days old, so on 10-03 it still called the drain live, 62 hours after its last run. The
session-start brief does check that the drain is `active`, but no session that ran the brief started
in those 62 hours.

The drain is what lands green work, so a stopped drain is a stopped merge rate. That is the link to
the correction. Scheduled workflows that check rather than merge (mutation, the audits, the
toolchain watches) belong to #1511 (the scheduled-workflows milestone, not yet merged), not here.

## What to build

1. `cadence-check` derives each workflow's expected interval from its own cron, which its header
   has recorded as owed since 2026-09-17. For the merge watchers (merge-drain, trunk-health), dead
   means no run in three intervals: 15 minutes for the drain, not 15 days.
2. A merge watcher whose state is anything but `active` is reported at once.
3. The report reaches a reader. Today it is a `::warning::` annotation inside a green `trunk-health`
   run. It should open one issue per stopped watcher, as `nife-smelter[bot]`, closed by the bot when
   the watcher runs again. A stopped `trunk-health` cannot report itself, so the drain reports it and
   it reports the drain.

## Index row

cadence-check derives intervals from cron and reports a disabled or silent merge watcher within three intervals as an issue; the drain sat disabled 62 hours and was called live.
