---
status: BUILT
raised: 2026-10-03
built: 2026-10-03
promoted_from: a-stopped-merge-watcher-is-reported-at-once
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 723. A stopped merge watcher is reported within three of its own intervals

Raised 2026-10-03 (UTC) by the maintainer session writing
the merge-rate correction (`notes/coes/2026-10-03-the-merge-rate.md`, on PR #1513's branch until it lands). Name provisional,
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

## What shipped

`helpers/watcher_watch.py` (`check`, `sync`, `--selftest`). `script/cadence-check` calls `check` for
the two merge watchers and no longer reads them through its 15-day rule. A watcher is reported when
its workflow state is anything but `active`, which is immediate, or when it has had no run of any
kind for its silence limit. `sync` opens one issue per stopped watcher and closes it when the watcher
runs again. `merge-drain.yml` and `trunk-health.yml` each end with a `sync` step, so each reports the
other, and a running watcher closes its own issue. It was exercised against the live API: both
watchers read live and `sync` opened nothing.

## One departure, ruled by calef

The ruling was three intervals, 15 minutes for the drain. Measured the same day, GitHub does not
deliver the five-minute cron at all. `trunk-health` ran 51 times in nine days with a median gap of
281 minutes and a longest of 499, all from `schedule`. The drain's gaps are a median of 2 minutes
and a longest of 140, because `workflow_run` after CI supplies most of its runs. Three intervals
would call `trunk-health` silent on every run it has, and the drain on every idle stretch.

So the silence limit is three intervals but never below a measured floor, 180 minutes for the drain
and 720 for `trunk-health` (`MEASURED_FLOOR_MIN`). The state check, which is the whole of the
2026-09-30 incident, has no floor. Deleting the floors restores the ruling to the letter, and is one
line each.

**Ruled 2026-10-03 (UTC) by calef:** keep the measured floors, 180 minutes for the drain and 720 for
`trunk-health`, on the silence check. The disabled-state check stays immediate. Fixing delivery so
15 minutes becomes true is not asked for.

## BUGS

- The floors rest on nine days of history for trunk-health and nine hours for the drain, and GitHub's
  delivery moves with its load. A real stop shorter than the floor is caught only by the state check.
- Both watchers stopping together is reported by nobody. A third party outside this repository is the
  only fix.
- The issue is opened as `nife-smelter[bot]` only if the App holds Issues permission, which milestone
  128's block says it was minted without (Contents and Pull requests). The first stop will use the
  fallback token and show as `github-actions[bot]`; the log names which. Granting the App Issues
  write is calef's, in the GitHub settings.
- It reads the last run, not the last success: a watcher that runs and fails every time reads live.
- `sync` writes an issue on a transition only, so a watcher that stays stopped is one issue and is
  not re-announced.

## Follow-on

- **Recorded.** The App's Issues permission and the floors, in this block's BUGS; both are calef's.
- **Recorded.** The other scheduled workflows are still on the 15-day rule. This block's own
  "Why" section assigns them to #1511 (the scheduled-workflows milestone, not yet merged).

## Index row

BUILT on `milestone/723-watcher-stop` (PR #1537). cadence-check derives intervals from cron and reports a disabled or silent merge watcher within three intervals as an issue; the drain sat disabled 62 hours and was called live.
