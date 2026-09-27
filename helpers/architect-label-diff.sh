#!/bin/sh
#
# helpers/architect-label-diff.sh: resolve the correct base to diff a pull request's OWN changes
# against, printing the merge-base sha on stdout.
#
#     helpers/architect-label-diff.sh <base-ref> [<fallback-base-sha>]
#         prints the merge-base sha (HEAD vs "origin/<base-ref>") on stdout, exit 0.
#         if "origin/<base-ref>" does not exist locally, falls back to <fallback-base-sha>.
#         prints nothing and exits 1 if neither resolves to a commit.
#
# # Why this exists
#
# `.github/workflows/architect-label.yml` used to diff `github.event.pull_request.base.sha`
# straight from the event payload against HEAD. That sha is a snapshot GitHub took when this pull
# request's base last changed FOR THIS PULL REQUEST (opened, or last synchronized); it does not
# track the base branch's current tip, so it goes stale the moment another pull request merges
# into the base branch without any push to this one. #1416 was flagged for `dependency:
# xtask/Cargo.toml: toml`: that dependency was #1405's, added to `main` after #1416 branched, and
# the payload's cached `base.sha` still pointed at #1414's merge, one commit behind #1405's on
# `main`. Diffing against that stale commit read #1405's own addition as though #1416 had
# introduced it, exactly the "diff was computed against a stale base" failure `ci.yml`'s identical
# `base.sha || merge_group.base_sha` pattern is also exposed to (not fixed here; a separate finding
# — see this lane's report).
#
# The fix: never trust the payload's cached sha as the primary source. `actions/checkout` with
# `fetch-depth: 0` already fetches every branch fresh in the same job, so `origin/<base-ref>` is
# the base branch's actual current tip, at the moment this job runs, for free. Take the merge base
# of THAT against HEAD (what a three-dot diff computes) rather than a two-dot diff against a sha
# that may already be behind it. A pull request's own diff is only ever the commits reachable from
# HEAD and not from that merge base; a base branch that has advanced past the fork point changes
# what the merge base IS, never what "the pull request's own changes" means.
#
# The fallback argument exists only for the case actions/checkout's own docs call out: a base
# branch renamed or deleted between the event firing and this job running, so `origin/<base-ref>`
# does not exist. It is the last-resort payload sha, kept for the same reason
# `architect-label.yml`'s original fallback comment gave: it costs nothing to keep this honest if
# the primary path cannot resolve.
#
# Pure git plumbing, no `gh`, no network beyond what the caller already fetched: cheap enough for
# `helpers/architect-label-diff-selftest.sh` to build a throwaway repo and run this against it
# directly, no subprocess mocking required.
#
# Name: provisional, minted by this lane, 2026-09-27; naming is an architect's call
# (design/naming.md).

set -eu

base_ref="${1:?usage: architect-label-diff.sh <base-ref> [<fallback-base-sha>]}"
fallback="${2:-}"

candidate="origin/$base_ref"
if ! git rev-parse --verify --quiet "$candidate" >/dev/null 2>&1; then
	candidate="$fallback"
fi

if [ -z "$candidate" ] || ! git rev-parse --verify --quiet "$candidate" >/dev/null 2>&1; then
	exit 1
fi

git merge-base HEAD "$candidate"
