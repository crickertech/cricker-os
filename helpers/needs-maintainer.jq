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
# Seven causes, each a fact the queue does not report to anyone:
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
#   off-main  ready, not armed, and based on a branch other than `main`, for `$minutes` since it
#             was opened or marked ready. The merge queue drains `main` only, so nothing will ever
#             merge this, and `eligible` (below) never shows it to the other causes. #1640 on
#             2026-10-04: stacked on #1630's branch, green and mergeable, and labelled by nothing
#             for about three hours until calef merged it into its base by hand. A `Blocked-by:`
#             pull request still open holds it, as it holds `unarmed`.
#   red       wearing `ci-failing` (helpers/ci-failing.sh decides that, from the required checks
#             on the head) for `$minutes` since the label last went on. That label is a report with
#             no reader: a lane that ended `WAITING` is not running, and a session's queue is this
#             label, so #1617 and #1653 sat armed and red on 2026-10-04 until calef pointed at them.
#             Armed or not, any base: an armed pull request whose checks fail never enters the
#             queue, so nothing else here would see it. A push clears `ci-failing` until its checks
#             fail, and a fresh label starts a fresh episode.
#   stale-draft  a draft from this repository whose head commit is `nm_stale_draft_hours` (6,
#             calef's figure, 2026-10-05) old by its committer date. Not `updatedAt`, which every
#             bot comment and every retarget bumps. #1644 had no commit after 2026-10-04 22:11 UTC
#             once its lane's session ended, and every other cause skips a draft. A draft holding
#             only its claim commit is not exempt: that is the clearest case of a lane that died.
#             An open `Blocked-by:` holds it, as it holds `unarmed`, and so does `nm_parked_label`.
#
# `ejected`, `conflict` and `unarmed` apply only to what `eligible` admits (helpers/queue-eligible.jq,
# spliced in front of this file); `off-main` and `red` to a ready pull request from this repository
# on any base; `stale-draft` to a draft from this repository on any base. None applies to one held
# with `needs-architect` or `held-for-red-trunk`, and only `stale` and `stale-draft` apply to a
# draft: a draft is its lane's while the lane is alive (converting an ejected pull request to a draft
# is how a lane says it has it), a fork's pull request is a person's decision, and a held one is
# waiting on purpose.
#
# A cause carries `key`, which names the episode: the ejection's time, the conflicting head, the
# entry's enqueue time, the moment it was last unarmed or made ready, the moment `ci-failing` went
# on, the draft's last commit. The shell posts one comment per cause per
# key, deduplicated by a marker in the comment, so a persisting cause never comments twice and a
# new episode does.
#
# Input: the GraphQL response the drain asks for (helpers/merge-drain.sh, `nm_query`), and:
#   $label     the label's name
#   $now       epoch seconds
#   $minutes   the grace before `unarmed`, `off-main` or `red`
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

def nm_unheld:
  ([.labels.nodes[].name]) as $names
  | select(nm_held | map(. as $h | $names | index($h)) | all(. == null));

def nm_ready_unheld: eligible | nm_unheld;

# Ready, from this repository, unheld, on any base: `eligible` without its `main` clause, for the
# two causes that exist because something is not on its way to `main`'s queue.
def nm_ready_unheld_any_base:
  select(.isDraft == false and .isCrossRepository == false) | nm_unheld;

# The label helpers/ci-failing.sh puts on a red head. Its name is that script's, provisional there.
def nm_red_label: "ci-failing";

# How long a draft's head may sit without a commit before its lane is presumed gone.
def nm_stale_draft_hours: 6;

# A draft held on purpose for work outside the lane system, such as calef's GLM runs (#1745,
# 2026-10-06). It exempts `stale-draft` only, and the pull request must carry a comment giving
# the reason it is parked. The name is provisional.
def nm_parked_label: "parked";

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

def nm_off_main($now; $minutes; $blockers):
  . as $pr
  | ([.createdAt, (.unarmed.nodes[-1].createdAt // null)] | map(select(. != null)) | max) as $since
  | ($blockers[$pr.number | tostring] // []) as $bs
  | nm_ready_unheld_any_base
  | select(.baseRefName != "main" and .autoMergeRequest == null)
  | select(($bs | index("OPEN")) == null)
  | select(($since | nm_ts) <= $now - $minutes * 60)
  | { cause: "off-main", key: $since, since: $since, base: $pr.baseRefName, branch: $pr.headRefName };

# When the label went on: the newest LabeledEvent for it among the ones the query fetched. A label
# older than that window reads as the pull request's creation, which is long enough ago to count.
def nm_red($now; $minutes):
  . as $pr
  | nm_ready_unheld_any_base
  | select([.labels.nodes[].name] | index(nm_red_label) != null)
  | ([.labelled.nodes[]? | select(.label.name == nm_red_label) | .createdAt] | max // $pr.createdAt) as $since
  | select(($since | nm_ts) <= $now - $minutes * 60)
  | { cause: "red", key: $since, since: $since, head: $pr.headRefOid, branch: $pr.headRefName,
      armed: ($pr.autoMergeRequest != null) };

def nm_stale_draft($now; $blockers):
  . as $pr
  | ($blockers[$pr.number | tostring] // []) as $bs
  | (.commits.nodes[-1].commit.committedDate // null) as $at
  | select(.isDraft == true and .isCrossRepository == false and $at != null)
  | nm_unheld
  | select([.labels.nodes[].name] | index(nm_parked_label) == null)
  | select(($bs | index("OPEN")) == null)
  | select(($at | nm_ts) <= $now - nm_stale_draft_hours * 3600)
  | { cause: "stale-draft", key: $at, since: $at, head: $pr.headRefOid, branch: $pr.headRefName, blockers: $bs };

def nm_decide($label; $now; $minutes; $blockers):
  .data as $d
  | [$d.repository.mergeQueue.entries.nodes[]? | .pullRequest.number] as $queued
  | ( [ $d.repository.pullRequests.nodes[]
        | . as $pr
        | ([nm_ejected($queued)] + [nm_conflict] + [nm_red($now; $minutes)]) as $hard
        | { number, labelled: ([.labels.nodes[].name] | index($label) != null),
            causes: ($hard + (if $hard == [] then [nm_unarmed($queued; $now; $minutes; $blockers)]
                                                + [nm_off_main($now; $minutes; $blockers)] else [] end)
                     + [nm_stale_draft($now; $blockers)]) } ]
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
