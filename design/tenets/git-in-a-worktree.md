# Git in a shared worktree: why the commit rules read as opposites

*Appendix to [`AGENTS.md`](../../AGENTS.md), which carries the rules: one purpose per commit, commit
early and push, squash checkpoints against the recorded base SHA, never `git stash` in a worktree,
never squash across purposes. This file carries why each rule exists and the failure that produced
it, all of them from days when a lane lost work or took another lane's. Moved here 2026-09-23 (UTC)
on calef's authorization, unchanged in substance.*


One purpose per commit. The message explains why, not what (the diff shows what). If a commit
records a correction or a surprise, say so in the message. See the history of milestone 1 (boot to
Rust on QEMU `virt`) for the shape.

Commit early and push, then curate before reporting. These two rules read as opposites and are
not, and the resolution is a criterion rather than a compromise: `git blame` is what a commit is
for. A reader tracing why a line looks the way it does must land on a commit that explains it.

So while working, commit whenever a piece works and push whenever a commit exists, because a pushed
branch survives a dead session, a killed process and a laptop that will not wake, and nothing else
does. On 2026-08-04 a lane sat on seven modified files with zero commits for hours; had that
worktree been pruned the work was gone, and it was caught by inspection rather than by any
mechanism. Uncommitted work in a lane worktree is the one thing no part of this system protects.

Then, before reporting, squash the checkpoints into the purposes and force-push. A checkpoint is
for the lane's own safety and has no reader; a purpose commit has one.

**Squash against the base commit you branched from, never against `origin/main`**, and this trap was
sprung the day the rule above was written (2026-08-04). Agent worktrees share one `.git`, so
`origin/main` moves under a lane while it works: a developer that ran `git reset --soft origin/main`
to squash silently staged four other lanes' files as its own, including a deletion, and caught
it only by reading `git status` before committing. Record the base SHA when the branch is cut and
squash against that. The wider rule it belongs to: in a worktree, `origin/*` is not a fixed point.

`git stash` is unsafe in these worktrees, for the same reason one level over. The stash stack is
per-`.git`, not per-worktree, so it is shared machine-wide across every lane. Found 2026-08-26 by
the two-core-crash lane of milestone 161 (the x86_64 kernel port): another session pushed a stash
between this lane's `git stash` and its `git stash pop`, so the pop popped *someone else's* entry
into this lane's tree and conflicted. Nothing was lost that time (the conflict preserved the other
entry, and it was restored and re-popped by name), but the failure mode is the same family as the
squash-against-`origin/main` trap above: a command that reads as lane-local silently touches shared
state. **Use a patch file (`git diff > /tmp/<lane>-<what>.patch`, later `git apply`) instead of `git
stash` in a worktree**, the same way `origin/*` is not a fixed point once more than one lane can
move it. Name it what no other lane would: this said `/tmp/x.patch` until two lanes both staged
a body at `/tmp/pr-body.md` on 2026-09-21 and one pushed the other's text.

**Never squash across purposes.** Squash-*merging* is already impossible: `allow_squash_merge` is
`false` on this repository, so the platform refuses it. This clause stays a prohibition because
nothing gates it. The lane of milestone 96 (one init: the spawn service written twice) put the
loader unification in its own commit *ahead of* the migration precisely so that a boot failure could
not be ambiguous between two changes, which is the whole reason that structure exists. A
squash-merge would have destroyed it. The merge commit carries the pull request's title, so `git log
--first-parent` already reads as one entry per piece of work while the detail stays reachable
underneath.

The exceptions worth keeping unsquashed: a commit that records a correction or a surprise, and a
commit whose separateness is itself the argument (96's loader, above).
