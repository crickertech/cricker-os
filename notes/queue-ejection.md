# A merge-queue ejection, caught before the queue and recovered after it

Milestone 630 (a merge-queue ejection is caught before the queue, and recovered after it),
2026-10-03 UTC. The name of this note, the `queue-ejected` label and the `ready status` workflow
are provisional.

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

## The pre-push hook runs all of lint

The hook grew one gate per lost queue cycle: clippy, two prose ratchets, then the baselines check.
Each new lint check stayed outside it until it cost a cycle too. Measured on patagonia with a warm
target directory, three runs each, wall clock, beside two lanes' QEMU:

| Command | Runs (s) | Median |
| --- | --- | --- |
| `script/lint --clippy` | 27.36, 32.90, 26.71 | 27.4 s |
| `script/lint` | 56.48, 55.20, 56.65 | 56.5 s |

The rest of lint is about 29 s, of which the three gates it replaced were about 6. No check after
clippy takes over 3.5 s (spelling 3.4, prose ratchet 3.0, counted claims 2.6, citations 2.6). There
was no slow tail to leave to CI, so the hook runs the whole thing. Its first push in this lane
refused a citation to an unmerged milestone that CI would have failed.

## After the queue: the drain

`helpers/merge-drain.sh` reads every open pull request's last `RemovedFromMergeQueueEvent` and last
`AddedToMergeQueueEvent` in one GraphQL call per pass. `helpers/queue-ejected.jq` decides, with
fixtures in `helpers/queue-ejected-selftest.sh`. A current ejection is a last removal whose
`reason` is neither `merged` nor `manual`, with no enqueue after it.

The event's `beforeCommit` is the group's merge commit, not the head, which was a surprise. Its
second parent is the head that was enqueued, and the group's runs are the `merge_group` runs at
`beforeCommit`.

| Case | Comment | Label | Arming |
| --- | --- | --- | --- |
| A group run failed or timed out, head unchanged | once | applied | held |
| Group runs cancelled, head unchanged | once | none | re-armed at the same head |
| `merge_conflict` (no group commit) | once | none | the existing `DIRTY` stall |
| A new head was pushed | once, if not yet said | removed | armed as usual |
| Back in the queue, or removed by hand | none | removed | as usual |

The split between failed and cancelled is measured. Of the 18 `failed_checks` ejections on this
repository by 2026-10-03, 5 had a CI run conclude `failure` (#1402, #1409, #1442, #1443 and #1473).
The other 13 had CI `cancelled` with every other workflow green. Three pull requests (#1450, #1454 and #1457) were each
ejected four times at an unchanged head, and #1463 once. Pull request #1454 merged on its fifth attempt with the
head it had on its first, so holding a cancelled head would have stranded it.

Removing the label by hand says "that failure was a flake, retry this head". The drain never
re-applies it for the same ejection. A `needs-architect` pull request is commented and labelled
like any other and never armed, because the drain's queue excludes it.

The log gains two event lines, countable like `ARMED`:

```console
$ grep -c 'merge-drain: EJECTED #' ~/Library/Logs/nife/merge-drain.log    # ejections said on the pull request
$ grep -c 'merge-drain: RELEASED #' ~/Library/Logs/nife/merge-drain.log   # holds released
```

Exercised before landing by stubbing `gh`'s write commands and running the handler against the
live repository. Pull request #1473 classified as moved, since calef pushed a fix three minutes after its
ejection. A fixture with the head unchanged produced one hold, one label and one comment. The
scheduled workflow runs only from `main`, so the first real comment comes after merge.

## BUGS

- `ready status` is not a required check, so it informs rather than blocks a pull request armed
  by hand. The drain will not arm one with a failing check, which covers the normal path. Adding
  `ready status (no IN-PROGRESS block on a ready branch)` to the `main` ruleset is calef's.
- The hold needs `nife-smelter` to apply a label it may have to create. The App holds Contents and
  Pull requests write. Labelling a pull request needs only the second; creating the label may
  need Issues. If it fails, the drain prints `STALLED. #N could not be labelled` and re-arms the
  head as before. One `gh label create queue-ejected` by a person removes the question.
- An ejection is said at the drain's next pass, which can trail it by minutes. The first pass after
  this lands also comments on any open pull request whose last removal was an ejection.
- `--ready-branch` matches by branch name. A block naming the wrong branch passes it and still
  fails in the group.
- `notes/check-inventory.md` does not list `--ready-branch` or the hook's wider lint. That table is
  a 2026-09-03 snapshot already over its word budget, and a row costs words it does not have.
