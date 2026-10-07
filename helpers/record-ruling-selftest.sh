#!/bin/sh
# helpers/record-ruling-selftest.sh: prove script/record-ruling posts the ruling with the marker
# helpers/open-question.jq reads, then swaps needs-architect for architect-ruled in one edit, with a
# stub `gh` and no network. script/lint runs it. Cases: a plain ruling; a send-back adds held-by-lane;
# --partial posts a marker that closes nothing and edits no label; an item without needs-architect
# is not asked to remove it; a swap that does not take exits 1; an issue goes through `gh issue`.
set -u
here="$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)"
tool="$here/../script/record-ruling"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
# The stub logs every call. `api repos/.../issues/N` answers from $STUB_KIND and from a label file
# that `edit` rewrites, unless $STUB_EDIT_FAILS is set.
cat >"$tmp/gh" <<'STUB'
#!/bin/sh
echo "$*" >>"$STUB_LOG"
case "$1" in
api)
	case "$*" in
	*'.pull_request'*) [ "$STUB_KIND" = pr ] && echo pr || echo issue ;;
	*) cat "$STUB_LABELS" ;;
	esac ;;
pr | issue)
	case "$2" in
	comment) cat >>"$STUB_LOG" ;;
	edit)
		[ -n "${STUB_EDIT_FAILS:-}" ] && exit 1
		shift 3
		while [ $# -gt 0 ]; do
			case "$1" in
			--add-label) echo "$2" >>"$STUB_LABELS" ;;
			--remove-label) grep -vx "$2" "$STUB_LABELS" >"$STUB_LABELS.new"; mv "$STUB_LABELS.new" "$STUB_LABELS" ;;
			esac
			shift
		done ;;
	esac ;;
esac
STUB
chmod +x "$tmp/gh"
fail=0
# run <name> <want-exit> <kind> <labels> [args...]
run() {
	name="$1" want="$2" kind="$3" labels="$4"
	shift 4
	: >"$tmp/log"
	printf '%s\n' $labels >"$tmp/labels"
	STUB_LOG="$tmp/log" STUB_LABELS="$tmp/labels" STUB_KIND="$kind" PATH="$tmp:$PATH" \
		sh "$tool" 7 --repo o/r "$@" >/dev/null 2>&1
	rc=$?
	[ "$rc" = "$want" ] || { echo "FAIL $name: exit $rc, want $want" >&2; fail=1; }
}
has() { grep -qF -- "$2" "$tmp/$1" || { echo "FAIL $name: $1 lacks: $2" >&2; fail=1; }; }
lacks() { ! grep -qF -- "$2" "$tmp/$1" || { echo "FAIL $name: $1 has: $2" >&2; fail=1; }; }

run plain 0 pr "needs-architect ci-failing" --text '"Yes."'
has log '<!-- architect-ruling -->'
has log 'pr comment 7'
has log 'pr edit 7 --repo o/r --add-label architect-ruled --remove-label needs-architect'
has labels architect-ruled
lacks labels needs-architect
[ "$(grep -n 'pr comment' "$tmp/log" | cut -d: -f1)" -lt "$(grep -n 'pr edit' "$tmp/log" | cut -d: -f1)" ] ||
	{ echo "FAIL plain: the labels moved before the ruling was posted" >&2; fail=1; }

run sent-back 0 pr "needs-architect" --text 'Build it.' --held-by-lane
has log '--add-label architect-ruled --add-label held-by-lane --remove-label needs-architect'
has labels held-by-lane

run partial 0 pr "needs-architect" --partial --text 'Fork 1: yes.'
has log '<!-- architect-ruling partial -->'
lacks log 'edit'
has labels needs-architect

run no-hold 0 pr "" --link https://example.invalid/r
has log 'Recorded at https://example.invalid/r.'
lacks log '--remove-label'

export STUB_EDIT_FAILS=1
run swap-fails 1 pr "needs-architect" --text 'Yes.'
unset STUB_EDIT_FAILS

run issue 0 issue "needs-architect" --text 'Yes.'
has log 'issue comment 7'
has log 'issue edit 7'

run no-ruling 2 pr "needs-architect"

[ "$fail" = 0 ] && echo "record-ruling-selftest: ok"
exit "$fail"
