# helpers/needs-maintainer.jq: which pull requests a maintainer session must pick up, and why.
#
# Milestone 727 (a queue eviction goes to a maintainer session), number provisional. calef's ruling
# on #1564, 2026-10-03 UTC: "I don't want the job of watching the queue." That day he was the
# queue's only working detector, while the drain re-armed and re-queued on his behalf and commented
# where nobody looked. So the drain stopped acting on the queue, and this decides the one thing it
# still owes: a label, `needs-maintainer` (name provisional), on a pull request a maintainer
# session must pick up, so the session finds it with `gh pr list --label needs-maintainer --state
# all` and not by watching. Nothing here arms, enqueues or re-queues; it only names.
#
# Four causes, each a fact the queue does not report to anyone:
#
#   ejected   the last removal from the queue was neither `merged` nor `manual`, nothing put it
#             back since, and the head is still the one that was ejected. A removal whose event
#             names no group commit (a `merge_conflict`) cannot say which head it was, so it holds
#             until the conflict is gone instead. `manual` is excluded because a person, or the
#             drain's own `dequeue_held`, took it out on purpose; the `unarmed` cause catches it 30
#             minutes later if nobody follows up.
#   conflict  ready, and `mergeable` is CONFLICTING against `main`.
#   stale     a merge-queue entry whose pull request is no longer open. #1555 had one on
#             2026-10-03: enqueued in the same second it merged, and removed by hand 15 minutes
#             later. A closed pull request is never in `gh pr list`'s default listing, which is why
#             the session's command says `--state all`.
#   unarmed   ready, not armed, not in the queue, for `$minutes` (30, calef's figure) since the
#             latest of: it was opened, it was marked ready, auto-merge was disabled, it left the
#             queue. A lane arms in the same command as `gh pr ready`, so this is the one that
#             forgot. Not raised beside `ejected` or `conflict`, which already say what is wrong,
#             nor while a `Blocked-by:` pull request is still open, which is an unarmed pull
#             request waiting on purpose.
#
# Every cause but `stale` applies only to what `eligible` admits (helpers/queue-eligible.jq, spliced
# in front of this file) and never to one held with `needs-architect` or `held-for-red-trunk`:
# a draft is its lane's (converting an ejected pull request to a draft is how a lane says it has
# it), a fork's pull request is a person's decision, and a held one is waiting on purpose.
#
# A cause carries `key`, which names the episode: the ejection's time, the conflicting head, the
# entry's enqueue time, the moment it was last unarmed. The shell posts one comment per cause per
# key, deduplicated by a marker in the comment, so a persisting cause never comments twice and a
# new episode does.
#
# Input: the GraphQL response the drain asks for (helpers/merge-drain.sh, `nm_query`), and:
#   $label     the label's name
#   $now       epoch seconds
#   $minutes   the `unarmed` grace
#   $blockers  { "<number>": ["OPEN", "MERGED", ...] }, the states of the pull requests a ready
#              pull request's `Blocked-by:` line names (the shell resolves them; this file must
#              not call `gh`)
#
# Output, one object per pull request that has a cause or wears the label:
#   { number, action: "label" | "keep" | "clear", causes: [ { cause, key, ... } ] }
#
# helpers/needs-maintainer-selftest.sh feeds recorded responses through this; script/lint runs it.

def nm_held: ["needs-architect", "held-for-red-trunk"];

def nm_ts: if . == null or . == "" then 0 else fromdateiso8601 end;

def nm_ready_unheld:
  ([.labels.nodes[].name]) as $names
  | eligible
  | select(nm_held | map(. as $h | $names | index($h)) | all(. == null));

def nm_ejected($queued):
  . as $pr
  | nm_ready_unheld
  | (.removed.nodes[-1] // null) as $r
  | (.added.nodes[-1].createdAt // "") as $added
  | ($r.beforeCommit.parents.nodes[1].oid // null) as $was
  | select($r != null and $r.reason != "merged" and $r.reason != "manual"
           and $added < $r.createdAt and ($queued | index($pr.number) | not))
  | select(if $was == null then $pr.mergeable != "MERGEABLE" else $was == $pr.headRefOid end)
  | { cause: "ejected", key: $r.createdAt, reason: $r.reason, at: $r.createdAt,
      group: ($r.beforeCommit.oid // null), head: $pr.headRefOid };

def nm_conflict:
  nm_ready_unheld
  | select(.mergeable == "CONFLICTING")
  | { cause: "conflict", key: .headRefOid, head: .headRefOid };

def nm_unarmed($queued; $now; $minutes; $blockers):
  . as $pr
  | ([.createdAt, (.unarmed.nodes[-1].createdAt // null)] | map(select(. != null)) | max) as $since
  | ($blockers[$pr.number | tostring] // []) as $bs
  | nm_ready_unheld
  | select(.autoMergeRequest == null and ($queued | index($pr.number) | not))
  | select(($bs | index("OPEN")) == null)
  | select(($since | nm_ts) <= $now - $minutes * 60)
  | { cause: "unarmed", key: $since, since: $since, blockers: $bs };

def nm_decide($label; $now; $minutes; $blockers):
  .data as $d
  | [$d.repository.mergeQueue.entries.nodes[]? | .pullRequest.number] as $queued
  | ( [ $d.repository.pullRequests.nodes[]
        | . as $pr
        | ([nm_ejected($queued)] + [nm_conflict]) as $hard
        | { number, labelled: ([.labels.nodes[].name] | index($label) != null),
            causes: ($hard + (if $hard == [] then [nm_unarmed($queued; $now; $minutes; $blockers)] else [] end)) } ]
    + [ $d.repository.mergeQueue.entries.nodes[]?
        | select(.pullRequest.state != "OPEN")
        | { number: .pullRequest.number, labelled: false,
            causes: [ { cause: "stale", key: .enqueuedAt, enqueued: .enqueuedAt, entry: .state,
                        state: .pullRequest.state, merged: .pullRequest.mergedAt, id: .pullRequest.id } ] } ]
    + [ $d.search.nodes[]? | select(.number != null)
        | { number, labelled: ([.labels.nodes[].name] | index($label) != null), causes: [] } ] )
  # A closed pull request with a stale entry also appears in the search for labelled ones, and an
  # open one can only appear once; merge by number, keeping every cause and any labelled flag.
  | group_by(.number)
  | map({ number: .[0].number, labelled: (map(.labelled) | any), causes: (map(.causes[]) | unique_by(.cause)) })
  | map(select(.labelled or (.causes | length) > 0))
  | map(. + { action: (if (.causes | length) == 0 then "clear" elif .labelled then "keep" else "label" end) })
  | map(del(.labelled))
  | .[];
