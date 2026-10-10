#!/bin/sh
#
# Selftest for script/claim's refusals, run by script/lint. Every case here must be refused before
# script/claim touches git, the disk or the network. `git` and `gh` are stubs on PATH that record
# being called and fail, so a refusal that regresses fails this test instead of making a real claim
# (the first falsification of this file, run without the stubs, opened draft #1898 and pushed its
# branch, 2026-10-10 UTC). The refusals are the point: a claim that
# accepts an unclassified lane looks the same as one that cannot (calef's ruling R1 on #1894,
# 2026-10-10 UTC: exactly one of --debt-paydown or --new-work; notes/debt-paydown.md).
#
# Name: provisional, 2026-10-10 (UTC), after the other helpers/*-selftest.sh.

set -eu
root="$(cd "$(dirname "$0")/.." && pwd)"
claim="$root/script/claim"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
export NIFE_WORKTREES="$work/worktrees"
real_git=$(command -v git)
mkdir "$work/bin"
for tool in git gh; do
    printf '#!/bin/sh\necho "%s $*" >> "%s/called"\nexit 97\n' "$tool" "$work" > "$work/bin/$tool"
    chmod +x "$work/bin/$tool"
done
PATH="$work/bin:$PATH"
branch="maintainer/claim-selftest-$$"

fail() { echo "claim-selftest: $*" >&2; exit 1; }

# expect NAME WANT_EXIT WANT_TEXT ARGS...: run script/claim, check its exit and its output.
expect() {
    name=$1 want=$2 text=$3
    shift 3
    set +e
    out=$(sh "$claim" "$@" 2>&1)
    got=$?
    set -e
    [ ! -e "$work/called" ] || fail "$name: reached $(cat "$work/called") before refusing"
    [ "$got" -eq "$want" ] || fail "$name: exit $got, wanted $want ($out)"
    printf '%s' "$out" | grep -q -- "$text" || fail "$name: output lacks '$text' ($out)"
}

expect "no classification" 2 "add --debt-paydown or --new-work" "$branch"
expect "both classifications" 2 "both given" "$branch" --debt-paydown --new-work
expect "both, other order" 2 "both given" --new-work "$branch" --debt-paydown
expect "help names both" 0 "(--debt-paydown | --new-work)" --help
# Classified, but the name is a milestone near-miss: the shape refusal still fires.
expect "classified, bad name" 1 "does not parse" milestone-1-x --debt-paydown

"$real_git" -C "$root" rev-parse --verify --quiet "refs/heads/$branch" >/dev/null \
    && fail "a refused claim created branch $branch"
[ ! -e "$NIFE_WORKTREES" ] || fail "a refused claim created $NIFE_WORKTREES"
echo "claim-selftest: ok"
