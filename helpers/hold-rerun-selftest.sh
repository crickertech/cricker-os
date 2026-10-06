#!/bin/sh
# helpers/hold-rerun-selftest.sh: prove helpers/hold-rerun.sh reruns the failed hold runs only when
# the hold is released, with a stub `gh` and no network. Then check structurally that
# architect-ruled-clears-hold.yml listens for the three release events and calls the helper with
# `actions: write`, since a helper nothing calls proves nothing.
set -u
here="$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)"
wf="$here/../.github/workflows/architect-ruled-clears-hold.yml"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
cat >"$tmp/gh" <<'STUB'
#!/bin/sh
case "$1 $2" in
"api repos/o/r/pulls/1") echo abc123 ;;
"api repos/o/r/issues/1") [ "${STUB_FAIL:-}" = 1 ] && exit 1; printf '%s\n' $STUB_LABELS ;;
api*/runs*) printf '%s\n' "$STUB_RUNS" ;;
"run rerun") echo "$3" >>"$STUB_LOG" ;;
*) echo "stub gh: unexpected $*" >&2; exit 1 ;;
esac
STUB
chmod +x "$tmp/gh"
run() { printf '{"id":%s,"status":"%s","conclusion":%s}' "$1" "$2" "$3"; }
red="$(run 11 completed '"failure"')"
gone="$(run 12 completed '"cancelled"')"
ok="$(run 13 completed '"success"')"
busy="$(run 14 in_progress null)"
fail=0
check() { # name expected-rc expected-reruns labels runs-json [label-read-fails]
	: >"$tmp/log"
	STUB_LABELS="$4" STUB_RUNS="{\"workflow_runs\":[$5]}" STUB_FAIL="${6:-0}" STUB_LOG="$tmp/log" \
		HOLD_RERUN_WAIT=2 HOLD_RERUN_SLEEP=0 PATH="$tmp:$PATH" \
		"$here/hold-rerun.sh" 1 o/r >/dev/null 2>&1
	rc=$?
	got="$(tr '\n' ' ' <"$tmp/log" | sed 's/ $//')"
	if [ "$rc" != "$2" ] || [ "$got" != "$3" ]; then
		echo "FAIL $1: rc $rc reruns '$got', want rc $2 reruns '$3'" >&2
		fail=1
	fi
}
check released-no-labels 0 "11 12" "" "$red,$ok,$gone"
check released-by-ruling 0 "11" "needs-architect architect-ruled" "$red,$ok"
check still-needs-architect 1 "" "needs-architect ci-failing" "$red,$ok"
check still-held-by-lane 1 "" "architect-ruled held-by-lane" "$red,$ok"
check near-miss-is-released 0 "11" "needs-architect-not held-by-lane-not" "$red"
check nothing-red 0 "" "" "$ok"
check in-flight-skipped 0 "11" "" "$red,$busy"
check label-read-fails 2 "" "" "$red" 1

# Structural: the workflow reacts to each release event and runs the helper with a token that can
# rerun. A grep, so it pins the strings, not the YAML's meaning; actionlint covers the syntax.
for want in "types: \[labeled, unlabeled\]" \
	"github.event.label.name == 'architect-ruled'" \
	"github.event.label.name == 'needs-architect'" \
	"github.event.label.name == 'held-by-lane'" \
	"actions: write" \
	"helpers/hold-rerun.sh"; do
	if ! grep -q "$want" "$wf"; then
		echo "FAIL workflow: architect-ruled-clears-hold.yml lacks '$want'" >&2
		fail=1
	fi
done
[ "$fail" = 0 ] && echo "hold-rerun-selftest: ok"
exit "$fail"
