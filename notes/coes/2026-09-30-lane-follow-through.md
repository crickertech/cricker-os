# A correction of error: the follow-through that lived in memory

*Recorded 2026-09-30 (UTC), by the maintainer session, after calef asked for a correction of error
by name. The error is the session's, the mechanism below is the fix, and both are written here so
the next session inherits them rather than rediscovering them.*

## What happened

Five instances in one session, across 2026-09-29 and 2026-09-30 (UTC):

1. Pull request #1370's lane finished at about 21:05Z. Its verified tip sat unpushed for close to
   two hours, until a queue sweep calef asked for pushed it.
2. Pull request #1381's rebase lane finished and was never pushed. The queue had also ejected the
   pull request to draft when its base moved, so two follow-through steps were owed, not one.
3. Pull request #1443's checks went green and it was never flipped to ready, so it sat out of the
   queue with nothing wrong with it.
4. Pull request #1450's worktree, branch, claim commit and draft pull request were all created,
   and the lane itself was never dispatched. Not unlanded work; no work at all.
5. Three dispatches, the #1360 restructure, the week-notes build and the flake measurement, died
   at spawn on an API connection failure and looked dispatched. A related death ended the #1360
   lane's first attempt on a permission wall the same way: dead, but indistinguishable from busy.

   Extended 2026-09-30 (UTC), a sixth class found the same evening this record was written: work
   whose mechanism existed and was silently taken away. A queue ejection cancels the auto-merge
   request and tells nobody. Pull request #1377 ran 60 CI checks in 36 hours, 42 of them green,
   and spent the rest cancelled or ejected while main moved under it; #1454 sat CLEAN, green and
   auto-less needing one command; #1381 did the same. Green work outside the queue looks exactly
   like stalled work until a person looks.

The control: #1446's state flip needed zero maintainer attention, because a watcher script had
been armed at dispatch time. The one case with a mechanism is the one case that worked.

## Impact

Four pull requests lost one to two hours each on a night the queue was otherwise draining. calef
spent attention asking for the sweep that found them. The pattern recurs by construction: sessions
are plural, interrupts are arrival-driven, and nothing in the tree tripped on any of it.

## Root cause

Not forgetting. The follow-through step existed only in conversation and in the session's
turn-by-turn attention. This repository's own ladder calls that rung zero: somebody will notice is
not a mechanism. Dispatching a lane creates asynchronous completion with no artifact that either
fires or fails when the lane ends.

Contributing factors, none of them excusing the classification: about fifteen lanes dispatched in
one evening, and a transient API failure at spawn. The ladder exists for exactly the condition
where nobody is watching.

## The mechanism

1. A PENDING file at dispatch time (rung 2). Every dispatch writes a file beside its log, named
   for the lane, holding the exact commands the session will run when the lane reports: gates,
   push, ready, auto-merge. Executing them flips it to DONE. The file is the contract; the log is
   not.
2. Liveness at sixty seconds (rung 2). Tail the log one minute after dispatch. A connection error
   or a permission rejection means the lane never started. Redispatch, and record the death in the
   PENDING file so the count is honest.
3. Setup ends in dispatch (rung 3). Creating a worktree, claim and draft pull request is one act
   that finishes with the dispatch itself. A claim commit with no lane and no DONE marker is a
   stalled claim, and a survey raises it by name rather than passing it.
4. Armed watchers stay preferred for pull-request state changes, on the #1446 evidence.
5. Queue ejections self-heal (rung 2, commissioned 2026-09-30, not yet built): the drain re-arms
   auto-merge on any pull request that is green, CLEAN and auto-less, and says it did. Until it
   exists, every ejection is a human eye or nothing.

The rules live in [briefs/gate-a-lane.md](../../briefs/gate-a-lane.md) and
[briefs/survey-the-queue.md](../../briefs/survey-the-queue.md), in tree, so the next session reads
them rather than remembering them. The marker files are per session; the contract is not.

## EXAMPLES

The five instances above are the examples. The sweep that found them is the shape the mechanism
replaces: `git worktree list` against running lanes, `gh pr list --draft` against dispatched
claims, and a tail of every lane log. It ran because a human asked, not because a rule said so.

## BUGS

- The PENDING ledger lives in session scratch, which dies with the session. A crash mid-lane still
  loses the record; the lane's own branch and pull request are the durable half.
- Nothing yet fails a session that answers a board question while a PENDING file sits unexecuted.
  The briefs say so; a gate that reads the scratch directory would be the next rung if this fails
  again.
- The re-arm half of mechanism 5 is owed. Until the drain learns it, the sixth class of ending
  still needs the human eye this record exists to retire.

  Answered 2026-10-03 (UTC) by milestone 630 (a merge-queue ejection is caught before the queue,
  and recovered after it), with one correction to the premise. The drain already re-armed: it arms
  every eligible pull request on every pass, and since 2026-09-30 it runs on CI completion as well
  as on the schedule. What was missing was saying why a pull request left the queue, and telling a
  head worth retrying from one that would fail again. It now comments once per ejection, holds a
  head whose group run failed (label `queue-ejected`), and releases it when the head moves. See
  [queue-ejection.md](../queue-ejection.md).
