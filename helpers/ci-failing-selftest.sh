#!/bin/sh
#
# helpers/ci-failing-selftest.sh: the decision in helpers/ci-failing.jq against a recorded commit
# and against synthetic cases, and helpers/ci-failing.sh checked for staying a reporter. Provisional
# names, with the script. script/lint runs it.
#
# The recorded fixture is the check runs of one real pull request head, kept to five names: its
# `ready status` ran twice, failed first and passed on the later run, which is the stale failure
# #1592 showed and the reason the decision reads the newest run per name.

set -e
here="$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)"
me="$(basename "$0")"

if ! command -v jq >/dev/null 2>&1; then
	echo "$me: jq is not installed, and the script cannot run without it either." >&2
	exit 1
fi

program="$(cat "$here/ci-failing.jq")"
# decide <runs JSON> <required JSON>: the failing names, joined.
decide() {
	echo "$1" | jq -r --argjson required "$2" "$program"' failing($required) | map(.name) | join(";")'
}
expect() {
	if [ "$2" != "$3" ]; then
		echo "$me: $1" >&2
		echo "  expected '$3'" >&2
		echo "  got      '$2'" >&2
		exit 1
	fi
}

req='["ready status (no IN-PROGRESS block on a ready branch)","clippy","rustfmt","architect hold (needs-architect label)"]'

# 1. Recorded: a failure superseded by a later success is not failing.
expect "a superseded failure was reported" \
	"$(decide "$(cat "$here/ci-failing-fixtures/recorded-2026-10-04-superseded-failure.json")" "$req")" ""

# 2. Synthetic.
runs='[
 {"id":1,"name":"clippy","conclusion":"failure","html_url":"u1"},
 {"id":2,"name":"rustfmt","conclusion":"success","html_url":"u2"},
 {"id":3,"name":"rustfmt","conclusion":"failure","html_url":"u3"},
 {"id":4,"name":"not required","conclusion":"failure","html_url":"u4"},
 {"id":5,"name":"architect hold (needs-architect label)","conclusion":"failure","html_url":"u5"}]'
expect "newest failure, an unrequired failure or the architect hold was mishandled" \
	"$(decide "$runs" "$req")" "clippy;rustfmt"
expect "a newer in-progress run must hide an older failure" \
	"$(decide '[{"id":1,"name":"clippy","conclusion":"failure"},{"id":2,"name":"clippy","conclusion":null,"status":"in_progress"}]' "$req")" ""
expect "a newer cancelled run must hide an older failure" \
	"$(decide '[{"id":1,"name":"clippy","conclusion":"failure"},{"id":2,"name":"clippy","conclusion":"cancelled"}]' "$req")" ""
expect "timed_out must count as failing" \
	"$(decide '[{"id":1,"name":"clippy","conclusion":"timed_out"}]' "$req")" "clippy"
expect "a required check with no run must not count" "$(decide '[]' "$req")" ""

# 3. The script reports and never resolves.
if grep -v '^[[:space:]]*#' "$here/ci-failing.sh" | grep -Eq 'dequeue|disable-auto|merge --auto|run rerun|enqueue'; then
	echo "$me: ci-failing.sh changes the queue or reruns CI; it must only label and comment." >&2
	exit 1
fi
echo "$me: ok"
