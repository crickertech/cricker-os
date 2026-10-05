#!/bin/sh
#
# The merge drain, after 2026-10-03: it watches the queue and says who must act. It never arms,
# enqueues or re-queues a pull request.
#
#     helpers/merge-drain.sh              # one pass
#     helpers/merge-drain.sh --dry-run    # one pass, printing every write instead of making it
#
# PROVISIONAL NAME. Minted 2026-08-04 for a script that drained the queue by arming it, and kept
# when that stopped, because a rename is calef's call. See notes/merge-queue.md.
#
# # What it does
#
#   - Labels `needs-maintainer` (name provisional) on a pull request a maintainer session must pick
#     up, comments once per cause with the evidence, and takes the label off when the cause is
#     gone (`needs_maintainer`, below; the decision is helpers/needs-maintainer.jq).
#   - Dequeues a pull request that picked up `needs-architect` or `held-for-red-trunk` after it was
#     enqueued (`dequeue_held`).
#   - Labels a paused draft `unblocked` once its `Blocked-by:` pull requests have resolved.
#   - Reruns, once, a CI run a concurrency group cancelled as a same-second duplicate.
#   - Runs helpers/lane-claim-check.sh, which reports a pushed lane branch with no pull request.
#
# # What it stopped doing, and why (milestone 727 (a queue eviction goes to a maintainer session), provisional; calef's rulings on #1564)
#
# On 2026-10-03 calef was the queue's only working detector: he spotted each conflicting or
# failing pull request, took it out, and told the maintainer session. Meanwhile this script armed
# every eligible pull request on every pass, re-armed ejected ones, enqueued what auto-merge had
# left behind, and commented on stalls. It re-queued pull requests the calef account had just
# taken out six or seven times that day, gave the already-merged #1555 a stale queue entry, and its
# 478 stall comments since 2026-08-26 moved nobody (55% of pull requests saw an action in the hour
# after one, 50% in the hour before). The record is
# notes/coes/2026-10-03-the-queue-judged-one-pull-request-at-a-time.md.
#
# calef ruled: "I don't want the job of watching the queue." So the arming, the re-arming, the
# enqueue of a stranded pull request, the stall comments, the stale-draft and stuck-check reports
# and milestone 630 (a merge-queue ejection is caught before the queue, and recovered after it)'s ejection hold all went. A lane arms its own pull request in the same command
# as `gh pr ready`. What none of them did was hand a problem to someone whose job it is, and that
# is the one thing built in their place.
#
# # Where it runs
#
# `.github/workflows/merge-drain.yml`, as `nife-smelter[bot]`, every five minutes and whenever CI
# completes. A pass from a checkout is the same pass, as whatever `gh` is logged in as. The old
# watching form is gone: the schedule is the loop, and a loop on a laptop was a second actor.

set -e
cd "$(dirname "$0")/.."

REPO="nifeos/nife"
HELD_LABEL="needs-architect"

# **Which drain spoke.** A GitHub App installation token carries the App and not the caller, so
# GitHub cannot tell a reader whether the workflow or a laptop acted; the tag lives in the content.
# The workflow sets `actions:<run id>`, a run somebody can open; a laptop tags itself with its
# hostname. `$ME` prefixes every line this script prints and every comment it posts. The comment
# markers are deliberately not tagged: they are the dedupe key, and a key that changed with the
# instance would let two drains each post the same comment once.
INSTANCE="${MERGE_DRAIN_INSTANCE:-$(hostname -s 2>/dev/null || echo unknown)}"
ME="merge-drain[$INSTANCE]"

# A hold on the whole queue rather than on one pull request: `helpers/queue-hold.sh` places it
# while `main` is broken, so that one fix lands alone. See notes/main-is-red.md.
RED_TRUNK_LABEL="held-for-red-trunk"

dry=""
case "$1" in
--dry-run) dry=1 ;;
# Accepted and ignored: every run is one pass now, and the workflow and older notes say `--once`.
--once | "") ;;
*)
	echo "usage: $(basename "$0") [--dry-run]" >&2
	exit 2
	;;
esac

# Every write goes through here, so `--dry-run` can exercise a pass against the live repository and
# change nothing. That is how a change to this script is tried before it merges, since the
# scheduled workflow runs only from `main`.
w() {
	if [ -n "$dry" ]; then
		echo "$ME: (dry run) would run: $(printf '%s ' "$@" | tr '\n' ' ' | cut -c1-160)"
	else
		"$@" >/dev/null 2>&1
	fi
}

# **Dequeue anything now held that is already in the merge queue.** Admission is checked once, at
# enqueue time, and until 2026-09-18 nothing ever re-checked it, so a label arriving *after* the
# enqueue was ignored by everything: the `architect hold` check on the pull request went red, and the
# queue carried on regardless because the queue does not read labels.
#
# **This is not hypothetical and the window is small enough to lose a race in.** On 2026-09-18 this
# drain enqueued #923 at 03:53:36; its lane, which had discovered mid-flight that its work disproved
# a DECISIONS premise and therefore needed calef, labelled it `needs-architect` at 03:54:49. Seventy
# three seconds. calef noticed it merging and it was dequeued by hand. The lane had already dequeued
# itself once at 03:08:05, and this drain simply put it back, which is the part a lane cannot defend
# against on its own.
#
# The shape is AGENTS.md's ladder: the label was rung two (a gate that fires without being
# remembered) for everything *except* the queue, where it was rung zero. This closes that, on the
# side that can see both facts. A lane discovering late that it needs an architect is the normal
# case rather than the exceptional one, because finding the thing that needs deciding is usually the
# work.
dequeue_held() {
	gh pr list --repo "$REPO" --state open --json number,labels,title 2>/dev/null |
		jq -r --arg L "$HELD_LABEL" --arg R "$RED_TRUNK_LABEL" '
			.[]
			| (.labels | map(.name)) as $names
			| ([$L, $R] | map(select(. as $l | $names | index($l))) | first) as $why
			| select($why != null)
			| "\(.number)\t\($why)\t\(.title)"' 2>/dev/null |
		while IFS="$(printf '\t')" read -r num why title; do
			[ -n "$num" ] || continue
			# `dequeuePullRequest` is a no-op on a pull request that is not queued, so this needs no
			# membership test: asking is cheaper than checking, and the check would race anyway.
			# REST rather than GraphQL for the lookup: it takes `owner/repo` as one string, so it
			# needs no second source of truth for the repository's name beyond $REPO.
			id=$(gh api "repos/$REPO/pulls/$num" --jq '.node_id' 2>/dev/null) || continue
			[ -n "$id" ] || continue
			before=$(gh api "repos/$REPO/issues/$num/timeline" \
				--jq '[.[] | select(.event=="removed_from_merge_queue")] | length' 2>/dev/null || echo 0)
			w gh api graphql -f query="mutation{dequeuePullRequest(input:{id:\"$id\"}){clientMutationId}}" || continue
			after=$(gh api "repos/$REPO/issues/$num/timeline" \
				--jq '[.[] | select(.event=="removed_from_merge_queue")] | length' 2>/dev/null || echo 0)
			# Only speak when something actually moved. A held pull request that was never queued is
			# the common case and saying so every five minutes is how a watcher gets muted.
			if [ "$after" -gt "$before" ]; then
				echo "$ME: DEQUEUED #$num ($why arrived after it was enqueued): $title"
			fi
		done
}

# What "eligible for the merge queue" means: not a draft, into `main`, and from this repository
# rather than a fork. One file, helpers/queue-eligible.jq, shared with queue-hold.sh and the
# detector's decision; helpers/queue-eligible-selftest.sh checks it under script/lint.
ELIGIBLE_JQ="$(dirname "$0")/queue-eligible.jq"

# Whether pull request $1 already carries a comment with marker $2.
marked() {
	count=$(gh pr view "$1" --repo "$REPO" --json comments 2>/dev/null |
		jq -r --arg m "$2" '[.comments[] | select(.body | contains($m))] | length' 2>/dev/null) || true
	[ "${count:-0}" != "0" ]
}

# Post `$3` on pull request `$1` once, ever, keyed by the marker `$2` (an HTML comment, invisible
# when rendered). A condition that persists is re-detected every pass, and a comment every pass
# buries the one useful comment within the hour.
notify() {
	marked "$1" "$2" && return 0
	w gh pr comment "$1" --repo "$REPO" --body "$3

<!-- $2 -->"
}

# `Blocked-by: #N[, #M ...]` in a pull request body: a hold that names its own release condition,
# for a mechanical ordering constraint (two branches that are green alone and red together), as
# opposed to `needs-architect`, which means a person must decide something. A manual hold label
# was refused because a label has to be removed by whoever remembers, and this tree has watched
# that fail (notes/merge-queue.md has the history). On a draft it earns the `unblocked` label
# below; on a ready pull request it keeps the detector's `unarmed` cause off until every pull
# request it names has merged or closed. helpers/blocked-by.sh is the parser.
. "$(dirname "$0")/blocked-by.sh"

UNBLOCKED_LABEL="unblocked"
BLOCKED_BY_RESOLUTION_JQ="$(dirname "$0")/blocked-by-resolution.jq"

# A paused DRAFT whose `Blocked-by:` line has resolved. A draft that is genuinely waiting on
# another pull request looks exactly like a finished lane that went quiet, so the waiting has to be
# written where a machine can read it.
#
# # Why this exists
#
# Draft #1289 was paused on 2026-09-25 waiting on #1288. #1288 merged an hour later. Nobody resumed
# #1289 for two days, because the pause was recorded only in prose (a comment, a report). A mechanism that fires once and a fact that lives only in prose is exactly the shape
# AGENTS.md's ladder warns about: "somebody will notice" is rung zero.
#
# So this is rung two instead: a draft that names `Blocked-by: #N[, #M ...]` in its own body gets a
# label, `unblocked`, and one comment, the moment every listed pull request has merged or been
# reported closed. Both are visible without opening the pull request: `gh pr list --label unblocked`
# is the whole board, the same shape as `gh pr list --draft` and `gh pr list --label
# needs-architect` already are.
#
# The decision itself (merged, still open, closed unmerged, or already labelled) is
# helpers/blocked-by-resolution.jq, spliced in below the same way helpers/queue-eligible.jq is
# elsewhere in this script; helpers/blocked-by-resolution-selftest.sh checks it against fixtures
# without a `gh` call, and script/lint runs that selftest. The label and the marker comment are
# this function's own job, because they need `gh` and the jq file must not.
#
# **A blocker CLOSED without merging is reported as such rather than folded into a plain
# "unblocked"**: it usually means the plan changed, and that is
# a fact for a person to read rather than release silently.
unblocked_drafts() {
	gh pr list --repo "$REPO" --state open --json number,isDraft,title,body,labels 2>/dev/null |
		jq -c --arg L "$UNBLOCKED_LABEL" '
			.[]
			| select(.isDraft == true)
			| select(.body | test("(?i)blocked-by:"))
			| {number, title, body, has_label: ((.labels | map(.name) | index($L)) != null)}
		' 2>/dev/null |
		while IFS= read -r pr; do
			[ -n "$pr" ] || continue
			num=$(printf '%s' "$pr" | jq -r '.number')
			title=$(printf '%s' "$pr" | jq -r '.title')
			body=$(printf '%s' "$pr" | jq -r '.body')
			has_label=$(printf '%s' "$pr" | jq -r '.has_label')

			blockers=$(nife_blocked_by "$body")
			[ -n "$blockers" ] || continue

			# One `gh pr view` per named blocker, building the fixture `blocked-by-resolution.jq`
			# reads and the human-readable lines (with timestamps) the jq file deliberately does
			# not carry, because its own selftest fixtures are state-only.
			blockers_json='[]'
			merged_line=""
			closed_line=""
			for b in $blockers; do
				bjson=$(gh pr view "$b" --repo "$REPO" --json state,mergedAt 2>/dev/null)
				bstate=$(printf '%s' "$bjson" | jq -r '.state // "OPEN"' 2>/dev/null)
				[ -n "$bstate" ] || bstate="OPEN"
				blockers_json=$(printf '%s' "$blockers_json" |
					jq -c --argjson n "$b" --arg s "$bstate" '. + [{number: $n, state: $s}]')
				case "$bstate" in
				MERGED)
					mergedat=$(printf '%s' "$bjson" | jq -r '.mergedAt // "an unknown time"' 2>/dev/null)
					merged_line="${merged_line}#$b merged at $mergedat; "
					;;
				CLOSED)
					closed_line="${closed_line}#$b "
					;;
				esac
			done

			result=$(jq -cn --argjson hl "$has_label" --argjson bl "$blockers_json" \
					'{has_label: $hl, blockers: $bl}' |
				jq -c "$(cat "$BLOCKED_BY_RESOLUTION_JQ")"'resolution' 2>/dev/null)
			status=$(printf '%s' "$result" | jq -r '.status' 2>/dev/null)

			case "$status" in
			already-labelled | waiting | "")
				continue
				;;
			closed-unmerged)
				msg="merge-drain: UNBLOCKED. ${closed_line}closed unmerged; resume this lane and check whether the plan changed."
				;;
			unblocked)
				msg="merge-drain: UNBLOCKED. ${merged_line}resume this lane."
				;;
			esac

			echo "$ME: UNBLOCKED #$num ($title)"
			notify "$num" "merge-drain:unblocked" "$msg"
			w gh pr edit "$num" --repo "$REPO" --add-label "$UNBLOCKED_LABEL" || true
		done
}

# The `unblocked` label removes itself the moment its own reason stops applying, the same shape as
# `Blocked-by:` itself: nothing to remember, because a manual label that outlives its reason is a
# false signal and this project has the receipts for that (see the comment above `blocked_by`,
# now `nife_blocked_by` in helpers/blocked-by.sh). Its reason stops applying in exactly two ways:
# a new commit lands (the lane resumed on its own before anyone read the comment), or the draft is
# marked ready (it is not paused any more, it is asking to merge). Checked every pass rather than
# left to the marker comment, because leaving a stale label costs nothing to detect and everything
# to trust if it lingers on a draft that has clearly moved on.
release_unblocked_labels() {
	gh pr list --repo "$REPO" --state open --json number,isDraft,labels,commits 2>/dev/null |
		jq -r --arg L "$UNBLOCKED_LABEL" '
			.[]
			| select((.labels | map(.name) | index($L)) != null)
			| [.number, .isDraft, (.commits[-1].committedDate // "")]
			| @tsv
		' 2>/dev/null |
		while IFS="$(printf '\t')" read -r num isdraft lastcommit; do
			[ -n "$num" ] || continue
			if [ "$isdraft" = "false" ]; then
				w gh pr edit "$num" --repo "$REPO" --remove-label "$UNBLOCKED_LABEL" || true
				continue
			fi
			[ -n "$lastcommit" ] || continue
			labeled_at=$(gh api "repos/$REPO/issues/$num/timeline" --paginate 2>/dev/null |
				jq -r --arg L "$UNBLOCKED_LABEL" '
					[.[] | select(.event == "labeled" and .label.name == $L)] | last | .created_at // empty
				' 2>/dev/null)
			[ -n "$labeled_at" ] || continue
			newer=$(jq -n --arg a "$lastcommit" --arg b "$labeled_at" \
				'(($a | fromdateiso8601) > ($b | fromdateiso8601))' 2>/dev/null)
			if [ "$newer" = "true" ]; then
				w gh pr edit "$num" --repo "$REPO" --remove-label "$UNBLOCKED_LABEL" || true
			fi
		done
}

# **Rerun the CI run a concurrency group cancelled as a duplicate, once** (2026-09-24, #1203's
# cause, found by the A′ lane and written up in notes/merge-queue.md's BUGS). One push can raise two
# `synchronize` events; the group cancels the newer copy before any job exists; GitHub reads the
# newest run per workflow, so the empty cancelled suite hides the green one and the queue answers
# "11 of 13 required status checks are expected". No workflow-level fix is sound, so the drain
# reruns the run it finds. It is not arming: it makes a check report, and the pull request's own
# auto-merge, or its absence, decides the rest. The detection is helpers/cancelled-duplicate.jq, the note's own query,
# and `rerunnable` is what decides: only a run at `run_attempt` 1, so the same run is never rerun
# twice and the run itself is the record. The rerun needs `actions: write`, which the App's token
# does not carry, since milestone 128 (the automation gets its own identity) minted it with
# Contents and Pull requests; `merge-drain.yml`
# passes the workflow's own token as MERGE_DRAIN_RERUN_TOKEN for this one call, and a laptop run
# uses whatever `gh` is logged in as.
CANCELLED_JQ="$(dirname "$0")/cancelled-duplicate.jq"
# The runs still owed a rerun at `$1` (a head SHA): "<id> <workflow name>" per line.
cancelled_duplicate_runs() {
	gh api "repos/$REPO/actions/runs?head_sha=$1&event=pull_request&per_page=100" 2>/dev/null |
		jq -r "$(cat "$CANCELLED_JQ")"'rerunnable | "\(.id) \(.name)"' 2>/dev/null
}
# `gh run rerun` with the token that may do it: the workflow's own under Actions, `gh`'s login on a
# laptop. Exported only for this call, and only when set, so a laptop run with no GH_TOKEN keeps
# its keyring login rather than an empty variable.
rerun_run() {
	if [ -n "$dry" ]; then
		echo "$ME: (dry run) would rerun run $1"
	elif [ -n "$MERGE_DRAIN_RERUN_TOKEN" ]; then
		GH_TOKEN="$MERGE_DRAIN_RERUN_TOKEN" gh run rerun "$1" --repo "$REPO" >/dev/null 2>&1
	else
		gh run rerun "$1" --repo "$REPO" >/dev/null 2>&1
	fi
}

# The candidates: eligible, BLOCKED, nothing still running, and a CANCELLED check at the head. The
# rollup is the cheap filter, so the runs API is asked only for a pull request in this shape.
rerun_cancelled_duplicates() {
	tab="$(printf '\t')"
	gh pr list --repo "$REPO" --state open \
		--json number,title,isDraft,baseRefName,isCrossRepository,mergeStateStatus,headRefOid,statusCheckRollup 2>/dev/null |
		jq -r "$(cat "$ELIGIBLE_JQ")"'
			.[] | eligible
			| select(.mergeStateStatus == "BLOCKED")
			| select(.statusCheckRollup
				| (map(select(.status == "QUEUED" or .status == "IN_PROGRESS" or .status == "PENDING")) | length) == 0
				  and (map(select(.conclusion == "CANCELLED")) | length) > 0)
			| "\(.number)\t\(.headRefOid)\t\(.title)"' 2>/dev/null |
		while IFS="$tab" read -r num sha title; do
			[ -n "$num" ] || continue
			cancelled_duplicate_runs "$sha" | while IFS=' ' read -r run_id run_name; do
				[ -n "$run_id" ] || continue
				if rerun_run "$run_id"; then
					echo "$ME: RERAN #$num run $run_id ($run_name was cancelled as a same-second duplicate and hid the green one) ($title)"
				else
					echo "$ME: #$num run $run_id ($run_name) is a cancelled duplicate and the rerun was refused; the token may lack actions:write ($title)"
				fi
			done
		done
}

# The merge group's runs that did not succeed, as markdown list lines: "- <name>: <conclusion>, <url>".
group_runs() {
	[ "$1" = "null" ] && return 0
	gh api "repos/$REPO/actions/runs?head_sha=$1&event=merge_group&per_page=100" \
		--jq '.workflow_runs[] | select(.conclusion != "success" and .conclusion != "skipped")
			| "- \(.name): \(.conclusion // .status), \(.html_url)"' 2>/dev/null || true
}

# **A pull request a maintainer session must pick up wears `needs-maintainer`** (milestone 727,
# provisional; the label's name is provisional too, and calef's call). The six causes, and why
# each is one, are in helpers/needs-maintainer.jq, which decides; this carries the decision out:
#
#   label   add the label, print `LABELLED #N`, and comment once per cause with the evidence
#   keep    comment on any cause whose episode has not been commented on yet
#   clear   the cause is gone (back in the queue, head moved, conflict fixed, armed, merged,
#           a draft again): take the label off and print `CLEARED #N`. A session never has to.
#
# The label is created idempotently by the workflow, with the workflow's own token, because the
# App's token may not create labels. A laptop pass assumes it exists.
#
# The session-side half is rung three, a written record: briefs/session-start.md runs
# `gh pr list --label needs-maintainer --state all` first and fixes those pull requests first,
# and helpers/nanny.py wakes a running session when the label lands. `--state all` because a stale
# queue entry belongs to a pull request that is no longer open.
NM_LABEL="needs-maintainer"
NM_MINUTES=${NM_MINUTES:-30}
NM_JQ="$(dirname "$0")/needs-maintainer.jq"
# Every field helpers/needs-maintainer.jq reads, in one call. helpers/needs-maintainer-selftest.sh
# checks this text names each of them, and its fixtures are this query's recorded responses.
NM_QUERY='query($owner: String!, $name: String!, $labelled: String!) {
  repository(owner: $owner, name: $name) {
    mergeQueue(branch: "main") { entries(first: 100) { nodes { enqueuedAt state pullRequest { id number state mergedAt } } } }
    pullRequests(states: OPEN, first: 100, orderBy: {field: CREATED_AT, direction: DESC}) { nodes {
      number isDraft baseRefName isCrossRepository headRefName headRefOid createdAt mergeable body
      labels(first: 30) { nodes { name } }
      autoMergeRequest { enabledAt }
      removed: timelineItems(last: 1, itemTypes: [REMOVED_FROM_MERGE_QUEUE_EVENT]) { nodes { ... on RemovedFromMergeQueueEvent { createdAt reason beforeCommit { oid parents(first: 2) { nodes { oid } } } } } }
      added: timelineItems(last: 1, itemTypes: [ADDED_TO_MERGE_QUEUE_EVENT]) { nodes { ... on AddedToMergeQueueEvent { createdAt } } }
      unarmed: timelineItems(last: 1, itemTypes: [READY_FOR_REVIEW_EVENT, AUTO_MERGE_DISABLED_EVENT, REMOVED_FROM_MERGE_QUEUE_EVENT]) { nodes { __typename ... on ReadyForReviewEvent { createdAt } ... on AutoMergeDisabledEvent { createdAt } ... on RemovedFromMergeQueueEvent { createdAt } } }
      labelled: timelineItems(last: 20, itemTypes: [LABELED_EVENT]) { nodes { ... on LabeledEvent { createdAt label { name } } } }
    } }
  }
  search(query: $labelled, type: ISSUE, first: 50) { nodes { ... on PullRequest { number state labels(first: 30) { nodes { name } } } } }
}'

# The files a head conflicts on against `main`, as markdown list lines. `git merge-tree
# --write-tree` needs no worktree; it exits 1 on a conflict and prints the tree, then one
# conflicted path per line. The workflow's checkout has every branch, so a same-repository head is
# local. GitHub's `mergeable` is computed lazily and can trail `main` (on 2026-10-03 it called #1569
# CONFLICTING while git merged it cleanly), so a clean merge here is said rather than hidden.
conflict_files() {
	if ! git cat-file -e "$1^{commit}" 2>/dev/null; then
		echo "- (the head is not in this checkout, so the conflicting files are not listed)"
		return 0
	fi
	if out=$(git merge-tree --write-tree --name-only --no-messages origin/main "$1" 2>/dev/null); then
		echo "- (git merges this head cleanly against \`main\` at $(git rev-parse --short origin/main 2>/dev/null), so GitHub's verdict may be stale; re-check it before rebasing)"
	else
		printf '%s\n' "$out" | tail -n +2 | head -20 | sed 's/^/- `/; s/$/`/'
	fi
}

# The comment for one cause ($2, a JSON object) on pull request $1.
nm_comment() {
	c="$2"
	cause=$(printf '%s' "$c" | jq -r '.cause')
	case "$cause" in
	ejected)
		at=$(printf '%s' "$c" | jq -r '.at')
		reason=$(printf '%s' "$c" | jq -r '.reason')
		group=$(printf '%s' "$c" | jq -r '.group')
		head=$(printf '%s' "$c" | jq -r '.head')
		runs=$(group_runs "$group")
		[ -n "$runs" ] || runs="- no merge-group run to name (a \`merge_conflict\` builds none)"
		what="EJECTED from the merge queue at $at, reason \`$reason\`, at head \`$head\`. The ejection cancelled auto-merge, and nothing re-queues it.

$runs

Hand it to its lane: a pushed fix, or the conflict resolved, takes this label off. If the failure was a flake, re-arm it (\`gh pr merge $1 --auto --merge\`); back in the queue takes the label off too."
		;;
	conflict)
		head=$(printf '%s' "$c" | jq -r '.head')
		files=$(conflict_files "$head") || files=""
		what="CONFLICTS with \`main\` at head \`$head\`.

$files

Rebase or merge \`main\` (briefs/rebase-onto-main.md), push, and arm it. The label comes off when the conflict is gone."
		;;
	unarmed)
		since=$(printf '%s' "$c" | jq -r '.since')
		blockers=$(printf '%s' "$c" | jq -r 'if (.blockers | length) > 0 then " Its `Blocked-by:` pull requests have all resolved (" + (.blockers | join(", ")) + ")." else "" end')
		what="READY AND UNARMED since $since, over $NM_MINUTES minutes, and not in the merge queue.$blockers Nothing arms a pull request but its lane or a maintainer session, and a lane that has ended its turn is not running: the maintainer session owns this.

If it is done, arm it: \`gh pr merge $1 --auto --merge\`. If it is not, make it a draft again: \`gh pr ready $1 --undo\`. Either takes the label off."
		;;
	off-main)
		since=$(printf '%s' "$c" | jq -r '.since')
		base=$(printf '%s' "$c" | jq -r '.base')
		basepr=$(gh pr list --repo "$REPO" --state open --head "$base" --json number -q '.[0].number // empty' 2>/dev/null) || basepr=""
		[ -n "$basepr" ] && basepr=" (#$basepr's head)" || basepr=""
		what="READY SINCE $since, over $NM_MINUTES minutes, and based on \`$base\`$basepr rather than \`main\`. The merge queue drains \`main\` only, so nothing will merge this, and nothing else watching the queue can see it. The maintainer session owns the next step, one of:

- merge it into its base, so it lands with that pull request: \`gh pr merge $1 --merge\`
- retarget it once its base has landed, then arm it: \`gh pr edit $1 --base main\`
- if it is waiting on its base on purpose, say so with a \`Blocked-by:\` line naming that pull request, or make it a draft: \`gh pr ready $1 --undo\`

Any of these takes the label off."
		;;
	red)
		since=$(printf '%s' "$c" | jq -r '.since')
		head=$(printf '%s' "$c" | jq -r '.head')
		branch=$(printf '%s' "$c" | jq -r '.branch')
		armed=$(printf '%s' "$c" | jq -r 'if .armed then "It is armed, and an armed pull request whose required checks fail never enters the queue, so it will sit here." else "It is not armed." end')
		what="RED: \`ci-failing\` has been on this pull request since $since, over $NM_MINUTES minutes, at head \`$head\`. The \`ci-failing\` comment names each failing check and its job. $armed

A lane that ended its turn \`WAITING\` is not running and will not see this. The maintainer session owns it: resume the lane that pushed \`$branch\` with the failing job's log, or fix it in place. A push whose checks pass takes \`ci-failing\` off, and this label with it."
		;;
	stale)
		state=$(printf '%s' "$c" | jq -r '.state')
		merged=$(printf '%s' "$c" | jq -r '.merged // "an unrecorded time"')
		enqueued=$(printf '%s' "$c" | jq -r '.enqueued')
		entry=$(printf '%s' "$c" | jq -r '.entry')
		id=$(printf '%s' "$c" | jq -r '.id')
		what="STILL IN THE MERGE QUEUE, though it is \`$state\` (merged at $merged). The entry was enqueued at $enqueued and reads \`$entry\`. Remove it:

    gh api graphql -f query='mutation{dequeuePullRequest(input:{id:\"$id\"}){clientMutationId}}'

The label comes off when the entry is gone."
		;;
	*)
		what="needs a maintainer for a cause this script does not describe: \`$cause\`."
		;;
	esac
	printf '%s' "$ME: needs-maintainer. $what

The label is how a maintainer session finds this without anyone watching the queue (milestone 727, provisional; helpers/needs-maintainer.jq has the six causes)."
}

needs_maintainer() {
	resp=$(gh api graphql -F owner="${REPO%/*}" -F name="${REPO#*/}" \
		-f labelled="repo:$REPO is:pr is:closed label:$NM_LABEL" -f query="$NM_QUERY" 2>/dev/null) || resp=""
	if [ -z "$resp" ] || [ "$(printf '%s' "$resp" | jq -r '.data.repository != null' 2>/dev/null)" != "true" ]; then
		# Loud, because a detector that fails quietly is the drain's old shape. The run stays green
		# so a GitHub outage does not page anyone twice; the next pass asks again.
		echo "$ME: the needs-maintainer query failed; nothing was labelled or cleared this pass"
		return 0
	fi

	# A ready pull request's `Blocked-by:` pull requests, resolved here because the jq file must not
	# call `gh`: { "<number>": ["OPEN", "MERGED", ...] }.
	blockers='{}'
	for num in $(printf '%s' "$resp" | jq -r '.data.repository.pullRequests.nodes[]
			| select(.isDraft == false and (.body // "" | test("(?i)blocked-by:"))) | .number'); do
		body=$(printf '%s' "$resp" | jq -r --argjson n "$num" '.data.repository.pullRequests.nodes[] | select(.number == $n) | .body')
		states='[]'
		for b in $(nife_blocked_by "$body"); do
			s=$(gh pr view "$b" --repo "$REPO" --json state -q .state 2>/dev/null) || s=""
			# Unknown reads as OPEN: a pull request is then not called unarmed this pass, and the
			# next pass asks again.
			states=$(printf '%s' "$states" | jq -c --arg s "${s:-OPEN}" '. + [$s]')
		done
		blockers=$(printf '%s' "$blockers" | jq -c --arg n "$num" --argjson s "$states" '. + {($n): $s}')
	done

	printf '%s' "$resp" |
		jq -c --arg l "$NM_LABEL" --argjson now "$(date +%s)" --argjson m "$NM_MINUTES" --argjson b "$blockers" \
			"$(cat "$ELIGIBLE_JQ" "$NM_JQ")"'nm_decide($l; $now; $m; $b)' |
		while IFS= read -r rec; do
			num=$(printf '%s' "$rec" | jq -r '.number')
			action=$(printf '%s' "$rec" | jq -r '.action')
			causes=$(printf '%s' "$rec" | jq -r '.causes | map(.cause) | join(", ")')
			case "$action" in
			clear)
				if w gh pr edit "$num" --repo "$REPO" --remove-label "$NM_LABEL"; then
					echo "$ME: CLEARED #$num (its cause is gone)"
				fi
				continue
				;;
			label)
				if w gh pr edit "$num" --repo "$REPO" --add-label "$NM_LABEL"; then
					echo "$ME: LABELLED #$num ($causes)"
				else
					echo "$ME: #$num needs a maintainer ($causes) and could not be labelled $NM_LABEL; does the label exist?"
				fi
				;;
			esac
			printf '%s' "$rec" | jq -c '.causes[]' | while IFS= read -r c; do
				marker="needs-maintainer:$(printf '%s' "$c" | jq -r '"\(.cause):\(.key)"')"
				notify "$num" "$marker" "$(nm_comment "$num" "$c")" || true
			done
		done
}

# # The log's event lines
#
# A snapshot answers "what is true now" and an event answers "what happened", and only events can
# be counted across passes (the correction is in `script/metrics` and notes/project-metrics.md).
# Each of these prints once per transition, never once per pass:
#
#     merge-drain[...]: DEQUEUED #N ...    a held pull request was taken out of the queue
#     merge-drain[...]: RERAN #N run <id>  a cancelled same-second duplicate was rerun, once
#     merge-drain[...]: UNBLOCKED #N ...   a paused draft's blockers resolved
#     merge-drain[...]: LABELLED #N ...    needs-maintainer added, with its causes
#     merge-drain[...]: CLEARED #N ...     needs-maintainer removed, its cause gone
pass() {
	# A pushed lane branch with no pull request is invisible to everything that starts from
	# `gh pr list`. Milestone 204 (a pushed lane branch with no draft pull request is a claim nobody
	# can see); the script owns its own grace period.
	sh helpers/lane-claim-check.sh || true
	dequeue_held || true
	unblocked_drafts || true
	release_unblocked_labels || true
	rerun_cancelled_duplicates || true
	needs_maintainer || true
}

pass
