---
status: BUILT
raised: 2026-10-03
built: 2026-10-04
promoted_from: a-queue-eviction-goes-to-a-maintainer-session
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 727. A queue eviction goes to a maintainer session

Raised 2026-10-03 (UTC) by calef's rulings on #1564, the correction of error
[`notes/coes/2026-10-03-the-queue-judged-one-pull-request-at-a-time.md`](../../notes/coes/2026-10-03-the-queue-judged-one-pull-request-at-a-time.md). The number is provisional until the merge queue lands it, and so are the title, the slug and
the `needs-maintainer` label's name.

## Why

calef, 2026-10-03: *"I don't want the job of watching the queue."* That day he was its only
working detector. He spotted each conflicting or failing pull request, removed it, and told the
maintainer session, while the merge drain armed, re-armed and re-queued on his behalf (six or seven
times over his own dequeues) and commented where nobody looked. Two rulings followed, and they land
together, because removing the re-arming without a detector puts the queue-watching back on calef.

1. Strip the drain back: no arming, no re-arming or re-queueing, no stall comments. Keep the
   architect-label bots, `architect hold` and `ready status`, with no API call in the group leg.
2. Build one detector that labels a pull request a maintainer session must pick up, and never
   re-queues or arms.

## Built

1. `helpers/merge-drain.sh` arms nothing. Its arming, stranded enqueue, stall comments,
   stale-draft and stuck-check reports and milestone 630 (a merge-queue ejection is caught before the queue, and recovered after it)'s ejection hold are gone, with
   `helpers/queue-stranded.jq`, `helpers/queue-ejected.jq` and their selftests. It keeps
   `dequeue_held`, the `unblocked` draft label, the cancelled-duplicate rerun and the lane-claim
   check, and gains `--dry-run`.
2. `helpers/needs-maintainer.jq` decides four causes: ejected, conflicting, a stale queue entry,
   and ready but unarmed for 30 minutes. The drain labels, comments once per cause per episode, and
   clears the label when the cause goes. The workflow creates the label idempotently and retires
   `queue-ejected`.
3. `helpers/needs-maintainer-selftest.sh` feeds it a response recorded live (#1569, ejected and
   conflicting) and one case per cause and per clearing. It also fails if the drain arms or
   enqueues again. `script/lint` runs it.
4. `ready status`'s group leg reads the branch from the group commit's subject, not the API.
5. The metrics, toolchain-bump and vendor-watch workflows arm the pull requests they open.
6. The session-side half, rung three: `briefs/session-start.md` reads the label first, and
   `helpers/nanny.py` wakes a running session when it lands.

`notes/queue-ejection.md` has the four causes and the comment each one posts.

## BUGS

- With no session running, a labelled pull request waits for the next one.
- A lane that forgets to arm is caught after 30 minutes, not 5.
- An armed pull request whose checks never report, or that auto-merge never enqueues, is not one
  of the four causes. The drain used to report the first and enqueue the second.

## Follow-on

- **Recorded.** An armed pull request whose checks never report, or that auto-merge never
  enqueues, is none of the four causes; beside the detector in `notes/queue-ejection.md`.
- **Recorded.** `helpers/nanny.py` still enqueues a stranded armed pull request with the session's
  token, the last automatic enqueuer; in its own header, `helpers/nanny.py`.
- **Recorded.** A finished draft never marked ready is no longer reported, since the stale-draft
  note went with the stall comments; in `notes/blocked-by-drafts.md`.

## Index row

BUILT on `lane/eviction-detector` (PR #1572). The merge drain stops arming and re-queueing, and labels an ejected, conflicting, stale or unarmed pull request `needs-maintainer` for a session to pick up, so calef no longer watches the queue.
