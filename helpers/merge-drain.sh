#!/bin/sh
#
# Drain the merge queue: enqueue every pull request that does not need an architect.
#
#     helpers/merge-drain.sh              # run until nothing is left to enqueue
#     helpers/merge-drain.sh --once       # one pass, then exit (for a cron or a check)
#
# PROVISIONAL NAME. Minted 2026-08-04; not put to calef. See the `Name:` block below.
#
# # Why this exists
#
# The maintainer holds merge authority, and merging is the one duty it is structurally worst at:
# when it is busy it is busy, and merging happens between conversations rather than during them. On
# 2026-08-04 two green pull requests sat unmerged for hours because nobody armed auto-merge, and the
# steward, which exists precisely to compensate for the maintainer being busy, only ever *reported*
# a stalled queue and never acted on one.
#
# # What this used to do, and why almost all of it is gone (2026-08-16)
#
# This script used to decide the merge ORDER: at most one pull request in flight, in flight before
# current before oldest, and one "Update branch" click per pass. That brain took four shapes and
# three of them starved something (the history is in notes/merge-queue.md, kept because it is
# evidence about the up-to-date rule rather than about this script).
#
# **GitHub's merge queue is now enabled on this repository, and it is that brain, one rung up.** It
# serializes candidates, tests each against the tip, and rejects what fails, which is exactly what
# the ordering logic was reconstructing from outside. Three consequences, all load-bearing:
#
#   - **Ordering is not ours any more.** Enqueue everything eligible; the queue decides.
#   - **Updating a branch is neither needed nor allowed.** The queue builds the merge candidate
#     itself, and GitHub answers `update-branch` on a queued pull request with a 422.
#   - **"Arm exactly one" is now the wrong answer**, not merely a redundant one: it leaves ready work
#     idle for a cycle when enqueueing costs nothing and the queue would have ordered it.
#
# # What is left, and why it is not nothing
#
# Two duties survive, and neither is something the platform knows:
#
#   - **The admission policy.** The queue merges what is enqueued; something has to decide what gets
#     enqueued. Drafts are not asking to be merged; `needs-architect` means the work is outside
#     standing merge authority (it touches the syscall surface, adds a dependency, or owes a
#     `DECISIONS` section); and `held-for-red-trunk` means the trunk is broken and one fix is landing
#     alone. CLAUDE.md describes the first label and the `## What I need from you` comment that goes
#     with it; notes/main-is-red.md describes the second.
#   - **Saying what stalled.** A queue never resolves a conflict (two were resolved by hand on
#     2026-08-16), and a pull request whose checks fail is ejected rather than fixed. Both need a
#     person, so both are reported and neither is retried.
#
# # Where this runs (2026-09-24)
#
# **In GitHub Actions, as `nife-smelter[bot]`**, on a five-minute schedule:
# `.github/workflows/merge-drain.yml`, which carries the reasoning, the tested premise, and the
# cadence BUGS. It used to run under `launchd` on patagonia as calef's own token, which conflated
# three actors under one name and made this singleton a singleton only because one laptop was awake.
# `helpers/lane-claim-check.sh` moved with it, because this script's `pass()` calls it; it needed no
# workflow of its own. notes/merge-queue.md has the retirement commands for the `launchd` jobs.
#
# The watching form still works from any checkout and is still the way to drive the queue by hand.
#
# A proposal to move the first duty onto a required check, which would make this script smaller
# still, is in design/decisions/ (`needs-architect` as a check rather than as a script's restraint).
#
# Name: unrecorded. Provisional, minted 2026-08-04 and not yet put to calef. Named for what it does
# to the queue rather than for the mechanism, in the family of `qemu-bounded.sh`. It lives in
# `helpers/` rather than `script/` because it is a maintainer's tool and not a front door a
# contributor types; `script/` is the normalised "Scripts to Rule Them All" set (notes/scripts.md).
# See notes/merge-queue.md.

set -e
cd "$(dirname "$0")/.."

REPO="nifeos/nife"
HELD_LABEL="needs-architect"

# **Which drain spoke.** "smelter did it" stops being an answer the moment the automation runs in
# more than one place, and as of 2026-09-24 it does: this script runs as a scheduled workflow
# (`.github/workflows/merge-drain.yml`) and still runs by hand from a checkout. A GitHub App
# installation token carries the App and not the caller, so GitHub itself cannot tell a reader which
# instance acted; the tag has to live in the content. That is rung three of AGENTS.md's ladder and
# there is nowhere higher to reach here.
#
# The workflow sets `actions:<run id>`, which is a run somebody can open. A laptop tags itself with
# its hostname. `$ME` prefixes every line this script prints and every comment it posts.
#
# The `notify()` MARKERS are deliberately not tagged: they are the dedupe key, and a key that
# changed with the instance would let two drains each post the same stall once.
INSTANCE="${MERGE_DRAIN_INSTANCE:-$(hostname -s 2>/dev/null || echo unknown)}"
ME="merge-drain[$INSTANCE]"

# **A second hold, and it is a hold on the whole queue rather than on one pull request.**
# `held-for-red-trunk` is placed by `helpers/queue-hold.sh` while `main` is broken, so that one fix
# lands alone against a trunk nothing else is racing. This script had to learn it, and the way it
# learned is the point: on 2026-09-23 the drain re-enqueued a held set **three times** while an
# operator watched, because it runs under `launchd` with `StartInterval 300` and its admission
# policy excluded exactly two things, drafts and `needs-architect`. A hold therefore survived five
# minutes at most, and the failure was invisible in the worst way: a dequeue leaves no trace of why
# an entry came back, so the operator concluded their own dequeue had failed and misdiagnosed it
# twice. The two labels mean different things (one pull request needs an architect; the trunk needs
# everybody to stop) and behave identically here, which is why they are two names and one policy.
RED_TRUNK_LABEL="held-for-red-trunk"
once=""
[ "$1" = "--once" ] && once=1

# A watcher must run from the MAIN checkout, never from a lane worktree, and this refuses rather
# than trusting anyone to remember. Measured cause, twice: `/bin/sh` reads a script LAZILY, so
# deleting the file under a running shell can kill it mid-loop. The merge drain died that way on
# 2026-08-18 when the worktree it was launched from was pruned, and `trunk-health.sh` died the same
# way later the same day during a 24-worktree cleanup, silently, while `main` was red on the
# fastpath gate for hours. The drain survived that second sweep only because it happened to have
# been relaunched with an absolute path into the main checkout.
#
# Only the watching form is refused. `--once` is a check anybody may run anywhere, including a lane
# gating its own work, and it exits long before a prune could reach it.
#
# `--git-dir` resolves to `.git/worktrees/<name>` in a linked worktree and to `.git` in the main
# checkout, which is the cheapest true test available; `--git-common-dir` points at the shared
# `.git` from both and cannot tell them apart.
if [ -z "$once" ] && [ "$(git rev-parse --git-dir 2>/dev/null)" != ".git" ]; then
	echo "$(basename "$0"): refusing to watch from a lane worktree." >&2
	echo "  A watcher outlives the lane that started it, and pruning that lane's worktree kills" >&2
	echo "  it silently, because /bin/sh reads a script lazily. Run it from the main checkout:" >&2
	echo "    cd <main checkout> && helpers/$(basename "$0") &" >&2
	echo "  ('--once' is fine from anywhere; only the watching form is refused.)" >&2
	exit 2
fi


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
# case
# rather than the exceptional one, because finding the thing that needs deciding is usually the
# work.
dequeue_held() {
	gh pr list --repo "$REPO" --state open 		--json number,labels,title 2>/dev/null |
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
			gh api graphql -f query="mutation{dequeuePullRequest(input:{id:\"$id\"}){clientMutationId}}" >/dev/null 2>&1 || continue
			after=$(gh api "repos/$REPO/issues/$num/timeline" \
				--jq '[.[] | select(.event=="removed_from_merge_queue")] | length' 2>/dev/null || echo 0)
			# Only speak when something actually moved. A held pull request that was never queued is
			# the common case and saying so every five minutes is how a watcher gets muted.
			if [ "$after" -gt "$before" ]; then
				echo "$ME: DEQUEUED #$num ($why arrived after it was enqueued): $title"
			fi
		done
}

# The unheld queue, lowest number first. Drafts are excluded: a draft is not asking to be merged.
# Both hold labels are excluded, for the reasons beside their definitions above.
#
# **And only pull requests into `main`.** A pull request stacked on another's branch (the pattern a
# maintainer uses so a follow-up can be reviewed before its base lands) targets a lane branch, which
# has no merge queue. `gh pr merge --auto` on it therefore does not enqueue: it merges straight into
# that branch the moment checks pass. That happened to #964 on 2026-09-19, merged into #963's branch
# rather than main, and its PR page read MERGED while `main` had none of it. Harmless that time,
# because it rode into main inside #963; a trap in general, because a stacked PR whose base is later
# abandoned reads as merged and is on no branch anyone lands. A stacked PR is left alone here, and
# GitHub retargets it to `main` when its base branch merges and is deleted, which is when it is armed.
#
# **And a head that lives in another repository is not a lane** (2026-09-24 security audit). Until
# that audit this predicate was "open, not a draft, against main", which admitted a fork's pull
# request from anyone on GitHub; with the ruleset on `main` requiring zero approving reviews, a
# stranger whose checks went green was one pass of this loop from merged by `nife-smelter[bot]`,
# unread. The predicate now lives in helpers/queue-eligible.jq, shared with queue-hold.sh and
# checked by helpers/queue-eligible-selftest.sh under script/lint, and it is spliced in front of
# the program below because jq cannot compose `-f` with inline text. If that file is missing, jq
# refuses the program and the `|| echo '[]'` arms nothing, which is the direction to fail in.
ELIGIBLE_JQ="$(dirname "$0")/queue-eligible.jq"
# `statusCheckRollup` rides along for `stranded_numbers` and the unreported-checks case below; it
# is the one field here that is a list rather than a scalar, at forty-odd entries per pull request.
queue() {
	gh pr list --repo "$REPO" --state open \
		--json number,mergeStateStatus,labels,isDraft,title,body,headRefName,headRefOid,baseRefName,autoMergeRequest,isCrossRepository,statusCheckRollup 2>/dev/null |
		jq -r --arg L "$HELD_LABEL" --arg R "$RED_TRUNK_LABEL" "$(cat "$ELIGIBLE_JQ")"'
			[ .[]
			  | eligible
			  | select((.labels | map(.name) | index($L)) | not)
			  | select((.labels | map(.name) | index($R)) | not) ]
			| sort_by(.number)' 2>/dev/null || echo '[]'
}

# **A report only this script's own stdout can see is not a report an architect will find in time.**
#
# # Why this exists
#
# On 2026-08-26 three armed pull requests (#530, #531, #532) sat "3 armed, 0 stalled" for hours
# while the queue drained nothing, because each carried a GitHub Actions run stuck `queued` with
# no job ever started. Once that stall shape was named (`stuck_checks` below), calef asked the
# obvious next question: could the script itself tell him, instead of a log file on patagonia that
# nothing prompts anyone to open. It can: `gh pr comment` is one API call, same shape as
# `gh pr merge --auto`.
#
# **The one real risk is spam, not correctness.** A stall that persists gets re-detected every
# five-minute pass, and posting a fresh comment every pass would bury the one useful comment under
# duplicates within the hour. So `notify` checks the pull request's own comments for a marker
# (an HTML comment, invisible when rendered) before posting, and posts once per marker per pull
# request, ever, not once per stall *episode*. A stall that clears and recurs later does not get a
# second comment. That is a real, accepted limitation rather than a solved problem: closing it needs
# either a timestamp-based cooldown or deleting the marker comment when a stall clears, and neither
# was worth building for a first cut. The five-minute log line still fires every pass regardless;
# only the PR comment is deduplicated.
notify() {
	num="$1"
	marker="$2"
	message="$3"
	existing=$(gh pr view "$num" --repo "$REPO" --json comments 2>/dev/null |
		jq -r --arg m "$marker" '[.comments[] | select(.body | contains($m))] | length' 2>/dev/null)
	if [ "${existing:-0}" = "0" ]; then
		gh pr comment "$num" --repo "$REPO" --body "$message

<!-- $marker -->" >/dev/null 2>&1
	fi
}

# A workflow run stuck at `queued`, no job ever started, no conclusion: the third stall shape, and
# neither DIRTY nor a FAILURE conclusion catches it, because both read false while a run sits in
# this state. `gh pr merge --auto` on a pull request in this state is not wrong, only useless: it
# re-arms a check that was never going to move, silently, forever.
#
# The threshold is generous for the same reason `STALE_DRAFT_MINUTES` is: this repository's own
# check suite (Kani proofs, fuzz targets, a full three-architecture boot) legitimately takes
# `in_progress` a long time. `queued` with zero jobs started for this long is a different thing:
# GitHub Actions ordinarily assigns a runner within seconds to low minutes, not tens of minutes.
STUCK_CHECK_MINUTES=${STUCK_CHECK_MINUTES:-20}

stuck_checks() {
	num="$1"
	head="$2"
	gh run list --repo "$REPO" --branch "$head" --json status,conclusion,createdAt --limit 5 2>/dev/null |
		jq -r --argjson mins "$STUCK_CHECK_MINUTES" --arg n "$num" --arg me "$ME" '
			(now - ($mins * 60)) as $cut
			| .[]
			| select(.status == "queued")
			| select((.createdAt | fromdateiso8601) < $cut)
			| "\($me): STALLED. #\($n) has a workflow run stuck queued for over " +
			  "\($mins) minutes with no job ever starting (GitHub infra, not this pull " +
			  "request). Push an empty commit to retrigger, or check the Actions tab."
		' 2>/dev/null || true
}

# A draft that has stopped moving is probably a finished lane that forgot to mark it ready.
#
# # Why this exists
#
# On 2026-08-19 PR #348's lane finished, reported, and left its pull request a draft. The drain
# excludes drafts **by design** ("a draft is not asking to be merged"), so it sat unmergeable for
# hours while every observer saw exactly what a healthy working lane looks like. It was found by
# calef asking why two pull requests were drafts, not by anything in this system.
#
# **That is the recurring shape rather than a one-off**: a state whose silence is indistinguishable
# from healthy operation. The same day, a watcher died and nothing said so while `main` was red, and
# a CI gate went red on a check that could not block a merge. In all three the observer was missing,
# not the signal.
#
# **Rung four is "tell lanes to mark ready", and that is what failed.** The mechanism has to be
# something that notices, so this reports a draft whose branch has stopped receiving commits. A live
# lane commits as it works, per AGENTS.md ("commit whenever a piece works and push whenever a commit
# exists"); a finished one goes quiet. Quiet for longer than a full gate takes is the signal.
#
# It **reports and does not act**. Marking somebody else's draft ready would be a judgement about
# whether their work is done, which is exactly the thing the draft is claiming. This says the words a
# person needs and leaves the decision.
#
# The threshold is generous on purpose. `script/test` runs both ISAs and a slow leg is tens of
# minutes, so anything tighter would fire on lanes that are working and teach everyone to ignore it.
STALE_DRAFT_MINUTES=${STALE_DRAFT_MINUTES:-75}

stale_drafts() {
	gh pr list --repo "$REPO" --state open --json number,isDraft,title,commits 2>/dev/null |
		jq -r --argjson mins "$STALE_DRAFT_MINUTES" --arg me "$ME" '
			(now - ($mins * 60)) as $cut
			| .[]
			| select(.isDraft == true)
			| select((.commits | length) > 0)
			| select((.commits[-1].committedDate | fromdateiso8601) < $cut)
			| [.number, ("\($me): STALE DRAFT. #\(.number) has not committed in over " +
			  "\($mins) minutes (\(.title[0:60])). If its lane is finished: gh pr ready \(.number). " +
			  "If it is waiting on another PR, add `Blocked-by: #N` to its body instead.")]
			| @tsv
		' 2>/dev/null |
		while IFS="$(printf '\t')" read -r num msg; do
			[ -z "$num" ] && continue
			echo "$msg"
			notify "$num" "merge-drain:stale-draft" "$msg"
		done
}

# A paused DRAFT whose `Blocked-by:` line has resolved: the mechanism `stale_drafts` above cannot
# be, because a draft that is genuinely waiting on another pull request should NOT be nagged with
# "if its lane is finished, gh pr ready" every time it goes quiet for `STALE_DRAFT_MINUTES`.
#
# # Why this exists
#
# Draft #1289 was paused on 2026-09-25 waiting on #1288. #1288 merged an hour later. Nobody resumed
# #1289 for two days, because the pause was recorded only in prose (a comment, a report) and
# `stale_drafts` posts its one STALE DRAFT note and then goes quiet by design (`notify`'s marker
# dedupes it). A mechanism that fires once and a fact that lives only in prose is exactly the shape
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
# "unblocked"**, mirroring the admission hold above: it usually means the plan changed, and that is
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
			gh pr edit "$num" --repo "$REPO" --add-label "$UNBLOCKED_LABEL" >/dev/null 2>&1
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
				gh pr edit "$num" --repo "$REPO" --remove-label "$UNBLOCKED_LABEL" >/dev/null 2>&1
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
				gh pr edit "$num" --repo "$REPO" --remove-label "$UNBLOCKED_LABEL" >/dev/null 2>&1
			fi
		done
}

# `Blocked-by: #N` in a pull request body: a SELF-RELEASING hold for a mechanical ordering
# constraint, as opposed to `needs-architect`, which means a person must decide something.
#
# # Why this exists, and why a plain "held" label was refused
#
# On 2026-08-18 #329 and #324 each carried a file named `97-*.md` in `design/decisions/`. Both were
# green alone; a merge-queue group containing both fails the decisions gate, because two sections
# cannot share a number. #329 was evicted as UNMERGEABLE while reporting CLEAN on its own page, and
# the only lever available to keep the drain from re-arming it was `needs-architect`, which says a
# person must rule on something. Using it here would have put a false entry on the architects'
# queue,
# which
# is the one queue in this project that must not accumulate noise.
#
# That is the same shape as #274, which was enqueued and evicted **29 times, 26 of them in a
# 3.5-hour loop**: #271 landed a doctest calling a method whose arity #274 was changing, git merged
# both without a conflict marker, and the pair was red only together. Green alone, green alone, red
# together is not a state any per-branch check can see.
#
# **A generic hold label was considered and refused**, and the reason is the failure mode rather
# than tidiness: a manual label has to be REMOVED by whoever remembers, and this project has a
# recorded history of exactly that going wrong. `needs-architect` was left on #320 and on #329 after
# both had been answered, on the same day, and calef found both. A hold that outlives its reason is
# a false blocker, and a false blocker is worse than none because it is believed.
#
# So the hold names its own release condition and evaporates without anybody acting: when #N merges,
# the next pass arms this pull request. Nothing to remember, and `gh pr list` shows the reason.
#
# The blocker being CLOSED rather than merged is reported loudly instead of silently released,
# because that is an anomaly: it means the thing this was sequenced behind is not coming.
# `nife_blocked_by`, the parser both this admission hold and the draft-unblock pass below use, and
# `nife_check_branch_name_shape`'s sourced-not-run counterpart: pulled into its own file
# 2026-09-27 so helpers/blocked-by-selftest.sh can check it without running this script's own
# `pass` loop. See helpers/blocked-by.sh for the convention and why every number on the line
# matters, not only the first.
. "$(dirname "$0")/blocked-by.sh"

# The label a paused DRAFT wears once every pull request its `Blocked-by:` line names has resolved,
# and the marker `notify` uses so the comment that says so posts once. See `unblocked_drafts` below
# and helpers/blocked-by-resolution.jq, the predicate that decides it.
UNBLOCKED_LABEL="unblocked"
BLOCKED_BY_RESOLUTION_JQ="$(dirname "$0")/blocked-by-resolution.jq"

# **Enqueue what the platform promised to and did not** (2026-09-24). Auto-merge is GitHub's promise
# to put a pull request into the queue when its checks go green. On 2026-09-24 #1202, #1200 and
# #1207 each sat armed, CLEAN, every required check green, and never entered `mergeQueue.entries`;
# each went in only when a person called the `enqueuePullRequest` mutation by hand, and a session
# watcher was doing that as a stopgap. This is that call, made by the drain, on the predicate in
# helpers/queue-stranded.jq: eligible (the same admission every arming passes, spliced first so the
# enqueue path cannot admit a head the drain would not arm), armed, CLEAN, absent from the queue,
# and in that state since before now minus STRANDED_MINUTES. **The minutes stand in for "two
# consecutive passes"**: each `merge-drain.yml` run is one `--once` pass in a fresh process, five
# minutes apart, so a pull request stranded for one interval is one that two passes in a row have
# seen stranded, and the default is that interval. No `jump`: the queue's order is the queue's.
STRANDED_MINUTES=${STRANDED_MINUTES:-5}
STRANDED_JQ="$(dirname "$0")/queue-stranded.jq"
# The numbers to enqueue, given `queue()`'s output and the numbers already queued (one per line).
stranded_numbers() {
	cutoff=$(( $(date +%s) - STRANDED_MINUTES * 60 ))
	queued_json=$(printf '%s\n' "$2" | jq -R 'select(length > 0) | tonumber' | jq -cs '.')
	printf '%s' "$1" | jq -r --argjson queued "$queued_json" --argjson cutoff "$cutoff" \
		"$(cat "$ELIGIBLE_JQ")$(cat "$STRANDED_JQ")"'[ .[] | stranded($queued; $cutoff) | .number ] | .[]' 2>/dev/null
}

# **Rerun the CI run a concurrency group cancelled as a duplicate, once** (2026-09-24, #1203's
# cause, found by the A′ lane and written up in notes/merge-queue.md's BUGS). One push can raise two
# `synchronize` events; the group cancels the newer copy before any job exists; GitHub reads the
# newest run per workflow, so the empty cancelled suite hides the green one and the queue answers
# "11 of 13 required status checks are expected". No workflow-level fix is sound, so the drain
# reruns the run it finds. The detection is helpers/cancelled-duplicate.jq, the note's own query,
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
	if [ -n "$MERGE_DRAIN_RERUN_TOKEN" ]; then
		GH_TOKEN="$MERGE_DRAIN_RERUN_TOKEN" gh run rerun "$1" --repo "$REPO" >/dev/null 2>&1
	else
		gh run rerun "$1" --repo "$REPO" >/dev/null 2>&1
	fi
}
# Whether any duplicate at `$1` has already had its one rerun: "yes" or nothing.
cancelled_duplicate_spent() {
	gh api "repos/$REPO/actions/runs?head_sha=$1&event=pull_request&per_page=100" 2>/dev/null |
		jq -r "$(cat "$CANCELLED_JQ")"'[ cancelled_duplicates | select(.run_attempt > 1) ] | if length > 0 then "yes" else empty end' 2>/dev/null
}

# The numbers currently IN the merge queue. One call, asked once per pass and reused, because
# `mergeQueue.entries` is the only thing that knows about a pull request whose arming has already
# become membership. See the verification block in `pass` for why neither field alone covers both
# shapes of "armed".
queued_numbers() {
	gh api graphql -f query='{repository(owner:"'"${REPO%/*}"'",name:"'"${REPO#*/}"'"){mergeQueue{entries(first:50){nodes{pullRequest{number}}}}}}' \
		--jq '.data.repository.mergeQueue.entries.nodes[].pullRequest.number' 2>/dev/null
}

# **An ejection is caught, said once, and recovered without re-spending a group on the same head**
# (milestone 630 (a merge-queue ejection is caught before the queue, and recovered after it)). The
# queue's ejection cancels the pull request's auto-merge and tells nobody: the sixth class in
# notes/coes/2026-09-30-lane-follow-through.md, and its mechanism 5. #1473 was ejected at
# 2026-10-03 01:45 UTC and nothing said so. This pass re-arms everything eligible, so it did re-arm
# after an ejection whenever it ran; what it did not do was say why the pull request had left, or
# tell an ejection worth retrying from one that would fail again at the same head, and with
# swish-check every group attempt costs 20 to 38 minutes.
#
# The decision is helpers/queue-ejected.jq, checked by its self-test, and notes/queue-ejection.md has
# the measurements behind it. What is done here:
#   - one comment per ejection (the marker carries the event's time, so a second ejection gets a
#     second comment), naming the reason and the group's runs that did not succeed, and an
#     `EJECTED #N` event line printed with it;
#   - the `queue-ejected` label (name provisional), and a hold on arming, only when a group run
#     FAILED or TIMED OUT at a head that has not moved. A cancelled run is runner supply, and the
#     pull request is re-armed at the same head as before;
#   - the label taken off when the head moves or the pull request is back in the queue, printed as a
#     `RELEASED #N` event; the ordinary pass below then arms it, unless it carries `needs-architect`,
#     and the queue admits it only once its own checks are green.
EJECTED_LABEL="queue-ejected"
EJECTED_JQ="$(dirname "$0")/queue-ejected.jq"
held=""
# One record per pull request that has a current ejection or carries the label, tab-separated:
# number action labelled reason at group ejected_head head ("-" for a null, so no field is empty).
ejection_states() {
	gh api graphql -f query='{repository(owner:"'"${REPO%/*}"'",name:"'"${REPO#*/}"'"){pullRequests(states:OPEN,first:100){nodes{number headRefOid labels(first:50){nodes{name}} removed:timelineItems(last:1,itemTypes:[REMOVED_FROM_MERGE_QUEUE_EVENT]){nodes{... on RemovedFromMergeQueueEvent{createdAt reason beforeCommit{oid parents(first:2){nodes{oid}}}}}} added:timelineItems(last:1,itemTypes:[ADDED_TO_MERGE_QUEUE_EVENT]){nodes{... on AddedToMergeQueueEvent{createdAt}}}}}}}' 2>/dev/null |
		jq -r --arg L "$EJECTED_LABEL" "$(cat "$EJECTED_JQ")"'
			.data.repository.pullRequests.nodes[] | ejection_state($L)
			| [.number, .action, .labelled, (.reason // "-"), (.at // "-"), (.group // "-"),
			   (.ejected_head // "-"), .head] | map(tostring) | join("\t")' 2>/dev/null
}
# The merge group's runs that did not succeed: "<conclusion>\t<name>\t<url>" per line.
group_runs() {
	[ "$1" = "-" ] && return 0
	gh api "repos/$REPO/actions/runs?head_sha=$1&event=merge_group&per_page=100" \
		--jq '.workflow_runs[] | select(.conclusion != "success" and .conclusion != "skipped")
			| "\(.conclusion // .status)\t\(.name)\t\(.html_url)"' 2>/dev/null
}
# Whether pull request $1 already carries a comment with marker $2.
marked() {
	count=$(gh pr view "$1" --repo "$REPO" --json comments 2>/dev/null |
		jq -r --arg m "$2" '[.comments[] | select(.body | contains($m))] | length' 2>/dev/null)
	[ "${count:-0}" != "0" ]
}
# The label is created on first use, so no setup step has to be remembered.
label_ejected() {
	gh pr edit "$1" --repo "$REPO" --add-label "$EJECTED_LABEL" >/dev/null 2>&1 && return 0
	gh label create "$EJECTED_LABEL" --repo "$REPO" --color B60205 \
		--description "Ejected from the merge queue on a failure; the drain holds this head (milestone 630)" >/dev/null 2>&1
	gh pr edit "$1" --repo "$REPO" --add-label "$EJECTED_LABEL" >/dev/null 2>&1
}
# Carries out the decisions and leaves the held numbers in `$held`. A `for` over lines rather than
# `| while`, because a pipe would run the loop in a subshell and `$held` would never leave it.
handle_ejections() {
	held=""
	states=$(ejection_states)
	[ -n "$states" ] || return 0
	nl='
'
	tab="$(printf '\t')"
	saved_ifs=$IFS
	IFS=$nl
	for rec in $states; do
		IFS=$tab
		# shellcheck disable=SC2086
		set -- $rec
		IFS=$nl
		e_num=$1 e_action=$2 e_labelled=$3 e_reason=$4 e_at=$5 e_group=$6 e_was=$7 e_head=$8
		if [ "$e_action" != "hold" ] && [ "$e_labelled" = "true" ]; then
			if gh pr edit "$e_num" --repo "$REPO" --remove-label "$EJECTED_LABEL" >/dev/null 2>&1; then
				if [ "$e_action" = "moved" ]; then
					echo "$ME: RELEASED #$e_num (a new head, ${e_head%"${e_head#????????}"}, was pushed after the ejection at $e_at)"
				else
					echo "$ME: RELEASED #$e_num (back in the queue, or taken out by hand, since it was ejected)"
				fi
			fi
			e_labelled=false
		fi
		[ "$e_action" = "release" ] && continue

		runs=$(group_runs "$e_group")
		failed=$(printf '%s\n' "$runs" | grep -cE '^(failure|timed_out)'"$tab" || true)
		marker="merge-drain:ejected:$e_at"
		if ! marked "$e_num" "$marker"; then
			list=$(printf '%s\n' "$runs" | awk -F'\t' 'NF == 3 { printf "- %s: %s, %s\n", $2, $1, $3 }')
			[ -n "$list" ] || list="- no merge-group run to name (a \`merge_conflict\` builds none)"
			if [ "$e_action" = "hold" ] && [ "$failed" -gt 0 ]; then
				next="The drain will not re-arm this head: every group attempt costs 20 to 38 minutes, and a failure at the same head usually fails again. It carries \`$EJECTED_LABEL\` until the head moves. Push a fix and the drain takes the label off and re-arms it, unless it carries \`needs-architect\`; the queue admits it once its own checks are green. If the failure was a flake, remove the label and the drain re-arms this head on its next pass."
				if label_ejected "$e_num"; then
					e_labelled=true
				else
					# Degrades to what the drain did before milestone 630: re-arm this head.
					echo "$ME: STALLED. #$e_num could not be labelled $EJECTED_LABEL (the label may not exist and this token may not create it), so this head is not held"
					next="The drain meant to hold this head and could not label it, so it will re-arm it as before. Push a fix rather than waiting on a second group build."
				fi
			elif [ "$e_action" = "hold" ] && [ "$e_group" != "-" ]; then
				next="Nothing in the group failed outright (cancelled, or still running when it was ejected). 13 of the 18 \`failed_checks\` ejections counted on 2026-10-03 were a cancelled CI run, which is runner supply rather than the change, so the drain re-arms this head on its next pass."
			elif [ "$e_action" = "hold" ]; then
				next="Resolve the conflict against \`main\` and push; the drain re-arms the new head."
			else
				next="A new head has been pushed since, and the drain arms it as usual."
			fi
			[ "$e_was" = "-" ] && e_was="(the event names no group commit)"
			gh pr comment "$e_num" --repo "$REPO" --body "$ME: EJECTED from the merge queue at $e_at, reason \`$e_reason\`, at head \`$e_was\`. The ejection cancelled auto-merge.

$list

$next (milestone 630; notes/queue-ejection.md)

<!-- $marker -->" >/dev/null 2>&1 &&
				echo "$ME: EJECTED #$e_num ($e_reason at $e_at, $failed group run(s) failed)"
		fi
		if [ "$e_action" = "hold" ] && [ "$e_labelled" = "true" ] && [ "$failed" -gt 0 ]; then
			held="$held $e_num"
		fi
	done
	IFS=$saved_ifs
}

# # This log has two kinds of line, and only one of them can be counted
#
# **A snapshot answers "what is true now"; an event answers "what happened".** Every line this
# script printed until 2026-09-23 was a snapshot, and the summary line is the clearest case:
# `10 armed, 7 stalled, of 17 unheld` is the state of the queue at the end of one pass, so summing
# it across passes double counts every pull request that was still armed on the next pass. With
# 3,355 passes on record in `~/Library/Logs/nife/merge-drain.log`, the question calef asked on
# 2026-09-23 -- how often does the drain act, as against him prompting a maintainer -- could not be
# answered from any of them. The `STALLED.` lines have the same defect: a stall that persists is
# re-detected and re-printed every pass, which is exactly why `notify` deduplicates the pull
# request comment and the log line does not.
#
# **This tree has made the same mistake once before and the correction is already written down**,
# so it is cited rather than re-argued: `script/metrics`' `built_by_week` and the "The only flow on
# this page" section of notes/project-metrics.md. A stock read late is merely stale; a flow read
# late lands in the wrong bucket.
#
# So two event lines are added, and both are named in capitals in the family of `STALLED.` so the
# log stays greppable by one pattern per kind:
#
#     merge-drain: ARMED #N ...        this pass put #N into the queue, or armed it to enter
#     merge-drain: DEQUEUED #N ...     this pass took #N back out
#     merge-drain: ENQUEUED #N ...     this pass put an armed, green #N into the queue itself,
#                                      because the platform had not (2026-09-24, see stranded_numbers)
#     merge-drain: RERAN #N run <id> .. this pass reran the CI run a concurrency group cancelled as a
#                                      same-second duplicate, once (2026-09-24, see cancelled_duplicate_runs)
#     merge-drain: EJECTED #N ...      this pass found #N's ejection from the queue and said so on it,
#                                      once per ejection (milestone 630, see handle_ejections)
#     merge-drain: RELEASED #N ...     this pass took `queue-ejected` off #N, because its head moved or
#                                      it is back in the queue (milestone 630)
#
# **What makes them events rather than snapshots is the suppression, not the wording.** Arming is
# idempotent and is attempted on every eligible pull request on every pass, so printing on every
# successful call would reproduce the summary line's defect with a new name on it. `ARMED` is
# therefore printed only where the pull request was *not* already armed when the pass began, which
# is what `armed_before` is for; a pull request armed on Monday and still queued on Tuesday
# contributes exactly one `ARMED` line. `DEQUEUED` already had this property and only needed the
# name: it prints only when the `removed_from_merge_queue` count actually moved.
#
# The summary line stays. It answers a question the events cannot ("is anything stuck right now"),
# and it is what the examples in notes/merge-queue.md show.
#
# Arming is one API call and changes nothing until the checks pass, so every eligible pull request
# is armed on every pass. Under the merge queue that is the whole job: an armed pull request enters
# the queue when its checks go green, and the queue lands them one at a time against the tip.
pass() {
	# A pushed lane branch with no pull request is invisible to everything below, because
	# everything below starts from `gh pr list`. Reported first, and before the empty-queue
	# return, because an empty queue is exactly when an unclaimed lane is easiest to miss:
	# nothing else on this pass will print a word. Milestone 204; the script owns its own
	# grace period and its own false-positive shapes.
	sh helpers/lane-claim-check.sh || true

	# Before admitting anything, reconcile what is already admitted: a label that arrived after an
	# enqueue is the one case the queue itself cannot see. See dequeue_held's own comment.
	dequeue_held || true

	# Same reasoning as `lane-claim-check.sh` above, and the same fix: these three read drafts
	# rather than the eligible queue, so they must not sit behind the empty-queue return below.
	# `stale_drafts` had exactly that blind spot until 2026-09-27, when it was moved here alongside
	# the two new passes rather than left where a repository with nothing else open would silently
	# stop reporting a paused draft too, which is precisely the shape this whole mechanism exists
	# to fix.
	stale_drafts
	unblocked_drafts
	release_unblocked_labels

	# Before arming anything: an ejected head that failed is held (milestone 630), and an ejection
	# is said once, on the pull request. See handle_ejections.
	handle_ejections || true

	q=$(queue)
	n=$(printf '%s' "$q" | jq -r 'length' 2>/dev/null || echo 0)
	if [ "$n" = "0" ] || [ -z "$n" ]; then
		echo "$ME: queue empty; nothing open that does not need an architect"
		return 1
	fi

	# What was ALREADY armed when this pass began. Both shapes, because the pull request object
	# reports a null `autoMergeRequest` once arming has become queue membership. This is the
	# baseline the `ARMED` event is printed against; see the events comment above `pass`.
	queued_now=$(queued_numbers)
	armed_before=" $(printf '%s' "$q" | jq -r '.[] | select(.autoMergeRequest != null) | .number' 2>/dev/null | tr '\n' ' ')$(printf '%s\n' "$queued_now" | tr '\n' ' ')"

	armed=0
	stalled=0
	attempted=""
	for num in $(printf '%s' "$q" | jq -r '.[].number'); do
		state=$(printf '%s' "$q" | jq -r --arg n "$num" '.[] | select(.number == ($n | tonumber)) | .mergeStateStatus')
		title=$(printf '%s' "$q" | jq -r --arg n "$num" '.[] | select(.number == ($n | tonumber)) | .title')

		# Ejected at this head on a failure: re-arming it would spend another group build on what
		# just failed. A snapshot line, not an event; `EJECTED` was the event.
		case " $held " in
		*" $num "*)
			echo "$ME: holding #$num: ejected from the merge queue on a failure at this head; push a fix, or remove $EJECTED_LABEL to retry it ($title)"
			continue
			;;
		esac

		# A declared ordering constraint, checked before anything else, because arming a pull
		# request that is sequenced behind another wastes a group build and can evict it.
		#
		# Every number on the `Blocked-by:` line is checked, not only the first (2026-09-27; the
		# BUGS note this fixed recorded that a pull request sequenced behind two others could only
		# say so once). `still_open` holds this pull request until every one of them has resolved;
		# `closed_unmerged` is reported even if some other blocker on the same line did merge,
		# because a blocker that closed without merging is the anomaly and deserves a person's
		# attention regardless of what the rest of the list did.
		body=$(printf '%s' "$q" | jq -r --arg n "$num" '.[] | select(.number == ($n | tonumber)) | .body')
		blockers=$(nife_blocked_by "$body")
		if [ -n "$blockers" ]; then
			still_open=""
			closed_unmerged=""
			for blocker in $blockers; do
				bstate=$(gh pr view "$blocker" --repo "$REPO" --json state -q .state 2>/dev/null)
				case "$bstate" in
				MERGED) ;;  # released, and nobody had to do anything
				CLOSED) closed_unmerged="$closed_unmerged #$blocker" ;;
				*) still_open="$still_open #$blocker" ;;
				esac
			done
			if [ -n "$closed_unmerged" ]; then
				msg="$ME: STALLED. #$num is blocked by$closed_unmerged, which closed without merging ($title)"
				echo "$msg"
				notify "$num" "merge-drain:blocker-closed" "$msg"
				stalled=$((stalled + 1))
				continue
			fi
			if [ -n "$still_open" ]; then
				echo "$ME: holding #$num until$still_open merges ($title)"
				continue
			fi
		fi

		# A conflict is the one state that cannot be waited out: the queue will not resolve it and
		# neither will another pass. Say which pull request it is and move on to the rest, because
		# one conflict must not stop the others being armed.
		if [ "$state" = "DIRTY" ]; then
			msg="$ME: STALLED. #$num has conflicts a person must resolve ($title)"
			echo "$msg"
			notify "$num" "merge-drain:conflict" "$msg"
			stalled=$((stalled + 1))
			continue
		fi

		# A failing check is the other. `--auto` on a failing pull request is harmless but says
		# nothing, so the failure is named instead: the queue ejects what fails, and nothing here
		# should retry it and burn CI.
		failed=$(gh pr view "$num" --repo "$REPO" --json statusCheckRollup \
			-q '[.statusCheckRollup[] | select(.conclusion == "FAILURE") | .name] | join(", ")' 2>/dev/null)
		if [ -n "$failed" ]; then
			msg="$ME: STALLED. #$num is failing $failed ($title)"
			echo "$msg"
			notify "$num" "merge-drain:check-failure" "$msg"
			stalled=$((stalled + 1))
			continue
		fi

		# A fourth shape (2026-09-24): BLOCKED with nothing failing and nothing running, and a
		# CANCELLED check at the head. That is the "N of M required checks expected" page, and its
		# one known cause is the same-second duplicate `cancelled_duplicate_runs` detects. The
		# rollup in `q` is the cheap pre-filter, so the runs API is asked only for a pull request
		# in this shape; then the cancelled duplicate is rerun once and said so as an event, a
		# duplicate already rerun is a stall a person must read, and a cancellation with no
		# same-second sibling is the older, unexplained stall line.
		if [ "$state" = "BLOCKED" ] && [ "$(printf '%s' "$q" | jq -r --arg n "$num" '
				.[] | select(.number == ($n | tonumber)) | .statusCheckRollup
				| (map(select(.status == "QUEUED" or .status == "IN_PROGRESS" or .status == "PENDING")) | length) == 0
				  and (map(select(.conclusion == "CANCELLED")) | length) > 0' 2>/dev/null)" = "true" ]; then
			sha=$(printf '%s' "$q" | jq -r --arg n "$num" '.[] | select(.number == ($n | tonumber)) | .headRefOid')
			# The loop runs in a subshell (a pipe), so its lines are collected and printed here
			# rather than counted there; the one thing the outer shell needs to know is whether
			# any rerun took.
			reran=$(cancelled_duplicate_runs "$sha" | while IFS=' ' read -r run_id run_name; do
				[ -n "$run_id" ] || continue
				if rerun_run "$run_id"; then
					echo "$ME: RERAN #$num run $run_id ($run_name was cancelled as a same-second duplicate and hid the green one) ($title)"
				else
					echo "$ME: STALLED. #$num run $run_id ($run_name) is a cancelled duplicate and the rerun was refused; the token may lack actions:write ($title)"
				fi
			done)
			if [ -n "$reran" ]; then
				printf '%s\n' "$reran"
				case "$reran" in
				*"RERAN #$num "*) continue ;;
				esac
				stalled=$((stalled + 1))
				continue
			fi
			if [ "$(cancelled_duplicate_spent "$sha")" = "yes" ]; then
				msg="$ME: STALLED. #$num has a cancelled duplicate run that was already rerun once and is still not reporting; a person should read it ($title)"
			else
				msg="$ME: STALLED. #$num has required checks that will never report: a run at its head was cancelled with no same-second sibling, which is not the shape the drain reruns (push an empty commit) ($title)"
			fi
			echo "$msg"
			notify "$num" "merge-drain:unreported-checks" "$msg"
			stalled=$((stalled + 1))
			continue
		fi

		# A third stall shape: neither DIRTY nor FAILURE, a run just never started. See
		# `stuck_checks`'s own comment for why this needs a person rather than a retry.
		head=$(printf '%s' "$q" | jq -r --arg n "$num" '.[] | select(.number == ($n | tonumber)) | .headRefName')
		stuck=$(stuck_checks "$num" "$head")
		if [ -n "$stuck" ]; then
			echo "$stuck"
			notify "$num" "merge-drain:stuck-check" "$stuck"
			stalled=$((stalled + 1))
			continue
		fi

		# NO `--delete-branch` HERE, and this is not a style preference. With a merge queue
		# enabled GitHub refuses the whole command with "Cannot use `-d` or `--delete-branch`
		# when merge queue enabled", so passing it enqueues NOTHING. The flag was also always
		# redundant: this repository sets `delete_branch_on_merge`, so the platform deletes the
		# head branch itself. `gh` prints "the merge strategy for main is set by the merge
		# queue" and enqueues anyway; that line is a notice, not a failure.
		#
		# The failure this cost: on 2026-08-17 the drain reported "9 armed" every pass for
		# three hours while the queue stayed empty and nothing merged, because the error went
		# to /dev/null and `|| true` swallowed the exit code. A count of ATTEMPTS was being
		# printed as a count of RESULTS.
		if ! gh pr merge "$num" --repo "$REPO" --auto --merge >/dev/null 2>&1; then
			msg="$ME: STALLED. #$num would not enqueue ($title)"
			echo "$msg"
			notify "$num" "merge-drain:would-not-enqueue" "$msg"
			stalled=$((stalled + 1))
			continue
		fi

		attempted="$attempted $num"
	done

	# What the platform left behind: armed, green, and not in the queue for a whole interval. The
	# `enqueuePullRequest` mutation is what a person types by hand for the same case; the drain
	# types it, once per stranded pull request per pass, and says so as an event. Verified below
	# with everything else that was attempted, so a call that took and changed nothing is a
	# `STALLED.` line and not a silent success.
	for num in $(stranded_numbers "$q" "$queued_now"); do
		title=$(printf '%s' "$q" | jq -r --arg n "$num" '.[] | select(.number == ($n | tonumber)) | .title')
		id=$(gh api "repos/$REPO/pulls/$num" --jq '.node_id' 2>/dev/null)
		if [ -z "$id" ] || ! gh api graphql -f query="mutation{enqueuePullRequest(input:{pullRequestId:\"$id\"}){clientMutationId}}" >/dev/null 2>&1; then
			msg="$ME: STALLED. #$num is armed and green but the queue refused it ($title)"
			echo "$msg"
			notify "$num" "merge-drain:would-not-enqueue" "$msg"
			stalled=$((stalled + 1))
			continue
		fi
		echo "$ME: ENQUEUED #$num (armed and green for $STRANDED_MINUTES minutes with no queue entry; the platform had not) ($title)"
		case " $attempted " in
		*" $num "*) ;;
		*) attempted="$attempted $num" ;;
		esac
	done

	# **Verify, and know that "armed" has two shapes, because neither field alone covers both.**
	#
	#   - Checks still running: the pull request carries an `autoMergeRequest` and reports
	#     `BLOCKED`. It enters the queue by itself when the last check goes green.
	#   - Checks green: it is IN the queue, and it reports `mergeStateStatus: CLEAN` with a
	#     **null** `autoMergeRequest`, because arming became membership.
	#
	# So a queued pull request looks unarmed on the pull request object, which is the same trap
	# that produced the bug above one level along: the obvious field looks authoritative and is
	# not. `mergeQueue.entries` is the only thing that knows about the second shape, and it is
	# asked once per pass rather than once per pull request.
	queued=$(queued_numbers)
	for num in $attempted; do
		if printf '%s\n' "$queued" | grep -qx "$num"; then
			now_armed=1
		elif [ "$(gh pr view "$num" --repo "$REPO" --json autoMergeRequest \
			-q '.autoMergeRequest != null' 2>/dev/null)" = "true" ]; then
			now_armed=1
		else
			now_armed=0
		fi

		if [ "$now_armed" = "0" ]; then
			msg="$ME: STALLED. #$num took the call but is neither queued nor armed"
			echo "$msg"
			notify "$num" "merge-drain:not-armed" "$msg"
			stalled=$((stalled + 1))
			continue
		fi

		armed=$((armed + 1))

		# The event, and the `case` is what keeps it one. A pull request armed on an earlier pass
		# is armed again on this one, harmlessly and by design, so only the transition is printed.
		case " $armed_before " in
		*" $num "*) ;;
		*)
			title=$(printf '%s' "$q" | jq -r --arg n "$num" '.[] | select(.number == ($n | tonumber)) | .title')
			echo "$ME: ARMED #$num ($title)"
			;;
		esac
	done

	echo "$ME: $armed armed, $stalled stalled, of $n unheld"

	# Nothing left to do on a pass where everything open is stalled: the remaining work needs a
	# person, and looping only re-prints the same lines.
	[ "$armed" -gt 0 ]
}

if [ -n "$once" ]; then
	pass || exit 0
	exit 0
fi

while pass; do
	sleep 150
done
