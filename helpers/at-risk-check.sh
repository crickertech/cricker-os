#!/bin/sh
#
# Report lane worktrees holding uncommitted modifications that have sat too long: the one failure
# in this system that destroys rather than delays, because a prune of that worktree is a prune of
# whatever it was holding.
#
#     helpers/at-risk-check.sh          # one pass, then exit
#
# PROVISIONAL NAME. Minted 2026-09-23; not put to calef. See `Name:` below.
#
# # Why this exists
#
# AGENTS.md gives this exact check to the steward and calls it the more valuable of its two watches:
#
#   "It watches for work at risk, not only for idleness: a lane worktree with modifications and no
#   commit in half an hour is uncommitted work one prune away from gone, which is the only failure
#   in this system that destroys rather than delays. That check earns its keep more than the idle
#   one."
#
# It did not exist. `launchctl list` shows `com.nife.merge-drain` and `com.nife.trunk-health` and
# nothing else; the duty AGENTS.md assigns by name had no mechanism behind it, which is rung four of
# AGENTS.md's own ladder (a sentence, relying on someone remembering it) for the specific case the
# sentence says is worst to leave there.
#
# Measured cost, all in one session on 2026-09-23: a fix to `helpers/open-lane.sh` found only while
# pruning merged worktrees, a fix to `system_tests/src/user/live_swap_tests.rs` that survived two prunes
# uncommitted and had to be recovered twice, and 62 lines of a decisions amendment sitting unsaved
# on a branch for hours after the conversation moved on. The maintainer pruned worktrees twice that
# session; any of the three could have been destroyed rather than merely delayed.
#
# # What this deliberately does not do
#
# **It reports and never acts.** It does not commit, does not stash, does not touch a file outside
# its own read. Committing on somebody else's behalf would guess at a message and a scope that are
# not this script's to invent; the fix is a person reading this line and running `git add`/`git
# commit` themselves.
#
# **It does not use `git stash`.** The stash stack is per-`.git`, not per-worktree, so it is shared
# machine-wide across every worktree of this repository: one worktree's `git stash` can be popped by
# a session working in a different worktree entirely, which is exactly the kind of action-at-a-
# distance this script exists to protect people from, not commit. See notes/merge-queue.md and
# AGENTS.md's own note on the same hazard. Nothing here ever calls it.
#
# # The clock
#
# The risk clock is the newest modification time among the worktree's changed files (tracked and
# untracked), not the branch's last commit date. A worktree can carry a commit from hours ago and
# still be safe, mid-edit, seconds old; the fact that matters is how long the CURRENT uncommitted
# state has sat without being turned into a commit. `AT_RISK_MINUTES` (default 30, AGENTS.md's own
# "half an hour") is overridable the way `GRACE_MINUTES` already is in `helpers/lane-claim-check.sh`.
#
# # Name: unrecorded.
#
# Provisional, minted by this lane. "at-risk" is AGENTS.md's own phrase for exactly this ("work at
# risk"); "-check" matches `helpers/lane-claim-check.sh`, `script/qemu-check` and
# `script/stack-frame-check`. It lives in `helpers/` rather than `script/` for the same reason
# `merge-drain.sh` gives for itself: a maintainer's tool, not a front door a contributor types, so it
# carries no `notes/scripts.md` entry (`script/lint`'s "script docs" check only walks `script/`).
#
# # BUGS
#
#   - **It does not distinguish a lane mid-work from a lane that stalled.** Thirty minutes without a
#     commit is common for an actively-edited file too (a long edit, a long test run before
#     committing), so this can flag live work. It is not wrong to flag it: the risk (one prune away
#     from gone) is real in both cases, and the fix is the same (commit, or push what exists as a
#     patch). A reader still has to tell "safe, still typing" from "abandoned" the same way
#     `helpers/lane-claim-check.sh` leaves "safe to delete" versus "somebody has to look" to a
#     person.
#   - **It does not dedupe across passes.** Every call reports every worktree still over the
#     threshold, so a worktree that stays at risk for two hours is reported on every pass that reads
#     it, unlike `merge-drain.sh`'s `notify()` or `trunk-health.sh`'s own transition-only reporting.
#     That was a deliberate choice over adding a second piece of state to track "already said": a
#     worktree at risk stays exactly as at risk on the next pass, and repetition costs nothing more
#     than log lines, which is the same trade `helpers/lane-claim-check.sh` already makes for the
#     identical reason.
#   - **A worktree whose directory was removed without `git worktree remove` reports `prunable` and
#     is skipped rather than flagged.** There is nothing left in it to lose, so this is correct, not
#     a gap; noted because a reader diffing this script's count against `git worktree list`'s might
#     otherwise wonder where an entry went.
#   - **mtime, not content.** A file touched (e.g. by a formatter, or `touch`) without a real edit
#     ages this the same as a genuine change. `git diff --shortstat` in the report line at least says
#     how much moved, but nothing here reads whether the touch mattered.

set -e
cd "$(dirname "$0")/.."

AT_RISK_MINUTES=${AT_RISK_MINUTES:-30}
threshold=$((AT_RISK_MINUTES * 60))
now=$(date -u +%s)

# Every worktree of this checkout, skipping the first: `git worktree list` always lists the main
# checkout first, and this script has nothing to say about it (it is not a lane, and pruning it is
# not a one-command accident the way pruning a linked worktree is). Skip anything `prunable`: its
# directory is already gone, so there is nothing left in it to lose.
worktrees=$(git worktree list --porcelain | awk '
	/^worktree / {
		if (n > 1 && prunable != "1") print path "\t" branch
		n++
		path = substr($0, 10)
		branch = "(detached)"
		prunable = "0"
		next
	}
	/^branch / {
		branch = substr($0, 8)
		sub(/^refs\/heads\//, "", branch)
		next
	}
	/^prunable/ { prunable = "1"; next }
	END { if (n > 1 && prunable != "1") print path "\t" branch }
')

[ -z "$worktrees" ] && exit 0

while IFS="$(printf '\t')" read -r path branch; do
	[ -z "$path" ] && continue
	[ -d "$path" ] || continue

	status=$(git -C "$path" status --porcelain 2>/dev/null)
	[ -z "$status" ] && continue

	count=$(printf '%s\n' "$status" | grep -c . || true)

	# The newest mtime among the changed paths. Reading the file list from `status`, not from
	# `git diff --name-only`, because it is the one command that also names untracked files, which
	# matter here: a new file nobody has `git add`-ed yet is exactly the kind of work a prune
	# destroys silently.
	newest=0
	while IFS= read -r line; do
		[ -z "$line" ] && continue
		# Porcelain v1: two status letters, a space, then the path (or "old -> new" for a
		# rename/copy, where the path that still exists on disk is the part after "-> ").
		entry=$(printf '%s' "$line" | cut -c4-)
		case "$entry" in
		*' -> '*) entry=${entry##*' -> '} ;;
		esac
		full="$path/$entry"
		[ -e "$full" ] || continue
		# GNU first: GNU `stat -f` means filesystem status and prints to stdout before failing, which
		# would leave several lines in $mtime. BSD `stat -c` fails without printing.
		mtime=$(stat -c %Y "$full" 2>/dev/null || stat -f %m "$full" 2>/dev/null || echo 0)
		[ "$mtime" -gt "$newest" ] && newest=$mtime
	done <<EOF
$status
EOF

	[ "$newest" -eq 0 ] && continue
	age=$((now - newest))
	[ "$age" -lt "$threshold" ] && continue

	mins=$((age / 60))
	echo "at-risk-check: UNCOMMITTED. $path ($branch) has $count changed file(s), newest" \
		"touched $mins minutes ago. One prune away from gone; commit and push."
done <<EOF
$worktrees
EOF

# Always exits 0: this is a report, not a gate, the same posture `helpers/lane-claim-check.sh`
# takes ("Not a gate" in that file's own header) for the same reason.
exit 0
