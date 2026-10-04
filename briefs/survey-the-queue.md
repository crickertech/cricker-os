Report the state of the merge queue. You are in the repository's git worktree. **Change no files.**
This is read-only and its whole output is your final message.

    git fetch origin
    gh pr list --json number,isDraft,mergeStateStatus,title,statusCheckRollup,labels
    gh pr list --label needs-maintainer --state all

## What to work out for each open pull request

- Does it carry `needs-maintainer`. The merge drain put it there for one of four causes (ejected,
  conflicting, a stale queue entry, ready and unarmed for 30 minutes) and said which in a comment.
  List these first, with the cause, whatever else the survey finds.

- Is it ready or draft, and is auto-merge already enabled.
- Is it `DIRTY` or `CONFLICTING`, which means it needs a rebase before anything else can happen.
- Does it have a failing check, and which one by name.
- Is it `BLOCKED` only by the `needs-architect` label, which is a deliberate hold rather than a
  problem, and must be reported as such rather than as a failure.

- Is the session's own ledger clean: every dispatched lane has either a DONE marker or a live
  process, and no PENDING file sits unexecuted. A claim commit with no lane behind it is a stalled
  claim, and this survey names it rather than passing it.

## The traps

- `BLOCKED` with no failing check usually means checks are still running. Count the checks with
  no conclusion yet and say so, rather than reporting it as stuck.
- `auto=false` does not mean unqueued. GitHub clears the auto-merge request once a pull request
  enters the merge queue, so a clean pull request showing `auto=false` may already be queued. Check
  `gh run list --event merge_group` before claiming anything is idle.
- A pull request absent from the default listing may have merged, not vanished. Confirm with
  `--state all` before reporting it as gone.
- A lane log that ends in a connection error or a permission rejection is a dead dispatch, not
  a running lane. The dispatch looked successful and produced nothing. Redispatch it and count
  the death, or hand it to whoever owns the dispatch
  ([notes/coes/2026-09-30-lane-follow-through.md](../notes/coes/2026-09-30-lane-follow-through.md)).

## What to report

A table: number, ready or draft, state, any `needs-maintainer` cause, and either the failing
check's name or the reason it is held. Then one short list of what needs a human: rebases needed, failures to investigate, and
anything held on `needs-architect`.

Do not rebase anything. Do not enqueue anything. Do not edit or commit.
