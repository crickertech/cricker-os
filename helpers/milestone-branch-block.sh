#!/bin/sh
#
# script/lint's check 4b: a `milestone/N-*` branch must touch that milestone's roadmap block, per
# §90 (the claim is a draft pull request; the status flip is a gate).
#
#     helpers/milestone-branch-block.sh <repo-root> <branch>
#
# Exits 1 when the branch names a milestone that has no block and the lane was not renumbered.
# Prints a warning and exits 0 when the branch changes nothing in its block. Exits 0 quietly
# otherwise. script/lint has the history of the check; this file holds the logic so that
# helpers/milestone-branch-block-selftest.sh can plant cases against it.
#
# **A renumbered lane keeps its branch** (2026-10-10 UTC, #1901). A lane's milestone number is
# provisional until merge, so a collision moves its block, file, commits and pull request title to a
# new number. Its branch cannot follow: renaming the head branch of an open pull request closes the
# pull request. #1901 was cut as `milestone/868-...`, renumbered to 870 and then 871, and kept the
# 868 branch; this check then warned that it changed nothing in 868's block, which by then belonged to
# a different milestone. So before warning, look for the block the lane actually moved to: a
# roadmap block this branch changes whose slug equals the branch's slug, or whose text names the
# branch verbatim. Either one is the lane's own record of where it went
# (notes/skills/developer-lane/SKILL.md, shared state). A changed block that matches neither is not
# taken as a renumber, so a branch that forgot its block still warns.
#
# Name: provisional, 2026-10-10 (UTC), describing what it checks. calef has not ruled on it.

set -eu
repo=$1
branch=$2

case "$branch" in
milestone/*) ;;
*) exit 0 ;;
esac
rest="${branch#milestone/}"
milestone_n="${rest%%-*}"
slug="${rest#*-}"
case "$milestone_n" in
'' | *[!0-9]*) exit 0 ;; # no leading number: check 4's shape rule owns that
esac

# The block's filename pads the number to four digits (calef, 2026-10-07 UTC); the branch name does
# not, so `milestone/7-x` and `milestone/0007-x` both find 0007-*.md.
file_n="$(printf '%04d' "$(expr "$milestone_n" + 0)")"
roadmap_block="$(ls "$repo/design/roadmap/$file_n-"*.md 2>/dev/null | head -1 || true)"

# Against the merge base, not the working tree: the question is what this branch changes, and a
# branch that has merged main is not thereby excused.
merge_base="$(git -C "$repo" merge-base HEAD origin/main 2>/dev/null || true)"
changed=""
if [ -n "$merge_base" ] && [ -n "$(git -C "$repo" rev-list -n1 "$merge_base"..HEAD 2>/dev/null)" ]; then
    has_commits=yes
    changed="$(git -C "$repo" diff --name-only "$merge_base"..HEAD -- 'design/roadmap/[0-9]*.md')"
else
    has_commits=no
fi

# The renumbered lane's block, if the branch changes one that records it.
moved_to=""
for f in $changed; do
    base="${f##*/}"
    case "$base" in "$file_n-"*) continue ;; esac
    [ -f "$repo/$f" ] || continue
    if [ "${base#*-}" = "$slug.md" ] || grep -qF -- "$branch" "$repo/$f"; then
        moved_to="$f"
        break
    fi
done

if [ -n "$moved_to" ]; then
    echo "==> 4b: branch '$branch' was renumbered; its block is ${moved_to#design/}"
    exit 0
fi

hint() {
    echo "  If this lane was renumbered, keep the branch name (renaming the branch closes its pull" >&2
    echo "  request) and let the moved block record it: the same slug as the branch, or the" >&2
    echo "  branch name written in the block (notes/skills/developer-lane/SKILL.md, shared state)." >&2
}

if [ -z "$roadmap_block" ]; then
    echo "lint: branch '$branch' names milestone $milestone_n, which has no roadmap block." >&2
    hint
    exit 1
fi

if [ "$has_commits" = yes ] && ! printf '%s\n' "$changed" | grep -q "^design/roadmap/$file_n-"; then
    echo "lint: branch '$branch' changes nothing in ${roadmap_block#*design/}." >&2
    echo "  A milestone branch lands its status flip in the same merge as the work (§90)." >&2
    echo "  A lane edits its own milestone's roadmap block, and only that: this is the" >&2
    echo "  one exception to 'a developer never edits design/' (AGENTS.md, the developer" >&2
    echo "  role). If the status genuinely does not move, say so in the block: that" >&2
    echo "  sentence was missing four times in a week." >&2
    hint
fi
exit 0
