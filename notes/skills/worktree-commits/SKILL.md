---
name: worktree-commits
description: >-
  Committing in nife: load before committing, squashing, force-pushing or saving work aside in a
  lane worktree. Never use git stash (the stash stack is shared machine-wide by every lane), and
  squash against the base SHA recorded when the branch was cut, never against origin/main, which
  moves under a lane while it works. One purpose per commit, the message says why, commit early and
  push, curate before reporting.
---

# Commits, and git in a worktree

Moved whole from `AGENTS.md` on 2026-10-06 (UTC) by lane/agents-md-skills, wording unchanged apart
from heading levels and link paths, so where it says "this file" it means `AGENTS.md`. Its name was
ratified by calef on 2026-10-06 (UTC); notes/skills/README.md records it.

## Commits

One purpose per commit. The message explains why, not what (the diff shows what). If a commit
records a correction or a surprise, say so in the message. Why these rules read as opposites and are
not, with the failure behind each, is in
[design/tenets/git-in-a-worktree.md](../../../design/tenets/git-in-a-worktree.md).

Commit early and push, then curate before reporting. The criterion that resolves the two: `git
blame` is what a commit is for. A reader tracing why a line looks the way it does must land on a
commit that explains it.

- While working, commit whenever a piece works and push whenever a commit exists. A pushed branch
  survives a dead session, a killed process and a laptop that will not wake, and nothing else does.
  Uncommitted work in a lane worktree is the one thing no part of this system protects.
- Before reporting, squash the checkpoints into the purposes and force-push.
- Every commit an agent writes ends with a `Co-Authored-By:` trailer naming its model, such as
  `Co-Authored-By: GLM 5.3 <noreply@z.ai>`, whatever the vendor or tool; `script/metrics` reads it.
- Squash against the base commit you branched from, never against `origin/main`. Agent worktrees
  share one `.git`, so `origin/main` moves under a lane while it works, and `git reset --soft
  origin/main` has silently staged four other lanes' files as one lane's own. Record the base SHA
  when the branch is cut and squash against that. The wider rule: in a worktree, `origin/*` is not a
  fixed point.
- `git stash` is unsafe in these worktrees, for the same reason one level over: the stash stack is
  per-`.git`, so it is shared machine-wide across every lane. Use a patch file (`git diff >
  /tmp/<lane>-<what>.patch`, later `git apply`) instead, and name it what no other lane would.
- Never squash across purposes. Squash-*merging* is already impossible (`allow_squash_merge` is
  `false` on this repository); this clause stays a prohibition because nothing gates it. The
  exceptions worth keeping unsquashed: a commit that records a correction or a surprise, and a
  commit whose separateness is itself the argument.
