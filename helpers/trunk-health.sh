#!/bin/sh
#
# Say when `main` is red, and say when it recovers.
#
#     helpers/trunk-health.sh             # watch until stopped
#     helpers/trunk-health.sh --once      # print the current state and exit
#
# PROVISIONAL NAME. Minted 2026-08-04; not put to calef. See the `Name:` block below.
#
# # Why this exists
#
# Nothing owned trunk health, and the gap was in the role definitions rather than in anyone's
# execution. A developer works one milestone in one lane and cannot see `main` by design. The
# steward watches pull request checks, conflicts, at-risk work and idle lanes, and its charter never
# mentioned the trunk. The maintainer is told to keep hygiene, and that list is prune the worktree,
# delete the branch, relink `nife-dev`, leave no QEMU. So the role that merges is the role that
# breaks `main`, and the role that exists to compensate for the merger being busy was not pointed at
# the thing merging breaks.
#
# **The signal was never missing** was this file's claim from 2026-08-04, and on 2026-09-23 it was
# **falsified and is corrected here rather than deleted**, because the correction is the lesson. CI
# does run on every push to `main` and has all along; what was wrong is the step after it, which is
# this script's own: that a green CI conclusion means a healthy tree. It does not. `ci.yml` skips
# the steps of the build jobs (`test`, `swish-check`, `boot-check`, and `swish-check-graphical` until its 2026-10-03 fold into `swish-check`)
# when every changed path matches `notes/`, `design/` or a root `*.md`, so a
# documentation-only commit posts a green required check having built and run nothing. That is a
# sound rule for a kernel and a false one for `crates/documentation`, whose tests RENDER those very
# files: `every_character_survives` never ran, `main` was broken for hours, and every signal a reader
# or this watcher could reach said green. Pull request #1168 fixes the skip. The durable lesson is
# about this file: **a conclusion is a claim about what ran, not about the tree.**
#
# The original point stands underneath it. On 2026-08-04 `main` went red and was found by someone
# running `script/lint` by hand after a merge, which is luck rather than process. This reads the
# signal that already exists, and now says what that signal does not cover.
#
# # What it says, and what it deliberately does not
#
# It reports the transition to red, naming the failing workflows, and the transition back to green.
# A green reached by citing a merge-group run rather than repeating it says so, with the run IDs.
# It does **not** report every red poll, because a trunk that stays broken for an hour is one fact
# and not twenty-four.
#
# It says "nobody is assigned to this" on purpose. A red trunk with an owner is a task; a red trunk
# without one is the failure this script exists to surface, and the wording is the difference.
#
# **Recovery is reported too, deliberately.** A watcher that only speaks on failure trains its reader
# to treat silence as health, and silence is also what a dead watcher produces.
#
# # Where this runs, and the watch that was folded in here and then unfolded (2026-09-24)
#
# **The trunk half runs in GitHub Actions as `nife-smelter[bot]`**, on a five-minute schedule:
# `.github/workflows/trunk-health.yml`, which carries the reasoning, the tested premise, and the
# cadence BUGS. It used to run under `launchd` on patagonia as calef's own token.
#
# **`helpers/at-risk-check.sh` did not come along, and could not have.** It was folded into this
# script's loop on 2026-09-23 on the reasoning that a third watcher is a third thing to start and a
# third thing that can die silently, and that `com.nife.trunk-health` was already firing on the
# right interval. That reasoning was correct for as long as both halves ran on the same machine.
# They no longer do: everything else here reads GitHub, while the at-risk check reads **this
# machine's** worktrees and their `git status`. A runner has no lane worktrees, so carrying the fold
# into Actions would have produced a check that reports nothing forever while the hazard it exists
# for sat on somebody's laptop unwatched, which is worse than not having it.
#
# So the at-risk watch is per developer, one per machine, under its own `launchd` job, and it needs
# no GitHub credential at all. notes/merge-queue.md has the plist and the retirement commands for
# the two jobs this replaces. `--once` here still reports it, because a person running this by hand
# on their own machine is exactly the case where both halves are true at once.
#
# # The thing that would prevent this rather than detect it
#
# GitHub's require-branches-to-be-up-to-date rule, applied 2026-08-04 (§73). Two pull requests, each
# green against the base it was cut from, merged in an order neither had ever been tested in and put
# `main` red. That rule forces a re-run against the new `main`, turning that failure into one re-run
# instead of a broken trunk. This script is the detection half; the rule is the prevention half.
#
# # BUGS
#
#   - **A green from a push that skipped its suites is sound, and says so** (A′, 2026-09-24). CI and
#     verify skip on a push to `main` when a `merge_group` run of the same workflow concluded
#     `success` at that exact commit, because the queue builds the very commit that lands. Those
#     runs conclude `success` with every gated job skipped, so this script reads them green, and
#     prints the merge-group runs it is relying on so a reader can tell that green from the one
#     below. It trusts the gate's lookup rather than re-deriving it: it does not check that the run
#     it names belongs to the same workflow as each skipped push run, only that some merge-group run
#     at this commit succeeded (CI or verify, whichever it found).
#   - **A green conclusion is not a healthy tree, and this script cannot tell the difference.** See
#     the correction above: a check whose steps were skipped reports `success`, so this watcher says
#     GREEN while the test that would have caught the breakage never ran. Nothing here reads whether
#     a job did any work, and a fix would mean reading each run's steps rather than its conclusion,
#     which is a different and much chattier API. Until then, a docs-only merge is a moment to
#     distrust this watcher rather than to be reassured by it.
#   - **It reports and never resolves**, deliberately, which was the whole gap calef named on
#     2026-09-23. The response now exists as `helpers/queue-hold.sh` and briefs/main-is-red.md, and
#     a person still has to run it: this script does not, because holding the queue needs the
#     judgement notes/merge-queue.md argues a watcher must not exercise.
#   - **It does not report its own death.** Shared with `helpers/merge-drain.sh`, accepted rather
#     than solved; notes/merge-queue.md has the reasoning and the `launchd` plists.
#
# Name: unrecorded. Provisional, minted 2026-08-04 and not yet put to calef. `trunk` rather than
# `main` because the branch could be renamed and the concept could not, and because "trunk health"
# is the term the field already uses. See notes/merge-queue.md.

set -e
cd "$(dirname "$0")/.."

REPO="nifeos/nife"
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


state() {
	full=$(git ls-remote "$(git remote get-url origin 2>/dev/null || echo origin)" refs/heads/main 2>/dev/null | cut -f1)
	sha=$(printf '%.8s' "$full")
	[ -z "$sha" ] && { echo "unknown"; return; }
	runs=$(gh run list --repo "$REPO" --branch main --limit 12 \
		--json workflowName,status,conclusion,headSha 2>/dev/null || echo '[]')
	failed=$(printf '%s' "$runs" | jq -r --arg s "$sha" \
		'[.[] | select(.headSha[0:8] == $s) | select(.conclusion == "failure") | .workflowName] | unique | join(", ")' 2>/dev/null)
	running=$(printf '%s' "$runs" | jq -r --arg s "$sha" \
		'[.[] | select(.headSha[0:8] == $s) | select(.status != "completed")] | length' 2>/dev/null)
	if [ -n "$failed" ]; then
		echo "RED $sha $failed"
	elif [ "$running" = "0" ]; then
		# Say which kind of green (A′, 2026-09-24): a push the merge group already tested skips its
		# suites and concludes `success` having run nothing here, which is sound only because the
		# merge-group run at this same commit did run them. Naming that run is what separates it
		# from the skipped-docs green in BUGS below. An API failure prints the plain form.
		tested=$(gh api "repos/$REPO/actions/runs?event=merge_group&head_sha=$full&status=success" \
			--jq '[.workflow_runs[] | select(.name == "CI" or .name == "verify") | "\(.name) \(.id)"] | join(", ")' 2>/dev/null || true)
		if [ -n "$tested" ]; then
			echo "GREEN $sha (merge group tested this commit: $tested)"
		else
			echo "GREEN $sha"
		fi
	else
		echo "PENDING $sha"
	fi
}

# **A scheduled workflow's red is invisible to `state()` above, structurally rather than by
# oversight**, and that is why this second question is asked here (milestone 238). `state()` filters
# runs to `headSha == main`'s current tip, which is the right filter for "is the trunk broken" and
# the wrong one for a cadence: a weekly job's run matches the tip only until the next merge, and at
# this tree's merge rate that window is minutes. So `mutation testing` failed four Mondays running
# and this watcher, pointed straight at the same API, could not have seen any of them.
#
# `script/cadence-check` is where the judgment lives; this only decides when to speak. Same
# transition discipline as RED/GREEN, for the same reason: a cadence that has been dead for a month
# is one fact, not four hundred polls of it. `|| true` because a dead cadence is its exit 1 and this
# script runs under `set -e`.
cadence() {
	script/cadence-check --quiet 2>/dev/null || true
}

# Relayed by `--once` only; the watching form no longer calls it. See the header section above for
# why the fold was undone. `helpers/at-risk-check.sh` reports and never acts; this only relays it,
# and on a machine with no lane worktrees it prints nothing rather than being wrong.
at_risk() {
	helpers/at-risk-check.sh 2>/dev/null || true
}

if [ -n "$once" ]; then
	s=$(state)
	case "$s" in
	RED*) echo "main is RED: ${s#RED }" ;;
	GREEN*) echo "main is green at ${s#GREEN }" ;;
	*) echo "main: ${s}" ;;
	esac
	c=$(cadence)
	[ -n "$c" ] && printf '%s\n' "$c"
	a=$(at_risk)
	[ -n "$a" ] && printf '%s\n' "$a"
	exit 0
fi

prev=""
prev_cadence="unknown"
while true; do
	s=$(state)
	case "$s" in
	RED*)
		[ "$s" != "$prev" ] && echo "MAIN IS RED at $(echo "$s" | cut -d' ' -f2) -- failing: $(echo "$s" | cut -d' ' -f3-) -- nobody is assigned to this"
		;;
	GREEN*)
		case "$prev" in
		RED*) echo "main recovered at $(echo "$s" | cut -d' ' -f2)" ;;
		esac
		;;
	esac
	prev="$s"

	# Recovery is announced here too, and for the reason the header already gives about RED/GREEN: a
	# watcher that only ever speaks on failure teaches its reader that silence means health, and
	# silence is also what a dead watcher sounds like.
	c=$(cadence)
	if [ "$c" != "$prev_cadence" ]; then
		if [ -n "$c" ]; then
			printf '%s\n' "$c"
		elif [ "$prev_cadence" != "unknown" ]; then
			echo "every scheduled workflow is producing results again"
		fi
		prev_cadence="$c"
	fi

	sleep 90
done
