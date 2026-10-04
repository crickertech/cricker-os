#!/bin/sh
#
# helpers/commit-msg-selftest.sh: prove .githooks/commit-msg refuses an agent's commit with no
# trailer and passes everything else, against a scratch repository (no tree state, no cargo).
#
#     helpers/commit-msg-selftest.sh
#
# Cases, each a way the hook would silently stop gating:
#   1. CLAUDECODE=1, no trailer: refused.   2. OPENCODE=1, no trailer: refused.
#   3. CLAUDECODE=1, trailer present: passes (also lowercase spelling).
#   4. no marker, no trailer: passes (calef's commits are untouched).
#   5. CLAUDECODE=1, a trailer only inside a '#' comment line, or with an empty value: refused.
#   6. CLAUDECODE=1, a merge in progress: passes.
#   7. end to end: a real `git commit` through core.hooksPath is refused, then accepted.

set -e
cd "$(dirname "$0")/.."
hook="$(pwd)/.githooks/commit-msg"
[ -x "$hook" ] || { echo "commit-msg-selftest: $hook is missing or not executable" >&2; exit 1; }
dir="$(mktemp -d "${TMPDIR:-/tmp}/commit-msg-selftest.XXXXXX")"
trap 'rm -rf "$dir"' EXIT
fail() { echo "commit-msg-selftest: $1" >&2; exit 1; }

git init -q "$dir/repo"
cd "$dir/repo"

# expect <pass|refuse> <label> <env assignments, or "none"> <message>
expect() {
    want="$1"; label="$2"; envs="$3"; body="$4"
    printf '%s\n' "$body" >"$dir/msg"
    if [ "$envs" = none ]; then
        env -u CLAUDECODE -u OPENCODE "$hook" "$dir/msg" >/dev/null 2>&1 && got=pass || got=refuse
    else
        env -u CLAUDECODE -u OPENCODE $envs "$hook" "$dir/msg" >/dev/null 2>&1 && got=pass || got=refuse
    fi
    [ "$got" = "$want" ] || fail "$label: wanted $want, got $got"
}

plain="why this change

body text"
signed="$plain

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"

expect refuse "claude, no trailer" CLAUDECODE=1 "$plain"
expect refuse "opencode, no trailer" OPENCODE=1 "$plain"
expect pass "claude, trailer" CLAUDECODE=1 "$signed"
expect pass "opencode, trailer" OPENCODE=1 "$signed"
expect pass "claude, lowercase trailer" CLAUDECODE=1 "$plain

co-authored-by: GLM <noreply@z.ai>"
expect pass "no marker, no trailer" none "$plain"
expect refuse "trailer only in a comment" CLAUDECODE=1 "$plain
# Co-Authored-By: Claude <noreply@anthropic.com>"
expect refuse "empty trailer value" CLAUDECODE=1 "$plain

Co-Authored-By:"
expect pass "marker not 1 is not a marker" CLAUDECODE=0 "$plain"

touch .git/MERGE_HEAD
expect pass "merge in progress" CLAUDECODE=1 "$plain"
rm .git/MERGE_HEAD

# End to end through core.hooksPath, the way script/setup wires it.
git config core.hooksPath "$(dirname "$hook")"
git config user.email t@example.com; git config user.name t
echo a >f; git add f
if CLAUDECODE=1 git commit -q -m "unsigned" 2>/dev/null; then fail "end to end: unsigned agent commit was accepted"; fi
CLAUDECODE=1 git commit -q -m "signed

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>" || fail "end to end: signed agent commit was refused"
echo b >f; git add f
env -u CLAUDECODE -u OPENCODE git commit -q -m "calef's own" || fail "end to end: a commit with no marker was refused"
echo "commit-msg-selftest: ok"
