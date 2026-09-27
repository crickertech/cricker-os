#!/bin/sh
#
# helpers/blocked-by-resolution-selftest.sh: the draft-unblock decision, checked against fixtures.
#
# Companion to helpers/queue-stranded-selftest.sh in shape: the predicate in
# helpers/blocked-by-resolution.jq is checked here without a `gh` call, and script/lint runs this
# so a change to the decision is a gate rather than a habit.
#
# Needs jq, which every consumer of the predicate needs too.

set -e
here="$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)"
me="$(basename "$0")"

if ! command -v jq >/dev/null 2>&1; then
	echo "$me: jq is not installed, and the merge drain cannot run without it either." >&2
	exit 1
fi

run() {
	printf '%s' "$1" | jq -c "$(cat "$here/blocked-by-resolution.jq")"'resolution'
}

check() {
	desc="$1"
	input="$2"
	expected="$3"
	got=$(run "$input")
	if [ "$got" != "$expected" ]; then
		echo "$me: $desc: expected $expected, got $got" >&2
		exit 1
	fi
}

# One number, merged: unblocked, and the number comes back for the comment to name.
check "one blocker, merged" \
	'{"has_label": false, "blockers": [{"number": 1352, "state": "MERGED"}]}' \
	'{"status":"unblocked","numbers":[1352]}'

# Several numbers, all merged: unblocked, every number reported.
check "several blockers, all merged" \
	'{"has_label": false, "blockers": [{"number": 1347, "state": "MERGED"}, {"number": 1354, "state": "MERGED"}]}' \
	'{"status":"unblocked","numbers":[1347,1354]}'

# Several numbers, one still open: still waiting, and nothing fires on a partial resolution.
check "several blockers, one still open" \
	'{"has_label": false, "blockers": [{"number": 1347, "state": "MERGED"}, {"number": 1354, "state": "OPEN"}]}' \
	'{"status":"waiting"}'

# One blocker, closed without merging: the anomaly, reported distinctly from a plain unblock.
check "one blocker, closed unmerged" \
	'{"has_label": false, "blockers": [{"number": 42, "state": "CLOSED"}]}' \
	'{"status":"closed-unmerged","numbers":[42]}'

# Mixed: one merged, one closed unmerged, none open: still reported as closed-unmerged, because
# that is the anomaly worth a person reading even though the other blocker did land cleanly.
check "mixed merged and closed unmerged" \
	'{"has_label": false, "blockers": [{"number": 1, "state": "MERGED"}, {"number": 2, "state": "CLOSED"}]}' \
	'{"status":"closed-unmerged","numbers":[2]}'

# Already labelled: short-circuits regardless of blocker states, so a draft is never relabelled or
# recommented once the label has done its job.
check "already labelled" \
	'{"has_label": true, "blockers": [{"number": 1, "state": "MERGED"}]}' \
	'{"status":"already-labelled"}'

echo "blocked-by resolution: unblocked (one and several), waiting, closed unmerged, mixed, already labelled"
