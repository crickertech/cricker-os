#!/bin/sh
# helpers/architect-ruled-selftest.sh: prove helpers/architect-ruled.sh reads the live label list,
# with a stub `gh` and no network. Cases: ruled alone; ruled with needs-architect also on (the
# #1712 end state); not ruled; a near-miss name must not match; API failure gives 2.
set -u
here="$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
cat >"$tmp/gh" <<'STUB'
#!/bin/sh
[ "${STUB_FAIL:-}" = 1 ] && exit 1
printf '%s\n' $STUB_LABELS
STUB
chmod +x "$tmp/gh"
fail=0
check() { # name expected labels [fail]
	STUB_LABELS="$3" STUB_FAIL="${4:-0}" PATH="$tmp:$PATH" "$here/architect-ruled.sh" 1 o/r 2>/dev/null
	rc=$?
	if [ "$rc" != "$2" ]; then echo "FAIL $1: got $rc want $2" >&2; fail=1; fi
}
check ruled-alone 0 "architect-ruled"
check ruled-and-needs 0 "needs-architect architect-ruled ci-failing"
check not-ruled 1 "needs-architect"
check no-labels 1 ""
check near-miss 1 "architect-ruled-not needs-architect"
check api-failure 2 "" 1
[ "$fail" = 0 ] && echo "architect-ruled-selftest: ok"
exit "$fail"
