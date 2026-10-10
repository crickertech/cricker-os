#!/bin/sh
#
# Selftest for helpers/milestone-branch-block.sh (script/lint's check 4b), run by script/lint.
# Builds a throwaway repository with an `origin/main` ref, so it needs no tree and no network. The
# cases are #1901's shape (cut as 868, renumbered to 871, branch kept) beside the mismatches the
# check exists to catch. The warning cases are the point: a check that never warns looks the same as
# one that cannot (design/roadmap/0640-a-gate-is-not-evidence-until-it-has-failed.md).
#
# Name: provisional, 2026-10-10 (UTC), after the other helpers/*-selftest.sh.

set -eu
check="$(cd "$(dirname "$0")" && pwd)/milestone-branch-block.sh"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
cd "$work"
export GIT_AUTHOR_NAME=t GIT_AUTHOR_EMAIL=t@t GIT_COMMITTER_NAME=t GIT_COMMITTER_EMAIL=t@t
export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_SYSTEM=/dev/null

fail() { echo "milestone-branch-block-selftest: $*" >&2; exit 1; }
g() { git -c commit.gpgsign=false -c core.hooksPath=/dev/null "$@"; }

# expect NAME WANT_EXIT WANT_TEXT|"" [NOT_TEXT]: run the check on the current branch.
expect() {
    name=$1 want=$2 text=$3 not=${4:-}
    set +e
    out=$(sh "$check" "$work" "$(g symbolic-ref --short HEAD)" 2>&1)
    got=$?
    set -e
    [ "$got" -eq "$want" ] || fail "$name: exit $got, wanted $want ($out)"
    if [ -n "$text" ]; then
        printf '%s' "$out" | grep -qF -- "$text" || fail "$name: output lacks '$text' ($out)"
    fi
    if [ -n "$not" ]; then
        printf '%s' "$out" | grep -qF -- "$not" && fail "$name: output has '$not' ($out)"
    fi
    return 0
}

# lane BRANCH: a fresh branch off main with one claim commit.
lane() {
    g checkout -q -b "$1" main
    g commit -q --allow-empty -m "claim"
}

g init -q -b main .
mkdir -p design/roadmap
echo "relibc's seed" > design/roadmap/0868-relibcs-seed.md
echo "caches" > design/roadmap/0870-caches.md
g add -A; g commit -q -m base
g update-ref refs/remotes/origin/main main

# 1. A lane that moves its own block passes quietly.
lane milestone/870-caches-again
echo "caches, built" > design/roadmap/0870-caches.md; g commit -q -am work
expect "own block" 0 "" "changes nothing"

# 2. #1901: cut as 868, renumbered to 871, branch kept, block slug matches the branch.
lane milestone/868-a-sixth-pass
echo "a sixth pass" > design/roadmap/0871-a-sixth-pass.md; g add -A; g commit -q -m work
expect "renumbered, slug" 0 "renumbered; its block is roadmap/0871-a-sixth-pass.md" "changes nothing"

# 3. Renumbered and retitled: the block names the branch, so it is still found.
lane milestone/868-old-title
printf 'Branch milestone/868-old-title, renumbered from 868.\n' > design/roadmap/0872-new-title.md
g add -A; g commit -q -m work
expect "renumbered, named" 0 "its block is roadmap/0872-new-title.md" "changes nothing"

# 4. Renumbered to a number main never had: the old number has no block, and that is not fatal.
lane milestone/875-a-thing
echo "a thing" > design/roadmap/0876-a-thing.md; g add -A; g commit -q -m work
expect "renumbered, old number free" 0 "its block is roadmap/0876-a-thing.md"

# 5. A real mismatch still warns: the branch changes another milestone's block, unrelated by slug or text.
lane milestone/868-a-sixth-pass-b
echo "caches, edited" > design/roadmap/0870-caches.md; g commit -q -am work
expect "unrelated block" 0 "changes nothing in roadmap/0868-relibcs-seed.md"

# 6. A branch that touches no block warns, with the renumbering hint.
lane milestone/868-forgot
echo x > other; g add -A; g commit -q -m work
expect "no block touched" 0 "If this lane was renumbered"

# 7. A number with no block and no renumber record still fails.
lane milestone/990-nowhere
echo x > other2; g add -A; g commit -q -m work
expect "no block at all" 1 "which has no roadmap block"

# 8. Not a milestone branch: nothing to say.
lane maintainer/whatever
expect "not a milestone" 0 "" "lint:"

echo "milestone-branch-block-selftest: ok"
