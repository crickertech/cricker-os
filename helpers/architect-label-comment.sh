#!/bin/sh
# shellcheck shell=sh
#
# helpers/architect-label-comment.sh: post .github/workflows/architect-label.yml's one finding
# comment, marker-guarded so a pull request gets it once, ever.
#
# Pulled out of the workflow file rather than written inline in its `run:` block for a mechanical
# reason, not a style preference: a YAML block literal (`run: |`) requires every line of its body
# to be indented at least as much as the block's own indentation, and a multi-line comment body
# with a fenced code block starting in column 1 breaks that the moment it is written the way a
# human would write a comment. A plain shell script has no such constraint, so the body is written
# here the way it will actually render.
#
# The marker-guard is `helpers/merge-drain.sh`'s own `notify()`, copied rather than re-invented:
# check the pull request's comments for an HTML comment marker before posting, so a later push
# that trips a second rule does not bury the first finding under a duplicate. This marker is a
# single constant rather than one per rule (unlike `merge-drain.sh`'s per-stall marker), matching
# `.github/workflows/coe-architect-label.yml`'s own restraint: this job never removes the label, so
# there is nothing for a second comment to add once the first has named a reason.
#
# Usage: helpers/architect-label-comment.sh <pr-number> <owner/repo> <report>
#   <report>  the rule engine's own output, one "rule: file: detail" line per match
#
# Needs `gh`, authenticated (the workflow passes GH_TOKEN); needs jq for the marker check, same as
# merge-drain.sh's notify(). Never exits non-zero: a failed comment is logged and swallowed, same
# reasoning as architect-label.yml's own label step, and the same as coe-architect-label.yml's
# label step gives in full: no correction of error, and by extension no diff finding, should ever
# be blocked from existing because an API call to post a comment failed.
#
# Name: provisional, minted by this lane, 2026-09-27. Hyphenated per design/naming.md's `helpers/`
# entry-point rule; calef has not ratified it.

set -eu

pr_number="$1"
repo="$2"
report="$3"
marker="architect-label-rules-finding"

if ! command -v jq >/dev/null 2>&1; then
	echo "architect-label-comment: jq is not installed; cannot check for the marker, skipping." >&2
	exit 0
fi

existing="$(gh pr view "$pr_number" --repo "$repo" --json comments 2>/dev/null |
	jq -r --arg m "$marker" '[.comments[] | select(.body | contains($m))] | length' 2>/dev/null || echo 0)"

if [ "${existing:-0}" != "0" ]; then
	echo "architect-label-comment: marker already on PR #$pr_number; not posting again"
	exit 0
fi

body_file="$(mktemp)"
trap 'rm -f "$body_file"' EXIT

cat >"$body_file" <<BODY
This pull request's diff matched \`helpers/architect-label-rules.py\`, so \`needs-architect\` was added:

\`\`\`
$report
\`\`\`

Each line is \`rule: file: what matched\`; \`helpers/architect-label-rules.py\`'s docstring names all
six rules and their known false positives. This is a best-effort finding, not a verdict: a false
positive is normal, and a human removes the label if this does not need an architect. Someone still
needs to add a \`## What I need from you\` comment naming the actual ask (the decisions skill,
notes/skills/decisions/SKILL.md); a label with no ask under it is flagged \`needs-maintainer\`
(\`hold-no-ask\`) after 30 minutes. If calef has already ruled, record it with
\`script/record-ruling\`.

<!-- $marker -->
BODY

if gh pr comment "$pr_number" --repo "$repo" --body-file "$body_file" >/dev/null 2>&1; then
	echo "architect-label-comment: posted the finding on PR #$pr_number"
else
	echo "architect-label-comment: could not post the finding comment on PR #$pr_number." >&2
	echo "architect-label-comment: not fatal -- the label itself is still on the pull request." >&2
fi
exit 0
