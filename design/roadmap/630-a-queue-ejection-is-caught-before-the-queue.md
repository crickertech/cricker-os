---
status: BUILT
raised: 2026-10-03
built: 2026-10-03
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 630. A merge-queue ejection is caught before the queue, and recovered after it

Raised 2026-10-03 (UTC) by calef, after #1473 (the x86_64 paint path) was ejected from the merge
queue at 01:45 UTC. The number is provisional until the merge queue lands it; the title and slug
are drafts, and every new name below is provisional.

## Why

swish-check makes every merge-queue attempt cost 20 to 38 minutes, so each ejection is expensive.
#1473's cause was `script/roadmap --check`'s "IN-PROGRESS branch that already merged" check, which
can only fail inside a merge group, because only there does the branch's merge commit exist. The
ejection also cancelled auto-merge silently, which is the sixth class in
`notes/coes/2026-09-30-lane-follow-through.md`, whose mechanism 5 commissioned the recovery half.

## Built

Each piece is described, with its measurements and its falsification, in
`notes/queue-ejection.md`.

1. Caught at ready time. `script/roadmap --ready-branch BRANCH` reads frontmatter only and fails
   when a block still reads IN-PROGRESS on BRANCH, naming the block and the fix.
   `script/roadmap --selftest` proves it against seven fixtures under `script/lint`.
   `.github/workflows/ready-status.yml` runs it on `pull_request` and `merge_group`, and a draft
   passes. Falsified live on #1491: green as a draft, red 11 seconds after `gh pr ready`, green
   again after `gh pr ready --undo`. It was closed and its branch deleted.
2. The pre-push hook runs all of lint. Measured warm, three runs each, `script/lint --clippy`
   took a median 27.4 s and `script/lint` 56.5 s. No check after clippy is over 3.5 s, so the
   hook runs the whole thing in place of `--clippy` and three hand-picked gates. It also runs
   `--ready-branch` when the pushed branch's pull request is ready. Its first full run found that
   a hook inherits GIT_DIR, which let a selftest write into the shared `.git/config`; the hook
   and the selftest now clear it.
3. Recovered after. `helpers/merge-drain.sh` reads each open pull request's last queue removal
   and enqueue, and `helpers/queue-ejected.jq` decides, with fixtures under `script/lint`. One
   comment per ejection names the reason and the group's unsuccessful runs. A head whose group run
   failed or timed out is labelled `queue-ejected` and not re-armed. A cancelled group is re-armed
   at the same head, because 13 of the 18 `failed_checks` ejections so far were cancellations.
   The label comes off when the head moves, and the drain re-arms it unless it carries
   `needs-architect`.

## BUGS

- `ready status` is not a required check, so a pull request armed by hand passes it by. The drain
  will not arm one with a failing check, which covers the normal path. Requiring it is calef's.
- Creating the `queue-ejected` label the first time may need Issues write, which `nife-smelter`
  lacks. The drain then says `STALLED`, comments that it could not hold, and re-arms as it did
  before. Nothing here has run as the bot yet: the drain runs only from `main`.
- `--ready-branch` matches by branch name. A block naming the wrong branch passes it and still
  fails in the group.

## Follow-on

- **Recorded.** Making `ready status` a required check and pre-creating the label are the two
  asks, written where a reader meets them: the BUGS of `notes/queue-ejection.md` and of
  `.github/workflows/ready-status.yml`.

## Index row

A ready pull request whose own block still reads IN-PROGRESS fails in seconds on `pull_request`,
not 20 to 38 minutes into a merge group. The pre-push hook runs all of `script/lint`. The merge
drain says why a pull request was ejected, holds a head whose group failed, and re-arms it when
the head moves.
