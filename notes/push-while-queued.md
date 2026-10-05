# A push to a queued branch

Provisional names: `helpers/push-queued-check.sh`, `helpers/push-queued-selftest.sh` and the
override `NIFE_PUSH_QUEUED`.

### The measurement Over about 28 hours ending 2026-10-05, 20 of 40 non-merge removals from the
queue had reason `manual`. The seven checked were a push or force-push to a branch already queued,
or a convert-to-draft. Two pull requests each lost three entries in two minutes to repeated pushes.
Every removal throws away that pull request's group build and rebuilds every group behind it.

### What GitHub does on a push to a queued branch The pull request leaves the queue
(`RemovedFromMergeQueueEvent`, reason `manual`) and does not come back by itself; auto-merge goes
with it, so re-arming is needed: `gh pr merge N --auto --merge`. Evidence is in the
timelines of the 30 most recently updated pull requests. #1617 has `AddedToMergeQueueEvent` at
22:19:52Z and `RemovedFromMergeQueueEvent` (`manual`) at 22:22:38Z on 2026-10-04. Its next
`AutoMergeEnabledEvent` is at 00:40:24Z, set by hand after three more commits. Nothing queued it
in between. This is an inference from timelines, not a controlled push. A draft cannot be queued, and the lane was told not to arm its pull request. A session
that wants the controlled version can queue a trivial pull request and push to it. Also, `autoMergeRequest` is already null while queued (see merge-queue.md), so `gh pr merge
--disable-auto` does not dequeue anything. The mutation that does is `dequeuePullRequest`, whose one
input field is the pull request's node id. That is checked by `__type` introspection and not yet run on a
live queued entry.

### The check `.githooks/pre-push` pipes git's ref lines to `helpers/push-queued-check.sh`, which
asks `gh api graphql` once per pushed branch for `mergeQueueEntry{state}` on the open pull request
with that head ref. Queued: exit 1 with the pull request number, the dequeue command and the
override. Anything else, including `gh` missing, unauthenticated, offline or GraphQL blocked: one
warning line and the push goes. It runs before the empty-push skip, because an empty push to a
queued branch dequeues it too.

## EXAMPLES

    $ git push --force-with-lease
    push-queued-check: pull request #1617 for lane/x is in the merge queue, and a push removes it.
      Better: take it out first, push, then arm it again after your LAST push:
        gh api graphql -f query='mutation{dequeuePullRequest(input:{id:"PR_..."}){clientMutationId}}'
        git push ...
        gh pr merge 1617 --auto --merge
      If this push is intended: NIFE_PUSH_QUEUED=1 git push ...

    $ NIFE_PUSH_QUEUED=1 git push      # you mean it; the queue entry is lost

The self-test (`helpers/push-queued-selftest.sh`, run by `script/lint`) uses a fake `gh` on PATH for
queued, not queued, `gh` failing, the override, no `gh` and a tag push.

## BUGS

The queued-branch push check only fires for pushes from a machine with the hook installed. The
hook comes from `script/setup` via `core.hooksPath`, so a lane in a cloud session, a fresh clone
that never ran setup, and `git push --no-verify` all push to a queued branch unimpeded. The check
also fails open when `gh` cannot answer.

Converting a queued pull request to draft also removes it from the queue (reason `manual`), and this check cannot see it, because it runs on push only. Closing the cloud gap needs a server-side answer (a workflow that comments when a
queued pull request's head moves), which nobody has written.
