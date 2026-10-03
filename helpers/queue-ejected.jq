# helpers/queue-ejected.jq: what the merge drain should do about a merge-queue ejection, in one place.
#
# Milestone 630 (a merge-queue ejection is caught before the queue, and recovered after it). An
# ejection cancels the pull request's auto-merge request and tells nobody
# (notes/coes/2026-09-30-lane-follow-through.md, the sixth class): #1377 ran 60 checks in 36 hours,
# #1454 and #1381 each sat green and auto-less needing one command, and #1473 was ejected at
# 2026-10-03 01:45 UTC on a check that could only fail inside the group. Every group attempt costs
# 20 to 38 minutes, so re-arming the SAME head is the wrong recovery: a deterministic failure fails
# again, and the drain re-arms everything eligible on every pass. So the head that was ejected is
# held, and the hold releases itself when the head moves.
#
# `ejection_state($label)` reads one pull request node from the drain's GraphQL query:
#     { number, headRefOid, labels { nodes { name } },
#       removed: timelineItems(last: 1, itemTypes: [REMOVED_FROM_MERGE_QUEUE_EVENT])
#                { nodes { createdAt reason beforeCommit { oid parents(first: 2) { nodes { oid } } } } },
#       added:   timelineItems(last: 1, itemTypes: [ADDED_TO_MERGE_QUEUE_EVENT]) { nodes { createdAt } } }
# and emits one object for a pull request that has a CURRENT ejection or carries `$label`, and
# nothing for any other.
#
# A current ejection is a last removal whose reason is neither `merged` nor `manual`, with no
# enqueue after it. `manual` is excluded because a person, or the drain's own `dequeue_held`, took
# it out on purpose and already said why; reporting a deliberate act back to its actor is noise.
# Reasons seen on this repository by 2026-10-03: merged 76, failed_checks 18, manual 6,
# merge_conflict 4. An unknown reason counts as an ejection, which is the direction to fail in.
#
# `beforeCommit` is the merge group's commit, not the pull request's head (verified on #1473:
# 47d8f11e is "Merge pull request #1473", the head_sha of the failed group runs). Its second parent
# is the head that was enqueued, which is what the hold compares against.
#
# `action` is the decision, and the shell only carries it out:
#   hold     ejected, and the head is still the one that was ejected, or the event does not say
#            which head it was (a `merge_conflict` has no group commit). Comment once; label, and do
#            not arm while the label is on, only when a group run FAILED or TIMED OUT. 13 of the 18
#            `failed_checks` ejections on this repository by 2026-10-03 were a CI run CANCELLED in the
#            group, not failed, and #1454 merged on its fifth attempt at an unchanged head, so a
#            cancellation is re-armed at the same head, as the drain always did. Removing the label by
#            hand is how a person says "that failure was a flake, try this head again", and the drain
#            never puts it back for the same ejection.
#   moved    ejected, and a new head has been pushed since. Take the label off; the drain's ordinary
#            pass arms it, and the queue admits it only when its own checks are green.
#   release  labelled, with no current ejection (enqueued again since, or removed by hand).
#            Take the label off.
#
# helpers/queue-ejected-selftest.sh checks this against fixtures; script/lint runs it.
def ejection_state($label):
  (.removed.nodes[-1] // null) as $r
  | (.added.nodes[-1].createdAt // "") as $added
  | ([.labels.nodes[].name] | index($label) != null) as $labelled
  | ($r != null and $r.reason != "merged" and $r.reason != "manual" and $added < $r.createdAt) as $ejected
  | ($r.beforeCommit.parents.nodes[1].oid // null) as $was
  | select($ejected or $labelled)
  | { number, labelled: $labelled, head: .headRefOid,
      reason: (if $ejected then $r.reason else null end),
      at: (if $ejected then $r.createdAt else null end),
      group: (if $ejected then $r.beforeCommit.oid else null end),
      ejected_head: (if $ejected then $was else null end),
      action: (if $ejected and ($was == null or $was == .headRefOid) then "hold"
               elif $ejected then "moved"
               else "release" end) };
