# A merge-queue ejection, caught before the queue and recovered after it

Milestone 630 (a merge-queue ejection is caught before the queue, and recovered after it),
2026-10-03 UTC, and its recovery half rebuilt the same day as milestone 727 (a queue eviction goes
to a maintainer session), provisional. The name of this note, the `needs-maintainer` label and the
`ready status` workflow are provisional.

Every merge-group attempt costs 20 to 38 minutes since swish-check joined CI. So an ejection costs
twice: once for the group that failed, and again for every retry that fails the same way. The
ejection also cancels auto-merge and tells nobody, which is the sixth class in
[the follow-through COE](coes/2026-09-30-lane-follow-through.md). Pull request #1473 met both at 01:45 UTC.
Its own roadmap block still read IN-PROGRESS, and `script/roadmap --check` can only see that
inside a merge group, because only there does the branch's merge commit exist.

## Before the queue: the ready check

`script/roadmap --ready-branch <branch>` reads frontmatter and nothing else. It fails when a block
still reads IN-PROGRESS on that branch, and names the block and the fix. `script/roadmap
--selftest` proves it against fixtures under `script/lint`.

`.github/workflows/ready-status.yml` runs it on `pull_request` (opened, reopened, synchronize,
ready_for_review, converted_to_draft) and on `merge_group`. It reads the live draft state, and a
draft passes, because IN-PROGRESS is a draft's honest status. The pre-push hook asks the same
question when the pushed branch's pull request is ready, and skips silently without `gh`, a
network or a pull request.

Falsified live with #1491, a throwaway whose block read IN-PROGRESS on its own branch:

| Step | Run | Result |
| --- | --- | --- |
| Opened as a draft | 37088183446 | success |
| `gh pr ready` | 37088221768 | failure, 11 s after the event, fix in the log |
| `gh pr ready --undo` | 37088256676 | success |

It was closed and its branch deleted the same minute.

## The pre-push hook runs what fits in seconds

The hook grew one gate per lost queue cycle: clippy, two prose ratchets, then the baselines check.
Milestone 630 (a merge-queue ejection is caught before the queue, and recovered after it) made it
all of `script/lint`. Measured on patagonia with a warm target directory, three runs each, wall
clock, beside two lanes' QEMU:

| Command | Runs (s) | Median |
| --- | --- | --- |
| `script/lint --clippy` | 27.36, 32.90, 26.71 | 27.4 s |
| `script/lint` | 56.48, 55.20, 56.65 | 56.5 s |

That was a quiet moment. Later the same day, with the load average at 11 to 19, `script/lint` took
70.3, 110.7 and 119.4 s warm and 201.9 s with a cold target directory. A claim push sat in it for a
whole lane and lanes took to `--no-verify`. The hook also refused a claim push five times for a
roadmap block that cannot exist before the claim.

calef approved the narrower hook on #1564 (2026-10-03 UTC). It runs `script/fmt --check`, then
`script/lint --no-cargo`, then `--ready-branch`, and skips all three for a push whose commits
change no files. Clippy stays in CI. Measured on patagonia at load 15 to 17, wall clock:

| Step | Runs (s) |
| --- | --- |
| `script/fmt --check` | 1.8 |
| `script/lint --no-cargo`, warm tree | 41.6, 47.2, 44.7 |
| `script/fmt --check` and `script/lint --no-cargo`, no `target/` directory | 43.0 |

The proposal estimated 33 s from one loaded run. The measurement is 42 to 47 s, under a load average
of 15 to 17. The mode is CPU bound (about 46 s of CPU in those runs) and never touches `target/`, so
a cold target directory costs nothing extra. An empty push costs the
hook's own startup, under a second.

**The partition is read out of `script/lint`, not listed beside it.** `helpers/lint-no-cargo.awk`
splits the script at each `echo "==> ` header and drops every section with a line that invokes
cargo (a command whose first word is `cargo`, a Python `["cargo"` argv, or `command -v cargo`).
Prose that mentions cargo does not count. A new check therefore lands in exactly one bucket when it
is written, and a section that runs cargo is dropped unless it carries a `# no-cargo-ok:` marker
and reads `$LINT_NO_CARGO` to skip its cargo part. A marker on a section with no cargo, a marker
with no guard, and cargo before the first header each fail the mode. Of 65 sections, 46 run and 19
are skipped (12 clippy, rustdoc, and 6 that call `cargo metadata` or `cargo machete`;
`script/lint --no-cargo --list` prints them).

The earlier run found one defect too. git exports GIT_DIR to a hook, and
`helpers/scope-merge-base-selftest.sh` builds a throwaway repository with it still set. Its
`git init` and `git config` wrote `core.bare = true` and a fake identity into the shared
`.git/config` before it failed. Both were restored by hand, no commit carried the identity, and
the hook and the selftest now clear git's environment first.

## After the queue: a label for a maintainer session

Milestone 630 first built this as a hold: the drain commented on an ejection, labelled
`queue-ejected`, and declined to re-arm a failed head, re-arming everything else. calef's rulings on
#1564 (2026-10-03) removed every re-arm and re-queue, because the automation fought his own
dequeues and still left him as the only detector
([the correction](coes/2026-10-03-the-queue-judged-one-pull-request-at-a-time.md)). What replaced it
labels and never acts on the queue.

`helpers/merge-drain.sh` asks GraphQL once per pass for every open pull request (its last
`RemovedFromMergeQueueEvent` and `AddedToMergeQueueEvent`, `mergeable`, auto-merge, and when it was
last unarmed), the queue's entries, and the closed pull requests still wearing the label.
`helpers/needs-maintainer.jq` decides. Each cause adds `needs-maintainer` and posts one comment per
episode, deduplicated by a marker:

| Cause | When | Evidence in the comment | Cleared when |
| --- | --- | --- | --- |
| ejected | last removal neither `merged` nor `manual`, nothing re-added it, same head | reason, time, head, the group's failed runs | back in the queue, or the head moves |
| conflict | ready and `mergeable` CONFLICTING | the conflicting files, from `git merge-tree` | the conflict is gone |
| stale | a queue entry whose pull request is merged or closed | entry state, enqueue and merge times, the dequeue command | the entry is gone |
| unarmed | ready, not armed, not queued, 30 minutes since it was last unarmed | since when, and any resolved `Blocked-by:` | armed, queued, or a draft again |

Every cause but `stale` needs a ready pull request into `main` from this repository, without
`needs-architect` or `held-for-red-trunk`. A `merge_conflict` ejection names no group commit, so it
cannot say which head was ejected and is cleared by the conflict going instead. `manual` is not an
ejection, because a person or `dequeue_held` meant it; if nobody follows up, it is `unarmed` 30
minutes later. `unarmed` is not raised beside `ejected` or `conflict`, which already say what is
wrong.

The event's `beforeCommit` is the group's merge commit, not the head, which was a surprise. Its
second parent is the head that was enqueued, and the group's runs are the `merge_group` runs at
`beforeCommit`.

The split milestone 630 measured still matters to whoever reads the comment. Of the 18
`failed_checks` ejections by 2026-10-03, 5 had a CI run conclude `failure` and 13 had CI
`cancelled` with every other workflow green, and #1454 merged on its fifth attempt at an unchanged
head. So the comment lists the group's runs by conclusion: a cancellation is usually runner supply,
and re-arming is the session's call to make, not the drain's.

`helpers/needs-maintainer-selftest.sh` feeds the decision a response recorded live at
2026-10-03T23:54:15Z, when #1569 had just been ejected on a `merge_conflict`, and one case per
cause and per clearing. A `--dry-run` pass against the live repository the same minute labelled
#1569 and nothing else. The scheduled workflow runs only from `main`, so the first real label comes
after merge.

The log's event lines:

```console
$ gh run view <run id> --log | grep -E 'LABELLED|CLEARED'
```

## BUGS

- A labelled pull request waits for a maintainer session. With none running, it waits for the
  next one, which is slower than the drain's old re-arm and is not calef's job.
- An ejection is labelled at the drain's next pass, which follows the group's CI completion through
  `workflow_run` but can trail it under load. The first pass after this lands also labels any open
  pull request whose last removal was a current ejection.
- An armed pull request whose required checks never report, or that auto-merge never enqueues, is
  none of the four causes. The drain used to comment on the first and enqueue the second.
- `--ready-branch` matches by branch name. A block naming the wrong branch passes it and still
  fails in the group.
- `notes/check-inventory.md` does not list `--ready-branch` or the hook's wider lint. That table is
  a 2026-09-03 snapshot already over its word budget, and a row costs words it does not have.
