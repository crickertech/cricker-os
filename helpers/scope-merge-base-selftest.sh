#!/bin/sh
#
# helpers/scope-merge-base-selftest.sh: reproduce the #1416 failure in a throwaway repo and prove
# helpers/scope-merge-base.sh no longer has it.
#
#     helpers/scope-merge-base-selftest.sh
#
# Builds a real git repo under `mktemp -d` (this script's own git plumbing needs real commits and
# real ancestry, not fixture strings, so it does not share `architect-label-rules.py --selftest`'s
# no-repository shape): a "main" branch, a pull request branch forked from it, then an unrelated
# commit landing on "main" AFTER the fork, the same shape #1405 landing after #1416 branched. It
# points a fake `origin/main` ref at the post-fork "main" tip (no real remote needed; the script
# under test only ever reads `origin/<base-ref>` and a fallback sha, both of which a local
# `update-ref` can stand in for) and checks out the pull request branch detached, the same way
# `actions/checkout` leaves a `pull_request` job's worktree.
#
# Every check is a property that mattered in a real failure, either #1416's (1-3) or one of the
# ways `ci.yml`'s six scope-check callers reach this script with an argument #1416 never exercised
# (4-6: an empty base-ref, the shape a `merge_group` or plain `push` event passes):
#   1. the merge base resolves to the fork point, not the post-fork "main" tip.
#   2. a diff against that merge base does NOT include the unrelated commit's file (the bug: a
#      diff against the stale/wrong base DID).
#   3. a diff against that merge base DOES include the pull request's own change.
#   4. the fallback sha is used when "origin/<base-ref>" does not exist.
#   5. an empty base-ref (no pull_request context at all) falls straight to the fallback sha.
#   6. neither resolving (empty base-ref, no fallback): no output, exit 1.

set -e
here="$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)"
me="$(basename "$0")"
under_test="$here/scope-merge-base.sh"

tmpdir="$(mktemp -d)"
cleanup() { rm -rf "$tmpdir"; }
trap cleanup EXIT

repo="$tmpdir/repo"
git init -q "$repo"
cd "$repo"
git config user.email "selftest@example.invalid"
git config user.name "scope-merge-base-selftest"
git checkout -q -b main

echo "root" >README
git add README
git commit -q -m "root"
fork_point="$(git rev-parse HEAD)"

# The pull request's own branch, forked at $fork_point.
git checkout -q -b pr
echo "pr change" >pr-file
git add pr-file
git commit -q -m "the pull request's own change"
pr_head="$(git rev-parse HEAD)"

# An unrelated pull request (#1405's shape) merges into main AFTER the fork, adding a dependency
# this pull request never touched.
git checkout -q main
echo "toml = \"1.1.6\"" >>README
git add README
git commit -q -m "unrelated: an external dependency lands on main after the fork"
main_tip="$(git rev-parse HEAD)"

# Simulate actions/checkout's fetch: a local "origin/main" tracking ref at main's current tip, and
# check out the pull request branch detached, same as a pull_request job's worktree.
git update-ref refs/remotes/origin/main "$main_tip"
git checkout -q --detach "$pr_head"

fail() {
	echo "$me: $1" >&2
	exit 1
}

# 1: the merge base is the fork point, not main's post-fork tip.
got="$($under_test main)"
[ "$got" = "$fork_point" ] || fail "merge base: expected fork point $fork_point, got $got"

# 2: a diff against that merge base does not mention the unrelated dependency line (the bug).
if git diff --unified=1000000 "$got" HEAD -- README | grep -q "toml ="; then
	fail "diff against the resolved merge base still includes the unrelated main-only change"
fi

# 3: the same diff does include the pull request's own file.
git diff --unified=1000000 "$got" HEAD -- pr-file | grep -q "pr change" \
	|| fail "diff against the resolved merge base is missing the pull request's own change"

# 4: no "origin/<base-ref>" for this name; the fallback sha is used instead.
got="$($under_test no-such-branch "$fork_point")"
[ "$got" = "$fork_point" ] || fail "fallback: expected $fork_point, got $got"

# 5: no base-ref at all (a merge_group or plain push event's shape); the fallback sha is used.
got="$($under_test "" "$fork_point")"
[ "$got" = "$fork_point" ] || fail "empty base-ref: expected $fork_point, got $got"

# 6: no base-ref and no fallback: nothing on stdout, exit 1.
if out="$($under_test "" 2>/dev/null)"; then
	fail "no base resolvable: expected a nonzero exit, got 0 with output [$out]"
fi
[ -z "${out:-}" ] || fail "no base resolvable: expected empty stdout, got [$out]"

echo "scope-merge-base: merge base tracks the current base branch tip, not a stale sha; " \
	"empty-ref, fallback and failure paths all hold"
