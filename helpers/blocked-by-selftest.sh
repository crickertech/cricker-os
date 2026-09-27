#!/bin/sh
#
# helpers/blocked-by-selftest.sh: the Blocked-by: parser, checked against fixtures.
#
# Companion to helpers/queue-eligible-selftest.sh in shape and reason: a change to
# helpers/blocked-by.sh's parsing is a gate rather than a habit (AGENTS.md's ladder, rung two).
# Every case here is a body string in, a number list out; nothing calls `gh`.

set -e
here="$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)"
me="$(basename "$0")"

. "$here/blocked-by.sh"

check() {
	desc="$1"
	body="$2"
	expected="$3"
	got=$(nife_blocked_by "$body" | tr '\n' ' ' | sed 's/ *$//')
	if [ "$got" != "$expected" ]; then
		echo "$me: $desc: expected [$expected], got [$got]" >&2
		exit 1
	fi
}

check "one number" \
	"**Lane:** x, written by an agent.

Blocked-by: #1352

Stacked on #1352 (milestone 206); retarget to main when that lands." \
	"1352"

check "several numbers, comma separated" \
	"Blocked-by: #1347, #1354

Stacked on #1347 and on #1354." \
	"1347 1354"

check "several numbers, space separated" \
	"Blocked-by: #1342 #1373" \
	"1342 1373"

check "no Blocked-by line" \
	"Stacked on #1352, mentioned in prose but not marked." \
	""

check "case-insensitive key" \
	"blocked-BY: #42" \
	"42"

check "only the first Blocked-by line is read" \
	"Blocked-by: #1

unrelated paragraph

Blocked-by: #2" \
	"1"

echo "blocked-by parsing: one number, several (comma or space separated), none, case-insensitivity, first-line-only"
