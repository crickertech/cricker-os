#!/bin/sh
#
# helpers/needs-maintainer-selftest.sh: the detector's decision (helpers/needs-maintainer.jq), fed
# GraphQL responses in the shape helpers/merge-drain.sh asks for, and the drain checked for still
# asking for every field the decision reads and for never arming or enqueueing. Milestone 727
# (a queue eviction goes to a maintainer session), number provisional. script/lint runs it.
#
# Two fixtures under helpers/needs-maintainer-fixtures/:
#
#   recorded-2026-10-03T23-54-15Z.json   the drain's query, run against the repository at that
#       moment and stored as returned, except that each `body` is emptied (they cite milestones,
#       which script/citations would hold this file to, and none had a `Blocked-by:` line, the
#       only thing the decision reads a body for). #1569 had just been ejected on a
#       `merge_conflict` and read CONFLICTING; everything else was a draft, armed, queued or held. Detection logic that
#       only runs against live GitHub is a defect, so the live shape is pinned here.
#   every-cause.json   one pull request per case, built from recorded event nodes (#1473's
#       `failed_checks` ejection, #1494's `merge_conflict`, #1530's manual dequeue, #1555's stale
#       entry) with the numbers changed so each case stands alone. Cases 20 to 29 are the two
#       causes added on 2026-10-04 (#1640 stacked and unlabelled; #1617 and #1653 red and unowned),
#       built in the same shape. Cases 30 to 37 are `stale-draft` (#1644, 2026-10-05),
#       and 38 and 39 its `parked` exemption (#1745, 2026-10-06), which leave out the
#       label-event alias only `red` reads.
#
# The order of the checks is the order of the ruling: the causes, then clearing, then the
# drain's own shape.

set -e
here="$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)"
me="$(basename "$0")"
fx="$here/needs-maintainer-fixtures"

if ! command -v jq >/dev/null 2>&1; then
	echo "$me: jq is not installed, and the merge drain cannot run without it either." >&2
	exit 1
fi

program="$(cat "$here/queue-eligible.jq" "$here/needs-maintainer.jq")"
# decide <fixture> <now, ISO> <blockers JSON>: one "number:action[:cause,cause]" per record.
decide() {
	jq -c --arg l needs-maintainer --argjson now "$(jq -n --arg t "$2" '$t | fromdateiso8601')" --argjson b "$3" \
		"$program"'[ nm_decide($l; $now; 30; $b)
			| "\(.number):\(.action)" + (if .action == "clear" then "" else ":" + (.causes | map(.cause) | join(",")) end) ]' "$1"
}
expect() {
	if [ "$2" != "$3" ]; then
		echo "$me: $1" >&2
		echo "  expected $3" >&2
		echo "  got      $2" >&2
		exit 1
	fi
}

# 1. The recorded response: one ejected, conflicting pull request, and nothing else flagged.
expect "the recorded 2026-10-03T23:54:15Z response decided wrong" \
	"$(decide "$fx/recorded-2026-10-03T23-54-15Z.json" 2026-10-03T23:54:15Z '{}')" \
	'["1569:label:conflict,ejected"]'

# 2. Every cause, and every clearing, at 22:00 UTC with a 30-minute grace. #11, #22 and #33 name
#    an open blocker and #12 only resolved ones.
want='["1:label:ejected","2:clear","3:clear","4:label:conflict,ejected","5:clear","6:label:unarmed","7:label:unarmed","9:label:unarmed","12:label:unarmed","14:label:conflict","16:clear","20:label:off-main","24:label:red","27:label:red","29:label:red","30:label:stale-draft","32:label:stale-draft","36:keep:stale-draft","39:clear","1555:keep:stale","1556:clear"]'
got="$(decide "$fx/every-cause.json" 2026-10-03T22:00:00Z '{"11": ["OPEN", "MERGED"], "12": ["MERGED"], "22": ["OPEN"], "33": ["OPEN"]}')"
if [ "$got" != "$want" ]; then
	echo "$me: the needs-maintainer decision is wrong." >&2
	echo "  expected $want" >&2
	echo "  got      $got" >&2
	cat >&2 <<'EOF'
  The cases: 1 ejected (failed_checks) at the head it still has. 2 the same, head moved and armed:
  clear. 3 the same, back in the queue: clear. 4 a merge_conflict ejection still CONFLICTING:
  both causes. 5 the same, conflict fixed and armed: clear. 6 the same, fixed but never re-armed
  for hours: unarmed. 7 dequeued by hand an hour ago and left: unarmed. 8 ready 10 minutes ago:
  nothing yet. 9 ready 60 minutes, unarmed. 10 held needs-architect: nothing. 11 Blocked-by an
  open pull request: nothing. 12 Blocked-by merged ones: unarmed. 13 a conflicting draft: nothing.
  14 conflicting and unarmed: conflict only. 15 a fork: nothing. 16 labelled and queued: clear.
  17 an ejected draft: nothing. 18 armed: nothing. 19 opened ready 15 minutes ago: nothing.
  1555 a merged pull request still in the queue, already labelled: keep. 1556 labelled, merged,
  and out of the queue: clear. 20 ready an hour on another pull request's branch, unarmed:
  off-main (#1640). 21 the same, ready 10 minutes: nothing yet. 22 the same, Blocked-by an open
  pull request: nothing. 23 the same, armed into its base: nothing. 24 armed and ci-failing for an
  hour: red (#1653), keyed on that label and not on a later one. 25 the same, 10 minutes: nothing
  yet. 26 ci-failing but held needs-architect: nothing. 27 ci-failing and unarmed: red, and not
  unarmed beside it. 28 a ci-failing draft: nothing. 29 ci-failing on another base, its label
  older than the fetched events: red. 30 a stacked draft whose head commit is 10 hours old:
  stale-draft (#1644). 31 the same, 4 hours: nothing yet. 32 a draft holding only a 22-hour-old
  claim commit: stale-draft, not exempt. 33 an old draft Blocked-by an open pull request:
  nothing. 34 an old draft held needs-architect: nothing. 35 a fork's old draft: nothing. 36 an
  old draft already labelled: keep. 37 a ready, armed pull request with an old commit: nothing.
  38 an old draft labeled parked: nothing. 39 the same, already labeled: clear.
EOF
	exit 1
fi

# 3. The episode keys the comment markers are built from, and the evidence the comment names.
got=$(jq -c --argjson now "$(jq -n '"2026-10-03T22:00:00Z" | fromdateiso8601')" \
	"$program"'[ nm_decide("needs-maintainer"; $now; 30; {}) | select(.number == 1 or .number == 1555 or .number == 9 or .number == 20 or .number == 24 or .number == 29 or .number == 30) | .causes[] | [.cause, .key] ]' \
	"$fx/every-cause.json")
expect "a cause's episode key came back wrong" "$got" \
	'[["ejected","2026-10-03T01:45:00Z"],["unarmed","2026-10-03T21:00:00Z"],["off-main","2026-10-03T21:00:00Z"],["red","2026-10-03T21:00:00Z"],["red","2026-10-03T00:00:00Z"],["stale-draft","2026-10-03T12:00:00Z"],["stale","2026-10-03T22:23:57Z"]]'
got=$(jq -c --argjson now "$(jq -n '"2026-10-03T22:00:00Z" | fromdateiso8601')" \
	"$program"'[ nm_decide("needs-maintainer"; $now; 30; {}) | select(.number == 1) | .causes[0] | .group, .head ]' \
	"$fx/every-cause.json")
expect "an ejection's group commit or head came back wrong" "$got" \
	'["47d8f11eee8260aa479d3905a50900008694fef2","4124d6390de36d200e2797616761b584e9dcd850"]'

# 4. The drain splices the decision and asks for every field it reads.
f="$here/merge-drain.sh"
if ! grep -qF '"$(cat "$ELIGIBLE_JQ" "$NM_JQ")"' "$f"; then
	echo "$me: merge-drain.sh does not splice queue-eligible.jq and needs-maintainer.jq, so nothing decides." >&2
	exit 1
fi
for field in 'mergeQueue(branch: "main")' enqueuedAt mergedAt isDraft baseRefName isCrossRepository \
	headRefName headRefOid createdAt mergeable body autoMergeRequest REMOVED_FROM_MERGE_QUEUE_EVENT \
	ADDED_TO_MERGE_QUEUE_EVENT READY_FOR_REVIEW_EVENT AUTO_MERGE_DISABLED_EVENT LABELED_EVENT committedDate beforeCommit \
	'parents(first: 2)' 'search(query: $labelled'; do
	if ! grep -qF -- "$field" "$f"; then
		echo "$me: merge-drain.sh's needs-maintainer query does not ask for $field, which the decision reads." >&2
		exit 1
	fi
done

# 5. calef's first ruling on #1564, held by a gate rather than by memory: the drain never arms
#    and never enqueues. Comments are skipped, so the history can still name what it did, and so
#    is a command quoted in a pull request comment (\`gh pr merge ...\`), which tells a person
#    what to type rather than typing it.
armers=$(grep -v '^[[:space:]]*#' "$f" | grep -e 'gh pr merge' -e 'enqueuePullRequest' | grep -v '\\`gh pr merge' || true)
if [ -n "$armers" ]; then
	echo "$me: merge-drain.sh arms or enqueues again, which calef ruled out on #1564 (2026-10-03):" >&2
	printf '  %s\n' "$armers" >&2
	echo "  A pull request a lane did not arm is labelled needs-maintainer instead." >&2
	exit 1
fi

echo "needs maintainer: ejected, conflicting, stale, unarmed, off-main, red and stale-draft pull requests labelled, each cleared when its cause goes; the drain splices it and never arms"
