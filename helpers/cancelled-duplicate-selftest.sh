#!/bin/sh
#
# helpers/cancelled-duplicate-selftest.sh: the drain's "rerun the cancelled duplicate, once"
# detection, checked against fixtures, and its consumer checked for still using it.
#
# Fixtures, all on one head SHA:
#   CI       #10 success 15:43:24, #11 cancelled 15:43:24 (newer, same second)   -> rerun #11
#   verify   #20 cancelled 15:43:24, #21 success 15:43:24 (cancelled is older)   -> nothing
#   bench    #30 cancelled 15:43:24, #31 success 15:44:10 (ordinary supersede)   -> nothing
#   lint     #40 success 15:43:24, #41 cancelled 15:43:24, run_attempt 2         -> nothing: rerun once
#   docs     #50 in_progress 15:43:24, #51 cancelled 15:43:24                    -> rerun #51 (sibling running)
# script/lint runs it.

set -e
here="$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)"
me="$(basename "$0")"

if ! command -v jq >/dev/null 2>&1; then
	echo "$me: jq is not installed, and the merge drain cannot run without it either." >&2
	exit 1
fi

program="$(cat "$here/cancelled-duplicate.jq")"
fixture='{"workflow_runs": [
  {"id": 10, "name": "CI",     "status": "completed",   "conclusion": "success",   "created_at": "2026-09-24T15:43:24Z", "run_attempt": 1},
  {"id": 11, "name": "CI",     "status": "completed",   "conclusion": "cancelled", "created_at": "2026-09-24T15:43:24Z", "run_attempt": 1},
  {"id": 20, "name": "verify", "status": "completed",   "conclusion": "cancelled", "created_at": "2026-09-24T15:43:24Z", "run_attempt": 1},
  {"id": 21, "name": "verify", "status": "completed",   "conclusion": "success",   "created_at": "2026-09-24T15:43:24Z", "run_attempt": 1},
  {"id": 30, "name": "bench",  "status": "completed",   "conclusion": "cancelled", "created_at": "2026-09-24T15:43:24Z", "run_attempt": 1},
  {"id": 31, "name": "bench",  "status": "completed",   "conclusion": "success",   "created_at": "2026-09-24T15:44:10Z", "run_attempt": 1},
  {"id": 40, "name": "lint",   "status": "completed",   "conclusion": "success",   "created_at": "2026-09-24T15:43:24Z", "run_attempt": 1},
  {"id": 41, "name": "lint",   "status": "completed",   "conclusion": "cancelled", "created_at": "2026-09-24T15:43:24Z", "run_attempt": 2},
  {"id": 50, "name": "docs",   "status": "in_progress", "conclusion": null,        "created_at": "2026-09-24T15:43:24Z", "run_attempt": 1},
  {"id": 51, "name": "docs",   "status": "completed",   "conclusion": "cancelled", "created_at": "2026-09-24T15:43:24Z", "run_attempt": 1}
]}'

# 1. Detection: the duplicates, rerun or not.
got=$(printf '%s' "$fixture" | jq -c "$program"'[ cancelled_duplicates | .id ]')
if [ "$got" != "[11,51,41]" ] && [ "$got" != "[11,41,51]" ]; then
	echo "$me: detection found the wrong duplicates: expected CI #11, docs #51 and lint #41, got $got" >&2
	echo "  (verify's cancelled run is the older one; bench's success came 46 seconds later)" >&2
	exit 1
fi

# 2. Once only: lint's #41 has run_attempt 2 and must not be rerun again.
got=$(printf '%s' "$fixture" | jq -c "$program"'[ rerunnable | .id ] | sort')
if [ "$got" != "[11,51]" ]; then
	echo "$me: the once-only rule failed: expected [11,51], got $got (lint #41 is attempt 2)" >&2
	exit 1
fi

# 3. The consumer splices the file, reruns only what `rerunnable` names, and never reruns by any
#    other route.
f="$here/merge-drain.sh"
if ! grep -qF '"$(cat "$CANCELLED_JQ")"' "$f"; then
	echo "$me: merge-drain.sh does not splice helpers/cancelled-duplicate.jq; a rerun without the detection is a rerun of anything cancelled." >&2
	exit 1
fi
if ! grep -q 'rerunnable' "$f"; then
	echo "$me: merge-drain.sh does not use rerunnable, so the once-only rule is not what decides a rerun." >&2
	exit 1
fi
# Every `gh run rerun` lives inside `rerun_run()`, and that helper has exactly one caller, which
# is the guarded loop. Counted with awk over the function body, so a second call site anywhere
# else in the script is a red test rather than a rerun nothing decided.
total=$(grep -v '^[[:space:]]*#' "$f" | grep -c 'gh run rerun')
inside=$(awk '/^rerun_run\(\) \{/{f=1} f && !/^[[:space:]]*#/ && /gh run rerun/{n++} f && /^\}/{f=0} END{print n+0}' "$f")
if [ "$total" = "0" ] || [ "$total" != "$inside" ]; then
	echo "$me: merge-drain.sh calls gh run rerun outside rerun_run() ($total total, $inside inside), so a rerun can happen without the detection." >&2
	exit 1
fi
if [ "$(grep -c 'rerun_run "\$run_id"' "$f")" != "1" ]; then
	echo "$me: rerun_run() has a caller other than the one loop the detection guards." >&2
	exit 1
fi
# The one other caller, `missing_check_scan`, reruns only what helpers/missing-check.jq names.
if [ "$(grep -c 'rerun_run "\$rid"' "$f")" != "1" ] || ! grep -q 'MISSING_JQ=' "$f"; then
	echo "$me: rerun_run() has a caller other than the two loops the detections guard." >&2
	exit 1
fi

echo "cancelled duplicate: newest run cancelled beside a same-second sibling, rerun once, by attempt number; the drain splices it"
