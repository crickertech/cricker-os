#!/bin/sh
#
# helpers/queue-ejected-selftest.sh: the merge drain's ejection decision (helpers/queue-ejected.jq),
# checked against fixtures, and its consumer checked for still using it. Milestone 630 (a merge-queue
# ejection is caught before the queue, and recovered after it).
#
# The fixture that matters most is the first: a pull request ejected at the head it still has is
# HELD, not re-armed, because every group attempt costs 20 to 38 minutes and the drain otherwise
# re-arms everything eligible on every pass. The second matters as much the other way: once the
# head moves, the hold must release, or the mechanism is a new way to strand green work, which is
# the class it exists to close. script/lint runs it.

set -e
here="$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)"
me="$(basename "$0")"

if ! command -v jq >/dev/null 2>&1; then
	echo "$me: jq is not installed, and the merge drain cannot run without it either." >&2
	exit 1
fi

program="$(cat "$here/queue-ejected.jq")"
# A removal whose merge-group commit is G1 with parents (main tip, head) = (M, A).
rm_at() { printf '{"createdAt": "%s", "reason": "%s", "beforeCommit": {"oid": "G1", "parents": {"nodes": [{"oid": "M"}, {"oid": "A"}]}}}' "$1" "$2"; }
fixture="[
  {\"number\": 1, \"headRefOid\": \"A\", \"labels\": {\"nodes\": []},
   \"removed\": {\"nodes\": [$(rm_at 2026-10-03T01:45:00Z failed_checks)]},
   \"added\": {\"nodes\": [{\"createdAt\": \"2026-10-03T01:11:14Z\"}]}},
  {\"number\": 2, \"headRefOid\": \"B\", \"labels\": {\"nodes\": [{\"name\": \"queue-ejected\"}]},
   \"removed\": {\"nodes\": [$(rm_at 2026-10-03T01:45:00Z failed_checks)]},
   \"added\": {\"nodes\": [{\"createdAt\": \"2026-10-03T01:11:14Z\"}]}},
  {\"number\": 3, \"headRefOid\": \"A\", \"labels\": {\"nodes\": [{\"name\": \"queue-ejected\"}]},
   \"removed\": {\"nodes\": [$(rm_at 2026-10-03T01:45:00Z failed_checks)]},
   \"added\": {\"nodes\": [{\"createdAt\": \"2026-10-03T02:30:00Z\"}]}},
  {\"number\": 4, \"headRefOid\": \"A\", \"labels\": {\"nodes\": []},
   \"removed\": {\"nodes\": [$(rm_at 2026-10-03T01:45:00Z merged)]}, \"added\": {\"nodes\": []}},
  {\"number\": 5, \"headRefOid\": \"A\", \"labels\": {\"nodes\": []},
   \"removed\": {\"nodes\": [$(rm_at 2026-10-03T01:45:00Z manual)]}, \"added\": {\"nodes\": []}},
  {\"number\": 6, \"headRefOid\": \"A\", \"labels\": {\"nodes\": [{\"name\": \"queue-ejected\"}]},
   \"removed\": {\"nodes\": [$(rm_at 2026-10-03T01:45:00Z manual)]}, \"added\": {\"nodes\": []}},
  {\"number\": 7, \"headRefOid\": \"A\", \"labels\": {\"nodes\": []},
   \"removed\": {\"nodes\": []}, \"added\": {\"nodes\": []}},
  {\"number\": 8, \"headRefOid\": \"C\", \"labels\": {\"nodes\": []},
   \"removed\": {\"nodes\": [{\"createdAt\": \"2026-09-27T07:27:04Z\", \"reason\": \"merge_conflict\", \"beforeCommit\": null}]},
   \"added\": {\"nodes\": []}},
  {\"number\": 9, \"headRefOid\": \"A\", \"labels\": {\"nodes\": []},
   \"removed\": {\"nodes\": [$(rm_at 2026-10-03T01:45:00Z some_reason_github_adds_later)]}, \"added\": {\"nodes\": []}}
]"
got=$(printf '%s' "$fixture" | jq -c "$program"'[ .[] | ejection_state("queue-ejected") | "\(.number):\(.action)" ]')
want='["1:hold","2:moved","3:release","6:release","8:hold","9:hold"]'
if [ "$got" != "$want" ]; then
	echo "$me: the ejection decision is wrong: expected $want, got $got" >&2
	echo "  (1 ejected at its own head; 2 pushed since; 3 enqueued again since; 4 merged and 5 removed" >&2
	echo "  by hand are not ejections; 6 is a stale label; 7 never queued; 8 a conflict with no group" >&2
	echo "  commit holds rather than guessing; 9 an unknown reason counts as an ejection)" >&2
	exit 1
fi

# The fields the shell carries out the decision with.
got=$(printf '%s' "$fixture" | jq -c "$program"'[ .[] | select(.number == 1) | ejection_state("queue-ejected") | [.reason, .at, .group, .ejected_head, .labelled] ]')
if [ "$got" != '[["failed_checks","2026-10-03T01:45:00Z","G1","A",false]]' ]; then
	echo "$me: an ejection's fields came back wrong: $got" >&2
	exit 1
fi

# The consumer splices the file, asks GraphQL for every field it reads, and holds what it says.
f="$here/merge-drain.sh"
if ! grep -qF '"$(cat "$EJECTED_JQ")"' "$f"; then
	echo "$me: merge-drain.sh does not splice queue-ejected.jq, so nothing decides about an ejection." >&2
	exit 1
fi
for field in REMOVED_FROM_MERGE_QUEUE_EVENT ADDED_TO_MERGE_QUEUE_EVENT beforeCommit headRefOid 'parents(first:2)'; do
	if ! grep -qF -- "$field" "$f"; then
		echo "$me: merge-drain.sh's ejection query does not ask for $field, which queue-ejected.jq reads." >&2
		exit 1
	fi
done
if ! grep -q 'case " $held " in' "$f"; then
	echo "$me: merge-drain.sh computes held pull requests and no longer skips them when arming." >&2
	exit 1
fi

echo "queue ejected: a head ejected on a failure is held, a moved head releases, a deliberate removal is not an ejection; the drain splices it"
